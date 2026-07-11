use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::app::App;
use super::help::render_status_bar;
use super::theme::Theme;

/// Renderiza a lista de servidores com barra de busca, lista e rodapé de status.
pub fn render_server_list(f: &mut Frame, app: &App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

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

    // Position cursor at end of search input when in search mode
    if app.input_mode == super::app::InputMode::Search {
        let cursor_x = chunks[0].x + 1 + app.input.len() as u16;
        let cursor_y = chunks[0].y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }

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

            let mut spans = vec![
                Span::styled(pin_icon, style),
                Span::styled(&server.name, Style::default().fg(Theme::primary()).add_modifier(Modifier::BOLD)),
                Span::styled(" → ", Style::default().fg(Theme::text_dim())),
                Span::styled(&server.host, Style::default().fg(Theme::text())),
                Span::styled(":", Style::default().fg(Theme::text_dim())),
                Span::styled(server.port.to_string(), Style::default().fg(Theme::secondary())),
            ];

            let max_tags = 5;
            let visible_tags: Vec<&str> = server.tags.iter().take(max_tags).map(|s| s.as_str()).collect();
            let overflow = server.tags.len().saturating_sub(max_tags);

            for tag in &visible_tags {
                let color = Theme::tag_color(tag);
                spans.push(Span::styled(
                    format!(" {} ", tag),
                    Style::default().fg(Color::Black).bg(color).add_modifier(Modifier::BOLD),
                ));
            }
            if overflow > 0 {
                spans.push(Span::styled(
                    format!(" +{}", overflow),
                    Style::default().fg(Theme::text_dim()),
                ));
            }

            let line = Line::from(spans);

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

    // Status bar with footer hints
    render_status_bar(f, chunks[2], &app.current_view, app.start_time);
}
