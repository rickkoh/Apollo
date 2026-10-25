use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};
use ts_rs::TS;

use crate::channel::Channel;
use crate::events::{
    self, OVERLAY_GEOMETRY, OVERLAY_POINTER, OVERLAY_SUMMON, OVERLAY_VISIBILITY, OverlayPointer, OverlaySummon,
    OverlayVisibility,
};
use crate::macos::{self, island};
use crate::settings::OverlayLayout;
use crate::state::AppState;
use crate::tray;

pub const LABEL: &str = "overlay";

/// Whether the page was last told the pointer is over the overlay.
#[derive(Default)]
struct Pointer(AtomicBool);
const INITIAL_HEIGHT: f64 = 72.0;
const SCREEN_MARGIN: f64 = 16.0;
/// The notch Apollo Dev's island draws in its corner.
const DRAWN_NOTCH: (f64, f64) = (180.0, 32.0);

/// Where the overlay hangs, and the notch the page draws the island around.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OverlayGeometry {
    pub anchor: OverlayAnchor,
    pub notch_width: f64,
    pub notch_height: f64,
    /// The screen's own notch shows the island at rest; otherwise the page draws one.
    pub hardware_notch: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export)]
pub enum OverlayAnchor {
    Top,
    BottomLeft,
    BottomRight,
}

/// The point a frame hangs from, in logical points.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Origin {
    TopCentre { x: f64, top: f64 },
    BottomLeft { x: f64, bottom: f64 },
    BottomRight { x: f64, bottom: f64 },
}

impl Origin {
    fn top_left(self, width: f64, height: f64) -> (f64, f64) {
        match self {
            Origin::TopCentre { x, top } => (x - width / 2.0, top),
            Origin::BottomLeft { x, bottom } => (x, bottom - height),
            Origin::BottomRight { x, bottom } => (x - width, bottom - height),
        }
    }
}

fn width<M: Manager<tauri::Wry>>(manager: &M) -> f64 {
    manager.state::<AppState>().settings.get().overlay.width.into()
}

pub fn create(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let window = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("overlay.html".into()))
        .title(format!("{} Overlay", app.package_info().name))
        .inner_size(width(app), INITIAL_HEIGHT)
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
    app.manage(Pointer::default());
    let crossed = window.clone();
    island::watch_pointer(&window, move || report_pointer(&crossed));
    let screens = app.clone();
    island::on_screens_changed(move || reposition(&screens));
    frame(&window, width(app), INITIAL_HEIGHT)?;
    let scaled = app.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::ScaleFactorChanged { .. } = event {
            reposition(&scaled);
        }
    });
    Ok(window)
}

/// The island hangs from the notch, Apollo Dev's from the bottom-left corner; the classic overlay sits bottom-right,
/// Apollo Dev's bottom-left. Main thread only.
fn placement(window: &WebviewWindow) -> tauri::Result<Option<(Origin, OverlayGeometry)>> {
    let state = window.state::<AppState>();
    let layout = state.settings.get().overlay.layout;
    if layout == OverlayLayout::Island && state.channel == Channel::Stable {
        return Ok(island::notch().map(|notch| {
            let origin = Origin::TopCentre {
                x: notch.center_x,
                top: notch.top,
            };
            let geometry = OverlayGeometry {
                anchor: OverlayAnchor::Top,
                notch_width: notch.width,
                notch_height: notch.height,
                hardware_notch: notch.hardware,
            };
            (origin, geometry)
        }));
    }
    let Some(monitor) = window.primary_monitor()? else {
        return Ok(None);
    };
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let left = area.position.x as f64 / scale + SCREEN_MARGIN;
    let right = (area.position.x as f64 + area.size.width as f64) / scale - SCREEN_MARGIN;
    let bottom = (area.position.y as f64 + area.size.height as f64) / scale - SCREEN_MARGIN;
    let (origin, anchor) = match state.channel {
        Channel::Stable => (Origin::BottomRight { x: right, bottom }, OverlayAnchor::BottomRight),
        Channel::Dev => (Origin::BottomLeft { x: left, bottom }, OverlayAnchor::BottomLeft),
    };
    let (notch_width, notch_height) = match layout {
        OverlayLayout::Island => DRAWN_NOTCH,
        OverlayLayout::Classic => (0.0, 0.0),
    };
    let geometry = OverlayGeometry {
        anchor,
        notch_width,
        notch_height,
        hardware_notch: false,
    };
    Ok(Some((origin, geometry)))
}

