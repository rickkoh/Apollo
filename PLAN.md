# Voice Bridge — design

Voice Bridge (working name) is a macOS menu-bar app with an always-on-top overlay. The user dictates into the overlay
with Wispr Flow; the text goes to one chosen Claude Code session through a Claude Code channel, and Claude's replies
show in the overlay. A control panel manages sessions, history and settings.

This file is the source of truth for the design. Decisions are numbered `D`, open questions `Q`, requirements `R`.
Record every new decision and question here.

## 1. Requirements

- **R1 Overlay.** Always on top: on every desktop Space and over full-screen apps, the way Wispr Flow's pill is.
  Bottom-right of the screen (Wispr Flow's pill sits at bottom centre). No window buttons anywhere. The text box and
  controls are floating "dynamic island" pills on a transparent window, not a panel with a background.
- **R2 Replies.** Claude's replies appear in the overlay as they arrive, truncated when long.
- **R3 Control panel.** A main window patterned on Wispr Flow's: sidebar navigation, content cards, the same calm
  look, more modern. Animation is not a priority.
- **R4 Menu bar.** An icon at the top right of the macOS menu bar opens a menu: open the control panel, show or hide
  the overlay, quit.
- **R5 Dock.** No Dock icon, except while the control panel is open.
- **R6 Sessions.** The app lists the Claude Code sessions that can receive voice; the user picks exactly one target.
  Messages go only to the target.
- **R7 Persistence.** Sessions, conversation history, the target and settings live in local app data (SQLite and
  files) and survive restarts. This is the app's own data, not agent memory.
- **R8 Dictation.** Clicking the overlay's text box and dictating with Wispr Flow sends at once; typed text sends on
  Enter.
- **R9 Shortcuts.** A Shortcuts page exists as a skeleton (for example a hotkey that focuses the text box); the
  shortcuts themselves come later.
- **R10 No Remote Control.** Everything works with Remote Control switched off; the organization disables it.
- **R11 Quality.** Designed at a low level, SOLID, no code smells, ready to extend with features, robustness and tests.
- **R12 Home.** Lives in `~/Desktop/voice-bridge`; the user sets up git and a remote.

## 2. Decisions

- **D1 Wispr Flow, not Pipecat.** Better transcription, and the user decides when a message is complete; Pipecat's
  pause-to-send sends unfinished prompts.
- **D2 Messages reach sessions through Claude Code channels** (research preview).
- **D3 A paste is read from the paste event's `clipboardData` and the default insertion is cancelled.** From Wispr
  Flow's second dictation on, the web view's own insertion comes up empty although the event carries the text. Holds
  in Chrome and in Tauri's WebKit.
- **D4 The reply tool is `alwaysLoad`, and the channel instructions lead with "plain text never reaches the user".**
  Deferred behind tool search, Claude answered in the terminal instead.
- **D5 No dependency on Remote Control.** Channels work with it switched off.
- **D6 A proper hub with a voice plugin, no Agent SDK.** Sessions stay normal terminal Claude Code sessions.
- **D7 The overlay and control panel are one Tauri v2 app.**
- **D8 The plugin loads per session with `--plugin-dir`, never installed.** Its channel is `plugin:voice@inline` in
  the flag, so the launcher is the only way in and no other session runs the channel.
- **D9 A channel identifies its session from `CLAUDE_CODE_SESSION_ID` and `CLAUDE_PROJECT_DIR`,** which Claude Code
  sets on MCP server processes (observed, not documented).
- **D10 The hub is a Rust library crate that runs inside the Tauri app.** One process, no sidecar to supervise and no
  Node runtime to locate from a GUI app. The crate has no Tauri dependency, so it also runs headless in tests and in a
  dev binary.
- **D11 Channel ↔ hub is a WebSocket with typed JSON frames (protocol v1).** One connection per session carries
  registration, delivery, acknowledgement and replies; the open connection is the session's presence.
- **D12 The hub binds a free port on 127.0.0.1 and publishes it with a token in a discovery file.** No fixed port to
  collide with, and a web page cannot impersonate a session.
- **D13 The UI talks to the hub only through Tauri commands and events.** The UI has no HTTP surface.
- **D14 Conversation history and hub state (the target) live in SQLite; preferences live in `settings.json`.**
- **D15 Frontend is Vue 3, TypeScript and Vite.** Two entry pages: overlay and control panel.
- **D16 Wire and UI types are defined once in Rust and generated for TypeScript with `ts-rs`,** so the hub, the
  channel and the UI cannot drift.
- **D17 One target session at a time.** A send goes to the target or fails visibly; nothing is broadcast or silently
  queued for later.
- **D18 The app installs a `cv` launcher into `~/.local/bin`,** the way editors install their command-line helper.
- **D19 The overlay window hugs its islands and resizes with them.** Its only transparent area is a small margin for
  shadows, so no click-through handling is needed yet.
- **D20 The overlay shows over full-screen apps** through native window settings Tauri doesn't expose: the
  `FullScreenAuxiliary` collection behaviour and a window level above normal windows. Spike S3 decides between a
  regular window and a non-activating panel.
- **D21 All app data lives in `~/Library/Application Support/dev.voicebridge.desktop/`** (layout in §8.1).
- **D22 Bundle identifier `dev.voicebridge.desktop`;** changeable before the first signed build.
- **D23 The app keeps a standard Edit menu.** Wispr Flow pastes with ⌘V, and macOS routes ⌘V through the main menu.
- **D24 Pinned stack: latest stable, not pre-release.** Tauri 2.12.x and its 2.x plugins (Tauri 3 is alpha), axum 0.8,
  rusqlite 0.41 (bundled), ts-rs 12, tokio 1.54, tokio-tungstenite 0.30, objc2-app-kit 0.3, Vue 3.5, vue-router 5,
  Vite 8, vue-tsc 3, MCP SDK 1.32, esbuild 0.28, TypeScript 5.9 (TS 7 is too new for vue-tsc).
- **D25 Q12: the hub refuses the upgrade only for an `http(s)` `Origin`** (§5.6 unchanged); a server test proves Node's
  built-in `WebSocket` passes.
- **D26 Q13: primary display only** until phase 5.
- **D27 Q14 placeholder: a generated "island wave"** (pill with three rising bars) as SVG → `.icns` and a monochrome
  menu-bar template; replaced by the final logo in phase 5.
- **D28 Q15 decision rule: the non-activating panel only if it passes every S3 step,** otherwise the regular window.
- **D29 Debug builds get a "Stop hub" command** (menu-bar item, debug only) so the hub-down state (red dot) can be
  observed while the app runs.
- **D30 ts-rs exports from every crate into one `shared/types/`** via `TS_RS_EXPORT_DIR` in `.cargo/config.toml`,
  including the shell's `Settings`, `UiError` and `CliStatus`.

## 3. Architecture

```
                  ┌──────────────────────── Voice Bridge.app ────────────────────────┐
 Wispr Flow ─paste─▶ overlay window (Vue) ─┐                                          │
                  │  control panel (Vue) ──┼─ Tauri commands/events ─▶ desktop shell ─┤
                  │                        │                          (windows, tray, │
                  │                        │                           settings, CLI) │
                  │                        └──────────────▶ hub crate (actor, store) ─┤
                  └───────────────────────────────────────────── ws://127.0.0.1:port ─┘
                                                                          ▲   one WebSocket
                                                                          │   per session
              terminal: cv ─▶ claude ─spawns─▶ voice channel (Node, stdio MCP) ┘
```

| Part | Language | Owns | Knows nothing about |
|---|---|---|---|
| Desktop shell | Rust (Tauri) | windows, menu bar, Dock policy, settings, `cv` installer; composition root | sockets, SQL |
| Hub | Rust crate | live sessions, target, routing, delivery state, history | Tauri, windows |
| Voice channel | TypeScript → one JS bundle | relaying between one Claude Code session and the hub | the UI |
| UI | Vue 3 | rendering hub state and settings; capturing input | storage, sockets |

### 3.1 Message flow

1. The user dictates into the overlay; the paste handler calls `send(text)`.
2. The hub checks the target is live, stores the message as `queued`, and pushes a `deliver` frame to that session.
3. The channel emits `notifications/claude/channel`, then answers `delivered`; the hub marks the message
   `delivered` and emits `message-updated`.
4. Claude calls the channel's `reply` tool; the channel sends a `reply` frame; the hub stores it as `received` and
   emits `message-added`.
5. The overlay shows the reply in the reply island; the control panel's history updates.

### 3.2 Startup

1. Load settings (defaults for missing keys).
2. Open `voice.db`, run migrations, purge messages older than the retention setting.
3. Start the hub on `127.0.0.1:0` with a fresh token; write `hub.json`.
4. Set the activation policy to `Accessory`; build the menu bar item and the app menu.
5. Create the overlay (hidden), apply the native full-screen settings, position it, show it if
   `overlay.visibleAtLaunch`.
6. Channels already retrying (sessions started while the app was closed) register within one backoff step.

### 3.3 Shutdown

1. Quit from the menu bar (or ⌘Q while the panel is focused).
2. The hub closes every channel connection with a normal close; channels go back to retrying.
3. Remove `hub.json`; flush SQLite; exit.

## 4. Repository layout

```
voice-bridge/
  Cargo.toml                      workspace: crates/hub, apps/desktop/src-tauri
  rustfmt.toml
  shared/types/                   ts-rs output (committed), imported by the plugin and the UI
  crates/hub/
    Cargo.toml
    src/lib.rs                    public API: Hub, HubConfig, RunningHub, HubHandle, domain types
    src/domain.rs
    src/protocol.rs
    src/actor.rs
    src/server.rs
    src/store/mod.rs              Store trait
    src/store/sqlite.rs           SqliteStore + migrations
    src/store/memory.rs           MemoryStore for tests
    src/events.rs                 HubEvents trait
    src/discovery.rs
    src/error.rs
    src/bin/hub-dev.rs            headless hub with a stdin command prompt
    tests/                        server and end-to-end tests
  apps/desktop/
    package.json, vite.config.ts, tsconfig.json
    overlay.html, panel.html      Vite entry pages
    src/shared/ src/overlay/ src/panel/
    src-tauri/
      Cargo.toml, build.rs, tauri.conf.json
      capabilities/overlay.json, capabilities/panel.json
      icons/
      src/main.rs                 calls lib::run
      src/lib.rs                  composition root
      src/state.rs                AppState (HubHandle, Settings)
      src/settings.rs             Settings service
      src/commands.rs             Tauri commands
      src/events.rs               HubEvents → Tauri emitter
      src/windows/overlay.rs      overlay window: build, position, resize
      src/windows/panel.rs        panel window: show/hide, Dock policy
      src/tray.rs                 menu bar item and menu
      src/menu.rs                 app menu (Edit menu, D23)
      src/cli.rs                  cv launcher install and status
      src/macos.rs                native window settings (D20)
  plugin/voice/
    .claude-plugin/plugin.json
    .mcp.json
    package.json, tsconfig.json, esbuild config in package.json scripts
    src/env.ts src/discovery.ts src/hub-client.ts src/mcp.ts src/main.ts
    test/
    dist/channel.js               build output (git-ignored; bundled into the app)
  bin/cv-dev                      launcher pointing at plugin/voice, for development
  reference/                      the prototype and spikes, kept for reference
```

## 5. Hub crate

### 5.1 Dependencies

`tokio` (rt-multi-thread, sync, time, macros), `axum` (ws), `serde`, `serde_json`, `rusqlite` (bundled), `time`
(serde, RFC 3339), `thiserror`, `tracing`, `getrandom`, `subtle`, `ts-rs`. Dev: `tokio-tungstenite`, `tempfile`.

### 5.2 Public API

```rust
pub struct HubConfig {
    pub token: Token,
    pub delivery_timeout: Duration,   // 5 s
    pub ping_interval: Duration,      // 15 s
    pub hello_timeout: Duration,      // 2 s
    pub max_text_bytes: usize,        // 16 KiB
}

pub struct Hub;
impl Hub {
    pub async fn start(config: HubConfig, store: Box<dyn Store>, events: Arc<dyn HubEvents>)
        -> Result<RunningHub, HubError>;
}

pub struct RunningHub { pub handle: HubHandle, pub port: u16, /* shutdown sender, join handles */ }
impl RunningHub { pub async fn shutdown(self); }

#[derive(Clone)]
pub struct HubHandle { /* mpsc::Sender<Command> */ }
impl HubHandle {
    pub async fn status(&self) -> HubStatus;
    pub async fn sessions(&self) -> Vec<SessionView>;
    pub async fn set_target(&self, id: SessionId) -> Result<(), HubError>;
    pub async fn send(&self, text: String) -> Result<Message, SendError>;
    pub async fn history(&self, id: SessionId, before: Option<MessageId>, limit: u32) -> Result<Vec<Message>, HubError>;
    pub async fn mark_read(&self, id: SessionId, upto: MessageId) -> Result<(), HubError>;
    pub async fn rename(&self, id: SessionId, title: Option<String>) -> Result<(), HubError>;
    pub async fn forget(&self, id: SessionId) -> Result<(), HubError>;
}
```

### 5.3 Domain types

All derive `Serialize`, `Deserialize`, `TS` (exported to `shared/types/`), with `camelCase` field names.
`MessageId` carries `#[ts(type = "number")]` (ts-rs maps `i64` to `bigint`, but JSON carries a number) and every
`OffsetDateTime` field carries `#[ts(type = "string")]` (serialised as RFC 3339).

```rust
pub struct SessionId(String);            // Claude Code session id
pub struct MessageId(i64);
pub struct Token(String);                // 32 random bytes, hex; never logged

pub struct SessionInfo { pub id: SessionId, pub cwd: String, pub claude_pid: u32, pub started_at: OffsetDateTime }

pub enum Direction { ToSession, FromSession }
pub enum DeliveryStatus { Queued, Delivered, Failed, Received }

pub struct Message {
    pub id: MessageId, pub session: SessionId, pub direction: Direction,
    pub text: String, pub at: OffsetDateTime, pub status: DeliveryStatus, pub detail: Option<String>,
}

pub struct SessionView {
    pub id: SessionId, pub title: String, pub cwd: String, pub live: bool, pub is_target: bool,
    pub last_activity: Option<OffsetDateTime>, pub unread: u32,
}

pub struct HubStatus { pub port: u16, pub live_sessions: u32, pub target: Option<SessionId> }
```

### 5.4 Ports

```rust
pub trait Store: Send {
    fn upsert_session(&mut self, info: &SessionInfo, now: OffsetDateTime) -> Result<(), StoreError>;
    fn touch_session(&mut self, id: &SessionId, now: OffsetDateTime) -> Result<(), StoreError>;
    fn sessions(&self) -> Result<Vec<SessionRecord>, StoreError>;      // with title, last activity, unread
    fn insert_message(&mut self, new: NewMessage) -> Result<Message, StoreError>;
    fn set_status(&mut self, id: MessageId, status: DeliveryStatus, detail: Option<&str>) -> Result<(), StoreError>;
    fn fail_queued(&mut self, session: &SessionId, detail: &str) -> Result<Vec<MessageId>, StoreError>;
    fn history(&self, id: &SessionId, before: Option<MessageId>, limit: u32) -> Result<Vec<Message>, StoreError>;
    fn mark_read(&mut self, id: &SessionId, upto: MessageId) -> Result<(), StoreError>;
    fn rename(&mut self, id: &SessionId, title: Option<&str>) -> Result<(), StoreError>;
    fn forget(&mut self, id: &SessionId) -> Result<(), StoreError>;
    fn target(&self) -> Result<Option<SessionId>, StoreError>;
    fn set_target(&mut self, id: Option<&SessionId>) -> Result<(), StoreError>;
    fn purge_before(&mut self, cutoff: OffsetDateTime) -> Result<u64, StoreError>;
}

pub trait HubEvents: Send + Sync {
    fn sessions_changed(&self, sessions: &[SessionView]);
    fn target_changed(&self, target: Option<&SessionId>);
    fn message_added(&self, message: &Message);
    fn message_updated(&self, id: MessageId, status: DeliveryStatus, detail: Option<&str>);
}
```

`SqliteStore` is the production store; `MemoryStore` backs actor tests. A recording `HubEvents` fake asserts what the
UI would have been told.

### 5.5 Actor

`HubActor` owns all state and runs on a dedicated OS thread, receiving `Command`s with `blocking_recv`. Socket tasks
and `HubHandle` send commands and await answers over oneshot channels. No locks, no shared mutable state, and SQLite
calls never block the async runtime.

State: `links: HashMap<SessionId, Link>` (connection id + outbound frame sender), `target: Option<SessionId>`,
`pending: HashMap<MessageId, Instant>` (awaiting `delivered`), the store, the events sink.

| Command | Sent by | Effect |
|---|---|---|
| `Register { info, conn, link, answer }` | server | Replace any existing link for `info.id` (the old one gets `error replaced` and is closed); upsert the session; if there is no live target, make this the target; answer with the title for `welcome`; emit `sessions_changed` (and `target_changed`). |
| `Unregister { id, conn }` | server | Remove the link only if `conn` still owns it; fail its pending messages ("session disconnected"); emit `sessions_changed`. |
| `Delivered { id }` | server | Mark `delivered`; drop from `pending`; emit `message_updated`. |
| `Reply { id, text }` | server | Insert `from_session` / `received`; touch the session; emit `message_added`, `sessions_changed` (unread). |
| `Send { text, answer }` | handle | Empty → `SendError::Empty`; too long → `TooLarge`; no target → `NoTarget`; target not live → `TargetOffline`. Else insert `to_session` / `queued`, push `deliver`, add to `pending`, answer with the message, emit `message_added`. |
| `SetTarget { id, answer }` | handle | Unknown or ended → error; else set, persist, emit `target_changed`, `sessions_changed`. |
| `Tick` | timer, every second | Fail pending messages older than `delivery_timeout` ("no acknowledgement"); emit `message_updated`. |
| `Sessions`, `Status`, `History`, `MarkRead`, `Rename`, `Forget` | handle | Reads and edits. `Forget` refuses a live session. |
| `Shutdown` | `RunningHub` | Close all links normally; stop. |

Rules:

- **Target restore.** At start the target is read from the store. It is not live until that session registers.
- **Automatic target.** A registering session becomes the target only when there is no live target. The target is
  never switched away from a live session automatically.
- **Titles.** The user's rename, else the folder name of `cwd`; when two live sessions share a title, the first four
  characters of the session id are appended.
- **Unread.** Count of `from_session` messages after the session's `last_read`. The overlay marks the target's replies
  read as it shows them.

### 5.6 Server

- axum on `127.0.0.1:0`; one route, `GET /channel`, upgrading to WebSocket.
- Refuse the upgrade (403) when an `Origin` header names an `http(s)` origin (Q12).
- First frame must be `hello` within `hello_timeout`, with a matching token (constant-time) and `protocol == 1`;
  otherwise send `error` and close.
- After `Register`, two tasks per connection: inbound (frames → commands) and outbound (link receiver → socket).
- Ping every `ping_interval`; close after two missed pongs.
- Frames over 64 KiB close the connection with `too_large`.
- On any close: `Unregister`.

### 5.7 Errors

```rust
pub enum SendError { Empty, TooLarge, NoTarget, TargetOffline { title: String }, Store(StoreError) }
pub enum HubError { UnknownSession, SessionLive, Bind(io::Error), Store(StoreError), Stopped }
```

All `thiserror`; the shell maps them to a serialisable `UiError { code, message }`.

### 5.8 Discovery file

`<app data>/hub.json`, mode 0600, written to a temp file and renamed, removed on clean shutdown:

```json
{ "version": 1, "port": 51234, "token": "<64 hex chars>", "pid": 4242 }
```

Written by the shell (it knows the app data folder); `hub-dev` writes it to the path given by `--discovery`.

### 5.9 `hub-dev`

`cargo run -p hub --bin hub-dev -- --data <dir> --discovery <path>` starts a headless hub with an in-terminal prompt:
`sessions`, `target <id-prefix>`, `send <text>`, `history <id-prefix>`; replies print as they arrive. It is the phase 2
manual test bench and the end-to-end tests' hub.

### 5.10 Logging

`tracing` everywhere; the shell routes it to `<app data>/logs/` through `tauri-plugin-log`. Tokens and message text
are never logged above `debug`.

## 6. Wire protocol v1

JSON text frames tagged by `type`; field names camelCase; timestamps RFC 3339.

Channel → hub:

| Frame | Fields | Example |
|---|---|---|
| `hello` | `protocol`, `token`, `session: { id, cwd, claudePid, startedAt }` | `{"type":"hello","protocol":1,"token":"…","session":{"id":"0fa6…","cwd":"/Users/…/orders.service","claudePid":7608,"startedAt":"2026-10-25T11:56:38Z"}}` |
| `delivered` | `id` | `{"type":"delivered","id":42}` |
| `reply` | `text` | `{"type":"reply","text":"Clicked Settle; the bet shows Won."}` |

Hub → channel:

| Frame | Fields | Example |
|---|---|---|
| `welcome` | `title` | `{"type":"welcome","title":"orders.service"}` |
| `deliver` | `id`, `text`, `at` | `{"type":"deliver","id":42,"text":"click Settle","at":"2026-10-25T12:00:21Z"}` |
| `error` | `code`, `message` | `{"type":"error","code":"bad_token","message":"token mismatch"}` |

Error codes: `bad_token`, `bad_protocol`, `no_hello`, `replaced`, `too_large`.

```
channel                        hub
  │── hello ──────────────────▶│  check token, protocol; Register
  │◀───────────────── welcome ─│
  │◀───────────────── deliver ─│  (user sends)
  │── delivered ──────────────▶│  after notifications/claude/channel is written
  │── reply ──────────────────▶│  (Claude calls reply)
```

## 7. Voice channel (plugin)

### 7.1 Files

- `.claude-plugin/plugin.json`: `{"name": "voice", "version": "0.1.0", "description": "…"}`.
- `.mcp.json`: `{"mcpServers": {"voice": {"command": "node", "args": ["${CLAUDE_PLUGIN_ROOT}/dist/channel.js"], "alwaysLoad": true}}}`.
- Resulting names in Claude Code: tool `mcp__plugin_voice_voice__reply`, channel tag `source="plugin:voice:voice"`.

### 7.2 Modules

```ts
// env.ts
export interface ChannelEnv { sessionId: string; projectDir: string; claudePid: number; discoveryPath: string }
export function readEnv(env: NodeJS.ProcessEnv, ppid: number): ChannelEnv | { error: string }
// discoveryPath = VOICE_BRIDGE_DISCOVERY ?? ~/Library/Application Support/dev.voicebridge.desktop/hub.json

// discovery.ts
export interface Discovery { version: 1; port: number; token: string; pid: number }
export async function readDiscovery(path: string): Promise<Discovery | null>   // null when missing or invalid

// hub-client.ts
export interface HubClientEvents { deliver(frame: Deliver): void; connected(title: string): void; disconnected(): void }
export class HubClient {
    constructor(env: ChannelEnv, readDiscovery: () => Promise<Discovery | null>)
    start(): void
    stop(): void
    get connected(): boolean
    delivered(id: number): void
    reply(text: string): Promise<void>        // rejects when not connected
    on<K extends keyof HubClientEvents>(event: K, listener: HubClientEvents[K]): this
}

// mcp.ts
export function createChannelServer(hub: Pick<HubClient, 'reply' | 'connected'>): Server

// main.ts: readEnv → HubClient → createChannelServer → connect stdio → hub.start()
//          hub.on('deliver', async f => { await server.notification(…f.text…); hub.delivered(f.id) })
```

### 7.3 Behaviour

- **No session id:** stay connected to Claude Code over stdio, never connect to the hub, log the reason to stderr.
- **Connect loop:** read discovery → open `ws://127.0.0.1:<port>/channel` with Node's built-in `WebSocket` → `hello`.
  Backoff 1 s, 2 s, 5 s, then every 10 s; reset after a `welcome`. Re-read discovery before every attempt, since the
  port changes per app launch.
- **`error` frame:** log it; `replaced` stops this client for good, the others retry with backoff.
- **Ordering:** `delivered` is sent only after the notification is written, so `delivered` means "in the session's
  transport".
- **`reply` tool while disconnected:** returns an MCP error result: "Voice Bridge is not running, so the user did not
  see this. Tell them in the terminal instead."
- **stdout is the MCP transport.** Logs go to stderr only.
- **No permission relay:** the channel never declares `claude/channel/permission`.

### 7.4 MCP surface

- Capabilities: `{ experimental: { 'claude/channel': {} }, tools: {} }`.
- Instructions (exact):
  > The user cannot see this terminal: anything they should see must go through this server's reply tool, and plain
  > text never reaches them. Messages from this voice channel are the user speaking to you, dictated while they
  > watch the screen, so expect transcription slips. Answer every one, small talk included, by calling reply with one
  > or two sentences on what you did or why you could not.
- Tool `reply`: description "Show a short reply to the user in the Voice Bridge overlay."; input
  `{ text: string }`, max 16 000 characters.

### 7.5 Build

`esbuild src/main.ts --bundle --platform=node --format=esm --target=node22 --outfile=dist/channel.js`. The only
runtime dependency, `@modelcontextprotocol/sdk`, is bundled in, so the plugin needs no `node_modules` at runtime.
Type checking with `tsc --noEmit` (strict).

### 7.6 Launchers

`cv`, written by the app (D18):

```sh
#!/bin/sh
# voice-bridge launcher
exec claude --plugin-dir '<resource dir>/plugin/voice' --dangerously-load-development-channels plugin:voice@inline "$@"
```

- `<resource dir>` comes from Tauri's resource path, so the same code works in `tauri dev` and in the built app.
- The marker line identifies the app's own file. Installing refuses to overwrite a `cv` without it, and reports when
  `~/.local/bin` is not on `PATH`.
- `bin/cv-dev` in the repo is the same script pointing at `plugin/voice`, for phase 2 before the app exists.

## 8. Persistence

### 8.1 App data folder

```
~/Library/Application Support/dev.voicebridge.desktop/
  hub.json          runtime discovery file; exists only while the app runs
  voice.db          SQLite (WAL): sessions, messages, hub state
  settings.json     preferences
  logs/             app logs, rotated by tauri-plugin-log
```

### 8.2 SQLite schema v1

```sql
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
```

- `PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;` at open.
- Migrations are an ordered list of SQL strings; `PRAGMA user_version` records the last one applied.
- `meta` holds `target`.
- Liveness is never stored; it comes from open connections.
- At startup, `queued` rows left over from a crash become `failed` ("app restarted").

### 8.3 Settings

```json
{
  "version": 1,
  "overlay": { "visibleAtLaunch": true, "replyPreviewLines": 3 },
  "history": { "retentionDays": 30 },
  "shortcuts": { "focusInput": null, "toggleOverlay": null }
}
```

- The `Settings` service loads with defaults for missing keys, validates (`replyPreviewLines` 1–8, `retentionDays`
  1–365), writes atomically, and emits `settings-changed`.
- `update_settings` takes a partial patch; unknown keys are rejected.
- `shortcuts` is stored now and read by the Shortcuts skeleton; nothing registers global shortcuts yet.

## 9. Desktop shell

### 9.1 Dependencies

`tauri` 2 (features `macos-private-api`, `tray-icon`), `tauri-plugin-log`, `tauri-plugin-single-instance`,
`objc2` + `objc2-app-kit` (for `macos.rs`), `serde`, `serde_json`, `thiserror`, `ts-rs`, the `hub` crate. Later:
`tauri-plugin-global-shortcut`, `tauri-plugin-autostart`.

### 9.2 Composition root (`lib.rs`)

`run()` builds the app: plugins (log, single-instance focusing the panel), `setup` running §3.2, `AppState` managed
in Tauri state, the invoke handler with every command in §9.7, and a `RunEvent::ExitRequested` hook running §3.3.
Nothing else constructs services.

### 9.3 Windows

| | Overlay (`overlay`) | Control panel (`panel`) |
|---|---|---|
| Entry | `overlay.html` | `panel.html` (vue-router, hash history) |
| Created | at startup, hidden | on first open; afterwards hidden and reshown |
| Builder | `decorations(false)`, `transparent(true)`, `shadow(false)`, `always_on_top(true)`, `visible_on_all_workspaces(true)`, `skip_taskbar(true)`, `resizable(false)`, `focused(false)` | `title_bar_style(Overlay)`, `hidden_title(true)`, 960 × 640, minimum 760 × 520, `center()` |
| Native | `macos::float_over_full_screen` after building (§9.4) | none |
| Size | width 420 logical px; height from `overlay_resize` | user-resizable |
| Position | bottom-right of the primary display's work area, 16 px margin (§9.5) | centred on first open |
| Close | impossible (no chrome) | `CloseRequested` → `prevent_close()` + hide |

`app.macOSPrivateApi: true` is required for the transparent window and rules out the Mac App Store; acceptable.

### 9.4 Native window settings (`macos.rs`)

```rust
pub fn float_over_full_screen(window: &WebviewWindow) -> tauri::Result<()>
```

On the main thread, through `ns_window()`:

- `collectionBehavior = CanJoinAllSpaces | FullScreenAuxiliary | Stationary | IgnoresCycle`
- `level = NSStatusWindowLevel` (above normal and floating windows; to be confirmed in S3)
- `hidesOnDeactivate = false`

Call it once after the window is built, and never call Tauri's `set_always_on_top` or
`set_visible_on_all_workspaces` afterwards: they rewrite the same settings.

If S3 picks a non-activating panel, this module instead converts the overlay to an `NSPanel` with the
non-activating style (directly or through `tauri-nspanel`) and keeps the same collection behaviour and level.

### 9.5 Overlay size and position

- The overlay root has a `ResizeObserver`; on change it calls `overlay_resize(contentHeight)` (debounced to one
  animation frame).
- The shell computes, in logical units from `primary_monitor().work_area()` and the scale factor:
  `x = area.x + area.width − 16 − 420`, `y = area.y + area.height − 16 − height`, then sets size and position
  together so the bottom edge stays anchored.
- The window adds a 12 px transparent margin inside its bounds so island shadows aren't clipped.
- Recomputed on every resize, on show, and on `ScaleFactorChanged`; a screen-change observer comes later.

### 9.6 Menu bar, app menu and Dock

- **Menu bar item** (`tray.rs`): template icon (`icon_as_template(true)`), menu shown on left click:
  **Open Control Panel**, **Show Overlay** / **Hide Overlay**, a disabled line with the current target, separator,
  **Quit Voice Bridge**. The target line and the show/hide label update on `target-changed` and on visibility changes.
- **App menu** (`menu.rs`, D23): app menu (About, Quit) and the standard Edit menu (Undo, Redo, Cut, Copy, Paste,
  Select All). It is only visible while the panel is open, but its key equivalents work always.
- **Dock** (`windows/panel.rs`): `ActivationPolicy::Accessory` at startup. Opening the panel switches to `Regular`,
  shows and focuses it; hiding it switches back to `Accessory`.
- **Single instance:** a second launch opens the panel of the running instance.

### 9.7 Commands and events

| Command | Arguments | Returns | Windows |
|---|---|---|---|
| `hub_status` | | `HubStatus` | both |
| `list_sessions` | | `SessionView[]` | both |
| `set_target` | `id` | `()` | both |
| `send` | `text` | `Message` | overlay |
| `history` | `session`, `before?`, `limit` | `Message[]` | both |
| `mark_read` | `session`, `upto` | `()` | both |
| `rename_session` | `id`, `title?` | `()` | panel |
| `forget_session` | `id` | `()` | panel |
| `get_settings` | | `Settings` | both |
| `update_settings` | `patch` | `Settings` | panel |
| `overlay_resize` | `height` | `()` | overlay |
| `open_panel` | `route?` | `()` | overlay |
| `cli_status` | | `CliStatus { installed, path, onPath, foreign }` | panel |
| `install_cli` | | `CliStatus` | panel |

Errors return `UiError { code, message }` with codes `empty`, `too_large`, `no_target`, `target_offline`,
`unknown_session`, `session_live`, `invalid_settings`, `cli_conflict`, `internal`.

| Event | Payload |
|---|---|
| `sessions-changed` | `SessionView[]` |
| `target-changed` | `{ target: SessionId \| null }` |
| `message-added` | `Message` |
| `message-updated` | `{ id, status, detail }` |
| `settings-changed` | `Settings` |
| `overlay-visibility` | `{ visible: boolean }` |

### 9.8 Capabilities

Tauri v2 permissions per window, least privilege: `capabilities/overlay.json` grants the overlay's commands from
§9.7 plus `core:event:default`; `capabilities/panel.json` grants the panel's. The app manifest in `build.rs` lists the
commands so unlisted ones are denied.

### 9.9 `tauri.conf.json` essentials

- `identifier: "dev.voicebridge.desktop"`, `productName: "Voice Bridge"`.
- `app.macOSPrivateApi: true`; `app.windows: []` (windows are built in code).
- `app.security.csp`: `default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src ipc: http://ipc.localhost`.
- `build.devUrl: http://localhost:1420`, `build.frontendDist: ../dist`, `beforeDevCommand: npm run dev`,
  `beforeBuildCommand: npm run build`.
- `bundle.resources`: the plugin's `.claude-plugin/`, `.mcp.json` and `dist/` into `plugin/voice/`.
- `bundle.macOS.minimumSystemVersion: "13.0"`.

## 10. Frontend

### 10.1 Stack

Vue 3 (`<script setup lang="ts">`), vue-router (panel only), Vite multi-page build (`overlay.html`, `panel.html`),
`vue-tsc --noEmit` for type checks, `@tauri-apps/api`. No component library: the visual language is ours. State lives
in composables over the API.

### 10.2 Layout

```
src/
  shared/
    api.ts                typed wrappers over invoke/listen; the only module that imports @tauri-apps/api
    types.ts              re-exports shared/types (generated)
    tokens.css            design tokens, light and dark
    format.ts             relative times, truncation helpers
    components/           StatusDot.vue, Icon.vue, Kbd.vue
  overlay/
    main.ts  Overlay.vue
    components/           InputIsland.vue, ReplyIsland.vue, SessionPicker.vue
    composables/          useSessions.ts, useTarget.ts, useReplies.ts, usePasteToSend.ts, useAutoResize.ts
  panel/
    main.ts  App.vue  router.ts
    components/           Sidebar.vue, Card.vue, MessageRow.vue, SettingRow.vue
    pages/                Home.vue, Sessions.vue, SessionDetail.vue, Shortcuts.vue, Settings.vue
```

### 10.3 `api.ts`

One function per command and one `on…` per event, typed with the generated types, for example
`send(text: string): Promise<Message>` and `onMessageAdded(cb: (m: Message) => void): Promise<UnlistenFn>`.
Errors arrive as `UiError`. Components never call `invoke` or `listen` directly.

### 10.4 Composables

| Composable | Contract |
|---|---|
| `useSessions()` | `{ sessions: Ref<SessionView[]>, live: ComputedRef<SessionView[]> }`; loads once, then follows `sessions-changed` |
| `useTarget()` | `{ target: ComputedRef<SessionView \| null>, select(id) }`; follows `target-changed` |
| `useReplies(target)` | `{ latest: Ref<Message \| null>, pending: Ref<Message \| null>, error: Ref<string \| null> }`; marks the target's replies read when shown |
| `usePasteToSend(el, send)` | D3: `paste` → `preventDefault()` + `send(clipboardData text)`; Enter sends the box; Shift+Enter is a newline |
| `useAutoResize(root)` | `ResizeObserver` → `overlay_resize` |

### 10.5 Components

| Component | Props | Emits | Notes |
|---|---|---|---|
| `InputIsland` | `target`, `status: 'live' \| 'pending' \| 'offline' \| 'hub-down'` | `send(text)`, `toggle-picker` | owns the text box; restores text on a failed send |
| `ReplyIsland` | `reply`, `error`, `previewLines` | `open-panel` | clamped preview, **more** expands, click toggles |
| `SessionPicker` | `sessions`, `targetId` | `select(id)`, `close` | live first, then ended (disabled); Esc or outside click closes |
| `Sidebar` | `routes` | | logo, nav, Settings at the bottom |

### 10.6 Design tokens

Dark and light follow the system (`prefers-color-scheme`). Islands are solid, slightly translucent pills: web-view
blur cannot see the desktop behind a transparent window, and native blur would also fill the gaps between islands.

| Token | Dark | Light |
|---|---|---|
| `--island-bg` | `rgb(24 24 27 / 0.92)` | `rgb(255 255 255 / 0.94)` |
| `--island-border` | `rgb(255 255 255 / 0.08)` | `rgb(0 0 0 / 0.08)` |
| `--island-shadow` | `0 8px 24px rgb(0 0 0 / 0.35)` | `0 8px 24px rgb(0 0 0 / 0.12)` |
| `--panel-bg` / `--card-bg` | `#111113` / `#18181b` | `#faf9f7` / `#ffffff` |
| `--text` / `--text-dim` | `#f4f4f5` / `#a1a1aa` | `#18181b` / `#71717a` |
| `--accent` | `AccentColor` (system accent) | same |
| `--ok` / `--warn` / `--error` / `--idle` | `#34d399` / `#fbbf24` / `#f87171` / `#71717a` | same |
| `--radius-input` / `--radius-reply` / `--radius-card` | `22px` / `18px` / `14px` | same |

Font: `-apple-system`, 13 px in the overlay, 14 px in the panel. Motion: height and opacity transitions only,
150–200 ms, and none under `prefers-reduced-motion`.

### 10.7 Overlay

Idle:

```
                                   ╭──────────────────────────────────────────╮
                                   │ ● orders.service ▾ │ Speak or type…     ⏎ │
                                   ╰──────────────────────────────────────────╯
```

With a reply:

```
                                       ╭──────────────────────────────────────╮
                                       │ Clicked "Settle". The bet now shows  │
                                       │ Settled · Won and the balance…  more │
                                       ╰──────────────────────────────────────╯
                                   ╭──────────────────────────────────────────╮
                                   │ ● orders.service ▾ │ Speak or type…     ⏎ │
                                   ╰──────────────────────────────────────────╯
```

Picking a session:

```
                                   ╭──────────────────────────────────────────╮
                                   │ ● orders.service            ✓            │
                                   │ ● frontend                       •       │
                                   │ ○ docs.portal          ended 14:02       │
                                   ╰──────────────────────────────────────────╯
                                   ╭──────────────────────────────────────────╮
                                   │ ● orders.service ▴ │ Speak or type…     ⏎ │
                                   ╰──────────────────────────────────────────╯
```

**Input island**

| Part | Behaviour |
|---|---|
| Status dot | green: target live · amber: last message still queued · grey: no live target · red: hub not running |
| Session chip | target title, or "No session"; ▾ opens the picker; a small dot when another session has unread replies |
| Text box | grows to three lines; paste sends at once (D3); Enter sends; Shift+Enter adds a line; disabled with a hint when there is no live target |
| Send feedback | a brief tick on `delivered`; on failure the reason shows in the reply island and the text returns to the box if it is empty |

**Reply island:** the target's latest reply, clamped to `replyPreviewLines` with **more** to expand up to 40% of the
screen height, scrollable. It stays until the next send; clicking toggles expand and collapse; double-click opens that
session in the panel. Errors use the same island in `--error`.

**Picker:** live sessions first (title; folder as a tooltip; unread dot), then sessions ended in the last 24 hours,
greyed out and not selectable. Selecting calls `set_target` and closes.

**Copy**

| Situation | Text |
|---|---|
| Placeholder | Speak or type… |
| No live target | Start a session with `cv` in a terminal |
| `no_target` | Pick a session first |
| `target_offline` | {title} isn't connected |
| `too_large` | That message is too long to send |
| Delivery failed | Not delivered: {detail} |
| Hub down | Voice Bridge isn't running its hub. Restart the app |

### 10.8 Control panel

```
┌──────────────┬───────────────────────────────────────────────────────────┐
│ ◖◗ Voice     │  Good evening                                             │
│    Bridge    │  ┌───────────────┐ ┌───────────────┐ ┌──────────────────┐ │
│              │  │ Overlay   On  │ │ Live sessions │ │ Target           │ │
│ ⌂ Home       │  │               │ │ 2             │ │ orders.service   │ │
│ ☰ Sessions   │  └───────────────┘ └───────────────┘ └──────────────────┘ │
│ ⌘ Shortcuts  │  Start a voice session                                    │
│              │  ┌─────────────────────────────────────────────┐ [Copy]   │
│              │  │ $ cv                                        │          │
│              │  └─────────────────────────────────────────────┘          │
│              │  Recent                                                   │
│              │  TODAY                                                    │
│              │  19:02  you → orders.service    click Settle              │
│ ⚙ Settings   │  19:02  orders.service          Clicked "Settle". The…    │
└──────────────┴───────────────────────────────────────────────────────────┘
```

| Page | Content |
|---|---|
| Home | greeting; status cards (overlay on or off with a toggle, live sessions, target); the `cv` command with Copy, or an Install button when `cv` isn't installed; recent messages across sessions grouped by day |
| Sessions | table of live and ended sessions: title, folder, last activity, unread; row actions **Set as target**, **Rename**, **Forget** (ended only); a row opens Session detail |
| Session detail | the conversation as a timeline, newest at the bottom, with delivery status per sent message; loads older pages on scroll |
| Shortcuts | skeleton: "Focus the voice box" and "Show or hide the overlay", each with a disabled key field and "Coming soon" |
| Settings | **General:** show the overlay at launch. **Overlay:** reply preview lines. **Terminal command:** status, Install or Reinstall, the path, a PATH warning. **History:** keep for N days, Clear all history (confirm). **About:** version, hub port, data folder with Reveal in Finder |

### 10.9 Accessibility

Every control is reachable by keyboard; the islands have accessible names; the status dot has a text equivalent;
contrast meets WCAG AA in both themes.

## 11. Security

- The hub listens on 127.0.0.1 only, on a random port.
- Every channel proves itself with the token from the 0600 discovery file; web origins are refused at upgrade.
- The UI uses Tauri IPC with per-window permissions, so no web page can reach the hub through the UI.
- Conversation text stays on this Mac, in the app data folder.
- No permission relay: nothing in the app can approve Claude's tool calls.
- The `cv` installer never overwrites a file it did not write.

## 12. Principles

- **Single responsibility:** the actor decides, the server translates frames, the store persists, the shell presents,
  the channel relays.
- **Open for extension:** new frames go through a versioned protocol; new pages consume the same events.
- **Dependency inversion:** the hub depends on `Store` and `HubEvents`; the shell's composition root supplies the
  implementations.
- **Interface segregation:** windows get only the commands they use (§9.8); the channel's MCP layer sees only
  `reply` and `connected`.
- **No smells by construction:** no fixed ports, no global mutable state, no stringly-typed messages, no duplicated
  types, no blocking calls on the async runtime, no UI-side copies of hub state, no magic numbers outside
  `HubConfig` and the token file.

## 13. Testing

| Layer | How | Covers |
|---|---|---|
| Hub actor | Rust unit tests with `MemoryStore` and a recording `HubEvents` | every command and rule in §5.5 |
| Store | Rust tests on a temp SQLite file | migrations from empty, history paging, cascade on forget, purge, crash recovery of `queued` |
| Server | Rust tests: server on a random port, `tokio-tungstenite` client | hello happy path, bad token, bad protocol, no hello, web origin refused, oversize frame, ping timeout, replaced connection |
| Channel | `node:test` with a fake hub (`ws` dev dependency) and a fake MCP client | env errors, discovery changes, reconnect and backoff, notify-then-ack order, `reply` while disconnected |
| End to end | `node:test` driving `hub-dev` and two real `dist/channel.js` processes | each message reaches only its session; replies tagged; a closed session drops; hub restart → reconnect within 10 s; delivery timeout → failed |
| Settings, CLI | Rust unit tests | defaults, validation, atomic write; installer refuses a foreign `cv` |
| UI | component tests later, once the islands settle | |

`reference/tests/` holds the prototype-era oracles the end-to-end test replaces.

## 14. Development

Prerequisites: Xcode command-line tools; Rust from Homebrew's `rustup` (add `/opt/homebrew/opt/rustup/bin` to
`PATH`); Node 24.

