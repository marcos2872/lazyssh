pub mod config;
pub mod sftp;
pub mod ssh;
pub mod tui;
pub mod vault;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;

use tui::{render_server_list, render_sftp_browser, App};

struct CleanupGuard;

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let _guard = CleanupGuard;

    let config = config::load_or_default();
    let mut app = App::new(config.servers);

    loop {
        terminal.draw(|f| {
            match app.current_view {
                tui::app::CurrentView::ServerList => {
                    render_server_list(f, &app);
                    if let Some(ref state) = app.insert_state {
                        let area = f.area();
                        let rect = ratatui::layout::Rect::new(
                            area.width / 4,
                            area.height / 2 - 4,
                            area.width / 2,
                            10,
                        );

                        let mut lines = vec![];
                        let fields = [
                            (tui::app::InsertField::Name, "Nome"),
                            (tui::app::InsertField::Host, "Host"),
                            (tui::app::InsertField::Port, "Porta"),
                            (tui::app::InsertField::User, "Usuário"),
                        ];

                        for (field_type, label) in &fields {
                            let value = match field_type {
                                tui::app::InsertField::Name => &state.name,
                                tui::app::InsertField::Host => &state.host,
                                tui::app::InsertField::Port => &state.port,
                                tui::app::InsertField::User => &state.user,
                            };
                            let marker = if &state.field == field_type { "▶" } else { " " };
                            let line = format!("{} {}: {}", marker, label, value);
                            lines.push(ratatui::text::Line::from(line));
                        }

                        lines.push(ratatui::text::Line::from(""));
                        lines.push(ratatui::text::Line::from("Tab: próximo campo"));
                        lines.push(ratatui::text::Line::from("Enter: salvar"));
                        lines.push(ratatui::text::Line::from("Esc: cancelar"));

                        let block = ratatui::widgets::Block::default()
                            .borders(ratatui::widgets::Borders::ALL)
                            .title("Novo Servidor");

                        let input = ratatui::widgets::Paragraph::new(lines)
                            .block(block);
                        f.render_widget(input, rect);
                    } else if app.input_mode == tui::app::InputMode::Edit {
                        let area = f.area();
                        let popup = ratatui::widgets::Block::default()
                            .borders(ratatui::widgets::Borders::ALL)
                            .title("Editar Nome");
                        let input = ratatui::widgets::Paragraph::new(app.input.as_str())
                            .block(popup);
                        let rect = ratatui::layout::Rect::new(
                            area.width / 4,
                            area.height / 2 - 2,
                            area.width / 2,
                            3,
                        );
                        f.render_widget(input, rect);
                    }
                }
                tui::app::CurrentView::SftpBrowser => {
                    if let Some(sftp) = &app.sftp_state {
                        render_sftp_browser(f, sftp);
                    }
                }
                _ => {}
            }
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.current_view {
                        tui::app::CurrentView::ServerList => {
                            match app.input_mode {
                                tui::app::InputMode::Normal => match key.code {
                                    KeyCode::Char('q') => app.should_quit = true,
                                    KeyCode::Char('j') | KeyCode::Down => app.next(),
                                    KeyCode::Char('k') | KeyCode::Up => app.previous(),
                                    KeyCode::Char('/') => {
                                        app.input_mode = tui::app::InputMode::Search;
                                        app.input.clear();
                                    }
                                    KeyCode::Char('a') => {
                                        app.insert_state = Some(tui::app::InsertState::new());
                                        app.input_mode = tui::app::InputMode::Insert;
                                    }
                                    KeyCode::Char('e') => {
                                        if let Some(server) = app.selected_server() {
                                            app.input = server.name.clone();
                                            app.input_mode = tui::app::InputMode::Edit;
                                        }
                                    }
                                    KeyCode::Char('p') => {
                                        if let Some(server) = app.selected_server_mut() {
                                            server.pinned = !server.pinned;
                                        }
                                    }
                                    KeyCode::Char('d') => {
                                        if let Some(server) = app.selected_server() {
                                            let name = server.name.clone();
                                            app.servers.retain(|s| s.name != name);
                                            app.filter(&app.input.clone());
                                            let _ = config::save_config(
                                                &config::AppConfig { servers: app.servers.clone() },
                                                &config::get_config_path(),
                                            );
                                        }
                                    }
                                    KeyCode::Char('s') => app.open_sftp(),
                                    KeyCode::Enter => {
                                        if let Some(_server) = app.selected_server() {
                                            // TODO: Open SSH terminal view
                                            app.should_quit = true;
                                        }
                                    }
                                    _ => {}
                                },
                                tui::app::InputMode::Insert => {
                                    if let Some(ref mut state) = app.insert_state {
                                        match key.code {
                                            KeyCode::Esc => {
                                                app.input_mode = tui::app::InputMode::Normal;
                                                app.insert_state = None;
                                            }
                                            KeyCode::Tab => {
                                                state.next_field();
                                            }
                                            KeyCode::Char(c) => {
                                                state.current_value_mut().push(c);
                                            }
                                            KeyCode::Backspace => {
                                                state.current_value_mut().pop();
                                            }
                                            KeyCode::Enter => {
                                                if matches!(state.field, tui::app::InsertField::User) {
                                                    let name = state.name.clone();
                                                    let host = state.host.clone();
                                                    let port: u16 = state.port.parse().unwrap_or(22);
                                                    let user = state.user.clone();

                                                    if !name.is_empty() && !host.is_empty() {
                                                        let server = config::Server {
                                                            name,
                                                            host,
                                                            port,
                                                            user,
                                                            auth: config::Auth::Key {
                                                                path: "~/.ssh/id_rsa".to_string(),
                                                                passphrase: None,
                                                            },
                                                            tags: vec![],
                                                            pinned: false,
                                                        };
                                                        app.servers.push(server);
                                                        app.filter(&app.input.clone());
                                                        let _ = config::save_config(
                                                            &config::AppConfig { servers: app.servers.clone() },
                                                            &config::get_config_path(),
                                                        );
                                                    }
                                                    app.input_mode = tui::app::InputMode::Normal;
                                                    app.insert_state = None;
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                },
                                tui::app::InputMode::Edit => match key.code {
                                    KeyCode::Esc => {
                                        app.input_mode = tui::app::InputMode::Normal;
                                        app.input.clear();
                                    }
                                    KeyCode::Char(c) => {
                                        app.input.push(c);
                                    }
                                    KeyCode::Backspace => {
                                        app.input.pop();
                                    }
                                    KeyCode::Enter => {
                                        let new_name = app.input.clone();
                                        if !new_name.is_empty() {
                                            if let Some(server) = app.selected_server_mut() {
                                                server.name = new_name;
                                                let _ = config::save_config(
                                                    &config::AppConfig { servers: app.servers.clone() },
                                                    &config::get_config_path(),
                                                );
                                            }
                                        }
                                        app.input_mode = tui::app::InputMode::Normal;
                                        app.input.clear();
                                    }
                                    _ => {}
                                },
                                tui::app::InputMode::Search => match key.code {
                                    KeyCode::Enter => app.input_mode = tui::app::InputMode::Normal,
                                    KeyCode::Esc => {
                                        app.input_mode = tui::app::InputMode::Normal;
                                        app.input.clear();
                                        app.filter("");
                                    }
                                    KeyCode::Char(c) => {
                                        app.input.push(c);
                                        app.filter(&app.input.clone());
                                    }
                                    KeyCode::Backspace => {
                                        app.input.pop();
                                        app.filter(&app.input.clone());
                                    }
                                    _ => {}
                                }
                            }
                        }
                        tui::app::CurrentView::SftpBrowser => {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => app.close_sftp(),
                                KeyCode::Tab => {
                                    if let Some(sftp) = &mut app.sftp_state {
                                        sftp.toggle_focus();
                                    }
                                }
                                KeyCode::Char('j') | KeyCode::Down => {
                                    if let Some(sftp) = &mut app.sftp_state {
                                        sftp.next_item();
                                    }
                                }
                                KeyCode::Char('k') | KeyCode::Up => {
                                    if let Some(sftp) = &mut app.sftp_state {
                                        sftp.previous_item();
                                    }
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
