//! Native calls Tauri doesn't expose: window settings (PLAN.md D20, D35, §9.4), a startup alert (D59) and Login Items
//! (D76).

use objc2::MainThreadMarker;
use objc2_app_kit::{NSAlert, NSStatusWindowLevel, NSWindowCollectionBehavior, NSWindowStyleMask};
use objc2_foundation::NSString;
use objc2_service_management::{SMAppService, SMAppServiceStatus};
use tauri::WebviewWindow;
use tauri_nspanel::WebviewWindowExt;

use self::panel_class::OverlayPanel;

pub mod island;
pub mod modifiers;

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

/// A modal alert, for a failure the app cannot start past. Main thread only.
pub fn alert(message: &str, detail: &str) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let alert = NSAlert::new(mtm);
    alert.setMessageText(&NSString::from_str(message));
    alert.setInformativeText(&NSString::from_str(detail));
    alert.runModal();
}

/// Whether macOS opens this app at login.
pub fn opens_at_login() -> bool {
    unsafe { SMAppService::mainAppService().status() == SMAppServiceStatus::Enabled }
}

pub fn set_opens_at_login(enabled: bool) -> Result<(), String> {
    let service = unsafe { SMAppService::mainAppService() };
    let result = unsafe {
        if enabled {
            service.registerAndReturnError()
        } else {
            service.unregisterAndReturnError()
        }
    };
    result.map_err(|err| err.localizedDescription().to_string())
}
