//! Stable "Apollo" or the working copy's "Apollo Dev" (PLAN.md D52), told apart by the bundle identifier.

use serde::Serialize;
use ts_rs::TS;

/// `tauri.dev.conf.json`'s identifier ends with this.
const DEV_SUFFIX: &str = ".dev";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Channel {
    Stable,
    Dev,
}

impl Channel {
    pub fn of(identifier: &str) -> Self {
        if identifier.ends_with(DEV_SUFFIX) {
            Channel::Dev
        } else {
            Channel::Stable
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dev_identifier_names_the_dev_channel() {
        assert_eq!(Channel::of("com.rickkoh.apollo.dev"), Channel::Dev);
        assert_eq!(Channel::of("com.rickkoh.apollo"), Channel::Stable);
        assert_eq!(Channel::of("dev.voicebridge.desktop"), Channel::Stable);
    }
}
