use std::path::PathBuf;
use std::sync::Mutex;

use hub::{HubHandle, RunningHub};

use crate::error::UiError;
use crate::settings::SettingsService;

pub struct AppPaths {
    pub data: PathBuf,
    pub discovery: PathBuf,
    /// The bundled marketplace holding the voice plugin (`<resource dir>/plugin`).
    pub marketplace: PathBuf,
    /// Where `cv` is installed (`~/.local/bin`).
    pub bin: PathBuf,
}

/// Managed Tauri state: the running hub (absent once stopped) and the settings service.
pub struct AppState {
    pub paths: AppPaths,
    pub settings: SettingsService,
    hub: Mutex<Option<RunningHub>>,
}

impl AppState {
    pub fn new(paths: AppPaths, settings: SettingsService, hub: RunningHub) -> Self {
        Self {
            paths,
            settings,
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
