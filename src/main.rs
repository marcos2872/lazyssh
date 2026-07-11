pub mod config;
pub mod sftp;
pub mod ssh;
pub mod tui;
pub mod vault;

use anyhow::{Context, Result};
use crossterm::{
    cursor,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
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

use tui::{render_notifications, render_server_list, render_sftp_browser, App, SftpOpResult};

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

fn native_shell_command(server: &config::Server) -> (String, Vec<String>, Option<String>) {
    crate::ssh::args::build_ssh_args(server)
}

fn run_native_shell_handoff(server: &config::Server) -> Result<ExitStatus> {
    let (command, args, password) = native_shell_command(server);

    leave_tui()?;

    // Reset terminal (clears screen, scrollback, and all state)
    use std::io::Write;
    let _ = std::io::stdout().write_all(b"\x1bc");
    let _ = std::io::stdout().flush();

    let mut cmd = Command::new(&command);
    cmd.args(&args);
    if let Some(ref pw) = password {
        cmd.env("SSHPASS", pw);
    }

    let status = cmd
        .status()
        .with_context(|| format!("failed to start `{}`", command))?;

    reenter_tui()?;

    Ok(status)
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
                    if app.form_state.is_some() {
                        let area = f.area();
                        let overlay = ratatui::widgets::Block::default()
                            .style(ratatui::style::Style::default().bg(ratatui::style::Color::Black).add_modifier(ratatui::style::Modifier::DIM));
                        f.render_widget(overlay, area);
                    }

                    if let Some(ref form) = app.form_state {
                        tui::modals::render_form_modal(f, form);
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
                tui::modals::render_confirm_modal(f, confirm);
            }

            // Renderizar modal de ajuda por cima de tudo
            if app.overlay == Some(tui::app::Overlay::Help) {
                tui::render_help_modal(f, &app.current_view);
            }
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            let event = event::read()?;

            match event {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        // Help modal intercepts all keys when visible
                        if tui::handlers::handle_help_key(&mut app, key) {
                            continue;
                        }
                        // Global ? handler
                        if tui::handlers::handle_global_keys(&mut app, key) {
                            continue;
                        }

                        match app.current_view {
                            tui::app::CurrentView::ServerList => {
                                match tui::handlers::handle_server_list_key(&mut app, key) {
                                    tui::handlers::HandlerResult::Quit => break,
                                    tui::handlers::HandlerResult::ConnectSsh(server) => {
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
                                    tui::handlers::HandlerResult::None => {}
                                }
                            }
                            tui::app::CurrentView::SftpBrowser => {
                                match tui::handlers::handle_sftp_key(&mut app, key) {
                                    tui::handlers::HandlerResult::Quit => break,
                                    tui::handlers::HandlerResult::ConnectSsh(server) => {
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
                                    tui::handlers::HandlerResult::None => {}
                                }
                            }
                            tui::app::CurrentView::SshTerminal => {}
                        }
                    }
                }

                Event::Mouse(mouse) => {
                    match tui::handlers::handle_mouse_event(&mut app, mouse) {
                        tui::handlers::HandlerResult::ConnectSsh(server) => {
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
        let (cmd, args, _pw) = native_shell_command(&server);
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
        let (cmd, args, pw) = native_shell_command(&server);
        assert_eq!(cmd, "sshpass");
        assert_eq!(args[0], "ssh");
        assert!(args.contains(&"root@example.com".to_string()));
        assert_eq!(pw, Some("secret123".into()));
        // Password must NOT appear in args
        assert!(!args.contains(&"secret123".to_string()));
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
        let (cmd, args, pw) = native_shell_command(&server);
        assert_eq!(cmd, "sshpass");
        assert!(args.contains(&"2222".to_string()));
        assert!(args.contains(&"admin@example.com".to_string()));
        assert_eq!(pw, Some("pass".into()));
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
        let (_, args, _) = native_shell_command(&server);
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
        let (_, args, _) = native_shell_command(&server);
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
        let (_, args, _) = native_shell_command(&server);
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
        let (_, args, _) = native_shell_command(&server);
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
        let (_, args, _) = native_shell_command(&server);
        assert!(!args.contains(&"-J".to_string()), "empty proxy_jump should be ignored");
    }
}