| Task | Command |
|---|---|
| Hub tests | `cargo test -p hub` (also regenerates `shared/types/`) |
| Lint | `cargo fmt --check && cargo clippy --workspace -- -D warnings` |
| Headless hub | `cargo run -p hub --bin hub-dev -- --data /tmp/vb --discovery /tmp/vb/hub.json` |
| Channel | `npm --prefix plugin/voice run build`, `… run test`, `… run typecheck` |
| Dev voice session | `VOICE_BRIDGE_DISCOVERY=/tmp/vb/hub.json bin/cv-dev` in a terminal |
| App | `npm --prefix apps/desktop run tauri dev` |
| UI type check | `npm --prefix apps/desktop run typecheck` |

## 15. Phases

Each phase ends with its checks passing; nothing advances on a missing check.

### Phase 1 — skeleton

Tasks: move `bridge.mjs`, `page.html`, `.mcp.json`, `package.json`, `package-lock.json`, `node_modules/` and
`spikes/` into `reference/` (once the prototype voice session is closed); create the workspace, the `hub` crate, the
Tauri app with Vue and two entry pages, the plugin package, `rustfmt.toml`, strict `tsconfig`s, `.gitignore`, the
`ts-rs` export path.

Checks: `cargo build` and `cargo clippy -- -D warnings` clean; `npm run build` in `plugin/voice` writes
`dist/channel.js`; `tauri dev` opens a blank overlay and panel; `cargo test -p hub` writes a sample type into
`shared/types/`.

