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
                    if app.input_mode == tui::app::InputMode::Insert {
                        let area = f.area();
                        let popup = ratatui::widgets::Block::default()
                            .borders(ratatui::widgets::Borders::ALL)
                            .title("Novo Servidor (nome)");
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
                                        app.input_mode = tui::app::InputMode::Insert;
                                        app.input.clear();
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
                                tui::app::InputMode::Insert => match key.code {
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
                                        let name = app.input.clone();
                                        if !name.is_empty() {
                                            let server = config::Server {
                                                name: name.clone(),
                                                host: "localhost".to_string(),
                                                port: 22,
                                                user: "root".to_string(),
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
                                },
                                _ => {}
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
