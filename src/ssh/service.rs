use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use russh::client::Handler;
use russh::keys::*;
use russh::*;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

use crate::config::models::{Auth, Server};

/// Client handler for SSH connections.
struct SshClient;

impl Handler for SshClient {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // TODO: Implement known_hosts verification before production use
        Ok(true)
    }
}

/// Represents the state of an SSH session.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionStatus {
    Connecting,
    Connected,
    Disconnected,
    Error(String),
}

/// A single SSH session with PTY support.
pub struct SshSession {
    pub id: String,
    pub server: Server,
    pub status: SessionStatus,
    handle: Option<client::Handle<SshClient>>,
}

impl SshSession {
    /// Create a new session (not yet connected).
    pub fn new(server: Server) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            server,
            status: SessionStatus::Connecting,
            handle: None,
        }
    }

    /// Connect to the remote server using the configured authentication.
    pub async fn connect(&mut self) -> Result<()> {
        let config = client::Config {
            inactivity_timeout: Some(std::time::Duration::from_secs(30)),
            ..Default::default()
        };
        let config = Arc::new(config);

        let mut session = client::connect(
            config,
            (self.server.host.as_str(), self.server.port),
            SshClient,
        )
        .await
        .context("Failed to establish SSH connection")?;

        match &self.server.auth {
            Auth::Key { .. } => {
                let key = super::auth::load_key(&self.server.auth)?;
                let auth_res = session
                    .authenticate_publickey(&self.server.user, key)
                    .await
                    .context("Public key auth request failed")?;
                if !auth_res.success() {
                    bail!("Public key authentication failed");
                }
            }
            Auth::Password { vault_key } => {
                if vault_key.is_empty() {
                    bail!("Password not configured for this server");
                }
                let auth_res = session
                    .authenticate_password(&self.server.user, vault_key)
                    .await
                    .context("Password auth request failed")?;
                if !auth_res.success() {
                    bail!("Password authentication failed");
                }
            }
        }

        self.handle = Some(session);
        self.status = SessionStatus::Connected;
        Ok(())
    }

    /// Open an interactive shell with optional PTY (pseudo-terminal).
    pub async fn open_shell(&mut self, use_pty: bool) -> Result<ShellChannel> {
        let handle = self
            .handle
            .as_ref()
            .context("Session not connected")?;

        let channel = handle.channel_open_session().await?;

        if use_pty {
            channel
                .request_pty(
                    false,
                    "xterm-256color",
                    80, 24, 0, 0,
                    &[],
                )
                .await?;

            channel.request_shell(true).await?;
        } else {
            channel.request_shell(true).await?;
        }

        let (data_tx, data_rx) = mpsc::channel::<Vec<u8>>(256);
        let (event_tx, event_rx) = mpsc::channel::<ShellEvent>(64);

        // Convert channel into a stream (consumes channel)
        let stream = channel.into_stream();

        // Split into reader and writer halves
        let (mut reader, writer) = tokio::io::split(stream);

        // Spawn reader task: channel -> data_tx
        tokio::spawn(async move {
            use tokio::io::AsyncReadExt;
            let mut buf = vec![0u8; 65536];
            loop {
                match reader.read(&mut buf).await {
                    Ok(0) => {
                        let _ = event_tx.send(ShellEvent::Closed).await;
                        break;
                    }
                    Ok(n) => {
                        if data_tx.send(buf[..n].to_vec()).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => {
                        let _ = event_tx.send(ShellEvent::Closed).await;
                        break;
                    }
                }
            }
        });

        Ok(ShellChannel {
            writer: Arc::new(Mutex::new(Box::new(writer))),
            data_rx: Arc::new(Mutex::new(data_rx)),
            event_rx: Arc::new(Mutex::new(event_rx)),
        })
    }

    /// Execute a single command and return the output.
    pub async fn execute(&self, command: &str) -> Result<String> {
        let handle = self
            .handle
            .as_ref()
            .context("Session not connected")?;

        let mut channel = handle.channel_open_session().await?;
        channel.exec(true, command).await?;

        let mut output = String::new();
        while let Some(msg) = channel.wait().await {
            match msg {
                ChannelMsg::Data { data } => {
                    output.push_str(&String::from_utf8_lossy(&data));
                }
                ChannelMsg::ExitStatus { .. } => break,
                _ => {}
            }
        }

        Ok(output)
    }

    /// Request a PTY resize (for when terminal window changes size).
    pub async fn resize_pty(
        &self,
        cols: u32,
        rows: u32,
        pixel_width: u32,
        pixel_height: u32,
    ) -> Result<()> {
        let handle = self
            .handle
            .as_ref()
            .context("Session not connected")?;

        let channel = handle.channel_open_session().await?;
        channel
            .window_change(cols, rows, pixel_width, pixel_height)
            .await?;

        Ok(())
    }

    /// Gracefully disconnect the session.
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(handle) = self.handle.take() {
            handle
                .disconnect(Disconnect::ByApplication, "", "English")
                .await?;
        }
        self.status = SessionStatus::Disconnected;
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        matches!(self.status, SessionStatus::Connected)
    }
}

