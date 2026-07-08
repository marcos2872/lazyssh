# LazySSH Rust TUI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use compose:subagent (recommended) or compose:execute to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an interactive TUI in Rust for managing SSH/SFTP connections with a vault for password storage.

**Architecture:** Ratatui-based TUI with async SSH/SFTP via russh. Config stored in TOML. Passwords encrypted with AES-256-GCM.

**Tech Stack:** Rust, Ratatui, crossterm, russh, tokio, toml, ring

## Global Constraints

- Rust edition 2021 minimum
- Config path: `~/.config/lazyssh/servers.toml`
- All async operations via tokio
- TUI must be responsive (non-blocking SSH operations)
- Passwords never stored in plaintext

---

## File Structure

```
lazyssh/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, CLI args
│   ├── config/
│   │   ├── mod.rs           # Config module
│   │   ├── models.rs        # Server, Auth structs
│   │   └── file.rs          # TOML read/write
│   ├── ssh/
│   │   ├── mod.rs           # SSH module
│   │   ├── connection.rs    # russh connection
│   │   └── auth.rs          # Auth methods
│   ├── sftp/
│   │   ├── mod.rs           # SFTP module
│   │   ├── local.rs         # Local filesystem
│   │   └── remote.rs        # Remote SFTP ops
│   ├── vault/
│   │   ├── mod.rs           # Vault module
│   │   └── crypto.rs        # AES-256-GCM
│   └── tui/
│       ├── mod.rs           # TUI module
│       ├── app.rs           # App state
│       ├── server_list.rs   # Server list view
│       ├── ssh_terminal.rs  # SSH terminal view
│       └── sftp_browser.rs  # SFTP dual-pane view
└── tests/
    ├── config_test.rs
    ├── vault_test.rs
    └── ssh_test.rs
```

---

### Task 1: Project Scaffolding

**Covers:** [S3]

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`

**Interfaces:**
- Produces: Runnable binary with basic TUI skeleton

- [ ] **Step 1: Initialize Cargo project**

Run: `cargo init lazyssh`
Expected: Creates `lazyssh/` directory with `Cargo.toml` and `src/main.rs`

- [ ] **Step 2: Add dependencies to Cargo.toml**

```toml
[package]
name = "lazyssh"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full"] }
ratatui = "0.29"
crossterm = "0.28"
russh = "0.50"
russh-keys = "0.50"
toml = "0.8"
serde = { version = "1", features = ["derive"] }
ring = "0.17"
pbkdf2 = "0.12"
rand = "0.8"
anyhow = "1"
dirs = "5"
fuzzy-matcher = "0.3"
```

- [ ] **Step 3: Create minimal main.rs**

```rust
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("LazySSH v0.1.0");
    Ok(())
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 5: Commit**

```bash
git add .
git commit -m "feat: initialize project with dependencies"
```

---

### Task 2: Config Models

**Covers:** [S4]

**Files:**
- Create: `src/config/mod.rs`
- Create: `src/config/models.rs`
- Test: `tests/config_test.rs`

**Interfaces:**
- Produces: `Server`, `Auth`, `AppConfig` structs with Serialize/Deserialize

- [ ] **Step 1: Write failing test for config models**

```rust
// tests/config_test.rs
use lazyssh::config::models::{Auth, Server};

#[test]
fn test_server_creation() {
    let server = Server {
        name: "myserver".to_string(),
        host: "192.168.1.100".to_string(),
        port: 22,
        user: "root".to_string(),
        auth: Auth::Key {
            path: "~/.ssh/id_ed25519".to_string(),
            passphrase: None,
        },
        tags: vec!["prod".to_string()],
        pinned: false,
    };
    assert_eq!(server.name, "myserver");
    assert_eq!(server.port, 22);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_server_creation`
Expected: FAIL with "unresolved import"

- [ ] **Step 3: Create config/models.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub servers: Vec<Server>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub auth: Auth,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Auth {
    Key {
        path: String,
        passphrase: Option<String>,
    },
    Password {
        vault_key: String,
    },
}
```

- [ ] **Step 4: Create config/mod.rs**

```rust
pub mod models;

pub use models::*;
```

- [ ] **Step 5: Update src/main.rs to expose config module**

```rust
pub mod config;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("LazySSH v0.1.0");
    Ok(())
}
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo test test_server_creation`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add src/config/ tests/config_test.rs
git commit -m "feat: add config models with Server and Auth structs"
```

---

### Task 3: Config File Operations

**Covers:** [S4]

**Files:**
- Create: `src/config/file.rs`
- Modify: `src/config/mod.rs`
- Test: `tests/config_test.rs`

