# LazySSH - Plano de Correção da Interação SSH

> **For agentic workers:** REQUIRED SUB-SKILL: Use compose:subagent (recommended) or compose:execute to implement this plan task-by-task.

**Goal:** Corrigir a interação com o servidor SSH usando russh com PTY, similar ao SSH_Orchestrator.

**Reference:** https://github.com/marcos2872/SSH_Orchestrator/blob/main/src-tauri/src/services/ssh.rs

---

## [S1] Problemas Atuais

1. **SSH usa comandos do sistema** - `sshpass`, `scp` são externos
2. **Sem sessão persistente** - Cada comando cria nova conexão
3. **Terminal sem PTY** - Não mostra saída formatada corretamente
4. **SFTP não usa russh-sftp** - Usa `ls` via SSH em vez de SFTP real
5. **Upload/download usa scp** - Não integrado com o TUI

---

## [S2] Solução

Usar `russh` + `russh-sftp` para conexão SSH/SFTP nativa em Rust, similar ao SSH_Orchestrator.

### Dependências Necessárias

```toml
russh = "0.50"
russh-keys = "0.50"
russh-sftp = "0.16"
tokio = { version = "1", features = ["full", "io-util"] }
```

---

## [S3] Task 30: SSH Service com russh

**Files:**
- Rewrite: `src/ssh/connection.rs`
- Create: `src/ssh/service.rs`

**Steps:**

1. Criar `src/ssh/service.rs`:
```rust
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use anyhow::Result;
use dashmap::DashMap;
use russh::*;
use russh_keys::*;
use tokio::sync::mpsc;

use crate::config::models::{Auth, Server};

pub struct SshSession {
    pub handle: Arc<tokio::sync::Mutex<Handle<SshClient>>>,
    pub msg_tx: mpsc::UnboundedSender<Vec<u8>>,
}

pub struct SshService {
    sessions: DashMap<String, SshSession>,
}

impl SshService {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
        }
    }

    pub async fn connect(&self, server: &Server, auth: &Auth) -> Result<String> {
        let config = Arc::new(client::Config::default());
        let handler = SshClient;

        let mut handle = client::connect(
            config,
            (server.host.as_str(), server.port),
            handler,
        ).await?;

        // Autenticar
        match auth {
            Auth::Key { path, passphrase } => {
                let key = load_secret_key(path, passphrase.as_deref()).await?;
                handle.authenticate_publickey(&server.user, key).await?;
            }
            Auth::Password { vault_key } => {
                handle.authenticate_password(&server.user, vault_key).await?;
            }
        }

        // Abrir shell interativo com PTY
        let mut channel = handle.channel_open_session().await?;
        channel.request_pty(
            true,
            "xterm-256color",
            80, 24, 0, 0, &[],
        ).await?;
        channel.request_shell(false).await?;

        let session_id = uuid::Uuid::new_v4().to_string();
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();

        // Background task para I/O
        let sid = session_id.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    msg = channel.wait() => {
                        match msg {
                            Some(ChannelMsg::Data { data }) => {
                                // Enviar output para o TUI
                            }
                            Some(ChannelMsg::ExitStatus { .. }) => break,
                            None | Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) => break,
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

        self.sessions.insert(session_id.clone(), SshSession {
            handle: Arc::new(tokio::sync::Mutex::new(handle)),
            msg_tx,
        });

        Ok(session_id)
    }

    pub async fn write(&self, session_id: &str, data: &[u8]) -> Result<()> {
        let session = self.sessions.get(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        session.msg_tx.send(data.to_vec())?;
        Ok(())
    }

    pub async fn disconnect(&self, session_id: &str) -> Result<()> {
        if let Some((_, session)) = self.sessions.remove(session_id) {
            drop(session.msg_tx);
        }
        Ok(())
    }
}
```

2. Commit: `feat: create SSH service with russh PTY support`

---

## [S4] Task 31: SFTP Service com russh-sftp

**Files:**
- Rewrite: `src/sftp/remote.rs`
- Create: `src/sftp/service.rs`

**Steps:**