/// Events from an interactive shell channel.
#[derive(Debug)]
pub enum ShellEvent {
    Exited(u32),
    Closed,
}

/// Handle to an interactive shell channel.
pub struct ShellChannel {
    pub writer: Arc<Mutex<Box<dyn tokio::io::AsyncWrite + Send + Unpin>>>,
    pub data_rx: Arc<Mutex<mpsc::Receiver<Vec<u8>>>>,
    event_rx: Arc<Mutex<mpsc::Receiver<ShellEvent>>>,
}

impl ShellChannel {
    /// Send user input to the remote shell.
    pub async fn send_input(&self, data: &[u8]) -> Result<()> {
        use tokio::io::AsyncWriteExt;
        let mut writer = self.writer.lock().await;
        writer.write_all(data).await.context("Failed to send input")?;
        writer.flush().await.context("Failed to flush")?;
        Ok(())
    }

    /// Check for shell events (exit, close).
    pub async fn next_event(&self) -> Option<ShellEvent> {
        self.event_rx.lock().await.recv().await
    }
}

/// Manages multiple SSH sessions.
pub struct SshService {
    sessions: HashMap<String, SshSession>,
}

impl SshService {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    /// Connect to a server and return the session ID.
    pub async fn connect(&mut self, server: &Server) -> Result<String> {
        let mut session = SshSession::new(server.clone());
        session.connect().await?;
        let id = session.id.clone();
        self.sessions.insert(id.clone(), session);
        Ok(id)
    }

    /// Open an interactive shell on an existing session.
    pub async fn open_shell(&mut self, session_id: &str, use_pty: bool) -> Result<ShellChannel> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
        session.open_shell(use_pty).await
    }

    /// Get a reference to a session by ID.
    pub fn get_session(&self, id: &str) -> Option<&SshSession> {
        self.sessions.get(id)
    }

    /// Get a mutable reference to a session by ID.
    pub fn get_session_mut(&mut self, id: &str) -> Option<&mut SshSession> {
        self.sessions.get_mut(id)
    }

    /// List all active session IDs.
    pub fn list_sessions(&self) -> Vec<String> {
        self.sessions.keys().cloned().collect()
    }

    /// Disconnect and remove a session.
    pub async fn disconnect(&mut self, id: &str) -> Result<()> {
        if let Some(mut session) = self.sessions.remove(id) {
            session.disconnect().await?;
        }
        Ok(())
    }

    /// Disconnect all sessions.
    pub async fn disconnect_all(&mut self) {
        for (_, mut session) in self.sessions.drain() {
            let _ = session.disconnect().await;
        }
    }

    /// Get the number of active sessions.
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Test TCP connectivity to a host:port with timeout.
    pub async fn test_connection(host: &str, port: u16, timeout_secs: u64) -> Result<(), String> {
        let addr = format!("{}:{}", host, port);
        let sock_addr: std::net::SocketAddr = addr
            .parse()
            .map_err(|e| format!("Endereço inválido: {}", e))?;
        let timeout = std::time::Duration::from_secs(timeout_secs);
        tokio::time::timeout(timeout, tokio::net::TcpStream::connect(sock_addr))
            .await
            .map_err(|_| format!("Timeout após {}s — servidor inacessível", timeout_secs))?
            .map_err(|e| format!("Falha na conexão TCP: {}", e))?;
        Ok(())
    }
}

