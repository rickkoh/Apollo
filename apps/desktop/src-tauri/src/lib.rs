mod commands;
mod error;
mod events;
mod macos;
mod menu;
mod settings;
mod state;
mod tray;
mod windows;

use std::sync::Arc;

use hub::store::{SqliteStore, Store};
use hub::{Hub, HubConfig, Token, discovery};
use tauri::{ActivationPolicy, AppHandle, Manager, RunEvent};
use tauri_plugin_log::{Target, TargetKind};
use time::{Duration, OffsetDateTime};

use crate::events::{HUB_STATE, HubState, TauriEvents};
use crate::settings::SettingsService;
use crate::state::{AppPaths, AppState};

/// Startup, PLAN.md §3.2.
fn start(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let data = app.path().app_data_dir()?;
    std::fs::create_dir_all(&data)?;
    app.plugin(
        tauri_plugin_log::Builder::new()
            .targets([
                Target::new(TargetKind::Folder {
                    path: data.join("logs"),
                    file_name: Some("voice-bridge".into()),
                }),
                Target::new(TargetKind::Stderr),
            ])
            .level(log::LevelFilter::Info)
            .build(),
    )?;
    let paths = AppPaths {
        discovery: data.join("hub.json"),
        data,
    };

    let settings = SettingsService::load(paths.data.join("settings.json"));
    let mut store = SqliteStore::open(&paths.data.join("voice.db"))?;
    let retention = Duration::days(settings.get().history.retention_days.into());
    let purged = store.purge_before(OffsetDateTime::now_utc() - retention)?;
    tracing::info!(purged, "history retention applied");

    app.set_activation_policy(ActivationPolicy::Accessory)?;
    app.set_menu(menu::build(app)?)?;
    tray::build(app)?;

    let token = Token::generate();
    let events = Arc::new(TauriEvents::new(app.clone()));
    let running = tauri::async_runtime::block_on(Hub::start(HubConfig::new(token.clone()), Box::new(store), events))?;
    discovery::write(&paths.discovery, running.port, &token)?;
    let visible_at_launch = settings.get().overlay.visible_at_launch;
    app.manage(AppState::new(paths, settings, running));

    windows::overlay::create(app)?;
    windows::overlay::set_visible(app, visible_at_launch)?;
    Ok(())
}

/// Shutdown, PLAN.md §3.3.
fn stop(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    if let Some(running) = state.take_hub() {
        tauri::async_runtime::block_on(running.shutdown());
    }
    if let Err(err) = discovery::remove(&state.paths.discovery) {
        tracing::warn!("removing the discovery file failed: {err}");
    }
}

/// Debug builds only (D29): stops the hub while the app keeps running, so the hub-down state can be observed.
pub(crate) fn stop_hub(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        if let Some(running) = state.take_hub() {
            running.shutdown().await;
        }
        let _ = discovery::remove(&state.paths.discovery);
        events::emit(&app, HUB_STATE, HubState { running: false });
    });
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Err(err) = windows::panel::open(app, None) {
                tracing::error!("opening the control panel failed: {err}");
            }
        }))
        .plugin(tauri_nspanel::init())
        .setup(|app| start(app.handle()))
        .invoke_handler(tauri::generate_handler![
            commands::hub_status,
            commands::list_sessions,
            commands::set_target,
            commands::send,
            commands::history,
            commands::mark_read,
            commands::rename_session,
            commands::forget_session,
            commands::clear_history,
            commands::get_settings,
            commands::update_settings,
            commands::overlay_resize,
            commands::overlay_visible,
            commands::set_overlay_visible,
            commands::open_panel,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Voice Bridge")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                stop(app);
            }
        });
}
