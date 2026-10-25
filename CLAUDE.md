# Apollo

macOS menu-bar app: an always-on-top overlay sends Wispr Flow dictation to one chosen Claude Code session through a
Claude Code channel and shows Claude's replies. `PLAN.md` is the design and the source of truth: read the relevant
sections before changing anything, follow its phases and checks, and record every new decision (`D`) and open
question (`Q`) there.

## Working rules

- Build phase by phase (`PLAN.md` §15). A phase is done only when all its checks pass; report a failed or skipped
  check as such. During the unattended build, `PLAN.md` §21 governs.
- Keep the layering in `PLAN.md` §3: the hub crate never depends on Tauri; components never call `invoke`/`listen`
  outside `src/shared/api.ts`; the shell's `lib.rs` is the only composition root.
- Wire and UI types come from Rust through `ts-rs` into `shared/types/`; never hand-write a TypeScript copy.
- Run the test tier that covers the change, not every suite.

## Invariants (break one and the app silently stops working)

- A paste is read from the `paste` event's `clipboardData` and the default insertion cancelled; the web view's own
  insertion drops Wispr Flow's second and later dictations.
- The channel's `reply` tool stays `alwaysLoad`, and its instructions open with "plain text never reaches the user";
  otherwise Claude answers in the terminal.
- The app keeps a standard Edit menu with Paste; Wispr Flow pastes with ⌘V.
- The channel's stdout is the MCP transport; log to stderr only.
- The hub binds 127.0.0.1 only, and every channel must present the token from the discovery file.
- Voice sessions are terminal sessions started with `cv` (or Apollo Dev's `cv-dev`); IDE panels never receive channel
  events.

## Environment

- Rust comes from Homebrew's `rustup`: put `/opt/homebrew/opt/rustup/bin` on `PATH` for cargo commands.
- Node 24; Xcode command-line tools.
- The single-session prototype and the S1/S2 spikes were removed after phase 3; `git show 472969f:reference/…` has
  them.

## Diagnosing a lost message

- Wispr Flow's `~/Library/Application Support/Wispr Flow/flow.sqlite`, table `History`, opened read-only
  (`sqlite3 "file:flow.sqlite?mode=ro"`): each dictation's `app`, `url`, `pastedText`, `e2eLatency`.
- What reached Claude: `~/.claude/projects/<cwd with / and . replaced by ->/<session id>.jsonl`.
- Channel processes carry `CLAUDE_CODE_SESSION_ID` and `CLAUDE_PROJECT_DIR` in their environment.