#[cfg(test)]
impl SshService {
    pub(crate) fn _test_add_session(&mut self, session: SshSession) -> String {
        let id = session.id.clone();
        self.sessions.insert(id.clone(), session);
        id
    }
}

impl Drop for SshService {
    fn drop(&mut self) {
        // Sessions are dropped here; the tokio tasks will be cancelled.
        // For graceful shutdown, call disconnect_all() before dropping.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_server() -> Server {
        Server {
            name: "test".to_string(),
            host: "localhost".to_string(),
            port: 22,
            user: "testuser".to_string(),
            auth: Auth::Key {
                path: "~/.ssh/id_rsa".to_string(),
                passphrase: None,
            },
            tags: vec![],
            pinned: false,
        last_connected: None,
        connection_count: 0,
            bookmarks: vec![],
            agent_forwarding: false,
            proxy_jump: None,


        }
    }

    fn mock_session() -> SshSession {
        SshSession::new(test_server())
    }

    #[tokio::test]
    async fn test_service_new() {
        let service = SshService::new();
        assert_eq!(service.session_count(), 0);
        assert!(service.list_sessions().is_empty());
    }

    #[test]
    fn test_session_status_transitions() {
        let server = test_server();
        let session = SshSession::new(server);
        assert_eq!(session.status, SessionStatus::Connecting);
        assert!(!session.is_connected());
    }

    #[test]
    fn test_session_id_unique() {
        let server = test_server();
        let s1 = SshSession::new(server.clone());
        let s2 = SshSession::new(server);
        assert_ne!(s1.id, s2.id);
    }

    #[tokio::test]
    async fn test_get_session_found() {
        let mut service = SshService::new();
        let session = mock_session();
        let id = session.id.clone();
        service._test_add_session(session);
        let found = service.get_session(&id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, id);
    }

    #[tokio::test]
    async fn test_get_session_not_found() {
        let service = SshService::new();
        assert!(service.get_session("nonexistent").is_none());
    }

    #[tokio::test]
    async fn test_get_session_mut() {
        let mut service = SshService::new();
        let session = mock_session();
        let id = session.id.clone();
        service._test_add_session(session);
        let found = service.get_session_mut(&id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, id);
    }

    #[tokio::test]
    async fn test_disconnect_removes_session() {
        let mut service = SshService::new();
        let session = mock_session();
        let id = session.id.clone();
        service._test_add_session(session);
        assert_eq!(service.session_count(), 1);
        service.disconnect(&id).await.unwrap();
        assert_eq!(service.session_count(), 0);
    }

    #[tokio::test]
    async fn test_disconnect_nonexistent_returns_ok() {
        let mut service = SshService::new();
        assert!(service.disconnect("ghost").await.is_ok());
    }

    #[tokio::test]
    async fn test_disconnect_all_empties_sessions() {
        let mut service = SshService::new();
        service._test_add_session(mock_session());
        service._test_add_session(mock_session());
        service._test_add_session(mock_session());
        assert_eq!(service.session_count(), 3);
        service.disconnect_all().await;
        assert_eq!(service.session_count(), 0);
    }

    #[tokio::test]
    async fn test_list_sessions_returns_ids() {
        let mut service = SshService::new();
        let session = mock_session();
        let id = session.id.clone();
        service._test_add_session(session);
        let ids = service.list_sessions();
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], id);
    }

    #[tokio::test]
    async fn test_session_count_non_empty() {
        let mut service = SshService::new();
        service._test_add_session(mock_session());
        service._test_add_session(mock_session());
        assert_eq!(service.session_count(), 2);
    }

    // --- T3.2: test_connection ---

    #[tokio::test]
    async fn test_connection_invalid_address() {
        let result = SshService::test_connection("not.a.valid.host", 22, 2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_connection_timeout() {
        // Port 1 on a non-routable address should timeout
        let result = SshService::test_connection("192.0.2.1", 1, 1).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Timeout"));
    }
}
