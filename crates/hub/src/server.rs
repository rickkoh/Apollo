use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use axum::Router;
use axum::extract::State;
use axum::extract::ws::{CloseFrame, Message as WsMessage, WebSocket, WebSocketUpgrade, close_code};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{MissedTickBehavior, interval, timeout};

use crate::HubConfig;
use crate::actor::{Command, Outbound};
use crate::domain::SessionInfo;
use crate::protocol::{ChannelFrame, ErrorCode, HubFrame, MAX_FRAME_BYTES, PROTOCOL_VERSION};

const MISSED_PONGS_ALLOWED: u32 = 2;

#[derive(Clone)]
pub(crate) struct ServerState {
    pub commands: mpsc::Sender<Command>,
    pub config: Arc<HubConfig>,
    pub next_conn: Arc<AtomicU64>,
}

pub(crate) fn router(state: ServerState) -> Router {
    Router::new().route("/channel", get(upgrade)).with_state(state)
}

fn is_web_origin(headers: &HeaderMap) -> bool {
    headers.get(header::ORIGIN).is_some_and(|origin| {
        let origin = origin.as_bytes();
        origin.starts_with(b"http://") || origin.starts_with(b"https://")
    })
}

async fn upgrade(ws: WebSocketUpgrade, headers: HeaderMap, State(state): State<ServerState>) -> Response {
    if is_web_origin(&headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    ws.max_message_size(MAX_FRAME_BYTES * 4)
        .on_upgrade(move |socket| connection(socket, state))
}

type Sink = SplitSink<WebSocket, WsMessage>;

async fn send_frame(sink: &mut Sink, frame: &HubFrame) -> bool {
    sink.send(WsMessage::Text(frame.to_json().into())).await.is_ok()
}

async fn close(sink: &mut Sink, code: u16) {
    let _ = sink
        .send(WsMessage::Close(Some(CloseFrame {
            code,
            reason: "".into(),
        })))
        .await;
}

async fn reject(sink: &mut Sink, code: ErrorCode, message: &str) {
    tracing::warn!(?code, "channel rejected: {message}");
    send_frame(sink, &HubFrame::error(code, message)).await;
    close(sink, close_code::POLICY).await;
}

async fn connection(socket: WebSocket, state: ServerState) {
    let (mut sink, mut stream) = socket.split();

    let info: SessionInfo = match timeout(state.config.hello_timeout, stream.next()).await {
        Err(_) => return reject(&mut sink, ErrorCode::NoHello, "no hello within the timeout").await,
        Ok(None | Some(Err(_))) => return,
        Ok(Some(Ok(WsMessage::Text(text)))) if text.len() > MAX_FRAME_BYTES => {
            return reject(&mut sink, ErrorCode::TooLarge, "frame too large").await;
        }
        Ok(Some(Ok(WsMessage::Text(text)))) => match serde_json::from_str::<ChannelFrame>(text.as_str()) {
            Ok(ChannelFrame::Hello { token, .. }) if !state.config.token.matches(&token) => {
                return reject(&mut sink, ErrorCode::BadToken, "token mismatch").await;
            }
            Ok(ChannelFrame::Hello { protocol, .. }) if protocol != PROTOCOL_VERSION => {
                return reject(&mut sink, ErrorCode::BadProtocol, "unsupported protocol version").await;
            }
            Ok(ChannelFrame::Hello { session, .. }) => session,
            _ => return reject(&mut sink, ErrorCode::NoHello, "the first frame must be hello").await,
        },
        Ok(Some(Ok(_))) => return reject(&mut sink, ErrorCode::NoHello, "the first frame must be hello").await,
    };

    let id = info.id.clone();
    let conn = state.next_conn.fetch_add(1, Ordering::Relaxed);
    let (link, mut outbound) = mpsc::unbounded_channel();
    let (answer, registered) = oneshot::channel();
    if state
        .commands
        .send(Command::Register {
            info,
            conn,
            link,
            answer,
        })
        .await
        .is_err()
    {
        return close(&mut sink, close_code::AWAY).await;
    }
    let Ok(Ok(title)) = registered.await else {
        return close(&mut sink, close_code::ERROR).await;
    };
    if !send_frame(&mut sink, &HubFrame::Welcome { title }).await {
        let _ = state.commands.send(Command::Unregister { id, conn }).await;
        return;
    }

    let mut ping = interval(state.config.ping_interval);
    ping.set_missed_tick_behavior(MissedTickBehavior::Delay);
    ping.tick().await;
    let mut missed = 0;

    loop {
        tokio::select! {
            frame = stream.next() => match frame {
                None | Some(Err(_)) | Some(Ok(WsMessage::Close(_))) => break,
                Some(Ok(WsMessage::Text(text))) if text.len() > MAX_FRAME_BYTES => {
                    reject(&mut sink, ErrorCode::TooLarge, "frame too large").await;
                    break;
                }
                Some(Ok(WsMessage::Binary(bytes))) if bytes.len() > MAX_FRAME_BYTES => {
                    reject(&mut sink, ErrorCode::TooLarge, "frame too large").await;
                    break;
                }
                Some(Ok(WsMessage::Text(text))) => {
                    let command = match serde_json::from_str::<ChannelFrame>(text.as_str()) {
                        Ok(ChannelFrame::Delivered { id: message }) => Command::Delivered { session: id.clone(), id: message },
                        Ok(ChannelFrame::Reply { text }) => Command::Reply { session: id.clone(), text },
                        Ok(ChannelFrame::Hello { .. }) | Err(_) => {
                            tracing::warn!(session = %id, "ignoring an unexpected frame");
                            continue;
                        }
                    };
                    if state.commands.send(command).await.is_err() {
                        break;
                    }
                }
                Some(Ok(WsMessage::Pong(_))) => missed = 0,
                Some(Ok(_)) => {}
            },
            out = outbound.recv() => match out {
                Some(Outbound::Frame(frame)) => {
                    if !send_frame(&mut sink, &frame).await {
                        break;
                    }
                }
                Some(Outbound::Close) | None => {
                    close(&mut sink, close_code::NORMAL).await;
                    break;
                }
            },
            _ = ping.tick() => {
                if missed >= MISSED_PONGS_ALLOWED {
                    tracing::warn!(session = %id, "closing a connection that stopped answering pings");
                    close(&mut sink, close_code::AWAY).await;
                    break;
                }
                missed += 1;
                if sink.send(WsMessage::Ping(Default::default())).await.is_err() {
                    break;
                }
            }
        }
    }

    let _ = state.commands.send(Command::Unregister { id, conn }).await;
}
