use crate::config::models::Server;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use super::sftp_browser::SftpState;
use super::ssh_terminal::SshTerminalState;

#[derive(Debug, Clone, PartialEq)]
pub enum EditField {
    Name,
    Host,
    Port,
    User,
}

#[derive(Debug, Clone)]
pub struct EditState {
    pub field: EditField,
    pub name: String,
    pub host: String,
    pub port: String,
    pub user: String,
    pub server_index: usize,
}

impl EditState {
    pub fn from_server(server: &Server, index: usize) -> Self {
        Self {
            field: EditField::Name,
            name: server.name.clone(),
            host: server.host.clone(),
            port: server.port.to_string(),
            user: server.user.clone(),
            server_index: index,
        }
    }

    pub fn current_value(&self) -> &str {
        match self.field {
            EditField::Name => &self.name,
            EditField::Host => &self.host,
            EditField::Port => &self.port,
            EditField::User => &self.user,
        }
    }

    pub fn current_value_mut(&mut self) -> &mut String {
        match self.field {
            EditField::Name => &mut self.name,
            EditField::Host => &mut self.host,
            EditField::Port => &mut self.port,
            EditField::User => &mut self.user,
        }
    }

    pub fn next_field(&mut self) {
        self.field = match self.field {
            EditField::Name => EditField::Host,
            EditField::Host => EditField::Port,
            EditField::Port => EditField::User,
            EditField::User => EditField::Name,
        };
    }

    pub fn field_label(&self) -> &str {
        match self.field {
            EditField::Name => "Nome",
            EditField::Host => "Host",
            EditField::Port => "Porta",
            EditField::User => "Usuário",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InsertField {
    Name,
    Host,
    Port,
    User,
}

#[derive(Debug, Clone)]
pub struct InsertState {
    pub field: InsertField,
    pub name: String,
    pub host: String,
    pub port: String,
    pub user: String,
}

impl InsertState {
    pub fn new() -> Self {
        Self {
            field: InsertField::Name,
            name: String::new(),
            host: String::new(),
            port: "22".to_string(),
            user: "root".to_string(),
        }
    }

    pub fn current_value(&self) -> &str {
        match self.field {
            InsertField::Name => &self.name,
            InsertField::Host => &self.host,
            InsertField::Port => &self.port,
            InsertField::User => &self.user,
        }
    }

    pub fn current_value_mut(&mut self) -> &mut String {
        match self.field {
            InsertField::Name => &mut self.name,
            InsertField::Host => &mut self.host,
            InsertField::Port => &mut self.port,
            InsertField::User => &mut self.user,
        }
    }

    pub fn next_field(&mut self) {
        self.field = match self.field {
            InsertField::Name => InsertField::Host,
            InsertField::Host => InsertField::Port,
            InsertField::Port => InsertField::User,
            InsertField::User => InsertField::Name,
        };
    }

    pub fn field_label(&self) -> &str {
        match self.field {
            InsertField::Name => "Nome",
            InsertField::Host => "Host",
            InsertField::Port => "Porta",
            InsertField::User => "Usuário",
        }
    }
}

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

#[derive(Debug)]
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
            ssh_state: None,
            insert_state: None,
            edit_state: None,
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
            self.filtered_indices = self.servers.iter()
                .enumerate()
                .filter_map(|(i, s)| {
                    let score = matcher.fuzzy_match(&s.name, query)
                        .or_else(|| matcher.fuzzy_match(&s.host, query))
                        .or_else(|| s.tags.iter().find_map(|t| matcher.fuzzy_match(t, query)));
                    score.map(|_| i)
                })
                .collect();
        }
        self.selected = 0;
    }

    pub fn open_sftp(&mut self) {
        self.current_view = CurrentView::SftpBrowser;
        self.sftp_state = Some(SftpState::new());
    }

    pub fn close_sftp(&mut self) {
        self.current_view = CurrentView::ServerList;
        self.sftp_state = None;
    }

    pub fn connect_ssh(&mut self) {
        let server_name = self
            .selected_server()
            .map(|s| s.name.clone());
        if let Some(name) = server_name {
            self.current_view = CurrentView::SshTerminal;
            self.ssh_state = Some(SshTerminalState::new(name));
        }
    }

    pub fn close_ssh(&mut self) {
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
                auth: Auth::Key { path: "~/.ssh/id_rsa".to_string(), passphrase: None },
                tags: vec!["prod".to_string()],
                pinned: false,
            },
            Server {
                name: "server2".to_string(),
                host: "192.168.1.2".to_string(),
                port: 22,
                user: "user".to_string(),
                auth: Auth::Password { vault_key: "key".to_string() },
                tags: vec!["dev".to_string()],
                pinned: true,
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
}
