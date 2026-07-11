# LazySSH Code Quality Refactor — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use compose:subagent (recommended) or compose:execute to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate all ERRO/AVISO findings from the code quality evaluation: God Object in main.rs, code duplication, dead code, security issues, and error handling — while keeping all 350 tests passing at every step.

**Architecture:** Decompose `main.rs` (1.888 lines) into focused modules: `tui/modals.rs` for modal rendering, `tui/handlers.rs` for event handling, `sftp/transfer.rs` for SSH transfer logic. Unify duplicated `InsertState`/`EditState` into a single `FormState`. Extract SSH args builder into a reusable helper. Fix security (sshpass), error handling, and dead code.

**Tech Stack:** Rust, ratatui, crossterm, tokio, russh, russh-sftp, keyring crate v3

## Global Constraints

- All 350 existing tests must pass after every task — run `cargo test` after each step
- `cargo build` must succeed after every step — no compile errors allowed
- No behavioral changes unless explicitly noted (security fix, error handling)
- User communicates in Portuguese (Brazilian) — commit messages in English, UI strings stay in Portuguese
- `eprintln!` is forbidden in TUI code — use `app.notifications` or `eprintln!` only in non-TUI modules

---

## File Structure (post-refactor)

| File | Responsibility | Lines (est.) |
|---|---|---|
| `src/main.rs` | Event loop shell only — poll + dispatch | ~400 |
| `src/tui/app.rs` | App struct, state, business logic | ~600 |
| `src/tui/modals.rs` | **NEW** — Insert/Edit/Confirm modal rendering | ~400 |
| `src/tui/handlers.rs` | **NEW** — All keyboard event handling per view/input mode | ~600 |
| `src/tui/sftp_browser.rs` | SFTP browser UI + SftpState | ~600 (unchanged) |
| `src/tui/server_list.rs` | Server list UI rendering | ~114 (unchanged) |
| `src/tui/ssh_terminal.rs` | SSH terminal rendering (dead, kept for future) | ~1059 (unchanged) |
| `src/sftp/transfer.rs` | **NEW** — SSH upload/download logic | ~200 |
| `src/config/file.rs` | Config load/save with keyring integration | ~119 (minor changes) |
| `src/vault/keyring.rs` | OS keyring wrapper | ~92 (minor changes) |

---

## Task Dependency Graph

```
T1 (FormState) ──────────────> T3 (Handlers) ──> T7 (Dead code cleanup)
T2 (SSH args builder) ───────> T4 (Modals) ────> T7
                              > T5 (Transfer) ─> T6 (Security fix)
T8 (Error handling) ─────────> T7
T9 (eprintln fix) ───────────> T7
T10 (Boolean flags) ─────────> T7
```

Tasks T1, T2, T8, T9, T10 are independent and can run in parallel.
Tasks T3, T4 depend on T1 (FormState).
Tasks T5, T6 depend on T2 (SSH args).
Task T7 (dead code) runs last after all others.

---

### Task 1: Unify InsertState + EditState into FormState

**Covers:** DRY violation, SRP for form state

**Rationale:** `InsertState` and `EditState` are 95% identical structs with identical methods. This task merges them into a single `FormState` that works for both insert and edit modes.

**Files:**
- Modify: `src/tui/app.rs` (lines 11-337 → replace InsertField/EditField/InsertState/EditState with FormState)
- Modify: `src/main.rs` (all references to InsertState/EditState/InsertField/EditField)
- Test: existing `src/tui/app.rs` tests (lines 793-1102)

**Interfaces:**
- Consumes: `Auth`, `Server` from `src/config/models.rs`
- Produces: `FormState` struct with `mode: FormMode` field, all existing methods

- [ ] **Step 1: Create FormState in app.rs**

