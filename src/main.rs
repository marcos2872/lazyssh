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

use tui::{render_server_list, App};

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Load config
    let config = config::load_or_default();
    let mut app = App::new(config.servers);

    // Main loop
    loop {
        terminal.draw(|f| {
            match app.current_view {
                tui::app::CurrentView::ServerList => render_server_list(f, &app),
                _ => {}
            }
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.input_mode {
                        tui::app::InputMode::Normal => match key.code {
                            KeyCode::Char('q') => app.should_quit = true,
                            KeyCode::Char('j') | KeyCode::Down => app.next(),
                            KeyCode::Char('k') | KeyCode::Up => app.previous(),
                            KeyCode::Char('/') => {
                                app.input_mode = tui::app::InputMode::Search;
                                app.input.clear();
                            }
                            KeyCode::Enter => {
                                if let Some(server) = app.selected_server() {
                                    println!("Connecting to {}...", server.name);
                                    app.should_quit = true;
                                }
                            }
                            _ => {}
                        },
                        tui::app::InputMode::Search => match key.code {
                            KeyCode::Enter => {
                                app.input_mode = tui::app::InputMode::Normal;
                            }
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
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
