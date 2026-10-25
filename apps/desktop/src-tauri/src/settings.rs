//! Preferences in `settings.json` (PLAN.md §8.3).

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

pub const SETTINGS_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
#[ts(export)]
pub struct Settings {
    pub version: u32,
    pub overlay: OverlaySettings,
    pub sessions: SessionSettings,
    pub history: HistorySettings,
    pub shortcuts: ShortcutSettings,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
#[ts(export)]
pub struct OverlaySettings {
    pub visible_at_launch: bool,
    pub reply_preview_lines: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
#[ts(export)]
pub struct SessionSettings {
    /// A session the hub has never seen takes the target as it connects (D92).
    pub target_new: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
#[ts(export)]
pub struct HistorySettings {
    pub retention_days: u16,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
#[ts(export)]
pub struct ShortcutSettings {
    pub focus_input: Option<String>,
    pub toggle_overlay: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            overlay: OverlaySettings::default(),
            sessions: SessionSettings::default(),
            history: HistorySettings::default(),
            shortcuts: ShortcutSettings::default(),
        }
    }
}

impl Default for OverlaySettings {
    fn default() -> Self {
        Self {
            visible_at_launch: true,
            reply_preview_lines: 3,
        }
    }
}

impl Default for SessionSettings {
    fn default() -> Self {
        Self { target_new: true }
    }
}

impl Default for HistorySettings {
    fn default() -> Self {
        Self { retention_days: 30 }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("{0}")]
    Invalid(String),
    #[error("cannot write settings: {0}")]
    Io(#[from] std::io::Error),
}

impl Settings {
    fn validate(&self) -> Result<(), SettingsError> {
        if !(1..=8).contains(&self.overlay.reply_preview_lines) {
            return Err(SettingsError::Invalid(
                "reply preview lines must be between 1 and 8".into(),
            ));
        }
        if !(1..=365).contains(&self.history.retention_days) {
            return Err(SettingsError::Invalid(
                "history must be kept between 1 and 365 days".into(),
            ));
        }
        Ok(())
    }
}

fn merge(base: &mut Value, patch: Value) {
    match (base, patch) {
        (Value::Object(base), Value::Object(patch)) => {
            for (key, value) in patch {
                match base.get_mut(&key) {
                    Some(slot) => merge(slot, value),
                    None => {
                        base.insert(key, value);
                    }
                }
            }
        }
        (slot, value) => *slot = value,
    }
}

/// Loads with defaults for missing keys, validates every change and writes atomically.
pub struct SettingsService {
    path: PathBuf,
    current: Mutex<Settings>,
}

impl SettingsService {
    pub fn load(path: PathBuf) -> Self {
        let current = match fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<Settings>(&text) {
                Ok(settings) if settings.validate().is_ok() => settings,
                _ => {
                    tracing::warn!("settings.json is invalid; using defaults");
                    Settings::default()
                }
            },
            Err(_) => Settings::default(),
        };
        Self {
            path,
            current: Mutex::new(current),
        }
    }

    pub fn get(&self) -> Settings {
        self.current.lock().expect("settings lock").clone()
    }

    /// Applies a partial patch; unknown keys and out-of-range values are rejected.
    pub fn update(&self, patch: Value) -> Result<Settings, SettingsError> {
        let mut current = self.current.lock().expect("settings lock");
        let mut merged = serde_json::to_value(&*current).expect("settings serialise");
        merge(&mut merged, patch);
        let next: Settings = serde_json::from_value(merged).map_err(|e| SettingsError::Invalid(e.to_string()))?;
        next.validate()?;
        write_atomically(&self.path, &next)?;
        *current = next.clone();
        Ok(next)
    }
}

fn write_atomically(path: &Path, settings: &Settings) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    let mut file = OpenOptions::new().write(true).create(true).truncate(true).open(&tmp)?;
    file.write_all(serde_json::to_string_pretty(settings)?.as_bytes())?;
    file.sync_all()?;
    fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn service() -> (tempfile::TempDir, SettingsService) {
        let dir = tempfile::tempdir().unwrap();
        let service = SettingsService::load(dir.path().join("settings.json"));
        (dir, service)
    }

    #[test]
    fn missing_file_and_missing_keys_get_defaults() {
        let (dir, service) = service();
        assert_eq!(service.get(), Settings::default());
        fs::write(
            dir.path().join("settings.json"),
            r#"{"overlay":{"replyPreviewLines":5}}"#,
        )
        .unwrap();
        let loaded = SettingsService::load(dir.path().join("settings.json")).get();
        assert_eq!(loaded.overlay.reply_preview_lines, 5);
        assert!(loaded.overlay.visible_at_launch);
        assert!(loaded.sessions.target_new);
        assert_eq!(loaded.history.retention_days, 30);
    }

    #[test]
    fn an_invalid_file_falls_back_to_defaults() {
        let (dir, _) = service();
        fs::write(
            dir.path().join("settings.json"),
            r#"{"overlay":{"replyPreviewLines":99}}"#,
        )
        .unwrap();
        assert_eq!(
            SettingsService::load(dir.path().join("settings.json")).get(),
            Settings::default()
        );
    }

    #[test]
    fn a_patch_merges_persists_and_reloads() {
        let (dir, service) = service();
        let updated = service.update(json!({"history": {"retentionDays": 7}})).unwrap();
        assert_eq!(updated.history.retention_days, 7);
        assert_eq!(updated.overlay, OverlaySettings::default());
        assert_eq!(SettingsService::load(dir.path().join("settings.json")).get(), updated);
        assert!(!dir.path().join("settings.json.tmp").exists());
    }

    #[test]
    fn out_of_range_values_are_rejected_and_nothing_changes() {
        let (dir, service) = service();
        assert!(service.update(json!({"overlay": {"replyPreviewLines": 0}})).is_err());
        assert!(service.update(json!({"history": {"retentionDays": 366}})).is_err());
        assert_eq!(service.get(), Settings::default());
        assert!(!dir.path().join("settings.json").exists());
    }

    #[test]
    fn unknown_keys_are_rejected() {
        let (_dir, service) = service();
        assert!(service.update(json!({"overlay": {"colour": "red"}})).is_err());
        assert!(service.update(json!({"theme": "dark"})).is_err());
    }
}
