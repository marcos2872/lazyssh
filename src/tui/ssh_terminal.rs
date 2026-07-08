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

    // Construir todas as linhas do terminal
    let mut lines: Vec<Line> = vec![];

    // Calcular quantas linhas cabem na tela (área útil - bordas - 1 para prompt)
    let visible_height = area.height.saturating_sub(2) as usize; // -2 para bordas

    // Adicionar output existente com scroll
    let total_lines = state.output.len();
    let start_idx = if total_lines > visible_height {
        total_lines.saturating_sub(visible_height + state.scroll_offset)
    } else {
        0
    };

    for i in start_idx..total_lines {
        lines.push(Line::from(Span::raw(&state.output[i])));
    }

    // Adicionar linha de comando atual (prompt + input)
    match &state.status {
        SshStatus::Connected => {
            let prompt = state.prompt();
            let prompt_str = prompt.clone();

            // Texto antes do cursor
            let before_cursor = &state.input[..state.cursor_pos];
            // Texto depois do cursor (se houver)
            let after_cursor = &state.input[state.cursor_pos..];

            let mut spans = vec![
                Span::styled(prompt_str, Style::default().fg(Color::Green)),
                Span::styled(before_cursor.to_string(), Style::default().fg(Color::White)),
            ];

            // Se cursor está no meio, mostra caractere sob cursor + resto
            if !after_cursor.is_empty() {
                let mut chars = after_cursor.chars();
                if let Some(cursor_char) = chars.next() {
                    // Caractere sob o cursor (invertido)
                    spans.push(Span::styled(
                        cursor_char.to_string(),
                        Style::default().fg(Color::Black).bg(Color::Green),
                    ));
                    // Resto do texto
                    let remaining: String = chars.collect();
                    if !remaining.is_empty() {
                        spans.push(Span::styled(remaining, Style::default().fg(Color::White)));
                    }
                }
            } else {
                // Cursor no final - mostra bloco
                spans.push(Span::styled("█", Style::default().fg(Color::Green)));
            }

            lines.push(Line::from(spans));
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

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}