**Interfaces:**
- Consumes: `Server`, `AppConfig` from Task 2
- Produces: `load_config()`, `save_config()`, `get_config_path()` functions

- [ ] **Step 1: Write failing tests for config file ops**

```rust
// Add to tests/config_test.rs
use lazyssh::config::file::{get_config_path, load_config, save_config};
use lazyssh::config::models::{AppConfig, Auth, Server};
use std::fs;

#[test]
fn test_config_path() {
    let path = get_config_path();
    assert!(path.to_string_lossy().contains("lazyssh"));
}

#[test]
fn test_save_and_load_config() {
    let test_dir = std::env::temp_dir().join("lazyssh_test");
    fs::create_dir_all(&test_dir).unwrap();
    
    let config = AppConfig {
        servers: vec![Server {
            name: "test".to_string(),
            host: "127.0.0.1".to_string(),
            port: 22,
            user: "user".to_string(),
            auth: Auth::Key {
                path: "~/.ssh/id_rsa".to_string(),
                passphrase: None,
            },
            tags: vec![],
            pinned: false,
        }],
    };
    
    let path = test_dir.join("servers.toml");
    save_config(&config, &path).unwrap();
    let loaded = load_config(&path).unwrap();
    
    assert_eq!(loaded.servers.len(), 1);
    assert_eq!(loaded.servers[0].name, "test");
    
    fs::remove_dir_all(&test_dir).unwrap();
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_save_and_load_config`
Expected: FAIL with "unresolved import"

- [ ] **Step 3: Create config/file.rs**

```rust
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use super::{AppConfig, Server};

pub fn get_config_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lazyssh");
    fs::create_dir_all(&config_dir).ok();
    config_dir.join("servers.toml")
}

pub fn load_config(path: &std::path::Path) -> Result<AppConfig> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config from {}", path.display()))?;
    let config: AppConfig = toml::from_str(&content)
        .with_context(|| "Failed to parse config TOML")?;
    Ok(config)
}

pub fn save_config(config: &AppConfig, path: &std::path::Path) -> Result<()> {
    let content = toml::to_string_pretty(config)
        .with_context(|| "Failed to serialize config")?;
    
    // Backup before write
    if path.exists() {
        let backup = path.with_extension("toml.backup");
        fs::copy(path, backup).ok();
    }
    
    fs::write(path, content)
        .with_context(|| format!("Failed to write config to {}", path.display()))?;
    Ok(())
}

pub fn load_or_default() -> AppConfig {
    let path = get_config_path();
    if path.exists() {
        load_config(&path).unwrap_or_else(|_| AppConfig { servers: vec![] })
    } else {
        AppConfig { servers: vec![] }
    }
}

pub fn add_server(server: Server) -> Result<()> {
    let path = get_config_path();
    let mut config = load_or_default();
    config.servers.push(server);
    save_config(&config, &path)?;
    Ok(())
}

pub fn remove_server(name: &str) -> Result<()> {
    let path = get_config_path();
    let mut config = load_or_default();
    config.servers.retain(|s| s.name != name);
    save_config(&config, &path)?;
    Ok(())
}

pub fn update_server(name: &str, updated: Server) -> Result<()> {
    let path = get_config_path();
    let mut config = load_or_default();
    if let Some(server) = config.servers.iter_mut().find(|s| s.name == name) {
        *server = updated;
        save_config(&config, &path)?;
    }
    Ok(())
}
```

- [ ] **Step 4: Update config/mod.rs**

```rust
pub mod file;
pub mod models;

pub use file::*;
pub use models::*;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test test_save_and_load_config`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/config/
git commit -m "feat: add config file operations with TOML persistence"
```

---

### Task 4: Vault Module

**Covers:** [S7]

**Files:**
- Create: `src/vault/mod.rs`
- Create: `src/vault/crypto.rs`
- Modify: `src/main.rs`
- Test: `tests/vault_test.rs`

**Interfaces:**
- Produces: `encrypt_password()`, `decrypt_password()`, `derive_key()` functions

- [ ] **Step 1: Write failing tests for vault**

```rust
// tests/vault_test.rs
use lazyssh::vault::crypto::{encrypt_password, decrypt_password, derive_key};

#[test]
fn test_encrypt_decrypt_roundtrip() {
    let master_password = "test_master_pass";
    let salt = [0u8; 16];
    let key = derive_key(master_password, &salt);
    
    let password = "my_secret_password";
    let encrypted = encrypt_password(password, &key).unwrap();
    let decrypted = decrypt_password(&encrypted, &key).unwrap();
    
    assert_eq!(decrypted, password);
}

