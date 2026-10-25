use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use time::OffsetDateTime;
use tokio::sync::{mpsc, oneshot};

use crate::HubConfig;
use crate::domain::{DeliveryStatus, Direction, HubStatus, Message, MessageId, SessionId, SessionInfo, SessionView};
use crate::error::{HubError, SendError, StoreError};
use crate::events::HubEvents;
use crate::presence::{Activity, Presence};
use crate::protocol::{ErrorCode, HubFrame};
use crate::store::{NewMessage, SessionRecord, Store};

pub(crate) type Answer<T> = oneshot::Sender<T>;

#[derive(Debug)]
pub(crate) enum Outbound {
    Frame(HubFrame),
    Close,
}

pub(crate) type LinkSender = mpsc::UnboundedSender<Outbound>;

const NOT_PICKED_UP: &str = "Claude hasn't picked this up";

struct Link {
    conn: u64,
    tx: LinkSender,
    claude_pid: u32,
}

/// A message delivered to an idle session, heard once that session's activity changes.
struct Pickup {
    session: SessionId,
    idle_since: OffsetDateTime,
    deadline: Instant,
}

pub(crate) enum Command {
    Register {
        info: SessionInfo,
        conn: u64,
        link: LinkSender,
        answer: Answer<Result<String, HubError>>,
    },
    Unregister {
        id: SessionId,
        conn: u64,
    },
    Delivered {
        session: SessionId,
        id: MessageId,
    },
    Reply {
        session: SessionId,
        text: String,
    },
    Send {
        text: String,
        answer: Answer<Result<Message, SendError>>,
    },
    SetTarget {
        id: SessionId,
        answer: Answer<Result<(), HubError>>,
    },
    Tick,
    Sessions {
        answer: Answer<Result<Vec<SessionView>, HubError>>,
    },
    Status {
        answer: Answer<HubStatus>,
    },
    History {
        id: SessionId,
        before: Option<MessageId>,
        limit: u32,
        answer: Answer<Result<Vec<Message>, HubError>>,
    },
    Recent {
        limit: u32,
        answer: Answer<Result<Vec<Message>, HubError>>,
    },
    MarkRead {
        id: SessionId,
        upto: MessageId,
        answer: Answer<Result<(), HubError>>,
    },
    Rename {
        id: SessionId,
        title: Option<String>,
        answer: Answer<Result<(), HubError>>,
    },
    Forget {
        id: SessionId,
        answer: Answer<Result<(), HubError>>,
    },
    ClearHistory {
        answer: Answer<Result<(), HubError>>,
    },
    Shutdown {
        answer: Answer<()>,
    },
}

pub(crate) struct HubActor {
    config: Arc<HubConfig>,
    port: u16,
    store: Box<dyn Store>,
    presence: Option<Box<dyn Presence>>,
    events: Arc<dyn HubEvents>,
    links: HashMap<SessionId, Link>,
    /// Live sessions whose activity the presence knows, with when it last changed.
    activity: HashMap<SessionId, (Activity, OffsetDateTime)>,
    /// Live sessions' titles from Claude Code, as last stored.
    claude_titles: HashMap<SessionId, String>,
    target: Option<SessionId>,
    pending: HashMap<MessageId, (SessionId, Instant)>,
    pickups: HashMap<MessageId, Pickup>,
    /// Set while a target restored at start may still reconnect before another session takes over.
    restoring_until: Option<Instant>,
}

fn folder_name(cwd: &str) -> String {
    Path::new(cwd)
        .file_name()
        .map_or_else(|| cwd.to_owned(), |name| name.to_string_lossy().into_owned())
}

impl HubActor {
    pub(crate) fn new(
        config: Arc<HubConfig>,
        port: u16,
        store: Box<dyn Store>,
        presence: Option<Box<dyn Presence>>,
        events: Arc<dyn HubEvents>,
    ) -> Result<Self, StoreError> {
        let target = store.target()?;
        let restoring_until = target.as_ref().map(|_| Instant::now() + config.restore_grace);
        Ok(Self {
            config,
            port,
            store,
            presence,
            events,
            links: HashMap::new(),
            activity: HashMap::new(),
            claude_titles: HashMap::new(),
            target,
            pending: HashMap::new(),
            pickups: HashMap::new(),
            restoring_until,
        })
    }

    pub(crate) fn run(mut self, mut commands: mpsc::Receiver<Command>) {
        while let Some(command) = commands.blocking_recv() {
            if let Command::Shutdown { answer } = command {
                for link in self.links.values() {
                    let _ = link.tx.send(Outbound::Close);
                }
                let _ = answer.send(());
                return;
            }
            self.handle(command);
        }
    }

