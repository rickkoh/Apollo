use std::io::Write;
use std::path::Path;

use hub::presence::ClaudeSessions;
use hub::{Activity, Presence};
use time::macros::datetime;

const AT_MS: i64 = 1_792_941_868_200;

fn registry(entries: &[(u32, &str)]) -> (tempfile::TempDir, ClaudeSessions) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("sessions")).unwrap();
    for (pid, text) in entries {
        std::fs::write(dir.path().join(format!("sessions/{pid}.json")), text).unwrap();
    }
    let sessions = ClaudeSessions::new(dir.path().to_owned());
    (dir, sessions)
}

fn entry(status: &str) -> String {
    format!(
        r#"{{"pid":7,"sessionId":"s","cwd":"/w","kind":"interactive","status":"{status}","statusUpdatedAt":{AT_MS}}}"#
    )
}

#[test]
fn busy_and_idle_with_when_they_started() {
    let (_dir, mut sessions) = registry(&[(7, &entry("busy")), (8, &entry("idle"))]);
    let since = datetime!(2026-10-25 15:24:28.200 UTC);
    assert_eq!(sessions.activity(7), Some((Activity::Busy, since)));
    assert_eq!(sessions.activity(8), Some((Activity::Idle, since)));
}

#[test]
fn a_permission_prompt_is_busy_and_a_background_shell_is_idle() {
    let (_dir, mut sessions) = registry(&[(7, &entry("waiting")), (8, &entry("shell"))]);
    assert_eq!(sessions.activity(7).map(|(a, _)| a), Some(Activity::Busy));
    assert_eq!(sessions.activity(8).map(|(a, _)| a), Some(Activity::Idle));
}

#[test]
fn an_unknown_status_is_unknown() {
    let (_dir, mut sessions) = registry(&[(7, &entry("thinking"))]);
    assert_eq!(sessions.activity(7), None);
    assert_eq!(sessions.activity(7), None);
}

#[test]
fn a_malformed_or_missing_entry_is_unknown() {
    let (_dir, mut sessions) = registry(&[(7, "{\"status\":"), (8, r#"{"status":"busy"}"#)]);
    assert_eq!(sessions.activity(7), None);
    assert_eq!(sessions.activity(8), None, "no statusUpdatedAt");
    assert_eq!(sessions.activity(9), None);
}

fn session(session_id: &str) -> String {
    format!(r#"{{"pid":7,"sessionId":"{session_id}","status":"idle","statusUpdatedAt":{AT_MS}}}"#)
}

fn append(dir: &Path, project: &str, session_id: &str, lines: &[String]) {
    let folder = dir.join("projects").join(project);
    std::fs::create_dir_all(&folder).unwrap();
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(folder.join(format!("{session_id}.jsonl")))
        .unwrap();
    for line in lines {
        writeln!(file, "{line}").unwrap();
    }
}

fn ai_title(title: &str) -> String {
    format!(r#"{{"type":"ai-title","aiTitle":"{title}","sessionId":"s1"}}"#)
}

fn custom_title(title: &str) -> String {
    format!(r#"{{"type":"custom-title","customTitle":"{title}","sessionId":"s1"}}"#)
}

fn chat(bytes: usize) -> String {
    format!(r#"{{"type":"user","message":"{}"}}"#, "x".repeat(bytes))
}

#[test]
fn the_title_comes_from_the_sessions_transcript_in_any_project_folder() {
    let (dir, mut sessions) = registry(&[(7, &session("s1"))]);
    append(dir.path(), "-Users-me-other", "s0", &[ai_title("Not this one")]);
    append(
        dir.path(),
        "-Users-me-work",
        "s1",
        &[chat(10), ai_title("Fix login"), chat(10)],
    );
    assert_eq!(sessions.title(7).as_deref(), Some("Fix login"));
}

#[test]
fn a_rename_wins_over_claudes_own_title() {
    let (dir, mut sessions) = registry(&[(7, &session("s1"))]);
    append(
        dir.path(),
        "p",
        "s1",
        &[ai_title("Fix login"), custom_title("Payments")],
    );
    assert_eq!(sessions.title(7).as_deref(), Some("Payments"));
    append(dir.path(), "p", "s1", &[ai_title("Fix login again")]);
    assert_eq!(sessions.title(7).as_deref(), Some("Payments"));
}

#[test]
fn the_title_follows_the_transcript_as_it_grows() {
    let (dir, mut sessions) = registry(&[(7, &session("s1"))]);
    assert_eq!(sessions.title(7), None, "no transcript before the first prompt");
    append(dir.path(), "p", "s1", &[chat(10)]);
    assert_eq!(sessions.title(7), None);
    append(dir.path(), "p", "s1", &[ai_title("Fix login")]);
    assert_eq!(sessions.title(7).as_deref(), Some("Fix login"));
    append(dir.path(), "p", "s1", &[custom_title("Payments")]);
    assert_eq!(sessions.title(7).as_deref(), Some("Payments"));
}

#[test]
fn the_latest_title_wins_and_outlives_a_tail_without_one() {
    let (dir, mut sessions) = registry(&[(7, &session("s1"))]);
    append(
        dir.path(),
        "p",
        "s1",
        &[ai_title("Old"), chat(70 * 1024), ai_title("New")],
    );
    assert_eq!(sessions.title(7).as_deref(), Some("New"));
    append(dir.path(), "p", "s1", &[chat(70 * 1024)]);
    assert_eq!(
        sessions.title(7).as_deref(),
        Some("New"),
        "kept while the tail holds no title"
    );
}

#[test]
fn a_message_quoting_a_title_line_is_not_a_title() {
    let (dir, mut sessions) = registry(&[(7, &session("s1"))]);
    let quoted = r#"{"type":"user","message":"{\"type\":\"custom-title\",\"customTitle\":\"Fake\"}"}"#;
    append(dir.path(), "p", "s1", &[ai_title("Fix login"), quoted.to_owned()]);
    assert_eq!(sessions.title(7).as_deref(), Some("Fix login"));
}
