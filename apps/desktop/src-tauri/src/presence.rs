use hub::presence::ClaudeSessions;
use hub::{Activity, Presence};
use time::OffsetDateTime;

use crate::claude_config::ClaudeConfig;

/// Claude Code's sessions in its config folder; activity and titles read as unknown until the folder is found.
pub fn claude_sessions(config: ClaudeConfig) -> Box<dyn Presence> {
    Box::new(Deferred { config, sessions: None })
}

struct Deferred {
    config: ClaudeConfig,
    sessions: Option<ClaudeSessions>,
}

impl Deferred {
    fn sessions(&mut self) -> Option<&mut ClaudeSessions> {
        if self.sessions.is_none() {
            self.sessions = self.config.get().map(|dir| ClaudeSessions::new(dir.to_owned()));
        }
        self.sessions.as_mut()
    }
}

impl Presence for Deferred {
    fn activity(&mut self, pid: u32) -> Option<(Activity, OffsetDateTime)> {
        self.sessions()?.activity(pid)
    }

    fn title(&mut self, pid: u32) -> Option<String> {
        self.sessions()?.title(pid)
    }
}
