use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::config::models::Server;
use super::theme::Theme;

#[derive(Debug)]
pub enum SshStatus {
    Connecting,
    Connected,
    Error(String),
    Disconnected,
}

#[derive(Debug, Clone)]
pub struct Selection {
    pub start_row: usize,
    pub start_col: usize,
    pub end_row: usize,
    pub end_col: usize,
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
    pub selection: Option<Selection>,
    pub is_selecting: bool,
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
            selection: None,
            is_selecting: false,
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

    pub fn start_selection(&mut self, row: usize, col: usize) {
        self.is_selecting = true;
        self.selection = Some(Selection {
            start_row: row,
            start_col: col,
            end_row: row,
            end_col: col,
        });
    }

    pub fn update_selection(&mut self, row: usize, col: usize) {
        if let Some(sel) = &mut self.selection {
            sel.end_row = row;
            sel.end_col = col;
        }
    }

    pub fn end_selection(&mut self) {
        self.is_selecting = false;
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
        self.is_selecting = false;
    }

    pub fn get_selected_text(&self) -> Option<String> {
        let sel = self.selection.as_ref()?;
        let lines = &self.output;

        let start_row = sel.start_row.min(sel.end_row);
        let end_row = sel.start_row.max(sel.end_row);
        let start_col = if sel.start_row <= sel.end_row { sel.start_col } else { sel.end_col };
        let end_col = if sel.start_row <= sel.end_row { sel.end_col } else { sel.start_col };

        let mut selected = String::new();

        for i in start_row..=end_row.min(lines.len().saturating_sub(1)) {
            let line = &lines[i];
            let line_chars: Vec<char> = line.chars().collect();
            let line_len = line_chars.len();

            let col_start = if i == start_row { start_col.min(line_len) } else { 0 };
            let col_end = if i == end_row { end_col.min(line_len) } else { line_len };

            if col_start < line_len && col_end > col_start {
                let selected_part: String = line_chars[col_start..col_end].iter().collect();
                if !selected.is_empty() {
                    selected.push('\n');
                }
                selected.push_str(&selected_part);
            }
        }

        if selected.is_empty() {
            None
        } else {
            Some(selected)
        }
    }

    pub fn copy_selection_to_clipboard(&self) -> bool {
        if let Some(text) = self.get_selected_text() {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                clipboard.set_text(&text).is_ok()
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn is_selected(&self, row: usize, col: usize) -> bool {
        if let Some(sel) = &self.selection {
            // A row aqui é relativa ao output, não à tela
            let start_row = sel.start_row.min(sel.end_row);
            let end_row = sel.start_row.max(sel.end_row);

            if row < start_row || row > end_row {
                return false;
            }

            let (start_col, end_col) = if sel.start_row <= sel.end_row {
                (sel.start_col, sel.end_col)
            } else {
                (sel.end_col, sel.start_col)
            };

            if row == start_row && row == end_row {
                col >= start_col && col <= end_col
            } else if row == start_row {
                col >= start_col
            } else if row == end_row {
                col <= end_col
            } else {
                true
            }
        } else {
            false
        }
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
        SshStatus::Connecting => Theme::warning(),
        SshStatus::Connected => Theme::success(),
        SshStatus::Error(_) => Theme::error(),
        SshStatus::Disconnected => Theme::text_dim(),
    };

    let status_text = match &state.status {
        SshStatus::Connecting => "⏳ Conectando...",
        SshStatus::Connected => "✓ Conectado",
        SshStatus::Error(e) => e.as_str(),
        SshStatus::Disconnected => "○ Desconectado",
    };

    // Espaço útil = total - bordas(2) - linha do prompt(1)
    let usable_height = area.height.saturating_sub(3) as usize;

    // Construir linhas do output com scroll e seleção
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
        let line = &state.output[i];
        let line_chars: Vec<char> = line.chars().collect();
        let mut spans = vec![];

        // Renderizar caractere por caractere para suportar seleção
        // i é o índice absoluto no output, usado para verificar seleção
        for (col, &ch) in line_chars.iter().enumerate() {
            let is_selected = state.is_selected(i, col);
            let style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::White)
            } else {
                Style::default().fg(Color::White)
            };
            spans.push(Span::styled(ch.to_string(), style));
        }

        // Se a linha for vazia, adicionar algo para poder selecionar
        if line_chars.is_empty() {
            spans.push(Span::raw(" "));
        }

        output_lines.push(Line::from(spans));
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
                Span::styled(prompt_str, Theme::prompt_style()),
                Span::styled(before_cursor.to_string(), Style::default().fg(Theme::text())),
            ];

            if !after_cursor.is_empty() {
                let mut chars = after_cursor.chars();
                if let Some(cursor_char) = chars.next() {
                    spans.push(Span::styled(
                        cursor_char.to_string(),
                        Style::default().fg(Color::Black).bg(Theme::primary()),
                    ));
                    let remaining: String = chars.collect();
                    if !remaining.is_empty() {
                        spans.push(Span::styled(remaining, Style::default().fg(Theme::text())));
                    }
                }
            } else {
                spans.push(Span::styled("█", Style::default().fg(Theme::primary())));
            }

            output_lines.push(Line::from(spans));
        }
        SshStatus::Connecting => {
            output_lines.push(Line::from(Span::styled(
                "⏳ Conectando...",
                Style::default().fg(Theme::warning()),
            )));
        }
        SshStatus::Error(_) => {
            output_lines.push(Line::from(Span::styled(
                "Pressione 'q' ou Esc para voltar",
                Theme::error_style(),
            )));
        }
        SshStatus::Disconnected => {
            output_lines.push(Line::from(Span::styled(
                "○ Desconectado. Pressione 'q' para voltar.",
                Theme::dim_style(),
            )));
        }
    }

    // Indicador de scroll e seleção
    let mut indicators = vec![];
    if state.scroll_offset > 0 {
        indicators.push(format!("↑{}", state.scroll_offset));
    }
    if state.selection.is_some() {
        indicators.push("📋".to_string());
    }
    let indicator_str = if indicators.is_empty() {
        String::new()
    } else {
        format!(" [{}]", indicators.join(" "))
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(
            " 🖥 {} [{}]{} ",
            state.server_name, status_text, indicator_str
        ))
        .title_style(Style::default().fg(status_color).add_modifier(ratatui::style::Modifier::BOLD))
        .border_style(Theme::border_style());

    let paragraph = Paragraph::new(output_lines).block(block);
    f.render_widget(paragraph, area);
}
