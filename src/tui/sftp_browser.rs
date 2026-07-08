use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::sftp::{LocalFs, RemoteFs};

#[derive(Debug)]
pub struct SftpState {
    pub local: LocalFs,
    pub remote: RemoteFs,
    pub local_selected: usize,
    pub remote_selected: usize,
    pub focus_side: Side,
}

#[derive(Debug, PartialEq)]
pub enum Side {
    Local,
    Remote,
}

impl SftpState {
    pub fn new() -> Self {
        Self {
            local: LocalFs::new(),
            remote: RemoteFs::new(),
            local_selected: 0,
            remote_selected: 0,
            focus_side: Side::Local,
        }
    }

    pub fn next_item(&mut self) {
        match self.focus_side {
            Side::Local => {
                let len = self.local.list().unwrap_or_default().len();
                if len > 0 && self.local_selected < len - 1 {
                    self.local_selected += 1;
                }
            }
            Side::Remote => {
                let len = self.remote.len();
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

    pub fn toggle_focus(&mut self) {
        self.focus_side = match self.focus_side {
            Side::Local => Side::Remote,
            Side::Remote => Side::Local,
        };
    }
}

pub fn render_sftp_browser(f: &mut Frame, state: &SftpState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(f.area());

    render_local_pane(f, state, chunks[0]);
    render_remote_pane(f, state, chunks[1]);
}

fn render_local_pane(f: &mut Frame, state: &SftpState, area: ratatui::layout::Rect) {
    let files = state.local.list().unwrap_or_default();

    let title = if state.focus_side == Side::Local {
        format!("Local: {} [FOCUS]", state.local.current_dir().display())
    } else {
        format!("Local: {}", state.local.current_dir().display())
    };

    let items: Vec<ListItem> = files
        .iter()
        .map(|file| {
            let style = if file.is_dir {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(if file.is_dir { "/ " } else { "  " }, style),
                Span::styled(&file.name, style),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    let mut list_state = ListState::default();
    list_state.select(Some(state.local_selected));
    f.render_stateful_widget(list, area, &mut list_state);
}

fn render_remote_pane(f: &mut Frame, state: &SftpState, area: ratatui::layout::Rect) {
    let title = if state.focus_side == Side::Remote {
        format!("Remote: {} [FOCUS]", state.remote.current_dir())
    } else {
        format!("Remote: {}", state.remote.current_dir())
    };

    let placeholder = Paragraph::new("Remote SFTP\n(Connect to server first)")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title),
        );
    f.render_widget(placeholder, area);
}
