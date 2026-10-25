use hub::presence::ClaudeSessions;
use hub::{Activity, Presence};
use time::macros::datetime;

const AT_MS: i64 = 1_792_941_868_200;

fn registry(entries: &[(u32, &str)]) -> (tempfile::TempDir, ClaudeSessions) {
    let dir = tempfile::tempdir().unwrap();
    for (pid, text) in entries {
        std::fs::write(dir.path().join(format!("{pid}.json")), text).unwrap();
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