In `src/tui/app.rs`, add the new types AFTER the existing `ConfirmState` block (line 347) and BEFORE `InputMode` (line 349):

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum FormField {
    Name,
    Host,
    Port,
    User,
    AuthType,
    KeyPath,
    Passphrase,
    Password,
    Tags,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormMode {
    Insert,
    Edit { server_index: usize },
}

#[derive(Debug, Clone)]
pub struct FormState {
    pub field: FormField,
    pub mode: FormMode,
    pub name: String,
    pub host: String,
    pub port: String,
    pub user: String,
    pub auth_type: String,
    pub key_path: String,
    pub passphrase: String,
    pub password: String,
    pub tags: String,
}

impl FormState {
    pub fn new_insert() -> Self {
        Self {
            field: FormField::Name,
            mode: FormMode::Insert,
            name: String::new(),
            host: String::new(),
            port: "22".to_string(),
            user: "root".to_string(),
            auth_type: "key".to_string(),
            key_path: "~/.ssh/id_rsa".to_string(),
            passphrase: String::new(),
            password: String::new(),
            tags: String::new(),
        }
    }

    pub fn new_edit(server: &Server, index: usize) -> Self {
        let (auth_type, key_path, passphrase) = match &server.auth {
            Auth::Key { path, passphrase } => (
                "key".to_string(),
                path.clone(),
                passphrase.clone().unwrap_or_default(),
            ),
            Auth::Password { .. } => ("password".to_string(), String::new(), String::new()),
        };
        Self {
            field: FormField::Name,
            mode: FormMode::Edit { server_index: index },
            name: server.name.clone(),
            host: server.host.clone(),
            port: server.port.to_string(),
            user: server.user.clone(),
            auth_type,
            key_path,
            passphrase,
            password: String::new(),
            tags: server.tags.join(", "),
        }
    }

    pub fn is_key_auth(&self) -> bool {
        self.auth_type.to_lowercase() == "key"
    }

    pub fn server_index(&self) -> Option<usize> {
        if let FormMode::Edit { server_index } = self.mode {
            Some(server_index)
        } else {
            None
        }
    }

    pub fn current_value(&self) -> &str {
        match self.field {
            FormField::Name => &self.name,
            FormField::Host => &self.host,
            FormField::Port => &self.port,
            FormField::User => &self.user,
            FormField::AuthType => &self.auth_type,
            FormField::KeyPath => &self.key_path,
            FormField::Passphrase => &self.passphrase,
            FormField::Password => &self.password,
            FormField::Tags => &self.tags,
        }
    }

    pub fn current_value_mut(&mut self) -> &mut String {
        match self.field {
            FormField::Name => &mut self.name,
            FormField::Host => &mut self.host,
            FormField::Port => &mut self.port,
            FormField::User => &mut self.user,
            FormField::AuthType => &mut self.auth_type,
            FormField::KeyPath => &mut self.key_path,
            FormField::Passphrase => &mut self.passphrase,
            FormField::Password => &mut self.password,
            FormField::Tags => &mut self.tags,
        }
    }

    pub fn next_field(&mut self) {
        self.field = if self.is_key_auth() {
            match self.field {
                FormField::Name => FormField::Host,
                FormField::Host => FormField::Port,
                FormField::Port => FormField::User,
                FormField::User => FormField::AuthType,
                FormField::AuthType => FormField::KeyPath,
                FormField::KeyPath => FormField::Passphrase,
                FormField::Passphrase => FormField::Tags,
                FormField::Tags => FormField::Name,
            }
        } else {
            match self.field {
                FormField::Name => FormField::Host,
                FormField::Host => FormField::Port,
                FormField::Port => FormField::User,
                FormField::User => FormField::AuthType,
                FormField::AuthType => FormField::Password,
                FormField::Password => FormField::Tags,
                FormField::Tags => FormField::Name,
            }
        };
    }

    pub fn prev_field(&mut self) {
        self.field = if self.is_key_auth() {
            match self.field {
                FormField::Name => FormField::Tags,
                FormField::Host => FormField::Name,
                FormField::Port => FormField::Host,
                FormField::User => FormField::Port,
                FormField::AuthType => FormField::User,
                FormField::KeyPath => FormField::AuthType,
                FormField::Passphrase => FormField::KeyPath,
                FormField::Tags => FormField::Passphrase,
            }
        } else {
            match self.field {
                FormField::Name => FormField::Tags,
                FormField::Host => FormField::Name,
                FormField::Port => FormField::Host,
                FormField::User => FormField::Port,
                FormField::AuthType => FormField::User,
                FormField::Password => FormField::AuthType,
                FormField::Tags => FormField::Password,
            }
        };
    }

    pub fn build_auth(&self) -> Auth {
        if self.auth_type.to_lowercase() == "password" {
            Auth::Password {
                vault_key: self.password.clone(),
            }
        } else {
            Auth::Key {
                path: if self.key_path.is_empty() {
                    "~/.ssh/id_rsa".to_string()
                } else {
                    self.key_path.clone()
                },
                passphrase: if self.passphrase.is_empty() {
                    None
                } else {
                    Some(self.passphrase.clone())
                },
            }
        }
    }

    pub fn toggle_auth_type(&mut self) {
        if self.auth_type == "key" {
            self.auth_type = "password".to_string();
        } else {
            self.auth_type = "key".to_string();
        }
    }
}
```

- [ ] **Step 2: Update App struct to use FormState**

In `src/tui/app.rs`, change the `App` struct fields (line 386-387):
```rust
// BEFORE:
pub insert_state: Option<InsertState>,
pub edit_state: Option<EditState>,

// AFTER:
pub form_state: Option<FormState>,
```

- [ ] **Step 3: Remove old types**

Delete the old `EditField`, `EditState`, `InsertField`, `InsertState` types and all their `impl` blocks from `src/tui/app.rs` (lines 11-337). Keep only `FormState`, `FormField`, `FormMode`.

- [ ] **Step 4: Update App::new()**

In `App::new()` (line 419), change:
```rust
// BEFORE:
insert_state: None,
edit_state: None,

// AFTER:
form_state: None,
```

- [ ] **Step 5: Update all main.rs references**

In `src/main.rs`, perform these replacements throughout:
- `app.insert_state` → `app.form_state` (when inserting)
- `app.edit_state` → `app.form_state` (when editing)
- `state: tui::app::InsertState` → `form: tui::app::FormState`
- `state: tui::app::EditState` → `form: tui::app::FormState`
- `tui::app::InsertField::` → `tui::app::FormField::`
- `tui::app::EditField::` → `tui::app::FormField::`
- `tui::app::InsertState::new()` → `tui::app::FormState::new_insert()`
- `tui::app::EditState::from_server(server, index)` → `tui::app::FormState::new_edit(server, index)`

Key locations in main.rs:
- Line 225: `if app.insert_state.is_some() || app.edit_state.is_some()` → `if app.form_state.is_some()`
- Line 232: `if let Some(ref state) = app.insert_state` → `if let Some(ref form) = app.form_state` (then check `form.mode == FormMode::Insert`)
- Line 369: `else if let Some(ref edit) = app.edit_state` → merged into the same block
- Line 608: `app.insert_state = Some(...)` → `app.form_state = Some(FormState::new_insert())`
- Line 635: `app.edit_state = Some(...)` → `app.form_state = Some(FormState::new_edit(...))`
- Line 700-703: `app.insert_state = None` → `app.form_state = None`
- Line 698: `if let Some(ref mut state) = app.insert_state` → `if let Some(ref mut form) = app.form_state`
- Line 774: `if let Some(ref mut edit) = app.edit_state` → merged into same block
- Line 827-828: `app.edit_state = None` → `app.form_state = None`

- [ ] **Step 6: Update modal rendering in main.rs**

The Insert modal (lines 232-368) and Edit modal (lines 369-505) must be merged into one block that checks `form.mode`:
```rust
if let Some(ref form) = app.form_state {
    let area = f.area();
    let is_insert = matches!(form.mode, tui::app::FormMode::Insert);
    let is_key = form.is_key_auth();
    let height: u16 = if is_key { 16 } else { 14 };
    let width: u16 = 50;
    // ... rest uses `form.` instead of `state.` or `edit.`
    // Title changes based on is_insert:
    // Insert: "  ➕ Novo Servidor  "
    // Edit: "  ✏️ Editar Servidor  "
}
```

- [ ] **Step 7: Update tests in app.rs**

Rewrite the test helper `test_servers()` and all tests that use `InsertState`/`EditState`:
```rust
// Replace test helpers:
fn test_servers() -> Vec<Server> {
    vec![
        Server {
            name: "server1".into(), host: "192.168.1.1".into(), port: 22,
            user: "user".into(),
            auth: Auth::Key { path: "~/.ssh/id_rsa".into(), passphrase: None },
            tags: vec!["prod".into()],
            pinned: false, last_connected: None, connection_count: 0,
            bookmarks: vec![], agent_forwarding: false, proxy_jump: None,
        },
        Server {
            name: "server2".into(), host: "192.168.1.2".into(), port: 22,
            user: "user".into(),
            auth: Auth::Password { vault_key: "key".into() },
            tags: vec!["dev".into()],
            pinned: true, last_connected: None, connection_count: 0,
            bookmarks: vec![], agent_forwarding: false, proxy_jump: None,
        },
    ]
}
```

All existing tests (`test_edit_state_*`, `test_insert_state_*`) must be rewritten to use `FormState::new_edit()` and `FormState::new_insert()`. The test logic stays identical — only type names change.

- [ ] **Step 8: Run tests**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | tail -10
```
Expected: 0 compile errors, all 350+ tests pass.

- [ ] **Step 9: Commit**

```bash
git add src/tui/app.rs src/main.rs
git commit -m "refactor: unify InsertState/EditState into FormState"
```

---

### Task 2: Extract SSH args builder into reusable helper

**Covers:** SSH args duplication (3 places), missing features in upload/download

**Rationale:** `native_shell_command()` in main.rs builds SSH args correctly (with `-A`, `-J`), but upload/download handlers duplicate the args logic inline **without** `-A` and `-J`. This extracts a shared `build_ssh_args()` that all three call sites use.

**Files:**
- Create: `src/ssh/args.rs`
- Modify: `src/ssh/mod.rs` (add `pub mod args;`)
- Modify: `src/main.rs` (use `ssh::args::build_ssh_args()` in native_shell_command, upload, download)

**Interfaces:**
- Produces: `pub fn build_ssh_args(server: &Server) -> (String, Vec<String>)` — returns (command, args)
- Consumes: `Server` from `config::models`

- [ ] **Step 1: Create src/ssh/args.rs**

```rust
use crate::config::models::{Auth, Server};

/// Build SSH command + args for connecting to a server.
/// Returns (program, args_vec). For password auth, program is "sshpass".
pub fn build_ssh_args(server: &Server) -> (String, Vec<String>) {
    let mut ssh_args = vec![];

    if server.port != 22 {
        ssh_args.push("-p".to_string());
        ssh_args.push(server.port.to_string());
    }

    if server.agent_forwarding {
        ssh_args.push("-A".to_string());
    }

    if let Some(ref jump) = server.proxy_jump {
        if !jump.is_empty() {
            ssh_args.push("-J".to_string());
            ssh_args.push(jump.clone());
        }
    }

    match &server.auth {
        Auth::Key { path, .. } => {
            let expanded = shellexpand::tilde(path).into_owned();
            ssh_args.push("-i".to_string());
            ssh_args.push(expanded);
        }
        Auth::Password { vault_key } => {
            if !vault_key.is_empty() {
                let mut args = vec![
                    "sshpass".to_string(),
                    "-p".to_string(),
                    vault_key.clone(),
                    "ssh".to_string(),
                ];
                args.append(&mut ssh_args);
                args.push(format!("{}@{}", server.user, server.host));
                return ("sshpass".to_string(), args);
            }
        }
    }

    ssh_args.push(format!("{}@{}", server.user, server.host));
    ("ssh".to_string(), ssh_args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::models::{Auth, Server};

    fn make_server(auth: Auth) -> Server {
        Server {
            name: "test".into(), host: "example.com".into(), port: 22,
            user: "root".into(), auth,
            tags: vec![], pinned: false, last_connected: None,
            connection_count: 0, bookmarks: vec![],
            agent_forwarding: false, proxy_jump: None,
        }
    }

    #[test]
    fn test_key_auth() {
        let server = make_server(Auth::Key { path: "~/.ssh/id_ed25519".into(), passphrase: None });
        let (cmd, args) = build_ssh_args(&server);
        assert_eq!(cmd, "ssh");
        assert!(args.contains(&"-i".to_string()));
        assert!(args.contains(&"root@example.com".to_string()));
    }

    #[test]
    fn test_password_auth() {
        let server = make_server(Auth::Password { vault_key: "secret123".into() });
        let (cmd, args) = build_ssh_args(&server);
        assert_eq!(cmd, "sshpass");
        assert_eq!(args[1], "-p");
        assert_eq!(args[2], "secret123");
    }

    #[test]
    fn test_custom_port() {
        let mut server = make_server(Auth::Password { vault_key: "pass".into() });
        server.port = 2222;
        let (_, args) = build_ssh_args(&server);
        assert!(args.contains(&"2222".to_string()));
    }

    #[test]
    fn test_agent_forwarding() {
        let mut server = make_server(Auth::Key { path: "~/.ssh/id_rsa".into(), passphrase: None });
        server.agent_forwarding = true;
        let (_, args) = build_ssh_args(&server);
        assert!(args.contains(&"-A".to_string()));
    }

    #[test]
    fn test_proxy_jump() {
        let mut server = make_server(Auth::Key { path: "~/.ssh/id_rsa".into(), passphrase: None });
        server.proxy_jump = Some("user@bastion.example.com".into());
        let (_, args) = build_ssh_args(&server);
        assert!(args.contains(&"-J".to_string()));
    }

    #[test]
    fn test_empty_proxy_jump_ignored() {
        let mut server = make_server(Auth::Key { path: "~/.ssh/id_rsa".into(), passphrase: None });
        server.proxy_jump = Some("".into());
        let (_, args) = build_ssh_args(&server);
        assert!(!args.contains(&"-J".to_string()));
    }
}
```

- [ ] **Step 2: Register module in src/ssh/mod.rs**

Add to `src/ssh/mod.rs`:
```rust
pub mod args;
pub mod auth;
pub mod connection;
pub mod service;

pub use service::{SshService, SessionStatus};
```

- [ ] **Step 3: Update native_shell_command in main.rs**

Replace `fn native_shell_command` (lines 48-90) with:
```rust
fn native_shell_command(server: &config::Server) -> (String, Vec<String>) {
    crate::ssh::args::build_ssh_args(server)
}
```

- [ ] **Step 4: Update upload handler in main.rs**

In the upload handler (lines 1185-1234), replace the inline SSH args construction with:
```rust
// Replace lines 1188-1210 with:
let (cmd, final_args) = crate::ssh::args::build_ssh_args(&server_clone);
// Then use `cmd` and `final_args` instead of building args inline
```

The full upload block becomes:
```rust
tokio::task::spawn_blocking(move || {
    for (name, local, remote) in &paths_clone {
        let (ref ssh_cmd, ref base_args) = crate::ssh::args::build_ssh_args(&server_clone);
        let mut final_args = base_args.clone();
        final_args.push(format!("cat > {}", remote));

        let local_file = std::fs::File::open(local)
            .map_err(|e| format!("Erro ao ler {}: {}", local, e));
        let child = match local_file {
            Ok(f) => std::process::Command::new(ssh_cmd)
                .args(&final_args)
                .stdin(f)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| format!("ssh erro: {}", e)),
            Err(e) => Err(e),
        };
        match child {
            Ok(mut proc) => {
                let _ = proc.wait();
                let _ = result_tx2.send(SftpOpResult::Upload(name.clone()));
            }
            Err(e) => {
                let _ = result_tx2.send(SftpOpResult::Error(format!("SCP erro: {}", e)));
            }
        }
    }
    let _ = result_tx2.send(SftpOpResult::Error("__done__".to_string()));
});
```

- [ ] **Step 5: Update download handler in main.rs**

In the download handler (lines 1299-1364), replace inline SSH args:
```rust
tokio::task::spawn_blocking(move || {
    for (name, remote, local) in &paths_clone {
        let (ref ssh_cmd, ref base_args) = crate::ssh::args::build_ssh_args(&server_clone);
        let mut final_args = base_args.clone();
        final_args.push(format!("cat {}", remote));

        let child = std::process::Command::new(ssh_cmd)
            .args(&final_args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("ssh erro: {}", e));
        // ... rest unchanged
    }
    let _ = result_tx2.send(SftpOpResult::Error("__done__".to_string()));
});
```

- [ ] **Step 6: Run tests**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | tail -10
```
Expected: 0 compile errors, all tests pass (including new ssh/args tests).

- [ ] **Step 7: Commit**

```bash
git add src/ssh/args.rs src/ssh/mod.rs src/main.rs
git commit -m "refactor: extract SSH args builder into ssh/args.rs"
```

---

### Task 3: Extract modal rendering to tui/modals.rs

**Covers:** main.rs God Object, SRP for rendering

**Rationale:** Insert/Edit/Confirm modal rendering accounts for ~350 lines inside `main.rs`'s `terminal.draw()` closure. Extracting to a dedicated module makes main.rs shorter and modals independently testable.

**Files:**
- Create: `src/tui/modals.rs`
- Modify: `src/tui/mod.rs` (add `pub mod modals;`)
- Modify: `src/main.rs` (replace inline modal rendering with calls to modals.rs)

**Interfaces:**
- Produces: `pub fn render_insert_edit_modal(f: &mut Frame, form: &FormState)` and `pub fn render_confirm_modal(f: &mut Frame, confirm: &ConfirmState)`
- Consumes: `FormState`, `ConfirmState`, `Theme`, ratatui types

- [ ] **Step 1: Create src/tui/modals.rs**

Move the rendering code from main.rs lines 232-505 (Insert modal) and 522-561 (Confirm modal) into this new file. Use `FormState` and `FormMode` from the new types.

```rust
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::app::{ConfirmState, FormMode, FormState};
use super::theme::Theme;

pub fn render_form_modal(f: &mut Frame, form: &FormState) {
    let area = f.area();
    let is_insert = matches!(form.mode, FormMode::Insert);
    let is_key = form.is_key_auth();
    let height: u16 = if is_key { 16 } else { 14 };
    let width: u16 = 50;
    let x = (area.width - width) / 2;
    let y = (area.height - height) / 2;
    let rect = Rect::new(x, y, width, height);

    let mut lines = vec![];

    // Title
    let title = if is_insert { "  ➕ Novo Servidor  " } else { "  ✏️ Editar Servidor  " };
    lines.push(Line::from(vec![
        Span::styled(title, Style::default().fg(Theme::accent()).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from("─".repeat(width as usize - 2)));

    // Base fields
    let base_fields = [
        (super::app::FormField::Name, "📝 Nome", &form.name),
        (super::app::FormField::Host, "🌐 Host", &form.host),
        (super::app::FormField::Port, "🔌 Porta", &form.port),
        (super::app::FormField::User, "👤 Usuário", &form.user),
    ];

    for (field_type, label, value) in &base_fields {
        let is_active = &form.field == field_type;
        render_field_line(&mut lines, is_active, &format!("{}: ", label), value, width);
    }

    // Auth type toggle
    {
        let is_active = form.field == super::app::FormField::AuthType;
        let marker = if is_active { "▶" } else { " " };
        let auth_label = if is_key { "key" } else { "password" };
        let style = if is_active {
            Style::default().fg(Theme::primary()).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::text())
        };
        let mut spans = vec![
            Span::styled(format!("{} ", marker), Style::default().fg(Theme::accent())),
            Span::styled("🔐 Auth: ", Style::default().fg(Theme::secondary())),
            Span::styled(format!("[{}]", auth_label), style),
        ];
        if is_active {
            spans.push(Span::styled(" ← →", Style::default().fg(Theme::text_dim())));
        }
        lines.push(Line::from(spans));
    }

    // Dynamic fields based on auth type
    if is_key {
        render_field_line(&mut lines, form.field == super::app::FormField::KeyPath, "🔑 Chave: ", &form.key_path, width);
        render_field_line(&mut lines, form.field == super::app::FormField::Passphrase, "🔑 Senha: ", &form.passphrase, width);
    } else {
        render_field_line(&mut lines, form.field == super::app::FormField::Password, "🔑 Senha: ", &form.password, width);
    }

    // Tags
    let tags_display = if form.tags.is_empty() { " (nenhuma)" } else { &form.tags };
    render_field_line(&mut lines, form.field == super::app::FormField::Tags, "🏷️ Tags: ", tags_display, width);

    // Footer hints
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  ↑/↓/Tab: próximo campo  ", Style::default().fg(Theme::text_dim())),
        Span::styled("│  Esc: cancelar", Style::default().fg(Theme::error())),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Enter: salvar (no último campo)  ", Style::default().fg(Theme::success())),
    ]));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::modal_border_style())
        .style(Style::default().bg(Color::Black));

    let input = Paragraph::new(lines).block(block);
    f.render_widget(input, rect);
}

