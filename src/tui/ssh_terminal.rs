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
    pub cursor_pos: usize,
    pub scroll_offset: usize,
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
            cursor_pos: 0,
            scroll_offset: 0,
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

    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor_pos, c);
        self.cursor_pos += 1;
    }

    pub fn delete_char_backward(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.input.remove(self.cursor_pos);
        }
    }

    pub fn delete_char_forward(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.input.remove(self.cursor_pos);
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.cursor_pos += 1;
        }
    }

    pub fn move_cursor_home(&mut self) {
        self.cursor_pos = 0;
    }

    pub fn move_cursor_end(&mut self) {
        self.cursor_pos = self.input.len();
    }

    pub fn clear_input(&mut self) {
        self.input.clear();
        self.cursor_pos = 0;
    }

    pub fn scroll_up(&mut self, lines: usize) {
        let max_scroll = self.output.len().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + lines).min(max_scroll);
    }

    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    pub fn scroll_page_up(&mut self, page_size: usize) {
        self.scroll_up(page_size);
    }

    pub fn scroll_page_down(&mut self, page_size: usize) {
        self.scroll_down(page_size);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn prompt(&self) -> String {
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

    // Espaço útil = total - bordas(2) - linha do prompt(1)
    let usable_height = area.height.saturating_sub(3) as usize;

    // Construir linhas do output com scroll
    let mut output_lines: Vec<Line> = vec![];
    let total_output = state.output.len();

    // Calcular quantas linhas mostrar
    let lines_to_show = if total_output > usable_height {
        usable_height
    } else {
        total_output
    };

    // Calcular índice inicial baseado no scroll
    let end_idx = total_output.saturating_sub(state.scroll_offset);
    let start_idx = end_idx.saturating_sub(lines_to_show);

    for i in start_idx..end_idx {
        output_lines.push(Line::from(Span::raw(&state.output[i])));
    }

    // Adicionar linhas vazias para preencher se output for menor que altura
    while output_lines.len() < usable_height {
        output_lines.insert(0, Line::from(""));
    }

    // Adicionar linha do prompt/input
    match &state.status {
        SshStatus::Connected => {
            let prompt = state.prompt();
            let prompt_str = prompt.clone();

            let before_cursor = &state.input[..state.cursor_pos];
            let after_cursor = &state.input[state.cursor_pos..];

            let mut spans = vec![
                Span::styled(prompt_str, Style::default().fg(Color::Green)),
                Span::styled(before_cursor.to_string(), Style::default().fg(Color::White)),
            ];

            if !after_cursor.is_empty() {
                let mut chars = after_cursor.chars();
                if let Some(cursor_char) = chars.next() {
                    spans.push(Span::styled(
                        cursor_char.to_string(),
                        Style::default().fg(Color::Black).bg(Color::Green),
                    ));
                    let remaining: String = chars.collect();
                    if !remaining.is_empty() {
                        spans.push(Span::styled(remaining, Style::default().fg(Color::White)));
                    }
                }
            } else {
                spans.push(Span::styled("█", Style::default().fg(Color::Green)));
            }

            output_lines.push(Line::from(spans));
        }
        SshStatus::Connecting => {
            output_lines.push(Line::from(Span::styled(
                "Conectando...",
                Style::default().fg(Color::Yellow),
            )));
        }
        SshStatus::Error(_) => {
            output_lines.push(Line::from(Span::styled(
                "Pressione 'q' ou Esc para voltar",
                Style::default().fg(Color::Red),
            )));
        }
        SshStatus::Disconnected => {
            output_lines.push(Line::from(Span::styled(
                "Desconectado. Pressione 'q' para voltar.",
                Style::default().fg(Color::Gray),
            )));
        }
    }

    // Indicador de scroll
    let scroll_indicator = if state.scroll_offset > 0 {
        format!(" [↑{}]", state.scroll_offset)
    } else {
        String::new()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(
            " {} [{}]{} ",
            state.server_name, status_text, scroll_indicator
        ))
        .title_style(Style::default().fg(status_color));

    let paragraph = Paragraph::new(output_lines).block(block);
    f.render_widget(paragraph, area);
}
