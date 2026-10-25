use std::sync::Mutex;
use std::time::Duration;

use time::macros::datetime;

use super::*;
use crate::HubHandle;
use crate::domain::Token;
use crate::store::MemoryStore;

#[derive(Debug, Clone, PartialEq)]
enum Event {
    Sessions(Vec<SessionView>),
    Target(Option<SessionId>),
    Added(Message),
    Updated(MessageId, DeliveryStatus, Option<String>),
}

#[derive(Default)]
struct Recorder(Mutex<Vec<Event>>);

impl Recorder {
    fn take(&self) -> Vec<Event> {
        std::mem::take(&mut self.0.lock().unwrap())
    }
}

impl HubEvents for Recorder {
    fn sessions_changed(&self, sessions: &[SessionView]) {
        self.0.lock().unwrap().push(Event::Sessions(sessions.to_vec()));
    }
    fn target_changed(&self, target: Option<&SessionId>) {
        self.0.lock().unwrap().push(Event::Target(target.cloned()));
    }
    fn message_added(&self, message: &Message) {
        self.0.lock().unwrap().push(Event::Added(message.clone()));
    }
    fn message_updated(&self, id: MessageId, status: DeliveryStatus, detail: Option<&str>) {
        self.0
            .lock()
            .unwrap()
            .push(Event::Updated(id, status, detail.map(str::to_owned)));
    }
}

/// Activity by Claude pid, set by the test.
#[derive(Clone, Default)]
struct FakePresence(Arc<Mutex<HashMap<u32, (Activity, OffsetDateTime)>>>);

impl FakePresence {
    fn set(&self, pid: u32, activity: Activity, since: OffsetDateTime) {
        self.0.lock().unwrap().insert(pid, (activity, since));
    }
}

impl Presence for FakePresence {
    fn activity(&mut self, pid: u32) -> Option<(Activity, OffsetDateTime)> {
        self.0.lock().unwrap().get(&pid).copied()
    }
}

struct Harness {
    commands: mpsc::Sender<Command>,
    handle: HubHandle,
    events: Arc<Recorder>,
    presence: FakePresence,
    next_conn: u64,
}

struct Peer {
    conn: u64,
    pid: u32,
    rx: mpsc::UnboundedReceiver<Outbound>,
}

impl Peer {
    fn drain(&mut self) -> Vec<Outbound> {
        std::iter::from_fn(|| self.rx.try_recv().ok()).collect()
    }
}

fn sid(s: &str) -> SessionId {
    SessionId(s.into())
}

fn harness_with(store: MemoryStore, delivery_timeout: Duration) -> Harness {
    harness_config(store, |config| config.delivery_timeout = delivery_timeout)
}

fn harness_config(store: MemoryStore, configure: impl FnOnce(&mut HubConfig)) -> Harness {
    let mut config = HubConfig::new(Token::generate());
    configure(&mut config);
    build(config, store, true)
}

fn build(config: HubConfig, store: MemoryStore, with_presence: bool) -> Harness {
    let events = Arc::new(Recorder::default());
    let presence = FakePresence::default();
    let shared = with_presence.then(|| Box::new(presence.clone()) as Box<dyn Presence>);
    let actor = HubActor::new(Arc::new(config), 4242, Box::new(store), shared, events.clone()).unwrap();
    let (commands, inbox) = mpsc::channel(16);
    std::thread::spawn(move || actor.run(inbox));
    Harness {
        handle: HubHandle {
            commands: commands.clone(),
        },
        commands,
        events,
        presence,
        next_conn: 1,
    }
}

fn harness() -> Harness {
    harness_with(MemoryStore::new(), Duration::from_secs(5))
}

impl Harness {
    async fn register(&mut self, id: &str, cwd: &str) -> (String, Peer) {
        let conn = self.next_conn;
        self.next_conn += 1;
        let pid = 1000 + conn as u32;
        let (link, rx) = mpsc::unbounded_channel();
        let (answer, title) = oneshot::channel();
        let info = SessionInfo {
            id: sid(id),
            cwd: cwd.into(),
            claude_pid: pid,
            started_at: OffsetDateTime::now_utc(),
        };
        self.commands
            .send(Command::Register {
                info,
                conn,
                link,
                answer,
            })
            .await
            .unwrap();
        (title.await.unwrap().unwrap(), Peer { conn, pid, rx })
    }

