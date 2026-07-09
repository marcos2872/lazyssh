# LazySSH - Plano Detalhado de Correção SSH/SFTP

> **For agentic workers:** REQUIRED SUB-SKILL: Use compose:subagent (recommended) or compose:execute to implement this plan task-by-task.

**Goal:** Implementar SSH/SFTP nativo usando russh, com PTY, sessão persistente, e upload/download integrado.

**Reference:** https://github.com/marcos2872/SSH_Orchestrator/blob/main/src-tauri/src/services/ssh.rs

---

## Pré-requisitos

### Dependências (Cargo.toml)

```toml
[dependencies]
# ... existentes ...
russh = "0.50"
russh-keys = "0.50"
russh-sftp = "0.16"
uuid = { version = "1", features = ["v4"] }
```

---

## Task 30: SSH Service com russh

### Arquivos
- Criar: `src/ssh/service.rs`
- Modificar: `src/ssh/mod.rs`

### Passo 1: Criar SSH Client Handler

```rust
// src/ssh/service.rs

use std::sync::Arc;
use anyhow::Result;
use russh::*;
use russh_keys::*;
use tokio::sync::mpsc;

use crate::config::models::{Auth, Server};

struct SshClient;

impl client::Handler for SshClient {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh_keys::PublicKey,
    ) -> Result<bool, Self::Error> {
        // TODO: Implementar known_hosts verification
        Ok(true)
    }
}
```

### Passo 2: Criar Session Manager

```rust
pub struct SshSession {
    pub id: String,
    pub server_name: String,
    pub handle: Arc<tokio::sync::Mutex<client::Handle<SshClient>>>,
    pub msg_tx: mpsc::UnboundedSender<Vec<u8>>,
}

pub struct SshService {
    sessions: std::collections::HashMap<String, SshSession>,
}

impl SshService {
    pub fn new() -> Self {
        Self {
            sessions: std::collections::HashMap::new(),
        }
    }
}
```

### Passo 3: Implementar connect()

```rust
pub async fn connect(&mut self, server: &Server, auth: &Auth) -> Result<String> {
    let config = Arc::new(client::Config::default());
    let handler = SshClient;

    let mut handle = client::connect(
        config,
        (server.host.as_str(), server.port),
        handler,
    ).await
    .map_err(|e| anyhow::anyhow!("Failed to connect: {}", e))?;

    // Autenticar
    match auth {
        Auth::Key { path, passphrase } => {
            let key = load_secret_key(path, passphrase.as_deref()).await
                .map_err(|e| anyhow::anyhow!("Failed to load key: {}", e))?;
            handle.authenticate_publickey(&server.user, key).await
                .map_err(|e| anyhow::anyhow!("Auth failed: {}", e))?;
        }
        Auth::Password { vault_key } => {
            handle.authenticate_password(&server.user, vault_key).await
                .map_err(|e| anyhow::anyhow!("Auth failed: {}", e))?;
        }
    }

    // Abrir canal de sessão
    let mut channel = handle.channel_open_session().await
        .map_err(|e| anyhow::anyhow!("Failed to open channel: {}", e))?;

    // Solicitar PTY para terminal interativo
    channel.request_pty(
        true,
        "xterm-256color",
        80, // cols
        24, // rows
        0,  // pixel width
        0,  // pixel height
        &[], // modes
    ).await
    .map_err(|e| anyhow::anyhow!("Failed to request PTY: {}", e))?;

    // Solicitar shell interativo
    channel.request_shell(false).await
        .map_err(|e| anyhow::anyhow!("Failed to request shell: {}", e))?;

    // Criar canal para enviar dados ao shell
    let (msg_tx, mut msg_rx) = mpsc::unbounded_channel::<Vec<u8>>();

    // Background task para I/O
    let session_id = uuid::Uuid::new_v4().to_string();
    let sid = session_id.clone();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                msg = channel.wait() => {
                    match msg {
                        Some(ChannelMsg::Data { data }) => {
                            // TODO: Enviar output para o TUI
                            eprint!("{}", String::from_utf8_lossy(&data));
                        }
                        Some(ChannelMsg::ExtendedData { data, .. }) => {
                            eprint!("{}", String::from_utf8_lossy(&data));
                        }
                        Some(ChannelMsg::ExitStatus { exit_status }) => {
                            eprintln!("\r\n[Process exited with code {}]\r\n", exit_status);
                            break;
                        }
                        Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) => {
                            break;
                        }
                        _ => {}
                    }
                }
                Some(data) = msg_rx.recv() => {
                    if channel.data(&data).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Salvar sessão
    self.sessions.insert(session_id.clone(), SshSession {
        id: session_id.clone(),
        server_name: server.name.clone(),
        handle: Arc::new(tokio::sync::Mutex::new(handle)),
        msg_tx,
    });

    Ok(session_id)
}
```

