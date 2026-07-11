use crate::config::models::{Auth, Server};
use crate::ssh::SshService;
use crate::sftp::{FileInfo, SftpService};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use tokio::sync::mpsc;
use super::effects::AppEffects;
use super::notifications::NotificationQueue;
use super::sftp_browser::SftpState;

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

    pub fn server_index(&self) -> Option<usize> {
        match self.mode {
            FormMode::Edit { server_index } => Some(server_index),
            FormMode::Insert => None,
        }
    }

    pub fn is_key_auth(&self) -> bool {
        self.auth_type == "key"
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
                FormField::Password => FormField::Name,
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
                FormField::KeyPath | FormField::Passphrase => FormField::Name,
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
                FormField::Password => FormField::Name,
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
                FormField::KeyPath | FormField::Passphrase => FormField::Name,
            }
        };
    }

    pub fn build_auth(&self) -> Auth {
        if self.auth_type == "password" {
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

#[derive(Debug, PartialEq)]
pub enum Overlay {
    Help,
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
    pub form_state: Option<FormState>,
    pub notifications: NotificationQueue,
    pub effects: AppEffects,
    pub ssh_service: SshService,
    pub sftp_service: std::sync::Arc<std::sync::Mutex<SftpService>>,
    pub sftp_progress_rx: Option<mpsc::UnboundedReceiver<u64>>,
    pub sftp_op_rx: Option<mpsc::UnboundedReceiver<SftpOpResult>>,
    pub overlay: Option<Overlay>,
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
            form_state: None,
            notifications,
            effects: AppEffects::new(),
            ssh_service: SshService::new(),
            sftp_service: std::sync::Arc::new(std::sync::Mutex::new(SftpService::new())),
            sftp_progress_rx: None,
            overlay: None,
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

    pub fn import_ssh_config(&mut self) {
        let home = std::env::var("HOME").unwrap_or_default();
        let config_path = std::path::PathBuf::from(format!("{}/.ssh/config", home));
        let imported = crate::config::ssh_config::parse_ssh_config(&config_path);

        if imported.is_empty() {
            self.notifications.warning("Nenhum servidor encontrado em ~/.ssh/config");
            return;
        }

        let mut added = 0;
        let mut skipped = 0;
        for server in imported {
            let key = format!("{}:{}:{}", server.host, server.port, server.user);
            let exists = self.servers.iter().any(|s| {
                format!("{}:{}:{}", s.host, s.port, s.user) == key
            });
            if exists {
                skipped += 1;
            } else {
                self.servers.push(server);
                added += 1;
            }
        }

        if added > 0 {
            self.filtered_indices = (0..self.servers.len()).collect();
            self.selected = 0;
            if let Err(e) = crate::config::save_config(
                &crate::config::AppConfig {
                    servers: self.servers.clone(),
                    sort_by: None,
                },
                &crate::config::get_config_path(),
            ) {
                self.notifications.warning(&format!("Falha ao salvar config: {}", e));
            }
        }

        self.notifications.info(&format!(
            "Importados {} servidor(es), {} ignorado(s)",
            added, skipped
        ));
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

    /// Upload a file via SCP (no ~1GB SFTP limit).
    pub fn sftp_upload_with_scp(&self, server: &crate::config::models::Server, local: &str, remote: &str) -> Result<(), String> {
        let mut args = vec![];

        if server.port != 22 {
            args.push("-P".to_string());
            args.push(server.port.to_string());
        }

        match &server.auth {
            crate::config::models::Auth::Key { path, .. } => {
                let expanded = shellexpand::tilde(path).into_owned();
                args.push("-i".to_string());
                args.push(expanded);
            }
            crate::config::models::Auth::Password { vault_key } => {
                if !vault_key.is_empty() {
                    // sshpass for password auth
                    args.insert(0, "sshpass".to_string());
                    args.insert(1, "-p".to_string());
                    args.insert(2, vault_key.clone());
                    args.insert(3, "scp".to_string());
                }
            }
        }

        args.push(local.to_string());
        args.push(format!("{}@{}:{}", server.user, server.host, remote));

        let cmd = if args[0] == "sshpass" { "sshpass" } else { "scp" };
        let status = std::process::Command::new(cmd)
            .args(&args[if cmd == "sshpass" { 1.. } else { 0.. }])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .status()
            .map_err(|e| format!("Falha ao executar SCP: {}", e))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("SCP falhou com código {}", status.code().unwrap_or(-1)))
        }
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
        // Not used — SSH opens in external shell via native_shell_handoff
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
            agent_forwarding: false,
            proxy_jump: None,


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
            agent_forwarding: false,
            proxy_jump: None,


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

    // --- FormState (Edit) ---

    #[test]
    fn test_form_state_edit_from_key_server() {
        let server = Server {
            name: "editme".into(), host: "10.0.0.1".into(), port: 2222,
            user: "admin".into(),
            auth: Auth::Key { path: "~/.ssh/id_ed25519".into(), passphrase: Some("secret".into()) },
            tags: vec![], pinned: true,
            last_connected: None, connection_count: 0,
            bookmarks: vec![],
            agent_forwarding: false,
            proxy_jump: None,
        };
        let fs = FormState::new_edit(&server, 0);
        assert_eq!(fs.name, "editme");
        assert_eq!(fs.port, "2222");
        assert_eq!(fs.key_path, "~/.ssh/id_ed25519");
        assert_eq!(fs.passphrase, "secret");
        assert!(fs.is_key_auth());
        assert_eq!(fs.server_index(), Some(0));
    }

    #[test]
    fn test_form_state_edit_from_password_server() {
        let server = Server {
            name: "p".into(), host: "h".into(), port: 22, user: "u".into(),
            auth: Auth::Password { vault_key: "vk".into() },
            tags: vec![], pinned: false,
            last_connected: None, connection_count: 0,
            bookmarks: vec![],
            agent_forwarding: false,
            proxy_jump: None,
        };
        let fs = FormState::new_edit(&server, 0);
        assert!(!fs.is_key_auth());
        assert_eq!(fs.auth_type, "password");
    }

    #[test]
    fn test_form_state_current_value() {
        let mut fs = FormState {
            field: FormField::Name, mode: FormMode::Insert, name: "n".into(), host: String::new(),
            port: "22".into(), user: String::new(), auth_type: "key".into(),
            key_path: String::new(), passphrase: String::new(), password: String::new(),
            tags: String::new(),
        };
        assert_eq!(fs.current_value(), "n");
        fs.field = FormField::Port;
        assert_eq!(fs.current_value(), "22");
    }

    #[test]
    fn test_form_state_next_field_key() {
        let mut fs = FormState {
            field: FormField::Name, mode: FormMode::Insert, name: String::new(), host: String::new(),
            port: "22".into(), user: String::new(), auth_type: "key".into(),
            key_path: String::new(), passphrase: String::new(), password: String::new(),
            tags: String::new(),
        };
        // Full cycle through 8 key-auth fields returns to Name
        for _ in 0..8 { fs.next_field(); }
        assert_eq!(fs.field, FormField::Name);
    }

    #[test]
    fn test_form_state_next_field_password() {
        let mut fs = FormState {
            field: FormField::Name, mode: FormMode::Insert, name: String::new(), host: String::new(),
            port: "22".into(), user: String::new(), auth_type: "password".into(),
            key_path: String::new(), passphrase: String::new(), password: String::new(),
            tags: String::new(),
        };
        fs.next_field();
        assert_eq!(fs.field, FormField::Host);
    }

    #[test]
    fn test_form_state_prev_field() {
        let mut fs = FormState {
            field: FormField::Passphrase, mode: FormMode::Insert, name: String::new(), host: String::new(),
            port: "22".into(), user: String::new(), auth_type: "key".into(),
            key_path: String::new(), passphrase: String::new(), password: String::new(),
            tags: String::new(),
        };
        fs.prev_field();
        assert_eq!(fs.field, FormField::KeyPath);
    }

    #[test]
    fn test_form_state_build_auth_key() {
        let fs = FormState {
            field: FormField::Name, mode: FormMode::Insert, name: "n".into(), host: String::new(),
            port: "22".into(), user: String::new(), auth_type: "key".into(),
            key_path: "~/.ssh/custom".into(), passphrase: String::new(), password: String::new(),
            tags: String::new(),
        };
        match fs.build_auth() {
            Auth::Key { path, passphrase } => {
                assert_eq!(path, "~/.ssh/custom");
                assert!(passphrase.is_none());
            }
            _ => panic!("expected Key auth"),
        }
    }

    #[test]
    fn test_form_state_build_auth_password() {
        let fs = FormState {
            field: FormField::Password, mode: FormMode::Insert, name: String::new(), host: String::new(),
            port: "22".into(), user: String::new(), auth_type: "password".into(),
            key_path: String::new(), passphrase: String::new(), password: "vaultkey".into(),
            tags: String::new(),
        };
        match fs.build_auth() {
            Auth::Password { vault_key } => assert_eq!(vault_key, "vaultkey"),
            _ => panic!("expected Password auth"),
        }
    }

    #[test]
    fn test_form_state_current_value_mut() {
        let mut fs = FormState {
            field: FormField::Name, mode: FormMode::Insert, name: "old".into(), host: String::new(),
            port: "22".into(), user: String::new(), auth_type: "key".into(),
            key_path: String::new(), passphrase: String::new(), password: String::new(),
            tags: String::new(),
        };
        *fs.current_value_mut() = "new".to_string();
        assert_eq!(fs.name, "new");
    }

    // --- FormState (Insert) ---

    #[test]
    fn test_form_state_insert_new_has_defaults() {
        let fs = FormState::new_insert();
        assert_eq!(fs.port, "22");
        assert_eq!(fs.user, "root");
        assert_eq!(fs.auth_type, "key");
        assert!(fs.is_key_auth());
        assert_eq!(fs.field, FormField::Name);
        assert_eq!(fs.server_index(), None);
    }

    #[test]
    fn test_form_state_insert_next_field() {
        let mut fs = FormState::new_insert();
        fs.next_field();
        assert_eq!(fs.field, FormField::Host);
    }

    #[test]
    fn test_form_state_insert_prev_field() {
        let mut fs = FormState {
            field: FormField::Passphrase, mode: FormMode::Insert, name: String::new(), host: String::new(),
            port: "22".into(), user: String::new(), auth_type: "key".into(),
            key_path: String::new(), passphrase: String::new(), password: String::new(),
            tags: String::new(),
        };
        fs.prev_field();
        assert_eq!(fs.field, FormField::KeyPath);
    }

    #[test]
    fn test_form_state_insert_current_value() {
        let fs = FormState::new_insert();
        assert_eq!(fs.current_value(), "");
        let mut fs2 = FormState::new_insert();
        fs2.field = FormField::Port;
        assert_eq!(fs2.current_value(), "22");
    }

    #[test]
    fn test_form_state_insert_current_value_mut() {
        let mut fs = FormState::new_insert();
        *fs.current_value_mut() = "newval".to_string();
        assert_eq!(fs.name, "newval");
    }

    #[test]
    fn test_form_state_insert_build_auth_key_default_path() {
        let fs = FormState::new_insert();
        match fs.build_auth() {
            Auth::Key { path, passphrase } => {
                assert_eq!(path, "~/.ssh/id_rsa");
                assert!(passphrase.is_none());
            }
            _ => panic!("expected Key"),
        }
    }

}
