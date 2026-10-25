//! The user's login-shell environment (PLAN.md D42): an app started from Finder lacks what shell profiles add.

use std::ffi::OsString;
use std::process::{Command, Stdio};

/// `name` as the user's login shell sets it, else the app's own; runs the shell, so keep it off the main thread.
pub fn var(name: &str) -> Option<OsString> {
    const TAG: &str = "__APOLLO_VAR__";
    let shell = std::env::var_os("SHELL").unwrap_or_else(|| "/bin/zsh".into());
    let output = Command::new(shell)
        .args(["-ilc", &format!("printf '{TAG}%s' \"${name}\"")])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.rsplit_once(TAG)
        .map(|(_, value)| value.trim())
        .filter(|value| !value.is_empty())
        .map(Into::into)
        .or_else(|| std::env::var_os(name))
}
