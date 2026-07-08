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

use tui::{render_notifications, render_server_list, render_sftp_browser, render_ssh_terminal, App};

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
                        let is_key = state.is_key_auth();
                        let height: u16 = if is_key { 12 } else { 10 };
                        let y = if area.height > height { (area.height - height) / 2 } else { 0 };
                        let rect = ratatui::layout::Rect::new(
                            area.width / 4,
                            y,
                            area.width / 2,
                            height,
                        );

                        let mut lines = vec![];

                        // Campos fixos
                        let base_fields = [
                            (tui::app::InsertField::Name, "Nome", &state.name),
                            (tui::app::InsertField::Host, "Host", &state.host),
                            (tui::app::InsertField::Port, "Porta", &state.port),
                            (tui::app::InsertField::User, "Usuário", &state.user),
                            (tui::app::InsertField::AuthType, "Auth (key/password)", &state.auth_type),
                        ];

                        for (field_type, label, value) in &base_fields {
                            let marker = if &state.field == field_type { "▶" } else { " " };
                            let line = format!("{} {}: {}", marker, label, value);
                            lines.push(ratatui::text::Line::from(line));
                        }

                        // Campos dinâmicos baseado no auth type
                        if is_key {
                            let marker_key = if state.field == tui::app::InsertField::KeyPath { "▶" } else { " " };
                            lines.push(ratatui::text::Line::from(format!("{} Caminho chave: {}", marker_key, state.key_path)));

                            let marker_pass = if state.field == tui::app::InsertField::Passphrase { "▶" } else { " " };
                            lines.push(ratatui::text::Line::from(format!("{} Passphrase: {}", marker_pass, state.passphrase)));
                        } else {
                            let marker_pass = if state.field == tui::app::InsertField::Password { "▶" } else { " " };
                            lines.push(ratatui::text::Line::from(format!("{} Senha: {}", marker_pass, state.password)));
                        }

                        lines.push(ratatui::text::Line::from(""));
                        lines.push(ratatui::text::Line::from("↑/↓/Tab: próximo campo"));
                        lines.push(ratatui::text::Line::from("Enter: salvar (no último campo)"));
                        lines.push(ratatui::text::Line::from("Esc: cancelar"));

                        let block = ratatui::widgets::Block::default()
                            .borders(ratatui::widgets::Borders::ALL)
                            .title("Novo Servidor");

                        let input = ratatui::widgets::Paragraph::new(lines).block(block);
                        f.render_widget(input, rect);
                    } else if let Some(ref edit) = app.edit_state {
                        let area = f.area();
                        let is_key = edit.is_key_auth();
                        let height: u16 = if is_key { 13 } else { 11 };
                        let y = if area.height > height { (area.height - height) / 2 } else { 0 };
                        let rect = ratatui::layout::Rect::new(
                            area.width / 4,
                            y,
                            area.width / 2,
                            height,
                        );

                        let mut lines = vec![];

                        // Campos fixos
                        let base_fields = [
                            (tui::app::EditField::Name, "Nome", &edit.name),
                            (tui::app::EditField::Host, "Host", &edit.host),
                            (tui::app::EditField::Port, "Porta", &edit.port),
                            (tui::app::EditField::User, "Usuário", &edit.user),
                            (tui::app::EditField::AuthType, "Auth (key/password)", &edit.auth_type),
                        ];

                        for (field_type, label, value) in &base_fields {
                            let marker = if &edit.field == field_type { "▶" } else { " " };
                            let line = format!("{} {}: {}", marker, label, value);
                            lines.push(ratatui::text::Line::from(line));
                        }

                        // Campos dinâmicos baseado no auth type
                        if is_key {
                            let marker_key = if edit.field == tui::app::EditField::KeyPath { "▶" } else { " " };
                            lines.push(ratatui::text::Line::from(format!("{} Caminho chave: {}", marker_key, edit.key_path)));

                            let marker_pass = if edit.field == tui::app::EditField::Passphrase { "▶" } else { " " };
                            lines.push(ratatui::text::Line::from(format!("{} Passphrase: {}", marker_pass, edit.passphrase)));
                        } else {
                            let marker_pass = if edit.field == tui::app::EditField::Password { "▶" } else { " " };
                            lines.push(ratatui::text::Line::from(format!("{} Senha: {}", marker_pass, edit.password)));
                        }

                        lines.push(ratatui::text::Line::from(""));
                        lines.push(ratatui::text::Line::from("↑/↓/Tab: próximo campo"));
                        lines.push(ratatui::text::Line::from("Enter: salvar (no último campo)"));
                        lines.push(ratatui::text::Line::from("Esc: cancelar"));

                        let block = ratatui::widgets::Block::default()
                            .borders(ratatui::widgets::Borders::ALL)
                            .title("Editar Servidor");

                        let input = ratatui::widgets::Paragraph::new(lines).block(block);
                        f.render_widget(input, rect);
                    }
                }
                tui::app::CurrentView::SftpBrowser => {
                    if let Some(sftp) = &app.sftp_state {
                        render_sftp_browser(f, sftp);
                    }
                }
                tui::app::CurrentView::SshTerminal => {
                    if let Some(ssh) = &app.ssh_state {
                        render_ssh_terminal(f, ssh);
                    }
                }
                _ => {}
            }

            // Renderizar notificações por cima de tudo
            app.notifications.clear_expired();
            render_notifications(f, &app.notifications, f.area());
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
                                            let index = app.filtered_indices[app.selected];
                                            app.edit_state = Some(tui::app::EditState::from_server(server, index));
                                            app.input_mode = tui::app::InputMode::Edit;
                                        }
                                    }
                                    KeyCode::Char('p') => {
                                        if let Some(server) = app.selected_server_mut() {
                                            server.pinned = !server.pinned;
                                            let status = if server.pinned { "fixado" } else { "desafixado" };
                                            app.notifications.success(&format!("Servidor {}!", status));
                                            let _ = config::save_config(
                                                &config::AppConfig { servers: app.servers.clone() },
                                                &config::get_config_path(),
                                            );
                                        }
                                    }
                                    KeyCode::Char('d') => {
                                        if let Some(server) = app.selected_server() {
                                            let name = server.name.clone();
                                            app.servers.retain(|s| s.name != name);
                                            app.filter(&app.input.clone());
                                            app.notifications.success(&format!("Servidor '{}' removido.", name));
                                            let _ = config::save_config(
                                                &config::AppConfig { servers: app.servers.clone() },
                                                &config::get_config_path(),
                                            );
                                        }
                                    }
                                    KeyCode::Char('s') => app.open_sftp(),
                                    KeyCode::Enter => {
                                        if let Some(_server) = app.selected_server() {
                                            app.connect_ssh();
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
                                            KeyCode::Tab | KeyCode::Down => {
                                                state.next_field();
                                            }
                                            KeyCode::Up => {
                                                state.prev_field();
                                            }
                                            KeyCode::Char(c) => {
                                                state.current_value_mut().push(c);
                                            }
                                            KeyCode::Backspace => {
                                                state.current_value_mut().pop();
                                            }
                                            KeyCode::Enter => {
                                                // Salvar no último campo baseado no auth type
                                                let should_save = if state.is_key_auth() {
                                                    matches!(state.field, tui::app::InsertField::Passphrase)
                                                } else {
                                                    matches!(state.field, tui::app::InsertField::Password)
                                                };

                                                if should_save {
                                                    let name = state.name.clone();
                                                    let host = state.host.clone();
                                                    let port: u16 = state.port.parse().unwrap_or(22);
                                                    let user = state.user.clone();
                                                    let auth = state.build_auth();

                                                    if !name.is_empty() && !host.is_empty() {
                                                        let server = config::Server {
                                                            name: name.clone(),
                                                            host,
                                                            port,
                                                            user,
                                                            auth,
                                                            tags: vec![],
                                                            pinned: false,
                                                        };
                                                        app.servers.push(server);
                                                        app.filter(&app.input.clone());
                                                        let _ = config::save_config(
                                                            &config::AppConfig { servers: app.servers.clone() },
                                                            &config::get_config_path(),
                                                        );
                                                        app.notifications.success(&format!("Servidor '{}' adicionado!", name));
                                                    } else {
                                                        app.notifications.warning("Nome e Host são obrigatórios.");
                                                    }
                                                    app.input_mode = tui::app::InputMode::Normal;
                                                    app.insert_state = None;
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                },
                                tui::app::InputMode::Edit => {
                                    if let Some(ref mut edit) = app.edit_state {
                                        match key.code {
                                            KeyCode::Esc => {
                                                app.input_mode = tui::app::InputMode::Normal;
                                                app.edit_state = None;
                                            }
                                            KeyCode::Tab | KeyCode::Down => {
                                                edit.next_field();
                                            }
                                            KeyCode::Up => {
                                                edit.prev_field();
                                            }
                                            KeyCode::Char(c) => {
                                                edit.current_value_mut().push(c);
                                            }
                                            KeyCode::Backspace => {
                                                edit.current_value_mut().pop();
                                            }
                                            KeyCode::Enter => {
                                                // Salvar no último campo baseado no auth type
                                                let should_save = if edit.is_key_auth() {
                                                    matches!(edit.field, tui::app::EditField::Passphrase)
                                                } else {
                                                    matches!(edit.field, tui::app::EditField::Password)
                                                };

                                                if should_save {
                                                    let index = edit.server_index;
                                                    let name = edit.name.clone();
                                                    let host = edit.host.clone();
                                                    let port: u16 = edit.port.parse().unwrap_or(22);
                                                    let user = edit.user.clone();
                                                    let auth = edit.build_auth();

                                                    if let Some(server) = app.servers.get_mut(index) {
                                                        server.name = name.clone();
                                                        server.host = host;
                                                        server.port = port;
                                                        server.user = user;
                                                        server.auth = auth;
                                                        let _ = config::save_config(
                                                            &config::AppConfig { servers: app.servers.clone() },
                                                            &config::get_config_path(),
                                                        );
                                                        app.notifications.success(&format!("Servidor '{}' atualizado!", name));
                                                    }
                                                    app.input_mode = tui::app::InputMode::Normal;
                                                    app.edit_state = None;
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
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
                                KeyCode::Enter => {
                                    if let Some(sftp) = &mut app.sftp_state {
                                        let files = sftp.local.list().unwrap_or_default();
                                        if let Some(file) = files.get(sftp.local_selected) {
                                            if file.is_dir {
                                                let _ = sftp.local.cd(&file.name);
                                                sftp.local_selected = 0;
                                            }
                                        }
                                    }
                                }
                                KeyCode::Backspace => {
                                    if let Some(sftp) = &mut app.sftp_state {
                                        let _ = sftp.local.cd("..");
                                        sftp.local_selected = 0;
                                    }
                                }
                                _ => {}
                            }
                        }
                        tui::app::CurrentView::SshTerminal => {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => app.close_ssh(),
                                KeyCode::Char(c) => {
                                    if let Some(ssh) = &mut app.ssh_state {
                                        if matches!(ssh.status, tui::ssh_terminal::SshStatus::Connected) {
                                            ssh.input.push(c);
                                        }
                                    }
                                }
                                KeyCode::Backspace => {
                                    if let Some(ssh) = &mut app.ssh_state {
                                        if matches!(ssh.status, tui::ssh_terminal::SshStatus::Connected) {
                                            ssh.input.pop();
                                        }
                                    }
                                }
                                KeyCode::Enter => {
                                    if let Some(ssh) = &mut app.ssh_state {
                                        if matches!(ssh.status, tui::ssh_terminal::SshStatus::Connected) {
                                            let cmd = ssh.input.clone();
                                            ssh.output.push(format!("> {}", cmd));

                                            if cmd.trim() == "exit" || cmd.trim() == "quit" {
                                                ssh.set_disconnected();
                                                app.current_view = tui::app::CurrentView::ServerList;
                                            } else if !cmd.trim().is_empty() {
                                                // Executar comando via SSH
                                                if let Some(server) = &ssh.server {
                                                    let server = server.clone();
                                                    let output = ssh::execute_ssh_command(&server, &cmd);
                                                    match output {
                                                        Ok(out) => {
                                                            if !out.is_empty() {
                                                                for line in out.lines() {
                                                                    ssh.add_output(line.to_string());
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            ssh.set_error(e.to_string());
                                                        }
                                                    }
                                                }
                                            }
                                            ssh.input.clear();
                                        }
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
