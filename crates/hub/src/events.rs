use crate::domain::{DeliveryStatus, Message, MessageId, SessionId, SessionView};

/// What the hub tells its host (the UI) about state changes. Called from the actor thread.
pub trait HubEvents: Send + Sync {
    fn sessions_changed(&self, sessions: &[SessionView]);
    fn target_changed(&self, target: Option<&SessionId>);
    fn message_added(&self, message: &Message);
    fn message_updated(&self, id: MessageId, status: DeliveryStatus, detail: Option<&str>);
}