#[test]
fn test_different_keys_fail() {
    let key1 = derive_key("password1", &[0u8; 16]);
    let key2 = derive_key("password2", &[0u8; 16]);
    
    let encrypted = encrypt_password("secret", &key1).unwrap();
    let result = decrypt_password(&encrypted, &key2);
    
    assert!(result.is_err());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_encrypt_decrypt_roundtrip`
Expected: FAIL with "unresolved import"

- [ ] **Step 3: Create vault/crypto.rs**

```rust
use anyhow::{Context, Result};
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundAES, NONCE_LEN};
use ring::rand::{SystemRandom, SecureRandom};
use ring::pbkdf2;
use std::num::NonZeroU32;

const ITERATIONS: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(100_000) };

pub fn derive_key(master_password: &str, salt: &[u8; 16]) -> [u8; 32] {
    let mut key = [0u8; 32];
    pbkdf2::derive(
        pbkdf2::HMAC_SHA256,
        ITERATIONS,
        salt,
        master_password.as_bytes(),
        &mut key,
    );
    key
}

pub fn encrypt_password(password: &str, key: &[u8; 32]) -> Result<Vec<u8>> {
    let rng = SystemRandom::new();
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rng.fill(&mut nonce_bytes)
        .context("Failed to generate nonce")?;
    
    let unbound = UnboundAES::new(&key.into())
        .context("Failed to create AES key")?;
    let less_safe = LessSafeKey::new(unbound);
    
    let nonce = Nonque::assume_unique_for_key(nonce_bytes);
    let mut in_out = password.as_bytes().to_vec();
    
    less_safe.seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)
        .context("Failed to encrypt")?;
    
    // Prepend nonce to ciphertext
    let mut result = nonce_bytes.to_vec();
    result.extend(in_out);
    Ok(result)
}

pub fn decrypt_password(encrypted: &[u8], key: &[u8; 32]) -> Result<String> {
    if encrypted.len() < NONCE_LEN {
        return Err(anyhow::anyhow!("Invalid encrypted data"));
    }
    
    let (nonce_bytes, ciphertext) = encrypted.split_at(NONCE_LEN);
    let nonce = Nonce::assume_unique_for_key(nonce_bytes.try_into().unwrap());
    
    let unbound = UnboundAES::new(&key.into())
        .context("Failed to create AES key")?;
    let less_safe = LessSafeKey::new(unbound);
    
    let mut in_out = ciphertext.to_vec();
    let plaintext = less_safe.open_in_place(nonce, Aad::empty(), &mut in_out)
        .context("Failed to decrypt (wrong key?)")?;
    
    String::from_utf8(plaintext.to_vec())
        .context("Invalid UTF-8 in decrypted password")
}
```

- [ ] **Step 4: Create vault/mod.rs**

```rust
pub mod crypto;

pub use crypto::*;
```

- [ ] **Step 5: Update main.rs to include vault module**

```rust
pub mod config;
pub mod vault;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("LazySSH v0.1.0");
    Ok(())
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add src/vault/ tests/vault_test.rs
git commit -m "feat: add vault module with AES-256-GCM encryption"
```

---

### Task 5: SSH Connection Module

**Covers:** [S5]

**Files:**
- Create: `src/ssh/mod.rs`
- Create: `src/ssh/connection.rs`
- Create: `src/ssh/auth.rs`
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `Server`, `Auth` from Task 2
- Produces: `connect_ssh()`, `SshSession` struct

- [ ] **Step 1: Create ssh/auth.rs**

```rust
use anyhow::{Context, Result};
use russh_keys::key::KeyPair;
use russh_keys::load_secret_key;
use std::path::Path;

use crate::config::models::Auth;

pub async fn load_key(auth: &Auth) -> Result<KeyPair> {
    match auth {
        Auth::Key { path, passphrase } => {
            let key_path = Path::new(path);
            let expanded = shellexpand::tilde(&key_path.to_string_lossy());
            let pass_bytes = passphrase.as_ref().map(|p| p.as_bytes());
            load_secret_key(expanded.as_ref(), pass_bytes)
                .await
                .context("Failed to load SSH key")
        }
        Auth::Password { .. } => Err(anyhow::anyhow!("Use password auth flow")),
    }
}
```

- [ ] **Step 2: Create ssh/connection.rs**

```rust
use anyhow::{Context, Result};
use russh::*;
use russh_keys::*;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::config::models::{Auth, Server};

struct SshClient {
    auth: Auth,
}

impl Handler for SshClient {
    type Error = russh::Error;
    
    fn check_server_key(&mut self, _key: &key::PublicKey) -> Result<bool, Self::Error> {
        // In production, verify against known_hosts
        Ok(true)
    }
}

pub struct SshSession {
    session: Arc<Mutex<Session>>,
}

