# LazySSH — Agent guide

Single-crate Rust TUI app (ratatui + crossterm) for SSH/SFTP. No workspace, no CI, no linter config.

## Build & test

```sh
cargo build                  # debug build
rtk cargo build              # preferred (rtk filters noisy output into context budget)
cargo test                   # 41 tests (unit + integration in tests/)
cargo test -- --list         # list all test names
```

No `cargo test` ordering needed (no external service deps). Tests are fast.

## Architecture

Two `CurrentView` states drive the event loop in `src/main.rs`:
- `ServerList` — search, add/edit/delete servers
- `SshTerminal` — input goes to PTY, render parses ANSI
- `SftpBrowser` — dual-pane local/remote file manager

### Dual SSH stacks (important)

| Stack | Location | Status |
|-------|----------|--------|
| **Native (russh)** | `src/ssh/service.rs`, `src/sftp/service.rs` | **Active** — used by TUI |
| Legacy (sshpass/SCP) | `src/ssh/exec.rs`, `src/sftp/remote.rs` | Unused, may be removed |

Always modify `service.rs` files. The legacy files exist but no code calls them.

### Async + TUI sync

TUI runs a sync event loop inside `#[tokio::main]`. To call async operations:

```rust
tokio::task::block_in_place(|| {
    tokio::runtime::Handle::current().block_on(async { ... })
})
```

SSH PTY data flows from a background tokio task into the TUI via `mpsc::unbounded_channel`. Drain happens at the top of each frame loop iteration.

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

In `SshTerminal` mode, Ctrl+Q or Esc disconnects. All other keys route to the remote PTY via `key_event_to_bytes()`. PageUp/PageDown scroll locally.

## Gotchas

- `eprintln!` corrupts ratatui TUI output — use `app.notifications.info/error/warning()` instead
- Clipboard on Wayland uses `wl-copy`, on X11 uses `xclip`, fallback to `arboard` — but `arboard` alone doesn't serve clipboard to other apps in Wayland
- `russh_sftp::SftpSession::write()` uses `OpenFlags::WRITE` only — for uploads use `open_with_flags(CREATE | WRITE | TRUNCATE)`
- `tests/config_test.rs` writes to temp dirs; runs fine in parallel
- Version numbers in Cargo.toml use pre-release deps (russh 0.50.0-beta.7) — check compat before bumping
