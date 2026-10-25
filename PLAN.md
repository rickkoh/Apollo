# Apollo — design

Apollo (formerly Voice Bridge) is a macOS menu-bar app with an always-on-top overlay. The user dictates into the overlay
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
- **R12 Home.** Lives in `~/Desktop/apollo`; the user sets up git and a remote.
- **R13 No dialog.** Once the channel is approved, a session started with `cv` opens without Claude Code's
  development-channels dialog.
- **R14 Any terminal.** Voice sessions run in any terminal, Cursor's integrated terminal included.
- **R15 Activity.** The overlay shows whether the target is working, since a message waits until Claude's turn ends.
- **R16 IDE panels.** Voice also reaches Claude Code sessions in Cursor's and VS Code's panels.
- **R17 Session titles.** Sessions carry the title Claude Code gives them, so sessions in one folder can be told apart.

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
  the flag, so the launcher is the only way in and no other session runs the channel. Phase 5 moved `cv` to the
  installed plugin (D44); Apollo Dev's `cv-dev` keeps this route (D75).
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
- **D21 All app data lives in `~/Library/Application Support/com.rickkoh.apollo/`** (layout in §8.1).
- **D22 (superseded by D51) Bundle identifier `dev.voicebridge.desktop`;** changeable before the first signed build.
- **D23 The app keeps a standard Edit menu.** Wispr Flow pastes with ⌘V, and macOS routes ⌘V through the main menu.
- **D24 Pinned stack: latest stable, not pre-release.** Tauri 2.12.x and its 2.x plugins (Tauri 3 is alpha), axum 0.8,
  rusqlite 0.41 (bundled), ts-rs 12, tokio 1.54, tokio-tungstenite 0.30, objc2-app-kit 0.3, Vue 3.5, vue-router 5,
  Vite 8, vue-tsc 3, MCP SDK 1.32, esbuild 0.28, TypeScript 5.9 (TS 7 is too new for vue-tsc).
- **D25 Q12: the hub refuses the upgrade only for an `http(s)` `Origin`** (§5.6 unchanged); a server test proves Node's
  built-in `WebSocket` passes.
- **D26 Q13: primary display only** until phase 6.
- **D27 Q14 placeholder: a generated "island wave"** (pill with three rising bars) as SVG → `.icns` and a monochrome
  menu-bar template; replaced by the final logo in phase 6.
- **D28 Q15 decision rule: the non-activating panel only if it passes every S3 step,** otherwise the regular window.
- **D29 Debug builds get a "Stop hub" command** (menu-bar item, debug only) so the hub-down state (red dot) can be
  observed while the app runs.
- **D31 Setting an ended session as target fails with `SessionOffline`** (`HubError::SessionOffline`, UI code
  `target_offline`), distinct from `UnknownSession`, so the UI can say which.
- **D32 One task per channel connection runs a `select!` over inbound frames, outbound frames and the ping timer**
  (instead of two tasks): one owner for the socket's lifetime and its single `Unregister`.
- **D33 The store gains `fail_all_queued` (startup crash recovery) and `clear_history` (Settings → Clear all
  history),** and the handle gains `clear_history`.
- **D34 Q11: `claude --resume` keeps the session id,** so a resumed session re-registers as the same session and its
  history continues (observed in phase 2's manual check).
- **D35 Q15: the overlay is a non-activating `NSPanel`** (through `tauri-nspanel` 2.1), floating at
  `NSStatusWindowLevel` with the §9.4 collection behaviour. S3: both kinds showed over a full-screen app, and Wispr
  Flow's ⌘V reached both under `ActivationPolicy::Accessory`; only the panel kept the app behind frontmost (Wispr Flow
  logged dictations against that app; the regular window logged them against Voice Bridge).
- **D36 The overlay shows a message stack instead of a single reply island** (requested after S3): one timeline of
  the target's messages, both sides, newest nearest the input, at most 3 cards. Your messages show dimmed as
  "You: …"; replies aren't paired with messages, so a reply starts the fade of every echo before it. Replies fade
  20 s after arriving; hovering the stack or expanding a card pauses the fade; errors stay until the next send.
  Cards slide up and fade in 180 ms, none under reduced motion. Logic in `overlay/stack.ts`, unit-tested.
- **D37 A target restored at start keeps priority for `restore_grace` (15 s)** while it reconnects; then the earliest
  live connection takes over. Without it, whichever session reconnected first after a relaunch became the target.
- **D38 Composables both windows use (`useSessions`, `useHub`, `useSettings`, `useListener`) live in
  `src/shared/composables/`.**
- **D39 Hub-down is a first-class UI state:** error code `hub_down`, event `hub-state { running }`, and the commands
  `overlay_visible`, `set_overlay_visible` and `clear_history` join §9.7.
- **D40 Home's recent messages come from one cross-session query:** `Store::recent` → `HubHandle::recent` →
  command `recent_messages`, rather than one `history` call per session.
- **D41 `about` and `reveal_data_folder` commands** back Settings → About (version, data folder, Reveal in Finder).
- **D42 `cv` status reads `PATH` from the user's login shell** (`$SHELL -ilc`), falling back to the app's own: an app
  started from Finder doesn't inherit what shell profiles add. `CliStatus` also carries `upToDate` (the launcher is
  this app's script), which drives Install vs Reinstall.
- **D43 The app menu gains a standard Window menu** (Minimize, Close Window) so ⌘W hides the panel.
- **D30 ts-rs exports from every crate into one `shared/types/`** via `TS_RS_EXPORT_DIR` in `.cargo/config.toml`,
  including the shell's `Settings`, `UiError` and `CliStatus`.
- **D44 S4 picked `per-session`** (§22.2): the plugin is installed from a marketplace the app bundles, disabled
  user-wide, and enabled per session by `cv` with `--settings`, so `--channels` loads it without the dialog. The
  drop-in approval file was read and `channelsEnabled` wasn't needed. `inline` stays the fallback if per-session
  enabling ever breaks; without an approval `cv` keeps the development flag.
- **D45 Channel approval is one managed drop-in file** holding the voice plugin's `allowedChannelPlugins` entry,
  written once by the user with a `sudo` command the panel shows; the app only reads it. Personal login only; on an
  organization login the admin decides (Q1).
- **D46 `cv` takes the approved route only when the approval file holds our entry as the approval command writes
  it** (a fixed-string `grep`), and the development flag otherwise, so a hand-edited or foreign file falls back to the
  dialog. `cv-dev` always uses the development flag.
- **D47 Voice reaches terminal sessions through `cv`,** Cursor's integrated terminal included, and IDE panel sessions
  through the panel bridge (D67).
- **D48 Cross-session messaging is not a delivery path.** Receivers treat peer messages as information, not user
  consent; Claude answers with `SendMessage` to a peer session, not to the overlay; the socket framing is undocumented.
- **D49 Activity comes from Claude Code's session registry,** read per live session as `<claude pid>.json` (the
  channel's parent pid, sent in `hello`) on the actor's tick through a hub `Presence` port: `busy` and `waiting` read
  as busy, `idle` and `shell` as idle, anything else as unknown (§22.4). Answers Q5 without plugin hooks.
- **D51 The project is called Apollo** (was Voice Bridge): app "Apollo", bundle id `com.rickkoh.apollo`, crates,
  plugin and marketplace (`apollo`), env vars (`APOLLO_DISCOVERY`), the approval file
  (`managed-settings.d/apollo.json`, so the one-time `sudo` approval is re-run) and the folder `~/Desktop/apollo`. The
  terminal command stays `cv`. Data under the old identifier is test data and is not migrated. The marketplace,
  approval file and approval entry are constants in `cli.rs`.
- **D52 Two channels on one Mac.** Stable: "Apollo", installed in /Applications, `com.rickkoh.apollo`, launcher `cv`.
  Dev: "Apollo Dev" from `tauri dev` with `tauri.dev.conf.json` layered on, `com.rickkoh.apollo.dev`, launcher
  `cv-dev` (development flag, D46), overlay bottom-left with a DEV tag. Separate data folders, discovery files and
  single-instance locks; each launcher sets `APOLLO_DISCOVERY` for its channel. Only the stable app runs the plugin
  installer; Apollo Dev's `cv-dev` loads the working copy with `--plugin-dir` and the development flag, so the two apps
  never re-point the shared marketplace at their own bundles.
- **D53 One version source:** `workspace.package.version` in `Cargo.toml` (Tauri reads it). About shows version,
  channel and git commit.
- **D54 Changelog from conventional commits with `git-cliff`:** `feat` bumps minor, `fix` patch, a breaking change
  major; `CHANGELOG.md` is committed and a release's notes are its section.
- **D55 (superseded by D83) Releases live in a private GitLab project (default branch `master`):** an annotated
  `vX.Y.Z` tag; the build files in the generic package registry (`apollo/X.Y.Z/`); a GitLab Release per tag with the
  notes and links to those files; the update feed in the same registry (D71, §23).
- **D56 Updates use `tauri-plugin-updater`:** the feed is checked at launch, every 6 hours and on demand; the menu bar
  and Settings → About offer "Update to vX.Y.Z" with the notes; install and relaunch happen on click. The updater key
  pair (minisign) stays outside the repo; its public key is in `tauri.conf.json`. The dev channel never updates.
- **D57 (superseded by D85) Reading the private project uses a GitLab deploy token** (`read_repository`, `read_package_registry`) kept in
  the macOS Keychain and entered once in Settings; never in the repo or the binary. Without one, updates are off and
  Settings says why.
- **D58 Builds are ad-hoc signed** (`signingIdentity: "-"`); a Developer ID and notarisation follow once the user's Apple
  Developer login is set up (Q21).
- **D59 Compatibility rules for updates:** the install path never changes (so `cv` stays valid); the hub accepts every
  wire-protocol version it has shipped (running sessions keep their old channel until restarted); migrations are
  forward-only and the app refuses a database newer than it knows, with a clear message.
