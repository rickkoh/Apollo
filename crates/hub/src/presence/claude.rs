use std::collections::HashSet;
use std::path::PathBuf;

use serde::Deserialize;
use time::OffsetDateTime;

use super::{Activity, Presence};

/// Claude Code's session registry, one `<pid>.json` per running session; its format is undocumented (PLAN.md §22.4).
pub struct ClaudeSessions {
    dir: PathBuf,
    warned: HashSet<Problem>,
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

impl ClaudeSessions {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            warned: HashSet::new(),
        }
    }

    fn warn_once(&mut self, problem: Problem, detail: String) {
        if self.warned.insert(problem) {
            tracing::warn!("session registry: {detail}");
        }
    }
}

impl Presence for ClaudeSessions {
    fn activity(&mut self, pid: u32) -> Option<(Activity, OffsetDateTime)> {
        let path = self.dir.join(format!("{pid}.json"));
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) => {
                self.warn_once(Problem::Missing, format!("cannot read {}: {err}", path.display()));
                return None;
            }
        };
        let entry: Entry = match serde_json::from_str(&text) {
            Ok(entry) => entry,
            Err(err) => {
                self.warn_once(
                    Problem::Malformed,
                    format!("unexpected format in {}: {err}", path.display()),
                );
                return None;
            }
        };
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
}
