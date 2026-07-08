use ratatui::{
    layout::{Constraint, Layout},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Debug)]
pub struct SshTerminalState {
    pub server_name: String,
    pub output: Vec<String>,
    pub input: String,
}

impl SshTerminalState {
    pub fn new(server_name: String) -> Self {
        Self {
            server_name,
            output: vec!["Conectado ao servidor...".to_string()],
            input: String::new(),
        }
    }
}

pub fn render_ssh_terminal(f: &mut Frame, state: &SshTerminalState) {
    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(f.area());

    let output_text: Vec<Line> = state
        .output
        .iter()
        .map(|line| Line::from(Span::raw(line)))
        .collect();

    let output = Paragraph::new(output_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("SSH: {}", state.server_name)),
    );
    f.render_widget(output, chunks[0]);

    let input = Paragraph::new(state.input.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Comando"),
    );
    f.render_widget(input, chunks[1]);
}
