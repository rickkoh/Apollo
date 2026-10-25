//! The island's native parts: the notch, pointer tracking, the keyboard hand-off and screen changes.

use std::ptr::NonNull;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::{AllocAnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSApplicationDidChangeScreenParametersNotification, NSEvent, NSScreen, NSTrackingArea, NSTrackingAreaOptions,
};
use objc2_foundation::{NSNotification, NSNotificationCenter, NSObject, NSObjectProtocol, NSOperationQueue, NSRect};
use tauri::{Manager, WebviewWindow};
use tauri_nspanel::{ManagerExt, PanelHandle};

const DRAWN_WIDTH: f64 = 180.0;
const DRAWN_MIN_HEIGHT: f64 = 24.0;

/// The notch the island hangs from, in Tauri's logical coordinates (origin at the primary screen's top-left).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Notch {
    pub center_x: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
    /// False when the screen has no notch and the island draws one.
    pub hardware: bool,
}

/// The notched screen's notch, else one drawn at the main screen's top centre as tall as its menu bar. Main thread
/// only.
pub fn notch() -> Option<Notch> {
    let mtm = MainThreadMarker::new()?;
    let screens = NSScreen::screens(mtm);
    let primary_height = screens.firstObject()?.frame().size.height;
    let notched = screens.iter().find(|s| s.safeAreaInsets().top > 0.0);
    let hardware = notched.is_some();
    let screen = match notched {
        Some(screen) => screen,
        None => NSScreen::mainScreen(mtm)?,
    };
    let frame = screen.frame();
    let top = primary_height - (frame.origin.y + frame.size.height);
    if hardware {
        let start = frame.origin.x + screen.auxiliaryTopLeftArea().size.width;
        let end = frame.origin.x + frame.size.width - screen.auxiliaryTopRightArea().size.width;
        return Some(Notch {
            center_x: (start + end) / 2.0,
            top,
            width: end - start,
            height: screen.safeAreaInsets().top,
            hardware,
        });
    }
    let visible = screen.visibleFrame();
    let menu_bar = frame.origin.y + frame.size.height - (visible.origin.y + visible.size.height);
    Some(Notch {
        center_x: frame.origin.x + frame.size.width / 2.0,
        top,
        width: DRAWN_WIDTH,
        height: menu_bar.max(DRAWN_MIN_HEIGHT),
        hardware,
    })
}

struct PointerIvars {
    on_crossing: Box<dyn Fn()>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "ApolloPointerWatcher"]
    #[ivars = PointerIvars]
    struct PointerWatcher;

    unsafe impl NSObjectProtocol for PointerWatcher {}

    impl PointerWatcher {
        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, _event: &NSEvent) {
            (self.ivars().on_crossing)()
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            (self.ivars().on_crossing)()
        }
    }
);

fn panel(window: &WebviewWindow) -> Option<PanelHandle<tauri::Wry>> {
    window.app_handle().get_webview_panel(window.label()).ok()
}

/// Calls `on_crossing` when the pointer may have entered or left the window, whether or not the panel is key; the web
/// view's own pointer events stop while it isn't. AppKit also sends these as the window resizes under a still pointer,
/// so the caller checks `pointer_inside`. Main thread only.
pub fn watch_pointer(window: &WebviewWindow, on_crossing: impl Fn() + 'static) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let Some(panel) = panel(window) else {
        return;
    };
    let Some(view) = panel.as_panel().contentView() else {
        return;
    };
    let watcher = PointerWatcher::alloc(mtm).set_ivars(PointerIvars {
        on_crossing: Box::new(on_crossing),
    });
    let watcher: Retained<PointerWatcher> = unsafe { msg_send![super(watcher), init] };
    let options = NSTrackingAreaOptions::MouseEnteredAndExited
        | NSTrackingAreaOptions::ActiveAlways
        | NSTrackingAreaOptions::InVisibleRect;
    let area = unsafe {
        NSTrackingArea::initWithRect_options_owner_userInfo(
            NSTrackingArea::alloc(),
            NSRect::ZERO,
            options,
            Some(&watcher),
            None,
        )
    };
    view.addTrackingArea(&area);
    // A tracking area doesn't retain its owner; the watcher lives as long as the app.
    std::mem::forget(watcher);
}

/// Whether the pointer is over the window, edges included. Main thread only.
pub fn pointer_inside(window: &WebviewWindow) -> bool {
    let Some(panel) = panel(window) else {
        return false;
    };
    let frame = panel.as_panel().frame();
    let pointer = NSEvent::mouseLocation();
    (frame.origin.x..=frame.origin.x + frame.size.width).contains(&pointer.x)
        && (frame.origin.y..=frame.origin.y + frame.size.height).contains(&pointer.y)
}

/// Takes the keyboard without activating the app, so the app behind stays frontmost.
pub fn take_keyboard(window: &WebviewWindow) {
    if let Some(panel) = panel(window) {
        panel.make_key_window();
    }
}

/// Hands the keyboard back to the frontmost app's window, with no click.
pub fn give_back_keyboard(window: &WebviewWindow) {
    if let Some(panel) = panel(window) {
        panel.resign_key_window();
    }
}

/// Calls `callback` on the main thread whenever displays are added, removed or rearranged, for as long as the app
/// runs.
pub fn on_screens_changed(callback: impl Fn() + 'static) {
    let block = RcBlock::new(move |_: NonNull<NSNotification>| callback());
    let observer = unsafe {
        NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
            Some(NSApplicationDidChangeScreenParametersNotification),
            None,
            Some(&NSOperationQueue::mainQueue()),
            &block,
        )
    };
    std::mem::forget(observer);
}
