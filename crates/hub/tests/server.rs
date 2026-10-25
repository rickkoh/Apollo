use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use hub::protocol::{ChannelFrame, ErrorCode, HubFrame};
use hub::store::MemoryStore;
use hub::{
    DeliveryStatus, Hub, HubConfig, HubEvents, Message, MessageId, RunningHub, SessionId, SessionInfo, SessionView,
    Token,
};
use time::OffsetDateTime;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::{self, Message as Ws};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

type Client = WebSocketStream<MaybeTlsStream<TcpStream>>;

struct Quiet;
impl HubEvents for Quiet {
    fn sessions_changed(&self, _: &[SessionView]) {}
    fn target_changed(&self, _: Option<&SessionId>) {}
    fn message_added(&self, _: &Message) {}
    fn message_updated(&self, _: MessageId, _: DeliveryStatus, _: Option<&str>) {}
}

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

async fn start(ping: Duration) -> RunningHub {
    let mut config = HubConfig::new(Token::from_hex(TOKEN));
    config.ping_interval = ping;
    config.hello_timeout = Duration::from_millis(300);
    Hub::start(config, Box::new(MemoryStore::new()), Arc::new(Quiet))
        .await
        .unwrap()
}

async fn connect(hub: &RunningHub) -> Client {
    tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{}/channel", hub.port))
        .await
        .unwrap()
        .0
}

fn hello(id: &str, token: &str, protocol: u32) -> Ws {
    let frame = ChannelFrame::Hello {
        protocol,
        token: token.into(),
        session: SessionInfo {
            id: SessionId(id.into()),
            cwd: format!("/w/{id}"),
            claude_pid: 7,
            started_at: OffsetDateTime::now_utc(),
        },
    };
    Ws::Text(serde_json::to_string(&frame).unwrap().into())
}

async fn next_frame(client: &mut Client) -> Option<HubFrame> {
    loop {
        match tokio::time::timeout(Duration::from_secs(3), client.next())
            .await
            .ok()??
        {
            Ok(Ws::Text(text)) => return Some(serde_json::from_str(text.as_str()).unwrap()),
            Ok(Ws::Close(_)) | Err(_) => return None,
            Ok(_) => {}
        }
    }
}

async fn expect_closed(client: &mut Client) {
    assert!(
        next_frame(client).await.is_none(),
        "expected the hub to close the connection"
    );
}

async fn live(hub: &RunningHub, id: &str) -> bool {
    hub.handle
        .sessions()
        .await
        .unwrap()
        .iter()
        .any(|s| s.id.as_str() == id && s.live)
}

