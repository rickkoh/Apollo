# Apollo

macOS menu-bar app: an always-on-top overlay sends dictation from any voice-to-text app to one chosen Claude Code
session through a Claude Code channel and shows Claude's replies. `README.md` covers the design, installing and the
development commands.

## Working rules

- Keep the layering: the hub crate never depends on Tauri; components never call `invoke`/`listen` outside
  `src/shared/api.ts`; the shell's `lib.rs` is the only composition root.
- Wire and UI types come from Rust through `ts-rs` into `shared/types/`; never hand-write a TypeScript copy.
- Run the test tier that covers the change, not every suite.
- The repository is public: nothing in it names an employer, its hosts or its projects.
- Apollo Dev runs with `npm --prefix apps/desktop run app`; releases go through the `release` skill.

## Invariants (break one and the app silently stops working)

- A paste is read from the `paste` event's `clipboardData` and the default insertion cancelled; the web view's own
  insertion drops the second and later pastes (seen with Wispr Flow).
- The channel's `reply` tool stays `alwaysLoad`, and its instructions open with "plain text never reaches the user";
  otherwise Claude answers in the terminal.
- The app keeps a standard Edit menu with Paste; dictation apps paste with ⌘V.
- The channel's stdout is the MCP transport; log to stderr only.
- The hub binds 127.0.0.1 only, and every channel must present the token from the discovery file.
- Voice sessions start through Apollo's launchers: `cv` (Apollo Dev's `cv-dev`) in a terminal, `cv-panel` as an editor
  panel's process wrapper. A panel ignores `--channels`; only the bridge's `channel_enable` switches its channel on.

## Environment

- Rust comes from Homebrew's `rustup`: put `/opt/homebrew/opt/rustup/bin` on `PATH` for cargo commands.
- Node 24; Xcode command-line tools.

## Diagnosing a lost message

- With Wispr Flow as the dictation app: `~/Library/Application Support/Wispr Flow/flow.sqlite`, table `History`,
  opened read-only (`sqlite3 "file:flow.sqlite?mode=ro"`): each dictation's `app`, `url`, `pastedText`, `e2eLatency`.
- What reached Claude: `~/.claude/projects/<cwd with / and . replaced by ->/<session id>.jsonl`.
- Channel processes carry `CLAUDE_CODE_SESSION_ID` and `CLAUDE_PROJECT_DIR` in their environment.
