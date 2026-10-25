mod actor;
pub mod discovery;
mod domain;
mod error;
mod events;
pub mod presence;
pub mod protocol;
mod server;
pub mod store;

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::Duration;

use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

pub use domain::*;
pub use error::{HubError, SendError, StoreError};
pub use events::HubEvents;
pub use presence::{Activity, Presence};
pub use store::Store;

use actor::{Command, HubActor};

const COMMAND_BUFFER: usize = 256;
const TICK: Duration = Duration::from_secs(1);
const SERVER_STOP_GRACE: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
pub struct HubConfig {
    pub token: Token,
    pub delivery_timeout: Duration,
    pub ping_interval: Duration,
    pub hello_timeout: Duration,
    pub max_text_bytes: usize,
    /// How long a target restored at start keeps priority while it reconnects; above the channel's 10 s retry.
    pub restore_grace: Duration,
    /// How long a message delivered to an idle session may go without starting a turn before it is flagged.
    pub pickup_timeout: Duration,
}

impl HubConfig {
    pub fn new(token: Token) -> Self {
        Self {
            token,
            delivery_timeout: Duration::from_secs(5),
            ping_interval: Duration::from_secs(15),
            hello_timeout: Duration::from_secs(2),
            max_text_bytes: 16 * 1024,
            restore_grace: Duration::from_secs(15),
            pickup_timeout: Duration::from_secs(10),
        }
    }
}

pub struct Hub;

impl Hub {
    /// Recovers queued rows left by a crash, binds `127.0.0.1:0` and starts the actor and the server.
    pub async fn start(
        config: HubConfig,
        mut store: Box<dyn Store>,
        presence: Option<Box<dyn Presence>>,
        events: Arc<dyn HubEvents>,
    ) -> Result<RunningHub, HubError> {
        store.fail_all_queued("app restarted")?;
        let listener = tokio::net::TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).await?;
        let port = listener.local_addr()?.port();
        let config = Arc::new(config);

        let (commands, inbox) = mpsc::channel(COMMAND_BUFFER);
        let actor = HubActor::new(config.clone(), port, store, presence, events)?;
        let actor = std::thread::Builder::new()
            .name("hub-actor".into())
            .spawn(move || actor.run(inbox))?;

        let ticker = {
            let commands = commands.clone();
            tokio::spawn(async move {
                let mut tick = tokio::time::interval(TICK);
                loop {
                    tick.tick().await;
                    if commands.send(Command::Tick).await.is_err() {
                        break;
                    }
                }
            })
        };

        let (stop, stopped) = oneshot::channel::<()>();
        let state = server::ServerState {
            commands: commands.clone(),
            config,
            next_conn: Arc::new(AtomicU64::new(1)),
        };
        let server = tokio::spawn(async move {
            let serve = axum::serve(listener, server::router(state)).with_graceful_shutdown(async {
                let _ = stopped.await;
            });
            if let Err(err) = serve.await {
                tracing::error!("hub server stopped: {err}");
            }
        });

        tracing::info!(port, "hub started");
        Ok(RunningHub {
            handle: HubHandle { commands },
            port,
            stop,
            server,
            ticker,
            actor,
        })
    }
}

pub struct RunningHub {
    pub handle: HubHandle,
    pub port: u16,
    stop: oneshot::Sender<()>,
    server: JoinHandle<()>,
    ticker: JoinHandle<()>,
    actor: std::thread::JoinHandle<()>,
}

impl RunningHub {
    /// Closes every channel connection normally, then stops the server and the actor.
    pub async fn shutdown(self) {
        let (answer, done) = oneshot::channel();
        if self.handle.commands.send(Command::Shutdown { answer }).await.is_ok() {
            let _ = done.await;
        }
        self.ticker.abort();
        let _ = self.stop.send(());
        if tokio::time::timeout(SERVER_STOP_GRACE, self.server).await.is_err() {
            tracing::warn!("hub server did not stop in time");
        }
        let _ = tokio::task::spawn_blocking(move || self.actor.join()).await;
        tracing::info!("hub stopped");
    }
}

#[derive(Clone)]
pub struct HubHandle {
    commands: mpsc::Sender<Command>,
}

impl HubHandle {
    async fn ask<T>(&self, make: impl FnOnce(oneshot::Sender<T>) -> Command) -> Option<T> {
        let (answer, response) = oneshot::channel();
        self.commands.send(make(answer)).await.ok()?;
        response.await.ok()
    }

    pub async fn status(&self) -> Result<HubStatus, HubError> {
        self.ask(|answer| Command::Status { answer })
            .await
            .ok_or(HubError::Stopped)
    }

    pub async fn sessions(&self) -> Result<Vec<SessionView>, HubError> {
        self.ask(|answer| Command::Sessions { answer })
            .await
            .unwrap_or(Err(HubError::Stopped))
    }

    pub async fn set_target(&self, id: SessionId) -> Result<(), HubError> {
        self.ask(|answer| Command::SetTarget { id, answer })
            .await
            .unwrap_or(Err(HubError::Stopped))
    }

    pub async fn send(&self, text: String) -> Result<Message, SendError> {
        self.ask(|answer| Command::Send { text, answer })
            .await
            .unwrap_or(Err(SendError::Stopped))
    }

    pub async fn history(
        &self,
        id: SessionId,
        before: Option<MessageId>,
        limit: u32,
    ) -> Result<Vec<Message>, HubError> {
        self.ask(|answer| Command::History {
            id,
            before,
            limit,
            answer,
        })
        .await
        .unwrap_or(Err(HubError::Stopped))
    }

    pub async fn recent(&self, limit: u32) -> Result<Vec<Message>, HubError> {
        self.ask(|answer| Command::Recent { limit, answer })
            .await
            .unwrap_or(Err(HubError::Stopped))
    }

    pub async fn mark_read(&self, id: SessionId, upto: MessageId) -> Result<(), HubError> {
        self.ask(|answer| Command::MarkRead { id, upto, answer })
            .await
            .unwrap_or(Err(HubError::Stopped))
    }

    pub async fn rename(&self, id: SessionId, title: Option<String>) -> Result<(), HubError> {
        self.ask(|answer| Command::Rename { id, title, answer })
            .await
            .unwrap_or(Err(HubError::Stopped))
    }

    pub async fn forget(&self, id: SessionId) -> Result<(), HubError> {
        self.ask(|answer| Command::Forget { id, answer })
            .await
            .unwrap_or(Err(HubError::Stopped))
    }

    pub async fn clear_history(&self) -> Result<(), HubError> {
        self.ask(|answer| Command::ClearHistory { answer })
            .await
            .unwrap_or(Err(HubError::Stopped))
    }
}
