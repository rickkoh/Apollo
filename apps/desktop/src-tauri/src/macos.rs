//! Native window settings Tauri doesn't expose (PLAN.md D20, D35, §9.4).

use objc2_app_kit::{NSStatusWindowLevel, NSWindowCollectionBehavior, NSWindowStyleMask};
use tauri::WebviewWindow;
use tauri_nspanel::WebviewWindowExt;

use self::panel_class::OverlayPanel;

/// The macro brings its own AppKit imports, so it lives apart from ours.
mod panel_class {
    tauri_nspanel::tauri_panel! {
        panel!(OverlayPanel {
            config: {
                can_become_key_window: true,
                is_floating_panel: true
            }
        })
    }
}

fn collection_behavior() -> NSWindowCollectionBehavior {
    NSWindowCollectionBehavior::CanJoinAllSpaces
        | NSWindowCollectionBehavior::FullScreenAuxiliary
        | NSWindowCollectionBehavior::Stationary
        | NSWindowCollectionBehavior::IgnoresCycle
}

/// Turns the overlay into a non-activating panel that floats over every Space and over full-screen apps, so the app
/// behind stays active while the user dictates. Call once after building the window, and never call Tauri's
/// `set_always_on_top` or `set_visible_on_all_workspaces` afterwards: they rewrite these settings.
pub fn float_over_full_screen(window: &WebviewWindow) -> tauri::Result<()> {
    let panel = window.to_panel::<OverlayPanel>()?;
    if let Err(err) = panel.set_style_mask(NSWindowStyleMask::NonactivatingPanel) {
        tracing::error!("cannot make the overlay non-activating: {err}");
    }
    panel.set_collection_behavior(collection_behavior());
    panel.set_level(NSStatusWindowLevel as i64);
    panel.set_hides_on_deactivate(false);
    Ok(())
}