### Passo 4: Implementar write()

```rust
pub async fn write(&self, session_id: &str, data: &[u8]) -> Result<()> {
    let session = self.sessions.get(session_id)
        .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
    session.msg_tx.send(data.to_vec())
        .map_err(|e| anyhow::anyhow!("Failed to send data: {}", e))?;
    Ok(())
}
```

### Passo 5: Implementar disconnect()

```rust
pub async fn disconnect(&mut self, session_id: &str) -> Result<()> {
    if let Some((_, session)) = self.sessions.remove(session_id) {
        drop(session.msg_tx);
        let handle = session.handle.lock().await;
        handle.disconnect(Disconnect::ByApplication, "User disconnected", "en").await.ok();
    }
    Ok(())
}
```

### Passo 6: Atualizar mod.rs

```rust
// src/ssh/mod.rs
pub mod auth;
pub mod connection;
pub mod exec;
pub mod service;

pub use connection::SshSession;
pub use exec::execute_ssh_command;
pub use service::SshService;
```

### Passo 7: Compilar e testar

```bash
cargo build
```

### Passo 8: Commit

```bash
git add src/ssh/
git commit -m "feat: add SSH service with russh PTY support"
```

---

## Task 31: SFTP Service com russh-sftp

### Arquivos
- Criar: `src/sftp/service.rs`
- Modificar: `src/sftp/mod.rs`

### Passo 1: Criar SFTP Service

```rust
// src/sftp/service.rs

use anyhow::Result;
use russh::client;
use russh_sftp::client::SftpSession;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::config::models::{Auth, Server};
use super::FileInfo;

struct SftpClient;

impl client::Handler for SftpClient {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh_keys::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

pub struct SftpService {
    session: Option<SftpSession>,
}

impl SftpService {
    pub fn new() -> Self {
        Self { session: None }
    }

    pub async fn connect(&mut self, server: &Server) -> Result<()> {
        let config = Arc::new(client::Config::default());
        let handler = SftpClient;

        let mut handle = client::connect(
            config,
            (server.host.as_str(), server.port),
            handler,
        ).await
        .map_err(|e| anyhow::anyhow!("Failed to connect: {}", e))?;

        // Autenticar
        match &server.auth {
            Auth::Key { path, passphrase } => {
                let key = russh_keys::load_secret_key(path, passphrase.as_deref()).await
                    .map_err(|e| anyhow::anyhow!("Failed to load key: {}", e))?;
                handle.authenticate_publickey(&server.user, key).await
                    .map_err(|e| anyhow::anyhow!("Auth failed: {}", e))?;
            }
            Auth::Password { vault_key } => {
                handle.authenticate_password(&server.user, vault_key).await
                    .map_err(|e| anyhow::anyhow!("Auth failed: {}", e))?;
            }
        }

        // Abrir canal SFTP
        let channel = handle.channel_open_session().await
            .map_err(|e| anyhow::anyhow!("Failed to open channel: {}", e))?;

        channel.request_subsystem(true, "sftp").await
            .map_err(|e| anyhow::anyhow!("Failed to request SFTP: {}", e))?;

        let session = SftpSession::new(channel.into_stream()).await
            .map_err(|e| anyhow::anyhow!("Failed to create SFTP session: {}", e))?;

        self.session = Some(session);
        Ok(())
    }

    pub async fn list(&self, path: &str) -> Result<Vec<FileInfo>> {
        let session = self.session.as_ref()
            .ok_or_else(|| anyhow::anyhow!("SFTP not connected"))?;

        let entries = session.read_dir(path).await
            .map_err(|e| anyhow::anyhow!("Failed to list dir: {}", e))?;

        let mut files = Vec::new();
        for entry in entries {
            let metadata = entry.metadata();
            files.push(FileInfo {
                name: entry.file_name(),
                is_dir: entry.file_type().is_dir(),
                size: metadata.size.unwrap_or(0),
            });
        }

        Ok(files)
    }

    pub async fn upload(&self, local_path: &str, remote_path: &str) -> Result<()> {
        let session = self.session.as_ref()
            .ok_or_else(|| anyhow::anyhow!("SFTP not connected"))?;

        let mut local_file = tokio::fs::File::open(local_path).await
            .map_err(|e| anyhow::anyhow!("Failed to open local file: {}", e))?;

        let mut remote_file = session.open_with_flags(
            remote_path,
            russh_sftp::protocol::OpenFlags::WRITE
                | russh_sftp::protocol::OpenFlags::CREATE
                | russh_sftp::protocol::OpenFlags::TRUNCATE,
        ).await
        .map_err(|e| anyhow::anyhow!("Failed to open remote file: {}", e))?;

        let mut buffer = [0u8; 65536];
        loop {
            let n = local_file.read(&mut buffer).await?;
            if n == 0 { break; }
            remote_file.write_all(&buffer[..n]).await
                .map_err(|e| anyhow::anyhow!("Failed to write: {}", e))?;
        }

        Ok(())
    }

    pub async fn download(&self, remote_path: &str, local_path: &str) -> Result<()> {
        let session = self.session.as_ref()
            .ok_or_else(|| anyhow::anyhow!("SFTP not connected"))?;

        let mut remote_file = session.open(remote_path).await
            .map_err(|e| anyhow::anyhow!("Failed to open remote file: {}", e))?;

        let mut local_file = tokio::fs::File::create(local_path).await
            .map_err(|e| anyhow::anyhow!("Failed to create local file: {}", e))?;

        let mut buffer = [0u8; 65536];
        loop {
            let n = remote_file.read(&mut buffer).await?;
            if n == 0 { break; }
            local_file.write_all(&buffer[..n]).await
                .map_err(|e| anyhow::anyhow!("Failed to write: {}", e))?;
        }

        Ok(())
    }

    pub async fn cd(&mut self, path: &str) -> Result<()> {
        // cd é feito mudando o pathbase
        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(session) = self.session.take() {
            session.close().await.ok();
        }
        Ok(())
    }
}
```

