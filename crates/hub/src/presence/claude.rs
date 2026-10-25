use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde::de::DeserializeOwned;
use time::OffsetDateTime;

use super::{Activity, Presence};

/// Claude Code re-appends its title lines, so the latest sits near a transcript's end.
const TITLE_TAIL: u64 = 64 * 1024;

/// Claude Code's config folder: the session registry, one `sessions/<pid>.json` per running session, and the
/// transcripts, `projects/<folder>/<session id>.jsonl`. Both formats are undocumented (PLAN.md §22.4, §22.5).
pub struct ClaudeSessions {
    dir: PathBuf,
    warned: HashSet<Problem>,
    transcripts: HashMap<String, Transcript>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum Problem {
    Missing,
    Malformed,
    UnknownStatus(String),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    status: String,
    /// Unix milliseconds.
    status_updated_at: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionOf {
    session_id: String,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum TitleLine {
    #[serde(rename_all = "camelCase")]
    CustomTitle { custom_title: String },
    #[serde(rename_all = "camelCase")]
    AiTitle { ai_title: String },
}

struct Transcript {
    path: PathBuf,
    len: u64,
    title: Option<String>,
}

impl ClaudeSessions {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            warned: HashSet::new(),
            transcripts: HashMap::new(),
        }
    }

    fn warn_once(&mut self, problem: Problem, detail: String) {
        if self.warned.insert(problem) {
            tracing::warn!("session registry: {detail}");
        }
    }

    fn entry<T: DeserializeOwned>(&mut self, pid: u32) -> Option<T> {
        let path = self.dir.join("sessions").join(format!("{pid}.json"));
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) => {
                self.warn_once(Problem::Missing, format!("cannot read {}: {err}", path.display()));
                return None;
            }
        };
        match serde_json::from_str(&text) {
            Ok(entry) => Some(entry),
            Err(err) => {
                self.warn_once(
                    Problem::Malformed,
                    format!("unexpected format in {}: {err}", path.display()),
                );
                None
            }
        }
    }
}

impl Presence for ClaudeSessions {
    fn activity(&mut self, pid: u32) -> Option<(Activity, OffsetDateTime)> {
        let entry: Entry = self.entry(pid)?;
        // `shell` is a finished turn with a background command still running; `waiting` is a turn held by a prompt.
        let activity = match entry.status.as_str() {
            "busy" | "waiting" => Activity::Busy,
            "idle" | "shell" => Activity::Idle,
            _ => {
                let detail = format!("unknown status {:?}", entry.status);
                self.warn_once(Problem::UnknownStatus(entry.status), detail);
                return None;
            }
        };
        let since = OffsetDateTime::from_unix_timestamp_nanos(i128::from(entry.status_updated_at) * 1_000_000).ok()?;
        Some((activity, since))
    }

    /// Through the registry's session id, which follows the conversation the user sees.
    fn title(&mut self, pid: u32) -> Option<String> {
        let SessionOf { session_id } = self.entry(pid)?;
        if !self.transcripts.contains_key(&session_id) {
            let path = find_transcript(&self.dir.join("projects"), &session_id)?;
            let transcript = Transcript {
                path,
                len: 0,
                title: None,
            };
            self.transcripts.insert(session_id.clone(), transcript);
        }
        let transcript = self.transcripts.get_mut(&session_id)?;
        transcript.refresh();
        transcript.title.clone()
    }
}

/// Searched rather than derived: the folder name follows a rule of Claude Code's own.
fn find_transcript(projects: &Path, session_id: &str) -> Option<PathBuf> {
    let name = format!("{session_id}.jsonl");
    std::fs::read_dir(projects)
        .ok()?
        .filter_map(Result::ok)
        .map(|folder| folder.path().join(&name))
        .find(|path| path.is_file())
}

impl Transcript {
    fn refresh(&mut self) {
        let Ok(len) = std::fs::metadata(&self.path).map(|m| m.len()) else {
            return;
        };
        if len == self.len {
            return;
        }
        self.len = len;
        if let Some(title) = tail_title(&self.path, len) {
            self.title = Some(title);
        }
    }
}

/// The latest `/rename` in the tail, else Claude's latest own title.
fn tail_title(path: &Path, len: u64) -> Option<String> {
    let start = len.saturating_sub(TITLE_TAIL);
    let mut file = File::open(path).ok()?;
    file.seek(SeekFrom::Start(start)).ok()?;
    let mut bytes = Vec::new();
    file.take(len - start).read_to_end(&mut bytes).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    // A tail that starts mid-file starts mid-line.
    let text = if start == 0 {
        &text[..]
    } else {
        text.split_once('\n').map_or("", |(_, rest)| rest)
    };
    let mut ai_title = None;
    for line in text.lines().rev().filter(|line| line.contains("-title\"")) {
        match serde_json::from_str(line) {
            Ok(TitleLine::CustomTitle { custom_title }) if !custom_title.trim().is_empty() => {
                return Some(custom_title);
            }
            Ok(TitleLine::AiTitle { ai_title: title }) if !title.trim().is_empty() => {
                ai_title.get_or_insert(title);
            }
            _ => {}
        }
    }
    ai_title
}