fn render_field_line(lines: &mut Vec<Line>, is_active: bool, label: &str, value: &str, _width: u16) {
    let marker = if is_active { "▶" } else { " " };
    let style = if is_active {
        Style::default().fg(Theme::primary()).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Theme::text())
    };
    lines.push(Line::from(vec![
        Span::styled(format!("{} ", marker), Style::default().fg(Theme::accent())),
        Span::styled(label, Style::default().fg(Theme::secondary())),
        Span::styled(value.to_string(), style),
    ]));
}

pub fn render_confirm_modal(f: &mut Frame, confirm: &ConfirmState) {
    let area = f.area();
    let width = 45u16;
    let height = 5u16;
    let x = (area.width - width) / 2;
    let y = (area.height - height) / 2;
    let rect = Rect::new(x, y, width, height);

    let msg = match &confirm.action {
        super::app::ConfirmAction::DeleteServer { name } => {
            format!("Remover servidor '{}'?", name)
        }
    };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("  {}  ", msg),
                Style::default().fg(Theme::text()).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Enter:Confirmar  ", Style::default().fg(Theme::success())),
            Span::styled("│  Esc:Cancelar", Style::default().fg(Theme::error())),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" ⚠ Confirmação ")
        .title_style(Theme::modal_title_style())
        .border_style(Theme::modal_border_style())
        .style(Style::default().bg(Color::Black));

    f.render_widget(ratatui::widgets::Clear, rect);
    f.render_widget(Paragraph::new(lines).block(block), rect);
}
```

- [ ] **Step 2: Register module in src/tui/mod.rs**

Add `pub mod modals;` to `src/tui/mod.rs`.

- [ ] **Step 3: Replace inline rendering in main.rs**

In `main.rs`, inside the `terminal.draw()` closure, replace the Insert/Edit/Confirm modal blocks with:
```rust
// Overlay when form is open
if app.form_state.is_some() {
    let area = f.area();
    let overlay = ratatui::widgets::Block::default()
        .style(ratatui::style::Style::default().bg(ratatui::style::Color::Black).add_modifier(ratatui::style::Modifier::DIM));
    f.render_widget(overlay, area);
}

