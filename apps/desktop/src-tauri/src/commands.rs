use hub::{HubStatus, Message, MessageId, SessionId, SessionView};
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, State, WebviewWindow};
use ts_rs::TS;

use crate::channel::Channel;
use crate::cli::{self, CliStatus, Launcher, Route};
use crate::error::UiError;
use crate::events::{self, SETTINGS_CHANGED};
use crate::ide_panel::{IdePanelLauncher, IdePanelStatus};
use crate::login_shell;
use crate::macos;
use crate::plugin_cli::{BundledPlugin, ClaudeCli, PluginStatus};
use crate::settings::{LIMITS, Settings, SettingsLimits};
use crate::state::AppState;
use crate::updates::{UpdateStatus, Updates};
use crate::windows::overlay::{self, OverlayGeometry};
use crate::windows::panel;

type CommandResult<T> = Result<T, UiError>;

#[tauri::command]
pub async fn hub_status(state: State<'_, AppState>) -> CommandResult<HubStatus> {
    Ok(state.hub()?.status().await?)
}

#[tauri::command]
pub async fn list_sessions(state: State<'_, AppState>) -> CommandResult<Vec<SessionView>> {
    Ok(state.hub()?.sessions().await?)
}

#[tauri::command]
pub async fn set_target(state: State<'_, AppState>, id: SessionId) -> CommandResult<()> {
    Ok(state.hub()?.set_target(id).await?)
}

#[tauri::command]
pub async fn send(state: State<'_, AppState>, text: String) -> CommandResult<Message> {
    Ok(state.hub()?.send(text).await?)
}

#[tauri::command]
pub async fn history(
    state: State<'_, AppState>,
    session: SessionId,
    before: Option<MessageId>,
    limit: u32,
) -> CommandResult<Vec<Message>> {
    Ok(state.hub()?.history(session, before, limit).await?)
}

#[tauri::command]
pub async fn mark_read(state: State<'_, AppState>, session: SessionId, upto: MessageId) -> CommandResult<()> {
    Ok(state.hub()?.mark_read(session, upto).await?)
}

#[tauri::command]
pub async fn recent_messages(state: State<'_, AppState>, limit: u32) -> CommandResult<Vec<Message>> {
    Ok(state.hub()?.recent(limit).await?)
}

#[tauri::command]
pub async fn rename_session(state: State<'_, AppState>, id: SessionId, title: Option<String>) -> CommandResult<()> {
    Ok(state.hub()?.rename(id, title).await?)
}

#[tauri::command]
pub async fn forget_session(state: State<'_, AppState>, id: SessionId) -> CommandResult<()> {
    Ok(state.hub()?.forget(id).await?)
}

#[tauri::command]
pub async fn clear_history(state: State<'_, AppState>) -> CommandResult<()> {
    Ok(state.hub()?.clear_history().await?)
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.get()
}

#[tauri::command]
pub async fn update_settings(app: AppHandle, state: State<'_, AppState>, patch: Value) -> CommandResult<Settings> {
    let before = state.settings.get().overlay;
    let settings = state.settings.update(patch)?;
    if settings.overlay.width != before.width || settings.overlay.layout != before.layout {
        overlay::reposition(&app);
    }
    if let Ok(hub) = state.hub() {
        hub.set_target_new_sessions(settings.sessions.target_new).await?;
    }
    events::emit(&app, SETTINGS_CHANGED, settings.clone());
    Ok(settings)
}

#[tauri::command]
pub fn settings_limits() -> SettingsLimits {
    LIMITS
}

#[tauri::command]
pub fn overlay_frame(window: WebviewWindow, width: f64, height: f64) -> CommandResult<()> {
    overlay::frame(&window, width.clamp(1.0, 2000.0), height.clamp(1.0, 2000.0)).map_err(UiError::internal)
}

#[tauri::command]
pub fn overlay_geometry(window: WebviewWindow) -> CommandResult<OverlayGeometry> {
    overlay::geometry(&window)
        .map_err(UiError::internal)?
        .ok_or_else(|| UiError::internal("no screen to place the overlay on"))
}

#[tauri::command]
pub fn overlay_keyboard(window: WebviewWindow, take: bool) {
    if take {
        macos::island::take_keyboard(&window);
    } else {
        macos::island::give_back_keyboard(&window);
    }
}

#[tauri::command]
pub fn overlay_visible(app: AppHandle) -> bool {
    overlay::is_visible(&app)
}

#[tauri::command]
pub fn set_overlay_visible(app: AppHandle, visible: bool) -> CommandResult<()> {
    overlay::set_visible(&app, visible).map_err(UiError::internal)
}

#[tauri::command]
pub fn open_panel(app: AppHandle, route: Option<String>) -> CommandResult<()> {
    panel::open(&app, route.as_deref()).map_err(UiError::internal)
}

