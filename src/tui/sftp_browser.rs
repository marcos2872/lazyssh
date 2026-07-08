use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::config::models::Server;
use crate::sftp::{FileInfo, LocalFs, RemoteFs};
use super::theme::Theme;

#[derive(Debug)]
pub struct SftpState {
    pub local: LocalFs,
    pub remote: RemoteFs,
    pub local_files: Vec<FileInfo>,
    pub remote_files: Vec<FileInfo>,
    pub local_selected: usize,
    pub remote_selected: usize,
    pub focus_side: Side,
    pub status: String,
}

#[derive(Debug, PartialEq)]
pub enum Side {
    Local,
    Remote,
}

impl SftpState {
    pub fn new(server: Server) -> Self {
        let mut remote = RemoteFs::with_server(server);
        let remote_files = remote.list();

        let local = LocalFs::new();
        let local_files = local.list().unwrap_or_default();

        Self {
            local,
            remote,
            local_files,
            remote_files,
            local_selected: 0,
            remote_selected: 0,
            focus_side: Side::Local,
            status: "Conectado".to_string(),
        }
    }

    pub fn refresh_remote(&mut self) {
        self.remote_files = self.remote.list();
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
                    self.remote.cd(&file.name).map_err(|e| e.to_string())?;
                    self.refresh_remote();
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
                self.remote.cd("..").map_err(|e| e.to_string())?;
                self.refresh_remote();
                Ok(())
            }
        }
    }

    pub fn upload_selected(&mut self) -> Result<String, String> {
        let file = self.local_files.get(self.local_selected)
            .ok_or_else(|| "No file selected".to_string())?;

        if file.is_dir {
            return Err("Cannot upload directories".to_string());
        }

        let name = file.name.clone();
        self.remote.upload(&name, &name).map_err(|e| e.to_string())?;
        self.refresh_remote();
        Ok(format!("Uploaded: {}", name))
    }

    pub fn download_selected(&mut self) -> Result<String, String> {
        let file = self.remote_files.get(self.remote_selected)
            .ok_or_else(|| "No file selected".to_string())?;

        if file.is_dir {
            return Err("Cannot download directories".to_string());
        }

        let name = file.name.clone();
        let content = self.remote.get_file_content(&name)?;
        self.local.download(&content, &name).map_err(|e| e.to_string())?;
        self.refresh_local();
        Ok(format!("Downloaded: {}", name))
    }

    pub fn toggle_focus(&mut self) {
        self.focus_side = match self.focus_side {
            Side::Local => Side::Remote,
            Side::Remote => Side::Local,
        };
    }
}

pub fn render_sftp_browser(f: &mut Frame, state: &SftpState) {
    let area = f.area();

    // Layout principal
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),      // Áreas dos painéis
            Constraint::Length(3),   // Barra de status/ajuda
        ])
        .split(area);

    // Layout horizontal para os painéis
    let panel_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    render_local_pane(f, state, panel_chunks[0]);
    render_remote_pane(f, state, panel_chunks[1]);
    render_sftp_help(f, state, chunks[1]);
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
        .map(|file| {
            let (icon, style) = if file.is_dir {
                ("/ ", Style::default().fg(Theme::primary()))
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
        state.remote.current_dir(),
        if is_focused { "[FOCUS]" } else { "" }
    );

    let items: Vec<ListItem> = state
        .remote_files
        .iter()
        .map(|file| {
            let (icon, style) = if file.is_dir {
                ("/ ", Style::default().fg(Theme::primary()))
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
    let help_text = Line::from(vec![
        Span::styled(" Tab: trocar painel ", Style::default().fg(Theme::primary())),
        Span::styled("| ", Style::default().fg(Theme::text_dim())),
        Span::styled("Enter: entrar pasta ", Style::default().fg(Theme::success())),
        Span::styled("| ", Style::default().fg(Theme::text_dim())),
        Span::styled("Backspace: voltar ", Style::default().fg(Theme::warning())),
        Span::styled("| ", Style::default().fg(Theme::text_dim())),
        Span::styled("u: upload ", Style::default().fg(Theme::success())),
        Span::styled("| ", Style::default().fg(Theme::text_dim())),
        Span::styled("d: download ", Style::default().fg(Theme::primary())),
        Span::styled("| ", Style::default().fg(Theme::text_dim())),
        Span::styled("q: sair", Style::default().fg(Theme::error())),
    ]);

    let status = Paragraph::new(help_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" SFTP ")
            .title_style(Theme::title_style())
            .border_style(Theme::border_style()),
    );
    f.render_widget(status, area);
}

fn format_size(size: u64) -> String {
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
