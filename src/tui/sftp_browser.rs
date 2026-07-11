use std::time::Instant;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::config::models::Server;
use crate::sftp::{FileInfo, LocalFs};
use super::theme::Theme;

/// Progresso de uma transferência de arquivo em andamento.
#[derive(Debug, Clone)]
pub struct TransferProgress {
    /// Nome do arquivo sendo transferido.
    pub file_name: String,
    /// Bytes já transferidos.
    pub bytes_done: u64,
    /// Total de bytes do arquivo.
    pub bytes_total: u64,
    /// `true` se é upload, `false` se é download.
    pub is_upload: bool,
    /// Instante em que a transferência começou.
    pub start_time: Instant,
}

impl TransferProgress {
    /// Retorna a porcentagem de conclusão (0-100).
    pub fn percentage(&self) -> u8 {
        if self.bytes_total == 0 {
            0
        } else {
            ((self.bytes_done as f64 / self.bytes_total as f64) * 100.0) as u8
        }
    }

    /// Retorna `true` se a transferência está completa.
    pub fn is_complete(&self) -> bool {
        self.bytes_done >= self.bytes_total
    }

    /// Estima o tempo restante em segundos, ou `None` se não puder calcular.
    pub fn eta_secs(&self) -> Option<u64> {
        if self.bytes_done == 0 || self.bytes_total == 0 {
            return None;
        }
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed < 0.1 {
            return None;
        }
        let rate = self.bytes_done as f64 / elapsed;
        if rate <= 0.0 {
            return None;
        }
        let remaining = (self.bytes_total - self.bytes_done) as f64 / rate;
        Some(remaining as u64)
    }
}

/// Modo de entrada do SFTP — controla o que o campo de input faz.
#[derive(Debug, Clone, PartialEq)]
pub enum SftpInputMode {
    /// Sem input ativo.
    None,
    /// Criando um novo diretório.
    Mkdir,
    /// Renomeando um arquivo/diretório.
    Rename,
    /// Alterando permissões (chmod) de um arquivo.
    Chmod,
    /// Salvando o diretório atual como marcador.
    Bookmark,
}

/// Estado da transferência de arquivo.
#[derive(Debug, Clone, PartialEq)]
pub enum TransferState {
    /// Nenhuma transferência em andamento.
    Idle,
    /// Transferência em andamento.
    Transferring {
        /// Nome do arquivo sendo transferido.
        file_name: String,
        /// Total de bytes do arquivo.
        bytes_total: u64,
        /// `true` se é upload, `false` se é download.
        is_upload: bool,
        /// Instante em que a transferência começou.
        start_time: Instant,
    },
}

/// Estado completo do navegador SFTP dual-pane (local/remoto).
#[derive(Debug)]
pub struct SftpState {
    /// Sistema de arquivos local sendo navegado.
    pub local: LocalFs,
    /// Arquivos no diretório local atual.
    pub local_files: Vec<FileInfo>,
    /// Arquivos no diretório remoto atual.
    pub remote_files: Vec<FileInfo>,
    /// Caminho absoluto do diretório remoto atual.
    pub remote_path: String,
    /// Índice do item selecionado no painel local.
    pub local_selected: usize,
    /// Índice do item selecionado no painel remoto.
    pub remote_selected: usize,
    /// Índices dos arquivos selecionados no painel local (multi-seleção).
    pub local_selected_files: Vec<usize>,
    /// Índices dos arquivos selecionados no painel remoto (multi-seleção).
    pub remote_selected_files: Vec<usize>,
    /// Painel atualmente focado (local ou remoto).
    pub focus_side: Side,
    /// Mensagem de status exibida na barra inferior.
    pub status: String,
    /// Progresso detalhado da transferência em andamento (opcional).
    pub transfer_progress: Option<TransferProgress>,
    /// Estado da transferência (Idle ou Transferring).
    pub transfer_state: TransferState,
    /// ID da sessão SFTP remota.
    pub session_id: Option<String>,
    /// Modo de input ativo (Mkdir, Rename, Chmod, Bookmark ou None).
    pub input_mode: SftpInputMode,
    /// Conteúdo do campo de input para mkdir/rename/chmod/bookmark.
    pub input_buffer: String,
    /// Posição do cursor no campo de input (para navegação com setas).
    pub input_cursor: usize,
    /// Instante em que a transferência começou (para timer).
    pub transfer_start: Option<std::time::Instant>,
}

/// Painel (lado) do navegador SFTP.
#[derive(Debug, PartialEq)]
pub enum Side {
    /// Painel local (filesystem da máquina).
    Local,
    /// Painel remoto (filesystem do servidor SSH).
    Remote,
}

