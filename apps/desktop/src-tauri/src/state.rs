use std::path::PathBuf;
use std::sync::Mutex;

use hub::{HubHandle, RunningHub};

use crate::channel::Channel;
use crate::error::UiError;
use crate::settings::SettingsService;
use crate::updates::Updates;

pub struct AppPaths {
    pub data: PathBuf,
    pub discovery: PathBuf,
    /// The bundled marketplace holding the voice plugin (`<resource dir>/plugin`).
    pub marketplace: PathBuf,
    /// Where `cv` and `cv-panel` are installed (`~/.local/bin`).
    pub bin: PathBuf,
    /// `~/Library/Application Support`, where editors keep their settings.
    pub app_support: PathBuf,
}

/// Managed Tauri state: the running hub (absent once stopped) and the settings service.
pub struct AppState {
    pub channel: Channel,
    pub paths: AppPaths,
    pub settings: SettingsService,
    /// Stable only: Apollo Dev never updates (D56).
    pub updates: Option<Updates>,
    hub: Mutex<Option<RunningHub>>,
}

impl AppState {
    pub fn new(
        channel: Channel,
        paths: AppPaths,
        settings: SettingsService,
        updates: Option<Updates>,
        hub: RunningHub,
    ) -> Self {
        Self {
            channel,
            paths,
            settings,
            updates,
            hub: Mutex::new(Some(hub)),
        }
    }

    pub fn hub(&self) -> Result<HubHandle, UiError> {
        self.hub
            .lock()
            .expect("hub lock")
            .as_ref()
            .map(|running| running.handle.clone())
            .ok_or_else(UiError::hub_down)
    }

    pub fn take_hub(&self) -> Option<RunningHub> {
        self.hub.lock().expect("hub lock").take()
    }
}
