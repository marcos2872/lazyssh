use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use russh::client::Handler;
use russh::keys::*;
use russh::*;
use russh_sftp::client::SftpSession;
use uuid::Uuid;

use crate::config::models::{Auth, Server};

use super::FileInfo;

/// Client handler for SSH connections (used for SFTP).
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

/// Represents the state of an SFTP session.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionStatus {
    Connecting,
    Connected,
    Disconnected,
    Error(String),
}

/// A single SFTP session for file transfer operations.
pub struct SftpServiceSession {
    pub id: String,
    pub server: Server,
    pub status: SessionStatus,
    sftp: Option<SftpSession>,
}

impl SftpServiceSession {
    /// Create a new session (not yet connected).
    pub fn new(server: Server) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            server,
            status: SessionStatus::Connecting,
            sftp: None,
        }
    }

    /// Connect to the remote server and initialize SFTP.
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

        // Authenticate
        match &self.server.auth {
            Auth::Key { .. } => {
                let key = crate::ssh::auth::load_key(&self.server.auth)?;
                let auth_res = session
                    .authenticate_publickey(&self.server.user, key)
                    .await
                    .context("Public key auth request failed")?;
                if !auth_res.success() {
                    anyhow::bail!("Public key authentication failed");
                }
            }
            Auth::Password { vault_key } => {
                if vault_key.is_empty() {
                    anyhow::bail!("Password not configured for this server");
                }
                let auth_res = session
                    .authenticate_password(&self.server.user, vault_key)
                    .await
                    .context("Password auth request failed")?;
                if !auth_res.success() {
                    anyhow::bail!("Password authentication failed");
                }
            }
        }

        // Open SFTP subsystem
        let channel = session.channel_open_session().await?;
        channel.request_subsystem(true, "sftp").await?;

        // Create SFTP session from the channel stream
        let stream = channel.into_stream();
        let sftp = SftpSession::new(stream)
            .await
            .context("Failed to initialize SFTP session")?;

        self.sftp = Some(sftp);
        self.status = SessionStatus::Connected;
        Ok(())
    }

    /// List files in a directory.
    pub async fn list_dir(&self, path: &str) -> Result<Vec<FileInfo>> {
        let sftp = self
            .sftp
            .as_ref()
            .context("SFTP session not connected")?;

        let entries = sftp
            .read_dir(path)
            .await
            .context("Failed to list directory")?;

        let mut files: Vec<FileInfo> = entries
            .map(|entry| {
                let metadata = entry.metadata();
                FileInfo {
                    name: entry.file_name(),
                    is_dir: metadata.is_dir(),
                    size: metadata.len(),
                }
            })
            .collect();

        files.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(files)
    }

    /// Read a file's contents as bytes.
    pub async fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        let sftp = self
            .sftp
            .as_ref()
            .context("SFTP session not connected")?;

        let data = sftp
            .read(path)
            .await
            .context("Failed to read file")?;

        Ok(data)
    }

    /// Read a file's contents as string.
    pub async fn read_file_string(&self, path: &str) -> Result<String> {
        let data = self.read_file(path).await?;
        String::from_utf8(data).context("File is not valid UTF-8")
    }

    /// Write data to a file (creates or overwrites).
    pub async fn write_file(&self, path: &str, data: &[u8]) -> Result<()> {
        let sftp = self
            .sftp
            .as_ref()
            .context("SFTP session not connected")?;

        use russh_sftp::protocol::OpenFlags;
        use tokio::io::AsyncWriteExt;
        let mut file = sftp
            .open_with_flags(
                path,
                OpenFlags::CREATE | OpenFlags::WRITE | OpenFlags::TRUNCATE,
            )
            .await
            .context("Failed to open remote file")?;
        file.write_all(data)
            .await
            .context("Failed to write file")?;

        Ok(())
    }

    /// Download a file from remote to local path.
    pub async fn download(&self, remote_path: &str, local_path: &str) -> Result<()> {
        let data = self.read_file(remote_path).await?;
        tokio::fs::write(local_path, data)
            .await
            .context("Failed to write local file")?;
        Ok(())
    }

    /// Upload a local file to remote path.
    pub async fn upload(&self, local_path: &str, remote_path: &str) -> Result<()> {
        let data = tokio::fs::read(local_path)
            .await
            .context("Failed to read local file")?;
        self.write_file(remote_path, &data).await
    }

    /// Check if a file or directory exists.
    pub async fn exists(&self, path: &str) -> Result<bool> {
        let sftp = self
            .sftp
            .as_ref()
            .context("SFTP session not connected")?;

        let exists = sftp
            .try_exists(path)
            .await
            .context("Failed to check existence")?;

        Ok(exists)
    }

    /// Get file size via metadata.
    pub async fn get_file_size(&self, path: &str) -> Result<u64> {
        let sftp = self
            .sftp
            .as_ref()
            .context("SFTP session not connected")?;

        let meta = sftp
            .metadata(path)
            .await
            .context("Failed to get metadata")?;

        Ok(meta.len())
    }

    /// Create a directory.
    pub async fn mkdir(&self, path: &str) -> Result<()> {
        let sftp = self
            .sftp
            .as_ref()
            .context("SFTP session not connected")?;

        sftp.create_dir(path)
            .await
            .context("Failed to create directory")?;

        Ok(())
    }

    /// Remove a file.
    pub async fn remove_file(&self, path: &str) -> Result<()> {
        let sftp = self
            .sftp
            .as_ref()
            .context("SFTP session not connected")?;

        sftp.remove_file(path)
            .await
            .context("Failed to remove file")?;

        Ok(())
    }

    /// Remove a directory.
    pub async fn remove_dir(&self, path: &str) -> Result<()> {
        let sftp = self
            .sftp
            .as_ref()
            .context("SFTP session not connected")?;

        sftp.remove_dir(path)
            .await
            .context("Failed to remove directory")?;

        Ok(())
    }

    /// Rename a file or directory.
    pub async fn rename(&self, old_path: &str, new_path: &str) -> Result<()> {
        let sftp = self
            .sftp
            .as_ref()
            .context("SFTP session not connected")?;

        sftp.rename(old_path, new_path)
            .await
            .context("Failed to rename")?;

        Ok(())
    }

    /// Gracefully disconnect the session.
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(sftp) = self.sftp.take() {
            let _ = sftp.close().await;
        }
        self.status = SessionStatus::Disconnected;
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        matches!(self.status, SessionStatus::Connected)
    }
}