### Phase 2 — hub and channel, headless

Tasks: §5 and §6 in full, `hub-dev`, §7 in full, `bin/cv-dev`, the tests in §13 except UI.

Checks: all Rust and channel tests green; the end-to-end test green; manually, two `cv-dev` sessions in two folders
against `hub-dev`: `send` reaches only the target, and replies print tagged with their session; one session resumed
with `--resume` reports whether its id (and so its history) carries over (Q11).

### Phase 3 — S3 spike, then the overlay

Tasks: run S3 (§19) and record the window decision; composition root, settings service, menu bar item, app menu,
Dock policy, overlay window with `macos.rs`, sizing and position, commands and events with capabilities, the overlay
UI (§10.7). Delete `reference/` once the checks below pass.

Checks:
- No Dock icon; the menu bar item and its menu work.
- The overlay sits bottom-right with no window buttons, on every Space and over a full-screen app, and stays on top
  after clicking another app.
- Three Wispr Flow dictations in a row each send; replies appear; a long reply truncates with **more**.
- Switching the target in the picker sends the next message only to the new target.
- Quitting and relaunching restores the target and history.
- "Stop hub" (debug, D29) shows the red dot; quitting and relaunching, channels reconnect.

### Phase 4 — control panel

Tasks: panel window lifecycle and Dock switching; Home, Sessions, Session detail, Shortcuts skeleton, Settings
(§10.8); the `cv` installer.