### Passo 2: Atualizar mod.rs

```rust
// src/sftp/mod.rs
pub mod local;
pub mod remote;
pub mod service;

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

pub use local::LocalFs;
pub use remote::RemoteFs;
pub use service::SftpService;
```

### Passo 3: Compilar

```bash
cargo build
```

### Passo 4: Commit

```bash
git add src/sftp/
git commit -m "feat: add SFTP service with russh-sftp"
```

---

## Task 32: Integrar no TUI

### Arquivos
- Modificar: `src/tui/app.rs`
- Modificar: `src/tui/ssh_terminal.rs`
- Modificar: `src/tui/sftp_browser.rs`
- Modificar: `src/main.rs`

### Passo 1: Adicionar services ao App

```rust
// src/tui/app.rs

use crate::ssh::service::SshService;
use crate::sftp::service::SftpService;

pub struct App {
    // ... campos existentes ...
    pub ssh_service: SshService,
    pub sftp_service: SftpService,
}
```

### Passo 2: Atualizar connect_ssh

```rust
pub fn connect_ssh(&mut self) {
    if let Some(server) = self.selected_server() {
        let server = server.clone();
        self.notifications.info(&format!("Conectando a {}...", server.name));

        // Conectar em background
        let ssh_service = self.ssh_service.clone();
        let server_clone = server.clone();
        let auth = server.auth.clone();

        tokio::spawn(async move {
            match ssh_service.connect(&server_clone, &auth).await {
                Ok(session_id) => {
                    // TODO: Enviar session_id de volta para o TUI
                    Ok(session_id)
                }
                Err(e) => Err(e),
            }
        });

        self.current_view = CurrentView::SshTerminal;
        // Criar estado do terminal
        self.ssh_state = Some(SshTerminalState::new(server));
    }
}
```

### Passo 3: Atualizar SFTP

```rust
pub fn open_sftp(&mut self) {
    if let Some(server) = self.selected_server() {
        let server = server.clone();
        self.current_view = CurrentView::SftpBrowser;
        self.sftp_state = Some(SftpState::new(server));
    }
}
```

### Passo 4: Atualizar SSH Terminal para receber output

```rust
// Na SshTerminalState
pub fn add_output(&mut self, line: String) {
    self.output.push(line);
    // Auto-scroll para baixo
    self.scroll_offset = 0;
}
```

### Passo 5: Atualizar SFTP para usar SftpService

