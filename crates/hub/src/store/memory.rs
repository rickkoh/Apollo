use time::OffsetDateTime;

use super::{NewMessage, SessionRecord, Store};
use crate::domain::{DeliveryStatus, Direction, Message, MessageId, SessionId, SessionInfo};
use crate::error::StoreError;

struct Session {
    id: SessionId,
    cwd: String,
    title: Option<String>,
    claude_title: Option<String>,
    last_seen: OffsetDateTime,
    last_read: i64,
}

/// In-memory `Store` for actor tests; mirrors `SqliteStore` semantics.
#[derive(Default)]
pub struct MemoryStore {
    sessions: Vec<Session>,
    messages: Vec<Message>,
    target: Option<SessionId>,
    next_id: i64,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn session_mut(&mut self, id: &SessionId) -> Option<&mut Session> {
        self.sessions.iter_mut().find(|s| &s.id == id)
    }
}

impl Store for MemoryStore {
    fn upsert_session(&mut self, info: &SessionInfo, now: OffsetDateTime) -> Result<(), StoreError> {
        match self.session_mut(&info.id) {
            Some(s) => {
                s.cwd = info.cwd.clone();
                s.last_seen = now;
            }
            None => self.sessions.push(Session {
                id: info.id.clone(),
                cwd: info.cwd.clone(),
                title: None,
                claude_title: None,
                last_seen: now,
                last_read: 0,
            }),
        }
        Ok(())
    }

    fn touch_session(&mut self, id: &SessionId, now: OffsetDateTime) -> Result<(), StoreError> {
        if let Some(s) = self.session_mut(id) {
            s.last_seen = now;
        }
        Ok(())
    }

    fn sessions(&self) -> Result<Vec<SessionRecord>, StoreError> {
        let mut out: Vec<SessionRecord> = self
            .sessions
            .iter()
            .map(|s| SessionRecord {
                id: s.id.clone(),
                cwd: s.cwd.clone(),
                title: s.title.clone(),
                claude_title: s.claude_title.clone(),
                last_seen: s.last_seen,
                unread: self
                    .messages
                    .iter()
                    .filter(|m| m.session == s.id && m.direction == Direction::FromSession && m.id.0 > s.last_read)
                    .count() as u32,
            })
            .collect();
        out.sort_by_key(|r| std::cmp::Reverse(r.last_seen));
        Ok(out)
    }

    fn insert_message(&mut self, new: NewMessage) -> Result<Message, StoreError> {
        self.next_id += 1;
        let message = Message {
            id: MessageId(self.next_id),
            session: new.session,
            direction: new.direction,
            text: new.text,
            at: new.at,
            status: new.status,
            detail: new.detail,
        };
        self.messages.push(message.clone());
        Ok(message)
    }

    fn set_status(&mut self, id: MessageId, status: DeliveryStatus, detail: Option<&str>) -> Result<(), StoreError> {
        if let Some(m) = self.messages.iter_mut().find(|m| m.id == id) {
            m.status = status;
            m.detail = detail.map(str::to_owned);
        }
        Ok(())
    }

    fn fail_queued(&mut self, session: &SessionId, detail: &str) -> Result<Vec<MessageId>, StoreError> {
        let mut ids = Vec::new();
        for m in self
            .messages
            .iter_mut()
            .filter(|m| &m.session == session && m.status == DeliveryStatus::Queued)
        {
            m.status = DeliveryStatus::Failed;
            m.detail = Some(detail.to_owned());
            ids.push(m.id);
        }
        Ok(ids)
    }

    fn fail_all_queued(&mut self, detail: &str) -> Result<u64, StoreError> {
        let mut n = 0;
        for m in self.messages.iter_mut().filter(|m| m.status == DeliveryStatus::Queued) {
            m.status = DeliveryStatus::Failed;
            m.detail = Some(detail.to_owned());
            n += 1;
        }
        Ok(n)
    }

    fn history(&self, id: &SessionId, before: Option<MessageId>, limit: u32) -> Result<Vec<Message>, StoreError> {
        let before = before.map_or(i64::MAX, |b| b.0);
        let mut out: Vec<Message> = self
            .messages
            .iter()
            .rev()
            .filter(|m| &m.session == id && m.id.0 < before)
            .take(limit as usize)
            .cloned()
            .collect();
        out.reverse();
        Ok(out)
    }

    fn recent(&self, limit: u32) -> Result<Vec<Message>, StoreError> {
        let skip = self.messages.len().saturating_sub(limit as usize);
        Ok(self.messages[skip..].to_vec())
    }

    fn mark_read(&mut self, id: &SessionId, upto: MessageId) -> Result<(), StoreError> {
        if let Some(s) = self.session_mut(id) {
            s.last_read = s.last_read.max(upto.0);
        }
        Ok(())
    }

    fn rename(&mut self, id: &SessionId, title: Option<&str>) -> Result<(), StoreError> {
        if let Some(s) = self.session_mut(id) {
            s.title = title.map(str::to_owned);
        }
        Ok(())
    }

    fn set_claude_title(&mut self, id: &SessionId, title: &str) -> Result<(), StoreError> {
        if let Some(s) = self.session_mut(id) {
            s.claude_title = Some(title.to_owned());
        }
        Ok(())
    }

    fn forget(&mut self, id: &SessionId) -> Result<(), StoreError> {
        self.sessions.retain(|s| &s.id != id);
        self.messages.retain(|m| &m.session != id);
        Ok(())
    }

    fn clear_history(&mut self) -> Result<(), StoreError> {
        self.messages.clear();
        Ok(())
    }

    fn target(&self) -> Result<Option<SessionId>, StoreError> {
        Ok(self.target.clone())
    }

    fn set_target(&mut self, id: Option<&SessionId>) -> Result<(), StoreError> {
        self.target = id.cloned();
        Ok(())
    }

    fn purge_before(&mut self, cutoff: OffsetDateTime) -> Result<u64, StoreError> {
        let before = self.messages.len();
        self.messages.retain(|m| m.at >= cutoff);
        Ok((before - self.messages.len()) as u64)
    }
}
