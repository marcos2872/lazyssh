use std::cell::Cell;
use std::sync::Arc;
use tokio::sync::Mutex;

use ratatui::{
    style::{Color, Modifier, Style},
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

pub struct SshTerminalState {
    pub server_name: String,
    pub server: Option<Server>,
    pub output: Vec<String>,
    /// Buffer de linha corrente para acumular output caractere a caractere.
    pub current_line: String,
    pub scroll_offset: usize,
    pub status: SshStatus,
    pub session_id: Option<String>,
    pub selection: Option<Selection>,
    pub is_selecting: bool,
    pub shell_writer: Option<Arc<Mutex<Box<dyn tokio::io::AsyncWrite + Send + Unpin>>>>,
    /// Clipboard reutilizado entre cópias (evita warning de drop precoce no Linux).
    pub clipboard: Option<arboard::Clipboard>,
    /// Index into `output` of the first visible line (updated during render).
    pub first_visible_line: Cell<usize>,
    /// Number of empty padding lines at the top of the content area (updated during render).
    pub padding_top: Cell<usize>,
}

impl SshTerminalState {
    pub fn new(server: Server) -> Self {
        let name = server.name.clone();
        Self {
            server_name: name,
            server: Some(server),
            output: vec![],
            current_line: String::new(),
            scroll_offset: 0,
            status: SshStatus::Connecting,
            session_id: None,
            selection: None,
            is_selecting: false,
            shell_writer: None,
            clipboard: None,
            first_visible_line: Cell::new(0),
            padding_top: Cell::new(0),
        }
    }

    pub fn add_output(&mut self, line: String) {
        self.output.push(line);
    }

    /// Processa texto recebido do PTY caractere por caractere,
    /// tratando `\r`, `\n`, `\b` para construir as linhas corretamente.
    pub fn feed_output(&mut self, text: &str) {
        let mut chars = text.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                '\r' => {
                    // Salva linha atual no histórico
                    self.output.push(std::mem::take(&mut self.current_line));
                    // Se vier \r\n, descarta o \n (a linha já foi salva)
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                }
                '\n' => {
                    // \n solitário
                    self.output.push(std::mem::take(&mut self.current_line));
                }
                '\x08' => {
                    // Backspace: apaga último caractere da linha corrente
                    self.current_line.pop();
                }
                c => {
                    self.current_line.push(c);
                }
            }
        }
    }

    /// Chama feed_output e em seguida descarta a linha corrente
    /// (usada para descarregar o buffer ao desconectar).
    pub fn flush_output(&mut self) {
        if !self.current_line.is_empty() {
            self.output.push(std::mem::take(&mut self.current_line));
        }
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

    pub fn copy_selection_to_clipboard(&mut self) -> bool {
        let text = match self.get_selected_text() {
            Some(t) => t,
            None => return false,
        };

        // Tentar wl-copy (Wayland), xclip (X11), ou arboard como fallback
        if let Ok(mut child) = std::process::Command::new("wl-copy")
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            use std::io::Write;
            let result = child
                .stdin
                .take()
                .and_then(|mut stdin| stdin.write_all(text.as_bytes()).ok())
                .and_then(|_| child.wait().ok())
                .map(|status| status.success())
                .unwrap_or(false);
            if result {
                return true;
            }
        }

        // Fallback: xclip
        if let Ok(mut child) = std::process::Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            use std::io::Write;
            let result = child
                .stdin
                .take()
                .and_then(|mut stdin| stdin.write_all(text.as_bytes()).ok())
                .and_then(|_| child.wait().ok())
                .map(|status| status.success())
                .unwrap_or(false);
            if result {
                return true;
            }
        }

        // Fallback: arboard
        if self.clipboard.is_none() {
            self.clipboard = arboard::Clipboard::new().ok();
        }
        match &mut self.clipboard {
            Some(cb) => cb.set_text(&text).is_ok(),
            None => false,
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
}