Checks: the Dock icon appears only while the panel is visible and it is in ⌘-Tab then; closing hides it; sessions
show live and ended with unread counts; rename persists across restarts; Forget removes history and refuses a live
session; Install writes `cv`, refuses a foreign one, and warns when `~/.local/bin` isn't on `PATH`; retention purges
at startup.

### Phase 5 — later

Global shortcuts (R9), a busy indicator from plugin hooks (Q5), focus hand-back after a send (Q3), launch at login,
a screen-change observer, multiple displays (Q13), the final logo (Q14), signing and notarisation.

## 16. Constraints

- Voice sessions are terminal sessions started with `cv`; the VS Code and Cursor panels never receive channel events.
- Messages queue while Claude is busy; a channel cannot interrupt a turn.
- Channels are a research preview: the flag and protocol may change.
- On Team and Enterprise plans an admin must enable channels; the development flag cannot override
  `channelsEnabled: false`.
- A channel cannot tell whether the flag enabled it; the startup handshake is identical without it. D8 keeps the
  plugin out of other sessions; if the plugin is ever installed globally, `cv` must set a marker the channel checks
  before registering.

## 17. Invariants learned from the prototype

- Never rely on the web view inserting pasted text: read `clipboardData` in the `paste` event and cancel it (D3).
- Keep `reply` always loaded and the instructions' first sentence about plain text (D4).
- Keep an Edit menu with Paste whenever the app menu changes (D23).
- The channel's stdout is the MCP transport: nothing else may write to it.
- To tell a lost paste from a lost send: Wispr Flow's
  `~/Library/Application Support/Wispr Flow/flow.sqlite`, table `History` (open read-only), records each dictation's
  `app`, `url`, `pastedText` and `e2eLatency`. What reached Claude is in
  `~/.claude/projects/<cwd with / and . replaced by ->/<session id>.jsonl`.

