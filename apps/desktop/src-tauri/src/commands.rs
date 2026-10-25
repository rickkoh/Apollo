use hub::{HubStatus, Message, MessageId, SessionId, SessionView};
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, State, WebviewWindow};
use ts_rs::TS;

use crate::cli::{self, CliStatus, Launcher};
use crate::error::UiError;
use crate::events::{self, SETTINGS_CHANGED};
use crate::plugin_cli::{BundledPlugin, ClaudeCli, PluginStatus};
use crate::settings::Settings;
use crate::state::AppState;
use crate::windows::{overlay, panel};

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
pub fn update_settings(app: AppHandle, state: State<'_, AppState>, patch: Value) -> CommandResult<Settings> {
    let settings = state.settings.update(patch)?;
    events::emit(&app, SETTINGS_CHANGED, settings.clone());
    Ok(settings)
}

#[tauri::command]
pub fn overlay_resize(window: WebviewWindow, height: f64) -> CommandResult<()> {
    overlay::place(&window, height.clamp(1.0, 2000.0)).map_err(UiError::internal)
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
    tauri::async_runtime::spawn_blocking(move || launcher.status(cli::login_shell_var("PATH").as_deref()))
        .await
        .map_err(UiError::internal)
}

/// Installs the plugin first, so a failure leaves the previous launcher in place.
#[tauri::command]
pub async fn install_cli(state: State<'_, AppState>) -> CommandResult<CliStatus> {
    let launcher = launcher(&state);
    let plugin = bundled_plugin(&state);
    tauri::async_runtime::spawn_blocking(move || {
        let path = cli::login_shell_var("PATH");
        plugin.install(&ClaudeCli::new(path.clone()))?;
        launcher.install(path.as_deref())
    })
    .await
    .map_err(UiError::internal)?
    .map_err(UiError::from)
}

/// Runs `claude` twice, so only Settings asks.
#[tauri::command]
pub async fn plugin_status(state: State<'_, AppState>) -> CommandResult<PluginStatus> {
    let plugin = bundled_plugin(&state);
    tauri::async_runtime::spawn_blocking(move || plugin.status(&ClaudeCli::new(cli::login_shell_var("PATH"))))
        .await
        .map_err(UiError::internal)?
        .map_err(UiError::from)
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AboutInfo {
    pub version: String,
    pub data_dir: String,
}

#[tauri::command]
pub fn about(app: AppHandle, state: State<'_, AppState>) -> AboutInfo {
    AboutInfo {
        version: app.package_info().version.to_string(),
        data_dir: state.paths.data.to_string_lossy().into_owned(),
    }
}

#[tauri::command]
pub fn reveal_data_folder(state: State<'_, AppState>) -> CommandResult<()> {
    std::process::Command::new("open")
        .arg(&state.paths.data)
        .status()
        .map_err(UiError::internal)
        .map(|_| ())
}