    async fn command(&self, command: Command) {
        self.commands.send(command).await.unwrap();
        self.handle.status().await.unwrap();
    }

    async fn view(&self, id: &str) -> SessionView {
        self.handle
            .sessions()
            .await
            .unwrap()
            .into_iter()
            .find(|v| v.id == sid(id))
            .unwrap()
    }
}

#[tokio::test]
async fn first_session_becomes_target_with_folder_title() {
    let mut h = harness();
    let (title, _a) = h.register("aaaa1", "/work/orders.service").await;
    assert_eq!(title, "orders.service");
    let events = h.events.take();
    assert!(events.contains(&Event::Target(Some(sid("aaaa1")))));
    assert!(matches!(events.last(), Some(Event::Sessions(v)) if v[0].live && v[0].is_target));
}

#[tokio::test]
async fn target_never_switches_away_from_a_live_session() {
    let mut h = harness();
    let (_, _a) = h.register("a", "/w/a").await;
    let (_, _b) = h.register("b", "/w/b").await;
    assert_eq!(h.handle.status().await.unwrap().target, Some(sid("a")));
}

#[tokio::test]
async fn registering_takes_over_a_target_that_disconnected() {
    let mut h = harness();
    let (_, a) = h.register("a", "/w/a").await;
    h.command(Command::Unregister {
        id: sid("a"),
        conn: a.conn,
    })
    .await;
    let (_, _b) = h.register("b", "/w/b").await;
    assert_eq!(h.handle.status().await.unwrap().target, Some(sid("b")));
}

#[tokio::test]
async fn a_restored_target_keeps_priority_during_the_grace_period() {
    let mut store = MemoryStore::new();
    store.set_target(Some(&sid("a"))).unwrap();
    let mut h = harness_config(store, |c| c.restore_grace = Duration::from_secs(60));
    let (_, _b) = h.register("b", "/w/b").await;
    assert_eq!(h.handle.status().await.unwrap().target, Some(sid("a")));
    let (_, _a) = h.register("a", "/w/a").await;
    h.command(Command::Tick).await;
    assert_eq!(h.handle.status().await.unwrap().target, Some(sid("a")));
}

#[tokio::test]
async fn after_the_grace_period_a_live_session_takes_over_a_target_that_never_came_back() {
    let mut store = MemoryStore::new();
    store.set_target(Some(&sid("gone"))).unwrap();
    let mut h = harness_config(store, |c| c.restore_grace = Duration::from_millis(30));
    let (_, _b) = h.register("b", "/w/b").await;
    let (_, _c) = h.register("c", "/w/c").await;
    assert_eq!(h.handle.status().await.unwrap().target, Some(sid("gone")));
    tokio::time::sleep(Duration::from_millis(50)).await;
    h.command(Command::Tick).await;
    assert_eq!(
        h.handle.status().await.unwrap().target,
        Some(sid("b")),
        "the earliest connection takes over"
    );
    assert!(h.events.take().contains(&Event::Target(Some(sid("b")))));
}

#[tokio::test]
async fn restored_target_stays_target_when_it_registers() {
    let mut store = MemoryStore::new();
    store.set_target(Some(&sid("a"))).unwrap();
    let mut h = harness_with(store, Duration::from_secs(5));
    let (_, _a) = h.register("a", "/w/a").await;
    let (_, _b) = h.register("b", "/w/b").await;
    assert_eq!(h.handle.status().await.unwrap().target, Some(sid("a")));
}

#[tokio::test]
async fn a_new_connection_replaces_the_old_one() {
    let mut h = harness();
    let (_, mut old) = h.register("a", "/w/a").await;
    let (_, _new) = h.register("a", "/w/a").await;
    assert!(matches!(
        &old.drain()[..],
        [
            Outbound::Frame(HubFrame::Error {
                code: ErrorCode::Replaced,
                ..
            }),
            Outbound::Close
        ]
    ));
    h.command(Command::Unregister {
        id: sid("a"),
        conn: old.conn,
    })
    .await;
    assert!(
        h.view("a").await.live,
        "the stale connection's close must not unregister the new one"
    );
}