if let Some(ref form) = app.form_state {
    tui::modals::render_form_modal(f, form);
}

// ... confirm modal:
if let Some(ref confirm) = app.confirm_state {
    tui::modals::render_confirm_modal(f, confirm);
}
```

Remove the ~350 lines of inline modal rendering (lines 232-505 and 522-561).

- [ ] **Step 4: Run tests**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | tail -10
```

- [ ] **Step 5: Commit**

```bash
git add src/tui/modals.rs src/tui/mod.rs src/main.rs
git commit -m "refactor: extract modal rendering to tui/modals.rs"
```

---

### Task 4: Extract event handlers to tui/handlers.rs

**Covers:** main.rs God Object, event handling separation

**Rationale:** All keyboard event handling lives inside `main()` as a massive match block. Extracting per-view handlers to `tui/handlers.rs` reduces main.rs by ~600 lines and makes key handling independently testable.

**Files:**
- Create: `src/tui/handlers.rs`
- Modify: `src/tui/mod.rs` (add `pub mod handlers;`)
- Modify: `src/main.rs` (replace match blocks with handler calls)

**Interfaces:**
- Produces: `pub fn handle_server_list_key(app: &mut App, key: KeyEvent, terminal: &mut ...) -> bool` and `pub fn handle_sftp_key(app: &mut App, key: KeyEvent) -> bool`
- Consumes: `App`, `KeyEvent`, `Terminal`

