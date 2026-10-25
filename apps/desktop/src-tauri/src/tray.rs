use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Wry};

use crate::windows::{overlay, panel};

const OPEN_PANEL: &str = "open-panel";
const TOGGLE_OVERLAY: &str = "toggle-overlay";
const UPDATE: &str = "update";
const STOP_HUB: &str = "stop-hub";
const QUIT: &str = "quit";

/// Menu items whose text follows app state; `update` is in the menu only while an update is available.
struct TrayItems {
    menu: Menu<Wry>,
    toggle: MenuItem<Wry>,
    target: MenuItem<Wry>,
    update: MenuItem<Wry>,
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let name = &app.package_info().name;
    let open = MenuItem::with_id(app, OPEN_PANEL, format!("Open {name}"), true, None::<&str>)?;
    let toggle = MenuItem::with_id(app, TOGGLE_OVERLAY, "Show Overlay", true, None::<&str>)?;
    let target = MenuItem::new(app, "Target: none", false, None::<&str>)?;
    let update = MenuItem::with_id(app, UPDATE, "Update", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, QUIT, format!("Quit {name}"), true, Some("CmdOrCtrl+Q"))?;
    let menu = Menu::with_items(app, &[&open, &toggle, &target, &PredefinedMenuItem::separator(app)?])?;
    if cfg!(debug_assertions) {
        menu.append(&MenuItem::with_id(
            app,
            STOP_HUB,
            "Stop Hub (debug)",
            true,
            None::<&str>,
        )?)?;
    }
    menu.append(&quit)?;

    TrayIconBuilder::with_id("main")
        .icon(Image::from_bytes(include_bytes!("../icons/tray@2x.png"))?)
        .icon_as_template(true)
        .tooltip(name)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| {
            let result = match event.id().as_ref() {
                OPEN_PANEL => panel::open(app, None),
                UPDATE => panel::open(app, Some("/settings")),
                TOGGLE_OVERLAY => overlay::toggle(app),
                STOP_HUB => {
                    crate::stop_hub(app);
                    Ok(())
                }
                QUIT => {
                    app.exit(0);
                    Ok(())
                }
                _ => Ok(()),
            };
            if let Err(err) = result {
                tracing::error!("menu action failed: {err}");
            }
        })
        .build(app)?;
    app.manage(TrayItems {
        menu: menu.clone(),
        toggle,
        target,
        update,
    });
    Ok(())
}

pub fn show_overlay_state(app: &AppHandle, visible: bool) {
    if let Some(items) = app.try_state::<TrayItems>() {
        let _ = items
            .toggle
            .set_text(if visible { "Hide Overlay" } else { "Show Overlay" });
    }
}

/// `target` is the target's title and whether it is connected.
pub fn show_target(app: &AppHandle, target: Option<(String, bool)>) {
    if let Some(items) = app.try_state::<TrayItems>() {
        let text = match target {
            Some((title, true)) => format!("Target: {title}"),
            Some((title, false)) => format!("Target: {title} (not connected)"),
            None => "Target: none".to_owned(),
        };
        let _ = items.target.set_text(text);
    }
}

/// Shows "Update to vX.Y.Z…" at the top of the menu while `version` is available; it opens Settings, where the notes
/// and the install button are (D56).
pub fn show_update(app: &AppHandle, version: Option<&str>) {
    let Some(items) = app.try_state::<TrayItems>() else {
        return;
    };
    let shown = items.menu.get(UPDATE).is_some();
    let result = match version {
        Some(version) => items.update.set_text(format!("Update to v{version}…")).and_then(|()| {
            if shown {
                Ok(())
            } else {
                items.menu.insert(&items.update, 0)
            }
        }),
        None if shown => items.menu.remove(&items.update),
        None => Ok(()),
    };
    if let Err(err) = result {
        tracing::warn!("updating the menu's update item failed: {err}");
    }
}