## 18. Open questions

- **Q1** Channels are confirmed on the account tested so far; recheck once on the organization login if that becomes
  the day-to-day one.
- **Q3** After a send, should focus return to the app you were in? (Phase 5.)
- **Q5** Beyond replies, should the app show Claude's activity (busy or idle, tool calls) from plugin hooks?
  (Phase 5.)
- **Q11** Does `claude --resume` keep the same session id, so a resumed session continues its history? Verified in the
  phase 2 manual check (no design change either way: the id comes from the environment).
- **Q14** Final logo: an "island wave" (a pill holding three rising bars) or a "prompt wave" (`›` followed by a sound
  wave)? Phase 5; phases 1–4 use the D27 placeholder.
- **Q15** Overlay window type (S3): a regular window, or a non-activating panel? Rule in D28; S3 records the result.

Answered: Q2 (Wispr Flow pastes into a Tauri window, S2), Q4 (plugin channel identity, S1), Q6 (full-screen
support: yes, D20), Q7 (no window buttons, R1), Q8 (Dock only with the panel, R5), Q9 (memory means local app data,
R7), Q10 (name: Voice Bridge for now), Q12 (D25), Q13 (D26).

## 19. Spikes

| Spike | Status | Result |
|---|---|---|
| S1 plugin channel, two sessions | passed | `plugin:voice@inline` works; each message reached only its session; replies tagged; ~8 s round trip; no way for a channel to see whether it is enabled |
| S2 Tauri overlay with Wispr Flow | passed for dictation | two sends, replies in 3–4 s; staying on top after clicking another app, and full-screen apps, not yet observed |
| S3 full screen and window type | planned, phase 3 | see below |