**NOTE:** Because the handlers need access to `terminal` (for SSH handoff) and `config` (for save), this task uses a `HandlerCtx` struct:

```rust
pub struct HandlerCtx<'a> {
    pub terminal: &'a mut Terminal<CrosstermBackend<io::Stdout>>,
}
```

- [ ] **Step 1: Create src/tui/handlers.rs**

Move the key handling blocks from main.rs:
- Lines 597-879 (ServerList key handling) → `handle_server_list_key()`
- Lines 881-1510 (SFTP key handling) → `handle_sftp_key()`

The function signatures:
```rust
use crossterm::event::KeyEvent;
use super::app::App;

/// Returns true if the app should quit after handling this key.
pub fn handle_global_key(app: &mut App, key: KeyEvent) -> bool {
    // Help modal intercept
    // ? handler
    false
}

/// Handle keys in ServerList view. Returns true if should quit.
pub fn handle_server_list_key(app: &mut App, key: KeyEvent, ctx: &mut HandlerCtx) -> bool {
    // All InputMode matches for ServerList
}

/// Handle keys in SFTP view. Returns true if should quit.
pub fn handle_sftp_key(app: &mut App, key: KeyEvent) -> bool {
    // All SFTP key handling
}
```

- [ ] **Step 2: Simplify main event loop**

Replace the massive match block in `main()` (lines 572-1516) with:
```rust
Event::Key(key) => {
    if key.kind == KeyEventKind::Press {
        // Global handlers
        if tui::handlers::handle_global_key(&mut app, key) {
            continue;
        }

        match app.current_view {
            tui::app::CurrentView::ServerList => {
                let mut ctx = tui::handlers::HandlerCtx { terminal: &mut terminal };
                if tui::handlers::handle_server_list_key(&mut app, key, &mut ctx) {
                    break;
                }
            }
            tui::app::CurrentView::SftpBrowser => {
                if tui::handlers::handle_sftp_key(&mut app, key) {
                    break;
                }
            }
            tui::app::CurrentView::SshTerminal => {}
        }
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | tail -10
```

- [ ] **Step 4: Commit**

```bash
git add src/tui/handlers.rs src/tui/mod.rs src/main.rs
git commit -m "refactor: extract event handlers to tui/handlers.rs"
```

---

### Task 5: Extract SSH transfer logic to sftp/transfer.rs

**Covers:** SSH upload/download duplication in main.rs

**Rationale:** Upload and download via SSH (~240 lines) are currently inline in main.rs event loop. Extracting to `sftp/transfer.rs` makes main.rs shorter and transfer logic reusable/testable.

**Files:**
- Create: `src/sftp/transfer.rs`
- Modify: `src/sftp/mod.rs` (add `pub mod transfer;`)
- Modify: `src/main.rs` (call transfer functions)

**Interfaces:**
- Produces: `pub fn upload_via_ssh(server: &Server, local: &str, remote: &str) -> Result<(), String>` and `pub fn download_via_ssh(server: &Server, remote: &str, local: &str) -> Result<(), String>`
- Consumes: `Server` from `config::models`, `build_ssh_args` from `ssh::args`

- [ ] **Step 1: Create src/sftp/transfer.rs**

```rust
use crate::config::models::Server;
use crate::ssh::args::build_ssh_args;
use std::io::Read;
use std::io::Write;

pub fn upload_via_ssh(server: &Server, local: &str, remote: &str) -> Result<(), String> {
    let (cmd, mut args) = build_ssh_args(server);
    args.push(format!("cat > {}", remote));

    let file = std::fs::File::open(local)
        .map_err(|e| format!("Erro ao ler {}: {}", local, e))?;

    let mut child = std::process::Command::new(&cmd)
        .args(&args)
        .stdin(file)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("ssh erro: {}", e))?;

    child.wait().map_err(|e| format!("ssh wait erro: {}", e))?;
    Ok(())
}

pub fn download_via_ssh(server: &Server, remote: &str, local: &str) -> Result<(), String> {
    let (cmd, mut args) = build_ssh_args(server);
    args.push(format!("cat {}", remote));

    let mut child = std::process::Command::new(&cmd)
        .args(&args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("ssh erro: {}", e))?;

    let mut stdout = child.stdout.take().ok_or("No stdout")?;
    let mut file = std::fs::File::create(local)
        .map_err(|e| format!("Erro ao criar {}: {}", local, e))?;

    let mut buf = [0u8; 65536];
    loop {
        match stdout.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => { let _ = file.write_all(&buf[..n]); }
            Err(_) => break,
        }
    }

    child.wait().map_err(|e| format!("ssh wait erro: {}", e))?;
    Ok(())
}
```

