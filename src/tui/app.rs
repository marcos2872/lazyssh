use crate::config::models::{Auth, Server};
use crate::ssh::SshService;
use crate::sftp::{FileInfo, SftpService};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use tokio::sync::mpsc;
use super::effects::AppEffects;
use super::notifications::NotificationQueue;
use super::sftp_browser::SftpState;
use super::ssh_terminal::SshTerminalState;

#[derive(Debug, Clone, PartialEq)]
pub enum EditField {
    Name,
    Host,
    Port,
    User,
    AuthType,
    KeyPath,
    Passphrase,
    Password,
}

#[derive(Debug, Clone)]
pub struct EditState {
    pub field: EditField,
    pub name: String,
    pub host: String,
    pub port: String,
    pub user: String,
    pub auth_type: String,
    pub key_path: String,
    pub passphrase: String,
    pub password: String,
    pub server_index: usize,
}

impl EditState {
    pub fn from_server(server: &Server, index: usize) -> Self {
        let (auth_type, key_path, passphrase) = match &server.auth {
            Auth::Key { path, passphrase } => (
                "key".to_string(),
                path.clone(),
                passphrase.clone().unwrap_or_default(),
            ),
            Auth::Password { .. } => ("password".to_string(), String::new(), String::new()),
        };
        Self {
            field: EditField::Name,
            name: server.name.clone(),
            host: server.host.clone(),
            port: server.port.to_string(),
            user: server.user.clone(),
            auth_type,
            key_path,
            passphrase,
            password: String::new(),
            server_index: index,
        }
    }

    pub fn is_key_auth(&self) -> bool {
        self.auth_type.to_lowercase() == "key"
    }

    pub fn current_value(&self) -> &str {
        match self.field {
            EditField::Name => &self.name,
            EditField::Host => &self.host,
            EditField::Port => &self.port,
            EditField::User => &self.user,
            EditField::AuthType => &self.auth_type,
            EditField::KeyPath => &self.key_path,
            EditField::Passphrase => &self.passphrase,
            EditField::Password => &self.password,
        }
    }

    pub fn current_value_mut(&mut self) -> &mut String {
        match self.field {
            EditField::Name => &mut self.name,
            EditField::Host => &mut self.host,
            EditField::Port => &mut self.port,
            EditField::User => &mut self.user,
            EditField::AuthType => &mut self.auth_type,
            EditField::KeyPath => &mut self.key_path,
            EditField::Passphrase => &mut self.passphrase,
            EditField::Password => &mut self.password,
        }
    }

    pub fn next_field(&mut self) {
        self.field = if self.is_key_auth() {
            match self.field {
                EditField::Name => EditField::Host,
                EditField::Host => EditField::Port,
                EditField::Port => EditField::User,
                EditField::User => EditField::AuthType,
                EditField::AuthType => EditField::KeyPath,
                EditField::KeyPath => EditField::Passphrase,
                EditField::Passphrase => EditField::Name,
                _ => EditField::Name,
            }
        } else {
            match self.field {
                EditField::Name => EditField::Host,
                EditField::Host => EditField::Port,
                EditField::Port => EditField::User,
                EditField::User => EditField::AuthType,
                EditField::AuthType => EditField::Password,
                EditField::Password => EditField::Name,
                _ => EditField::Name,
            }
        };
    }

