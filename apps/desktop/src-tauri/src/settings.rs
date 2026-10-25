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
    pub layout: OverlayLayout,
    /// Logical pixels, including the transparent shadow margin the page draws inside the window.
    pub width: u16,
    pub reply_preview_lines: u8,
    pub reply_fade_seconds: u16,
    pub max_cards: u8,
    pub input_lines: u8,
    pub expanded_height_percent: u8,
    /// Ended sessions stay in the picker this long; 0 hides them.
    pub picker_ended_hours: u16,
}

/// The island hangs from the notch; the classic overlay floats in a bottom corner.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum OverlayLayout {
    #[default]
    Island,
    Classic,
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
            layout: OverlayLayout::Island,
            width: 420,
            reply_preview_lines: 3,
            reply_fade_seconds: 20,
            max_cards: 3,
            input_lines: 3,
            expanded_height_percent: 40,
            picker_ended_hours: 24,
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

/// Inclusive range of a numeric setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Limit {
    pub min: u16,
    pub max: u16,
}

impl Limit {
    const fn new(min: u16, max: u16) -> Self {
        Self { min, max }
    }

    fn check(self, value: u16, what: &str) -> Result<(), SettingsError> {
        if (self.min..=self.max).contains(&value) {
            Ok(())
        } else {
            Err(SettingsError::Invalid(format!(
                "{what} must be between {} and {}",
                self.min, self.max
            )))
        }
    }
}

/// The ranges `validate` enforces, also served to the Settings page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SettingsLimits {
    pub overlay: OverlayLimits,
    pub history: HistoryLimits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OverlayLimits {
    pub width: Limit,
    pub reply_preview_lines: Limit,
    pub reply_fade_seconds: Limit,
    pub max_cards: Limit,
    pub input_lines: Limit,
    pub expanded_height_percent: Limit,
    pub picker_ended_hours: Limit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HistoryLimits {
    pub retention_days: Limit,
}

pub const LIMITS: SettingsLimits = SettingsLimits {
    overlay: OverlayLimits {
        width: Limit::new(320, 720),
        reply_preview_lines: Limit::new(1, 8),
        reply_fade_seconds: Limit::new(5, 300),
        max_cards: Limit::new(1, 6),
        input_lines: Limit::new(1, 8),
        expanded_height_percent: Limit::new(20, 80),
        picker_ended_hours: Limit::new(0, 168),
    },
    history: HistoryLimits {
        retention_days: Limit::new(1, 365),
    },
};

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("{0}")]
    Invalid(String),
    #[error("cannot write settings: {0}")]
    Io(#[from] std::io::Error),
}

impl Settings {
    fn validate(&self) -> Result<(), SettingsError> {
        let (o, l) = (&self.overlay, &LIMITS.overlay);
        let checks: [(Limit, u16, &str); 8] = [
            (l.width, o.width, "overlay width"),
            (
                l.reply_preview_lines,
                o.reply_preview_lines.into(),
                "reply preview lines",
            ),
            (l.reply_fade_seconds, o.reply_fade_seconds, "reply fade seconds"),
            (l.max_cards, o.max_cards.into(), "cards in the stack"),
            (l.input_lines, o.input_lines.into(), "text box lines"),
            (
                l.expanded_height_percent,
                o.expanded_height_percent.into(),
                "expanded reply height",
            ),
            (l.picker_ended_hours, o.picker_ended_hours, "ended session hours"),
            (
                LIMITS.history.retention_days,
                self.history.retention_days,
                "days of history",
            ),
        ];
        checks
            .into_iter()
            .try_for_each(|(limit, value, what)| limit.check(value, what))
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
    fn the_layout_defaults_to_the_island_and_keeps_classic() {
        let (dir, service) = service();
        assert_eq!(service.get().overlay.layout, OverlayLayout::Island);
        let updated = service.update(json!({"overlay": {"layout": "classic"}})).unwrap();
        assert_eq!(updated.overlay.layout, OverlayLayout::Classic);
        assert_eq!(
            SettingsService::load(dir.path().join("settings.json"))
                .get()
                .overlay
                .layout,
            OverlayLayout::Classic
        );
        assert!(service.update(json!({"overlay": {"layout": "corner"}})).is_err());
    }

    #[test]
    fn defaults_fall_inside_their_limits() {
        Settings::default().validate().unwrap();
    }

    #[test]
    fn out_of_range_values_are_rejected_and_nothing_changes() {
        let (dir, service) = service();
        assert!(service.update(json!({"overlay": {"replyPreviewLines": 0}})).is_err());
        assert!(service.update(json!({"overlay": {"width": 319}})).is_err());
        assert!(service.update(json!({"overlay": {"replyFadeSeconds": 4}})).is_err());
        assert!(service.update(json!({"overlay": {"maxCards": 7}})).is_err());
        assert!(service.update(json!({"overlay": {"pickerEndedHours": 169}})).is_err());
        assert!(service.update(json!({"history": {"retentionDays": 366}})).is_err());
        assert_eq!(service.get(), Settings::default());
        assert!(!dir.path().join("settings.json").exists());
    }

    #[test]
    fn limit_edges_are_accepted() {
        let (_dir, service) = service();
        let updated = service
            .update(json!({"overlay": {"width": 720, "pickerEndedHours": 0, "expandedHeightPercent": 20}}))
            .unwrap();
        assert_eq!(updated.overlay.width, 720);
        assert_eq!(updated.overlay.picker_ended_hours, 0);
    }

    #[test]
    fn unknown_keys_are_rejected() {
        let (_dir, service) = service();
        assert!(service.update(json!({"overlay": {"colour": "red"}})).is_err());
        assert!(service.update(json!({"theme": "dark"})).is_err());
    }
}
