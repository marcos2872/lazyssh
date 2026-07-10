# LazySSH — Agent guide

Single-crate Rust TUI app (ratatui + crossterm) for SSH/SFTP. No workspace, no CI, no linter config.

## Build & test

```sh
cargo build                  # debug build
cargo test                   # ~197 tests (unit + integration in tests/)
cargo test -- --list         # list all test names
```

No `cargo test` ordering needed (no external service deps). Tests are fast.

## Architecture

Three `CurrentView` states drive the event loop in `src/main.rs`:
- `ServerList` — search, add/edit/delete servers, help modal
- `SshTerminal` — input goes to PTY, render parses ANSI
- `SftpBrowser` — dual-pane local/remote file manager

### SSH/SFTP stacks

| Stack | Location | Purpose |
|-------|----------|---------|
| **Native SSH (russh)** | `src/ssh/service.rs` | Interactive PTY sessions |
| **Native SFTP (russh-sftp)** | `src/sftp/service.rs` | Filesystem ops (list, mkdir, rename, chmod) |
| **SSH for transfers** | `src/main.rs` (spawn_blocking) | Upload (cat \| ssh) and Download (ssh cat) |

Upload/Download use `cat local | ssh user@host cat > remote` via spawn_blocking — no SFTP 1GB limit. SFTP remains for filesystem operations only.

### Async + TUI sync

TUI runs a sync event loop inside `#[tokio::main]`. To call async operations:

```rust
tokio::task::block_in_place(|| {
    tokio::runtime::Handle::current().block_on(async { ... })
})
```

SSH PTY data flows from a background tokio task into the TUI via `mpsc::unbounded_channel`. Drain happens at the top of each frame loop iteration.

Upload/download use `tokio::task::spawn_blocking` to avoid blocking the TUI. Results come through `sftp_op_rx` channel.

### ANSI handling

**Never strip ANSI codes.** The parser has two stages:

1. `feed_output()` on incoming PTY data — detects CSI clear sequences (`[2J`, `[J`) and clears the local buffer
2. `parse_ansi_spans()` at render time — converts SGR codes to ratatui `Style` and discards non-SGR CSI silently

Both are in `src/tui/ssh_terminal.rs`. If you see raw `[01;32m` in output, the parser needs fixing — not a strip.

### Config format

TOML at `~/.config/lazyssh/servers.toml`. Auth is a serde tagged enum:
- `type = "key"` with `path` + optional `passphrase`
- `type = "password"` with `vault_key`

`Auth::Password { vault_key }` stores the password directly in config. Vault crypto (AES-256-GCM + PBKDF2) exists in `vault/crypto.rs` but vault integration is partial.

### Key events

**ServerList:** j/k navigate, Enter connects SSH, s opens SFTP, a adds, e edits, d deletes (with confirm), p pins, / searches, ? help, y/Y clipboard, O sorts

**SshTerminal:** Ctrl+Q/Esc disconnects. All other keys route to remote PTY via `key_event_to_bytes()`. PageUp/PageDown scroll locally.

**SftpBrowser:** Tab switches panes, u uploads (SSH), d downloads (SSH), M mkdir, R rename, x remove, m chmod, b/B bookmarks, Space selects

### Help system

Press `?` in any view to see available keybindings. Footer bar shows 3-4 key shortcuts for the current view.

## Gotchas

- `eprintln!` corrupts ratatui TUI output — use `app.notifications.info/error/warning()` instead
- Clipboard on Wayland uses `wl-copy`, on X11 uses `xclip`, fallback to `arboard`
- `tests/config_test.rs` writes to temp dirs; runs fine in parallel
- Version numbers in Cargo.toml use pre-release deps (russh 0.50.0-beta.7) — check compat before bumping
- Upload/download runs via SSH (`cat | ssh`), NOT SFTP — SFTP has ~1GB server-side limit
- `spawn_blocking` blocks a tokio thread pool thread — avoid for UI-critical paths
- `SftpService` is `Arc<Mutex>` for thread-safe access from background tasks