//! Global shortcuts: a double tap of one modifier key, fired on the second press (PLAN.md R9).

use std::cell::RefCell;

use tauri::{AppHandle, Manager};

use crate::macos::modifiers::{self, ModifierChange};
use crate::settings::{ModifierKey, ShortcutSettings};
use crate::state::AppState;
use crate::windows::overlay;

/// Longest first tap, in seconds.
const TAP: f64 = 0.3;
/// Longest pause between the taps, in seconds.
const GAP: f64 = 0.4;

/// Device-dependent modifier bits of `NSEvent.modifierFlags`, one per physical key, plus Fn.
const HELD_MASK: u64 = 0x1 | 0x2 | 0x4 | 0x8 | 0x10 | 0x20 | 0x40 | 0x2000 | 0x80_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gesture {
    /// The second press of a double tap.
    Pressed(ModifierKey),
    /// That press came up.
    Released(ModifierKey),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    FocusInput,
    ToggleOverlay,
}

impl ModifierKey {
    fn from_key_code(code: u16) -> Option<Self> {
        Some(match code {
            58 => Self::LeftOption,
            61 => Self::RightOption,
            55 => Self::LeftCommand,
            54 => Self::RightCommand,
            59 => Self::LeftControl,
            62 => Self::RightControl,
            _ => return None,
        })
    }

    fn device_bit(self) -> u64 {
        match self {
            Self::LeftControl => 0x1,
            Self::LeftCommand => 0x8,
            Self::RightCommand => 0x10,
            Self::LeftOption => 0x20,
            Self::RightOption => 0x40,
            Self::RightControl => 0x2000,
        }
    }
}

