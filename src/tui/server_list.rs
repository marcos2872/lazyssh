use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::app::App;

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
    let search_input = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title("Search [/]"));
    f.render_widget(search_input, chunks[0]);

    // Server list
    let items: Vec<ListItem> = app
        .filtered_indices
        .iter()
        .filter_map(|&i| app.servers.get(i))
        .map(|server| {
            let style = if server.pinned {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };

            let line = Line::from(vec![
                Span::styled(
                    if server.pinned { "● " } else { "○ " },
                    style,
                ),
                Span::styled(&server.name, Style::default().fg(Color::Cyan)),
                Span::raw(" - "),
                Span::raw(&server.host),
                Span::raw(":"),
                Span::raw(server.port.to_string()),
                Span::raw(" ["),
                Span::raw(server.tags.join(", ")),
                Span::raw("]"),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Servers"))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    let mut state = ListState::default();
    state.select(Some(app.selected));
    f.render_stateful_widget(list, chunks[1], &mut state);

    // Help bar
    let help_text = Line::from(vec![
        Span::styled("Enter: Connect ", Style::default().fg(Color::Green)),
        Span::styled("s: SFTP ", Style::default().fg(Color::Green)),
        Span::styled("a: Add ", Style::default().fg(Color::Green)),
        Span::styled("e: Edit ", Style::default().fg(Color::Green)),
        Span::styled("d: Delete ", Style::default().fg(Color::Green)),
        Span::styled("p: Pin ", Style::default().fg(Color::Green)),
        Span::styled("q: Quit ", Style::default().fg(Color::Red)),
    ]);

    let help = Paragraph::new(help_text).block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[2]);
}
