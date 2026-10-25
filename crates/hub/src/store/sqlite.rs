use std::path::Path;

use rusqlite::{Connection, OptionalExtension, Row, params};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::{NewMessage, SessionRecord, Store};
use crate::domain::{DeliveryStatus, Direction, Message, MessageId, SessionId, SessionInfo};
use crate::error::StoreError;

const MIGRATIONS: &[&str] = &[
    "
    CREATE TABLE sessions (
        id          TEXT PRIMARY KEY,
        cwd         TEXT NOT NULL,
        title       TEXT,
        first_seen  TEXT NOT NULL,
        last_seen   TEXT NOT NULL,
        last_read   INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE messages (
        id          INTEGER PRIMARY KEY,
        session_id  TEXT NOT NULL REFERENCES sessions (id) ON DELETE CASCADE,
        direction   TEXT NOT NULL CHECK (direction IN ('to_session', 'from_session')),
        text        TEXT NOT NULL,
        created_at  TEXT NOT NULL,
        status      TEXT NOT NULL CHECK (status IN ('queued', 'delivered', 'failed', 'received')),
        detail      TEXT
    );
    CREATE INDEX messages_by_session ON messages (session_id, id);
    CREATE TABLE meta (
        key    TEXT PRIMARY KEY,
        value  TEXT NOT NULL
    );
",
    "
    ALTER TABLE sessions ADD COLUMN claude_title TEXT;
",
];

pub struct SqliteStore {
    conn: Connection,
}

impl SqliteStore {
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self, StoreError> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Self, StoreError> {
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        let applied: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        let known = MIGRATIONS.len() as i64;
        if applied > known {
            return Err(StoreError::NewerSchema { found: applied, known });
        }
        for (index, sql) in (0i64..).zip(MIGRATIONS).skip(applied as usize) {
            let tx = conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", index + 1)?;
            tx.commit()?;
        }
        Ok(Self { conn })
    }
}

fn ts(at: OffsetDateTime) -> String {
    at.format(&Rfc3339).expect("RFC 3339 formats every UTC timestamp")
}

fn parse_ts(text: &str) -> Result<OffsetDateTime, StoreError> {
    OffsetDateTime::parse(text, &Rfc3339).map_err(|e| StoreError::Time(e.to_string()))
}

fn direction_str(d: Direction) -> &'static str {
    match d {
        Direction::ToSession => "to_session",
        Direction::FromSession => "from_session",
    }
}

fn status_str(s: DeliveryStatus) -> &'static str {
    match s {
        DeliveryStatus::Queued => "queued",
        DeliveryStatus::Delivered => "delivered",
        DeliveryStatus::Failed => "failed",
        DeliveryStatus::Received => "received",
    }
}

fn parse_direction(s: &str) -> Result<Direction, StoreError> {
    match s {
        "to_session" => Ok(Direction::ToSession),
        "from_session" => Ok(Direction::FromSession),
        other => Err(StoreError::Corrupt(format!("direction {other}"))),
    }
}

fn parse_status(s: &str) -> Result<DeliveryStatus, StoreError> {
    match s {
        "queued" => Ok(DeliveryStatus::Queued),
        "delivered" => Ok(DeliveryStatus::Delivered),
        "failed" => Ok(DeliveryStatus::Failed),
        "received" => Ok(DeliveryStatus::Received),
        other => Err(StoreError::Corrupt(format!("status {other}"))),
    }
}

type RawMessage = (i64, String, String, String, String, String, Option<String>);

fn raw_message(row: &Row<'_>) -> rusqlite::Result<RawMessage> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
    ))
}

fn to_message(raw: RawMessage) -> Result<Message, StoreError> {
    let (id, session, direction, text, at, status, detail) = raw;
    Ok(Message {
        id: MessageId(id),
        session: SessionId(session),
        direction: parse_direction(&direction)?,
        text,
        at: parse_ts(&at)?,
        status: parse_status(&status)?,
        detail,
    })
}

