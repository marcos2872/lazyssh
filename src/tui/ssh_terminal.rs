use ratatui::{
    layout::{Constraint, Layout},
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
            output: vec!["Conectando...".to_string()],
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
        self.output.push("Conectado com sucesso!".to_string());
        self.output.push("Digite comandos ou 'exit' para desconectar.".to_string());
    }

    pub fn set_error(&mut self, error: String) {
        self.status = SshStatus::Error(error.clone());
        self.output.push(format!("Erro: {}", error));
    }

    pub fn set_disconnected(&mut self) {
        self.status = SshStatus::Disconnected;
        self.output.push("Desconectado.".to_string());
    }
}

pub fn render_ssh_terminal(f: &mut Frame, state: &SshTerminalState) {
    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(f.area());

    // Status bar
    let status_color = match &state.status {
        SshStatus::Connecting => Color::Yellow,
        SshStatus::Connected => Color::Green,
        SshStatus::Error(_) => Color::Red,
        SshStatus::Disconnected => Color::Gray,
    };

    let status_text = match &state.status {
        SshStatus::Connecting => "Conectando...",
        SshStatus::Connected => "Conectado",
        SshStatus::Error(e) => e,
        SshStatus::Disconnected => "Desconectado",
    };

    let output_text: Vec<Line> = state
        .output
        .iter()
        .map(|line| Line::from(Span::raw(line)))
        .collect();

    let output = Paragraph::new(output_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("SSH: {} [{}]", state.server_name, status_text))
            .title_style(Style::default().fg(status_color)),
    );
    f.render_widget(output, chunks[0]);

    let input = Paragraph::new(state.input.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Comando"),
    );
    f.render_widget(input, chunks[1]);
}
