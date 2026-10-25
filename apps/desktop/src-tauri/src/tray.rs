use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Wry};

use crate::windows::{overlay, panel};

const OPEN_PANEL: &str = "open-panel";
const TOGGLE_OVERLAY: &str = "toggle-overlay";
const STOP_HUB: &str = "stop-hub";
const QUIT: &str = "quit";

/// Menu items whose text follows app state.
struct TrayItems {
    toggle: MenuItem<Wry>,
    target: MenuItem<Wry>,
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, OPEN_PANEL, "Open Apollo", true, None::<&str>)?;
    let toggle = MenuItem::with_id(app, TOGGLE_OVERLAY, "Show Overlay", true, None::<&str>)?;
    let target = MenuItem::new(app, "Target: none", false, None::<&str>)?;
    let quit = MenuItem::with_id(app, QUIT, "Quit Voice Bridge", true, Some("CmdOrCtrl+Q"))?;
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
        .tooltip("Voice Bridge")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| {
            let result = match event.id().as_ref() {
                OPEN_PANEL => panel::open(app, None),
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
    app.manage(TrayItems { toggle, target });
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