impl SshSession {
    pub async fn connect(server: &Server, auth: &Auth) -> Result<Self> {
        let mut session = Session::connect()?;
        
        match auth {
            Auth::Key { path, passphrase } => {
                let key_pair = super::auth::load_key(auth).await?;
                session.authenticate_publickey(&server.user, key_pair).await?;
            }
            Auth::Password { vault_key } => {
                // TODO: decrypt from vault and authenticate
                session.authenticate_password(&server.user, "placeholder").await?;
            }
        }
        
        Ok(Self {
            session: Arc::new(Mutex::new(session)),
        })
    }
    
    pub async fn execute(&self, command: &str) -> Result<String> {
        let session = self.session.lock().await;
        let mut channel = session.channel_open_session().await?;
        channel.exec(true, command).await?;
        
        let mut output = String::new();
        while let Some(msg) = channel.wait().await {
            match msg {
                ChannelMsg::Data { data } => {
                    output.push_str(&String::from_utf8_lossy(&data));
                }
                _ => break,
            }
        }
        
        Ok(output)
    }
    
    pub async fn shell(&self) -> Result<()> {
        let session = self.session.lock().await;
        let mut channel = session.channel_open_session().await?;
        channel.request_shell(true).await?;
        
        // Interactive shell handling would go here
        // For now, this is a placeholder
        
        Ok(())
    }
}
```

- [ ] **Step 3: Create ssh/mod.rs**

```rust
pub mod auth;
pub mod connection;

pub use connection::SshSession;
```

- [ ] **Step 4: Update main.rs**

```rust
pub mod config;
pub mod ssh;
pub mod vault;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("LazySSH v0.1.0");
    Ok(())
}
```

- [ ] **Step 5: Add shellexpand dependency**

Add to Cargo.toml:
```toml
shellexpand = "3"
```

- [ ] **Step 6: Verify compilation**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 7: Commit**

```bash
git add src/ssh/
git commit -m "feat: add SSH connection module with russh"
```

---

### Task 6: SFTP Module

**Covers:** [S5]

**Files:**
- Create: `src/sftp/mod.rs`
- Create: `src/sftp/local.rs`
- Create: `src/sftp/remote.rs`
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `SshSession` from Task 5
- Produces: `LocalFs`, `RemoteFs` structs with listing operations

- [ ] **Step 1: Create sftp/local.rs**

```rust
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct FileInfo {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

pub struct LocalFs {
    current_dir: PathBuf,
}

impl LocalFs {
    pub fn new() -> Self {
        Self {
            current_dir: dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
        }
    }
    
    pub fn list(&self) -> Result<Vec<FileInfo>> {
        let mut entries = Vec::new();
        
        for entry in fs::read_dir(&self.current_dir)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let name = entry.file_name().to_string_lossy().to_string();
            
            entries.push(FileInfo {
                name,
                is_dir: metadata.is_dir(),
                size: metadata.len(),
            });
        }
        
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }
    
    pub fn cd(&mut self, path: &str) -> Result<()> {
        let new_path = if path.starts_with('/') {
            PathBuf::from(path)
        } else {
            self.current_dir.join(path)
        };
        
        if new_path.is_dir() {
            self.current_dir = fs::canonicalize(new_path)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Not a directory: {}", path))
        }
    }
    
    pub fn current_dir(&self) -> &Path {
        &self.current_dir
    }
}
```

- [ ] **Step 2: Create sftp/remote.rs**

```rust
use anyhow::Result;
use crate::ssh::SshSession;

#[derive(Debug)]
pub struct FileInfo {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

pub struct RemoteFs {
    current_dir: String,
}

impl RemoteFs {
    pub fn new() -> Self {
        Self {
            current_dir: "/".to_string(),
        }
    }
    
    pub async fn list(&self, session: &SshSession) -> Result<Vec<FileInfo>> {
        let output = session.execute(&format!("ls -la {}", self.current_dir)).await?;
        // Parse ls output - simplified
        let entries = output.lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 9 {
                    let is_dir = parts[0].starts_with('d');
                    let name = parts[8..].join(" ");
                    let size: u64 = parts[4].parse().unwrap_or(0);
                    Some(FileInfo { name, is_dir, size })
                } else {
                    None
                }
            })
            .collect();
        Ok(entries)
    }
    
    pub async fn cd(&mut self, session: &SshSession, path: &str) -> Result<()> {
        let output = session.execute(&format!("cd {} && pwd", path)).await?;
        self.current_dir = output.trim().to_string();
        Ok(())
    }
    
    pub fn current_dir(&self) -> &str {
        &self.current_dir
    }
}
```

- [ ] **Step 3: Create sftp/mod.rs**

```rust
pub mod local;
pub mod remote;