impl SftpState {
    pub fn new(server: Server) -> Self {
        let local = LocalFs::new();
        let local_files = local.list().unwrap_or_default();
        let home_dir = format!("/home/{}", server.user);

        Self {
            local,
            local_files,
            remote_files: Vec::new(),
            remote_path: home_dir,
            local_selected: 0,
            remote_selected: 0,
            local_selected_files: Vec::new(),
            remote_selected_files: Vec::new(),
            focus_side: Side::Local,
            status: "Conectando...".to_string(),
            transfer_progress: None,
            transfer_state: TransferState::Idle,
            session_id: None,
            input_mode: SftpInputMode::None,
            input_buffer: String::new(),
            input_cursor: 0,
            transfer_start: None,
        }
    }

    pub fn refresh_remote(&mut self, files: Vec<FileInfo>) {
        self.remote_files = files;
        self.remote_selected = 0;
    }

    pub fn refresh_local(&mut self) {
        self.local_files = self.local.list().unwrap_or_default();
        self.local_selected = 0;
    }

    pub fn next_item(&mut self) {
        match self.focus_side {
            Side::Local => {
                let len = self.local_files.len();
                if len > 0 && self.local_selected < len - 1 {
                    self.local_selected += 1;
                }
            }
            Side::Remote => {
                let len = self.remote_files.len();
                if len > 0 && self.remote_selected < len - 1 {
                    self.remote_selected += 1;
                }
            }
        }
    }

    pub fn previous_item(&mut self) {
        match self.focus_side {
            Side::Local => {
                if self.local_selected > 0 {
                    self.local_selected -= 1;
                }
            }
            Side::Remote => {
                if self.remote_selected > 0 {
                    self.remote_selected -= 1;
                }
            }
        }
    }

    pub fn enter_directory(&mut self) -> Result<(), String> {
        match self.focus_side {
            Side::Local => {
                let file = self.local_files.get(self.local_selected)
                    .ok_or_else(|| "No file selected".to_string())?;

                if file.is_dir {
                    self.local.cd(&file.name).map_err(|e| e.to_string())?;
                    self.refresh_local();
                    Ok(())
                } else {
                    Err("Not a directory".to_string())
                }
            }
            Side::Remote => {
                let file = self.remote_files.get(self.remote_selected)
                    .ok_or_else(|| "No file selected".to_string())?;

                if file.is_dir {
                    self.remote_path = if self.remote_path.ends_with('/') {
                        format!("{}{}", self.remote_path, file.name)
                    } else {
                        format!("{}/{}", self.remote_path, file.name)
                    };
                    // remote_files will be populated by the caller via App
                    Ok(())
                } else {
                    Err("Not a directory".to_string())
                }
            }
        }
    }

    pub fn go_parent(&mut self) -> Result<(), String> {
        match self.focus_side {
            Side::Local => {
                self.local.cd("..").map_err(|e| e.to_string())?;
                self.refresh_local();
                Ok(())
            }
            Side::Remote => {
                // Go to parent directory
                let parent = if self.remote_path.ends_with('/') && self.remote_path.len() > 1 {
                    self.remote_path.trim_end_matches('/')
                } else {
                    &self.remote_path
                };

                let parent = std::path::Path::new(parent)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "/".to_string());

                self.remote_path = if parent.is_empty() { "/".to_string() } else { parent };
                // remote_files will be populated by the caller via App
                Ok(())
            }
        }
    }

    pub fn toggle_focus(&mut self) {
        self.focus_side = match self.focus_side {
            Side::Local => Side::Remote,
            Side::Remote => Side::Local,
        };
    }

    pub fn toggle_select_current(&mut self) {
        match self.focus_side {
            Side::Local => {
                let idx = self.local_selected;
                if let Some(pos) = self.local_selected_files.iter().position(|&i| i == idx) {
                    self.local_selected_files.remove(pos);
                } else {
                    self.local_selected_files.push(idx);
                }
            }
            Side::Remote => {
                let idx = self.remote_selected;
                if let Some(pos) = self.remote_selected_files.iter().position(|&i| i == idx) {
                    self.remote_selected_files.remove(pos);
                } else {
                    self.remote_selected_files.push(idx);
                }
            }
        }
    }

    pub fn select_all_current(&mut self) {
        match self.focus_side {
            Side::Local => {
                self.local_selected_files = (0..self.local_files.len()).collect();
            }
            Side::Remote => {
                self.remote_selected_files = (0..self.remote_files.len()).collect();
            }
        }
    }

    pub fn clear_selection(&mut self) {
        match self.focus_side {
            Side::Local => self.local_selected_files.clear(),
            Side::Remote => self.remote_selected_files.clear(),
        }
    }

    pub fn get_selected_files(&self) -> Vec<&FileInfo> {
        match self.focus_side {
            Side::Local => {
                self.local_selected_files.iter()
                    .filter_map(|&i| self.local_files.get(i))
                    .collect()
            }
            Side::Remote => {
                self.remote_selected_files.iter()
                    .filter_map(|&i| self.remote_files.get(i))
                    .collect()
            }
        }
    }

    pub fn start_transfer(&mut self, file_name: String, total_bytes: u64, is_upload: bool) {
        let start_time = Instant::now();
        self.transfer_state = TransferState::Transferring {
            file_name: file_name.clone(),
            bytes_total: total_bytes,
            is_upload,
            start_time,
        };
        self.transfer_start = Some(start_time);
        self.transfer_progress = Some(TransferProgress {
            file_name,
            bytes_done: 0,
            bytes_total: total_bytes,
            is_upload,
            start_time,
        });
    }

    pub fn update_transfer_progress(&mut self, bytes_done: u64) {
        if let Some(progress) = &mut self.transfer_progress {
            progress.bytes_done = bytes_done;
            if progress.is_complete() {
                self.transfer_state = TransferState::Idle;
            }
        }
    }

    pub fn finish_transfer(&mut self) {
        self.transfer_state = TransferState::Idle;
        self.transfer_progress = None;
        self.local_selected_files.clear();
        self.remote_selected_files.clear();
    }
}

