pub mod config;
pub mod sftp;
pub mod ssh;
pub mod tui;
pub mod vault;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    Terminal,
};
use std::io;

use tui::{render_notifications, render_server_list, render_sftp_browser, render_ssh_terminal, App, Theme};

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

                    // Overlay escuro quando modal está aberto
                    if app.insert_state.is_some() || app.edit_state.is_some() {
                        let area = f.area();
                        let overlay = ratatui::widgets::Block::default()
                            .style(ratatui::style::Style::default().bg(ratatui::style::Color::Black).add_modifier(ratatui::style::Modifier::DIM));
                        f.render_widget(overlay, area);
                    }

                    if let Some(ref state) = app.insert_state {
                        let area = f.area();
                        let is_key = state.is_key_auth();
                        let height: u16 = if is_key { 14 } else { 12 };
                        let width: u16 = 50;
                        let x = (area.width - width) / 2;
                        let y = (area.height - height) / 2;
                        let rect = ratatui::layout::Rect::new(x, y, width, height);

                        let mut lines = vec![];

                        // Título estilizado
                        lines.push(ratatui::text::Line::from(vec![
                            ratatui::text::Span::styled("  ➕ Novo Servidor  ", 
                                ratatui::style::Style::default().fg(Theme::accent()).add_modifier(ratatui::style::Modifier::BOLD)),
                        ]));
                        lines.push(ratatui::text::Line::from("─".repeat(width as usize - 2)));

                        // Campos fixos
                        let base_fields = [
                            (tui::app::InsertField::Name, "📝 Nome", &state.name),
                            (tui::app::InsertField::Host, "🌐 Host", &state.host),
                            (tui::app::InsertField::Port, "🔌 Porta", &state.port),
                            (tui::app::InsertField::User, "👤 Usuário", &state.user),
                            (tui::app::InsertField::AuthType, "🔐 Auth", &state.auth_type),
                        ];

                        for (field_type, label, value) in &base_fields {
                            let is_active = &state.field == field_type;
                            let marker = if is_active { "▶" } else { " " };
                            let style = if is_active { 
                                ratatui::style::Style::default().fg(Theme::primary()).add_modifier(ratatui::style::Modifier::BOLD)
                            } else { 
                                ratatui::style::Style::default().fg(Theme::text()) 
                            };
                            lines.push(ratatui::text::Line::from(vec![
                                ratatui::text::Span::styled(format!("{} ", marker), ratatui::style::Style::default().fg(Theme::accent())),
                                ratatui::text::Span::styled(format!("{}: ", label), ratatui::style::Style::default().fg(Theme::secondary())),
                                ratatui::text::Span::styled(*value, style),
                            ]));
                        }

                        // Campos dinâmicos baseado no auth type
                        if is_key {
                            let is_active = state.field == tui::app::InsertField::KeyPath;
                            let marker = if is_active { "▶" } else { " " };
                            let style = if is_active { 
                                ratatui::style::Style::default().fg(Theme::primary()).add_modifier(ratatui::style::Modifier::BOLD)
                            } else { 
                                ratatui::style::Style::default().fg(Theme::text()) 
                            };
                            lines.push(ratatui::text::Line::from(vec![
                                ratatui::text::Span::styled(format!("{} ", marker), ratatui::style::Style::default().fg(Theme::accent())),
                                ratatui::text::Span::styled("🔑 Chave: ", ratatui::style::Style::default().fg(Theme::secondary())),
                                ratatui::text::Span::styled(&state.key_path, style),
                            ]));

                            let is_active = state.field == tui::app::InsertField::Passphrase;
                            let marker = if is_active { "▶" } else { " " };
                            let style = if is_active { 
                                ratatui::style::Style::default().fg(Theme::primary()).add_modifier(ratatui::style::Modifier::BOLD)
                            } else { 
                                ratatui::style::Style::default().fg(Theme::text()) 
                            };
                            lines.push(ratatui::text::Line::from(vec![
                                ratatui::text::Span::styled(format!("{} ", marker), ratatui::style::Style::default().fg(Theme::accent())),
                                ratatui::text::Span::styled("🔑 Senha: ", ratatui::style::Style::default().fg(Theme::secondary())),
                                ratatui::text::Span::styled(&state.passphrase, style),
                            ]));
                        } else {
                            let is_active = state.field == tui::app::InsertField::Password;
                            let marker = if is_active { "▶" } else { " " };
                            let style = if is_active { 
                                ratatui::style::Style::default().fg(Theme::primary()).add_modifier(ratatui::style::Modifier::BOLD)
                            } else { 
                                ratatui::style::Style::default().fg(Theme::text()) 
                            };
                            lines.push(ratatui::text::Line::from(vec![
                                ratatui::text::Span::styled(format!("{} ", marker), ratatui::style::Style::default().fg(Theme::accent())),
                                ratatui::text::Span::styled("🔑 Senha: ", ratatui::style::Style::default().fg(Theme::secondary())),
                                ratatui::text::Span::styled(&state.password, style),
                            ]));
                        }

                        lines.push(ratatui::text::Line::from(""));
                        lines.push(ratatui::text::Line::from(vec![
                            ratatui::text::Span::styled("  ↑/↓/Tab: próximo campo  ", ratatui::style::Style::default().fg(Theme::text_dim())),
                            ratatui::text::Span::styled("│  Esc: cancelar", ratatui::style::Style::default().fg(Theme::error())),
                        ]));
                        lines.push(ratatui::text::Line::from(vec![
                            ratatui::text::Span::styled("  Enter: salvar (no último campo)  ", ratatui::style::Style::default().fg(Theme::success())),
                        ]));

                        let block = ratatui::widgets::Block::default()
                            .borders(ratatui::widgets::Borders::ALL)
                            .border_style(Theme::modal_border_style())
                            .style(ratatui::style::Style::default().bg(ratatui::style::Color::Black));

                        let input = ratatui::widgets::Paragraph::new(lines).block(block);
                        f.render_widget(input, rect);
                    } else if let Some(ref edit) = app.edit_state {
                        let area = f.area();
                        let is_key = edit.is_key_auth();
                        let height: u16 = if is_key { 15 } else { 13 };
                        let width: u16 = 50;
                        let x = (area.width - width) / 2;
                        let y = (area.height - height) / 2;
                        let rect = ratatui::layout::Rect::new(x, y, width, height);

                        let mut lines = vec![];

                        // Título estilizado
                        lines.push(ratatui::text::Line::from(vec![
                            ratatui::text::Span::styled("  ✏️ Editar Servidor  ", 
                                ratatui::style::Style::default().fg(Theme::accent()).add_modifier(ratatui::style::Modifier::BOLD)),
                        ]));
                        lines.push(ratatui::text::Line::from("─".repeat(width as usize - 2)));

                        // Campos fixos
                        let base_fields = [
                            (tui::app::EditField::Name, "📝 Nome", &edit.name),
                            (tui::app::EditField::Host, "🌐 Host", &edit.host),
                            (tui::app::EditField::Port, "🔌 Porta", &edit.port),
                            (tui::app::EditField::User, "👤 Usuário", &edit.user),
                            (tui::app::EditField::AuthType, "🔐 Auth", &edit.auth_type),
                        ];

                        for (field_type, label, value) in &base_fields {
                            let is_active = &edit.field == field_type;
                            let marker = if is_active { "▶" } else { " " };
                            let style = if is_active { 
                                ratatui::style::Style::default().fg(Theme::primary()).add_modifier(ratatui::style::Modifier::BOLD)
                            } else { 
                                ratatui::style::Style::default().fg(Theme::text()) 
                            };
                            lines.push(ratatui::text::Line::from(vec![
                                ratatui::text::Span::styled(format!("{} ", marker), ratatui::style::Style::default().fg(Theme::accent())),
                                ratatui::text::Span::styled(format!("{}: ", label), ratatui::style::Style::default().fg(Theme::secondary())),
                                ratatui::text::Span::styled(*value, style),
                            ]));
                        }

                        // Campos dinâmicos baseado no auth type
                        if is_key {
                            let is_active = edit.field == tui::app::EditField::KeyPath;
                            let marker = if is_active { "▶" } else { " " };
                            let style = if is_active { 
                                ratatui::style::Style::default().fg(Theme::primary()).add_modifier(ratatui::style::Modifier::BOLD)
                            } else { 
                                ratatui::style::Style::default().fg(Theme::text()) 
                            };
                            lines.push(ratatui::text::Line::from(vec![
                                ratatui::text::Span::styled(format!("{} ", marker), ratatui::style::Style::default().fg(Theme::accent())),
                                ratatui::text::Span::styled("🔑 Chave: ", ratatui::style::Style::default().fg(Theme::secondary())),
                                ratatui::text::Span::styled(&edit.key_path, style),
                            ]));

                            let is_active = edit.field == tui::app::EditField::Passphrase;
                            let marker = if is_active { "▶" } else { " " };
                            let style = if is_active { 
                                ratatui::style::Style::default().fg(Theme::primary()).add_modifier(ratatui::style::Modifier::BOLD)
                            } else { 
                                ratatui::style::Style::default().fg(Theme::text()) 
                            };
                            lines.push(ratatui::text::Line::from(vec![
                                ratatui::text::Span::styled(format!("{} ", marker), ratatui::style::Style::default().fg(Theme::accent())),
                                ratatui::text::Span::styled("🔑 Senha: ", ratatui::style::Style::default().fg(Theme::secondary())),
                                ratatui::text::Span::styled(&edit.passphrase, style),
                            ]));
                        } else {
                            let is_active = edit.field == tui::app::EditField::Password;
                            let marker = if is_active { "▶" } else { " " };
                            let style = if is_active { 
                                ratatui::style::Style::default().fg(Theme::primary()).add_modifier(ratatui::style::Modifier::BOLD)
                            } else { 
                                ratatui::style::Style::default().fg(Theme::text()) 
                            };
                            lines.push(ratatui::text::Line::from(vec![
                                ratatui::text::Span::styled(format!("{} ", marker), ratatui::style::Style::default().fg(Theme::accent())),
                                ratatui::text::Span::styled("🔑 Senha: ", ratatui::style::Style::default().fg(Theme::secondary())),
                                ratatui::text::Span::styled(&edit.password, style),
                            ]));
                        }

                        lines.push(ratatui::text::Line::from(""));
                        lines.push(ratatui::text::Line::from(vec![
                            ratatui::text::Span::styled("  ↑/↓/Tab: próximo campo  ", ratatui::style::Style::default().fg(Theme::text_dim())),
                            ratatui::text::Span::styled("│  Esc: cancelar", ratatui::style::Style::default().fg(Theme::error())),
                        ]));
                        lines.push(ratatui::text::Line::from(vec![
                            ratatui::text::Span::styled("  Enter: salvar (no último campo)  ", ratatui::style::Style::default().fg(Theme::success())),
                        ]));

                        let block = ratatui::widgets::Block::default()
                            .borders(ratatui::widgets::Borders::ALL)
                            .border_style(Theme::modal_border_style())
                            .style(ratatui::style::Style::default().bg(ratatui::style::Color::Black));

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
            let event = event::read()?;

            match event {
                Event::Key(key) => {
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
                                KeyCode::Char('q') | KeyCode::Esc => {
                                    if let Some(sftp) = &mut app.sftp_state {
                                        if !sftp.local_selected_files.is_empty() || !sftp.remote_selected_files.is_empty() {
                                            sftp.clear_selection();
                                        } else {
                                            app.close_sftp();
                                        }
                                    }
                                }
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
                                        if let Err(e) = sftp.enter_directory() {
                                            app.notifications.warning(&e);
                                        }
                                    }
                                }
                                KeyCode::Backspace => {
                                    if let Some(sftp) = &mut app.sftp_state {
                                        if let Err(e) = sftp.go_parent() {
                                            app.notifications.warning(&e);
                                        }
                                    }
                                }
                                KeyCode::Char(' ') => {
                                    // Selecionar/desselecionar arquivo
                                    if let Some(sftp) = &mut app.sftp_state {
                                        sftp.toggle_select_current();
                                    }
                                }
                                KeyCode::Char('a') => {
                                    // Selecionar todos
                                    if let Some(sftp) = &mut app.sftp_state {
                                        sftp.select_all_current();
                                        let count = match sftp.focus_side {
                                            tui::sftp_browser::Side::Local => sftp.local_selected_files.len(),
                                            tui::sftp_browser::Side::Remote => sftp.remote_selected_files.len(),
                                        };
                                        app.notifications.info(&format!("{} arquivo(s) selecionado(s)", count));
                                    }
                                }
                                KeyCode::Char('u') => {
                                    // Upload arquivo(s) selecionado(s)
                                    if let Some(sftp) = &mut app.sftp_state {
                                        if sftp.is_transferring {
                                            app.notifications.warning("Transferência em andamento!");
                                        } else {
                                            let files: Vec<(String, u64)> = match sftp.focus_side {
                                                tui::sftp_browser::Side::Local => {
                                                    if !sftp.local_selected_files.is_empty() {
                                                        sftp.local_selected_files.iter()
                                                            .filter_map(|&i| sftp.local_files.get(i))
                                                            .filter(|f| !f.is_dir)
                                                            .map(|f| (f.name.clone(), f.size))
                                                            .collect()
                                                    } else if let Some(file) = sftp.local_files.get(sftp.local_selected) {
                                                        if !file.is_dir {
                                                            vec![(file.name.clone(), file.size)]
                                                        } else {
                                                            vec![]
                                                        }
                                                    } else {
                                                        vec![]
                                                    }
                                                }
                                                tui::sftp_browser::Side::Remote => vec![],
                                            };

                                            if files.is_empty() {
                                                app.notifications.warning("Nenhum arquivo para upload");
                                            } else {
                                                let total_size: u64 = files.iter().map(|(_, s)| s).sum();
                                                let file_name = if files.len() == 1 {
                                                    files[0].0.clone()
                                                } else {
                                                    format!("{} arquivos", files.len())
                                                };
                                                sftp.start_transfer(file_name, total_size, true);
                                                app.notifications.info(&format!("Enviando {} arquivo(s)...", files.len()));

                                                // Upload via SCP
                                                let server = sftp.remote.server.clone();
                                                if let Some(server) = server {
                                                    for (name, _) in &files {
                                                        let local_path = sftp.local.get_full_path(name);
                                                        let remote_path = format!("{}/{}", sftp.remote.current_dir, name);
                                                        match sftp.remote.upload(&local_path, &remote_path) {
                                                            Ok(_) => {
                                                                app.notifications.success(&format!("Enviado: {}", name));
                                                            }
                                                            Err(e) => {
                                                                app.notifications.error(&format!("Erro ao enviar {}: {}", name, e));
                                                            }
                                                        }
                                                    }
                                                    sftp.refresh_remote();
                                                    sftp.finish_transfer();
                                                }
                                            }
                                        }
                                    }
                                }
                                KeyCode::Char('d') => {
                                    // Download arquivo(s) selecionado(s)
                                    if let Some(sftp) = &mut app.sftp_state {
                                        if sftp.is_transferring {
                                            app.notifications.warning("Transferência em andamento!");
                                        } else {
                                            let files: Vec<(String, u64)> = match sftp.focus_side {
                                                tui::sftp_browser::Side::Remote => {
                                                    if !sftp.remote_selected_files.is_empty() {
                                                        sftp.remote_selected_files.iter()
                                                            .filter_map(|&i| sftp.remote_files.get(i))
                                                            .filter(|f| !f.is_dir)
                                                            .map(|f| (f.name.clone(), f.size))
                                                            .collect()
                                                    } else if let Some(file) = sftp.remote_files.get(sftp.remote_selected) {
                                                        if !file.is_dir {
                                                            vec![(file.name.clone(), file.size)]
                                                        } else {
                                                            vec![]
                                                        }
                                                    } else {
                                                        vec![]
                                                    }
                                                }
                                                tui::sftp_browser::Side::Local => vec![],
                                            };

                                            if files.is_empty() {
                                                app.notifications.warning("Nenhum arquivo para download");
                                            } else {
                                                let total_size: u64 = files.iter().map(|(_, s)| s).sum();
                                                let file_name = if files.len() == 1 {
                                                    files[0].0.clone()
                                                } else {
                                                    format!("{} arquivos", files.len())
                                                };
                                                sftp.start_transfer(file_name, total_size, false);
                                                app.notifications.info(&format!("Baixando {} arquivo(s)...", files.len()));

                                                // Download via SCP
                                                for (name, _) in &files {
                                                    let remote_path = format!("{}/{}", sftp.remote.current_dir, name);
                                                    let local_path = sftp.local.get_full_path(name);
                                                    match sftp.remote.download(&remote_path, &local_path) {
                                                        Ok(_) => {
                                                            app.notifications.success(&format!("Baixado: {}", name));
                                                        }
                                                        Err(e) => {
                                                            app.notifications.error(&format!("Erro ao baixar {}: {}", name, e));
                                                        }
                                                    }
                                                }
                                                sftp.refresh_local();
                                                sftp.finish_transfer();
                                            }
                                        }
                                    }
                                }
                                KeyCode::Char('r') => {
                                    if let Some(sftp) = &mut app.sftp_state {
                                        sftp.refresh_remote();
                                        sftp.refresh_local();
                                        app.notifications.info("Atualizado!");
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
                                            ssh.insert_char(c);
                                        }
                                    }
                                }
                                KeyCode::Backspace => {
                                    if let Some(ssh) = &mut app.ssh_state {
                                        if matches!(ssh.status, tui::ssh_terminal::SshStatus::Connected) {
                                            ssh.delete_char_backward();
                                        }
                                    }
                                }
                                KeyCode::Delete => {
                                    if let Some(ssh) = &mut app.ssh_state {
                                        if matches!(ssh.status, tui::ssh_terminal::SshStatus::Connected) {
                                            ssh.delete_char_forward();
                                        }
                                    }
                                }
                                KeyCode::Left => {
                                    if let Some(ssh) = &mut app.ssh_state {
                                        if matches!(ssh.status, tui::ssh_terminal::SshStatus::Connected) {
                                            ssh.move_cursor_left();
                                        }
                                    }
                                }
                                KeyCode::Right => {
                                    if let Some(ssh) = &mut app.ssh_state {
                                        if matches!(ssh.status, tui::ssh_terminal::SshStatus::Connected) {
                                            ssh.move_cursor_right();
                                        }
                                    }
                                }
                                KeyCode::Home => {
                                    if let Some(ssh) = &mut app.ssh_state {
                                        if matches!(ssh.status, tui::ssh_terminal::SshStatus::Connected) {
                                            ssh.move_cursor_home();
                                        }
                                    }
                                }
                                KeyCode::End => {
                                    if let Some(ssh) = &mut app.ssh_state {
                                        if matches!(ssh.status, tui::ssh_terminal::SshStatus::Connected) {
                                            ssh.move_cursor_end();
                                        }
                                    }
                                }
                                KeyCode::PageUp => {
                                    if let Some(ssh) = &mut app.ssh_state {
                                        ssh.scroll_page_up(10);
                                    }
                                }
                                KeyCode::PageDown => {
                                    if let Some(ssh) = &mut app.ssh_state {
                                        ssh.scroll_page_down(10);
                                    }
                                }
                                KeyCode::Enter => {
                                    if let Some(ssh) = &mut app.ssh_state {
                                        if matches!(ssh.status, tui::ssh_terminal::SshStatus::Connected) {
                                            let cmd = ssh.input.clone();
                                            ssh.output.push(format!("{}{}", ssh.prompt(), cmd));

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
                                                            // Mostrar erro mas não fechar conexão
                                                            ssh.add_output(format!("Erro: {}", e));
                                                        }
                                                    }
                                                }
                                            }
                                            ssh.clear_input();
                                            ssh.scroll_to_bottom();
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

                // Tratar eventos do mouse (scroll, cliques e seleção)
                Event::Mouse(mouse) => {
                    match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            if matches!(app.current_view, tui::app::CurrentView::SshTerminal) {
                                if let Some(ssh) = &mut app.ssh_state {
                                    ssh.scroll_up(3);
                                }
                            } else if matches!(app.current_view, tui::app::CurrentView::SftpBrowser) {
                                if let Some(sftp) = &mut app.sftp_state {
                                    sftp.previous_item();
                                }
                            } else {
                                app.previous();
                            }
                        }
                        MouseEventKind::ScrollDown => {
                            if matches!(app.current_view, tui::app::CurrentView::SshTerminal) {
                                if let Some(ssh) = &mut app.ssh_state {
                                    ssh.scroll_down(3);
                                }
                            } else if matches!(app.current_view, tui::app::CurrentView::SftpBrowser) {
                                if let Some(sftp) = &mut app.sftp_state {
                                    sftp.next_item();
                                }
                            } else {
                                app.next();
                            }
                        }
                        MouseEventKind::Down(MouseButton::Left) => {
                            // Iniciar seleção no terminal SSH
                            if matches!(app.current_view, tui::app::CurrentView::SshTerminal) {
                                if let Some(ssh) = &mut app.ssh_state {
                                    // Coordenadas do mouse são absolutas na tela
                                    // Borda superior = row 1, borda esquerda = col 1
                                    let row = mouse.row.saturating_sub(1) as usize; // -1 para borda
                                    let col = mouse.column.saturating_sub(1) as usize; // -1 para borda
                                    let total_lines = ssh.output.len();

                                    if total_lines > 0 && row < total_lines {
                                        ssh.start_selection(row, col);
                                    }
                                }
                            }
                            // Clique na lista de servidores
                            if matches!(app.current_view, tui::app::CurrentView::ServerList)
                                && app.insert_state.is_none()
                                && app.edit_state.is_none()
                            {
                                // Borda + barra de busca = row 4 para conteúdo
                                if mouse.row >= 4 {
                                    let clicked_index = (mouse.row - 4) as usize;
                                    if clicked_index < app.filtered_indices.len() {
                                        app.selected = clicked_index;
                                    }
                                }
                            }
                        }
                        MouseEventKind::Drag(MouseButton::Left) => {
                            // Atualizar seleção no terminal SSH
                            if matches!(app.current_view, tui::app::CurrentView::SshTerminal) {
                                if let Some(ssh) = &mut app.ssh_state {
                                    let row = mouse.row.saturating_sub(1) as usize;
                                    let col = mouse.column.saturating_sub(1) as usize;
                                    let total_lines = ssh.output.len();

                                    if total_lines > 0 && row < total_lines {
                                        ssh.update_selection(row, col);
                                    }
                                }
                            }
                        }
                        MouseEventKind::Up(MouseButton::Left) => {
                            // Finalizar seleção e copiar
                            if matches!(app.current_view, tui::app::CurrentView::SshTerminal) {
                                if let Some(ssh) = &mut app.ssh_state {
                                    ssh.end_selection();
                                    if ssh.selection.is_some() {
                                        if ssh.copy_selection_to_clipboard() {
                                            app.notifications.success("Texto copiado!");
                                        }
                                    }
                                }
                            }
                        }
                        MouseEventKind::Down(MouseButton::Right) => {
                            // Duplo clique direito para conectar
                            if matches!(app.current_view, tui::app::CurrentView::ServerList)
                                && app.insert_state.is_none()
                                && app.edit_state.is_none()
                            {
                                if mouse.row >= 4 {
                                    let clicked_index = (mouse.row - 4) as usize;
                                    if clicked_index < app.filtered_indices.len() {
                                        app.selected = clicked_index;
                                        app.connect_ssh();
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