pub use local::LocalFs;
pub use remote::RemoteFs;
```

- [ ] **Step 4: Update main.rs**

```rust
pub mod config;
pub mod sftp;
pub mod ssh;
pub mod vault;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("LazySSH v0.1.0");
    Ok(())
}
```

- [ ] **Step 5: Verify compilation**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 6: Commit**

```bash
git add src/sftp/
git commit -m "feat: add SFTP module with local and remote filesystem ops"
```

---

### Task 7: TUI App State

**Covers:** [S6]

**Files:**
- Create: `src/tui/mod.rs`
- Create: `src/tui/app.rs`
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `AppConfig` from Task 3
- Produces: `App` struct with state management

- [ ] **Step 1: Create tui/app.rs**

```rust
use crate::config::models::Server;

#[derive(Debug, PartialEq)]
pub enum InputMode {
    Normal,
    Search,
    Insert,
    Edit,
}

#[derive(Debug, PartialEq)]
pub enum CurrentView {
    ServerList,
    SshTerminal,
    SftpBrowser,
}

pub struct App {
    pub servers: Vec<Server>,
    pub filtered_indices: Vec<usize>,
    pub selected: usize,
    pub current_view: CurrentView,
    pub input_mode: InputMode,
    pub input: String,
    pub should_quit: bool,
}

impl App {
    pub fn new(servers: Vec<Server>) -> Self {
        let filtered_indices = (0..servers.len()).collect();
        Self {
            servers,
            filtered_indices,
            selected: 0,
            current_view: CurrentView::ServerList,
            input_mode: InputMode::Normal,
            input: String::new(),
            should_quit: false,
        }
    }
    
    pub fn next(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.filtered_indices.len();
    }
    
    pub fn previous(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }
        self.selected = if self.selected == 0 {
            self.filtered_indices.len() - 1
        } else {
            self.selected - 1
        };
    }
    
    pub fn selected_server(&self) -> Option<&Server> {
        self.filtered_indices.get(self.selected)
            .and_then(|&i| self.servers.get(i))
    }
    
    pub fn filter(&mut self, query: &str) {
        if query.is_empty() {
            self.filtered_indices = (0..self.servers.len()).collect();
        } else {
            let query_lower = query.to_lowercase();
            self.filtered_indices = self.servers.iter()
                .enumerate()
                .filter(|(_, s)| {
                    s.name.to_lowercase().contains(&query_lower)
                        || s.host.to_lowercase().contains(&query_lower)
                        || s.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
                })
                .map(|(i, _)| i)
                .collect();
        }
        self.selected = 0;
    }
}
```

- [ ] **Step 2: Create tui/mod.rs**

```rust
pub mod app;

pub use app::App;
```

- [ ] **Step 3: Update main.rs**

```rust
pub mod config;
pub mod sftp;
pub mod ssh;
pub mod tui;
pub mod vault;

use anyhow::Result;
use tui::App;