/// Manages multiple SFTP sessions.
pub struct SftpService {
    sessions: HashMap<String, SftpServiceSession>,
}

impl SftpService {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    /// Connect to a server and return the session ID.
    pub async fn connect(&mut self, server: &Server) -> Result<String> {
        let mut session = SftpServiceSession::new(server.clone());
        session.connect().await?;
        let id = session.id.clone();
        self.sessions.insert(id.clone(), session);
        Ok(id)
    }

    /// Get a reference to a session by ID.
    pub fn get_session(&self, id: &str) -> Option<&SftpServiceSession> {
        self.sessions.get(id)
    }

    /// Get a mutable reference to a session by ID.
    pub fn get_session_mut(&mut self, id: &str) -> Option<&mut SftpServiceSession> {
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
}

impl Default for SftpService {
    fn default() -> Self {
        Self::new()
    }
}

/// Insert a session for testing without real SFTP connection.
#[cfg(test)]
impl SftpService {
    pub(crate) fn _test_add_session(&mut self, session: SftpServiceSession) -> String {
        let id = session.id.clone();
        self.sessions.insert(id.clone(), session);
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::models::Auth;

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
        }
    }

    fn mock_session() -> SftpServiceSession {
        SftpServiceSession::new(test_server())
    }

    #[test]
    fn test_service_new() {
        let service = SftpService::new();
        assert_eq!(service.session_count(), 0);
        assert!(service.list_sessions().is_empty());
    }

    #[test]
    fn test_session_status_transitions() {
        let server = test_server();
        let session = SftpServiceSession::new(server);
        assert_eq!(session.status, SessionStatus::Connecting);
        assert!(!session.is_connected());
    }

    #[test]
    fn test_session_id_unique() {
        let server = test_server();
        let s1 = SftpServiceSession::new(server.clone());
        let s2 = SftpServiceSession::new(server);
        assert_ne!(s1.id, s2.id);
    }

    #[tokio::test]
    async fn test_get_session_found() {
        let mut service = SftpService::new();
        let session = mock_session();
        let id = session.id.clone();
        service._test_add_session(session);
        assert!(service.get_session(&id).is_some());
    }

    #[tokio::test]
    async fn test_get_session_not_found() {
        let service = SftpService::new();
        assert!(service.get_session("ghost").is_none());
    }

    #[tokio::test]
    async fn test_get_session_mut() {
        let mut service = SftpService::new();
        let session = mock_session();
        let id = session.id.clone();
        service._test_add_session(session);
        assert!(service.get_session_mut(&id).is_some());
    }

    #[tokio::test]
    async fn test_disconnect_removes_session() {
        let mut service = SftpService::new();
        let session = mock_session();
        let id = session.id.clone();
        service._test_add_session(session);
        assert_eq!(service.session_count(), 1);
        service.disconnect(&id).await.unwrap();
        assert_eq!(service.session_count(), 0);
    }

    #[tokio::test]
    async fn test_disconnect_nonexistent_returns_ok() {
        let mut service = SftpService::new();
        assert!(service.disconnect("ghost").await.is_ok());
    }

    #[tokio::test]
    async fn test_disconnect_all_empties_sessions() {
        let mut service = SftpService::new();
        service._test_add_session(mock_session());
        service._test_add_session(mock_session());
        assert_eq!(service.session_count(), 2);
        service.disconnect_all().await;
        assert_eq!(service.session_count(), 0);
    }

    #[tokio::test]
    async fn test_list_sessions_non_empty() {
        let mut service = SftpService::new();
        service._test_add_session(mock_session());
        assert_eq!(service.list_sessions().len(), 1);
    }

    #[tokio::test]
    async fn test_session_count_non_empty() {
        let mut service = SftpService::new();
        service._test_add_session(mock_session());
        service._test_add_session(mock_session());
        assert_eq!(service.session_count(), 2);
    }

    #[test]
    fn test_default() {
        let service = SftpService::default();
        assert_eq!(service.session_count(), 0);
    }
}