pub fn render_sftp_browser(f: &mut Frame, state: &SftpState) {
    let area = f.area();

    let has_input = state.input_mode != SftpInputMode::None;

    // Layout principal
    let chunks = if has_input {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),      // Áreas dos painéis
                Constraint::Length(3),   // Input line (bordas ALL = 3 linhas)
                Constraint::Length(3),   // Barra de status/ajuda
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),      // Áreas dos painéis
                Constraint::Length(3),   // Barra de status/ajuda
            ])
            .split(area)
    };

    // Layout horizontal para os painéis
    let panel_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    render_local_pane(f, state, panel_chunks[0]);
    render_remote_pane(f, state, panel_chunks[1]);

    if has_input {
        render_sftp_input(f, state, chunks[1]);
        render_sftp_help(f, state, chunks[2]);
    } else {
        render_sftp_help(f, state, chunks[1]);
    }
}

fn render_local_pane(f: &mut Frame, state: &SftpState, area: ratatui::layout::Rect) {
    let is_focused = state.focus_side == Side::Local;

    let title = format!(
        " [L] Local: {} {}",
        state.local.current_dir().display(),
        if is_focused { "[FOCUS]" } else { "" }
    );

    let items: Vec<ListItem> = state
        .local_files
        .iter()
        .enumerate()
        .map(|(idx, file)| {
            let is_selected = state.local_selected_files.contains(&idx);
            let (icon, style) = if file.is_dir {
                ("/ ", Style::default().fg(Theme::primary()))
            } else if is_selected {
                ("↑ ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            } else {
                ("  ", Style::default().fg(Theme::text()))
            };

            let size_str = if file.is_dir {
                String::new()
            } else {
                format_size(file.size)
            };

            ListItem::new(Line::from(vec![
                Span::styled(icon, style),
                Span::styled(&file.name, style),
                if size_str.is_empty() {
                    Span::raw("")
                } else {
                    Span::styled(format!(" ({})", size_str), Style::default().fg(Theme::text_dim()))
                },
            ]))
        })
        .collect();

    let border_style = if is_focused {
        Theme::modal_border_style()
    } else {
        Theme::border_style()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_style(Theme::title_style())
                .border_style(border_style),
        )
        .highlight_style(Theme::selected_style())
        .highlight_symbol(" ➜ ");

    let mut list_state = ListState::default();
    list_state.select(Some(state.local_selected));
    f.render_stateful_widget(list, area, &mut list_state);
}

fn render_remote_pane(f: &mut Frame, state: &SftpState, area: ratatui::layout::Rect) {
    let is_focused = state.focus_side == Side::Remote;

    let title = format!(
        " [R] Remote: {} {}",
        state.remote_path,
        if is_focused { "[FOCUS]" } else { "" }
    );

    let items: Vec<ListItem> = state
        .remote_files
        .iter()
        .enumerate()
        .map(|(idx, file)| {
            let is_selected = state.remote_selected_files.contains(&idx);
            let (icon, style) = if file.is_dir {
                ("/ ", Style::default().fg(Theme::primary()))
            } else if is_selected {
                ("↓ ", Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD))
            } else {
                ("  ", Style::default().fg(Theme::text()))
            };

            let size_str = if file.is_dir {
                String::new()
            } else {
                format_size(file.size)
            };

            ListItem::new(Line::from(vec![
                Span::styled(icon, style),
                Span::styled(&file.name, style),
                if size_str.is_empty() {
                    Span::raw("")
                } else {
                    Span::styled(format!(" ({})", size_str), Style::default().fg(Theme::text_dim()))
                },
            ]))
        })
        .collect();

    let border_style = if is_focused {
        Theme::modal_border_style()
    } else {
        Theme::border_style()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_style(Theme::title_style())
                .border_style(border_style),
        )
        .highlight_style(Theme::selected_style())
        .highlight_symbol(" ➜ ");

    let mut list_state = ListState::default();
    list_state.select(Some(state.remote_selected));
    f.render_stateful_widget(list, area, &mut list_state);
}