#[tokio::main]
async fn main() -> Result<()> {
    let config = config::load_or_default();
    let mut app = App::new(config.servers);
    
    println!("LazySSH v0.1.0 - {} servers loaded", app.servers.len());
    Ok(())
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 5: Commit**

```bash
git add src/tui/
git commit -m "feat: add TUI app state management"
```

---

### Task 8: TUI Server List View

**Covers:** [S6]

**Files:**
- Create: `src/tui/server_list.rs`
- Modify: `src/tui/mod.rs`
- Modify: `src/tui/app.rs`

**Interfaces:**
- Consumes: `App` from Task 7
- Produces: `render_server_list()` function

- [ ] **Step 1: Create tui/server_list.rs**

```rust
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::config::models::Server;

use super::app::App;

pub fn render_server_list(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());
    
    // Search bar
    let search_input = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title("Search [/]"));
    f.render_widget(search_input, chunks[0]);
    
    // Server list
    let items: Vec<ListItem> = app.filtered_indices.iter()
        .filter_map(|&i| app.servers.get(i))
        .map(|server| {
            let style = if server.pinned {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            
            let line = Line::from(vec![
                Span::styled(
                    if server.pinned { "● " } else { "○ " },
                    style,
                ),
                Span::styled(&server.name, Style::default().fg(Color::Cyan)),
                Span::raw(" - "),
                Span::raw(&server.host),
                Span::raw(":"),
                Span::raw(server.port.to_string()),
                Span::raw(" ["),
                Span::raw(server.tags.join(", ")),
                Span::raw("]"),
            ]);
            
            ListItem::new(line)
        })
        .collect();
    
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Servers"))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");
    
    let mut state = ListState::default();
    state.select(Some(app.selected));
    f.render_stateful_widget(list, chunks[1], &mut state);
    
    // Help bar
    let help_text = Line::from(vec![
        Span::styled("Enter: Connect ", Style::default().fg(Color::Green)),
        Span::styled("s: SFTP ", Style::default().fg(Color::Green)),
        Span::styled("a: Add ", Style::default().fg(Color::Green)),
        Span::styled("e: Edit ", Style::default().fg(Color::Green)),
        Span::styled("d: Delete ", Style::default().fg(Color::Green)),
        Span::styled("p: Pin ", Style::default().fg(Color::Green)),
        Span::styled("q: Quit ", Style::default().fg(Color::Red)),
    ]);
    
    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[2]);
}
```

- [ ] **Step 2: Update tui/mod.rs**

```rust
pub mod app;
pub mod server_list;

pub use app::App;
pub use server_list::render_server_list;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add src/tui/server_list.rs
git commit -m "feat: add TUI server list view with search and keybindings"
```

---

### Task 9: TUI Main Loop

**Covers:** [S6]

**Files:**
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `App`, `render_server_list` from Tasks 7-8
- Produces: Interactive TUI loop

- [ ] **Step 1: Update main.rs with TUI loop**

```rust
pub mod config;
pub mod sftp;
pub mod ssh;
pub mod tui;
pub mod vault;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;

use tui::{render_server_list, App};

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Load config
    let config = config::load_or_default();
    let mut app = App::new(config.servers);
    
    // Main loop
    loop {
        terminal.draw(|f| {
            match app.current_view {
                tui::app::CurrentView::ServerList => render_server_list(f, &app),
                _ => {}
            }
        })?;
        
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.input_mode {
                        tui::app::InputMode::Normal => match key.code {
                            KeyCode::Char('q') => app.should_quit = true,
                            KeyCode::Char('j') | KeyCode::Down => app.next(),
                            KeyCode::Char('k') | KeyCode::Up => app.previous(),
                            KeyCode::Char('/') => {
                                app.input_mode = tui::app::InputMode::Search;
                                app.input.clear();
                            }
                            KeyCode::Enter => {
                                if let Some(server) = app.selected_server() {
                                    println!("Connecting to {}...", server.name);
                                    app.should_quit = true;
                                }
                            }
                            _ => {}
                        },
                        tui::app::InputMode::Search => match key.code {
                            KeyCode::Enter => {
                                app.input_mode = tui::app::InputMode::Normal;
                            }
                            KeyCode::Esc => {
                                app.input_mode = tui::app::InputMode::Normal;
                                app.input.clear();
                                app.filter("");
                            }
                            KeyCode::Char(c) => {
                                app.input.push(c);
                                app.filter(&app.input.clone());
                            }
                            KeyCode::Backspace => {
                                app.input.pop();
                                app.filter(&app.input.clone());
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }
        }
        
        if app.should_quit {
            break;
        }
    }
    
    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    
    Ok(())
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 3: Test the TUI**

Run: `cargo run`
Expected: TUI displays server list (empty if no servers configured)

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: add TUI main loop with keyboard navigation"
```

---

### Task 10: Integration Tests

**Covers:** [S4, S7]

**Files:**
- Create: `tests/integration_test.rs`

**Interfaces:**
- Consumes: All modules from Tasks 2-9
- Produces: End-to-end config + vault tests

- [ ] **Step 1: Create integration test**

```rust
// tests/integration_test.rs
use lazyssh::config::{self, AppConfig, Auth, Server};
use lazyssh::vault::crypto::{derive_key, encrypt_password, decrypt_password};
use std::fs;

#[test]
fn test_full_config_workflow() {
    let test_dir = std::env::temp_dir().join("lazyssh_integration_test");
    fs::create_dir_all(&test_dir).unwrap();
    
    // Create config
    let config = AppConfig {
        servers: vec![
            Server {
                name: "server1".to_string(),
                host: "192.168.1.1".to_string(),
                port: 22,
                user: "user1".to_string(),
                auth: Auth::Key {
                    path: "~/.ssh/id_rsa".to_string(),
                    passphrase: None,
                },
                tags: vec!["prod".to_string()],
                pinned: true,
            },
            Server {
                name: "server2".to_string(),
                host: "192.168.1.2".to_string(),
                port: 22,
                user: "user2".to_string(),
                auth: Auth::Password {
                    vault_key: "server2_pass".to_string(),
                },
                tags: vec!["dev".to_string()],
                pinned: false,
            },
        ],
    };
    
    let config_path = test_dir.join("servers.toml");
    config::save_config(&config, &config_path).unwrap();
    
    // Load and verify
    let loaded = config::load_config(&config_path).unwrap();
    assert_eq!(loaded.servers.len(), 2);
    assert_eq!(loaded.servers[0].name, "server1");
    assert_eq!(loaded.servers[1].name, "server2");
    
    // Test vault with password auth
    let master_pass = "integration_test_master";
    let salt = [1u8; 16];
    let key = derive_key(master_pass, &salt);
    
    if let Auth::Password { vault_key } = &loaded.servers[1].auth {
        let encrypted = encrypt_password(vault_key, &key).unwrap();
        let decrypted = decrypt_password(&encrypted, &key).unwrap();
        assert_eq!(decrypted, "server2_pass");
    }
    
    fs::remove_dir_all(&test_dir).unwrap();
}
```

- [ ] **Step 2: Run integration test**

Run: `cargo test test_full_config_workflow`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add tests/integration_test.rs
git commit -m "test: add integration test for config and vault"
```

---

### Task 11: SFTP Dual-Pane View

**Covers:** [S5]

**Files:**
- Create: `src/tui/sftp_browser.rs`
- Modify: `src/tui/mod.rs`
- Modify: `src/tui/app.rs`

**Interfaces:**
- Consumes: `LocalFs`, `RemoteFs` from Task 6
- Produces: `render_sftp_browser()` function

- [ ] **Step 1: Create tui/sftp_browser.rs**

```rust
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::sftp::{LocalFs, RemoteFs};

use super::app::App;

pub struct SftpState {
    pub local: LocalFs,
    pub remote: RemoteFs,
    pub local_selected: usize,
    pub remote_selected: usize,
    pub focus_side: Side,
}

#[derive(Debug, PartialEq)]
pub enum Side {
    Local,
    Remote,
}

impl SftpState {
    pub fn new() -> Self {
        Self {
            local: LocalFs::new(),
            remote: RemoteFs::new(),
            local_selected: 0,
            remote_selected: 0,
            focus_side: Side::Local,
        }
    }
    
    pub fn next_item(&mut self) {
        match self.focus_side {
            Side::Local => self.local_selected += 1,
            Side::Remote => self.remote_selected += 1,
        }
    }
    
    pub fn previous_item(&mut self) {
        match self.focus_side {
            Side::Local => {
                if self.local_selected > 0 {
                    self.local_selected -= 1;
                }
            }
            Side::Remote => {
                if self.remote_selected > 0 {
                    self.remote_selected -= 1;
                }
            }
        }
    }
    
    pub fn toggle_focus(&mut self) {
        self.focus_side = match self.focus_side {
            Side::Local => Side::Remote,
            Side::Remote => Side::Local,
        };
    }
}

pub fn render_sftp_browser(f: &mut Frame, state: &SftpState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(f.area());
    
    // Local pane
    render_local_pane(f, state, chunks[0]);
    
    // Remote pane
    render_remote_pane(f, state, chunks[1]);
}

fn render_local_pane(f: &mut Frame, state: &SftpState, area: ratatui::layout::Rect) {
    let files = state.local.list().unwrap_or_default();
    
    let items: Vec<ListItem> = files.iter().map(|file| {
        let style = if file.is_dir {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };
        
        ListItem::new(Line::from(vec![
            Span::styled(
                if file.is_dir { "📁 " } else { "📄 " },
                style,
            ),
            Span::styled(&file.name, style),
        ]))
    }).collect();
    
    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(format!("Local: {}", state.local.current_dir().display())))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));
    
    let mut list_state = ListState::default();
    list_state.select(Some(state.local_selected));
    f.render_stateful_widget(list, area, &mut list_state);
}

