mod claude;

pub use claude::ClaudeSessions;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use ts_rs::TS;

/// Whether a session's Claude is mid-turn, so a message sent now waits until the turn ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Activity {
    Busy,
    Idle,
}

/// Where the hub learns what a live session's Claude is doing. Used from the actor thread only.
pub trait Presence: Send {
    /// The activity of the Claude process `pid` and when it last changed; `None` when unknown.
    fn activity(&mut self, pid: u32) -> Option<(Activity, OffsetDateTime)>;
}
