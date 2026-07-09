use ratatui::{
    layout::{Constraint, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::app::App;
use super::theme::Theme;

pub fn render_server_list(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Search bar
    let search_style = if app.input_mode == super::app::InputMode::Search {
        Theme::input_active_style()
    } else {
        Theme::input_style()
    };

    let search_input = Paragraph::new(app.input.as_str())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" 🔍 Search ")
                .title_style(Theme::title_style())
                .border_style(Theme::border_style()),
        )
        .style(search_style);
    f.render_widget(search_input, chunks[0]);

    // Server list
    let items: Vec<ListItem> = app
        .filtered_indices
        .iter()
        .filter_map(|&i| app.servers.get(i))
        .map(|server| {
            let style = if server.pinned {
                Style::default().fg(Theme::warning())
            } else {
                Style::default().fg(Theme::text())
            };

            let pin_icon = if server.pinned { "📌 " } else { "   " };

            let line = Line::from(vec![
                Span::styled(pin_icon, style),
                Span::styled(&server.name, Style::default().fg(Theme::primary()).add_modifier(Modifier::BOLD)),
                Span::styled(" → ", Style::default().fg(Theme::text_dim())),
                Span::styled(&server.host, Style::default().fg(Theme::text())),
                Span::styled(":", Style::default().fg(Theme::text_dim())),
                Span::styled(server.port.to_string(), Style::default().fg(Theme::secondary())),
                if server.tags.is_empty() {
                    Span::raw("")
                } else {
                    Span::styled(
                        format!(" [{}]", server.tags.join(", ")),
                        Style::default().fg(Theme::text_dim()),
                    )
                },
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" 🖥 Servers ")
                .title_style(Theme::title_style())
                .border_style(Theme::border_style()),
        )
        .highlight_style(Theme::selected_style())
        .highlight_symbol(" ➜ ");

    let mut state = ListState::default();
    state.select(Some(app.selected));
    f.render_stateful_widget(list, chunks[1], &mut state);

    // Help bar
    let help_text = Line::from(vec![
        Span::styled("⏎ Connect ", Style::default().fg(Theme::success())),
        Span::styled("s SFTP ", Style::default().fg(Theme::success())),
        Span::styled("a Add ", Style::default().fg(Theme::success())),
        Span::styled("e Edit ", Style::default().fg(Theme::success())),
        Span::styled("d Delete ", Style::default().fg(Theme::warning())),
        Span::styled("p Pin ", Style::default().fg(Theme::success())),
        Span::styled("q Quit ", Style::default().fg(Theme::error())),
    ]);

    let help = Paragraph::new(help_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Theme::border_style()),
    );
    f.render_widget(help, chunks[2]);
}