impl ShortcutSettings {
    fn action_for(&self, key: ModifierKey) -> Option<Action> {
        if self.focus_input == Some(key) {
            Some(Action::FocusInput)
        } else if self.toggle_overlay == Some(key) {
            Some(Action::ToggleOverlay)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
enum Phase {
    #[default]
    Idle,
    Pressed {
        key: ModifierKey,
        down_at: f64,
    },
    Tapped {
        key: ModifierKey,
        down_at: f64,
        up_at: f64,
    },
    Holding {
        key: ModifierKey,
    },
}

/// Turns modifier changes into double taps and the release of their second press. A tap with any other modifier held,
/// or with a key typed since the first press, starts over.
#[derive(Debug, Default)]
pub struct Detector {
    phase: Phase,
}

impl Detector {
    pub fn on(&mut self, change: ModifierChange) -> Option<Gesture> {
        if let Phase::Holding { key } = self.phase {
            if change.flags & key.device_bit() != 0 {
                return None;
            }
            self.phase = Phase::Idle;
            return Some(Gesture::Released(key));
        }
        let held = change.flags & HELD_MASK;
        let (next, fired) = match ModifierKey::from_key_code(change.key_code) {
            Some(key) if held == key.device_bit() => match self.phase {
                Phase::Tapped { key: k, down_at, up_at }
                    if k == key && change.at - up_at <= GAP && change.key_down_ago > change.at - down_at =>
                {
                    (Phase::Holding { key }, Some(Gesture::Pressed(key)))
                }
                _ => (
                    Phase::Pressed {
                        key,
                        down_at: change.at,
                    },
                    None,
                ),
            },
            Some(key) if held == 0 => match self.phase {
                Phase::Pressed { key: k, down_at } if k == key && change.at - down_at <= TAP => (
                    Phase::Tapped {
                        key,
                        down_at,
                        up_at: change.at,
                    },
                    None,
                ),
                _ => (Phase::Idle, None),
            },
            _ => (Phase::Idle, None),
        };
        self.phase = next;
        fired
    }
}

/// Watches modifier keys for the app's lifetime, reading the bindings from settings on every gesture.
pub fn watch(app: &AppHandle) {
    let app = app.clone();
    let detector = RefCell::new(Detector::default());
    modifiers::watch(move |change| {
        let Some(gesture) = detector.borrow_mut().on(change) else {
            return;
        };
        let shortcuts = app.state::<AppState>().settings.get().shortcuts;
        let result = match gesture {
            Gesture::Pressed(key) => match shortcuts.action_for(key) {
                Some(Action::FocusInput) => overlay::summon(&app),
                Some(Action::ToggleOverlay) => overlay::toggle(&app),
                None => Ok(()),
            },
            Gesture::Released(key) => {
                if shortcuts.action_for(key) == Some(Action::FocusInput) {
                    overlay::summon_released(&app);
                }
                Ok(())
            }
        };
        if let Err(err) = result {
            tracing::error!("running the {gesture:?} shortcut failed: {err}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const RIGHT_OPTION: u16 = 61;
    const LEFT_OPTION: u16 = 58;
    const RIGHT_OPTION_BIT: u64 = 0x40;
    const SHIFT_BIT: u64 = 0x2;
    const NO_TYPING: f64 = 100.0;

    fn change(key_code: u16, flags: u64, at: f64) -> ModifierChange {
        ModifierChange {
            key_code,
            flags,
            at,
            key_down_ago: NO_TYPING,
        }
    }

    fn run(changes: &[ModifierChange]) -> Vec<Option<Gesture>> {
        let mut detector = Detector::default();
        changes.iter().map(|&c| detector.on(c)).collect()
    }

    #[test]
    fn a_tap_then_a_held_press_fires_on_the_press_and_reports_the_release() {
        let fired = run(&[
            change(RIGHT_OPTION, RIGHT_OPTION_BIT, 12.695),
            change(RIGHT_OPTION, 0, 12.751),
            change(RIGHT_OPTION, RIGHT_OPTION_BIT, 12.847),
            change(RIGHT_OPTION, 0, 13.977),
        ]);
        assert_eq!(
            fired,
            [
                None,
                None,
                Some(Gesture::Pressed(ModifierKey::RightOption)),
                Some(Gesture::Released(ModifierKey::RightOption)),
            ]
        );
    }

    #[test]
    fn other_modifiers_during_the_hold_wait_for_the_key_to_come_up() {
        let fired = run(&[
            change(RIGHT_OPTION, RIGHT_OPTION_BIT, 0.0),
            change(RIGHT_OPTION, 0, 0.05),
            change(RIGHT_OPTION, RIGHT_OPTION_BIT, 0.15),
            change(56, RIGHT_OPTION_BIT | SHIFT_BIT, 0.5),
            change(56, RIGHT_OPTION_BIT, 0.6),
            change(RIGHT_OPTION, 0, 1.0),
        ]);
        assert_eq!(
            fired[3..],
            [None, None, Some(Gesture::Released(ModifierKey::RightOption))]
        );
    }

    #[test]
    fn a_single_hold_does_not_fire() {
        let fired = run(&[
            change(RIGHT_OPTION, RIGHT_OPTION_BIT, 24.800),
            change(RIGHT_OPTION, 0, 26.060),
            change(RIGHT_OPTION, RIGHT_OPTION_BIT, 26.200),
        ]);
        assert_eq!(fired, [None, None, None]);
    }

    #[test]
    fn a_slow_tap_or_a_long_pause_does_not_fire() {
        let slow_tap = run(&[
            change(RIGHT_OPTION, RIGHT_OPTION_BIT, 0.0),
            change(RIGHT_OPTION, 0, TAP + 0.01),
            change(RIGHT_OPTION, RIGHT_OPTION_BIT, TAP + 0.1),
        ]);
        let long_pause = run(&[
            change(RIGHT_OPTION, RIGHT_OPTION_BIT, 0.0),
            change(RIGHT_OPTION, 0, 0.1),
            change(RIGHT_OPTION, RIGHT_OPTION_BIT, 0.1 + GAP + 0.01),
        ]);
        assert_eq!(slow_tap.last(), Some(&None));
        assert_eq!(long_pause.last(), Some(&None));
    }

    #[test]
    fn a_tap_with_another_modifier_held_does_not_count() {
        let fired = run(&[
            change(RIGHT_OPTION, RIGHT_OPTION_BIT | SHIFT_BIT, 0.0),
            change(RIGHT_OPTION, SHIFT_BIT, 0.05),
            change(RIGHT_OPTION, RIGHT_OPTION_BIT, 0.15),
        ]);
        assert_eq!(fired, [None, None, None]);
    }

    #[test]
    fn the_other_side_breaks_the_sequence() {
        let fired = run(&[
            change(RIGHT_OPTION, RIGHT_OPTION_BIT, 0.0),
            change(RIGHT_OPTION, 0, 0.05),
            change(LEFT_OPTION, 0x20, 0.15),
        ]);
        assert_eq!(fired.last(), Some(&None));
    }

    #[test]
    fn typing_with_option_does_not_fire() {
        let mut typed = change(RIGHT_OPTION, RIGHT_OPTION_BIT, 0.15);
        typed.key_down_ago = 0.05;
        let fired = run(&[
            change(RIGHT_OPTION, RIGHT_OPTION_BIT, 0.0),
            change(RIGHT_OPTION, 0, 0.1),
            typed,
        ]);
        assert_eq!(fired.last(), Some(&None));
    }

    #[test]
    fn each_key_maps_to_its_own_action() {
        let shortcuts = ShortcutSettings {
            focus_input: Some(ModifierKey::RightOption),
            toggle_overlay: Some(ModifierKey::RightCommand),
        };
        assert_eq!(shortcuts.action_for(ModifierKey::RightOption), Some(Action::FocusInput));
        assert_eq!(
            shortcuts.action_for(ModifierKey::RightCommand),
            Some(Action::ToggleOverlay)
        );
        assert_eq!(shortcuts.action_for(ModifierKey::LeftOption), None);
    }
}