fn launcher(state: &AppState) -> Launcher {
    Launcher {
        bin_dir: state.paths.bin.clone(),
        approval_path: cli::APPROVAL_PATH.into(),
        discovery: state.paths.discovery.clone(),
        route: match state.channel {
            Channel::Stable => Route::Installed,
            Channel::Dev => Route::WorkingCopy {
                plugin_dir: cli::working_copy_plugin(),
            },
        },
    }
}

fn ide_panel_launcher(state: &AppState) -> IdePanelLauncher {
    IdePanelLauncher {
        bin_dir: state.paths.bin.clone(),
        bridge: state.paths.marketplace.join("voice/dist/panel-bridge.js"),
        discovery: state.paths.discovery.clone(),
        app_support: state.paths.app_support.clone(),
    }
}

fn bundled_plugin(state: &AppState) -> BundledPlugin {
    BundledPlugin {
        marketplace: state.paths.marketplace.clone(),
    }
}

#[tauri::command]
pub async fn cli_status(state: State<'_, AppState>) -> CommandResult<CliStatus> {
    let launcher = launcher(&state);
    tauri::async_runtime::spawn_blocking(move || launcher.status(login_shell::var("PATH").as_deref()))
        .await
        .map_err(UiError::internal)
}

/// Installs the plugin first, so a failure leaves the previous launcher in place. Apollo Dev installs only `cv-dev`
/// (D75).
#[tauri::command]
pub async fn install_cli(state: State<'_, AppState>) -> CommandResult<CliStatus> {
    let launcher = launcher(&state);
    let stable = (state.channel == Channel::Stable).then(|| (ide_panel_launcher(&state), bundled_plugin(&state)));
    tauri::async_runtime::spawn_blocking(move || {
        let path = login_shell::var("PATH");
        if let Some((ide_panel, plugin)) = stable {
            plugin.install(&ClaudeCli::new(path.clone()))?;
            ide_panel.install()?;
        }
        launcher.install(path.as_deref())
    })
    .await
    .map_err(UiError::internal)?
    .map_err(UiError::from)
}

#[tauri::command]
pub fn ide_panel_status(state: State<'_, AppState>) -> IdePanelStatus {
    ide_panel_launcher(&state).status()
}

/// Runs `claude` twice, so only Settings asks.
#[tauri::command]
pub async fn plugin_status(state: State<'_, AppState>) -> CommandResult<PluginStatus> {
    let plugin = bundled_plugin(&state);
    tauri::async_runtime::spawn_blocking(move || plugin.status(&ClaudeCli::new(login_shell::var("PATH"))))
        .await
        .map_err(UiError::internal)?
        .map_err(UiError::from)
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AboutInfo {
    pub version: String,
    pub channel: Channel,
    /// The launcher this app installs: `cv`, or `cv-dev` for Apollo Dev.
    pub command: String,
    pub commit: String,
    pub data_dir: String,
}

#[tauri::command]
pub fn about(app: AppHandle, state: State<'_, AppState>) -> AboutInfo {
    AboutInfo {
        version: app.package_info().version.to_string(),
        channel: state.channel,
        command: launcher(&state).route.command().into(),
        commit: env!("GIT_COMMIT").into(),
        data_dir: state.paths.data.to_string_lossy().into_owned(),
    }
}

fn updates(state: &AppState) -> CommandResult<&Updates> {
    state
        .updates
        .as_ref()
        .ok_or_else(|| UiError::internal("Apollo Dev doesn't update"))
}

/// `None` in Apollo Dev, which never updates.
#[tauri::command]
pub fn update_status(state: State<'_, AppState>) -> Option<UpdateStatus> {
    state.updates.as_ref().map(Updates::status)
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle, state: State<'_, AppState>) -> CommandResult<UpdateStatus> {
    Ok(updates(&state)?.check(&app).await)
}

/// Relaunches on success, so it returns only on failure.
#[tauri::command]
pub async fn install_update(app: AppHandle, state: State<'_, AppState>) -> CommandResult<()> {
    updates(&state)?.install(&app).await.map_err(UiError::internal)
}

/// `None` in Apollo Dev, which doesn't offer it (D76).
#[tauri::command]
pub fn open_at_login(state: State<'_, AppState>) -> Option<bool> {
    (state.channel == Channel::Stable).then(macos::opens_at_login)
}

#[tauri::command]
pub fn set_open_at_login(enabled: bool) -> CommandResult<bool> {
    macos::set_opens_at_login(enabled).map_err(UiError::internal)?;
    Ok(macos::opens_at_login())
}

#[tauri::command]
pub fn reveal_data_folder(state: State<'_, AppState>) -> CommandResult<()> {
    std::process::Command::new("open")
        .arg(&state.paths.data)
        .status()
        .map_err(UiError::internal)
        .map(|_| ())
}