- [ ] **Step 2: Register module in src/sftp/mod.rs**

Add `pub mod transfer;` to `src/sftp/mod.rs`.

- [ ] **Step 3: Update main.rs upload handler**

Replace the upload `spawn_blocking` block (lines 1185-1234) with:
```rust
tokio::task::spawn_blocking(move || {
    for (name, local, remote) in &paths_clone {
        match crate::sftp::transfer::upload_via_ssh(&server_clone, local, remote) {
            Ok(()) => { let _ = result_tx2.send(SftpOpResult::Upload(name.clone())); }
            Err(e) => { let _ = result_tx2.send(SftpOpResult::Error(format!("SCP erro: {}", e))); }
        }
    }
    let _ = result_tx2.send(SftpOpResult::Error("__done__".to_string()));
});
```

- [ ] **Step 4: Update main.rs download handler**

Replace the download `spawn_blocking` block (lines 1299-1364) with:
```rust
tokio::task::spawn_blocking(move || {
    for (name, remote, local) in &paths_clone {
        match crate::sftp::transfer::download_via_ssh(&server_clone, remote, local) {
            Ok(()) => { let _ = result_tx2.send(SftpOpResult::Upload(name.clone())); }
            Err(e) => { let _ = result_tx2.send(SftpOpResult::Error(format!("Erro: {}", e))); }
        }
    }
    let _ = result_tx2.send(SftpOpResult::Error("__done__".to_string()));
});
```

- [ ] **Step 5: Run tests**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | tail -10
```

- [ ] **Step 6: Commit**

```bash
git add src/sftp/transfer.rs src/sftp/mod.rs src/main.rs
git commit -m "refactor: extract SSH transfer logic to sftp/transfer.rs"
```

---

### Task 6: Fix security — sshpass password visibility

**Covers:** Security: password visible in /proc/*/cmdline

**Rationale:** `sshpass -p <password>` exposes the password in the process command line visible to all users via `ps`. Fix: use `sshpass -e` which reads from `SSH_ASKPASS` env var, or set password via env var + stdin. The simplest portable approach is to set `SSHPASS` env var and use `sshpass -e`.

**Files:**
- Modify: `src/ssh/args.rs` (change `-p` to `-e` + env var approach)
- Modify: `src/sftp/transfer.rs` (no change needed — uses `build_ssh_args`)
- Modify: `src/main.rs` (SSH handoff needs env var setup)

**Interfaces:**
- Changes: `build_ssh_args()` return type changes to `(String, Vec<String>, Option<String>)` where the third element is the password (for env var setup by caller)

**IMPORTANT:** `sshpass -e` reads password from `SSHPASS` env var. The caller must set this env var before spawning the process. Since `Command::env()` is the standard way, we return the password separately.

- [ ] **Step 1: Update build_ssh_args in src/ssh/args.rs**

Change the return type to `(String, Vec<String>, Option<String>)`:

```rust
use crate::config::models::{Auth, Server};

/// Build SSH command + args.
/// Returns (program, args, optional_password).
/// For password auth: program="sshpass", password is set via SSHPASS env var.
pub fn build_ssh_args(server: &Server) -> (String, Vec<String>, Option<String>) {
    let mut ssh_args = vec![];
    let mut password = None;

    if server.port != 22 {
        ssh_args.push("-p".to_string());
        ssh_args.push(server.port.to_string());
    }

    if server.agent_forwarding {
        ssh_args.push("-A".to_string());
    }

    if let Some(ref jump) = server.proxy_jump {
        if !jump.is_empty() {
            ssh_args.push("-J".to_string());
            ssh_args.push(jump.clone());
        }
    }

    match &server.auth {
        Auth::Key { path, .. } => {
            let expanded = shellexpand::tilde(path).into_owned();
            ssh_args.push("-i".to_string());
            ssh_args.push(expanded);
        }
        Auth::Password { vault_key } => {
            if !vault_key.is_empty() {
                password = Some(vault_key.clone());
                ssh_args.insert(0, "ssh".to_string());
                return ("sshpass".to_string(), ssh_args, password);
            }
        }
    }

    ssh_args.push(format!("{}@{}", server.user, server.host));
    ("ssh".to_string(), ssh_args, password)
}

/// Spawn an SSH process with optional password via SSHPASS env var.
pub fn spawn_ssh_process(
    server: &Server,
    extra_args: Vec<String>,
    stdin: Option<std::process::Stdio>,
    stdout: Option<std::process::Stdio>,
    stderr: Option<std::process::Stdio>,
) -> Result<std::process::Child, String> {
    let (cmd, mut args, password) = build_ssh_args(server);
    args.extend(extra_args);

    let mut command = std::process::Command::new(&cmd);
    command.args(&args);
    if let Some(s) = stdin { command.stdin(s); }
    if let Some(s) = stdout { command.stdout(s); }
    if let Some(s) = stderr { command.stderr(s); }

    // Set SSHPASS env var for password auth (sshpass -e reads it)
    if let Some(ref pw) = password {
        command.env("SSHPASS", pw);
    }

    command.spawn().map_err(|e| format!("{} erro: {}", cmd, e))
}
```

- [ ] **Step 2: Update native_shell_command in main.rs**

```rust
fn native_shell_command(server: &config::Server) -> (String, Vec<String>, Option<String>) {
    crate::ssh::args::build_ssh_args(server)
}
```

- [ ] **Step 3: Update run_native_shell_handoff in main.rs**

```rust
fn run_native_shell_handoff(server: &config::Server) -> Result<ExitStatus> {
    let (command, args, password) = native_shell_command(server);

    leave_tui()?;

    use std::io::Write;
    let _ = std::io::stdout().write_all(b"\x1bc");
    let _ = std::io::stdout().flush();

    let mut cmd = Command::new(&command);
    cmd.args(&args);
    if let Some(ref pw) = password {
        cmd.env("SSHPASS", pw);
    }

    let status = cmd.status()
        .with_context(|| format!("failed to start `{}`", command))?;

    reenter_tui()?;

    Ok(status)
}
```

- [ ] **Step 4: Update transfer.rs to use spawn_ssh_process**

```rust
pub fn upload_via_ssh(server: &Server, local: &str, remote: &str) -> Result<(), String> {
    let file = std::fs::File::open(local)
        .map_err(|e| format!("Erro ao ler {}: {}", local, e))?;

    let extra = vec![format!("cat > {}", remote)];
    let mut child = crate::ssh::args::spawn_ssh_process(
        server, extra,
        Some(file),
        Some(std::process::Stdio::piped()),
        Some(std::process::Stdio::piped()),
    )?;

    child.wait().map_err(|e| format!("ssh wait erro: {}", e))?;
    Ok(())
}

