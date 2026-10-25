use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

use hub::presence::ClaudeSessions;
use hub::{Activity, Presence};
use time::OffsetDateTime;

use crate::cli;

/// Claude Code's session registry under `$CLAUDE_CONFIG_DIR`, else `~/.claude`, found off the startup path since the
/// login shell is slow; activity reads as unknown until then.
pub fn claude_sessions(home: PathBuf) -> Box<dyn Presence> {
    let (found, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let config = cli::login_shell_var("CLAUDE_CONFIG_DIR").map_or_else(|| home.join(".claude"), PathBuf::from);
        let _ = found.send(ClaudeSessions::new(config.join("sessions")));
    });
    Box::new(Deferred { rx, sessions: None })
}

struct Deferred {
    rx: Receiver<ClaudeSessions>,
    sessions: Option<ClaudeSessions>,
}

impl Presence for Deferred {
    fn activity(&mut self, pid: u32) -> Option<(Activity, OffsetDateTime)> {
        if self.sessions.is_none() {
            self.sessions = self.rx.try_recv().ok();
        }
        self.sessions.as_mut()?.activity(pid)
    }
}
