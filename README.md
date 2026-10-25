<p align="center">
  <img src="assets/logo/app-icon.png" width="128" alt="Apollo app icon">
</p>

<h1 align="center">Apollo</h1>

<p align="center">Your voice, from anywhere on your desktop, to Claude Code.</p>

Apollo is a macOS menu-bar app that carries your voice to Claude Code. A small overlay floats above every window, on
every Space and over full-screen apps. Dictate into it with your voice-to-text app and the message goes to the Claude
Code session you picked, whether it runs in a terminal or in an editor panel. Claude's answer comes back in the
overlay, so you can keep a session moving without switching to it.

- Pick one target among your running sessions; messages reach only that one.
- See when the target is working, and when a message hasn't been picked up.
- A control panel lists your sessions and their history, and holds the settings.
- History and settings stay on your Mac.
- Apollo updates itself from GitHub Releases.

## How it works

```
┌─────────────── Apollo.app ───────────────┐
│  overlay        control panel            │◀── your dictation app
│     └────────┬────────┘                  │    pastes the text
│             hub ── history, settings     │
└──────────────┬───────────────────────────┘
               │ WebSocket on 127.0.0.1
      ┌────────┴─────────┬──────────────────┐
voice channel      voice channel      voice channel
claude (cv)        claude (cv)        claude (cv-panel)
terminal           terminal           editor panel
```

| Part | Built with | Does |
|---|---|---|
| Overlay and control panel | Vue 3 | the floating input and replies; sessions, history and settings |
| Desktop shell | Rust, Tauri 2 | windows, menu bar, settings, installing `cv` and the plugin, updates |
| Hub | Rust | live sessions, the target, routing, delivery state, history in SQLite |
| Voice channel | TypeScript | a Claude Code plugin relaying between one session and the hub |

1. A dictation pastes into the overlay, which hands the text to the hub.
2. The hub stores it and pushes it to the target's voice channel. The hub listens on 127.0.0.1 only, and each channel
   must present a token Apollo writes at startup.
3. The channel passes it to Claude Code as a [channel](https://code.claude.com/docs/en/channels) message.
4. Claude answers by calling the channel's `reply` tool, and the reply travels back through the hub to the overlay.

`cv` is `claude` with the voice channel switched on for that one session. Plain `claude` sessions are left alone.

```
crates/hub/       the hub: sessions, routing, history, WebSocket server
apps/desktop/     the Tauri app: Rust shell (src-tauri/) and Vue UI (src/)
plugin/voice/     the voice channel, a Claude Code plugin
shared/types/     TypeScript types generated from Rust
scripts/          release tooling
```

## Install

You need:

- A Mac with Apple Silicon, on macOS 13 or later.
- [Claude Code](https://code.claude.com), signed in with a claude.ai account. Channels are a research preview; on Team
  and Enterprise plans an admin must turn them on.
- Node 22 or later on your `PATH`: the voice channel runs on it.
- A voice-to-text app, such as Wispr Flow. One that pastes its text sends the moment it pastes; you can also type
  and press Enter.

Then:

1. Download the `.dmg` from the [latest release](https://github.com/rickkoh/Apollo/releases/latest) and drag Apollo
   into Applications.
2. Open Apollo. It isn't notarised yet, so macOS blocks the first launch: open **System Settings → Privacy &
   Security** and click **Open Anyway**.
3. Click Apollo's icon in the menu bar, choose **Open Apollo**, go to **Settings**, and under **Terminal command**
   click **Install**. This adds the `cv` command to `~/.local/bin` and the voice plugin to Claude Code.
4. Optional: under **Channel approval**, copy the command and run it once in a terminal. It approves the voice plugin
   for Claude Code (it needs `sudo`), so `cv` starts without Claude Code's development-channels dialog. Without it,
   voice still works once you confirm the dialog.

Apollo checks for updates on its own and offers them in the menu bar; nothing installs without your click.

## Use

1. Start Claude Code with `cv` instead of `claude`, in any terminal, Cursor's included. Arguments pass through to
   `claude`.
2. The session appears in the overlay's picker; choose it as the target.
3. Click the overlay's text box and dictate. Claude's reply shows in the overlay.

A message sent while Claude is busy waits until its turn ends.

For Claude Code panels in Cursor or VS Code, **Settings → Cursor and VS Code panels** shows a line to add to the
editor's settings. Panel tabs opened after that carry voice too.

## Contribute

Issues and pull requests are welcome.

You need Xcode's command-line tools, Rust (through `rustup`) and Node 24.

```sh
git clone https://github.com/rickkoh/Apollo.git && cd Apollo
npm --prefix plugin/voice ci
npm --prefix apps/desktop ci
npm --prefix apps/desktop run app
```

The last command runs **Apollo Dev**, which sits beside an installed Apollo with its own data and an overlay at the
bottom-left marked DEV. Install its launcher, `cv-dev`, from Apollo Dev's Settings: it runs the voice plugin from your
checkout, so a `cv-dev` session talks only to Apollo Dev.

| Task | Command |
|---|---|
| Rust tests | `cargo test --workspace` (also regenerates `shared/types/`) |
| Rust lint | `cargo fmt --check && cargo clippy --workspace -- -D warnings` |
| Channel | `npm --prefix plugin/voice run build`, `… run test`, `… run test:e2e`, `… run typecheck` |
| UI | `npm --prefix apps/desktop run test`, `… run typecheck` |
| Headless hub | `cargo run -p hub --bin hub-dev -- --data /tmp/apollo --discovery /tmp/apollo/hub.json` |

When you change something:

- Keep the layers apart: the hub crate never depends on Tauri, UI components reach the backend only through
  `apps/desktop/src/shared/api.ts`, and `apps/desktop/src-tauri/src/lib.rs` is the only place the parts are wired
  together.
- Types shared between Rust and TypeScript come from Rust through `ts-rs`; commit the generated files in
  `shared/types/` and never edit them by hand.
- Add tests with the change, run the suites that cover it, and keep lint clean.
- Write commits as `<type>(<scope>): <subject>` in lowercase, for example `feat(overlay): show the session's folder`.
  The changelog and the next version come from the `feat`, `fix` and `perf` subjects, so write those for users.

Releases are cut from `main` with `scripts/release.sh`, which builds the app, tags it and publishes a GitHub Release
with the `.dmg` and the update feed.
