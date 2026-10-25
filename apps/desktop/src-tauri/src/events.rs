use hub::{DeliveryStatus, HubEvents, Message, MessageId, SessionId, SessionView};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use ts_rs::TS;

use crate::tray;

pub const SESSIONS_CHANGED: &str = "sessions-changed";
pub const TARGET_CHANGED: &str = "target-changed";
pub const MESSAGE_ADDED: &str = "message-added";
pub const MESSAGE_UPDATED: &str = "message-updated";
pub const SETTINGS_CHANGED: &str = "settings-changed";
pub const OVERLAY_VISIBILITY: &str = "overlay-visibility";
pub const HUB_STATE: &str = "hub-state";
pub const UPDATE_STATUS: &str = "update-status";

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct TargetChanged {
    pub target: Option<SessionId>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct MessageUpdated {
    pub id: MessageId,
    pub status: DeliveryStatus,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct OverlayVisibility {
    pub visible: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct HubState {
    pub running: bool,
}

pub fn emit<T: Serialize + Clone>(app: &AppHandle, event: &str, payload: T) {
    if let Err(err) = app.emit(event, payload) {
        tracing::warn!("emitting {event} failed: {err}");
    }
}

/// Forwards hub state changes to every window and keeps the menu bar's target line current.
pub struct TauriEvents {
    app: AppHandle,
}

impl TauriEvents {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl HubEvents for TauriEvents {
    fn sessions_changed(&self, sessions: &[SessionView]) {
        let target = sessions.iter().find(|s| s.is_target).map(|s| (s.title.clone(), s.live));
        tray::show_target(&self.app, target);
        emit(&self.app, SESSIONS_CHANGED, sessions.to_vec());
    }

    fn target_changed(&self, target: Option<&SessionId>) {
        emit(
            &self.app,
            TARGET_CHANGED,
            TargetChanged {
                target: target.cloned(),
            },
        );
    }

    fn message_added(&self, message: &Message) {
        emit(&self.app, MESSAGE_ADDED, message.clone());
    }

    fn message_updated(&self, id: MessageId, status: DeliveryStatus, detail: Option<&str>) {
        emit(
            &self.app,
            MESSAGE_UPDATED,
            MessageUpdated {
                id,
                status,
                detail: detail.map(str::to_owned),
            },
        );
    }
}