pub fn download_via_ssh(server: &Server, remote: &str, local: &str) -> Result<(), String> {
    let extra = vec![format!("cat {}", remote)];
    let mut child = crate::ssh::args::spawn_ssh_process(
        server, extra,
        None,
        Some(std::process::Stdio::piped()),
        Some(std::process::Stdio::piped()),
    )?;

    let mut stdout = child.stdout.take().ok_or("No stdout")?;
    let mut file = std::fs::File::create(local)
        .map_err(|e| format!("Erro ao criar {}: {}", local, e))?;

    let mut buf = [0u8; 65536];
    loop {
        match stdout.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => { let _ = file.write_all(&buf[..n]); }
            Err(_) => break,
        }
    }

    child.wait().map_err(|e| format!("ssh wait erro: {}", e))?;
    Ok(())
}
```

- [ ] **Step 5: Update upload handler in main.rs (if not already done by Task 5)**

If Task 5 already simplified the upload handler to use `transfer::upload_via_ssh()`, no change needed here. Otherwise, update similarly.

- [ ] **Step 6: Run tests**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | tail -10
```

- [ ] **Step 7: Commit**

```bash
git add src/ssh/args.rs src/sftp/transfer.rs src/main.rs
git commit -m "security: use SSHPASS env var instead of sshpass -p to hide passwords"
```

---

### Task 7: Fix error handling — stop silencing save_config errors

**Covers:** Silent config save failures

**Rationale:** 5+ places in main.rs use `let _ = config::save_config(...)` which silently discards errors. If the disk is full or keyring fails, the user never knows. Fix: at minimum log to notifications, ideally propagate.

**Files:**
- Modify: `src/main.rs` (all `let _ = config::save_config` locations)
- Modify: `src/tui/handlers.rs` (if extracted by Task 4)

**Interfaces:**
- Changes: error handling in all config save locations

- [ ] **Step 1: Find all let _ = config::save_config occurrences**

```bash
grep -n "let _ = config::save_config" src/main.rs
grep -n "let _ = crate::config::save_config" src/main.rs
grep -n "let _ =.*save_config" src/main.rs src/tui/handlers.rs src/tui/app.rs
```

- [ ] **Step 2: Replace each with notification on error**

At each location, replace:
```rust
// BEFORE:
let _ = config::save_config(
    &config::AppConfig { servers: app.servers.clone(), sort_by: app.sort_by.clone() },
    &config::get_config_path(),
);

// AFTER:
if let Err(e) = config::save_config(
    &config::AppConfig { servers: app.servers.clone(), sort_by: app.sort_by.clone() },
    &config::get_config_path(),
) {
    app.notifications.warning(&format!("Falha ao salvar config: {}", e));
}
```

Do this for every occurrence (typically 5-6 in main.rs/handlers.rs):
1. Pin/unpin save (line ~644)
2. Insert server save (line ~757)
3. Edit server save (line ~821)
4. Delete server save (line ~864)
5. Bookmark save (line ~945)
6. Import SSH config save (line ~526 in app.rs)

- [ ] **Step 3: Also fix save_config in app.rs (import_ssh_config)**

In `app.rs:import_ssh_config()` (line 526), same pattern:
```rust
if let Err(e) = crate::config::save_config(...) {
    self.notifications.warning(&format!("Falha ao salvar config: {}", e));
}
```

- [ ] **Step 4: Run tests**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | tail -10
```

- [ ] **Step 5: Commit**

```bash
git add src/main.rs src/tui/app.rs src/tui/handlers.rs
git commit -m "fix: notify user when config save fails instead of silently ignoring"
```

---

### Task 8: Fix eprintln in vault/keyring.rs

**Covers:** eprintln corrupting TUI output

**Rationale:** `vault/keyring.rs` uses `eprintln!` which corrupts ratatui output. The vault module doesn't have access to `App`, so it should return `Result` and let callers handle errors.

**Files:**
- Modify: `src/vault/keyring.rs` (change `store_password` to return `Result<(), Error>`)
- Modify: `src/config/file.rs` (handle keyring errors at call sites)

**Interfaces:**
- Changes: `store_password()` returns `Result<(), keyring::Error>` instead of `bool`
- Changes: `delete_password()` returns `Result<(), keyring::Error>` instead of `bool`
- `get_password()` stays `Option<String>` (already silent on failure)

- [ ] **Step 1: Update vault/keyring.rs**

```rust
use crate::config::models::Server;

const SERVICE: &str = "lazyssh";

fn account_key(server: &Server) -> String {
    format!("{}@{}:{}", server.user, server.host, server.port)
}

pub fn store_password(server: &Server, password: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE, &account_key(server))
        .map_err(|e| format!("keyring entry: {}", e))?;
    entry.set_password(password)
        .map_err(|e| format!("keyring store: {}", e))
}

pub fn get_password(server: &Server) -> Option<String> {
    keyring::Entry::new(SERVICE, &account_key(server))
        .ok()
        .and_then(|entry| entry.get_password().ok())
}

pub fn delete_password(server: &Server) {
    if let Ok(entry) = keyring::Entry::new(SERVICE, &account_key(server)) {
        let _ = entry.delete_credential();
    }
}

pub fn is_available() -> bool {
    keyring::Entry::new("lazyssh-test", "availability-check").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::models::{Auth, Server};

    fn test_server() -> Server {
        Server {
            name: "test".into(), host: "10.0.0.1".into(), port: 22,
            user: "root".into(),
            auth: Auth::Key { path: "~/.ssh/id_rsa".into(), passphrase: None },
            tags: vec![], pinned: false, last_connected: None,
            connection_count: 0, bookmarks: vec![],
            agent_forwarding: false, proxy_jump: None,
        }
    }

    #[test]
    fn test_account_key_format() {
        let server = test_server();
        assert_eq!(account_key(&server), "root@10.0.0.1:22");
    }

    #[test]
    #[ignore]
    fn test_store_get_delete_roundtrip() {
        let server = test_server();
        assert!(store_password(&server, "test_pass_123").is_ok());
        assert_eq!(get_password(&server), Some("test_pass_123".into()));
        delete_password(&server);
        assert_eq!(get_password(&server), None);
    }

    #[test]
    #[ignore]
    fn test_get_nonexistent_returns_none() {
        let mut server = test_server();
        server.host = "nonexistent-host-99999".into();
        assert_eq!(get_password(&server), None);
    }
}
```

- [ ] **Step 2: Update config/file.rs callers**

Update all `keyring::store_password()` and `keyring::delete_password()` calls:

In `save_config()` (line 47):
```rust
// BEFORE:
if !vault_key.is_empty() {
    keyring::store_password(server, vault_key)
}