    fn handle(&mut self, command: Command) {
        match command {
            Command::Register {
                info,
                conn,
                link,
                answer,
            } => {
                let _ = answer.send(self.register(info, conn, link));
            }
            Command::Unregister { id, conn } => self.unregister(&id, conn),
            Command::Delivered { session, id } => self.delivered(&session, id),
            Command::Reply { session, text } => self.reply(session, text),
            Command::Send { text, answer } => {
                let _ = answer.send(self.send(text));
            }
            Command::SetTarget { id, answer } => {
                let _ = answer.send(self.set_target(id));
            }
            Command::Tick => self.tick(),
            Command::Sessions { answer } => {
                let _ = answer.send(self.views().map_err(HubError::from));
            }
            Command::Status { answer } => {
                let _ = answer.send(HubStatus {
                    port: self.port,
                    live_sessions: self.links.len() as u32,
                    target: self.target.clone(),
                });
            }
            Command::History {
                id,
                before,
                limit,
                answer,
            } => {
                let _ = answer.send(self.store.history(&id, before, limit).map_err(HubError::from));
            }
            Command::Recent { limit, answer } => {
                let _ = answer.send(self.store.recent(limit).map_err(HubError::from));
            }
            Command::MarkRead { id, upto, answer } => {
                let result = self.store.mark_read(&id, upto).map_err(HubError::from);
                self.emit_sessions();
                let _ = answer.send(result);
            }
            Command::Rename { id, title, answer } => {
                let _ = answer.send(self.rename(&id, title));
            }
            Command::Forget { id, answer } => {
                let _ = answer.send(self.forget(&id));
            }
            Command::ClearHistory { answer } => {
                let result = self.store.clear_history().map_err(HubError::from);
                self.pending.clear();
                self.pickups.clear();
                self.emit_sessions();
                let _ = answer.send(result);
            }
            Command::Shutdown { .. } => unreachable!("handled in run"),
        }
    }

