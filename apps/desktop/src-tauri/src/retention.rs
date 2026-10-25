//! Sessions follow Claude Code (D94): an ended session whose transcript Claude Code no longer keeps is removed.

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::Duration;

use hub::{HubHandle, SessionId, SessionView};

pub const PRUNE_EVERY: Duration = Duration::from_secs(60 * 60);

/// Session ids with a transcript, `projects/<folder>/<id>.jsonl`; `None` when there are none at all.
fn transcripts(projects: &Path) -> Option<HashSet<String>> {
    let ids: HashSet<String> = fs::read_dir(projects)
        .ok()?
        .flatten()
        .filter_map(|folder| fs::read_dir(folder.path()).ok())
        .flatten()
        .flatten()
        .filter_map(|file| {
            let path = file.path();
            if path.extension()? != "jsonl" {
                return None;
            }
            Some(path.file_stem()?.to_str()?.to_owned())
        })
        .collect();
    (!ids.is_empty()).then_some(ids)
}

/// Ended sessions without a transcript; `None` when Claude Code keeps none, so a changed layout removes nothing.
fn dropped(projects: &Path, sessions: &[SessionView]) -> Option<Vec<SessionId>> {
    let kept = transcripts(projects)?;
    Some(
        sessions
            .iter()
            .filter(|s| !s.live && !kept.contains(s.id.as_str()))
            .map(|s| s.id.clone())
            .collect(),
    )
}

pub async fn forget_dropped(projects: &Path, hub: &HubHandle) {
    let sessions = match hub.sessions().await {
        Ok(sessions) => sessions,
        Err(err) => {
            tracing::warn!("listing sessions to compare with Claude Code failed: {err}");
            return;
        }
    };
    let scan = projects.to_owned();
    let Ok(Some(dropped)) = tauri::async_runtime::spawn_blocking(move || dropped(&scan, &sessions)).await else {
        tracing::warn!(
            "no Claude Code transcripts in {}; keeping every session",
            projects.display()
        );
        return;
    };
    for id in dropped {
        match hub.forget(id.clone()).await {
            Ok(()) => tracing::info!(session = %id, "removed a session Claude Code no longer keeps"),
            Err(err) => tracing::warn!(session = %id, "removing a session Claude Code dropped failed: {err}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "{}\n").unwrap();
    }

    fn session(id: &str, live: bool) -> SessionView {
        SessionView {
            id: SessionId(id.into()),
            title: id.into(),
            cwd: "/work".into(),
            live,
            is_target: false,
            last_activity: None,
            unread: 0,
            activity: None,
        }
    }

    #[test]
    fn transcripts_come_from_every_project_folder_and_nothing_deeper() {
        let dir = tempfile::tempdir().unwrap();
        let projects = dir.path().join("projects");
        write(&projects.join("-work-a/s1.jsonl"));
        write(&projects.join("-work-a/notes.md"));
        write(&projects.join("-work-a/s1/subagents/agent-1.jsonl"));
        write(&projects.join("-work-b/s2.jsonl"));
        write(&projects.join("stray.jsonl"));
        let ids = transcripts(&projects).unwrap();
        assert_eq!(ids, HashSet::from(["s1".to_owned(), "s2".to_owned()]));
    }

    #[test]
    fn without_any_transcript_nothing_is_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let projects = dir.path().join("projects");
        let sessions = [session("gone", false)];
        assert_eq!(dropped(&projects, &sessions), None, "no projects folder");
        fs::create_dir_all(projects.join("-work-a")).unwrap();
        write(&projects.join("-work-a/notes.md"));
        assert_eq!(dropped(&projects, &sessions), None, "folders without transcripts");
    }

    #[test]
    fn only_ended_sessions_without_a_transcript_are_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let projects = dir.path().join("projects");
        write(&projects.join("-work-a/kept.jsonl"));
        write(&projects.join("-work-a/live-kept.jsonl"));
        let sessions = [
            session("kept", false),
            session("gone", false),
            session("live-new", true),
            session("live-kept", true),
        ];
        assert_eq!(dropped(&projects, &sessions), Some(vec![SessionId("gone".into())]));
    }
}