// AFTER:
if !vault_key.is_empty() {
    keyring::store_password(server, vault_key).is_ok()
}
```

In `remove_server()` (line 98):
```rust
// BEFORE:
keyring::delete_password(server);

// AFTER:
keyring::delete_password(server); // already returns nothing now
```

In `update_server()` (line 113):
```rust
// BEFORE:
if let Auth::Password { .. } = &old_server.auth {
    keyring::delete_password(&old_server);
}

// AFTER:
if let Auth::Password { .. } = &old_server.auth {
    keyring::delete_password(&old_server);
}
```

- [ ] **Step 3: Run tests**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | tail -10
```

- [ ] **Step 4: Commit**

```bash
git add src/vault/keyring.rs src/config/file.rs
git commit -m "fix: remove eprintln from vault, return Result instead"
```

---

### Task 9: Remove dead code

**Covers:** Dead code cleanup

**Rationale:** ~200 lines of unused code pollutes the codebase and misleads future contributors.

**Files:**
- Modify: `src/main.rs` (remove dead code)
- Modify: `src/tui/app.rs` (remove `connect_ssh()`, unused SFTP methods)
- Modify: `src/tui/ssh_terminal.rs` (remove if confirmed unused)

- [ ] **Step 1: Remove key_event_to_bytes in main.rs**

Delete `fn key_event_to_bytes` and all its tests (lines 112-141, 1597-1674) — it's `#[allow(dead_code)]`.

- [ ] **Step 2: Remove connect_ssh() in app.rs**

Delete `pub fn connect_ssh(&mut self) {}` (line 787-789).

- [ ] **Step 3: Remove unused sftp_download_file in app.rs**

Delete `pub fn sftp_download_file()` (lines 700-710) — downloads use SSH, not SFTP.

- [ ] **Step 4: Remove unused sftp_upload_file in app.rs**

Delete `pub fn sftp_upload_file()` (lines 644-654) — uploads use SSH, not SFTP.

- [ ] **Step 5: Remove unused SftpOpResult variants**

In `src/tui/app.rs` (lines 365-375), remove unused variants:
```rust
pub enum SftpOpResult {
    Upload(String),
    Error(String),
    // Removed: ListDir, Download, Mkdir, Unlink, Rmdir, Rename, SetPermissions
}
```

- [ ] **Step 6: Clean up ssh_terminal.rs (optional, if no future use)**

`src/tui/ssh_terminal.rs` has 1059 lines but is a no-op in the current architecture (SSH uses external shell). **Keep for now** — it may be used in future for embedded SSH. Just add a `#[allow(dead_code)]` note if the compiler warns.

- [ ] **Step 7: Run tests**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | tail -10
```

- [ ] **Step 8: Commit**

```bash
git add src/main.rs src/tui/app.rs src/tui/ssh_terminal.rs
git commit -m "refactor: remove dead code (~200 lines)"
```

---

### Task 10: Replace boolean flags with enums

**Covers:** Boolean flags as control flow

**Rationale:** `help_visible: bool` and `is_transferring: bool` should be enums for clarity and extensibility.

**Files:**
- Modify: `src/tui/app.rs` (change `help_visible` to `overlay: Option<Overlay>`)
- Modify: `src/tui/sftp_browser.rs` (change `is_transferring` to `transfer_state: TransferState`)
- Modify: `src/main.rs` / `src/tui/handlers.rs` (update references)

- [ ] **Step 1: Add Overlay enum to app.rs**

```rust
#[derive(Debug, PartialEq)]
pub enum Overlay {
    Help,
}

// In App struct:
// BEFORE: pub help_visible: bool,
// AFTER:  pub overlay: Option<Overlay>,
```

Update `App::new()`:
```rust
// BEFORE: help_visible: false,
// AFTER:  overlay: None,
```

- [ ] **Step 2: Update all help_visible references**

In main.rs/handlers.rs, replace:
- `app.help_visible = true` → `app.overlay = Some(Overlay::Help)`
- `app.help_visible = false` → `app.overlay = None`
- `if app.help_visible` → `if app.overlay == Some(Overlay::Help)`

- [ ] **Step 3: Add TransferState enum to sftp_browser.rs**

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum TransferState {
    Idle,
    Transferring {
        file_name: String,
        bytes_total: u64,
        is_upload: bool,
        start_time: Instant,
    },
}

// In SftpState:
// BEFORE: pub is_transferring: bool,
// AFTER:  pub transfer_state: TransferState,
```

- [ ] **Step 4: Update all is_transferring references**

Replace `sftp.is_transferring` checks with `matches!(sftp.transfer_state, TransferState::Transferring { .. })`.
Replace `sftp.is_transferring = true` with `sftp.transfer_state = TransferState::Transferring { ... }`.
Replace `sftp.is_transferring = false` with `sftp.transfer_state = TransferState::Idle`.

- [ ] **Step 5: Run tests**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | tail -10
```

- [ ] **Step 6: Commit**

```bash
git add src/tui/app.rs src/tui/sftp_browser.rs src/main.rs src/tui/handlers.rs
git commit -m "refactor: replace boolean flags with enums (Overlay, TransferState)"
```

---

### Task 11: Final verification and cleanup

**Covers:** Ensure everything compiles, all tests pass, no warnings

**Rationale:** Final pass to catch any issues from the refactoring.

- [ ] **Step 1: Full build and test**

```bash
cargo build 2>&1
cargo test 2>&1
```

- [ ] **Step 2: Check for warnings**

```bash
cargo build 2>&1 | grep -i "warning" | head -20
```

Fix any warnings found.

- [ ] **Step 3: Run clippy**

```bash
cargo clippy 2>&1 | head -30
```

Fix any clippy warnings that are clear improvements (don't add abstractions).

- [ ] **Step 4: Final commit if needed**

```bash
git add -A
git commit -m "chore: final cleanup after code quality refactor"
```
