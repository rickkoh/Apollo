mod channel;
mod cli;
mod commands;
mod error;
mod events;
mod macos;
mod menu;
mod panel;
mod plugin_cli;
mod presence;
mod settings;
mod state;
mod tray;
mod updates;
mod windows;

use std::sync::Arc;

use hub::StoreError;
use hub::store::{SqliteStore, Store};
use hub::{Hub, HubConfig, Token, discovery};
use tauri::{ActivationPolicy, AppHandle, Manager, RunEvent};
use tauri_plugin_log::{Target, TargetKind};
use time::{Duration, OffsetDateTime};

use crate::channel::Channel;
use crate::events::{HUB_STATE, HubState, TauriEvents};
use crate::settings::SettingsService;
use crate::state::{AppPaths, AppState};
use crate::updates::Updates;

/// Startup, PLAN.md §3.2.
fn start(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let channel = Channel::of(&app.config().identifier);
    if cfg!(debug_assertions) && channel == Channel::Stable {
        return Err("a debug build runs as Apollo Dev, with its own data and launcher: `npm run app`".into());
    }
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
    let home = app.path().home_dir()?;
    let paths = AppPaths {
        discovery: data.join("hub.json"),
        marketplace: app.path().resource_dir()?.join("plugin"),
        bin: home.join(".local/bin"),
        app_support: home.join("Library/Application Support"),
        data,
    };

    let settings = SettingsService::load(paths.data.join("settings.json"));
    let mut store = match SqliteStore::open(&paths.data.join("voice.db")) {
        Err(StoreError::NewerSchema { found, known }) => {
            let name = &app.package_info().name;
            macos::alert(
                &format!("This {name} is older than its data"),
                &format!(
                    "A newer {name} updated the history database (schema {found}; this version knows {known}). \
                     Install the latest {name}, or move {} aside to start fresh.",
                    paths.data.display()
                ),
            );
            std::process::exit(1);
        }
        store => store?,
    };
    let retention = Duration::days(settings.get().history.retention_days.into());
    let purged = store.purge_before(OffsetDateTime::now_utc() - retention)?;
    tracing::info!(purged, "history retention applied");

    app.set_activation_policy(ActivationPolicy::Accessory)?;
    app.set_menu(menu::build(app)?)?;
    tray::build(app)?;

    let token = Token::generate();
    let events = Arc::new(TauriEvents::new(app.clone()));
    let running = tauri::async_runtime::block_on(Hub::start(
        HubConfig::new(token.clone()),
        Box::new(store),
        Some(presence::claude_sessions(home)),
        events,
    ))?;
    discovery::write(&paths.discovery, running.port, &token)?;
    let visible_at_launch = settings.get().overlay.visible_at_launch;
    let updates = (channel == Channel::Stable).then(Updates::new);
    app.manage(AppState::new(channel, paths, settings, updates, running));

    windows::overlay::create(app)?;
    windows::overlay::set_visible(app, visible_at_launch)?;
    if channel == Channel::Stable {
        check_for_updates_every(app.clone());
    }
    Ok(())
}

/// At launch, then every `CHECK_EVERY` (D56).
fn check_for_updates_every(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            if let Some(updates) = &app.state::<AppState>().updates {
                updates.check(&app).await;
            }
            tokio::time::sleep(updates::CHECK_EVERY).await;
        }
    });
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
    let context = tauri::generate_context!();
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Err(err) = windows::panel::open(app, None) {
                tracing::error!("opening the control panel failed: {err}");
            }
        }))
        .plugin(tauri_nspanel::init());
    if Channel::of(&context.config().identifier) == Channel::Stable {
        builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    }
    builder
        .setup(|app| start(app.handle()))
        .invoke_handler(tauri::generate_handler![
            commands::hub_status,
            commands::list_sessions,
            commands::set_target,
            commands::send,
            commands::history,
            commands::mark_read,
            commands::recent_messages,
            commands::rename_session,
            commands::forget_session,
            commands::clear_history,
            commands::get_settings,
            commands::update_settings,
            commands::overlay_resize,
            commands::overlay_visible,
            commands::set_overlay_visible,
            commands::open_panel,
            commands::cli_status,
            commands::install_cli,
            commands::plugin_status,
            commands::panel_status,
            commands::about,
            commands::reveal_data_folder,
            commands::update_status,
            commands::check_for_updates,
            commands::install_update,
            commands::open_at_login,
            commands::set_open_at_login,
        ])
        .build(context)
        .expect("error while building Voice Bridge")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                stop(app);
            }
        });
}