pub fn geometry(window: &WebviewWindow) -> tauri::Result<Option<OverlayGeometry>> {
    Ok(placement(window)?.map(|(_, geometry)| geometry))
}

/// Sizes the window to `width` × `height` and hangs it from its anchor.
pub fn frame(window: &WebviewWindow, width: f64, height: f64) -> tauri::Result<()> {
    let Some((origin, _)) = placement(window)? else {
        return Ok(());
    };
    let (x, y) = origin.top_left(width, height);
    window.set_size(LogicalSize::new(width, height))?;
    window.set_position(LogicalPosition::new(x, y))?;
    report_pointer(window);
    Ok(())
}

/// Tells the page when the pointer has really crossed the overlay's edge, after a pointer event or a resize.
fn report_pointer(window: &WebviewWindow) {
    let inside = island::pointer_inside(window);
    if window.state::<Pointer>().0.swap(inside, Ordering::Relaxed) != inside {
        events::emit(window.app_handle(), OVERLAY_POINTER, OverlayPointer { inside });
    }
}

fn current_size(window: &WebviewWindow) -> (f64, f64) {
    let scale = window.scale_factor().unwrap_or(1.0);
    window.inner_size().map_or((width(window), INITIAL_HEIGHT), |size| {
        (size.width as f64 / scale, size.height as f64 / scale)
    })
}

/// Re-hangs the overlay at its current size and sends the page its geometry, after a screen or layout change.
pub fn reposition(app: &AppHandle) {
    let handle = app.clone();
    let queued = app.run_on_main_thread(move || {
        let Some(window) = handle.get_webview_window(LABEL) else {
            return;
        };
        let (width, height) = current_size(&window);
        if let Err(err) = frame(&window, width, height) {
            tracing::error!("re-placing the overlay failed: {err}");
        }
        if let Ok(Some(geometry)) = geometry(&window) {
            events::emit(&handle, OVERLAY_GEOMETRY, geometry);
        }
    });
    if let Err(err) = queued {
        tracing::error!("re-placing the overlay failed: {err}");
    }
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
        let (width, height) = current_size(&window);
        frame(&window, width, height)?;
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

/// Shows the overlay and has the page open it with the keyboard in the text box.
pub fn summon(app: &AppHandle) -> tauri::Result<()> {
    set_visible(app, true)?;
    events::emit(app, OVERLAY_SUMMON, OverlaySummon { released: false });
    Ok(())
}

/// The key held after the summoning tap came up.
pub fn summon_released(app: &AppHandle) {
    events::emit(app, OVERLAY_SUMMON, OverlaySummon { released: true });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_top_centre_frame_hangs_centred_from_the_top_edge() {
        let notch = Origin::TopCentre { x: 901.5, top: 0.0 };
        assert_eq!(notch.top_left(221.0, 38.0), (791.0, 0.0));
        assert_eq!(notch.top_left(420.0, 200.0), (691.5, 0.0));
    }

    #[test]
    fn corner_frames_grow_up_and_inward() {
        let left = Origin::BottomLeft {
            x: 16.0,
            bottom: 1000.0,
        };
        assert_eq!(left.top_left(420.0, 100.0), (16.0, 900.0));
        let right = Origin::BottomRight {
            x: 1784.0,
            bottom: 1000.0,
        };
        assert_eq!(right.top_left(420.0, 100.0), (1364.0, 900.0));
    }
}