**S3** (~45 min), with a full-screen app (for example Chrome full screen), and with
`ActivationPolicy::Accessory` plus the §9.6 Edit menu in place: S2 ran as a regular app, so whether ⌘V reaches the
overlay without a Dock icon is unproven.

1. Regular window + §9.4 settings. Pass: visible over the full-screen app; clicking the text box doesn't switch
   Spaces; three Wispr Flow dictations send.
2. Non-activating panel + the same settings. Pass: the same, and the app behind stays active while you dictate.

Pick 2 if it passes everything, otherwise 1. Record the choice as a decision.

## 20. Name and logo

- **Name:** Voice Bridge for now. Alternatives to consider before a first release: Murmur, Parley, Cue, Hark.
- **Logo:** parked (Q14). Needs a full-colour app icon (`.icns`) and a monochrome menu-bar template (18 pt, PDF or
  @1x/@2x PNG).

## 21. Build run

Phases 1–4 are built in one unattended run under Claude Code's `/goal`. These rules govern it. After a context
compaction, re-read this section and the status table before continuing.

### Rules

1. Before any code, ask every question needed to finish phases 1–4 without stopping, through AskUserQuestion only
   (multiple choice, the recommended option first and marked "(Recommended)"), and record the answers below.
2. Work through the phases in order without waiting between them. At the end of each phase, run every check listed
   for it and post a phase report: each check, passed or failed, with the command and its relevant output, or the
   user's confirmation for a manual check. Update the status table, commit if git is enabled, then continue.