```rust
// No SftpState
pub fn connect_sftp(&mut self) -> Result<(), String> {
    if let Some(server) = &self.remote.server {
        // Conectar via SftpService
        Ok(())
    } else {
        Err("No server".to_string())
    }
}
```

### Passo 6: Atualizar main.rs com tokio runtime

```rust
// src/main.rs

#[tokio::main]
async fn main() -> Result<()> {
    // ... setup ...

    let ssh_service = SshService::new();
    let sftp_service = SftpService::new();

    let mut app = App {
        // ... campos existentes ...
        ssh_service,
        sftp_service,
    };

    // ... main loop ...
}
```

### Passo 7: Compilar

```bash
cargo build
```

### Passo 8: Commit

```bash
git add src/tui/ src/main.rs
git commit -m "feat: integrate SSH/SFTP services into TUI"
```

---

## Task 33: Conectar Output SSH ao TUI

### Arquivos
- Modificar: `src/tui/ssh_terminal.rs`
- Modificar: `src/main.rs`

### Passo 1: Criar canal para output SSH

```rust
// No App
pub ssh_output_rx: Option<mpsc::UnboundedReceiver<String>>,
```

### Passo 2: Processar output no main loop

```rust
// No loop principal
if let Some(rx) = &mut app.ssh_output_rx {
    while let Ok(output) = rx.try_recv() {
        if let Some(ssh) = &mut app.ssh_state {
            ssh.add_output(output);
        }
    }
}
```

### Passo 3: Compilar

```bash
cargo build
```

### Passo 4: Commit

```bash
git commit -m "feat: connect SSH output to TUI display"
```

---

## Task 34: Upload/Download com Progresso

### Arquivos
- Modificar: `src/tui/sftp_browser.rs`
- Modificar: `src/sftp/service.rs`

### Passo 1: Adicionar callback de progresso

```rust
pub async fn upload_with_progress(
    &self,
    local_path: &str,
    remote_path: &str,
    progress_cb: impl Fn(u64, u64),
) -> Result<()> {
    let session = self.session.as_ref()
        .ok_or_else(|| anyhow::anyhow!("SFTP not connected"))?;

    let mut local_file = tokio::fs::File::open(local_path).await?;
    let total = local_file.metadata().await?.len();

    let mut remote_file = session.open_with_flags(
        remote_path,
        russh_sftp::protocol::OpenFlags::WRITE
            | russh_sftp::protocol::OpenFlags::CREATE
            | russh_sftp::protocol::OpenFlags::TRUNCATE,
    ).await?;

    let mut buffer = [0u8; 65536];
    let mut done = 0u64;

    loop {
        let n = local_file.read(&mut buffer).await?;
        if n == 0 { break; }
        remote_file.write_all(&buffer[..n]).await?;
        done += n as u64;
        progress_cb(done, total);
    }

    Ok(())
}
```

### Passo 2: Atualizar SFTP browser para mostrar progresso

```rust
// No render_sftp_browser
if let Some(progress) = &state.transfer_progress {
    // Renderizar barra de progresso
}
```

### Passo 3: Compilar

```bash
cargo build
```

### Passo 4: Commit

```bash
git commit -m "feat: add progress bar for SFTP transfers"
```

---

## Resumo das Tasks

| Task | Descrição | Dependências |
|------|-----------|--------------|
| 30 | SSH Service com russh | Nenhuma |
| 31 | SFTP Service com russh-sftp | Nenhuma |
| 32 | Integração no TUI | 30, 31 |
| 33 | Conectar Output SSH | 30, 32 |
| 34 | Upload/Download com Progresso | 31, 32 |

---

## Notas Importantes

### russh vs sshpass

| Aspecto | sshpass (atual) | russh (novo) |
|---------|-----------------|--------------|
| Conexão | Processo externo | Nativo Rust |
| Sessão | Nova por comando | Persistente |
| PTY | Não suporta | Suporta |
| SFTP | Via ls | Nativo |
| Performance | Lenta | Rápida |

### Exemplo de Uso Final

```
1. Usuário seleciona servidor
2. Pressiona Enter → SSH conecta via russh com PTY
3. Terminal mostra prompt do servidor
4. Usuário digita comandos → output em tempo real
5. Pressiona 's' → SFTP abre via russh-sftp
6. Navega arquivos locais e remotos
7. Seleciona arquivos com Space
8. Pressiona 'u' → Upload via SFTP com progresso
```
