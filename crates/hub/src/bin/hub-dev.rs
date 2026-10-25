//! Headless hub with a stdin prompt: the phase 2 test bench and the end-to-end tests' hub.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use hub::presence::ClaudeSessions;
use hub::store::SqliteStore;
use hub::{
    Activity, DeliveryStatus, Direction, Hub, HubConfig, HubEvents, HubHandle, Message, MessageId, SessionId,
    SessionView, Token, discovery,
};
use tokio::io::{AsyncBufReadExt, BufReader};

const USAGE: &str = "usage: hub-dev --data <dir> --discovery <path>";
const HELP: &str = "commands: sessions | target <id-prefix> | send <text> | history <id-prefix> | quit";

/// Prints what a UI would show; keeps titles to tag replies with their session.
#[derive(Default)]
struct Printer {
    titles: Mutex<HashMap<SessionId, String>>,
}

impl HubEvents for Printer {
    fn sessions_changed(&self, sessions: &[SessionView]) {
        let mut titles = self.titles.lock().unwrap();
        for s in sessions {
            titles.insert(s.id.clone(), s.title.clone());
        }
    }

    fn target_changed(&self, target: Option<&SessionId>) {
        println!("target: {}", target.map_or("none".into(), |t| t.to_string()));
    }

    fn message_added(&self, message: &Message) {
        if message.direction == Direction::FromSession {
            let title = self
                .titles
                .lock()
                .unwrap()
                .get(&message.session)
                .cloned()
                .unwrap_or_default();
            println!("reply [{title} {}] {}", message.session, message.text);
        }
    }

    fn message_updated(&self, id: MessageId, status: DeliveryStatus, detail: Option<&str>) {
        println!(
            "message {} {status:?}{}",
            id.0,
            detail.map(|d| format!(": {d}")).unwrap_or_default()
        );
    }
}

fn args() -> Result<(PathBuf, PathBuf), String> {
    let mut data = None;
    let mut discovery = None;
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--data" => data = it.next().map(PathBuf::from),
            "--discovery" => discovery = it.next().map(PathBuf::from),
            other => return Err(format!("unknown argument {other}\n{USAGE}")),
        }
    }
    Ok((data.ok_or(USAGE)?, discovery.ok_or(USAGE)?))
}

async fn find(handle: &HubHandle, prefix: &str) -> Option<SessionId> {
    let sessions = handle.sessions().await.ok()?;
    let matches: Vec<_> = sessions
        .into_iter()
        .filter(|s| s.id.as_str().starts_with(prefix))
        .collect();
    match &matches[..] {
        [one] => Some(one.id.clone()),
        [] => {
            println!("no session starts with {prefix}");
            None
        }
        _ => {
            println!("{prefix} matches more than one session");
            None
        }
    }
}

async fn run_command(handle: &HubHandle, line: &str) {
    let (command, rest) = line.split_once(' ').unwrap_or((line, ""));
    match command {
        "sessions" => match handle.sessions().await {
            Ok(sessions) if sessions.is_empty() => println!("no sessions"),
            Ok(sessions) => {
                for s in sessions {
                    let state = match (s.live, s.activity) {
                        (false, _) => "ended",
                        (true, Some(Activity::Busy)) => "live, busy",
                        (true, Some(Activity::Idle)) => "live, idle",
                        (true, None) => "live",
                    };
                    let target = if s.is_target { " *target*" } else { "" };
                    println!("{} {} [{state}] {} unread={}{target}", s.id, s.title, s.cwd, s.unread);
                }
            }
            Err(err) => println!("error: {err}"),
        },
        "target" => {
            if let Some(id) = find(handle, rest.trim()).await {
                match handle.set_target(id).await {
                    Ok(()) => {}
                    Err(err) => println!("error: {err}"),
                }
            }
        }
        "send" => match handle.send(rest.to_owned()).await {
            Ok(m) => println!("message {} queued for {}", m.id.0, m.session),
            Err(err) => println!("error: {err}"),
        },
        "history" => {
            if let Some(id) = find(handle, rest.trim()).await {
                for m in handle.history(id, None, 50).await.unwrap_or_default() {
                    let arrow = if m.direction == Direction::ToSession {
                        "->"
                    } else {
                        "<-"
                    };
                    println!("{} {arrow} {} ({:?})", m.id.0, m.text, m.status);
                }
            }
        }
        "" => {}
        _ => println!("{HELP}"),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    let (data, discovery_path) = args()?;
    std::fs::create_dir_all(&data)?;

    let token = Token::generate();
    let store = SqliteStore::open(&data.join("voice.db"))?;
    let claude_dir = std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::home_dir().map(|home| home.join(".claude")))
        .ok_or("no home folder")?;
    let running = Hub::start(
        HubConfig::new(token.clone()),
        Box::new(store),
        Some(Box::new(ClaudeSessions::new(claude_dir.join("sessions")))),
        Arc::new(Printer::default()),
    )
    .await?;
    discovery::write(&discovery_path, running.port, &token)?;
    println!("hub-dev listening on 127.0.0.1:{}; {HELP}", running.port);

    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    loop {
        tokio::select! {
            line = lines.next_line() => match line? {
                Some(line) if line.trim() == "quit" => break,
                Some(line) => run_command(&running.handle, line.trim()).await,
                None => break,
            },
            _ = tokio::signal::ctrl_c() => break,
        }
    }

    running.shutdown().await;
    discovery::remove(&discovery_path)?;
    Ok(())
}
