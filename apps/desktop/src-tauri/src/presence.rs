use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

use hub::presence::ClaudeSessions;
use hub::{Activity, Presence};
use time::OffsetDateTime;

use crate::login_shell;

/// Claude Code's sessions under `$CLAUDE_CONFIG_DIR`, else `~/.claude`, found off the startup path since the login
/// shell is slow; activity and titles read as unknown until then.
pub fn claude_sessions(home: PathBuf) -> Box<dyn Presence> {
    let (found, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let config = login_shell::var("CLAUDE_CONFIG_DIR").map_or_else(|| home.join(".claude"), PathBuf::from);
        let _ = found.send(ClaudeSessions::new(config));
    });
    Box::new(Deferred { rx, sessions: None })
}

struct Deferred {
    rx: Receiver<ClaudeSessions>,
    sessions: Option<ClaudeSessions>,
}

impl Deferred {
    fn sessions(&mut self) -> Option<&mut ClaudeSessions> {
        if self.sessions.is_none() {
            self.sessions = self.rx.try_recv().ok();
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