- **D60 A `release` skill:** check the tree and tests, compute the version and notes, show them and
  ask, then `scripts/release.sh` bumps, builds, uploads, writes the feed, commits, tags and pushes.
- **D61 Every value the user sees in the overlay is a setting** (§8.3): width, reply fade, cards in the stack, text
  box lines, expanded reply height and the picker's ended-session window, with D36's behaviour as the defaults. Hub
  and protocol timing stays in `HubConfig`, since its values depend on each other; UI micro-timings, page sizes and
  design tokens stay in code.
- **D62 Setting ranges live once, in `settings::LIMITS`:** `validate` enforces them and the `settings_limits`
  command serves them, so the Settings page builds its rows, bounds and hints from them instead of repeating them.
- **D63 Each window loads its settings before it mounts** (`provideSettings`) and components `inject` them, so no
  component handles missing settings or repeats a default. The overlay re-places itself when the width changes.
- **D64 A message delivered to an idle session that starts no turn within `pickup_timeout` (10 s) is flagged**
  "Claude hasn't picked this up", reusing `detail` on `delivered`, and the overlay shows it under the echo card. A
  refused channel still connects and acknowledges, and no file check sees an organization's allowlist. Any change of
  the session's `statusUpdatedAt` counts as heard, so a missed busy phase never flags.
- **D65 The shell reads `CLAUDE_CONFIG_DIR` from the login shell on a background thread** (`login_shell::var`, D42);
  activity reads as unknown until it answers, so a slow shell profile never delays startup.
- **D66 Install keeps the plugin in line with this app:** it re-adds the `apollo` marketplace when its path isn't
  this app's `<resource dir>/plugin`, reinstalls when the installed `dist/channel.js` or `.mcp.json` differ from the
  bundle's, and disables the plugin user-wide; each step runs only when needed, through a `PluginCli` port. Settings
  reads it through its own `plugin_status` command, since that runs `claude`.
- **D67 IDE panel sessions get voice through the Claude Code extension's `claudeCode.claudeProcessWrapper`,** which
  runs our `cv-panel` launcher as `<launcher> <its claude binary> <args>` (§24). A session start (its arguments carry
  `--input-format`) goes through the panel bridge; every other call, such as `plugin list` or `auth status`, runs
  unchanged.
- **D68 The panel bridge enables the channel with the SDK's `channel_enable` control request** (`enableChannel` in the
  Agent SDK; the extension never sends it) once claude has answered the extension's `initialize`, retrying while the
  server is still connecting. It adds `--settings` enabling the plugin for that session only, sets
  `APOLLO_HOST=panel`, forwards everything else line by line, hides claude's answers to its own requests, passes
  signals and the exit code through, and logs to stderr only, since stdout is the extension's transport.
- **D69 Panel sessions get their own channel instructions:** dictated messages are answered with `reply`, typed chat
  messages in the chat. Terminal sessions keep D4's.
- **D70 The app installs `cv-panel` beside `cv` and only reads the editors' settings:** Settings shows, for Cursor and
  VS Code, whether `claudeCode.claudeProcessWrapper` points at it, with the line to add and Copy; a different wrapper
  already set is reported and left alone.
- **D71 (superseded by D84) The update feed lives in the generic package registry, not in the repository:** package `apollo-feed`, version
  `latest`, file `latest.json`, uploaded as each release's last step; GitLab serves the newest file of that name. The
  feed and the downloads share one route and one header, so the deploy token needs only `read_package_registry`.
  Replaces D55's committed `release/latest.json`.
- **D72 One version for every part:** `workspace.package.version`, both `package.json` files and `plugin.json` move
  together. The MCP server reads its version from `package.json`, `release.sh` bumps them all, and a channel test
  checks they agree. Extends D53.
- **D73 The changelog lists Features, Fixes and Performance** (`feat`, `fix`, `perf`); other types are left out. Each
  release regenerates `CHANGELOG.md` from the history (`git cliff --tag`), so the file is never edited by hand; an
  unclear line is fixed by a better subject next time.
- **D74 The release checks are code:** `scripts/release.sh preflight` checks the branch, the tree, `origin`, every test
  tier and lint, then prints the next version and its notes; `scripts/release.sh <version>` repeats the branch and tree
  checks and releases. The `release` skill runs the first, shows its output, asks, and runs the second.
- **D75 Apollo Dev installs `cv-dev`, never `cv`:** the working copy's `plugin/voice` with `--plugin-dir` and the
  development flag, pointed at Apollo Dev's discovery file unless the variable is already set (so `hub-dev` still
  works). `bin/cv-dev` goes. Apollo Dev installs no panel wrapper and runs no plugin installer; panels reach the stable
  app.
- **D76 The stable app can open at login:** Settings → General, off by default, through macOS Login Items (D80).
  Apollo Dev doesn't offer it. Moved from phase 7.
- **D77 The channel comes from the bundle identifier** (`Channel`, `.dev` suffix), and a debug build under the stable
  identifier refuses to start: `tauri dev` without the dev config would otherwise share the stable app's data and
  overwrite its `cv` and marketplace. Apollo Dev starts with `npm --prefix apps/desktop run app`.
- **D78 Every launcher (`cv`, `cv-dev`, `cv-panel`) sets `APOLLO_DISCOVERY` to its app's `hub.json` unless it is
  already set,** right after the marker, so a session reaches the app that installed its launcher and `hub-dev` can
  still be pointed at by hand.
- **D79 Menus, the menu bar item and window titles take the app name from `productName`,** so Apollo Dev names itself
  and the rename changes only the config for them.
- **D80 Open at login uses `SMAppService.mainApp`** (ServiceManagement, macOS 13+), not `tauri-plugin-autostart`: the
  system's own Login Items entry, no LaunchAgent file and no Automation prompt. Its state is read from the system, so
  removing it in System Settings shows in Apollo.
- **D81 The menu bar's "Update to vX.Y.Z…" opens Settings, where the notes and the install button are;** installing
  happens only there, so the notes are always in front of the click.
- **D82 The updater requires the signed version** (`requireSignedVersion`), so a feed can't pair a newer number with an
  older signed build. (Its deploy-token half went with D85.)
- **D83 Releases live on GitHub, `rickkoh/Apollo` (public), branch `main`:** an annotated `vX.Y.Z` tag and a GitHub
  Release per tag with the notes and the build as assets (`Apollo.app.tar.gz`, its `.sig`, the `.dmg`, `latest.json`).
  Replaces D55; commits carry the owner's GitHub identity.
- **D84 The feed is the latest release's `latest.json`** (`…/releases/latest/download/latest.json`). The release is
  created as a draft holding every asset and published last, so "latest" only ever names a complete release. Replaces
  D71.
- **D85 The repository is public, so the feed and the downloads need no credentials:** no deploy token, no Keychain
  item, no token rows in Settings. Replaces D57 and answers Q18 without S7.
- **D87 The voice plugin keeps its name in the `apollo` marketplace** (`voice@apollo`, channel `plugin:voice@apollo`),
  so the reply tool stays `mcp__plugin_voice_voice__reply`; D51's "plugin" rename covers its marketplace, its
  description and its variables (`APOLLO_DISCOVERY`, `APOLLO_HOST`).
- **D88 Nothing in the repository names an employer:** the bundle id is `com.rickkoh.apollo` (`.dev` for Apollo Dev),
  commits carry the owner's GitHub noreply identity, and examples use neutral names (`orders.service`).
- **D89 `release.sh` makes the `.dmg` itself with one `hdiutil create -srcfolder`** (the app and an Applications link),
  retried; Tauri builds only the app bundle and the update package. Tauri's DMG script mounts a scratch image, opens
  it in Finder and converts it afterwards: a click in that window failed one release, and without Finder the convert
  raced the system (`Resource temporarily unavailable`).
- **D86 A release is cut from any clean checkout whose HEAD is `origin/main`** and pushes `HEAD:main` with its tag in
  one atomic push, so a worktree can release without moving the main checkout. The release commit is made and tagged
  before the build, so About's commit is the tagged one; a failure before the push drops both.
- **D90 A session's title is the app's rename, else Claude Code's, else the folder name** (R17, §22.5). Claude Code's
  title is the latest `/rename` in the session's transcript, else its own title. The hub finds the transcript through
  the registry's session id, so the title follows the conversation in the terminal. Saving Rename with the name
  unchanged saves nothing, so opening it never pins Claude's title.
- **D91 Claude's title is stored with the session** (`claude_title`, schema v2), so an ended session keeps its name in
  the picker and on Sessions; the actor writes it only when it changes.
- **D92 A new session takes the target as it connects, behind a setting** (`sessions.targetNew`, on by default). New
  means the hub has never seen its id, so a reconnect, a relaunch or a resumed conversation never takes the target
  from a live one; with the setting off, §5.5's earlier rule holds. Settings sends the value to the hub through
  `set_target_new_sessions`, and `HubConfig` carries it at start.
