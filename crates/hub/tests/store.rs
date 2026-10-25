use hub::store::{NewMessage, SqliteStore, Store};
use hub::{DeliveryStatus, Direction, MessageId, SessionId, SessionInfo};
use time::{Duration, OffsetDateTime};

fn sid(s: &str) -> SessionId {
    SessionId(s.into())
}

fn info(id: &str) -> SessionInfo {
    SessionInfo {
        id: sid(id),
        cwd: format!("/w/{id}"),
        claude_pid: 1,
        started_at: OffsetDateTime::now_utc(),
    }
}

fn message(session: &str, text: &str, at: OffsetDateTime, status: DeliveryStatus) -> NewMessage {
    let direction = if status == DeliveryStatus::Received {
        Direction::FromSession
    } else {
        Direction::ToSession
    };
    NewMessage {
        session: sid(session),
        direction,
        text: text.into(),
        at,
        status,
        detail: None,
    }
}

fn open(dir: &tempfile::TempDir) -> SqliteStore {
    SqliteStore::open(&dir.path().join("voice.db")).unwrap()
}

#[test]
fn migrates_from_empty_and_reopens() {
    let dir = tempfile::tempdir().unwrap();
    let now = OffsetDateTime::now_utc();
    {
        let mut store = open(&dir);
        store.upsert_session(&info("a"), now).unwrap();
        store.set_target(Some(&sid("a"))).unwrap();
    }
    let store = open(&dir);
    assert_eq!(store.sessions().unwrap().len(), 1);
    assert_eq!(store.target().unwrap(), Some(sid("a")));
    let conn = rusqlite::Connection::open(dir.path().join("voice.db")).unwrap();
    let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0)).unwrap();
    assert_eq!(version, 1);
}

#[test]
fn history_pages_backwards_oldest_first() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(&dir);
    let now = OffsetDateTime::now_utc();
    store.upsert_session(&info("a"), now).unwrap();
    store.upsert_session(&info("b"), now).unwrap();
    let ids: Vec<MessageId> = (0..5)
        .map(|i| {
            store
                .insert_message(message("a", &format!("m{i}"), now, DeliveryStatus::Delivered))
                .unwrap()
                .id
        })
        .collect();
    store
        .insert_message(message("b", "other", now, DeliveryStatus::Delivered))
        .unwrap();

    let latest = store.history(&sid("a"), None, 2).unwrap();
    assert_eq!(latest.iter().map(|m| m.text.as_str()).collect::<Vec<_>>(), ["m3", "m4"]);
    let older = store.history(&sid("a"), Some(ids[3]), 10).unwrap();
    assert_eq!(
        older.iter().map(|m| m.text.as_str()).collect::<Vec<_>>(),
        ["m0", "m1", "m2"]
    );
    assert_eq!(latest[0].at.unix_timestamp(), now.unix_timestamp());
}

#[test]
fn forget_cascades_to_messages() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(&dir);
    let now = OffsetDateTime::now_utc();
    store.upsert_session(&info("a"), now).unwrap();
    store
        .insert_message(message("a", "hi", now, DeliveryStatus::Delivered))
        .unwrap();
    store.forget(&sid("a")).unwrap();
    assert!(store.sessions().unwrap().is_empty());
    assert!(store.history(&sid("a"), None, 10).unwrap().is_empty());
}

#[test]
fn purge_removes_only_older_messages() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(&dir);
    let now = OffsetDateTime::now_utc();
    store.upsert_session(&info("a"), now).unwrap();
    store
        .insert_message(message("a", "old", now - Duration::days(40), DeliveryStatus::Delivered))
        .unwrap();
    store
        .insert_message(message("a", "new", now, DeliveryStatus::Delivered))
        .unwrap();
    assert_eq!(store.purge_before(now - Duration::days(30)).unwrap(), 1);
    assert_eq!(store.history(&sid("a"), None, 10).unwrap()[0].text, "new");
}

#[test]
fn queued_rows_left_by_a_crash_become_failed() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(&dir);
    let now = OffsetDateTime::now_utc();
    store.upsert_session(&info("a"), now).unwrap();
    store
        .insert_message(message("a", "stuck", now, DeliveryStatus::Queued))
        .unwrap();
    store
        .insert_message(message("a", "fine", now, DeliveryStatus::Delivered))
        .unwrap();
    assert_eq!(store.fail_all_queued("app restarted").unwrap(), 1);
    let history = store.history(&sid("a"), None, 10).unwrap();
    assert_eq!(
        (history[0].status, history[0].detail.as_deref()),
        (DeliveryStatus::Failed, Some("app restarted"))
    );
    assert_eq!(history[1].status, DeliveryStatus::Delivered);
}

#[test]
fn unread_counts_replies_after_last_read() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(&dir);
    let now = OffsetDateTime::now_utc();
    store.upsert_session(&info("a"), now).unwrap();
    store
        .insert_message(message("a", "sent", now, DeliveryStatus::Delivered))
        .unwrap();
    let first = store
        .insert_message(message("a", "r1", now, DeliveryStatus::Received))
        .unwrap();
    store
        .insert_message(message("a", "r2", now, DeliveryStatus::Received))
        .unwrap();
    assert_eq!(store.sessions().unwrap()[0].unread, 2);
    store.mark_read(&sid("a"), first.id).unwrap();
    assert_eq!(store.sessions().unwrap()[0].unread, 1);
}

#[test]
fn rename_persists() {
    let dir = tempfile::tempdir().unwrap();
    let now = OffsetDateTime::now_utc();
    {
        let mut store = open(&dir);
        store.upsert_session(&info("a"), now).unwrap();
        store.rename(&sid("a"), Some("Payments")).unwrap();
    }
    assert_eq!(open(&dir).sessions().unwrap()[0].title.as_deref(), Some("Payments"));
}