fn render_remote_pane(f: &mut Frame, state: &SftpState, area: ratatui::layout::Rect) {
    let placeholder = Paragraph::new("Remote SFTP\n(Connect to server first)")
        .block(Block::default()
            .borders(Borders::ALL)
            .title(format!("Remote: {}", state.remote.current_dir())));
    f.render_widget(placeholder, area);
}
```

- [ ] **Step 2: Update tui/mod.rs**

```rust
pub mod app;
pub mod sftp_browser;
pub mod server_list;

pub use app::App;
pub use sftp_browser::render_sftp_browser;
pub use server_list::render_server_list;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add src/tui/sftp_browser.rs
git commit -m "feat: add SFTP dual-pane browser view"
```

---

### Task 12: Final Integration

**Covers:** [S5, S6]

**Files:**
- Modify: `src/main.rs`
- Modify: `src/tui/app.rs`

**Interfaces:**
- Consumes: All modules from Tasks 1-11
- Produces: Complete working TUI

- [ ] **Step 1: Update app.rs with SFTP state**

```rust
use crate::config::models::Server;
use super::sftp_browser::SftpState;

#[derive(Debug, PartialEq)]
pub enum InputMode {
    Normal,
    Search,
    Insert,
    Edit,
}

#[derive(Debug, PartialEq)]
pub enum CurrentView {
    ServerList,
    SshTerminal,
    SftpBrowser,
}

