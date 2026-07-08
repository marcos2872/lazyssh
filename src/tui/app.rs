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

    pub fn filter(&mut self, query: &str) {
        if query.is_empty() {
            self.filtered_indices = (0..self.servers.len()).collect();
        } else {
            let query_lower = query.to_lowercase();
            self.filtered_indices = self
                .servers
                .iter()
                .enumerate()
                .filter(|(_, s)| {
                    s.name.to_lowercase().contains(&query_lower)
                        || s.host.to_lowercase().contains(&query_lower)
                        || s.tags
                            .iter()
                            .any(|t| t.to_lowercase().contains(&query_lower))
                })
                .map(|(i, _)| i)
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
