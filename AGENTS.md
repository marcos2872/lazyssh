# LazySSH — Agent guide

Single-crate Rust TUI app (ratatui + crossterm) for SSH/SFTP. No workspace, no CI, no linter config.

## Build & test

```sh
cargo build                  # debug build
cargo test                   # ~346 tests (unit + integration in tests/)
cargo test -- --list         # list all test names
```

No `cargo test` ordering needed (no external service deps). Tests are fast.

## Architecture

Three `CurrentView` states drive the event loop in `src/main.rs`:
- `ServerList` — search, add/edit/delete servers, help modal, import SSH config
- `SshTerminal` — placeholder (SSH opens in external shell)
- `SftpBrowser` — dual-pane local/remote file manager

### SSH connection: Native Shell Handoff

SSH connections do NOT use the internal russh PTY. Instead:
1. `leave_tui()` — exits raw mode + alternate screen
2. `\x1bc` — full terminal reset (clears scrollback)
3. Spawns `ssh` or `sshpass ssh` as external process
4. User interacts with real SSH terminal
5. On exit, `reenter_tui()` restores TUI

`native_shell_command()` in `src/main.rs` builds the SSH args with flags: `-p`, `-A`, `-J`, `-i`, `sshpass`.

### Upload/Download

Upload: `cat local | ssh user@host 'cat > remote'` via `spawn_blocking`
Download: `ssh user@host 'cat remote' > local` via `spawn_blocking`

Both use SSH pipe — no SFTP 1GB limit. TUI stays responsive.

### SFTP (filesystem ops only)

SFTP is used only for: `read_dir`, `create_dir`, `rename`, `remove_file`, `remove_dir`, `set_permissions` (chmod).
Native russh-sftp via `src/sftp/service.rs`.

### Keyring for passwords

- `src/vault/keyring.rs` wraps the `keyring` crate
- On save: password stored in OS keyring, `vault_key` cleared in TOML
- On load: if `vault_key` empty, read from keyring
- Fallback: if keyring unavailable, password stays in plaintext TOML

### Config format

TOML at `~/.config/lazyssh/servers.toml`. Auth is a serde tagged enum:
- `type = "key"` with `path` + optional `passphrase`
- `type = "password"` with `vault_key` (empty when in keyring)

Additional fields: `tags`, `pinned`, `agent_forwarding`, `proxy_jump`.

### Import SSH config

Press `i` in ServerList to import from `~/.ssh/config`. Parses Host, HostName, Port, User, IdentityFile. Skips wildcards. Deduplicates by host:port:user.

### Key events

**ServerList:** j/k navigate, Enter connects SSH, s opens SFTP, a adds, e edits, d deletes, p pins, / searches, i imports, t tests connection, y/Y clipboard, O sorts, ? help, q quits

**SftpBrowser:** Tab switches panes, u uploads (SSH), d downloads (SSH), M mkdir, R rename, x remove, m chmod, b/B bookmarks, Space selects

**SFTP Input Mode:** ← → cursor navigation, Home/End, Delete, Backspace, Enter saves, Esc cancels

### Help system

Press `?` in any view to see available keybindings. Footer bar shows key shortcuts for the current view.

## Gotchas

- `eprintln!` corrupts ratatui TUI output — use `app.notifications.info/error/warning()` instead
- Clipboard on Wayland uses `wl-copy`, on X11 uses `xclip`, fallback to `arboard`
- `tests/config_test.rs` writes to temp dirs; runs fine in parallel
- Upload/download runs via SSH (`cat | ssh`), NOT SFTP — SFTP has ~1GB server-side limit
- SSH opens in external shell — the internal russh PTY path exists but is NOT used for connections
- `spawn_blocking` blocks a tokio thread pool thread — avoid for UI-critical paths
- `SftpService` is `Arc<Mutex>` for thread-safe access from background tasks
- Keyring tests use `#[ignore]` — they need a real OS keyring to run
- `sshpass` is required for password auth in SSH and upload/download