pub struct App {
    pub servers: Vec<Server>,
    pub filtered_indices: Vec<usize>,
    pub selected: usize,
    pub current_view: CurrentView,
    pub input_mode: InputMode,
    pub input: String,
    pub should_quit: bool,
    pub sftp_state: Option<SftpState>,
}

impl App {
    pub fn new(servers: Vec<Server>) -> Self {
        let filtered_indices = (0..servers.len()).collect();
        Self {
            servers,
            filtered_indices,
            selected: 0,
            current_view: CurrentView::ServerList,
            input_mode: InputMode::Normal,
            input: String::new(),
            should_quit: false,
            sftp_state: None,
        }
    }
    
    // ... existing methods ...
    
    pub fn open_sftp(&mut self) {
        self.current_view = CurrentView::SftpBrowser;
        self.sftp_state = Some(SftpState::new());
    }
    
    pub fn close_sftp(&mut self) {
        self.current_view = CurrentView::ServerList;
        self.sftp_state = None;
    }
}
```

- [ ] **Step 2: Update main.rs with full TUI**

```rust
pub mod config;
pub mod sftp;
pub mod ssh;
pub mod tui;
pub mod vault;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;

use tui::{render_server_list, render_sftp_browser, App};

#[tokio::main]
async fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    let config = config::load_or_default();
    let mut app = App::new(config.servers);
    
    loop {
        terminal.draw(|f| {
            match app.current_view {
                tui::app::CurrentView::ServerList => render_server_list(f, &app),
                tui::app::CurrentView::SftpBrowser => {
                    if let Some(sftp) = &app.sftp_state {
                        render_sftp_browser(f, sftp);
                    }
                }
                _ => {}
            }
        })?;
        
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.current_view {
                        tui::app::CurrentView::ServerList => {
                            match app.input_mode {
                                tui::app::InputMode::Normal => match key.code {
                                    KeyCode::Char('q') => app.should_quit = true,
                                    KeyCode::Char('j') | KeyCode::Down => app.next(),
                                    KeyCode::Char('k') | KeyCode::Up => app.previous(),
                                    KeyCode::Char('/') => {
                                        app.input_mode = tui::app::InputMode::Search;
                                        app.input.clear();
                                    }
                                    KeyCode::Char('s') => app.open_sftp(),
                                    KeyCode::Enter => {
                                        if let Some(server) = app.selected_server() {
                                            println!("Connecting to {}...", server.name);
                                            app.should_quit = true;
                                        }
                                    }
                                    _ => {}
                                },
                                tui::app::InputMode::Search => match key.code {
                                    KeyCode::Enter => app.input_mode = tui::app::InputMode::Normal,
                                    KeyCode::Esc => {
                                        app.input_mode = tui::app::InputMode::Normal;
                                        app.input.clear();
                                        app.filter("");
                                    }
                                    KeyCode::Char(c) => {
                                        app.input.push(c);
                                        app.filter(&app.input.clone());
                                    }
                                    KeyCode::Backspace => {
                                        app.input.pop();
                                        app.filter(&app.input.clone());
                                    }
                                    _ => {}
                                },
                                _ => {}
                            }
                        }
                        tui::app::CurrentView::SftpBrowser => {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => app.close_sftp(),
                                KeyCode::Tab => {
                                    if let Some(sftp) = &mut app.sftp_state {
                                        sftp.toggle_focus();
                                    }
                                }
                                KeyCode::Char('j') | KeyCode::Down => {
                                    if let Some(sftp) = &mut app.sftp_state {
                                        sftp.next_item();
                                    }
                                }
                                KeyCode::Char('k') | KeyCode::Up => {
                                    if let Some(sftp) = &mut app.sftp_state {
                                        sftp.previous_item();
                                    }
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        
        if app.should_quit {
            break;
        }
    }
    
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    
    Ok(())
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 4: Run the application**

Run: `cargo run`
Expected: TUI launches with server list, can navigate with j/k, open SFTP with s

- [ ] **Step 5: Final commit**

```bash
git add .
git commit -m "feat: complete TUI with server list and SFTP browser views"
```

---

## Summary

The plan covers 12 tasks implementing:
- Project scaffolding with dependencies
- Config models and TOML persistence
- Vault module with AES-256-GCM encryption
- SSH connection module with russh
- SFTP local/remote filesystem operations
- TUI with Ratatui (server list + SFTP dual-pane)
- Integration tests

Each task follows TDD with failing test → implementation → verification → commit.