/// Parse a string with ANSI escape codes into styled spans.
fn parse_ansi_spans(text: &str) -> Vec<(String, Style)> {
    let mut segments: Vec<(String, Style)> = Vec::new();
    let mut current = Style::default().fg(Color::White);
    let mut chars = text.chars();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            match chars.next() {
                Some('[') => {
                    // Collect CSI parameters until we hit a command letter
                    let mut params = String::new();
                    let terminator = loop {
                        match chars.next() {
                            Some(t) if ('@'..='~').contains(&t) => break t,
                            Some(p) => params.push(p),
                            None => break '\0',
                        }
                    };
                    // Only handle SGR (command letter 'm')
                    if terminator == 'm' {
                        // Apply SGR parameters
                        if params.is_empty() || params == "0" || params == "00" {
                            current = Style::default().fg(Color::White);
                        } else {
                            for param in params.split(';') {
                                match param {
                                    "0" | "00" => current = Style::default().fg(Color::White),
                                    "1" | "01" => current = current.add_modifier(Modifier::BOLD),
                                    "3" => current = current.add_modifier(Modifier::ITALIC),
                                    "4" => current = current.add_modifier(Modifier::UNDERLINED),
                                    "22" => current = current.remove_modifier(Modifier::BOLD),
                                    "23" => current = current.remove_modifier(Modifier::ITALIC),
                                    "24" => current = current.remove_modifier(Modifier::UNDERLINED),
                                    "30" => current = current.fg(Color::Black),
                                    "31" => current = current.fg(Color::Red),
                                    "32" => current = current.fg(Color::Green),
                                    "33" => current = current.fg(Color::Yellow),
                                    "34" => current = current.fg(Color::Blue),
                                    "35" => current = current.fg(Color::Magenta),
                                    "36" => current = current.fg(Color::Cyan),
                                    "37" => current = current.fg(Color::White),
                                    "40" => current = current.bg(Color::Black),
                                    "41" => current = current.bg(Color::Red),
                                    "42" => current = current.bg(Color::Green),
                                    "43" => current = current.bg(Color::Yellow),
                                    "44" => current = current.bg(Color::Blue),
                                    "45" => current = current.bg(Color::Magenta),
                                    "46" => current = current.bg(Color::Cyan),
                                    "47" => current = current.bg(Color::White),
                                    "90" => current = current.fg(Color::DarkGray),
                                    "91" => current = current.fg(Color::LightRed),
                                    "92" => current = current.fg(Color::LightGreen),
                                    "93" => current = current.fg(Color::LightYellow),
                                    "94" => current = current.fg(Color::LightBlue),
                                    "95" => current = current.fg(Color::LightMagenta),
                                    "96" => current = current.fg(Color::LightCyan),
                                    "97" => current = current.fg(Color::White),
                                    "100" => current = current.bg(Color::DarkGray),
                                    "101" => current = current.bg(Color::LightRed),
                                    "102" => current = current.bg(Color::LightGreen),
                                    "103" => current = current.bg(Color::LightYellow),
                                    "104" => current = current.bg(Color::LightBlue),
                                    "105" => current = current.bg(Color::LightMagenta),
                                    "106" => current = current.bg(Color::LightCyan),
                                    "107" => {
                                        current = current.bg(Color::White);
                                        current = current.add_modifier(Modifier::BOLD);
                                    }
                                    _ => {
                                        // Try 256-color: 38;5;N or 48;5;N
                                        if param.starts_with("38;5;") {
                                            if let Ok(n) = param[5..].parse::<u8>() {
                                                current = current.fg(Color::Indexed(n));
                                            }
                                        } else if param.starts_with("48;5;") {
                                            if let Ok(n) = param[5..].parse::<u8>() {
                                                current = current.bg(Color::Indexed(n));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } // else: non-SGR CSI (cursor movement, mode set, etc.) — silently discarded
                }
                Some(']') => {
                    // OSC sequence — skip until BEL or ST
                    for c in chars.by_ref() {
                        if c == '\x07' {
                            break;
                        }
                        if c == '\x1b' {
                            if let Some('\\') = chars.next() {
                                break;
                            }
                            break;
                        }
                    }
                }
                _ => {}
            }
        } else if c != '\r' {
            // Append character to last segment or create new one
            if let Some((_, last_style)) = segments.last() {
                if *last_style == current {
                    segments.last_mut().unwrap().0.push(c);
                    continue;
                }
            }
            segments.push((c.to_string(), current));
        }
    }

    segments
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

    // Espaço útil = total - bordas(2)
    let usable_height = area.height.saturating_sub(2) as usize;

    // Construir linhas do output com scroll e seleção, incluindo a linha corrente
    let mut all_output = state.output.clone();
    match &state.status {
        SshStatus::Connected => {
            if !state.current_line.is_empty() {
                all_output.push(state.current_line.clone());
            }
        }
        SshStatus::Connecting => {
            if !state.current_line.is_empty() {
                all_output.push(state.current_line.clone());
            }
            all_output.push("⏳ Conectando...".to_string());
        }
        SshStatus::Error(_) => {
            if !state.current_line.is_empty() {
                all_output.push(state.current_line.clone());
            }
            all_output.push("Pressione Ctrl+Q para voltar".to_string());
        }
        SshStatus::Disconnected => {
            if !state.current_line.is_empty() {
                all_output.push(state.current_line.clone());
            }
            all_output.push("○ Desconectado. Ctrl+Q para voltar.".to_string());
        }
    }

    let mut output_lines: Vec<Line> = vec![];
    let total_output = all_output.len();

    // Calcular quantas linhas mostrar
    let lines_to_show = if total_output > usable_height {
        usable_height
    } else {
        total_output
    };

    // Calcular índice inicial baseado no scroll
    let end_idx = total_output.saturating_sub(state.scroll_offset);
    let start_idx = end_idx.saturating_sub(lines_to_show);
    let padding_top = usable_height.saturating_sub(lines_to_show);

    // Salvar offsets para o handler de mouse
    state.first_visible_line.set(start_idx);
    state.padding_top.set(padding_top);

    for i in start_idx..end_idx {
        let line = &all_output[i];
        let segments = parse_ansi_spans(line);
        let mut spans = vec![];

        // Renderizar spans com estilo ANSI + suporte a seleção
        let mut col = 0;
        for (text, style) in &segments {
            for ch in text.chars() {
                let is_selected = state.is_selected(i, col);
                let final_style = if is_selected {
                    Style::default().fg(Color::Black).bg(Color::White)
                } else {
                    *style
                };
                spans.push(Span::styled(ch.to_string(), final_style));
                col += 1;
            }
        }

        // Se a linha for vazia, adicionar algo para poder selecionar
        if segments.is_empty() || (segments.len() == 1 && segments[0].0.is_empty()) {
            spans.push(Span::raw(" "));
        }

        output_lines.push(Line::from(spans));
    }

    // Adicionar linhas vazias para preencher se output for menor que altura
    while output_lines.len() < usable_height {
        output_lines.insert(0, Line::from(""));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn segments_text(segments: &[(String, Style)]) -> String {
        segments.iter().map(|(t, _)| t.as_str()).collect()
    }

    #[test]
    fn test_parse_bracketed_paste() {
        // CSI ?2004h should be silently discarded
        let r = parse_ansi_spans("\x1b[?2004h");
        assert!(r.is_empty(), "bracketed paste CSI h should produce no segments");
    }

    #[test]
    fn test_parse_bracketed_paste_off() {
        // CSI ?2004l should be silently discarded
        let r = parse_ansi_spans("\x1b[?2004l");
        assert!(r.is_empty(), "bracketed paste CSI l should produce no segments");
    }

    #[test]
    fn test_parse_osc_title() {
        // OSC 0 (set window title) terminated by BEL
        let r = parse_ansi_spans("\x1b]0;user@host: ~\x07hello");
        assert_eq!(segments_text(&r), "hello", "OSC should be skipped, only 'hello' remains");
    }

    #[test]
    fn test_parse_osc_st() {
        // OSC terminated by ST (ESC \)
        let r = parse_ansi_spans("\x1b]3008;start=uuid\x1b\\hello");
        assert_eq!(segments_text(&r), "hello", "OSC with ST should be skipped");
    }

    #[test]
    fn test_parse_sgr_colors() {
        let r = parse_ansi_spans("\x1b[31mred\x1b[32mgreen");
        assert_eq!(segments_text(&r), "redgreen");
        assert_eq!(r[0].1.fg, Some(Color::Red), "first segment should be red");
        assert_eq!(r[1].1.fg, Some(Color::Green), "second segment should be green");
    }

    #[test]
    fn test_parse_sgr_reset() {
        let r = parse_ansi_spans("\x1b[31mred\x1b[0mnormal");
        assert_eq!(segments_text(&r), "rednormal");
        assert_eq!(r[0].1.fg, Some(Color::Red), "first segment is red");
        assert_eq!(r[1].1.fg, Some(Color::White), "after reset should be default");
    }

    #[test]
    fn test_parse_bold_green_blue() {
        // Simulação do prompt: [01;32muser@host [00m: [01;34m~ [00m
        let r = parse_ansi_spans("\x1b[01;32muser\x1b[00m:\x1b[01;34m~\x1b[00m$ ");
        assert_eq!(segments_text(&r), "user:~$ ");
        let bold = Modifier::BOLD;
        assert!(r[0].1.add_modifier.contains(bold), "user should be bold");
        assert!(!r[1].1.add_modifier.contains(bold), ": should not be bold");
    }

    #[test]
    fn test_parse_mixed_sequence() {
        // Realistic sequence: bracketed paste + OSC title + SGR prompt
        let input = "\x1b[?2004h\x1b]0;admin@host: ~\x07\x1b[01;32madmin@host\x1b[00m:\x1b[01;34m~\x1b[00m$ ";
        let r = parse_ansi_spans(input);
        let text = segments_text(&r);
        assert!(text.contains("$"), "should contain prompt dollar sign");
        assert!(text.contains("admin@host"), "should contain username@host");
        assert!(!text.contains("2004"), "should not contain raw CSI codes");
    }

    #[test]
    fn test_parse_sgr_leading_zero() {
        // \x1b[00m should reset like \x1b[0m
        let r = parse_ansi_spans("\x1b[31mred\x1b[00mnormal");
        assert_eq!(segments_text(&r), "rednormal");
        assert_eq!(r[0].1.fg, Some(Color::Red));
        assert_eq!(r[1].1.fg, Some(Color::White));
    }
}
