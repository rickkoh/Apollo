//! System-wide modifier key changes, for the global shortcuts.

use std::ptr::NonNull;
use std::rc::Rc;

use block2::RcBlock;
use objc2_app_kit::{NSEvent, NSEventMask};
use objc2_core_graphics::{CGEventSource, CGEventSourceStateID, CGEventType};

/// A modifier key went down or up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModifierChange {
    pub key_code: u16,
    /// `NSEvent.modifierFlags` after the change.
    pub flags: u64,
    /// Seconds since boot.
    pub at: f64,
    /// Seconds since any key was last pressed, in any app.
    pub key_down_ago: f64,
}

fn change(event: &NSEvent) -> ModifierChange {
    ModifierChange {
        key_code: event.keyCode(),
        flags: event.modifierFlags().0 as u64,
        at: event.timestamp(),
        key_down_ago: CGEventSource::seconds_since_last_event_type(
            CGEventSourceStateID::CombinedSessionState,
            CGEventType::KeyDown,
        ),
    }
}

/// Calls `on_change` for every modifier change system-wide, for the app's lifetime. Needs no permission. Main thread
/// only.
pub fn watch(on_change: impl Fn(ModifierChange) + 'static) {
    let on_change = Rc::new(on_change);
    let global = {
        let on_change = on_change.clone();
        RcBlock::new(move |event: NonNull<NSEvent>| on_change(change(unsafe { event.as_ref() })))
    };
    let local = RcBlock::new(move |event: NonNull<NSEvent>| {
        on_change(change(unsafe { event.as_ref() }));
        event.as_ptr()
    });
    // A global monitor misses events sent to this app, so a local one covers those.
    let monitors = [
        NSEvent::addGlobalMonitorForEventsMatchingMask_handler(NSEventMask::FlagsChanged, &global),
        unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(NSEventMask::FlagsChanged, &local) },
    ];
    if monitors.iter().any(Option::is_none) {
        tracing::error!("cannot watch modifier keys; shortcuts are off");
    }
    std::mem::forget(monitors);
}