    fn now() -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }

    fn live_target(&self) -> Option<&SessionId> {
        self.target.as_ref().filter(|t| self.links.contains_key(*t))
    }

    fn record(&self, id: &SessionId) -> Result<Option<SessionRecord>, StoreError> {
        Ok(self.store.sessions()?.into_iter().find(|r| &r.id == id))
    }

    fn views(&self) -> Result<Vec<SessionView>, StoreError> {
        let records = self.store.sessions()?;
        let base = |r: &SessionRecord| {
            r.title
                .clone()
                .or_else(|| r.claude_title.clone())
                .unwrap_or_else(|| folder_name(&r.cwd))
        };
        let mut live_titles: HashMap<String, u32> = HashMap::new();
        for r in records.iter().filter(|r| self.links.contains_key(&r.id)) {
            *live_titles.entry(base(r)).or_default() += 1;
        }
        Ok(records
            .iter()
            .map(|r| {
                let live = self.links.contains_key(&r.id);
                let mut title = base(r);
                if live && live_titles.get(&title).copied().unwrap_or(0) > 1 {
                    title = format!("{title} ({})", r.id.as_str().chars().take(4).collect::<String>());
                }
                SessionView {
                    id: r.id.clone(),
                    title,
                    cwd: r.cwd.clone(),
                    live,
                    is_target: self.target.as_ref() == Some(&r.id),
                    last_activity: Some(r.last_seen),
                    unread: r.unread,
                    activity: self.activity.get(&r.id).map(|(activity, _)| *activity),
                }
            })
            .collect())
    }

    fn title_of(&self, id: &SessionId) -> String {
        self.views()
            .ok()
            .and_then(|views| views.into_iter().find(|v| &v.id == id).map(|v| v.title))
            .unwrap_or_else(|| id.to_string())
    }

    fn emit_sessions(&self) {
        match self.views() {
            Ok(views) => self.events.sessions_changed(&views),
            Err(err) => tracing::error!("listing sessions failed: {err}"),
        }
    }

    fn change_target(&mut self, target: Option<SessionId>) -> Result<(), StoreError> {
        self.store.set_target(target.as_ref())?;
        self.target = target;
        self.events.target_changed(self.target.as_ref());
        Ok(())
    }

    fn register(&mut self, info: SessionInfo, conn: u64, tx: LinkSender) -> Result<String, HubError> {
        let id = info.id.clone();
        let link = Link {
            conn,
            tx,
            claude_pid: info.claude_pid,
        };
        if let Some(old) = self.links.insert(id.clone(), link) {
            let _ = old.tx.send(Outbound::Frame(HubFrame::error(
                ErrorCode::Replaced,
                "another connection took over",
            )));
            let _ = old.tx.send(Outbound::Close);
        }
        self.store.upsert_session(&info, Self::now())?;
        if self.target.as_ref() != Some(&id) && self.may_take_target() {
            self.change_target(Some(id.clone()))?;
        }
        tracing::info!(session = %id, conn, "session registered");
        self.emit_sessions();
        Ok(self.title_of(&id))
    }

    fn may_take_target(&self) -> bool {
        self.live_target().is_none() && (self.target.is_none() || self.restoring_until.is_none())
    }

    /// Ends the restore grace; when the restored target never came back, the earliest live connection takes over.
    fn end_restore_grace(&mut self) {
        if self.restoring_until.is_none_or(|until| Instant::now() < until) {
            return;
        }
        self.restoring_until = None;
        if self.live_target().is_some() {
            return;
        }
        let Some(id) = self
            .links
            .iter()
            .min_by_key(|(_, link)| link.conn)
            .map(|(id, _)| id.clone())
        else {
            return;
        };
        match self.change_target(Some(id)) {
            Ok(()) => self.emit_sessions(),
            Err(err) => tracing::error!("handing over the target failed: {err}"),
        }
    }

    fn unregister(&mut self, id: &SessionId, conn: u64) {
        if !self.links.get(id).is_some_and(|l| l.conn == conn) {
            return;
        }
        self.links.remove(id);
        self.activity.remove(id);
        self.claude_titles.remove(id);
        tracing::info!(session = %id, conn, "session unregistered");
        self.fail_pending_of(id, "session disconnected");
        self.emit_sessions();
    }

    fn fail_pending_of(&mut self, id: &SessionId, detail: &str) {
        match self.store.fail_queued(id, detail) {
            Ok(ids) => {
                for message in ids {
                    self.pending.remove(&message);
                    self.events
                        .message_updated(message, DeliveryStatus::Failed, Some(detail));
                }
            }
            Err(err) => tracing::error!("failing queued messages failed: {err}"),
        }
    }

    fn delivered(&mut self, session: &SessionId, id: MessageId) {
        if !self.pending.get(&id).is_some_and(|(s, _)| s == session) {
            return;
        }
        self.pending.remove(&id);
        if let Err(err) = self.store.set_status(id, DeliveryStatus::Delivered, None) {
            tracing::error!("marking delivered failed: {err}");
            return;
        }
        self.events.message_updated(id, DeliveryStatus::Delivered, None);
        if let Some((Activity::Idle, since)) = self.activity.get(session) {
            let pickup = Pickup {
                session: session.clone(),
                idle_since: *since,
                deadline: Instant::now() + self.config.pickup_timeout,
            };
            self.pickups.insert(id, pickup);
        }
    }

    /// Flags messages whose idle session never started a turn; a channel Claude Code refused still acknowledges.
    fn check_pickups(&mut self) {
        let now = Instant::now();
        let mut unheard = Vec::new();
        self.pickups.retain(|id, pickup| {
            let still_idle = self
                .activity
                .get(&pickup.session)
                .is_some_and(|(_, since)| *since == pickup.idle_since);
            if still_idle && now >= pickup.deadline {
                unheard.push(*id);
                return false;
            }
            still_idle
        });
        for id in unheard {
            match self
                .store
                .set_status(id, DeliveryStatus::Delivered, Some(NOT_PICKED_UP))
            {
                Ok(()) => self
                    .events
                    .message_updated(id, DeliveryStatus::Delivered, Some(NOT_PICKED_UP)),
                Err(err) => tracing::error!("flagging an unheard message failed: {err}"),
            }
        }
    }

    fn reply(&mut self, session: SessionId, text: String) {
        if !self.links.contains_key(&session) {
            return;
        }
        let now = Self::now();
        let new = NewMessage {
            session: session.clone(),
            direction: Direction::FromSession,
            text,
            at: now,
            status: DeliveryStatus::Received,
            detail: None,
        };
        let result = self
            .store
            .insert_message(new)
            .and_then(|m| self.store.touch_session(&session, now).map(|()| m));
        match result {
            Ok(message) => {
                self.events.message_added(&message);
                self.emit_sessions();
            }
            Err(err) => tracing::error!("storing a reply failed: {err}"),
        }
    }

    fn send(&mut self, text: String) -> Result<Message, SendError> {
        let text = text.trim().to_owned();
        if text.is_empty() {
            return Err(SendError::Empty);
        }
        if text.len() > self.config.max_text_bytes {
            return Err(SendError::TooLarge);
        }
        let target = self.target.clone().ok_or(SendError::NoTarget)?;
        let Some(link) = self.links.get(&target) else {
            return Err(SendError::TargetOffline {
                title: self.title_of(&target),
            });
        };
        let tx = link.tx.clone();
        let now = Self::now();
        let message = self.store.insert_message(NewMessage {
            session: target.clone(),
            direction: Direction::ToSession,
            text: text.clone(),
            at: now,
            status: DeliveryStatus::Queued,
            detail: None,
        })?;
        self.store.touch_session(&target, now)?;
        self.pending.insert(message.id, (target, Instant::now()));
        let _ = tx.send(Outbound::Frame(HubFrame::Deliver {
            id: message.id,
            text,
            at: now,
        }));
        self.events.message_added(&message);
        self.emit_sessions();
        Ok(message)
    }

    fn set_target(&mut self, id: SessionId) -> Result<(), HubError> {
        if !self.links.contains_key(&id) {
            return Err(if self.record(&id)?.is_some() {
                HubError::SessionOffline
            } else {
                HubError::UnknownSession
            });
        }
        if self.target.as_ref() != Some(&id) {
            self.change_target(Some(id))?;
            self.emit_sessions();
        }
        Ok(())
    }

    fn rename(&mut self, id: &SessionId, title: Option<String>) -> Result<(), HubError> {
        if self.record(id)?.is_none() {
            return Err(HubError::UnknownSession);
        }
        let title = title.map(|t| t.trim().to_owned()).filter(|t| !t.is_empty());
        self.store.rename(id, title.as_deref())?;
        self.emit_sessions();
        Ok(())
    }

    fn forget(&mut self, id: &SessionId) -> Result<(), HubError> {
        if self.links.contains_key(id) {
            return Err(HubError::SessionLive);
        }
        if self.record(id)?.is_none() {
            return Err(HubError::UnknownSession);
        }
        self.store.forget(id)?;
        self.pending.retain(|_, (s, _)| s != id);
        if self.target.as_ref() == Some(id) {
            self.change_target(None)?;
        }
        self.emit_sessions();
        Ok(())
    }

    /// Re-reads the live sessions' activity and titles and tells the UI only when one changed.
    fn refresh_presence(&mut self) {
        let Some(presence) = self.presence.as_mut() else {
            return;
        };
        if self.links.is_empty() {
            return;
        }
        let fresh: HashMap<SessionId, (Activity, OffsetDateTime)> = self
            .links
            .iter()
            .filter_map(|(id, link)| Some((id.clone(), presence.activity(link.claude_pid)?)))
            .collect();
        let retitled: Vec<(SessionId, String)> = self
            .links
            .iter()
            .filter_map(|(id, link)| Some((id.clone(), presence.title(link.claude_pid)?)))
            .filter(|(id, title)| self.claude_titles.get(id) != Some(title))
            .collect();
        let mut changed = fresh.len() != self.activity.len()
            || fresh
                .iter()
                .any(|(id, (now, _))| self.activity.get(id).map(|(before, _)| before) != Some(now));
        self.activity = fresh;
        for (id, title) in retitled {
            match self.store.set_claude_title(&id, &title) {
                Ok(()) => {
                    self.claude_titles.insert(id, title);
                    changed = true;
                }
                Err(err) => tracing::error!("storing a session title failed: {err}"),
            }
        }
        if changed {
            self.emit_sessions();
        }
    }

    fn tick(&mut self) {
        self.end_restore_grace();
        self.refresh_presence();
        self.check_pickups();
        let timeout = self.config.delivery_timeout;
        let expired: Vec<MessageId> = self
            .pending
            .iter()
            .filter(|(_, (_, since))| since.elapsed() >= timeout)
            .map(|(id, _)| *id)
            .collect();
        for id in expired {
            self.pending.remove(&id);
            let detail = "no acknowledgement";
            match self.store.set_status(id, DeliveryStatus::Failed, Some(detail)) {
                Ok(()) => self.events.message_updated(id, DeliveryStatus::Failed, Some(detail)),
                Err(err) => tracing::error!("failing an unacknowledged message failed: {err}"),
            }
        }
    }
}

#[cfg(test)]
#[path = "actor_tests.rs"]
mod tests;