#[tokio::test]
async fn send_errors() {
    let mut h = harness();
    assert!(matches!(h.handle.send("hi".into()).await, Err(SendError::NoTarget)));
    let (_, a) = h.register("a", "/w/alpha").await;
    assert!(matches!(h.handle.send("   ".into()).await, Err(SendError::Empty)));
    assert!(matches!(
        h.handle.send("x".repeat(16 * 1024 + 1)).await,
        Err(SendError::TooLarge)
    ));
    h.command(Command::Unregister {
        id: sid("a"),
        conn: a.conn,
    })
    .await;
    assert!(matches!(h.handle.send("hi".into()).await, Err(SendError::TargetOffline { title }) if title == "alpha"));
}

#[tokio::test]
async fn send_reaches_only_the_target_and_delivered_marks_it() {
    let mut h = harness();
    let (_, mut a) = h.register("a", "/w/a").await;
    let (_, mut b) = h.register("b", "/w/b").await;
    h.events.take();

    let message = h.handle.send(" click Settle ".into()).await.unwrap();
    assert_eq!(
        (message.text.as_str(), message.status),
        ("click Settle", DeliveryStatus::Queued)
    );
    assert!(matches!(
        &a.drain()[..],
        [Outbound::Frame(HubFrame::Deliver { id, text, .. })] if *id == message.id && text == "click Settle"
    ));
    assert!(b.drain().is_empty());

    h.command(Command::Delivered {
        session: sid("b"),
        id: message.id,
    })
    .await;
    assert!(
        !h.events.take().iter().any(|e| matches!(e, Event::Updated(..))),
        "another session cannot ack it"
    );

    h.command(Command::Delivered {
        session: sid("a"),
        id: message.id,
    })
    .await;
    assert!(
        h.events
            .take()
            .contains(&Event::Updated(message.id, DeliveryStatus::Delivered, None))
    );
    let history = h.handle.history(sid("a"), None, 10).await.unwrap();
    assert_eq!(history[0].status, DeliveryStatus::Delivered);
}

#[tokio::test]
async fn unacknowledged_messages_fail_after_the_timeout() {
    let mut h = harness_with(MemoryStore::new(), Duration::from_millis(30));
    let (_, _a) = h.register("a", "/w/a").await;
    let message = h.handle.send("hello".into()).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    h.command(Command::Tick).await;
    let detail = Some("no acknowledgement".to_owned());
    assert!(
        h.events
            .take()
            .contains(&Event::Updated(message.id, DeliveryStatus::Failed, detail.clone()))
    );
    assert_eq!(h.handle.history(sid("a"), None, 1).await.unwrap()[0].detail, detail);
}

#[tokio::test]
async fn a_disconnect_fails_its_pending_messages() {
    let mut h = harness();
    let (_, a) = h.register("a", "/w/a").await;
    let message = h.handle.send("hello".into()).await.unwrap();
    h.command(Command::Unregister {
        id: sid("a"),
        conn: a.conn,
    })
    .await;
    let detail = Some("session disconnected".to_owned());
    assert!(
        h.events
            .take()
            .contains(&Event::Updated(message.id, DeliveryStatus::Failed, detail))
    );
    assert!(!h.view("a").await.live);
}

#[tokio::test]
async fn replies_are_stored_counted_unread_and_marked_read() {
    let mut h = harness();
    let (_, _a) = h.register("a", "/w/a").await;
    h.events.take();
    h.command(Command::Reply {
        session: sid("a"),
        text: "Clicked it.".into(),
    })
    .await;
    let events = h.events.take();
    let Some(Event::Added(reply)) = events.iter().find(|e| matches!(e, Event::Added(_))).cloned() else {
        panic!("no message_added")
    };
    assert_eq!(
        (reply.direction, reply.status),
        (Direction::FromSession, DeliveryStatus::Received)
    );
    assert_eq!(h.view("a").await.unread, 1);
    h.handle.mark_read(sid("a"), reply.id).await.unwrap();
    assert_eq!(h.view("a").await.unread, 0);
}