impl Store for SqliteStore {
    fn upsert_session(&mut self, info: &SessionInfo, now: OffsetDateTime) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO sessions (id, cwd, first_seen, last_seen) VALUES (?1, ?2, ?3, ?3)
             ON CONFLICT (id) DO UPDATE SET cwd = excluded.cwd, last_seen = excluded.last_seen",
            params![info.id.as_str(), info.cwd, ts(now)],
        )?;
        Ok(())
    }

    fn touch_session(&mut self, id: &SessionId, now: OffsetDateTime) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE sessions SET last_seen = ?2 WHERE id = ?1",
            params![id.as_str(), ts(now)],
        )?;
        Ok(())
    }

    fn sessions(&self) -> Result<Vec<SessionRecord>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.cwd, s.title, s.claude_title, s.last_seen,
                    (SELECT COUNT(*) FROM messages m
                      WHERE m.session_id = s.id AND m.direction = 'from_session' AND m.id > s.last_read)
               FROM sessions s ORDER BY s.last_seen DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, u32>(5)?,
            ))
        })?;
        rows.map(|row| {
            let (id, cwd, title, claude_title, last_seen, unread) = row?;
            Ok(SessionRecord {
                id: SessionId(id),
                cwd,
                title,
                claude_title,
                last_seen: parse_ts(&last_seen)?,
                unread,
            })
        })
        .collect()
    }

    fn insert_message(&mut self, new: NewMessage) -> Result<Message, StoreError> {
        self.conn.execute(
            "INSERT INTO messages (session_id, direction, text, created_at, status, detail) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![new.session.as_str(), direction_str(new.direction), new.text, ts(new.at), status_str(new.status), new.detail],
        )?;
        Ok(Message {
            id: MessageId(self.conn.last_insert_rowid()),
            session: new.session,
            direction: new.direction,
            text: new.text,
            at: new.at,
            status: new.status,
            detail: new.detail,
        })
    }

    fn set_status(&mut self, id: MessageId, status: DeliveryStatus, detail: Option<&str>) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE messages SET status = ?2, detail = ?3 WHERE id = ?1",
            params![id.0, status_str(status), detail],
        )?;
        Ok(())
    }

    fn fail_queued(&mut self, session: &SessionId, detail: &str) -> Result<Vec<MessageId>, StoreError> {
        let mut stmt = self.conn.prepare(
            "UPDATE messages SET status = 'failed', detail = ?2 WHERE session_id = ?1 AND status = 'queued' RETURNING id",
        )?;
        let ids = stmt.query_map(params![session.as_str(), detail], |r| r.get(0))?;
        let mut out: Vec<MessageId> = ids.map(|id| id.map(MessageId)).collect::<Result<_, _>>()?;
        out.sort();
        Ok(out)
    }

    fn fail_all_queued(&mut self, detail: &str) -> Result<u64, StoreError> {
        let n = self.conn.execute(
            "UPDATE messages SET status = 'failed', detail = ?1 WHERE status = 'queued'",
            params![detail],
        )?;
        Ok(n as u64)
    }

    fn history(&self, id: &SessionId, before: Option<MessageId>, limit: u32) -> Result<Vec<Message>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, direction, text, created_at, status, detail FROM messages
              WHERE session_id = ?1 AND id < ?2 ORDER BY id DESC LIMIT ?3",
        )?;
        let rows = stmt.query_map(
            params![id.as_str(), before.map_or(i64::MAX, |b| b.0), limit],
            raw_message,
        )?;
        let mut out = rows.map(|r| to_message(r?)).collect::<Result<Vec<_>, _>>()?;
        out.reverse();
        Ok(out)
    }

    fn recent(&self, limit: u32) -> Result<Vec<Message>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, direction, text, created_at, status, detail FROM messages ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], raw_message)?;
        let mut out = rows.map(|r| to_message(r?)).collect::<Result<Vec<_>, _>>()?;
        out.reverse();
        Ok(out)
    }

    fn mark_read(&mut self, id: &SessionId, upto: MessageId) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE sessions SET last_read = MAX(last_read, ?2) WHERE id = ?1",
            params![id.as_str(), upto.0],
        )?;
        Ok(())
    }

    fn rename(&mut self, id: &SessionId, title: Option<&str>) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE sessions SET title = ?2 WHERE id = ?1",
            params![id.as_str(), title],
        )?;
        Ok(())
    }

    fn set_claude_title(&mut self, id: &SessionId, title: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE sessions SET claude_title = ?2 WHERE id = ?1",
            params![id.as_str(), title],
        )?;
        Ok(())
    }

    fn forget(&mut self, id: &SessionId) -> Result<(), StoreError> {
        self.conn
            .execute("DELETE FROM sessions WHERE id = ?1", params![id.as_str()])?;
        Ok(())
    }

    fn clear_history(&mut self) -> Result<(), StoreError> {
        self.conn.execute("DELETE FROM messages", [])?;
        Ok(())
    }

    fn target(&self) -> Result<Option<SessionId>, StoreError> {
        Ok(self
            .conn
            .query_row("SELECT value FROM meta WHERE key = 'target'", [], |r| {
                r.get::<_, String>(0)
            })
            .optional()?
            .map(SessionId))
    }

    fn set_target(&mut self, id: Option<&SessionId>) -> Result<(), StoreError> {
        match id {
            Some(id) => self.conn.execute(
                "INSERT INTO meta (key, value) VALUES ('target', ?1) ON CONFLICT (key) DO UPDATE SET value = excluded.value",
                params![id.as_str()],
            )?,
            None => self.conn.execute("DELETE FROM meta WHERE key = 'target'", [])?,
        };
        Ok(())
    }

    fn purge_before(&mut self, cutoff: OffsetDateTime) -> Result<u64, StoreError> {
        let n = self
            .conn
            .execute("DELETE FROM messages WHERE created_at < ?1", params![ts(cutoff)])?;
        Ok(n as u64)
    }
}
