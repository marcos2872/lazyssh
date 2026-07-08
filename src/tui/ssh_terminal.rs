use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::config::models::Server;

#[derive(Debug)]
pub enum SshStatus {
    Connecting,
    Connected,
    Error(String),
    Disconnected,
}

#[derive(Debug)]
pub struct SshTerminalState {
    pub server_name: String,
    pub server: Option<Server>,
    pub output: Vec<String>,
    pub input: String,
    pub status: SshStatus,
    pub session_id: Option<String>,
}

impl SshTerminalState {
    pub fn new(server: Server) -> Self {
        let name = server.name.clone();
        Self {
            server_name: name,
            server: Some(server),
            output: vec![],
            input: String::new(),
            status: SshStatus::Connecting,
            session_id: None,
        }
    }

    pub fn add_output(&mut self, line: String) {
        self.output.push(line);
    }

    pub fn set_connected(&mut self, session_id: String) {
        self.status = SshStatus::Connected;
        self.session_id = Some(session_id);
    }

    pub fn set_error(&mut self, error: String) {
        self.status = SshStatus::Error(error.clone());
        self.output.push(format!("Erro: {}", error));
    }

    pub fn set_disconnected(&mut self) {
        self.status = SshStatus::Disconnected;
    }

    fn prompt(&self) -> String {
        if let Some(server) = &self.server {
            format!("{}@{}:~$ ", server.user, server.host)
        } else {
            "$ ".to_string()
        }
    }
}

pub fn render_ssh_terminal(f: &mut Frame, state: &SshTerminalState) {
    let area = f.area();

    // Status bar no topo
    let status_color = match &state.status {
        SshStatus::Connecting => Color::Yellow,
        SshStatus::Connected => Color::Green,
        SshStatus::Error(_) => Color::Red,
        SshStatus::Disconnected => Color::Gray,
    };

    let status_text = match &state.status {
        SshStatus::Connecting => "Conectando...",
        SshStatus::Connected => "Conectado",
        SshStatus::Error(e) => e.as_str(),
        SshStatus::Disconnected => "Desconectado",
    };

    // Construir todas as linhas do terminal
    let mut lines: Vec<Line> = vec![];

    // Adicionar output existente
    for line in &state.output {
        lines.push(Line::from(Span::raw(line)));
    }

    // Adicionar linha de comando atual (prompt + input)
    match &state.status {
        SshStatus::Connected => {
            let prompt = state.prompt();
            let prompt_str = prompt.clone();
            lines.push(Line::from(vec![
                Span::styled(prompt_str, Style::default().fg(Color::Green)),
                Span::styled(&state.input, Style::default().fg(Color::White)),
                Span::styled("█", Style::default().fg(Color::Green)),
            ]));
        }
        SshStatus::Connecting => {
            lines.push(Line::from(Span::styled(
                "Conectando...",
                Style::default().fg(Color::Yellow),
            )));
        }
        SshStatus::Error(_) => {
            lines.push(Line::from(Span::styled(
                "Pressione 'q' ou Esc para voltar",
                Style::default().fg(Color::Red),
            )));
        }
        SshStatus::Disconnected => {
            lines.push(Line::from(Span::styled(
                "Desconectado. Pressione 'q' para voltar.",
                Style::default().fg(Color::Gray),
            )));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} [{}] ", state.server_name, status_text))
        .title_style(Style::default().fg(status_color));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}
