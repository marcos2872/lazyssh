pub mod config;
pub mod sftp;
pub mod ssh;
pub mod tui;
pub mod vault;

use anyhow::{Context, Result};
use crossterm::{
    cursor,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::{
    io,
    process::{Command, ExitStatus},
};

use tui::{render_notifications, render_server_list, render_sftp_browser, App, SftpOpResult, Theme};

struct CleanupGuard;

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
    }
}

fn reenter_tui() -> Result<()> {
    enable_raw_mode()?;
    let mut stream = io::stdout();
    execute!(stream, EnterAlternateScreen, EnableMouseCapture)?;
    Ok(())
}

fn leave_tui() -> Result<()> {
    disable_raw_mode()?;
    let mut stream = io::stdout();
    execute!(stream, LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

fn native_shell_command(server: &config::Server) -> (String, Vec<String>) {
    let mut ssh_args = vec![];

    if server.port != 22 {
        ssh_args.push("-p".to_string());
        ssh_args.push(server.port.to_string());
    }

    if server.agent_forwarding {
        ssh_args.push("-A".to_string());
    }

    if let Some(ref jump) = server.proxy_jump {
        if !jump.is_empty() {
            ssh_args.push("-J".to_string());
            ssh_args.push(jump.clone());
        }
    }

    match &server.auth {
        crate::config::models::Auth::Key { path, .. } => {
            let expanded = shellexpand::tilde(path).into_owned();
            ssh_args.push("-i".to_string());
            ssh_args.push(expanded);
        }
        crate::config::models::Auth::Password { vault_key } => {
            if !vault_key.is_empty() {
                let mut args = vec![
                    "sshpass".to_string(),
                    "-p".to_string(),
                    vault_key.clone(),
                    "ssh".to_string(),
                ];
                args.append(&mut ssh_args);
                args.push(format!("{}@{}", server.user, server.host));
                return ("sshpass".to_string(), args);
            }
        }
    }

    ssh_args.push(format!("{}@{}", server.user, server.host));
    ("ssh".to_string(), ssh_args)
}

fn run_native_shell_handoff(server: &config::Server) -> Result<ExitStatus> {
    let (command, args) = native_shell_command(server);

    leave_tui()?;

    // Reset terminal (clears screen, scrollback, and all state)
    use std::io::Write;
    let _ = std::io::stdout().write_all(b"\x1bc");
    let _ = std::io::stdout().flush();

    let status = Command::new(&command)
        .args(args)
        .status()
        .with_context(|| format!("failed to start `{}`", command))?;

    reenter_tui()?;

    Ok(status)
}

// Helper: convert crossterm key events to byte sequences for the remote PTY
#[allow(dead_code)]
fn key_event_to_bytes(ev: &KeyEvent) -> Vec<u8> {
    let mods = ev.modifiers;
    match ev.code {
        KeyCode::Char(c) => {
            if mods.contains(KeyModifiers::CONTROL) && c.is_ascii_alphabetic() {
                return vec![(c.to_ascii_lowercase() as u8) - b'a' + 1];
            }
            if mods.contains(KeyModifiers::ALT) {
                let mut bytes = vec![0x1b];
                bytes.extend_from_slice(c.encode_utf8(&mut [0u8; 4]).as_bytes());
                return bytes;
            }
            c.to_string().into_bytes()
        }
        KeyCode::Enter => b"\r".to_vec(),
        KeyCode::Backspace => b"\x7f".to_vec(),
        KeyCode::Tab => b"\t".to_vec(),
        KeyCode::Esc => b"\x1b".to_vec(),
        KeyCode::Delete => b"\x1b[3~".to_vec(),
        KeyCode::Up => b"\x1b[A".to_vec(),
        KeyCode::Down => b"\x1b[B".to_vec(),
        KeyCode::Left => b"\x1b[D".to_vec(),
        KeyCode::Right => b"\x1b[C".to_vec(),
        KeyCode::Home => b"\x1b[H".to_vec(),
        KeyCode::End => b"\x1b[F".to_vec(),
        _ => vec![],
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture, cursor::Hide)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let _guard = CleanupGuard;

    let config = config::load_or_default();
    let mut app = App::new(config.servers);

    loop {
        // Drain SFTP upload progress
        if let Some(rx) = &mut app.sftp_progress_rx {
            loop {
                use tokio::sync::mpsc::error::TryRecvError;
                match rx.try_recv() {
                    Ok(bytes) => {
                        if let Some(sftp) = &mut app.sftp_state {
                            sftp.update_transfer_progress(bytes);
                        }
                    }
                    Err(TryRecvError::Disconnected) => {
                        app.sftp_progress_rx = None;
                        break;
                    }
                    Err(TryRecvError::Empty) => break,
                }
            }
        }

        // Drain SFTP async operation results
        if let Some(rx) = &mut app.sftp_op_rx {
            let mut done = false;
            loop {
                use tokio::sync::mpsc::error::TryRecvError;
                match rx.try_recv() {
                    Ok(SftpOpResult::Upload(name)) => {
                        app.notifications.success(&format!("Enviado: {}", name));
                    }
                    Ok(SftpOpResult::Error(msg)) => {
                        if msg == "__done__" {
                            done = true;
                        } else {
                            app.notifications.error(&msg);
                        }
                    }
                    Ok(_) => {}
                    Err(TryRecvError::Disconnected) => {
                        done = true;
                        break;
                    }
                    Err(TryRecvError::Empty) => break,
                }
            }
            if done {
                app.sftp_op_rx = None;
                // Refresh remote listing
                let entry = (
                    app.sftp_state.as_ref().and_then(|s| s.session_id.clone()),
                    app.sftp_state.as_ref().map(|s| s.remote_path.clone()),
                );
                if let (Some(sid), Some(rp)) = entry {
                    if let Ok(files) = app.sftp_list_remote_dir(&sid, &rp) {
                        if let Some(sftp) = &mut app.sftp_state {
                            sftp.refresh_remote(files);
                        }
                    }
                }
                if let Some(sftp) = &mut app.sftp_state {
                    sftp.finish_transfer();
                }
            }
        }

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
                    // SSH opens in external shell — this view is not used currently
                }
            }

            // Renderizar notificações por cima de tudo
            app.notifications.clear_expired();
            render_notifications(f, &app.notifications, f.area());

            // Renderizar modal de confirmação
            if let Some(ref confirm) = app.confirm_state {
                let area = f.area();
                let width = 45u16;
                let height = 5u16;
                let x = (area.width - width) / 2;
                let y = (area.height - height) / 2;
                let rect = ratatui::layout::Rect::new(x, y, width, height);

                let msg = match &confirm.action {
                    tui::app::ConfirmAction::DeleteServer { name } => {
                        format!("Remover servidor '{}'?", name)
                    }
                };

                let lines = vec![
                    ratatui::text::Line::from(""),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled(
                            format!("  {}  ", msg),
                            ratatui::style::Style::default().fg(Theme::text()).add_modifier(ratatui::style::Modifier::BOLD),
                        ),
                    ]),
                    ratatui::text::Line::from(""),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled("  Enter:Confirmar  ", ratatui::style::Style::default().fg(Theme::success())),
                        ratatui::text::Span::styled("│  Esc:Cancelar", ratatui::style::Style::default().fg(Theme::error())),
                    ]),
                ];

                let block = ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .title(" ⚠ Confirmação ")
                    .title_style(Theme::modal_title_style())
                    .border_style(Theme::modal_border_style())
                    .style(ratatui::style::Style::default().bg(ratatui::style::Color::Black));

                f.render_widget(ratatui::widgets::Clear, rect);
                f.render_widget(ratatui::widgets::Paragraph::new(lines).block(block), rect);
            }

            // Renderizar modal de ajuda por cima de tudo
            if app.help_visible {
                tui::render_help_modal(f, &app.current_view);
            }
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            let event = event::read()?;

            match event {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        // Help modal intercepts all keys when visible
                        if app.help_visible {
                            match key.code {
                                KeyCode::Esc | KeyCode::Char('?') => {
                                    app.help_visible = false;
                                }
                                _ => {}
                            }
                            continue;
                        }

                        // Global ? handler — open help from any view
                        if key.code == KeyCode::Char('?')
                            && !matches!(app.input_mode, tui::app::InputMode::Insert | tui::app::InputMode::Edit)
                        {
                            app.help_visible = true;
                            continue;
                        }

                        match app.current_view {
                        tui::app::CurrentView::ServerList => {
                            match app.input_mode {
                                tui::app::InputMode::Normal => match key.code {
                                    KeyCode::Char('q') => app.should_quit = true,
                                    KeyCode::Char('O') => app.cycle_sort_by(),
                                    KeyCode::Char('j') | KeyCode::Down => app.next(),
                                    KeyCode::Char('k') | KeyCode::Up => app.previous(),
                                    KeyCode::Char('/') => {
                                        app.input_mode = tui::app::InputMode::Search;
                                        app.input.clear();
                                        let _ = execute!(io::stdout(), cursor::Show, cursor::SetCursorStyle::BlinkingBar);
                                    }
                                    KeyCode::Char('a') => {
                                        app.insert_state = Some(tui::app::InsertState::new());
                                        app.input_mode = tui::app::InputMode::Insert;
                                    }
                                    KeyCode::Char('i') => {
                                        app.import_ssh_config();
                                    }
                                    KeyCode::Char('t') => {
                                        if let Some(server) = app.selected_server().cloned() {
                                            app.notifications.info(&format!("Testando conexão {}...", server.name));
                                            let host = server.host.clone();
                                            let port = server.port;
                                            let name = server.name.clone();
                                            let result = tokio::task::block_in_place(|| {
                                                tokio::runtime::Handle::current().block_on(async {
                                                    crate::ssh::service::SshService::test_connection(&host, port, 5).await
                                                })
                                            });
                                            match result {
                                                Ok(()) => app.notifications.success(&format!("{}: Servidor acessível ✓", name)),
                                                Err(e) => app.notifications.error(&format!("{}: {}", name, e)),
                                            }
                                        }
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
                                                &config::AppConfig { servers: app.servers.clone(), sort_by: None },
                                                &config::get_config_path(),
                                            );
                                        }
                                    }
                                    KeyCode::Char('d') => {
                                        if let Some(server) = app.selected_server() {
                                            let name = server.name.clone();
                                            app.confirm_state = Some(tui::app::ConfirmState {
                                                action: tui::app::ConfirmAction::DeleteServer { name },
                                            });
                                            app.input_mode = tui::app::InputMode::Confirm;
                                        }
                                    }
                                    KeyCode::Char('s') => app.open_sftp(),
                                    KeyCode::Char('y') => {
                                        if let Some(server) = app.selected_server() {
                                            if tui::ssh_terminal::copy_to_clipboard(&server.host) {
                                                app.notifications.success(&format!("Copiado: {}", server.host));
                                            } else {
                                                app.notifications.error("Falha ao copiar.");
                                            }
                                        }
                                    }
                                    KeyCode::Char('Y') => {
                                        if let Some(server) = app.selected_server() {
                                            let text = format!("{}@{}:{}", server.user, server.host, server.port);
                                            if tui::ssh_terminal::copy_to_clipboard(&text) {
                                                app.notifications.success(&format!("Copiado: {}", text));
                                            } else {
                                                app.notifications.error("Falha ao copiar.");
                                            }
                                        }
                                    }
                                    KeyCode::Enter => {
                                        if let Some(server) = app.selected_server().cloned() {
                                            match run_native_shell_handoff(&server) {
                                                Ok(status) => {
                                                    let message = if status.success() {
                                                        "Conexão SSH encerrada."
                                                    } else {
                                                        "SSH saiu sem término bem-sucedido."
                                                    };
                                                    app.notifications.info(message);
                                                    let _ = terminal.clear();
                                                }
                                                Err(e) => app.notifications.error(&format!("Falha ao abrir SSH: {}", e)),
                                            }
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
                                                            last_connected: None,
                                                            connection_count: 0,
            bookmarks: vec![],
            agent_forwarding: false,
            proxy_jump: None,


                                                        };
                                                        app.servers.push(server);
                                                        app.filter(&app.input.clone());
                                                        let _ = config::save_config(
                                                            &config::AppConfig { servers: app.servers.clone(), sort_by: None },
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
                                                            &config::AppConfig { servers: app.servers.clone(), sort_by: None },
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
                                    KeyCode::Enter => {
                                        app.input_mode = tui::app::InputMode::Normal;
                                        let _ = execute!(io::stdout(), cursor::Hide, cursor::SetCursorStyle::DefaultUserShape);
                                    }
                                    KeyCode::Esc => {
                                        app.input_mode = tui::app::InputMode::Normal;
                                        app.input.clear();
                                        app.filter("");
                                        let _ = execute!(io::stdout(), cursor::Hide, cursor::SetCursorStyle::DefaultUserShape);
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
                                tui::app::InputMode::Confirm => match key.code {
                                    KeyCode::Enter => {
                                        if let Some(confirm) = app.confirm_state.take() {
                                            match confirm.action {
                                                tui::app::ConfirmAction::DeleteServer { name } => {
                                                    app.servers.retain(|s| s.name != name);
                                                    app.filter(&app.input.clone());
                                                    app.notifications.success(&format!("Servidor '{}' removido.", name));
                                                    let _ = config::save_config(
                                                        &config::AppConfig { servers: app.servers.clone(), sort_by: None },
                                                        &config::get_config_path(),
                                                    );
                                                }
                                            }
                                        }
                                        app.input_mode = tui::app::InputMode::Normal;
                                    }
                                    KeyCode::Esc => {
                                        app.confirm_state = None;
                                        app.input_mode = tui::app::InputMode::Normal;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        tui::app::CurrentView::SftpBrowser => {
                            // Handle SFTP input modes (mkdir/rename)
                            if let Some(sftp) = &app.sftp_state {
                                if sftp.input_mode != tui::sftp_browser::SftpInputMode::None {
                                    match key.code {
                                        KeyCode::Esc => {
                                            if let Some(sftp) = &mut app.sftp_state {
                                                sftp.input_mode = tui::sftp_browser::SftpInputMode::None;
                                                sftp.input_buffer.clear();
                                            }
                                        }
                                        KeyCode::Enter => {
                                            let input = app.sftp_state.as_ref().map(|s| s.input_buffer.clone());
                                            let mode = app.sftp_state.as_ref().map(|s| s.input_mode.clone());
                                            if let (Some(input), Some(mode)) = (input, mode) {
                                                if !input.is_empty() {
                                                    let entry = (
                                                        app.sftp_state.as_ref().and_then(|s| s.session_id.clone()),
                                                        app.sftp_state.as_ref().map(|s| s.remote_path.clone()),
                                                    );
                                                    if let (Some(sid), Some(path)) = entry {
                                                        let result = match mode {
                                                            tui::sftp_browser::SftpInputMode::Mkdir => {
                                                                let full_path = format!("{}/{}", path, input);
                                                                app.sftp_mkdir(&sid, &full_path)
                                                            }
                                                            tui::sftp_browser::SftpInputMode::Rename => {
                                                                if let Some(old_name) = app.sftp_state.as_ref()
                                                                    .and_then(|s| s.remote_files.get(s.remote_selected))
                                                                    .map(|f| f.name.clone())
                                                                {
                                                                    let old_path = format!("{}/{}", path, old_name);
                                                                    let new_path = format!("{}/{}", path, input);
                                                                    app.sftp_rename(&sid, &old_path, &new_path)
                                                                } else {
                                                                    Err("No file selected".to_string())
                                                                }
                                                            }
                                                            tui::sftp_browser::SftpInputMode::Chmod => {
                                                                if let Some(name) = app.sftp_state.as_ref()
                                                                    .and_then(|s| s.remote_files.get(s.remote_selected))
                                                                    .map(|f| f.name.clone())
                                                                {
                                                                    let full_path = format!("{}/{}", path, name);
                                                                    if let Ok(mode) = u32::from_str_radix(input.trim(), 8) {
                                                                        app.sftp_set_permissions(&sid, &full_path, mode)
                                                                    } else {
                                                                        Err("Invalid octal permissions".to_string())
                                                                    }
                                                                } else {
                                                                    Err("No file selected".to_string())
                                                                }
                                                            }
                                                            tui::sftp_browser::SftpInputMode::Bookmark => {
                                                                let bookmark = crate::config::models::ServerBookmark {
                                                                    name: input.clone(),
                                                                    path: path.clone(),
                                                                };
                                                                // Find the server and add bookmark
                                                                let server_idx = app.filtered_indices.get(app.selected).copied();
                                                                if let Some(idx) = server_idx {
                                                                    if let Some(server) = app.servers.get_mut(idx) {
                                                                        server.bookmarks.push(bookmark);
                                                                        let _ = crate::config::save_config(
                                                                            &crate::config::AppConfig { servers: app.servers.clone(), sort_by: app.sort_by.clone() },
                                                                            &crate::config::get_config_path(),
                                                                        );
                                                                        Ok(())
                                                                    } else {
                                                                        Err("Server not found".to_string())
                                                                    }
                                                                } else {
                                                                    Err("No server selected".to_string())
                                                                }
                                                            }
                                                            _ => Ok(()),
                                                        };
                                                        match result {
                                                            Ok(()) => {
                                                                if let Some(sftp) = &mut app.sftp_state {
                                                                    sftp.input_mode = tui::sftp_browser::SftpInputMode::None;
                                                                    sftp.input_buffer.clear();
                                                                }
                                                                if let Ok(files) = app.sftp_list_remote_dir(&sid, &path) {
                                                                    if let Some(sftp) = &mut app.sftp_state {
                                                                        sftp.refresh_remote(files);
                                                                    }
                                                                }
                                                                app.notifications.success("Operação concluída!");
                                                            }
                                                            Err(e) => app.notifications.error(&format!("Erro: {}", e)),
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        KeyCode::Char(c) => {
                                            if let Some(sftp) = &mut app.sftp_state {
                                                sftp.input_buffer.push(c);
                                            }
                                        }
                                        KeyCode::Backspace => {
                                            if let Some(sftp) = &mut app.sftp_state {
                                                sftp.input_buffer.pop();
                                            }
                                        }
                                        _ => {}
                                    }
                                    continue;
                                }
                            }

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
                                    // If remote side entered, list new directory
                                    let needs_refresh = app.sftp_state.as_ref()
                                        .is_some_and(|s| s.focus_side == tui::sftp_browser::Side::Remote);
                                    if needs_refresh {
                                        let entry = (
                                            app.sftp_state.as_ref().and_then(|s| s.session_id.clone()),
                                            app.sftp_state.as_ref().map(|s| s.remote_path.clone()),
                                        );
                                        if let (Some(sid), Some(path)) = entry {
                                            match app.sftp_list_remote_dir(&sid, &path) {
                                                Ok(files) => {
                                                    if let Some(sftp) = &mut app.sftp_state {
                                                        sftp.refresh_remote(files);
                                                    }
                                                }
                                                Err(e) => app.notifications.error(&e),
                                            }
                                        }
                                    }
                                }
                                KeyCode::Backspace => {
                                    if let Some(sftp) = &mut app.sftp_state {
                                        if let Err(e) = sftp.go_parent() {
                                            app.notifications.warning(&e);
                                        }
                                    }
                                    // If remote side go_parent, list the parent directory
                                    let needs_refresh = app.sftp_state.as_ref()
                                        .is_some_and(|s| s.focus_side == tui::sftp_browser::Side::Remote);
                                    if needs_refresh {
                                        let entry = (
                                            app.sftp_state.as_ref().and_then(|s| s.session_id.clone()),
                                            app.sftp_state.as_ref().map(|s| s.remote_path.clone()),
                                        );
                                        if let (Some(sid), Some(path)) = entry {
                                            match app.sftp_list_remote_dir(&sid, &path) {
                                                Ok(files) => {
                                                    if let Some(sftp) = &mut app.sftp_state {
                                                        sftp.refresh_remote(files);
                                                    }
                                                }
                                                Err(e) => app.notifications.error(&e),
                                            }
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
                                    let mut upload_data: Option<(Vec<(String, String, String)>, Option<String>)> = None;
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

                                                // Collect paths while sftp is borrowed
                                                let session_id = sftp.session_id.clone();
                                                let remote_path = sftp.remote_path.clone();
                                                let paths: Vec<(String, String, String)> = files.iter().map(|(name, _)| {
                                                    let local = sftp.local.get_full_path(name);
                                                    let remote = format!("{}/{}", remote_path, name);
                                                    (name.clone(), local, remote)
                                                }).collect();
                                                upload_data = Some((paths, session_id));
                                            }
                                        }
                                    }

                                    // Do upload via SSH (cat | ssh, sem limite de 1GB)
                                    if let Some((ref paths, _)) = upload_data {
                                        if let Some(server) = app.selected_server().cloned() {
                                            app.notifications.info("Enviando... (barra de progresso em breve)");
                                            let server_clone = server.clone();
                                            let paths_clone = paths.clone();
                                            let (result_tx2, result_rx2) = tokio::sync::mpsc::unbounded_channel();
                                            app.sftp_op_rx = Some(result_rx2);
                                            tokio::task::spawn_blocking(move || {
                                                for (name, local, remote) in &paths_clone {
                                                    // Use: cat local | ssh user@host 'cat > remote'
                                                    let mut ssh_args = vec![];
                                                    if server_clone.port != 22 {
                                                        ssh_args.push("-p".to_string());
                                                        ssh_args.push(server_clone.port.to_string());
                                                    }
                                                    match &server_clone.auth {
                                                        crate::config::models::Auth::Key { path, .. } => {
                                                            let expanded = shellexpand::tilde(path).into_owned();
                                                            ssh_args.push("-i".to_string());
                                                            ssh_args.push(expanded);
                                                        }
                                                        crate::config::models::Auth::Password { .. } => {}
                                                    }
                                                    ssh_args.push(format!("{}@{}", server_clone.user, server_clone.host));
                                                    ssh_args.push(format!("cat > {}", remote));
                                                    let (cmd, final_args) = match &server_clone.auth {
                                                        crate::config::models::Auth::Password { vault_key } if !vault_key.is_empty() => {
                                                            let mut a = vec!["-p".to_string(), vault_key.clone(), "ssh".to_string()];
                                                            a.extend(ssh_args);
                                                            ("sshpass".to_string(), a)
                                                        }
                                                        _ => ("ssh".to_string(), ssh_args),
                                                    };
                                                    let local_file = std::fs::File::open(local)
                                                        .map_err(|e| format!("Erro ao ler {}: {}", local, e));
                                                    let child = match local_file {
                                                        Ok(f) => std::process::Command::new(&cmd)
                                                            .args(&final_args)
                                                            .stdin(f)
                                                            .stdout(std::process::Stdio::piped())
                                                            .stderr(std::process::Stdio::piped())
                                                            .spawn()
                                                            .map_err(|e| format!("ssh erro: {}", e)),
                                                        Err(e) => Err(e),
                                                    };
                                                    match child {
                                                        Ok(mut proc) => {
                                                            let _ = proc.wait();
                                                            let _ = result_tx2.send(SftpOpResult::Upload(name.clone()));
                                                        }
                                                        Err(e) => {
                                                            let _ = result_tx2.send(SftpOpResult::Error(format!("SCP erro: {}", e)));
                                                        }
                                                    }
                                                }
                                                let _ = result_tx2.send(SftpOpResult::Error("__done__".to_string()));
                                            });
                                        }
                                    }
                                }
                                KeyCode::Char('d') => {
                                    // Download arquivo(s) selecionado(s)
                                    let mut download_data: Option<(Vec<(String, String, String)>, Option<String>)> = None;
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

                                                // Collect paths while sftp is borrowed
                                                let session_id = sftp.session_id.clone();
                                                let remote_path = sftp.remote_path.clone();
                                                let paths: Vec<(String, String, String)> = files.iter().map(|(name, _)| {
                                                    let remote = format!("{}/{}", remote_path, name);
                                                    let local = sftp.local.get_full_path(name);
                                                    (name.clone(), remote, local)
                                                }).collect();
                                                download_data = Some((paths, session_id));
                                            }
                                        }
                                    }

                                    // Do download via SSH (cat remote > local)
                                    if let Some((ref paths, _)) = download_data {
                                        if let Some(server) = app.selected_server().cloned() {
                                            app.notifications.info("Baixando... (barra de progresso em breve)");
                                            let server_clone = server.clone();
                                            let paths_clone = paths.clone();
                                            let (result_tx2, result_rx2) = tokio::sync::mpsc::unbounded_channel();
                                            app.sftp_op_rx = Some(result_rx2);
                                            tokio::task::spawn_blocking(move || {
                                                for (name, remote, local) in &paths_clone {
                                                    let mut ssh_args = vec![];
                                                    if server_clone.port != 22 {
                                                        ssh_args.push("-p".to_string());
                                                        ssh_args.push(server_clone.port.to_string());
                                                    }
                                                    match &server_clone.auth {
                                                        crate::config::models::Auth::Key { path, .. } => {
                                                            let expanded = shellexpand::tilde(path).into_owned();
                                                            ssh_args.push("-i".to_string());
                                                            ssh_args.push(expanded);
                                                        }
                                                        crate::config::models::Auth::Password { .. } => {}
                                                    }
                                                    ssh_args.push(format!("{}@{}", server_clone.user, server_clone.host));
                                                    ssh_args.push(format!("cat {}", remote));
                                                    let (cmd, final_args) = match &server_clone.auth {
                                                        crate::config::models::Auth::Password { vault_key } if !vault_key.is_empty() => {
                                                            let mut a = vec!["-p".to_string(), vault_key.clone(), "ssh".to_string()];
                                                            a.extend(ssh_args);
                                                            ("sshpass".to_string(), a)
                                                        }
                                                        _ => ("ssh".to_string(), ssh_args),
                                                    };
                                                    let child = std::process::Command::new(&cmd)
                                                        .args(&final_args)
                                                        .stdout(std::process::Stdio::piped())
                                                        .stderr(std::process::Stdio::piped())
                                                        .spawn()
                                                        .map_err(|e| format!("ssh erro: {}", e));
                                                    match child {
                                                        Ok(mut proc) => {
                                                            // Read stdout and write to local file
                                                            use std::io::Read;
                                                            use std::io::Write;
                                                            let mut stdout = proc.stdout.take().unwrap();
                                                            let local_file = std::fs::File::create(local)
                                                                .map_err(|e| format!("Erro ao criar {}: {}", local, e));
                                                            match local_file {
                                                                Ok(mut f) => {
                                                                    let mut buf = [0u8; 65536];
                                                                    loop {
                                                                        match stdout.read(&mut buf) {
                                                                            Ok(0) => break,
                                                                            Ok(n) => {
                                                                                let _ = f.write_all(&buf[..n]);
                                                                            }
                                                                            Err(_) => break,
                                                                        }
                                                                    }
                                                                    let _ = proc.wait();
                                                                    let _ = result_tx2.send(SftpOpResult::Upload(name.clone()));
                                                                }
                                                                Err(e) => {
                                                                    let _ = result_tx2.send(SftpOpResult::Error(format!("Erro: {}", e)));
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            let _ = result_tx2.send(SftpOpResult::Error(format!("ssh erro: {}", e)));
                                                        }
                                                    }
                                                }
                                                let _ = result_tx2.send(SftpOpResult::Error("__done__".to_string()));
                                            });
                                        }
                                    }
                                }
                                KeyCode::Char('r') => {
                                    if let Some(sftp) = &mut app.sftp_state {
                                        sftp.refresh_local();
                                    }
                                    // Refresh remote via service
                                    let entry = (
                                        app.sftp_state.as_ref().and_then(|s| s.session_id.clone()),
                                        app.sftp_state.as_ref().map(|s| s.remote_path.clone()),
                                    );
                                    if let (Some(sid), Some(path)) = entry {
                                        match app.sftp_list_remote_dir(&sid, &path) {
                                            Ok(files) => {
                                                if let Some(sftp) = &mut app.sftp_state {
                                                    sftp.refresh_remote(files);
                                                }
                                            }
                                            Err(e) => app.notifications.error(&e),
                                        }
                                    }
                                    app.notifications.info("Atualizado!");
                                }
                                KeyCode::Char('M') => {
                                    // Create directory
                                    if let Some(sftp) = &mut app.sftp_state {
                                        if sftp.is_transferring {
                                            app.notifications.warning("Transferência em andamento!");
                                        } else {
                                            sftp.input_mode = tui::sftp_browser::SftpInputMode::Mkdir;
                                            sftp.input_buffer.clear();
                                        }
                                    }
                                }
                                KeyCode::Char('R') => {
                                    // Rename file/directory
                                    if let Some(sftp) = &mut app.sftp_state {
                                        if sftp.is_transferring {
                                            app.notifications.warning("Transferência em andamento!");
                                        } else if sftp.focus_side == tui::sftp_browser::Side::Remote {
                                            if let Some(file) = sftp.remote_files.get(sftp.remote_selected) {
                                                sftp.input_mode = tui::sftp_browser::SftpInputMode::Rename;
                                                sftp.input_buffer = file.name.clone();
                                            }
                                        }
                                    }
                                }
                                KeyCode::Char('x') => {
                                    // Remove file/directory
                                    if let Some(sftp) = &mut app.sftp_state {
                                        if sftp.is_transferring {
                                            app.notifications.warning("Transferência em andamento!");
                                        } else if sftp.focus_side == tui::sftp_browser::Side::Remote {
                                            let sid = sftp.session_id.clone();
                                            let path = sftp.remote_path.clone();
                                            let name = sftp.remote_files.get(sftp.remote_selected).map(|f| f.name.clone());
                                            let is_dir = sftp.remote_files.get(sftp.remote_selected).map(|f| f.is_dir);
                                            if let (Some(sid), Some(name), Some(is_dir)) = (sid, name, is_dir) {
                                                let full_path = format!("{}/{}", path, name);
                                                let result = if is_dir {
                                                    app.sftp_rmdir(&sid, &full_path)
                                                } else {
                                                    app.sftp_unlink(&sid, &full_path)
                                                };
                                                match result {
                                                    Ok(()) => {
                                                        app.notifications.success(&format!("Removido: {}", name));
                                                        // Refresh
                                                        if let Ok(files) = app.sftp_list_remote_dir(&sid, &path) {
                                                            if let Some(sftp) = &mut app.sftp_state {
                                                                sftp.refresh_remote(files);
                                                            }
                                                        }
                                                    }
                                                    Err(e) => app.notifications.error(&format!("Erro ao remover: {}", e)),
                                                }
                                            }
                                        }
                                    }
                                }
                                KeyCode::Char('m') => {
                                    // Chmod - change permissions
                                    if let Some(sftp) = &mut app.sftp_state {
                                        if sftp.is_transferring {
                                            app.notifications.warning("Transferência em andamento!");
                                        } else if sftp.focus_side == tui::sftp_browser::Side::Remote {
                                            if let Some(file) = sftp.remote_files.get(sftp.remote_selected) {
                                                let perm_str = file.permissions
                                                    .map(|p| format!("{:04o}", p & 0o7777))
                                                    .unwrap_or_else(|| "????".to_string());
                                                sftp.input_mode = tui::sftp_browser::SftpInputMode::Chmod;
                                                sftp.input_buffer = perm_str;
                                            }
                                        }
                                    }
                                }
                                KeyCode::Char('b') => {
                                    // Save current remote directory as bookmark
                                    if let Some(sftp) = &mut app.sftp_state {
                                        if sftp.focus_side == tui::sftp_browser::Side::Remote {
                                            sftp.input_mode = tui::sftp_browser::SftpInputMode::Bookmark;
                                            // Default name is the last component of the path
                                            let default_name = sftp.remote_path
                                                .rsplit('/')
                                                .next()
                                                .unwrap_or("root")
                                                .to_string();
                                            sftp.input_buffer = default_name;
                                        }
                                    }
                                }
                                KeyCode::Char('B') => {
                                    // Navigate to bookmark
                                    if let Some(sftp) = &app.sftp_state {
                                        if sftp.focus_side == tui::sftp_browser::Side::Remote {
                                            let server_idx = app.filtered_indices.get(app.selected).copied();
                                            if let Some(idx) = server_idx {
                                                if let Some(server) = app.servers.get(idx) {
                                                    if server.bookmarks.is_empty() {
                                                        app.notifications.info("Nenhum bookmark salvo. Use 'b' para salvar.");
                                                    } else {
                                                        // For now, show first bookmark (future: selection modal)
                                                        let first = &server.bookmarks[0];
                                                        let path = first.path.clone();
                                                        app.notifications.info(&format!("Navegando para: {}", first.name));
                                                        if let Some(sid) = sftp.session_id.clone() {
                                                            if let Ok(files) = app.sftp_list_remote_dir(&sid, &path) {
                                                                if let Some(sftp) = &mut app.sftp_state {
                                                                    sftp.remote_path = path;
                                                                    sftp.refresh_remote(files);
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        tui::app::CurrentView::SshTerminal => {
                            // SSH opens in external shell — no TUI key handling
                        }
                    }
                }
                }

                // Tratar eventos do mouse (scroll, cliques e seleção)
                Event::Mouse(mouse) => {
                    match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            if matches!(app.current_view, tui::app::CurrentView::SftpBrowser) {
                                if let Some(sftp) = &mut app.sftp_state {
                                    sftp.previous_item();
                                }
                            } else {
                                app.previous();
                            }
                        }
                        MouseEventKind::ScrollDown => {
                            if matches!(app.current_view, tui::app::CurrentView::SftpBrowser) {
                                if let Some(sftp) = &mut app.sftp_state {
                                    sftp.next_item();
                                }
                            } else {
                                app.next();
                            }
                        }
                        MouseEventKind::Down(MouseButton::Left) => {
                            if matches!(app.current_view, tui::app::CurrentView::ServerList)
                                && app.insert_state.is_none()
                                && app.edit_state.is_none()
                            {
                                if mouse.row >= 4 {
                                    let clicked_index = (mouse.row - 4) as usize;
                                    if clicked_index < app.filtered_indices.len() {
                                        app.selected = clicked_index;
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
                                        match app.selected_server().cloned() {
                                            Some(server) => match run_native_shell_handoff(&server) {
                                                Ok(status) => {
                                                    let message = if status.success() {
                                                        "Conexão SSH encerrada."
                                                    } else {
                                                        "SSH saiu sem término bem-sucedido."
                                                    };
                                                    app.notifications.info(message);
                                                    let _ = terminal.clear();
                                                }
                                                Err(e) => app.notifications.error(&format!("Falha ao abrir SSH: {}", e)),
                                            },
                                            None => {}
                                        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn test_key_event_regular_char() {
        let ev = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(key_event_to_bytes(&ev), b"a");
    }

    #[test]
    fn test_key_event_ctrl_c() {
        let ev = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(key_event_to_bytes(&ev), &[3]);
    }

    #[test]
    fn test_key_event_ctrl_d() {
        let ev = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
        assert_eq!(key_event_to_bytes(&ev), &[4]);
    }

    #[test]
    fn test_key_event_alt_char() {
        let ev = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT);
        assert_eq!(key_event_to_bytes(&ev), &[0x1b, b'x']);
    }

    #[test]
    fn test_key_event_enter() {
        let ev = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(key_event_to_bytes(&ev), b"\r");
    }

    #[test]
    fn test_key_event_backspace() {
        let ev = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!(key_event_to_bytes(&ev), b"\x7f");
    }

    #[test]
    fn test_key_event_tab() {
        let ev = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(key_event_to_bytes(&ev), b"\t");
    }

    #[test]
    fn test_key_event_esc() {
        let ev = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(key_event_to_bytes(&ev), b"\x1b");
    }

    #[test]
    fn test_key_event_delete() {
        let ev = KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE);
        assert_eq!(key_event_to_bytes(&ev), b"\x1b[3~");
    }

    #[test]
    fn test_key_event_arrows() {
        assert_eq!(key_event_to_bytes(&KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)), b"\x1b[A");
        assert_eq!(key_event_to_bytes(&KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)), b"\x1b[B");
        assert_eq!(key_event_to_bytes(&KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)), b"\x1b[D");
        assert_eq!(key_event_to_bytes(&KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)), b"\x1b[C");
    }

    #[test]
    fn test_key_event_home_end() {
        assert_eq!(key_event_to_bytes(&KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)), b"\x1b[H");
        assert_eq!(key_event_to_bytes(&KeyEvent::new(KeyCode::End, KeyModifiers::NONE)), b"\x1b[F");
    }

    #[test]
    fn test_key_event_unmapped_returns_empty() {
        let ev = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
        assert!(key_event_to_bytes(&ev).is_empty());
    }

    #[test]
    fn test_native_shell_command_key_auth() {
        let server = crate::config::models::Server {
            name: "test".into(),
            host: "example.com".into(),
            port: 22,
            user: "root".into(),
            auth: crate::config::models::Auth::Key {
                path: "~/.ssh/id_ed25519".into(),
                passphrase: None,
            },
            tags: vec![],
            pinned: false,
                                                            last_connected: None,
                                                            connection_count: 0,
            bookmarks: vec![],
            agent_forwarding: false,
            proxy_jump: None,


        };
        let (cmd, args) = native_shell_command(&server);
        assert_eq!(cmd, "ssh");
        assert!(args.contains(&"-i".to_string()));
        assert!(args.contains(&"root@example.com".to_string()));
    }

    #[test]
    fn test_native_shell_command_password_auth() {
        let server = crate::config::models::Server {
            name: "test".into(),
            host: "example.com".into(),
            port: 22,
            user: "root".into(),
            auth: crate::config::models::Auth::Password {
                vault_key: "secret123".into(),
            },
            tags: vec![],
            pinned: false,
                                                            last_connected: None,
                                                            connection_count: 0,
            bookmarks: vec![],
            agent_forwarding: false,
            proxy_jump: None,


        };
        let (cmd, args) = native_shell_command(&server);
        assert_eq!(cmd, "sshpass");
        assert_eq!(args[0], "sshpass");
        assert_eq!(args[1], "-p");
        assert_eq!(args[2], "secret123");
        assert_eq!(args[3], "ssh");
        assert!(args.contains(&"root@example.com".to_string()));
    }

    #[test]
    fn test_native_shell_command_custom_port() {
        let server = crate::config::models::Server {
            name: "test".into(),
            host: "example.com".into(),
            port: 2222,
            user: "admin".into(),
            auth: crate::config::models::Auth::Password {
                vault_key: "pass".into(),
            },
            tags: vec![],
            pinned: false,
            last_connected: None,
            connection_count: 0,
            bookmarks: vec![],
            agent_forwarding: false,
            proxy_jump: None,


        };
        let (cmd, args) = native_shell_command(&server);
        assert_eq!(cmd, "sshpass");
        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"2222".to_string()));
        assert!(args.contains(&"admin@example.com".to_string()));
    }

    // --- T3.3: agent_forwarding ---

    #[test]
    fn test_native_shell_command_agent_forwarding() {
        let server = crate::config::models::Server {
            name: "af-test".into(),
            host: "10.0.0.1".into(),
            port: 22,
            user: "root".into(),
            auth: crate::config::models::Auth::Key {
                path: "~/.ssh/id_rsa".into(),
                passphrase: None,
            },
            tags: vec![],
            pinned: false,
            last_connected: None,
            connection_count: 0,
            bookmarks: vec![],
            agent_forwarding: true,
            proxy_jump: None,


        };
        let (_, args) = native_shell_command(&server);
        assert!(args.contains(&"-A".to_string()), "should contain -A flag");
    }

    #[test]
    fn test_native_shell_command_no_agent_forwarding() {
        let server = crate::config::models::Server {
            name: "no-af".into(),
            host: "10.0.0.1".into(),
            port: 22,
            user: "root".into(),
            auth: crate::config::models::Auth::Key {
                path: "~/.ssh/id_rsa".into(),
                passphrase: None,
            },
            tags: vec![],
            pinned: false,
            last_connected: None,
            connection_count: 0,
            bookmarks: vec![],
            agent_forwarding: false,
            proxy_jump: None,


        };
        let (_, args) = native_shell_command(&server);
        assert!(!args.contains(&"-A".to_string()), "should NOT contain -A");
    }

    // --- T3.4: proxy_jump ---

    #[test]
    fn test_native_shell_command_proxy_jump() {
        let server = crate::config::models::Server {
            name: "pj-test".into(),
            host: "internal.dev".into(),
            port: 22,
            user: "deploy".into(),
            auth: crate::config::models::Auth::Key {
                path: "~/.ssh/id_rsa".into(),
                passphrase: None,
            },
            tags: vec![],
            pinned: false,
            last_connected: None,
            connection_count: 0,
            bookmarks: vec![],
            agent_forwarding: false,
            proxy_jump: Some("user@bastion.example.com".to_string()),


        };
        let (_, args) = native_shell_command(&server);
        assert!(args.contains(&"-J".to_string()), "should contain -J flag");
        assert!(args.contains(&"user@bastion.example.com".to_string()), "should contain jump host");
    }

    #[test]
    fn test_native_shell_command_no_proxy_jump() {
        let server = crate::config::models::Server {
            name: "no-pj".into(),
            host: "direct.dev".into(),
            port: 22,
            user: "root".into(),
            auth: crate::config::models::Auth::Key {
                path: "~/.ssh/id_rsa".into(),
                passphrase: None,
            },
            tags: vec![],
            pinned: false,
            last_connected: None,
            connection_count: 0,
            bookmarks: vec![],
            agent_forwarding: false,
            proxy_jump: None,


        };
        let (_, args) = native_shell_command(&server);
        assert!(!args.contains(&"-J".to_string()), "should NOT contain -J");
    }

    #[test]
    fn test_native_shell_command_empty_proxy_jump() {
        let server = crate::config::models::Server {
            name: "empty-pj".into(),
            host: "direct.dev".into(),
            port: 22,
            user: "root".into(),
            auth: crate::config::models::Auth::Key {
                path: "~/.ssh/id_rsa".into(),
                passphrase: None,
            },
            tags: vec![],
            pinned: false,
            last_connected: None,
            connection_count: 0,
            bookmarks: vec![],
            agent_forwarding: false,
            proxy_jump: Some("".to_string()),


        };
        let (_, args) = native_shell_command(&server);
        assert!(!args.contains(&"-J".to_string()), "empty proxy_jump should be ignored");
    }
}