#[tokio::test]
async fn set_target_rules() {
    let mut h = harness();
    let (_, a) = h.register("a", "/w/a").await;
    let (_, _b) = h.register("b", "/w/b").await;
    assert!(matches!(
        h.handle.set_target(sid("zzz")).await,
        Err(HubError::UnknownSession)
    ));
    h.handle.set_target(sid("b")).await.unwrap();
    assert!(h.events.take().contains(&Event::Target(Some(sid("b")))));
    h.command(Command::Unregister {
        id: sid("a"),
        conn: a.conn,
    })
    .await;
    assert!(matches!(
        h.handle.set_target(sid("a")).await,
        Err(HubError::SessionOffline)
    ));
}

#[tokio::test]
async fn live_sessions_sharing_a_title_get_an_id_suffix() {
    let mut h = harness();
    let (first, _a) = h.register("abcd1111", "/x/repo").await;
    let (second, _b) = h.register("efgh2222", "/y/repo").await;
    assert_eq!(first, "repo");
    assert_eq!(second, "repo (efgh)");
    assert_eq!(h.view("abcd1111").await.title, "repo (abcd)");
}

#[tokio::test]
async fn rename_overrides_the_title_and_empty_clears_it() {
    let mut h = harness();
    let (_, _a) = h.register("a", "/w/alpha").await;
    h.handle.rename(sid("a"), Some(" Payments ".into())).await.unwrap();
    assert_eq!(h.view("a").await.title, "Payments");
    h.handle.rename(sid("a"), Some("  ".into())).await.unwrap();
    assert_eq!(h.view("a").await.title, "alpha");
    assert!(matches!(
        h.handle.rename(sid("nope"), None).await,
        Err(HubError::UnknownSession)
    ));
}

#[tokio::test]
async fn forget_refuses_a_live_session_and_removes_an_ended_one() {
    let mut h = harness();
    let (_, a) = h.register("a", "/w/a").await;
    h.handle.send("hi".into()).await.unwrap();
    assert!(matches!(h.handle.forget(sid("a")).await, Err(HubError::SessionLive)));
    h.command(Command::Unregister {
        id: sid("a"),
        conn: a.conn,
    })
    .await;
    h.events.take();
    h.handle.forget(sid("a")).await.unwrap();
    assert!(h.events.take().contains(&Event::Target(None)));
    assert!(h.handle.sessions().await.unwrap().is_empty());
    assert!(h.handle.history(sid("a"), None, 10).await.unwrap().is_empty());
    assert!(matches!(h.handle.forget(sid("a")).await, Err(HubError::UnknownSession)));
}

#[tokio::test]
async fn status_reports_port_live_count_and_target() {
    let mut h = harness();
    let (_, _a) = h.register("a", "/w/a").await;
    let status = h.handle.status().await.unwrap();
    assert_eq!(
        (status.port, status.live_sessions, status.target),
        (4242, 1, Some(sid("a")))
    );
}

#[tokio::test]
async fn clear_history_removes_every_message() {
    let mut h = harness();
    let (_, _a) = h.register("a", "/w/a").await;
    h.handle.send("hi".into()).await.unwrap();
    h.handle.clear_history().await.unwrap();
    assert!(h.handle.history(sid("a"), None, 10).await.unwrap().is_empty());
}

fn sessions_events(events: &[Event]) -> usize {
    events.iter().filter(|e| matches!(e, Event::Sessions(_))).count()
}

#[tokio::test]
async fn a_live_sessions_activity_shows_once_the_presence_knows_it() {
    let mut h = harness();
    let (_, a) = h.register("a", "/w/a").await;
    assert_eq!(h.view("a").await.activity, None);
    h.presence.set(a.pid, Activity::Busy, datetime!(2026-10-25 15:00 UTC));
    h.command(Command::Tick).await;
    assert_eq!(h.view("a").await.activity, Some(Activity::Busy));
}

