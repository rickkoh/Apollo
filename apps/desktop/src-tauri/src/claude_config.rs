//! Claude Code's config folder: `$CLAUDE_CONFIG_DIR` from the login shell, else `~/.claude` (D65).

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use crate::login_shell;

/// Found on a background thread, since the login shell is slow; `get` reads `None` until then.
#[derive(Clone, Default)]
pub struct ClaudeConfig(Arc<OnceLock<PathBuf>>);

impl ClaudeConfig {
    pub fn locate(home: PathBuf) -> Self {
        let config = Self::default();
        let found = config.0.clone();
        std::thread::spawn(move || {
            let dir = login_shell::var("CLAUDE_CONFIG_DIR").map_or_else(|| home.join(".claude"), PathBuf::from);
            let _ = found.set(dir);
        });
        config
    }

    pub fn get(&self) -> Option<&Path> {
        self.0.get().map(PathBuf::as_path)
    }

    /// Blocks until the folder is found.
    pub fn wait(&self) -> &Path {
        self.0.wait()
    }
}