fn render_sftp_help(f: &mut Frame, state: &SftpState, area: ratatui::layout::Rect) {
    // Mostrar mensagem de transferência se ativo
    if matches!(state.transfer_state, TransferState::Transferring { .. }) {
        let elapsed = state.transfer_start
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0);
        let time_str = format!("{:02}:{:02}", elapsed / 60, elapsed % 60);
        let (action, color) = state.transfer_progress
            .as_ref()
            .map(|p| if p.is_upload { ("Enviando", Theme::success()) } else { ("Baixando", Theme::primary()) })
            .unwrap_or(("Transferindo", Theme::warning()));
        let transfer_msg = Line::from(vec![
            Span::styled(
                format!(" ⏳ {}... ", action),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &time_str,
                Style::default().fg(Theme::text()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " (barra de progresso em breve)",
                Style::default().fg(Theme::text_dim()),
            ),
        ]);
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Transferindo ")
            .title_style(Style::default().fg(Theme::warning()).add_modifier(Modifier::BOLD))
            .border_style(Style::default().fg(Theme::warning()));
        let status = Paragraph::new(transfer_msg).block(block);
        f.render_widget(status, area);
        return;
    }

    // Barra de ajuda normal
    let selected_count = match state.focus_side {
        Side::Local => state.local_selected_files.len(),
        Side::Remote => state.remote_selected_files.len(),
    };

    let help_text = if selected_count > 0 {
        Line::from(vec![
            Span::styled(
                format!(" {} selecionado(s) ", selected_count),
                Style::default().fg(Theme::success()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("| ", Style::default().fg(Theme::text_dim())),
            Span::styled("Space ", Style::default().fg(Theme::primary())),
            Span::styled("| ", Style::default().fg(Theme::text_dim())),
            Span::styled("a: todos ", Style::default().fg(Theme::primary())),
            Span::styled("| ", Style::default().fg(Theme::text_dim())),
            Span::styled("u/d ", Style::default().fg(Theme::success())),
        ])
    } else {
        Line::from(vec![
            Span::styled(" Tab ", Style::default().fg(Theme::primary())),
            Span::styled("| ", Style::default().fg(Theme::text_dim())),
            Span::styled("Enter ", Style::default().fg(Theme::success())),
            Span::styled("| ", Style::default().fg(Theme::text_dim())),
            Span::styled("M/x/R/m ", Style::default().fg(Theme::primary())),
            Span::styled("| ", Style::default().fg(Theme::text_dim())),
            Span::styled("b/B: bookmark ", Style::default().fg(Theme::secondary())),
            Span::styled("| ", Style::default().fg(Theme::text_dim())),
            Span::styled("u/d ", Style::default().fg(Theme::success())),
            Span::styled("| ", Style::default().fg(Theme::text_dim())),
            Span::styled("q", Style::default().fg(Theme::error())),
        ])
    };

    let status = Paragraph::new(help_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" SFTP ")
            .title_style(Theme::title_style())
            .border_style(Theme::border_style()),
    );
    f.render_widget(status, area);
}

fn render_sftp_input(f: &mut Frame, state: &SftpState, area: ratatui::layout::Rect) {
    let (title, placeholder) = match state.input_mode {
        SftpInputMode::Mkdir => (" Criar pasta ", "Nome da pasta..."),
        SftpInputMode::Rename => (" Renomear ", "Novo nome..."),
        SftpInputMode::Chmod => (" Permissões ", "755"),
        SftpInputMode::Bookmark => (" Salvar bookmark ", "Nome do bookmark..."),
        SftpInputMode::None => return,
    };

    let display = if state.input_buffer.is_empty() {
        Span::styled(placeholder, Style::default().fg(Theme::text_dim()))
    } else {
        Span::styled(&state.input_buffer, Style::default().fg(Theme::text()))
    };

    let input_line = Line::from(vec![
        Span::styled("  ", Style::default()),
        display,
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Theme::modal_title_style())
        .border_style(Theme::modal_border_style());

    let input = Paragraph::new(input_line).block(block);
    f.render_widget(input, area);

    // Position cursor: +1 for left border, +2 for text prefix spaces
    let cursor_x = area.x + 3 + state.input_cursor as u16;
    let cursor_y = area.y + 1;
    f.set_cursor_position((cursor_x, cursor_y));
}

pub fn format_size(size: u64) -> String {
    if size < 1024 {
        format!("{}B", size)
    } else if size < 1024 * 1024 {
        format!("{:.1}K", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.1}M", size as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1}G", size as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
