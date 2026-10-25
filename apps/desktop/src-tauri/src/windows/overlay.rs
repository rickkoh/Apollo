use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

use crate::events::{self, OVERLAY_VISIBILITY, OverlayVisibility};
use crate::macos;
use crate::tray;

pub const LABEL: &str = "overlay";
/// Logical width, including the transparent shadow margin the page draws inside it.
pub const WIDTH: f64 = 420.0;
const INITIAL_HEIGHT: f64 = 72.0;
const SCREEN_MARGIN: f64 = 16.0;

pub fn create(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let window = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("overlay.html".into()))
        .title("Voice Bridge Overlay")
        .inner_size(WIDTH, INITIAL_HEIGHT)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .visible_on_all_workspaces(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(false)
        .build()?;
    macos::float_over_full_screen(&window)?;
    place(&window, INITIAL_HEIGHT)?;
    let handle = window.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::ScaleFactorChanged { .. } = event {
            let _ = place(&handle, current_height(&handle));
        }
    });
    Ok(window)
}

fn current_height(window: &WebviewWindow) -> f64 {
    let scale = window.scale_factor().unwrap_or(1.0);
    window
        .inner_size()
        .map_or(INITIAL_HEIGHT, |size| size.height as f64 / scale)
}

/// Sizes the window to `height` and anchors its bottom-right corner to the primary display's work area.
pub fn place(window: &WebviewWindow, height: f64) -> tauri::Result<()> {
    let Some(monitor) = window.primary_monitor()? else {
        return Ok(());
    };
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let right = (area.position.x as f64 + area.size.width as f64) / scale;
    let bottom = (area.position.y as f64 + area.size.height as f64) / scale;
    window.set_size(LogicalSize::new(WIDTH, height))?;
    window.set_position(LogicalPosition::new(
        right - SCREEN_MARGIN - WIDTH,
        bottom - SCREEN_MARGIN - height,
    ))
}

pub fn is_visible(app: &AppHandle) -> bool {
    app.get_webview_window(LABEL)
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false)
}

pub fn set_visible(app: &AppHandle, visible: bool) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(LABEL) else {
        return Ok(());
    };
    if visible {
        place(&window, current_height(&window))?;
        window.show()?;
    } else {
        window.hide()?;
    }
    tray::show_overlay_state(app, visible);
    events::emit(app, OVERLAY_VISIBILITY, OverlayVisibility { visible });
    Ok(())
}

pub fn toggle(app: &AppHandle) -> tauri::Result<()> {
    set_visible(app, !is_visible(app))
}