- **D93 Q14: the logo is an upright lyre** (Apollo's instrument), replacing the D27 placeholder: a round bowl, horns
  curling outward, a yoke, three strings and a bridge, drawn as round-capped strokes on a 32-unit grid.
  `assets/logo/render.py` renders the app icon and the menu-bar template from the same SVG paths `Logo.vue` draws;
  `tauri icon` turns the app icon into the bundle's `.icns` and PNGs.
- **D94 Sessions follow Claude Code:** an ended session is forgotten once Claude Code keeps no transcript for it
  (`projects/<folder>/<session id>.jsonl` in its config folder, what `--resume` reads), so Apollo's session list lasts
  as long as Claude Code's. The shell compares once the login shell has given the config folder (D65), then hourly,
  through the hub's `forget`; the hub is unchanged. Every project folder is read, so Claude Code's folder naming is
  not parsed. When `projects/` is missing or holds no transcripts, the shell keeps every session and logs it, so a
  changed layout never wipes the history. Messages keep their own retention setting.
- **D95 The control panel takes Wispr Flow's layout:** the window buttons sit inset in a 44 px title strip
  (`traffic_light_position`) beside a sidebar toggle, which collapses the sidebar to an icon rail; pages sit on a
  rounded sheet inset from the window edge; icons are inline SVG line icons (`Icon.vue`). Settings is a modal
  `<dialog>` over the current page, with its own section nav and the version at its foot. The open section lives in the
  query (`?settings=about`) and `/settings/:section` redirects there, so a deep link such as the tray's Update item
  opens it over the page showing, and navigating to another page closes it.
- **D96 The Sessions table has four columns:** session, folder, last activity and actions. Unread is a pill beside the
  title; a folder shows home-relative (`~/…`, from Tauri's home directory) and truncates from the left; row actions are
  icon buttons in fixed slots, so Set as target, Rename and Forget line up across rows; the title links to the session
  detail, so the keyboard reaches it.
- **D97 Text the UI shows in more than one place lives once, in `shared/copy.ts`.** The launcher hint renders from it
  through `StartSessionHint.vue`, which sets the command in code. An error from the shell shows its own `UiError`
  message rather than a UI copy of it, so the hub-down sentence is `error.rs`'s in an error and `copy.ts`'s only where
  no error carries it (the overlay's placeholder).

## 3. Architecture

```
                  ┌─────────────────────────── Apollo.app ────────────────────────────┐
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
7. Once the login shell gives Claude Code's config folder, ended sessions it keeps no transcript for are forgotten, and
   again every hour (D94).

### 3.3 Shutdown

1. Quit from the menu bar (or ⌘Q while the panel is focused).
2. The hub closes every channel connection with a normal close; channels go back to retrying.
3. Remove `hub.json`; flush SQLite; exit.

## 4. Repository layout

```
apollo/
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
    src/presence/mod.rs           Presence trait, Activity
    src/presence/claude.rs        ClaudeSessions: Claude Code's session registry
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
      Cargo.toml, build.rs, tauri.conf.json, tauri.dev.conf.json (Apollo Dev, D52)
      capabilities/overlay.json, capabilities/panel.json
      icons/
      src/main.rs                 calls lib::run
      src/lib.rs                  composition root
      src/state.rs                AppState (channel, HubHandle, Settings)
      src/channel.rs              Stable or Dev, from the bundle identifier (D77)
      src/settings.rs             Settings service
      src/commands.rs             Tauri commands
      src/events.rs               HubEvents → Tauri emitter
      src/windows/overlay.rs      overlay window: build, position, resize
      src/windows/panel.rs        control panel window: show/hide, Dock policy
      src/tray.rs                 menu bar item and menu
      src/menu.rs                 app menu (Edit menu, D23)
      src/cli.rs                  cv launcher install, approval status
      src/ide_panel.rs            cv-panel launcher, the editors' panel wrapper setting (§24)
      src/plugin_cli.rs           voice plugin install through the claude CLI
      src/presence.rs             the session registry in Claude Code's config folder
      src/claude_config.rs        Claude Code's config folder, found through the login shell (D65)
      src/retention.rs            ended sessions follow Claude Code's transcripts (D94)
      src/login_shell.rs          the user's login-shell environment (D42)
      src/macos.rs                native window settings (D20)
  plugin/.claude-plugin/marketplace.json   the bundled marketplace (§22.2)
  plugin/voice/
    .claude-plugin/plugin.json
    .mcp.json
    package.json, tsconfig.json, esbuild config in package.json scripts
    src/env.ts src/discovery.ts src/hub-client.ts src/mcp.ts src/main.ts
    src/bridge.ts src/bridge-main.ts   the panel bridge (§24)
    test/
    dist/channel.js, dist/panel-bridge.js   build output (git-ignored; bundled into the app)
  cliff.toml, CHANGELOG.md        changelog config and its output (D73)
  scripts/release.sh              release checks and the release itself (D74, §23.2)
  .claude/skills/release/         the release skill (D60)
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
    pub restore_grace: Duration,      // 15 s
    pub pickup_timeout: Duration,     // 10 s
    pub target_new_sessions: bool,    // true (D92)
}

pub struct Hub;
impl Hub {
    pub async fn start(config: HubConfig, store: Box<dyn Store>, presence: Option<Box<dyn Presence>>,
        events: Arc<dyn HubEvents>) -> Result<RunningHub, HubError>;
}

pub struct RunningHub { pub handle: HubHandle, pub port: u16, /* shutdown sender, join handles */ }
impl RunningHub { pub async fn shutdown(self); }

#[derive(Clone)]
pub struct HubHandle { /* mpsc::Sender<Command> */ }
impl HubHandle {
    pub async fn status(&self) -> HubStatus;
    pub async fn sessions(&self) -> Vec<SessionView>;
    pub async fn set_target(&self, id: SessionId) -> Result<(), HubError>;
    pub async fn set_target_new_sessions(&self, on: bool) -> Result<(), HubError>;
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
    pub last_activity: Option<OffsetDateTime>, pub unread: u32, pub activity: Option<Activity>,
}

pub enum Activity { Busy, Idle }

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
    fn set_claude_title(&mut self, id: &SessionId, title: &str) -> Result<(), StoreError>;
    fn forget(&mut self, id: &SessionId) -> Result<(), StoreError>;
    fn target(&self) -> Result<Option<SessionId>, StoreError>;
    fn set_target(&mut self, id: Option<&SessionId>) -> Result<(), StoreError>;
    fn purge_before(&mut self, cutoff: OffsetDateTime) -> Result<u64, StoreError>;
}

pub trait Presence: Send {
    fn activity(&mut self, pid: u32) -> Option<(Activity, OffsetDateTime)>;
    fn title(&mut self, pid: u32) -> Option<String>;
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
`pending: HashMap<MessageId, Instant>` (awaiting `delivered`), `activity` and `claude_titles` (live sessions', from the
presence),
`pickups` (delivered messages watched for a turn, D64), the store, the presence, the events sink.

| Command | Sent by | Effect |
|---|---|---|
| `Register { info, conn, link, answer }` | server | Replace any existing link for `info.id` (the old one gets `error replaced` and is closed); upsert the session; if there is no live target, make this the target; answer with the title for `welcome`; emit `sessions_changed` (and `target_changed`). |
| `Unregister { id, conn }` | server | Remove the link only if `conn` still owns it; fail its pending messages ("session disconnected"); emit `sessions_changed`. |
| `Delivered { id }` | server | Mark `delivered`; drop from `pending`; emit `message_updated`; when the session is idle, watch for a turn (D64). |
| `Reply { id, text }` | server | Insert `from_session` / `received`; touch the session; emit `message_added`, `sessions_changed` (unread). |
| `Send { text, answer }` | handle | Empty → `SendError::Empty`; too long → `TooLarge`; no target → `NoTarget`; target not live → `TargetOffline`. Else insert `to_session` / `queued`, push `deliver`, add to `pending`, answer with the message, emit `message_added`. |
| `SetTarget { id, answer }` | handle | Unknown or ended → error; else set, persist, emit `target_changed`, `sessions_changed`. |
| `TargetNewSessions { on, answer }` | handle | Set whether a new session takes the target (D92). |
| `Tick` | timer, every second | Fail pending messages older than `delivery_timeout` ("no acknowledgement"); refresh live sessions' activity and titles, storing a changed title (D91) and emitting `sessions_changed` on a change; flag watched messages whose session stayed idle past `pickup_timeout`. |
| `Sessions`, `Status`, `History`, `MarkRead`, `Rename`, `Forget` | handle | Reads and edits. `Forget` refuses a live session. |
| `Shutdown` | `RunningHub` | Close all links normally; stop. |

Rules:

- **Target restore.** At start the target is read from the store. It is not live until that session registers.
- **Automatic target.** A session the hub has never seen becomes the target as it registers while
  `target_new_sessions` is on (D92). Any other registering session becomes the target only when there is no live
  target, and is never given it from a live session.
- **Titles.** The user's rename, else Claude Code's title (D90), else the folder name of `cwd`; when two live sessions
  share a title, the first four characters of the session id are appended.
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
// discoveryPath = APOLLO_DISCOVERY ?? ~/Library/Application Support/com.rickkoh.apollo/hub.json

// discovery.ts; Discovery comes from the hub through ts-rs
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
- **`reply` tool while disconnected:** returns an MCP error result: "Apollo is not running, so the user did not
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
- Tool `reply`: description "Show a short reply to the user in the Apollo overlay."; input
  `{ text: string }`, max 16 000 characters.

### 7.5 Build

`esbuild src/main.ts --bundle --platform=node --format=esm --target=node22 --outfile=dist/channel.js`. The only
runtime dependency, `@modelcontextprotocol/sdk`, is bundled in, so the plugin needs no `node_modules` at runtime.
Type checking with `tsc --noEmit` (strict).

### 7.6 Launchers

`cv`, written by the app (D18), is the two-branch launcher in §22.2: the installed plugin enabled for its session,
with `--channels` when the approval file holds our entry and the development flag otherwise.

- The marker line identifies the app's own file. Installing refuses to overwrite a `cv` without it, and reports when
  `~/.local/bin` is not on `PATH`.
- Apollo Dev installs `cv-dev` instead (D75): this checkout's `plugin/voice` with `--plugin-dir` and the development
  flag (`plugin:voice@inline`), for working on the channel.
- Every launcher sets the discovery variable to its app's `hub.json` unless it is already set (D78).

## 8. Persistence

### 8.1 App data folder

```
~/Library/Application Support/com.rickkoh.apollo/
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

- v2 adds `claude_title TEXT` to `sessions` (D91).
- `PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;` at open.
- Migrations are an ordered list of SQL strings; `PRAGMA user_version` records the last one applied.
- `meta` holds `target`.
- Liveness is never stored; it comes from open connections.
- At startup, `queued` rows left over from a crash become `failed` ("app restarted").

### 8.3 Settings

```json
{
  "version": 1,
  "overlay": {
    "visibleAtLaunch": true,
    "width": 420,
    "replyPreviewLines": 3,
    "replyFadeSeconds": 20,
    "maxCards": 3,
    "inputLines": 3,
    "expandedHeightPercent": 40,
    "pickerEndedHours": 24
  },
  "sessions": { "targetNew": true },
  "history": { "retentionDays": 30 },
  "shortcuts": { "focusInput": null, "toggleOverlay": null }
}
```

- The `Settings` service loads with defaults for missing keys, validates against `LIMITS` (D62), writes atomically,
  and emits `settings-changed`.

| Key | Range | Meaning |
|---|---|---|
| `overlay.width` | 320–720 | logical px, shadow margin included |
| `overlay.replyPreviewLines` | 1–8 | lines a card shows before **more** |
| `overlay.replyFadeSeconds` | 5–300 | how long a card shows once it starts fading |
| `overlay.maxCards` | 1–6 | cards in the stack; the oldest leaves first |
| `overlay.inputLines` | 1–8 | lines the text box grows to |
| `overlay.expandedHeightPercent` | 20–80 | share of the screen height an expanded card can fill |
| `overlay.pickerEndedHours` | 0–168 | how long an ended session stays in the picker; 0 hides them |
| `history.retentionDays` | 1–365 | applied at startup |

- `update_settings` takes a partial patch; unknown keys are rejected. It passes `sessions.targetNew` to the hub.
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
| Builder | `decorations(false)`, `transparent(true)`, `shadow(false)`, `always_on_top(true)`, `visible_on_all_workspaces(true)`, `skip_taskbar(true)`, `resizable(false)`, `focused(false)` | `title_bar_style(Overlay)`, `hidden_title(true)`, `traffic_light_position(16, 24)` (D95), 960 × 640, minimum 760 × 520, `center()` |
| Native | `macos::float_over_full_screen` after building (§9.4) | none |
| Size | width from `overlay.width`; height from `overlay_resize` | user-resizable |
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
  `x = area.x + area.width − 16 − width`, `y = area.y + area.height − 16 − height`, then sets size and position
  together so the bottom edge stays anchored.
- The window adds a 12 px transparent margin inside its bounds so island shadows aren't clipped.
- Recomputed on every resize, on show, and on `ScaleFactorChanged`; a screen-change observer comes later.

### 9.6 Menu bar, app menu and Dock

- **Menu bar item** (`tray.rs`): template icon (`icon_as_template(true)`), menu shown on left click:
  **Open Apollo**, **Show Overlay** / **Hide Overlay**, a disabled line with the current target, separator,
  **Quit Apollo**. The target line and the show/hide label update on `target-changed` and on visibility changes.
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
| `settings_limits` | | `SettingsLimits` | panel |
| `overlay_resize` | `height` | `()` | overlay |
| `open_panel` | `route?` | `()` | overlay |
| `cli_status` | | `CliStatus { installed, upToDate, path, onPath, foreign, approval, approvalCommand }` | panel |
| `install_cli` | | `CliStatus` (sets up the plugin, then writes `cv`) | panel |
| `plugin_status` | | `PluginStatus { installed, upToDate }` | panel |
| `ide_panel_status` | | `IdePanelStatus { installed, upToDate, path, foreign, setting, editors }` (D70) | panel |
| `recent_messages` | `limit` | `Message[]` | panel |
| `clear_history` | | `()` | panel |
| `overlay_visible` / `set_overlay_visible` | / `visible` | `boolean` / `()` | panel |
| `about` | | `AboutInfo { version, channel, command, commit, dataDir }` | both |
| `update_status` | | `UpdateStatus \| null` (null in Apollo Dev) | panel |
| `check_for_updates` / `install_update` | | `UpdateStatus` / `()` (relaunches) | panel |
| `save_deploy_token` | `token?` | `UpdateStatus` | panel |
| `open_at_login` / `set_open_at_login` | / `enabled` | `boolean \| null` / `boolean` | panel |
| `reveal_data_folder` | | `()` | panel |

Errors return `UiError { code, message }` with codes `empty`, `too_large`, `no_target`, `target_offline`,
`unknown_session`, `session_live`, `invalid_settings`, `cli_conflict`, `plugin_install`, `internal`.

| Event | Payload |
|---|---|
| `sessions-changed` | `SessionView[]` |
| `target-changed` | `{ target: SessionId \| null }` |
| `message-added` | `Message` |
| `message-updated` | `{ id, status, detail }` |
| `settings-changed` | `Settings` |
| `overlay-visibility` | `{ visible: boolean }` |
| `update-status` | `UpdateStatus` |

### 9.8 Capabilities

Tauri v2 permissions per window, least privilege: `capabilities/overlay.json` grants the overlay's commands from
§9.7 plus `core:event:default`; `capabilities/panel.json` grants the panel's, plus `core:path:allow-resolve-directory`
for the home folder (D96). The app manifest in `build.rs` lists the commands so unlisted ones are denied.

### 9.9 `tauri.conf.json` essentials

- `identifier: "com.rickkoh.apollo"`, `productName: "Apollo"`.
- `app.macOSPrivateApi: true`; `app.windows: []` (windows are built in code).
- `app.security.csp`: `default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src ipc: http://ipc.localhost`.
- `build.devUrl: http://localhost:1420`, `build.frontendDist: ../dist`, `beforeDevCommand: npm run dev`,
  `beforeBuildCommand: npm run build`.
- `bundle.resources`: the plugin's `.claude-plugin/`, `.mcp.json`, `dist/channel.js` and `dist/panel-bridge.js` into
  `plugin/voice/`, and the marketplace manifest into `plugin/.claude-plugin/`.
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
    copy.ts               text shown in more than one place (D97)
    components/           StatusDot.vue, Icon.vue, StartSessionHint.vue
  overlay/
    main.ts  Overlay.vue
    components/           InputIsland.vue, ReplyIsland.vue, SessionPicker.vue
    composables/          useSessions.ts, useTarget.ts, useReplies.ts, usePasteToSend.ts, useAutoResize.ts
  panel/
    main.ts  App.vue  router.ts
    components/           Sidebar.vue, SettingsDialog.vue, Card.vue, MessageRow.vue, SettingRow.vue
    composables/          useMessageFeed.ts, useOverlayVisible.ts, useCopy.ts
    pages/                Home.vue, Sessions.vue, SessionDetail.vue, Shortcuts.vue, Settings.vue (the dialog's body)
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
| `useStack(target, options, onUndelivered)` | `{ stack: MessageStack, hovered: Ref<boolean> }` (D36); marks the target's replies read when shown |
| `usePasteToSend(el, { paste, enter })` | D3: `paste` → `preventDefault()` + `paste(clipboardData text)`; Enter → `preventDefault()` + `enter()`, which sends the box; Shift+Enter is a newline |
| `useAutoResize(root)` | `ResizeObserver` → `overlay_resize` |

### 10.5 Components

| Component | Props | Emits | Notes |
|---|---|---|---|
| `InputIsland` | `target`, `status: 'live' \| 'pending' \| 'offline' \| 'hub-down'`, `maxLines` | `send(text)`, `toggle-picker` | owns the text box; restores text on a failed send |
| `MessageStack` | `cards`, `previewLines`, `expandedHeightPercent` | `hover`, `open-panel` | transition group of `ReplyIsland` cards (D36) |
| `ReplyIsland` | `card`, `previewLines`, `expandedHeightPercent` | `update:expanded`, `open-panel` | one card: clamped preview, **more** expands, click toggles |
| `SessionPicker` | `sessions`, `targetId`, `endedHours` | `select(id)`, `close` | live first, then ended (disabled); Esc or outside click closes |
| `Sidebar` | `collapsed` | | logo, nav, Settings at the bottom; an icon rail when collapsed |
| `SettingsDialog` | `section` | | modal `<dialog>`: section nav, version, `Settings` for the section; Esc, the close button or a backdrop click closes it (D95) |

### 10.6 Design tokens

Dark and light follow the system (`prefers-color-scheme`). Islands are solid, slightly translucent pills: web-view
blur cannot see the desktop behind a transparent window, and native blur would also fill the gaps between islands.

| Token | Dark | Light |
|---|---|---|
| `--island-bg` | `rgb(24 24 27 / 0.92)` | `rgb(255 255 255 / 0.94)` |
| `--island-border` | `rgb(255 255 255 / 0.08)` | `rgb(0 0 0 / 0.08)` |
| `--island-shadow` | `0 8px 24px rgb(0 0 0 / 0.35)` | `0 8px 24px rgb(0 0 0 / 0.12)` |
| `--panel-bg` / `--surface` / `--card-bg` | `#111113` / `#18181b` / `#1f1f23` | `#f2f1ee` / `#fafaf8` / `#ffffff` |
| `--control` / `--control-hover` | `rgb(255 255 255 / 0.07)` / `0.11` | `rgb(0 0 0 / 0.05)` / `0.09` |
| `--backdrop` / `--dialog-shadow` | `rgb(0 0 0 / 0.5)` / `0 24px 64px rgb(0 0 0 / 0.5)` | `rgb(0 0 0 / 0.2)` / `… / 0.18` |
| `--text` / `--text-dim` | `#f4f4f5` / `#a1a1aa` | `#18181b` / `#71717a` |
| `--accent` | `AccentColor` (system accent) | same |
| `--ok` / `--warn` / `--error` / `--idle` | `#34d399` / `#fbbf24` / `#f87171` / `#71717a` | same |
| `--radius-input` / `--radius-reply` / `--radius-card` / `--radius-sheet` | `22px` / `18px` / `14px` / `12px` | same |

The window and sidebar use `--panel-bg`, the page sheet and the settings dialog `--surface`, and cards `--card-bg`;
buttons and inputs are filled with `--control`. Font: `-apple-system`, 13 px in the overlay, 14 px in the panel; page
titles use `--font-display` (`ui-serif`, the system's New York). Motion: height and opacity transitions only,
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
| Session chip | target title, or "No session"; "· working" while the target is busy (D49); ▾ opens the picker; a small dot when another session has unread replies |
| Text box | grows to `inputLines` lines; paste sends at once (D3); Enter sends; Shift+Enter adds a line; disabled with a hint when there is no live target |
| Send feedback | a brief tick on `delivered`; on failure the reason shows in the reply island and the text returns to the box if it is empty |

**Message stack (D36):** up to `maxCards` cards above the input, each clamped to `replyPreviewLines` with **more** to
expand up to `expandedHeightPercent` of the screen height, scrollable; clicking toggles expand and collapse; double-click opens the target
in the panel. Errors use the same card in `--error`.

**Picker:** live sessions first (title; folder as a tooltip; unread dot), then sessions ended in the last `pickerEndedHours` hours,
greyed out and not selectable. Selecting calls `set_target` and closes.

**Copy**

| Situation | Text |
|---|---|
| Placeholder | Speak or type… |
| No live target | Start a session with cv in any terminal, Cursor's included |
| `no_target` | Pick a session first |
| `target_offline` | {title} isn't connected |
| `too_large` | That message is too long to send |
| Delivery failed | Not delivered: {detail} |
| Not picked up (D64) | Claude hasn't picked this up (under the echo) |
| Hub down | Apollo isn't running its hub. Restart the app |

### 10.8 Control panel

```
┌──────────────────────────────────────────────────────────────────────────┐
│ ● ● ●  ▯                                                                 │
│ ◖◗ Apollo    ╭─────────────────────────────────────────────────────────╮ │
│              │  Good evening                                           │ │
│ ⌂ Home       │  ┌──────────────┐ ┌───────────────┐ ┌────────────────┐  │ │
│ ▣ Sessions   │  │ Overlay  On  │ │ Live sessions │ │ Target         │  │ │
│ ⌘ Shortcuts  │  │              │ │ 2             │ │ orders.service │  │ │
│              │  └──────────────┘ └───────────────┘ └────────────────┘  │ │
│              │  Start a voice session                                  │ │
│              │  ┌───────────────────────────────────────────────────┐  │ │
│              │  │ $ cv                                       [Copy] │  │ │
│              │  └───────────────────────────────────────────────────┘  │ │
│              │  Recent                                                 │ │
│              │  TODAY                                                  │ │
│              │  ┌───────────────────────────────────────────────────┐  │ │
│              │  │ 19:02  you → orders.service   click Settle        │  │ │
│ ⚙ Settings   │  │ 19:02  orders.service         Clicked "Settle"…   │  │ │
│              ╰─────────────────────────────────────────────────────────╯ │
└──────────────────────────────────────────────────────────────────────────┘
```

A 44 px title strip across the top drags the window and holds the inset window buttons and the sidebar toggle (D95);
pages scroll inside the sheet below it, so it never scrolls away. Settings opens as a dialog over the page:

```
╭──────────────────┬─────────────────────────────────────────────────╮
│ SETTINGS         │ General                                       ✕ │
│ ≡ General        │ ┌─────────────────────────────────────────────┐ │
│ ▭ Overlay        │ │ Show the overlay at launch             (●)  │ │
│ › Terminal       │ │ Open at login                          ( )  │ │
│ ‹› Editors       │ └─────────────────────────────────────────────┘ │
│ ↺ History        │                                                 │
│ APOLLO           │                                                 │
│ ⓘ About          │                                                 │
│ Apollo v0.1.1    │                                                 │
╰──────────────────┴─────────────────────────────────────────────────╯
```

| Page | Content |
|---|---|
| Home | greeting; status cards (overlay on or off with a toggle, live sessions, target); the `cv` command with Copy, or an Install button when `cv` isn't installed; recent messages across sessions grouped by day |
| Sessions | table of live and ended sessions: title with an unread pill, folder, last activity; row actions **Set as target**, **Rename** (empty returns to Claude's title, D90), **Forget** (ended only) as icon buttons (D96); a row opens Session detail |
| Session detail | the conversation as a timeline, newest at the bottom, with delivery status per sent message; loads older pages on scroll |
| Shortcuts | skeleton: "Focus the voice box" and "Show or hide the overlay", each with a disabled key field and "Coming soon" |
| Settings | a dialog (D95). **General:** show the overlay at launch; target new sessions (D92). **Overlay:** every `overlay` number in §8.3, one row each, bounds and hints from `settings_limits`. **Terminal command:** `cv` with Install or Reinstall (which also sets up the plugin), the path, a PATH warning; the voice plugin's state; channel approval (Approved, or what's wrong with the `sudo` command and Copy). **History:** keep for N days, Clear all history (confirm). **About:** version, hub port, data folder with Reveal in Finder |

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
- Channel approval is a managed-settings file the user writes with `sudo` (D45); the app reads it and never asks for
  admin rights. `cv` and Settings accept only our exact entry (D46), and since a refused channel still connects, a
  delivered message that starts no turn is flagged (D64).

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
| Channel | `node:test` with a fake hub (`ws` dev dependency) and a fake MCP client | env errors, discovery changes, reconnect and backoff, notify-then-ack order, `reply` while disconnected, panel instructions; the panel bridge over fake streams: forwarding, `channel_enable` after `initialize`, retries, hidden answers |
| End to end | `node:test` driving `hub-dev` and two real `dist/channel.js` processes | each message reaches only its session; replies tagged; a closed session drops; hub restart → reconnect within 10 s; delivery timeout → failed |
| Settings, CLI | Rust unit tests | defaults, validation, atomic write; installer refuses a foreign `cv`; the generated `cv` run against a stub `claude` with a matching, a wrong and no approval file; approval states; plugin install steps through a fake `PluginCli` |
| Presence | Rust tests on a temp Claude config folder | busy, idle, `waiting`, `shell`, unknown `status`, extra fields, malformed or missing file; titles: any project folder, `/rename` over Claude's own, a growing transcript, a tail without a title, a quoted title line |
| Retention | Rust tests on a temp projects folder | transcripts from every project folder and nothing deeper; live sessions and sessions with a transcript stay; no projects folder or no transcripts removes nothing |
| UI | component tests later, once the islands settle | |

The prototype and spikes were removed after phase 3; they remain in git history (commit `d8b64d8`, `reference/`).

## 14. Development

Prerequisites: Xcode command-line tools; Rust from Homebrew's `rustup` (add `/opt/homebrew/opt/rustup/bin` to
`PATH`); Node 24.

| Task | Command |
|---|---|
| Hub tests | `cargo test -p hub` (also regenerates `shared/types/`) |
| Lint | `cargo fmt --check && cargo clippy --workspace -- -D warnings` |
| Headless hub | `cargo run -p hub --bin hub-dev -- --data /tmp/vb --discovery /tmp/vb/hub.json` |
| Channel | `npm --prefix plugin/voice run build`, `… run test`, `… run typecheck` |
| Dev voice session | `APOLLO_DISCOVERY=/tmp/vb/hub.json cv-dev` in a terminal |
| App (Apollo Dev) | `npm --prefix apps/desktop run app` |
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

### Phase 5 — native integration

Tasks (§22): S5, then activity (§22.4) and the unheard hint (D64); S4 and D44's outcome; approval for `per-session`
(§22.2): the launcher, approval status, the marketplace manifest and bundle resource, the installer, `plugin_status`
and the Settings rows; the copy in §22.3.

Checks:
- `cv` in a new terminal starts with no dialog; a dictation reaches the session and the reply shows. The same in
  Cursor's integrated terminal.
- With the approval file moved away, `cv` shows the dialog and voice still works; Settings shows Not approved with the
  command.
- Install sets up the marketplace and the plugin, and a second Install changes nothing; a rebuilt plugin shows an
  update due, which Reinstall clears; a plain `claude` session starts no `channel.js`.
- During a long turn the chip reads "working"; it clears when the turn ends.
- A session whose channel is refused shows "Claude hasn't picked this up" on its message within about 10 s.
- The §13 rows for Settings, CLI and Presence are green. Lint clean.

### Phase 5b — IDE panels (§24)

Tasks: S6 (done); the panel bridge with tests; the panel instructions; `cv-panel` and the editors' wrapper status in
Settings.

Checks:
- With Cursor's setting pointing at `cv-panel`, a new panel tab shows live in the overlay; a dictation is answered in
  the overlay, and a message typed in that panel is answered in the panel.
- Panels without the setting, `cv` sessions and plain `claude` sessions behave as before; the extension's plugin and
  sign-in screens still work.
- With the setting removed, new panel tabs start as before.
- The bridge and instruction tests are green; Settings shows each editor's state. Lint clean.

### Phase 5c — Session titles (§22.5) and new sessions as the target (D92)

Checks:
- A new `cv-dev` session shows its folder name, and Claude's title after a typed prompt; after `/clear` and another
  typed prompt it takes the new conversation's title. A `/rename` replaces it and survives `/clear`. A session started
  by dictation keeps its folder name.
- A rename on Sessions overrides it and an empty rename returns to it; Rename left unchanged keeps following Claude.
- An ended session keeps its title; an existing `voice.db` migrates to v2 with its sessions intact.
- With **Target new sessions** on, a session opened while another is the target takes it at once; one that reconnects
  doesn't. Turned off in Settings, the target stays put without a restart.
- The §13 rows for Hub actor, Store and Presence are green. Lint clean.

### Phase 6 — Apollo: channels, releases and self-update (§23)

After phase 5. Spikes first: **S7** answers Q18; **S8** proves the updater replacing an ad-hoc-signed
`/Applications/Apollo.app` and relaunching with Gatekeeper quiet.

- **6a Rename and channels.** D51, D52, D53. Checks: every test tier green; `tauri dev` runs as "Apollo Dev" with its
  data in `com.rickkoh.apollo.dev`; a `cv-dev` session reaches only Apollo Dev; the approval is re-run under the new
  name and `cv` starts with no dialog; `origin` is `rickkoh/Apollo` and `main` is pushed (D83).
- **6b Changelog and release tooling.** D54, D60, D74, D83–D86, D89. Checks: `git-cliff` produces the notes for
  `v0.1.0`; the `release` skill stops on a dirty tree and on a failing test.
- **6c Self-update and the first releases.** D56, D58, D59, D76, D80–D82; S8. Checks: `v0.1.0` is released (tag,
  GitHub Release with its files, feed); installed from its `.dmg`, Apollo runs from /Applications with `cv`; Apollo and
  Apollo Dev run side by side and each session reaches only its own app; after `v0.1.1` is released, the installed app
  offers it with its notes, installs it, relaunches at 0.1.1, and its sessions reconnect.

### Phase 7 — later

Global shortcuts (R9), focus hand-back after a send (Q3), a screen-change observer, multiple displays
(Q13), signing and notarisation.

## 16. Constraints

- Voice sessions are terminal sessions started with `cv`, or IDE panel sessions started through `cv-panel` (§24). A
  panel runs Claude Code as an SDK host, which ignores `--channels`. Cursor's integrated terminal is a terminal.
- The approved route (§22.2) needs the managed approval file; without it `cv` falls back to the development flag and
  its dialog. An organization's server-managed `allowedChannelPlugins` replaces the file.
- Messages queue while Claude is busy; a channel cannot interrupt a turn.
- Channels are a research preview: the flag and protocol may change.
- On Team and Enterprise plans an admin must enable channels; the development flag cannot override
  `channelsEnabled: false`.
- A channel cannot tell whether Claude Code enabled it; the startup handshake is identical either way. The plugin is
  installed but disabled user-wide, so only `cv` loads it (D44), and a refused channel is caught by the unheard hint
  (D64).

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
  the day-to-day one. On that login an admin must also set `channelsEnabled: true` and put the voice plugin on the
  organization's `allowedChannelPlugins`, which replaces the local approval file (D45).
- **Q3** After a send, should focus return to the app you were in? (Phase 7.)
- **Q21** Developer ID signing and notarisation: once the user's Apple Developer login is set up, sign with the
  Developer ID, notarise the `.dmg` and staple it, so a downloaded Apollo opens without Gatekeeper's warning (D58).
- **Q20** Why did a panel session started through the first S6 wrapper not load the plugin from `--settings`, when a
  headless session with the same flags did? The bridge's log answers it if it recurs.
- **Q19** A message that arrives mid-turn reaches Claude as a queued command at the next tool boundary, and in S5 Claude
  declined to act on it as not coming from the user. Should the channel instructions say mid-turn voice messages are
  the user too? Check with a dictation sent during a long turn.
- **Q22** Now that titles describe the conversation, should the picker show the folder as a dimmed second line, so
  sessions from two projects with similar titles stay apart? Today it is a tooltip there and a column on Sessions.

Answered: Q14 (D93), Q18 (D85), Q2 (Wispr Flow pastes into a Tauri window, S2), Q4 (plugin channel identity, S1), Q5
(D49), Q6 (full-screen support: yes, D20), Q7 (no window buttons, R1), Q8 (Dock only with the panel, R5), Q9 (memory
means local app data, R7), Q10 (name: Voice Bridge for now), Q11 (D34), Q12 (D25), Q13 (D26), Q15 (D35), Q23 (a session
started by dictation keeps its folder name until it has a title, §22.5).

## 19. Spikes

| Spike | Status | Result |
|---|---|---|
| S1 plugin channel, two sessions | passed | `plugin:voice@inline` works; each message reached only its session; replies tagged; ~8 s round trip; no way for a channel to see whether it is enabled |
| S2 Tauri overlay with Wispr Flow | passed for dictation | two sends, replies in 3–4 s; staying on top after clicking another app, and full-screen apps, not yet observed |
| S3 full screen and window type | passed | non-activating panel chosen (D35) |
| S4 channel approval | passed: `per-session` | the drop-in file was read and `channelsEnabled` wasn't needed; `--settings` enabled the installed, user-disabled plugin and `--channels` loaded it with no dialog (debug log: `Channel notifications registered`); a dictation round-tripped; a plain `claude` started no `channel.js`. Without the file: "plugin voice@voice-bridge is not on the approved channels allowlist (use --dangerously-load-development-channels for local dev)", and the channel still registered with the hub (D64) |
| S5 session registry activity | passed | the channel's parent pid names `~/.claude/sessions/<pid>.json`; around a foreground `sleep 15`, `status` read `idle → busy → idle` with `statusUpdatedAt` (Unix ms) moving each time. Two more values: `shell` (the turn ended, a background command still runs) and `waiting` (a permission prompt). A second message sent mid-turn queued and joined the same turn at its next tool boundary, with no `idle` between (Q19). The file goes when the session ends |
| S6 IDE panel channel | passed | the wrapper call shape, `channel_enable` and unprompted turns (§24.1); a prototype bridge in Cursor: three panel sessions enabled on the first attempt, dictations answered in the overlay, closing a tab ended the session |
| S7 GitLab feed and package access | not needed | releases moved to public GitHub releases (D83–D85), which need no credentials |
| S8 in-place update | passed | a throwaway "Apollo S8" 0.0.1 in /Applications found 0.0.2 on a local feed at launch; one click downloaded, verified and swapped the ad-hoc-signed bundle and relaunched it; the new process started its hub and re-registered its sessions with no dialog. The signature's trusted comment carries the version, so `requireSignedVersion` holds. A stable app's Install takes over `cv`, `cv-panel` and the marketplace, so a throwaway stable build must not be installed from |

**S3** (~45 min), with a full-screen app (for example Chrome full screen), and with
`ActivationPolicy::Accessory` plus the §9.6 Edit menu in place: S2 ran as a regular app, so whether ⌘V reaches the
overlay without a Dock icon is unproven.

1. Regular window + §9.4 settings. Pass: visible over the full-screen app; clicking the text box doesn't switch
   Spaces; three Wispr Flow dictations send.
2. Non-activating panel + the same settings. Pass: the same, and the app behind stays active while you dictate.

Pick 2 if it passes everything, otherwise 1. Record the choice as a decision.

**S4** (~20 min, manual; the app running; a scratch folder). A step passes when no dialog appears, no refusal line
prints, the session shows live in the overlay, and a dictation reaches it and the reply shows.

1. Approval: `sudo mkdir -p "/Library/Application Support/ClaudeCode/managed-settings.d"`, then
   `echo '{"allowedChannelPlugins":[{"marketplace":"voice-bridge","plugin":"voice"}]}' | sudo tee
   "/Library/Application Support/ClaudeCode/managed-settings.d/voice-bridge.json"`.
2. `claude plugin marketplace add ~/Desktop/voice-bridge/plugin`; `claude plugin install voice@voice-bridge`;
   `claude plugin disable voice@voice-bridge`.
3. `claude --settings '{"enabledPlugins":{"voice@voice-bridge":true}}' --channels plugin:voice@voice-bridge`; a plain
   `claude` in another folder must start no `channel.js` (`pgrep -fl channel.js`). Refused over `channelsEnabled`:
   add it and retry once. Refused as with no file: put the entry into `managed-settings.json` and retry.
4. Only if the plugin doesn't load under `--settings`: `inline`, with an `inline` entry and
   `claude --plugin-dir … --channels plugin:voice@inline`.
5. Neither passes: the dialog stays (D44); undo the file and the marketplace.

**S5** (~5 min; Claude can run it). With a `cv` session live, find its registry entry by its Claude pid; dictate
"run sleep 15, then reply done"; sample `status` every 500 ms. Pass: `idle → busy → idle`, with `busy` covering the
sleep. Fail: the hooks fallback in §22.4.

## 20. Name and logo

- **Name:** Apollo (D51); the terminal command stays `cv`.
- **Logo:** an upright lyre (D93): a full-colour app icon (`.icns`) and a monochrome menu-bar template (18 pt, @1x/@2x
  PNG).

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

Phase 6 question round 2026-10-26, asked in chat (no AskUserQuestion in that session); every recommended option taken.
Decisions D71–D76 came from it.

- **Order:** 6b tooling, S7, the channel split, 6c and S8, then the rename once phase 5 and `overlay-settings` are in
  `master`, the approval re-run and the folder move, then the push and the first releases. Work happens in
  `.worktrees/apollo-release`; nothing is edited or committed in the main checkout until the fast-forward.
- **Numbering:** phase 6 decisions start at D71 and questions at Q21 (D61–D70, Q19 and Q20 were taken).
- **Names during the split and 6c:** today's names (`VOICE_BRIDGE_*`, `voice-bridge`); only the dev identity is new.
  The rename is one mechanical commit.
- **S7:** the deploy token is stored by the user under Keychain service `apollo-s7-probe` and read straight into the
  request header, never printed; the probe file upload is asked for first.
- **Updater key:** `~/.tauri/apollo-updater.key` (0600), its password in the login Keychain, both read by
  `release.sh`; asked for again right before generating. Lose it and installed apps need one manual reinstall.
- **S8:** a throwaway "Apollo S8" and a local feed; nothing uploaded. A Keychain prompt after the update stops the run
  with the evidence.
- **Old install after the rename:** removed by the user once with given commands; no legacy code.
- **Folder move:** by the user, with every session in the folder closed; Claude gives the commands (move,
  `git worktree repair`, the memory folder, a clean rebuild).
- **Fast-forward:** once phase 5's session has stopped, Claude fast-forwards `main` from the main checkout, renames it
  to `master`, adds `origin` and pushes. Tags and GitLab Releases are asked for each time.
- **Apollo Dev in the worktree after the split:** on a free port from 1421 up (another worktree held 1421, so 1422)
  through a command-line override, without asking.
- **Manual checks:** steps as chat text, then one numbered pass/fail list answered in chat.

Later the same day the user moved releases to GitHub `rickkoh/Apollo` (D83–D86), approved generating the updater key
and going on with the rest, asked for every commit to carry their private GitHub identity, and will set up an Apple
Developer login afterwards (Q21). Then: nothing may tie Apollo to an employer (D88).

### Status

| Phase | Status | Date | Notes |
|---|---|---|---|
| 1 Skeleton | done | 2026-10-25 | all four checks passed; window check re-verified (overlay and panel `onscreen 1`, rendering confirmed by the user) |
| 2 Hub and channel | done | 2026-10-25 | 44 Rust + 15 channel + 4 end-to-end tests green; manual two-session check and Q11 passed |
| 3 S3 and overlay | done | 2026-10-25 | S3 → panel (D35); stack (D36) and restore grace (D37) added; all checks passed |
| 4 Control panel | done | 2026-10-25 | all checks passed; `cv` installed in `~/.local/bin` (points at `target/debug`; reinstall from a built app) |
| 5 Native integration | done | 2026-10-26 | S5 passed; S4 → `per-session` (D44); activity, the unheard hint (D64), the approved launcher, the plugin installer (D66) and the copy are in; every automated check green; manual checks confirmed by the user |
| 5b IDE panels | done | 2026-10-26 | S6 passed with a prototype bridge; the bundled bridge, panel instructions, `cv-panel` and the Settings section are in; every automated check green; manual checks confirmed by the user |
| 6a Rename and channels | done | 2026-10-26 | every tier green; Apollo Dev ran from a worktree with its data in `com.rickkoh.apollo.dev` and a scripted `cv-dev` session registered only with its hub; the user re-ran the approval and a scripted `cv` session opened with no dialog; `main` is on `rickkoh/Apollo`; the folder move is left to the user |
| 6b Changelog and release tooling | done | 2026-10-26 | preflight prints `0.1.0` and its notes; stops on a dirty tree and on a failing channel test; S7 not needed (D85); the first release run failed in Tauri's DMG script and rolled back cleanly (D89) |
| 6c Self-update and first releases | done | 2026-10-26 | S8 passed; `v0.1.0` released (tag, Release, four assets, public feed); the installed binary matches the release DMG's; Apollo and Apollo Dev ran side by side, each session reaching only its own hub; the installed app offered `v0.1.1`, installed it and relaunched at 0.1.1 (confirmed by the user); a live `cv` session re-registered 1 s after the app restarted |

## 22. Native integration

Phase 5 makes voice sessions feel native: no dialog when a session starts (R13), voice from any terminal (R14), and
Claude's activity in the overlay (R15). Spikes S4 and S5 settled what the findings left open.

### 22.1 Findings

From Claude Code 2.1.310's docs and binary.

- **The dialog belongs to the development flag, not to channels.** `--dangerously-load-development-channels` always
  shows it. `--channels` shows none but loads only allowlisted plugins. Managed `allowedChannelPlugins` (entries
  `{ "marketplace", "plugin" }`) replaces Anthropic's allowlist; its schema notes it requires `channelsEnabled: true`.
  Both keys are managed-only: no user or project setting does this.
- **Managed settings** come from `/Library/Application Support/ClaudeCode/managed-settings.json`, the drop-in folder
  `managed-settings.d/` beside it, the managed preferences plist, and server-managed settings. The highest source that
  sets `allowedChannelPlugins` owns it whole, so an organization's list replaces a local one.
- **Refusals** print "is not on the approved channels allowlist" (no managed list), "is not on your org's approved
  channels list" and, when the plugin's source doesn't match the requested marketplace, "from an unknown source". The
  last is the risk for `inline`. A refused channel's MCP server still starts.
- **IDE panels** run Claude Code as an SDK host (`--input-format stream-json --output-format stream-json
  --permission-prompt-tool=stdio`). The docs cover channels in the terminal CLI only and say the flag is ignored under
  the Agent SDK. `cv` in Cursor's integrated terminal is a CLI session. S6 found the host can switch a channel on
  (§24).
- **Session registry.** Every session, panels included, writes `~/.claude/sessions/<pid>.json` for cross-session
  messaging: `sessionId`, `cwd`, `name`, `entrypoint` (`cli`, `claude-vscode`), `status` (`busy`, `idle`, `shell`,
  `waiting`), `statusUpdatedAt` (Unix ms), `messagingSocketPath`, with a `.key` file beside it. The file goes when the
  session ends. Cross-session messaging is documented; this file format is not.
- **Transcripts.** A session's conversation is `projects/<folder>/<session id>.jsonl` in the config folder: `--resume`
  reads it and `cleanupPeriodDays` (default 30) deletes old ones. A session that ends before its first turn has none
  (13 of the first 25 Apollo recorded). The format is undocumented.
- **Directory marketplaces are read in place.** A plugin installed from a `directory` marketplace runs from that folder
  (`readFromFolder` in `claude plugin list --json`), with a cached copy beside it. Removing a marketplace uninstalls
  its plugins.
- **Cross-session messages** reach panel sessions and are read between tool calls, but the receiver treats them as
  information, not user consent. Claude answers with `SendMessage` to a peer session, and the socket framing is
  documented only as far as the auth line.
- **Claude desktop** cannot send to a chosen running Claude Code session: Quick Entry opens a new chat, Dispatch starts
  new Code sessions, and the Code tab has no channels. Claude Code's `/voice` dictates into the focused terminal.

### 22.2 Approval (R13)

S4 picked `per-session` (D44): the plugin comes from a local marketplace the app bundles, installed and disabled
user-wide, and `cv` enables it for its own session with `--settings`, so `--channels` needs only the approval.
`inline` (`--plugin-dir` with `plugin:voice@inline` and an `inline` entry) stays the fallback if per-session enabling
ever breaks; it rests on Claude Code's internal marketplace name.

**Approval file (D45):** `/Library/Application Support/ClaudeCode/managed-settings.d/apollo.json`, compact:

```json
{"allowedChannelPlugins":[{"marketplace":"apollo","plugin":"voice"}]}
```

- The user writes it once with the `sudo` command Settings shows with Copy (`CliStatus.approvalCommand`, built in
  `cli.rs`); the app only reads it.
- The drop-in folder is read, and `channelsEnabled` isn't needed on a personal login (S4).
- Settings reads `ApprovalState`: `approved`, `missing`, `wrong_entry` (no entry, or not as the command writes it, which
  the launcher's `grep` would miss), `unreadable`, `conflict` (`managed-settings.json` sets its own list without ours).
- It replaces Anthropic's default allowlist, so an official channel plugin (Telegram, Discord, iMessage) would need
  its own entry.
- Personal login only. On a Team or Enterprise login the admin owns `channelsEnabled` and the allowlist (Q1).

**Launcher (D46):**

```sh
#!/bin/sh
# apollo launcher
if grep -qF '{"marketplace":"apollo","plugin":"voice"}' '/Library/Application Support/ClaudeCode/managed-settings.d/apollo.json' 2>/dev/null; then
    exec claude --settings '{"enabledPlugins":{"voice@apollo":true}}' --channels plugin:voice@apollo "$@"
fi
exec claude --settings '{"enabledPlugins":{"voice@apollo":true}}' --dangerously-load-development-channels plugin:voice@apollo "$@"
```

- Never worse than before: without our entry the dialog comes back and voice still works.
- Both branches load the same plugin, so a session never runs two voice channels.
- The marker stays on line 2; `upToDate` compares the whole script.
- A user's own `cv --settings …` replaces ours and drops the plugin.
- Apollo Dev's `cv-dev` keeps `--plugin-dir` and the development flag: it runs the working copy, and the installed
  plugin is disabled, so nothing loads twice.

**Marketplace and installer (D66):**

- `plugin/.claude-plugin/marketplace.json`: `{"name": "apollo", "owner": {"name": "Apollo"}, "plugins":
  [{"name": "voice", "source": "./voice"}]}`, in `bundle.resources`, so the marketplace root is `<resource dir>/plugin`
  (`target/debug/plugin` under `tauri dev`). `claude plugin validate plugin` checks it.
- Install runs, through the login-shell `claude` (D42), only what is needed: `plugin marketplace remove` and `add` when
  the marketplace path isn't this app's; `plugin install voice@apollo --scope user` when missing, after an
  `uninstall` when the installed `dist/channel.js` or `.mcp.json` differ from the bundle's; `plugin disable` when
  enabled. Then it writes `cv`. A failing step returns UI code `plugin_install` with the CLI's output.
- The plugin runs from the marketplace folder (§22.1), so an app update reaches the next session; the content check
  keeps the cached copy current in case that changes.
- `plugin.json`, `package.json` and the MCP server agree on one version (a channel test).
- The CLI calls sit behind a `PluginCli` port, so tests use a fake.

**Status:** `CliStatus` gains `approval` and `approvalCommand`; `plugin_status` (Settings only) returns
`PluginStatus { installed, upToDate }`. Settings → Terminal command shows three rows: the launcher, the voice plugin,
and channel approval.

### 22.3 Any terminal (R14)

- `cv` is the voice session in every terminal, Cursor's integrated terminal included; that session still links to
  Cursor through Claude Code's IDE integration.
- Copy: the overlay's no-live-target hint, the picker's and Sessions' empty states and Home's "Start a voice session"
  card read "Start a session with cv in any terminal, Cursor's included".
- `alias claude=cv` is the user's choice, not app code. Once approval removes the dialog it makes every terminal
  session voice-ready at no cost; each one registers, and with **Target new sessions** on (D92) each new one takes the
  target, a quick side session included.
- IDE panel sessions get voice through the panel bridge (§24).

### 22.4 Activity (R15)

```rust
pub enum Activity { Busy, Idle }

pub trait Presence: Send {
    fn activity(&mut self, pid: u32) -> Option<(Activity, OffsetDateTime)>;
}
```

- `ClaudeSessions` reads `sessions/<pid>.json` under Claude Code's config folder for each live session's Claude pid (the channel's parent, sent in
  `hello`): `status` and `statusUpdatedAt`, other fields ignored. It never lists the folder, so `.key` files are never
  touched. `busy` and `waiting` (a permission prompt) are busy; `idle` and `shell` (a background command after the
  turn) are idle. An unknown status or a malformed or missing file reads as unknown and is logged once per kind of
  problem, so format drift shows in the log.
- The config folder is `$CLAUDE_CONFIG_DIR`, else `~/.claude`, with `CLAUDE_CONFIG_DIR` from the login shell
  (D65); `hub-dev` uses its own environment, and server tests pass `None`.
- The actor refreshes live sessions on its `Tick`, skips when none is live, and emits `sessions_changed` only when an
  activity changes. `SessionView.activity` is set for live sessions (ts-rs → `shared/types/Activity.ts`).
- Overlay: the session chip adds "· working" while the target is busy; amber keeps meaning queued in the hub. A 1 s
  poll misses the busy → idle → busy flip when Claude picks up a queued message, so the echo cards don't show it.
- **Unheard hint (D64):** on `delivered` to an idle session the actor notes its `statusUpdatedAt`; if that is
  unchanged after `pickup_timeout`, the message keeps `delivered` with the detail "Claude hasn't picked this up".
- Fallback if the format drifts: hooks in the voice plugin (`UserPromptSubmit`, `Stop`) reporting through a new
  `activity` frame (protocol v2). Built only if needed.

### 22.5 Titles (R17)

```rust
pub trait Presence: Send {
    fn activity(&mut self, pid: u32) -> Option<(Activity, OffsetDateTime)>;
    fn title(&mut self, pid: u32) -> Option<String>;
}
```

- Claude Code appends title lines to the session's transcript, `projects/<folder>/<session id>.jsonl` under the config
  folder: `{"type":"custom-title","customTitle":…}` for `/rename` (a panel's rename too) and
  `{"type":"ai-title","aiTitle":…}` for its own title, written after the first prompt. It re-appends them as the
  transcript grows and writes no `ai-title` once a `custom-title` exists. The format is undocumented.
- `title` reads `sessionId` from `sessions/<pid>.json`, finds `<sessionId>.jsonl` in any project folder once (the
  folder's name follows Claude Code's own rule), and re-reads the last 64 KiB whenever the file's size changes: the
  latest `custom-title` there, else the latest `ai-title`. A tail without a title keeps the last one found; no
  transcript yet means no title.
- Across 202 transcripts with a title, up to 8 MB, the latest title line was never more than 32 KiB from the end.
- Claude Code writes `ai-title` only when the session's first prompt was typed: 200 of 207 typed-first transcripts
  had one, 0 of 13 that began with a dictation did (one of them had typed prompts later). A session started by voice
  keeps its folder name until `/rename`, which is fine (Q23).
- `/clear` starts a new transcript under a new session id. The channel keeps the old id in its environment while
  `sessions/<pid>.json` moves to the new one, which is why `title` goes through the registry. A `/rename` carries
  across `/clear`: Claude Code writes the `custom-title` into the new transcript; otherwise the title becomes the new
  conversation's own. `/rename` also sets the registry's
  `name`, with `nameSource: "user"` (otherwise `"derived"`).
- The actor reads titles on its `Tick` with activity, stores a changed one (D91) and emits `sessions_changed` once.
- If the format drifts, titles read as none and sessions show their folder names, as before.

### 22.6 Not pursued

- Cross-session messages as the delivery path (D48).
- The hidden `--managed-settings` flag in `cv`: undocumented, and a launcher approving its own channel defeats the
  allowlist.
- An admin prompt in the app that writes the approval file: a privilege-escalation path in app code for a one-time
  step.
- An official marketplace listing: needs an Anthropic partner contact.
- `everywhere`, the plugin enabled in every session and idle without a `cv` marker: its idle path still serves the
  `reply` tool and the "plain text never reaches the user" instructions to every session, panels included; the marker
  leaks into nested `claude` processes; and `cv-dev` would load a second copy for the same session.
- Sessions without a channel shown as "Not voice-ready" (was D50, Q17) and an Activity column on Sessions.

## 23. Release and update

Phase 6 turns the working copy into an installed, self-updating Apollo while development continues beside it.

### 23.1 Channels

| | Stable | Dev |
|---|---|---|
| App | "Apollo", `/Applications/Apollo.app` | "Apollo Dev", `npm --prefix apps/desktop run tauri dev` |
| Bundle id and data folder | `com.rickkoh.apollo` | `com.rickkoh.apollo.dev` (`tauri.dev.conf.json`) |
| Launcher | `cv` | `cv-dev` (development flag) |
| Overlay | bottom-right | bottom-left, DEV tag on the chip |
| Updates | from the feed (D56) | none: it is the working copy |

### 23.2 A release

1. `scripts/release.sh preflight` (D74): HEAD is `origin/main` (D86), clean tree, every test tier and lint green; then
   `git cliff --bumped-version` gives the version and `git cliff --unreleased` the notes. The `release` skill shows
   them; nothing goes on until the user agrees.
2. `scripts/release.sh <version>`: bump every version (D72), regenerate `CHANGELOG.md` (D73), commit
   `chore(release): v<version>`, tag, `tauri build` of the app bundle (stable config, ad-hoc signed, updater artifacts
   on), the `.dmg` (D89), push
   `HEAD:main` and the tag, create a draft GitHub Release with the notes, `Apollo.app.tar.gz`, its `.sig`, the `.dmg`
   and `latest.json`, then publish it (D84), so an installed app never sees a version whose files are missing.
3. Never move or delete a pushed tag; a broken release is fixed by the next one.

`latest.json`, in the updater's static format:

```json
{
  "version": "0.1.1",
  "notes": "### Features\n- …",
  "pub_date": "2026-11-02T10:00:00Z",
  "platforms": {
    "darwin-aarch64": {
      "signature": "<contents of Apollo.app.tar.gz.sig>",
      "url": "https://github.com/rickkoh/Apollo/releases/download/v0.1.1/Apollo.app.tar.gz"
    }
  }
}
```

### 23.3 Updating

- The updater reads `https://github.com/rickkoh/Apollo/releases/latest/download/latest.json`, no credentials (D85).
- Checked at launch, every 6 hours and from Settings → About → Check for updates. A newer version shows "Update to
  vX.Y.Z…" in the menu bar, which opens Settings (D81), and in About with its notes; nothing installs without a click.
- Install: download, verify the signature, replace `/Applications/Apollo.app`, relaunch. The hub restarts and the
  channels reconnect (phase 2's end-to-end test covers it); running sessions keep their old channel code (D59).
- The dev channel builds without the updater's check and shows "Dev build" with the commit instead.

## 24. IDE panel sessions (R16)

Cursor's and VS Code's Claude Code panels run Claude Code as an SDK host: the extension starts `claude --input-format
stream-json --output-format stream-json --permission-prompt-tool=stdio …` and talks to it in JSON lines. In that mode
`--channels` is ignored; a channel is switched on only by the host's `channel_enable` control request, which the
extension never sends. The panel bridge sends it on the extension's behalf (D67, D68).

### 24.1 Findings (S6)

- The extension runs `claudeCode.claudeProcessWrapper` as `<wrapper> <its claude binary> <args>`, for sessions and for
  one-off subcommands (`auth status --json`, `plugin list --json`, `plugin enable …`); subcommands reject a trailing
  `--settings`.
- `{"type":"control_request","request_id":…,"request":{"subtype":"channel_enable","serverName":"plugin:voice:voice"}}`
  answers `success` once the server is connected ("server … is not connected" before), and the channel then registers
  as with `--channels` in a terminal. The code runs it through the same allowlist gate; a refusal without the approval
  file is expected but untested.
- A channel message starts a turn on its own, with no host input; the panel shows it as a user message and shows the
  `reply` call.
- The extension's SDK files a `control_response` it didn't ask for under unmatched responses (bounded) and otherwise
  ignores it.
- `--settings '{"enabledPlugins":{"voice@voice-bridge":true}}'` loads the user-disabled plugin in this mode (headless;
  Q20 for the one panel session that didn't).
- Panel sessions write the session registry with `status`, so activity works unchanged.
- Under the terminal instructions Claude tells the panel "the overlay is the only place you see those replies" (D69).

### 24.2 Launcher and bridge

```sh
#!/bin/sh
# apollo launcher
case " $* " in
    *" --input-format "*|*" --input-format="*) exec node '<resource dir>/plugin/voice/dist/panel-bridge.js' "$@" ;;
esac
exec "$@"
```

- `panel-bridge.js` is the plugin's second bundle (`src/bridge.ts`, entry `src/bridge-main.ts`), shipped beside
  `channel.js`.
- The bridge starts `<binary> <args> --settings '{"enabledPlugins":{"voice@apollo":true}}'` with
  `APOLLO_HOST=panel`. It forwards the extension's lines to claude and claude's lines to the extension. After
  claude answers the extension's `initialize`, it sends `channel_enable` (`request_id` `apollo-<n>`), retrying
  every second while the server isn't connected, up to 30 times, and drops claude's answers to those requests. It
  passes stderr through, forwards SIGTERM, SIGINT and SIGHUP, and exits with claude's code.
- A bridge fault costs voice in a panel, never the panel: nothing of ours is sent before the extension's own first
  request is answered, and nothing of the extension's is changed.

### 24.3 Setup (D70)

- Install writes `cv-panel` beside `cv`, under the same marker rules.
- Settings → Cursor and VS Code panels: `cv-panel`'s state and, for each editor found on this Mac (Cursor:
  `~/Library/Application Support/Cursor/User/settings.json`; VS Code: `…/Code/User/settings.json`), whether
  `claudeCode.claudeProcessWrapper` is ours, unset or another wrapper, with the line to add or replace it and Copy;
  the app never edits the editors' settings.
- Only panel sessions started after the setting carry voice. Removing the setting returns panels to how they were.

### 24.4 Panel instructions (D69)

> Messages from this voice channel are the user speaking to you from the Apollo overlay, dictated while they
> may be looking away from this chat, so expect transcription slips. Answer every one, small talk included, by calling
> this server's reply tool with one or two sentences on what you did or why you could not. Answer messages typed in
> this chat in the chat, as usual.