#[tokio::test]
async fn activity_emits_sessions_once_per_change() {
    let mut h = harness();
    let (_, a) = h.register("a", "/w/a").await;
    h.presence.set(a.pid, Activity::Idle, datetime!(2026-10-25 15:00 UTC));
    h.command(Command::Tick).await;
    h.events.take();

    h.command(Command::Tick).await;
    h.presence.set(a.pid, Activity::Idle, datetime!(2026-10-25 15:01 UTC));
    h.command(Command::Tick).await;
    assert_eq!(sessions_events(&h.events.take()), 0, "same activity, no event");

    h.presence.set(a.pid, Activity::Busy, datetime!(2026-10-25 15:02 UTC));
    h.command(Command::Tick).await;
    h.command(Command::Tick).await;
    let events = h.events.take();
    assert_eq!(sessions_events(&events), 1);
    assert!(matches!(events.last(), Some(Event::Sessions(v)) if v[0].activity == Some(Activity::Busy)));
}

#[tokio::test]
async fn activity_is_dropped_when_the_session_ends() {
    let mut h = harness();
    let (_, a) = h.register("a", "/w/a").await;
    h.presence.set(a.pid, Activity::Busy, datetime!(2026-10-25 15:00 UTC));
    h.command(Command::Tick).await;
    h.command(Command::Unregister {
        id: sid("a"),
        conn: a.conn,
    })
    .await;
    assert_eq!(h.view("a").await.activity, None);
}

#[tokio::test]
async fn without_a_presence_activity_stays_unknown() {
    let mut h = build(HubConfig::new(Token::generate()), MemoryStore::new(), false);
    let (_, a) = h.register("a", "/w/a").await;
    h.presence.set(a.pid, Activity::Busy, datetime!(2026-10-25 15:00 UTC));
    h.command(Command::Tick).await;
    assert_eq!(h.view("a").await.activity, None);
}

fn pickup_harness() -> Harness {
    harness_config(MemoryStore::new(), |c| c.pickup_timeout = Duration::from_millis(30))
}

/// Registers a session with the given activity, sends it a message and acknowledges it.
async fn delivered_while(h: &mut Harness, activity: Activity) -> (Peer, MessageId) {
    let (_, a) = h.register("a", "/w/a").await;
    h.presence.set(a.pid, activity, datetime!(2026-10-25 15:00 UTC));
    h.command(Command::Tick).await;
    let message = h.handle.send("hello".into()).await.unwrap();
    h.command(Command::Delivered {
        session: sid("a"),
        id: message.id,
    })
    .await;
    h.events.take();
    (a, message.id)
}

fn unheard(id: MessageId) -> Event {
    Event::Updated(id, DeliveryStatus::Delivered, Some(NOT_PICKED_UP.to_owned()))
}

async fn after_the_pickup_timeout(h: &Harness) -> Vec<Event> {
    tokio::time::sleep(Duration::from_millis(50)).await;
    h.command(Command::Tick).await;
    h.command(Command::Tick).await;
    h.events.take()
}

#[tokio::test]
async fn a_message_an_idle_session_never_picks_up_is_flagged_once() {
    let mut h = pickup_harness();
    let (_a, id) = delivered_while(&mut h, Activity::Idle).await;
    h.command(Command::Tick).await;
    assert!(!h.events.take().contains(&unheard(id)), "not before the timeout");
    let events = after_the_pickup_timeout(&h).await;
    assert_eq!(events.iter().filter(|e| **e == unheard(id)).count(), 1);
    let stored = h.handle.history(sid("a"), None, 1).await.unwrap().remove(0);
    assert_eq!(
        (stored.status, stored.detail.as_deref()),
        (DeliveryStatus::Delivered, Some(NOT_PICKED_UP))
    );
}

#[tokio::test]
async fn any_activity_change_counts_as_heard_even_a_missed_busy_phase() {
    let mut h = pickup_harness();
    let (a, id) = delivered_while(&mut h, Activity::Idle).await;
    h.presence
        .set(a.pid, Activity::Idle, datetime!(2026-10-25 15:00:05 UTC));
    h.command(Command::Tick).await;
    assert!(!after_the_pickup_timeout(&h).await.contains(&unheard(id)));
}

#[tokio::test]
async fn a_message_delivered_mid_turn_is_not_watched() {
    let mut h = pickup_harness();
    let (_a, id) = delivered_while(&mut h, Activity::Busy).await;
    assert!(!after_the_pickup_timeout(&h).await.contains(&unheard(id)));
}