#[tokio::test]
async fn hello_welcome_deliver_delivered_reply() {
    let hub = start(Duration::from_secs(15)).await;
    let mut client = connect(&hub).await;
    client.send(hello("abc", TOKEN, 1)).await.unwrap();
    assert_eq!(
        next_frame(&mut client).await,
        Some(HubFrame::Welcome { title: "abc".into() })
    );

    let sent = hub.handle.send("click Settle".into()).await.unwrap();
    let Some(HubFrame::Deliver { id, text, .. }) = next_frame(&mut client).await else {
        panic!("no deliver")
    };
    assert_eq!((id, text.as_str()), (sent.id, "click Settle"));

    client
        .send(Ws::Text(
            serde_json::to_string(&ChannelFrame::Delivered { id }).unwrap().into(),
        ))
        .await
        .unwrap();
    client
        .send(Ws::Text(r#"{"type":"reply","text":"Done."}"#.into()))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let history = hub.handle.history(SessionId("abc".into()), None, 10).await.unwrap();
    assert_eq!(history[0].status, DeliveryStatus::Delivered);
    assert_eq!(
        (history[1].text.as_str(), history[1].status),
        ("Done.", DeliveryStatus::Received)
    );
    hub.shutdown().await;
}

#[tokio::test]
async fn bad_token_is_refused() {
    let hub = start(Duration::from_secs(15)).await;
    let mut client = connect(&hub).await;
    client.send(hello("abc", &"f".repeat(64), 1)).await.unwrap();
    assert!(matches!(
        next_frame(&mut client).await,
        Some(HubFrame::Error {
            code: ErrorCode::BadToken,
            ..
        })
    ));
    expect_closed(&mut client).await;
    assert!(!live(&hub, "abc").await);
    hub.shutdown().await;
}

#[tokio::test]
async fn bad_protocol_is_refused() {
    let hub = start(Duration::from_secs(15)).await;
    let mut client = connect(&hub).await;
    client.send(hello("abc", TOKEN, 2)).await.unwrap();
    assert!(matches!(
        next_frame(&mut client).await,
        Some(HubFrame::Error {
            code: ErrorCode::BadProtocol,
            ..
        })
    ));
    expect_closed(&mut client).await;
    hub.shutdown().await;
}

#[tokio::test]
async fn no_hello_times_out() {
    let hub = start(Duration::from_secs(15)).await;
    let mut client = connect(&hub).await;
    assert!(matches!(
        next_frame(&mut client).await,
        Some(HubFrame::Error {
            code: ErrorCode::NoHello,
            ..
        })
    ));
    expect_closed(&mut client).await;
    hub.shutdown().await;
}

#[tokio::test]
async fn a_web_origin_is_refused_at_upgrade() {
    let hub = start(Duration::from_secs(15)).await;
    let mut request = format!("ws://127.0.0.1:{}/channel", hub.port)
        .into_client_request()
        .unwrap();
    request
        .headers_mut()
        .insert("Origin", "https://evil.example".parse().unwrap());
    match tokio_tungstenite::connect_async(request).await {
        Err(tungstenite::Error::Http(response)) => assert_eq!(response.status(), 403),
        other => panic!("expected 403, got {:?}", other.map(|_| ())),
    }
    hub.shutdown().await;
}

#[tokio::test]
async fn an_oversized_frame_closes_the_connection() {
    let hub = start(Duration::from_secs(15)).await;
    let mut client = connect(&hub).await;
    client.send(hello("abc", TOKEN, 1)).await.unwrap();
    next_frame(&mut client).await;
    let text = format!(r#"{{"type":"reply","text":"{}"}}"#, "x".repeat(100 * 1024));
    client.send(Ws::Text(text.into())).await.unwrap();
    assert!(matches!(
        next_frame(&mut client).await,
        Some(HubFrame::Error {
            code: ErrorCode::TooLarge,
            ..
        })
    ));
    expect_closed(&mut client).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!live(&hub, "abc").await);
    hub.shutdown().await;
}

#[tokio::test]
async fn a_peer_that_stops_answering_pings_is_dropped() {
    let hub = start(Duration::from_millis(100)).await;
    let mut client = connect(&hub).await;
    client.send(hello("abc", TOKEN, 1)).await.unwrap();
    next_frame(&mut client).await;
    assert!(live(&hub, "abc").await);
    // Not reading means tungstenite never answers the pings.
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(!live(&hub, "abc").await);
    hub.shutdown().await;
}

#[tokio::test]
async fn a_peer_that_answers_pings_stays() {
    let hub = start(Duration::from_millis(100)).await;
    let mut client = connect(&hub).await;
    client.send(hello("abc", TOKEN, 1)).await.unwrap();
    let reader = tokio::spawn(async move { while client.next().await.is_some() {} });
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(live(&hub, "abc").await);
    hub.shutdown().await;
    reader.abort();
}

#[tokio::test]
async fn a_second_connection_replaces_the_first() {
    let hub = start(Duration::from_secs(15)).await;
    let mut first = connect(&hub).await;
    first.send(hello("abc", TOKEN, 1)).await.unwrap();
    next_frame(&mut first).await;
    let mut second = connect(&hub).await;
    second.send(hello("abc", TOKEN, 1)).await.unwrap();
    next_frame(&mut second).await;
    assert!(matches!(
        next_frame(&mut first).await,
        Some(HubFrame::Error {
            code: ErrorCode::Replaced,
            ..
        })
    ));
    expect_closed(&mut first).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(live(&hub, "abc").await);
    hub.shutdown().await;
}

#[tokio::test]
async fn shutdown_closes_channels_normally() {
    let hub = start(Duration::from_secs(15)).await;
    let mut client = connect(&hub).await;
    client.send(hello("abc", TOKEN, 1)).await.unwrap();
    next_frame(&mut client).await;
    hub.shutdown().await;
    expect_closed(&mut client).await;
}
