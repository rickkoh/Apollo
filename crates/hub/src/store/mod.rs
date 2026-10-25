mod memory;
mod sqlite;

pub use memory::MemoryStore;
pub use sqlite::SqliteStore;

use time::OffsetDateTime;

use crate::domain::{DeliveryStatus, Direction, Message, MessageId, SessionId, SessionInfo};
use crate::error::StoreError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRecord {
    pub id: SessionId,
    pub cwd: String,
    pub title: Option<String>,
    pub last_seen: OffsetDateTime,
    pub unread: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewMessage {
    pub session: SessionId,
    pub direction: Direction,
    pub text: String,
    pub at: OffsetDateTime,
    pub status: DeliveryStatus,
    pub detail: Option<String>,
}

/// Persistence port of the hub. Implementations are used from the actor thread only.
pub trait Store: Send {
    fn upsert_session(&mut self, info: &SessionInfo, now: OffsetDateTime) -> Result<(), StoreError>;
    fn touch_session(&mut self, id: &SessionId, now: OffsetDateTime) -> Result<(), StoreError>;
    fn sessions(&self) -> Result<Vec<SessionRecord>, StoreError>;
    fn insert_message(&mut self, new: NewMessage) -> Result<Message, StoreError>;
    fn set_status(&mut self, id: MessageId, status: DeliveryStatus, detail: Option<&str>) -> Result<(), StoreError>;
    fn fail_queued(&mut self, session: &SessionId, detail: &str) -> Result<Vec<MessageId>, StoreError>;
    fn fail_all_queued(&mut self, detail: &str) -> Result<u64, StoreError>;
    /// The latest `limit` messages older than `before`, oldest first.
    fn history(&self, id: &SessionId, before: Option<MessageId>, limit: u32) -> Result<Vec<Message>, StoreError>;
    /// The latest `limit` messages across all sessions, oldest first.
    fn recent(&self, limit: u32) -> Result<Vec<Message>, StoreError>;
    fn mark_read(&mut self, id: &SessionId, upto: MessageId) -> Result<(), StoreError>;
    fn rename(&mut self, id: &SessionId, title: Option<&str>) -> Result<(), StoreError>;
    fn forget(&mut self, id: &SessionId) -> Result<(), StoreError>;
    fn clear_history(&mut self) -> Result<(), StoreError>;
    fn target(&self) -> Result<Option<SessionId>, StoreError>;
    fn set_target(&mut self, id: Option<&SessionId>) -> Result<(), StoreError>;
    fn purge_before(&mut self, cutoff: OffsetDateTime) -> Result<u64, StoreError>;
}