    pub fn prev_field(&mut self) {
        self.field = if self.is_key_auth() {
            match self.field {
                EditField::Name => EditField::Passphrase,
                EditField::Host => EditField::Name,
                EditField::Port => EditField::Host,
                EditField::User => EditField::Port,
                EditField::AuthType => EditField::User,
                EditField::KeyPath => EditField::AuthType,
                EditField::Passphrase => EditField::KeyPath,
                _ => EditField::Name,
            }
        } else {
            match self.field {
                EditField::Name => EditField::Password,
                EditField::Host => EditField::Name,
                EditField::Port => EditField::Host,
                EditField::User => EditField::Port,
                EditField::AuthType => EditField::User,
                EditField::Password => EditField::AuthType,
                _ => EditField::Name,
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
}

#[derive(Debug, Clone, PartialEq)]
pub enum InsertField {
    Name,
    Host,
    Port,
    User,
    AuthType,
    KeyPath,
    Passphrase,
    Password,
}

#[derive(Debug, Clone)]
pub struct InsertState {
    pub field: InsertField,
    pub name: String,
    pub host: String,
    pub port: String,
    pub user: String,
    pub auth_type: String,
    pub key_path: String,
    pub passphrase: String,
    pub password: String,
}

impl InsertState {
    pub fn new() -> Self {
        Self {
            field: InsertField::Name,
            name: String::new(),
            host: String::new(),
            port: "22".to_string(),
            user: "root".to_string(),
            auth_type: "key".to_string(),
            key_path: "~/.ssh/id_rsa".to_string(),
            passphrase: String::new(),
            password: String::new(),
        }
    }

    pub fn is_key_auth(&self) -> bool {
        self.auth_type.to_lowercase() == "key"
    }

    pub fn current_value(&self) -> &str {
        match self.field {
            InsertField::Name => &self.name,
            InsertField::Host => &self.host,
            InsertField::Port => &self.port,
            InsertField::User => &self.user,
            InsertField::AuthType => &self.auth_type,
            InsertField::KeyPath => &self.key_path,
            InsertField::Passphrase => &self.passphrase,
            InsertField::Password => &self.password,
        }
    }

    pub fn current_value_mut(&mut self) -> &mut String {
        match self.field {
            InsertField::Name => &mut self.name,
            InsertField::Host => &mut self.host,
            InsertField::Port => &mut self.port,
            InsertField::User => &mut self.user,
            InsertField::AuthType => &mut self.auth_type,
            InsertField::KeyPath => &mut self.key_path,
            InsertField::Passphrase => &mut self.passphrase,
            InsertField::Password => &mut self.password,
        }
    }

    pub fn next_field(&mut self) {
        self.field = if self.is_key_auth() {
            match self.field {
                InsertField::Name => InsertField::Host,
                InsertField::Host => InsertField::Port,
                InsertField::Port => InsertField::User,
                InsertField::User => InsertField::AuthType,
                InsertField::AuthType => InsertField::KeyPath,
                InsertField::KeyPath => InsertField::Passphrase,
                InsertField::Passphrase => InsertField::Name,
                _ => InsertField::Name,
            }
        } else {
            match self.field {
                InsertField::Name => InsertField::Host,
                InsertField::Host => InsertField::Port,
                InsertField::Port => InsertField::User,
                InsertField::User => InsertField::AuthType,
                InsertField::AuthType => InsertField::Password,
                InsertField::Password => InsertField::Name,
                _ => InsertField::Name,
            }
        };
    }

    pub fn prev_field(&mut self) {
        self.field = if self.is_key_auth() {
            match self.field {
                InsertField::Name => InsertField::Passphrase,
                InsertField::Host => InsertField::Name,
                InsertField::Port => InsertField::Host,
                InsertField::User => InsertField::Port,
                InsertField::AuthType => InsertField::User,
                InsertField::KeyPath => InsertField::AuthType,
                InsertField::Passphrase => InsertField::KeyPath,
                _ => InsertField::Name,
            }
        } else {
            match self.field {
                InsertField::Name => InsertField::Password,
                InsertField::Host => InsertField::Name,
                InsertField::Port => InsertField::Host,
                InsertField::User => InsertField::Port,
                InsertField::AuthType => InsertField::User,
                InsertField::Password => InsertField::AuthType,
                _ => InsertField::Name,
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
}

#[derive(Debug, Clone)]
pub enum ConfirmAction {
    DeleteServer { name: String },
}

#[derive(Debug, Clone)]
pub struct ConfirmState {
    pub action: ConfirmAction,
}

#[derive(Debug, PartialEq)]
pub enum InputMode {
    Normal,
    Search,
    Insert,
    Edit,
    Confirm,
}

#[derive(Debug, PartialEq)]
pub enum CurrentView {
    ServerList,
    SshTerminal,
    SftpBrowser,
}

pub enum SftpOpResult {
    ListDir(Vec<crate::sftp::FileInfo>),
    Upload(String),
    Download(String),
    Mkdir,
    Unlink(String),
    Rmdir(String),
    Rename(String),
    SetPermissions,
    Error(String),
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
    pub ssh_state: Option<SshTerminalState>,
    pub insert_state: Option<InsertState>,
    pub edit_state: Option<EditState>,
    pub notifications: NotificationQueue,
    pub effects: AppEffects,
    pub ssh_service: SshService,
    pub sftp_service: std::sync::Arc<std::sync::Mutex<SftpService>>,
    pub ssh_output_rx: Option<mpsc::UnboundedReceiver<String>>,
    pub sftp_progress_rx: Option<mpsc::UnboundedReceiver<u64>>,
    pub sftp_op_rx: Option<mpsc::UnboundedReceiver<SftpOpResult>>,
    pub help_visible: bool,
    pub confirm_state: Option<ConfirmState>,
    pub start_time: std::time::Instant,
    pub sort_by: Option<String>,
}

impl App {
    pub fn new(servers: Vec<Server>) -> Self {
        let filtered_indices = (0..servers.len()).collect();
        let mut notifications = NotificationQueue::new();
        if servers.is_empty() {
            notifications.info("Nenhum servidor configurado. Pressione 'a' para adicionar.");
        } else {
            notifications.success(&format!("{} servidor(es) carregado(s).", servers.len()));
        }
        Self {
            servers,
            filtered_indices,
            selected: 0,
            current_view: CurrentView::ServerList,
            input_mode: InputMode::Normal,
            input: String::new(),
            should_quit: false,
            sftp_state: None,
            ssh_state: None,
            insert_state: None,
            edit_state: None,
            notifications,
            effects: AppEffects::new(),
            ssh_service: SshService::new(),
            sftp_service: std::sync::Arc::new(std::sync::Mutex::new(SftpService::new())),
            ssh_output_rx: None,
            sftp_progress_rx: None,
            help_visible: false,
            confirm_state: None,
            start_time: std::time::Instant::now(),
            sort_by: None,
            sftp_op_rx: None,
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

    pub fn sort_servers(&mut self) {
        match self.sort_by.as_deref() {
            Some("name") => {
                self.servers.sort_by(|a, b| a.name.cmp(&b.name));
            }
            Some("port") => {
                self.servers.sort_by(|a, b| a.port.cmp(&b.port));
            }
            Some("last_connected") => {
                self.servers.sort_by(|a, b| {
                    b.last_connected.cmp(&a.last_connected)
                });
            }
            Some("frequency") => {
                self.servers.sort_by(|a, b| {
                    b.connection_count.cmp(&a.connection_count)
                });
            }
            _ => {}
        }
        // Pinned servers always on top
        self.servers.sort_by(|a, b| b.pinned.cmp(&a.pinned));
        self.filter(&self.input.clone());
    }

    pub fn cycle_sort_by(&mut self) {
        let next = match self.sort_by.as_deref() {
            None => Some("name"),
            Some("name") => Some("port"),
            Some("port") => Some("last_connected"),
            Some("last_connected") => Some("frequency"),
            Some("frequency") => None,
            _ => None,
        };
        self.sort_by = next.map(|s| s.to_string());
        self.sort_servers();

        let label = match self.sort_by.as_deref() {
            Some("name") => "Nome",
            Some("port") => "Porta",
            Some("last_connected") => "Último acesso",
            Some("frequency") => "Frequência",
            _ => "Padrão",
        };
        self.notifications.info(&format!("Ordenado por: {}", label));
    }

    pub fn selected_server(&self) -> Option<&Server> {
        self.filtered_indices
            .get(self.selected)
            .and_then(|&i| self.servers.get(i))
    }

    pub fn selected_server_mut(&mut self) -> Option<&mut Server> {
        self.filtered_indices
            .get(self.selected)
            .and_then(|&i| self.servers.get_mut(i))
    }

    pub fn filter(&mut self, query: &str) {
        if query.is_empty() {
            self.filtered_indices = (0..self.servers.len()).collect();
        } else {
            let matcher = SkimMatcherV2::default();
            self.filtered_indices = self
                .servers
                .iter()
                .enumerate()
                .filter_map(|(i, s)| {
                    let score = matcher
                        .fuzzy_match(&s.name, query)
                        .or_else(|| matcher.fuzzy_match(&s.host, query))
                        .or_else(|| {
                            s.tags
                                .iter()
                                .find_map(|t| matcher.fuzzy_match(t, query))
                        });
                    score.map(|_| i)
                })
                .collect();
        }
        self.selected = 0;
    }

    pub fn open_sftp(&mut self) {
        if let Some(server) = self.selected_server() {
            let server = server.clone();

            // Connect using SftpService (async via block_in_place)
            let result = {
                let mut svc = self.sftp_service.lock().unwrap();
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current()
                        .block_on(svc.connect(&server))
                })
            };

            match result {
                Ok(session_id) => {
                    self.current_view = CurrentView::SftpBrowser;
                    let mut state = SftpState::new(server.clone());
                    state.session_id = Some(session_id.clone());

                    // List initial remote directory
                    let list_result = self.sftp_list_remote_dir(&session_id, &state.remote_path);
                    match list_result {
                        Ok(files) => {
                            state.refresh_remote(files);
                            state.status = "Conectado".to_string();
                        }
                        Err(e) => {
                            state.status = format!("Erro: {}", e);
                        }
                    }

                    self.sftp_state = Some(state);
                    self.notifications.info("SFTP conectado!");
                }
                Err(e) => {
                    self.notifications
                        .error(&format!("Falha ao conectar SFTP: {}", e));
                }
            }
        }
    }

    /// Spawn an SFTP operation as a background task to avoid blocking the TUI.
    fn spawn_sftp_op<F, Fut>(&mut self, op_name: &str, f: F)
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = SftpOpResult> + Send + 'static,
    {
        let (tx, rx) = mpsc::unbounded_channel();
        self.sftp_op_rx = Some(rx);
        tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(async move {
                let result = f().await;
                let _ = tx.send(result);
            });
        });
    }

    /// List a remote directory via SFTP service.
    pub fn sftp_list_remote_dir(&self, session_id: &str, path: &str) -> Result<Vec<FileInfo>, String> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let svc = self.sftp_service.lock().unwrap(); let session = svc
                    .get_session(session_id)
                    .ok_or_else(|| "SFTP session not found".to_string())?;
                session.list_dir(path).await.map_err(|e| e.to_string())
            })
        })
    }

    /// Navigate remote directory and return listing.
    pub fn sftp_enter_dir(&self, session_id: &str, path: &str) -> Result<Vec<FileInfo>, String> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let svc = self.sftp_service.lock().unwrap(); let session = svc
                    .get_session(session_id)
                    .ok_or_else(|| "SFTP session not found".to_string())?;
                session.list_dir(path).await.map_err(|e| e.to_string())
            })
        })
    }

    /// Upload a file via SFTP service.
    pub fn sftp_upload_file(&self, session_id: &str, local: &str, remote: &str) -> Result<(), String> {
        let svc = self.sftp_service.lock().unwrap();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let session = svc.get_session(session_id)
                    .ok_or_else(|| "SFTP session not found".to_string())?;
                session.upload(local, remote, None).await.map_err(|e| e.to_string())
            })
        })
    }

    /// Download a file via SFTP service.
    pub fn sftp_download_file(&self, session_id: &str, remote: &str, local: &str) -> Result<(), String> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let svc = self.sftp_service.lock().unwrap(); let session = svc
                    .get_session(session_id)
                    .ok_or_else(|| "SFTP session not found".to_string())?;
                session.download(remote, local).await.map_err(|e| e.to_string())
            })
        })
    }

    /// Create a directory via SFTP.
    pub fn sftp_mkdir(&self, session_id: &str, path: &str) -> Result<(), String> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let svc = self.sftp_service.lock().unwrap(); let session = svc
                    .get_session(session_id)
                    .ok_or_else(|| "SFTP session not found".to_string())?;
                session.mkdir(path).await.map_err(|e| e.to_string())
            })
        })
    }

    /// Remove a file via SFTP.
    pub fn sftp_unlink(&self, session_id: &str, path: &str) -> Result<(), String> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let svc = self.sftp_service.lock().unwrap(); let session = svc
                    .get_session(session_id)
                    .ok_or_else(|| "SFTP session not found".to_string())?;
                session.remove_file(path).await.map_err(|e| e.to_string())
            })
        })
    }

    /// Remove a directory via SFTP.
    pub fn sftp_rmdir(&self, session_id: &str, path: &str) -> Result<(), String> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let svc = self.sftp_service.lock().unwrap(); let session = svc
                    .get_session(session_id)
                    .ok_or_else(|| "SFTP session not found".to_string())?;
                session.remove_dir(path).await.map_err(|e| e.to_string())
            })
        })
    }

    /// Rename a file or directory via SFTP.
    pub fn sftp_rename(&self, session_id: &str, old_path: &str, new_path: &str) -> Result<(), String> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let svc = self.sftp_service.lock().unwrap(); let session = svc
                    .get_session(session_id)
                    .ok_or_else(|| "SFTP session not found".to_string())?;
                session.rename(old_path, new_path).await.map_err(|e| e.to_string())
            })
        })
    }

    /// Set file permissions via SFTP.
    pub fn sftp_set_permissions(&self, session_id: &str, path: &str, mode: u32) -> Result<(), String> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let svc = self.sftp_service.lock().unwrap(); let session = svc
                    .get_session(session_id)
                    .ok_or_else(|| "SFTP session not found".to_string())?;
                session.set_permissions(path, mode).await.map_err(|e| e.to_string())
            })
        })
    }

    pub fn close_sftp(&mut self) {
        // Disconnect SFTP session from service
        if let Some(sftp) = &self.sftp_state {
            if let Some(ref session_id) = sftp.session_id {
                let mut svc = self.sftp_service.lock().unwrap();
                let _ = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current()
                        .block_on(svc.disconnect(session_id))
                });
            }
        }
        self.current_view = CurrentView::ServerList;
        self.sftp_state = None;
    }

    pub fn connect_ssh(&mut self) {
        if let Some(server) = self.selected_server() {
            let server = server.clone();
            self.notifications
                .info(&format!("Conectando a {}...", server.name));
            self.current_view = CurrentView::SshTerminal;

            let mut state = SshTerminalState::new(server.clone());
            state.output.clear();

            // 1. Connect via SshService
            let session_result = {
                let ssh_service = &mut self.ssh_service;
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current()
                        .block_on(ssh_service.connect(&server))
                })
            };

            match session_result {
                Ok(session_id) => {
                    // 2. Open PTY shell
                    let shell_result = {
                        let ssh_service = &mut self.ssh_service;
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current()
                                .block_on(ssh_service.open_shell(&session_id, true))
                        })
                    };

                    match shell_result {
                        Ok(channel) => {
                            // 3. Spawn background task to forward output
                            let (output_tx, output_rx) = mpsc::unbounded_channel();
                            let data_rx = channel.data_rx.clone();

                            tokio::spawn(async move {
                                loop {
                                    let data = data_rx.lock().await.recv().await;
                                    match data {
                                        Some(bytes) => {
                                            let text = String::from_utf8_lossy(&bytes).to_string();
                                            if output_tx.send(text).is_err() {
                                                break;
                                            }
                                        }
                                        None => break,
                                    }
                                }
                            });

                            self.ssh_output_rx = Some(output_rx);
                            state.set_connected(session_id);
                            state.shell_writer = Some(channel.writer.clone());
                            self.notifications
                                .success(&format!("Conectado a {}!", server.name));
                        }
                        Err(e) => {
                            state.set_error(format!("Falha ao abrir shell: {}", e));
                            self.notifications
                                .error(&format!("Falha ao abrir shell: {}", e));
                        }
                    }
                }
                Err(e) => {
                    state.set_error(e.to_string());
                    self.notifications
                        .error(&format!("Falha ao conectar: {}", e));
                }
            }

            self.ssh_state = Some(state);
        }
    }

    pub fn close_ssh(&mut self) {
        // Disconnect SSH session from service
        if let Some(ssh) = &self.ssh_state {
            if let Some(ref session_id) = ssh.session_id {
                let ssh_service = &mut self.ssh_service;
                let _ = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current()
                        .block_on(ssh_service.disconnect(session_id))
                });
            }
        }
        self.current_view = CurrentView::ServerList;
        self.ssh_state = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::models::{Auth, Server};

    fn test_servers() -> Vec<Server> {
        vec![
            Server {
                name: "server1".to_string(),
                host: "192.168.1.1".to_string(),
                port: 22,
                user: "user".to_string(),
                auth: Auth::Key {
                    path: "~/.ssh/id_rsa".to_string(),
                    passphrase: None,
                },
                tags: vec!["prod".to_string()],
                pinned: false,
                last_connected: None,
                connection_count: 0,
            bookmarks: vec![],
            },
            Server {
                name: "server2".to_string(),
                host: "192.168.1.2".to_string(),
                port: 22,
                user: "user".to_string(),
                auth: Auth::Password {
                    vault_key: "key".to_string(),
                },
                tags: vec!["dev".to_string()],
                pinned: true,
                last_connected: None,
                connection_count: 0,
            bookmarks: vec![],
            },
        ]
    }

    #[test]
    fn test_empty_app() {
        let app = App::new(vec![]);
        assert!(app.filtered_indices.is_empty());
        assert_eq!(app.selected, 0);
        assert!(app.selected_server().is_none());
    }

    #[test]
    fn test_next_wraps() {
        let mut app = App::new(test_servers());
        app.next();
        assert_eq!(app.selected, 1);
        app.next();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_previous_wraps() {
        let mut app = App::new(test_servers());
        assert_eq!(app.selected, 0);
        app.previous();
        assert_eq!(app.selected, 1);
    }

    #[test]
    fn test_filter_by_name() {
        let mut app = App::new(test_servers());
        app.filter("server1");
        assert_eq!(app.filtered_indices.len(), 1);
        assert_eq!(app.selected_server().unwrap().name, "server1");
    }

    #[test]
    fn test_filter_by_tag() {
        let mut app = App::new(test_servers());
        app.filter("prod");
        assert_eq!(app.filtered_indices.len(), 1);
    }

    #[test]
    fn test_filter_no_matches() {
        let mut app = App::new(test_servers());
        app.filter("nonexistent");
        assert!(app.filtered_indices.is_empty());
    }

    #[test]
    fn test_selected_server() {
        let app = App::new(test_servers());
        let s = app.selected_server();
        assert!(s.is_some());
        assert_eq!(s.unwrap().name, "server1");
    }

    #[test]
    fn test_selected_server_empty() {
        let app = App::new(vec![]);
        assert!(app.selected_server().is_none());
    }

    // --- EditState ---

    #[test]
    fn test_edit_state_from_key_server() {
        let server = Server {
            name: "editme".into(), host: "10.0.0.1".into(), port: 2222,
            user: "admin".into(),
            auth: Auth::Key { path: "~/.ssh/id_ed25519".into(), passphrase: Some("secret".into()) },
            tags: vec![], pinned: true,
            last_connected: None, connection_count: 0,
            bookmarks: vec![],
        };
        let es = EditState::from_server(&server, 0);
        assert_eq!(es.name, "editme");
        assert_eq!(es.port, "2222");
        assert_eq!(es.key_path, "~/.ssh/id_ed25519");
        assert_eq!(es.passphrase, "secret");
        assert!(es.is_key_auth());
    }

    #[test]
    fn test_edit_state_from_password_server() {
        let server = Server {
            name: "p".into(), host: "h".into(), port: 22, user: "u".into(),
            auth: Auth::Password { vault_key: "vk".into() },
            tags: vec![], pinned: false,
            last_connected: None, connection_count: 0,
            bookmarks: vec![],
        };
        let es = EditState::from_server(&server, 0);
        assert!(!es.is_key_auth());
        assert_eq!(es.auth_type, "password");
    }

    #[test]
    fn test_edit_state_current_value() {
        let mut es = EditState {
            field: EditField::Name, name: "n".into(), host: String::new(),
            port: "22".into(), user: String::new(), auth_type: "key".into(),
            key_path: String::new(), passphrase: String::new(), password: String::new(),
            server_index: 0,
        };
        assert_eq!(es.current_value(), "n");
        es.field = EditField::Port;
        assert_eq!(es.current_value(), "22");
    }

    #[test]
    fn test_edit_state_next_field_key() {
        let mut es = EditState {
            field: EditField::Name, name: String::new(), host: String::new(),
            port: "22".into(), user: String::new(), auth_type: "key".into(),
            key_path: String::new(), passphrase: String::new(), password: String::new(),
            server_index: 0,
        };
        // Full cycle through 7 key-auth fields returns to Name
        for _ in 0..7 { es.next_field(); }
        assert_eq!(es.field, EditField::Name);
    }

    #[test]
    fn test_edit_state_next_field_password() {
        let mut es = EditState {
            field: EditField::Name, name: String::new(), host: String::new(),
            port: "22".into(), user: String::new(), auth_type: "password".into(),
            key_path: String::new(), passphrase: String::new(), password: String::new(),
            server_index: 0,
        };
        es.next_field();
        assert_eq!(es.field, EditField::Host);
    }

    #[test]
    fn test_edit_state_prev_field() {
        let mut es = EditState {
            field: EditField::Passphrase, name: String::new(), host: String::new(),
            port: "22".into(), user: String::new(), auth_type: "key".into(),
            key_path: String::new(), passphrase: String::new(), password: String::new(),
            server_index: 0,
        };
        es.prev_field();
        assert_eq!(es.field, EditField::KeyPath);
    }

    #[test]
    fn test_edit_state_build_auth_key() {
        let es = EditState {
            field: EditField::Name, name: "n".into(), host: String::new(),
            port: "22".into(), user: String::new(), auth_type: "key".into(),
            key_path: "~/.ssh/custom".into(), passphrase: String::new(), password: String::new(),
            server_index: 0,
        };
        match es.build_auth() {
            Auth::Key { path, passphrase } => {
                assert_eq!(path, "~/.ssh/custom");
                assert!(passphrase.is_none());
            }
            _ => panic!("expected Key auth"),
        }
    }

    #[test]
    fn test_edit_state_build_auth_password() {
        let es = EditState {
            field: EditField::Password, name: String::new(), host: String::new(),
            port: "22".into(), user: String::new(), auth_type: "password".into(),
            key_path: String::new(), passphrase: String::new(), password: "vaultkey".into(),
            server_index: 0,
        };
        match es.build_auth() {
            Auth::Password { vault_key } => assert_eq!(vault_key, "vaultkey"),
            _ => panic!("expected Password auth"),
        }
    }

    #[test]
    fn test_edit_state_current_value_mut() {
        let mut es = EditState {
            field: EditField::Name, name: "old".into(), host: String::new(),
            port: "22".into(), user: String::new(), auth_type: "key".into(),
            key_path: String::new(), passphrase: String::new(), password: String::new(),
            server_index: 0,
        };
        *es.current_value_mut() = "new".to_string();
        assert_eq!(es.name, "new");
    }

    // --- InsertState ---

    #[test]
    fn test_insert_state_new_has_defaults() {
        let is = InsertState::new();
        assert_eq!(is.port, "22");
        assert_eq!(is.user, "root");
        assert_eq!(is.auth_type, "key");
        assert!(is.is_key_auth());
        assert_eq!(is.field, InsertField::Name);
    }

    #[test]
    fn test_insert_state_next_field() {
        let mut is = InsertState::new();
        is.next_field();
        assert_eq!(is.field, InsertField::Host);
    }

    #[test]
    fn test_insert_state_prev_field() {
        let mut is = InsertState {
            field: InsertField::Passphrase, name: String::new(), host: String::new(),
            port: "22".into(), user: String::new(), auth_type: "key".into(),
            key_path: String::new(), passphrase: String::new(), password: String::new(),
        };
        is.prev_field();
        assert_eq!(is.field, InsertField::KeyPath);
    }

    #[test]
    fn test_insert_state_current_value() {
        let is = InsertState::new();
        assert_eq!(is.current_value(), "");
        let mut is2 = InsertState::new();
        is2.field = InsertField::Port;
        assert_eq!(is2.current_value(), "22");
    }

    #[test]
    fn test_insert_state_current_value_mut() {
        let mut is = InsertState::new();
        *is.current_value_mut() = "newval".to_string();
        assert_eq!(is.name, "newval");
    }

    #[test]
    fn test_insert_state_build_auth_key_default_path() {
        let is = InsertState::new();
        match is.build_auth() {
            Auth::Key { path, passphrase } => {
                assert_eq!(path, "~/.ssh/id_rsa");
                assert!(passphrase.is_none());
            }
            _ => panic!("expected Key"),
        }
    }
}