3. A failing check is fixed and re-run, never marked passed or skipped.
4. Stop for the user only through AskUserQuestion, and only for a manual check or a blocker that cannot be resolved
   within the design. Never end a turn with a question in plain text.
5. A design change that keeps every requirement and principle: make it, record it as a new D entry with the reason,
   and list it in the phase report. A change to a requirement or to user-visible behaviour: ask first.
6. Stop long-lived processes (`tauri dev`, `hub-dev`) once their check is done: the goal check is skipped while a
   background command is still running.

### Answers

Question round 2026-10-25. Decisions D24–D30 came from it.

- **Versions:** D24.
- **Prototype processes:** the prototype session (`bridge.mjs`) was stopped in the question round; the two S1 spike
  sessions may still run and may be force-killed at phase 1. Phase 1 then moves the files into `reference/`.
- **Q11:** checked in phase 2's manual check. **Q12:** D25. **Q13:** D26. **Q14:** D27. **Q15:** D28.
- **Manual checks:** batched once per phase through one AskUserQuestion with a numbered checklist (pass/fail per
  item). Claude prepares everything first (`hub-dev`, `tauri dev`, `cv-dev` terminals) and may drive the screen with
  `osascript` (open Terminal windows, bring apps forward, put Chrome full screen).
- **Diagnostics:** Claude may read Wispr Flow's `flow.sqlite` and `~/.claude/projects/**.jsonl` read-only without
  asking.
- **Failing manual check outside the design:** diagnose first, then propose a fix through AskUserQuestion; never mark
  it passed or move on.
- **Permissions granted:** install npm and cargo packages; `git init` with a local commit after each phase (no remote,
  no push; lowercase `type(scope): subject`, no `Co-Authored-By` line); install `cv` into `~/.local/bin` in phase 4
  (already on `PATH`, no `cv` there yet); delete `reference/` after phase 3's checks.
- **reference/:** committed in phase 1 without `node_modules/`, `dist/` and `target/`, so it stays in git history
  after phase 3 deletes it.
- **Hub-down check:** D29.
- **ts-rs:** D30.

### Status

| Phase | Status | Date | Notes |
|---|---|---|---|
| 1 Skeleton | done | 2026-10-25 | all four checks passed; window check by CoreGraphics window list (no screen-capture permission) |
| 2 Hub and channel | not started | | |
| 3 S3 and overlay | not started | | |
| 4 Control panel | not started | | |