1. Criar `src/sftp/service.rs`:
```rust
use anyhow::Result;
use russh_sftp::client::SftpSession;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::config::models::Server;
use super::FileInfo;

pub struct SftpService {
    session: Option<SftpSession>,
}

impl SftpService {
    pub fn new() -> Self {
        Self { session: None }
    }

    pub async fn connect(&mut self, server: &Server) -> Result<()> {
        // Conectar via SSH e abrir canal SFTP
        let config = std::sync::Arc::new(russh::client::Config::default());
        let mut handle = russh::client::connect(
            config,
            (server.host.as_str(), server.port),
            SftpClient,
        ).await?;

        // Autenticar
        match &server.auth {
            crate::config::models::Auth::Key { path, passphrase } => {
                let key = russh_keys::load_secret_key(path, passphrase.as_deref()).await?;
                handle.authenticate_publickey(&server.user, key).await?;
            }
            crate::config::models::Auth::Password { vault_key } => {
                handle.authenticate_password(&server.user, vault_key).await?;
            }
        }

        // Abrir canal SFTP
        let channel = handle.channel_open_session().await?;
        channel.request_subsystem(true, "sftp").await?;
        let session = SftpSession::new(channel.into_stream()).await?;

        self.session = Some(session);
        Ok(())
    }

    pub async fn list(&self, path: &str) -> Result<Vec<FileInfo>> {
        let session = self.session.as_ref()
            .ok_or_else(|| anyhow::anyhow!("SFTP not connected"))?;

        let entries = session.read_dir(path).await?;
        let files = entries.map(|e| {
            FileInfo {
                name: e.file_name(),
                is_dir: e.file_type().is_dir(),
                size: e.metadata().size.unwrap_or(0),
            }
        }).collect();

        Ok(files)
    }

    pub async fn upload(&self, local_path: &str, remote_path: &str) -> Result<()> {
        let session = self.session.as_ref()
            .ok_or_else(|| anyhow::anyhow!("SFTP not connected"))?;

        let mut local_file = tokio::fs::File::open(local_path).await?;
        let mut remote_file = session.open_with_flags(
            remote_path,
            OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNCATE,
        ).await?;

        let mut buffer = [0u8; 65536];
        loop {
            let n = local_file.read(&mut buffer).await?;
            if n == 0 { break; }
            remote_file.write_all(&buffer[..n]).await?;
        }

        Ok(())
    }

    pub async fn download(&self, remote_path: &str, local_path: &str) -> Result<()> {
        let session = self.session.as_ref()
            .ok_or_else(|| anyhow::anyhow!("SFTP not connected"))?;

        let mut remote_file = session.open(remote_path).await?;
        let mut local_file = tokio::fs::File::create(local_path).await?;

        let mut buffer = [0u8; 65536];
        loop {
            let n = remote_file.read(&mut buffer).await?;
            if n == 0 { break; }
            local_file.write_all(&buffer[..n]).await?;
        }

        Ok(())
    }
}
```

2. Commit: `feat: create SFTP service with russh-sftp`

---

## [S5] Task 32: Integrar no TUI

**Files:**
- Modify: `src/tui/app.rs`
- Modify: `src/tui/ssh_terminal.rs`
- Modify: `src/tui/sftp_browser.rs`
- Modify: `src/main.rs`

**Steps:**

1. Adicionar `SshService` e `SftpService` ao `App`
2. Atualizar `connect_ssh` para usar o novo service
3. Atualizar `SFTP` para usar o novo service
4. Conectar TUI ao service via channels
5. Commit: `feat: integrate new SSH/SFTP services into TUI`

---

## [S6] Resumo

| Task | Descrição | Dependências |
|------|-----------|--------------|
| 30 | SSH Service com russh | russh |
| 31 | SFTP Service com russh-sftp | russh-sftp |
| 32 | Integração no TUI | Tasks 30-31 |

### Mudanças Principais

1. **Sessão SSH persistente** - Uma conexão para todos os comandos
2. **PTY real** - Terminal interativo com saída formatada
3. **SFTP nativo** - Via russh-sftp em vez de ls via SSH
4. **Upload/download integrado** - Via SFTP real com progresso
5. **Background I/O** - Não bloqueia o TUI

### Nota

Esta é uma refatoração significativa. A implementação atual com sshpass/scp funciona mas é menos integrada. O novo approach será mais robusto e com melhor performance.
