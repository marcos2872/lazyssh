use crossterm::event::KeyCode;
use crossterm::execute;
use std::io;

use crate::config;
use crate::sftp::transfer;
use crate::ssh;
use super::app::{
    App, ConfirmAction, ConfirmState, CurrentView, FormMode, InputMode, Overlay, SftpOpResult,
};
use super::sftp_browser::{SftpInputMode, Side, TransferState};
use super::ssh_terminal;

/// Resultado do manipulação de um evento de tecla.
pub enum HandlerResult {
    /// Nenhuma ação especial — continuar o loop.
    None,
    /// Solicita encerramento da aplicação.
    Quit,
    /// Solicita abertura de conexão SSH com o servidor informado.
    ConnectSsh(config::models::Server),
}

/// Manipula teclas quando o modal de ajuda está aberto. Retorna `true` se a tecla foi consumida.
pub fn handle_help_key(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    if app.overlay == Some(Overlay::Help) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('?') => {
                app.overlay = None;
            }
            _ => {}
        }
        return true;
    }
    false
}

/// Tecla global `?` — abre ajuda de qualquer visão. Retorna `true` se tratada.
pub fn handle_global_keys(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    if key.code == KeyCode::Char('?')
        && !matches!(app.input_mode, InputMode::Insert | InputMode::Edit)
    {
        app.overlay = Some(Overlay::Help);
        return true;
    }
    false
}

/// Manipula teclas da lista de servidores. Retorna `HandlerResult`.
pub fn handle_server_list_key(app: &mut App, key: crossterm::event::KeyEvent) -> HandlerResult {
    match app.input_mode {
        InputMode::Normal => match key.code {
            KeyCode::Char('q') => return HandlerResult::Quit,
            KeyCode::Char('O') => app.cycle_sort_by(),
            KeyCode::Char('j') | KeyCode::Down => app.next(),
            KeyCode::Char('k') | KeyCode::Up => app.previous(),
            KeyCode::Char('/') => {
                app.input_mode = InputMode::Search;
                app.input.clear();
                let _ = execute!(io::stdout(), crossterm::cursor::Show, crossterm::cursor::SetCursorStyle::BlinkingBar);
            }
            KeyCode::Char('a') => {
                app.form_state = Some(super::app::FormState::new_insert());
                app.input_mode = InputMode::Insert;
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
                            ssh::SshService::test_connection(&host, port, 5).await
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
                    app.form_state = Some(super::app::FormState::new_edit(server, index));
                    app.input_mode = InputMode::Edit;
                }
            }
            KeyCode::Char('p') => {
                if let Some(server) = app.selected_server_mut() {
                    server.pinned = !server.pinned;
                    let status = if server.pinned { "fixado" } else { "desafixado" };
                    app.notifications.success(&format!("Servidor {}!", status));
                    if let Err(e) = config::save_config(
                        &config::AppConfig { servers: app.servers.clone(), sort_by: None },
                        &config::get_config_path(),
                    ) {
                        app.notifications.warning(&format!("Falha ao salvar config: {}", e));
                    }
                }
            }
            KeyCode::Char('d') => {
                if let Some(server) = app.selected_server() {
                    let name = server.name.clone();
                    app.confirm_state = Some(ConfirmState {
                        action: ConfirmAction::DeleteServer { name },
                    });
                    app.input_mode = InputMode::Confirm;
                }
            }
            KeyCode::Char('s') => app.open_sftp(),
            KeyCode::Char('y') => {
                if let Some(server) = app.selected_server() {
                    if ssh_terminal::copy_to_clipboard(&server.host) {
                        app.notifications.success(&format!("Copiado: {}", server.host));
                    } else {
                        app.notifications.error("Falha ao copiar.");
                    }
                }
            }
            KeyCode::Char('Y') => {
                if let Some(server) = app.selected_server() {
                    let text = format!("{}@{}:{}", server.user, server.host, server.port);
                    if ssh_terminal::copy_to_clipboard(&text) {
                        app.notifications.success(&format!("Copiado: {}", text));
                    } else {
                        app.notifications.error("Falha ao copiar.");
                    }
                }
            }
            KeyCode::Enter => {
                if let Some(server) = app.selected_server().cloned() {
                    return HandlerResult::ConnectSsh(server);
                }
            }
            _ => {}
        },
        InputMode::Insert | InputMode::Edit => {
            if let Some(ref mut form) = app.form_state {
                match key.code {
                    KeyCode::Esc => {
                        app.input_mode = InputMode::Normal;
                        app.form_state = None;
                    }
                    KeyCode::Tab | KeyCode::Down => {
                        form.next_field();
                    }
                    KeyCode::Up => {
                        form.prev_field();
                    }
                    KeyCode::Char(' ') if form.field == super::app::FormField::AuthType => {
                        form.toggle_auth_type();
                    }
                    KeyCode::Left | KeyCode::Right if form.field == super::app::FormField::AuthType => {
                        form.toggle_auth_type();
                    }
                    KeyCode::Char(c) => {
                        form.current_value_mut().push(c);
                    }
                    KeyCode::Backspace => {
                        form.current_value_mut().pop();
                    }
                    KeyCode::Enter => {
                        let should_save = matches!(form.field, super::app::FormField::Tags);

                        if should_save {
                            let name = form.name.clone();
                            let host = form.host.clone();
                            let port: u16 = form.port.parse().unwrap_or(22);
                            let user = form.user.clone();
                            let auth = form.build_auth();
                            let tags: Vec<String> = form.tags.split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();

                            match form.mode {
                                FormMode::Insert => {
                                    if !name.is_empty() && !host.is_empty() {
                                        let server = config::Server {
                                            name: name.clone(),
                                            host,
                                            port,
                                            user,
                                            auth,
                                            tags,
                                            pinned: false,
                                            last_connected: None,
                                            connection_count: 0,
                                            bookmarks: vec![],
                                            agent_forwarding: false,
                                            proxy_jump: None,
                                        };
                                        app.servers.push(server);
                                        app.filter(&app.input.clone());
                                        if let Err(e) = config::save_config(
                                            &config::AppConfig { servers: app.servers.clone(), sort_by: None },
                                            &config::get_config_path(),
                                        ) {
                                            app.notifications.warning(&format!("Falha ao salvar config: {}", e));
                                        }
                                        app.notifications.success(&format!("Servidor '{}' adicionado!", name));
                                    } else {
                                        app.notifications.warning("Nome e Host são obrigatórios.");
                                    }
                                }
                                FormMode::Edit { server_index } => {
                                    if let Some(server) = app.servers.get_mut(server_index) {
                                        server.name = name.clone();
                                        server.host = host;
                                        server.port = port;
                                        server.user = user;
                                        server.auth = auth;
                                        server.tags = tags;
                                        if let Err(e) = config::save_config(
                                            &config::AppConfig { servers: app.servers.clone(), sort_by: None },
                                            &config::get_config_path(),
                                        ) {
                                            app.notifications.warning(&format!("Falha ao salvar config: {}", e));
                                        }
                                        app.notifications.success(&format!("Servidor '{}' atualizado!", name));
                                    }
                                }
                            }
                            app.input_mode = InputMode::Normal;
                            app.form_state = None;
                        }
                    }
                    _ => {}
                }
            }
        }
        InputMode::Search => match key.code {
            KeyCode::Enter => {
                app.input_mode = InputMode::Normal;
                let _ = execute!(io::stdout(), crossterm::cursor::Hide, crossterm::cursor::SetCursorStyle::DefaultUserShape);
            }
            KeyCode::Esc => {
                app.input_mode = InputMode::Normal;
                app.input.clear();
                app.filter("");
                let _ = execute!(io::stdout(), crossterm::cursor::Hide, crossterm::cursor::SetCursorStyle::DefaultUserShape);
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
        InputMode::Confirm => match key.code {
            KeyCode::Enter => {
                if let Some(confirm) = app.confirm_state.take() {
                    match confirm.action {
                        ConfirmAction::DeleteServer { name } => {
                            app.servers.retain(|s| s.name != name);
                            app.filter(&app.input.clone());
                            app.notifications.success(&format!("Servidor '{}' removido.", name));
                            if let Err(e) = config::save_config(
                                &config::AppConfig { servers: app.servers.clone(), sort_by: None },
                                &config::get_config_path(),
                            ) {
                                app.notifications.warning(&format!("Falha ao salvar config: {}", e));
                            }
                        }
                    }
                }
                app.input_mode = InputMode::Normal;
            }
            KeyCode::Esc => {
                app.confirm_state = None;
                app.input_mode = InputMode::Normal;
            }
            _ => {}
        },
    }
    HandlerResult::None
}

/// Manipula teclas do navegador SFTP. Retorna `HandlerResult`.
pub fn handle_sftp_key(app: &mut App, key: crossterm::event::KeyEvent) -> HandlerResult {
    // Handle SFTP input modes (mkdir/rename/chmod/bookmark)
    if let Some(sftp) = &app.sftp_state {
        if sftp.input_mode != SftpInputMode::None {
            match key.code {
                KeyCode::Esc => {
                    if let Some(sftp) = &mut app.sftp_state {
                        sftp.input_mode = SftpInputMode::None;
                        sftp.input_buffer.clear();
                        sftp.input_cursor = 0;
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
                                    SftpInputMode::Mkdir => {
                                        let full_path = format!("{}/{}", path, input);
                                        app.sftp_mkdir(&sid, &full_path)
                                    }
                                    SftpInputMode::Rename => {
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
                                    SftpInputMode::Chmod => {
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
                                    SftpInputMode::Bookmark => {
                                        let bookmark = config::models::ServerBookmark {
                                            name: input.clone(),
                                            path: path.clone(),
                                        };
                                        let server_idx = app.filtered_indices.get(app.selected).copied();
                                        if let Some(idx) = server_idx {
                                            if let Some(server) = app.servers.get_mut(idx) {
                                                server.bookmarks.push(bookmark);
                                                if let Err(e) = config::save_config(
                                                    &config::AppConfig { servers: app.servers.clone(), sort_by: app.sort_by.clone() },
                                                    &config::get_config_path(),
                                                ) {
                                                    app.notifications.warning(&format!("Falha ao salvar config: {}", e));
                                                }
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
                                            sftp.input_mode = SftpInputMode::None;
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
                        let pos = sftp.input_cursor;
                        sftp.input_buffer.insert(pos, c);
                        sftp.input_cursor += 1;
                    }
                }
                KeyCode::Backspace => {
                    if let Some(sftp) = &mut app.sftp_state {
                        if sftp.input_cursor > 0 {
                            sftp.input_cursor -= 1;
                            sftp.input_buffer.remove(sftp.input_cursor);
                        }
                    }
                }
                KeyCode::Delete => {
                    if let Some(sftp) = &mut app.sftp_state {
                        if sftp.input_cursor < sftp.input_buffer.len() {
                            sftp.input_buffer.remove(sftp.input_cursor);
                        }
                    }
                }
                KeyCode::Left => {
                    if let Some(sftp) = &mut app.sftp_state {
                        if sftp.input_cursor > 0 {
                            sftp.input_cursor -= 1;
                        }
                    }
                }
                KeyCode::Right => {
                    if let Some(sftp) = &mut app.sftp_state {
                        if sftp.input_cursor < sftp.input_buffer.len() {
                            sftp.input_cursor += 1;
                        }
                    }
                }
                KeyCode::Home => {
                    if let Some(sftp) = &mut app.sftp_state {
                        sftp.input_cursor = 0;
                    }
                }
                KeyCode::End => {
                    if let Some(sftp) = &mut app.sftp_state {
                        sftp.input_cursor = sftp.input_buffer.len();
                    }
                }
                _ => {}
            }
            return HandlerResult::None;
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
            let needs_refresh = app.sftp_state.as_ref()
                .is_some_and(|s| s.focus_side == Side::Remote);
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
            let needs_refresh = app.sftp_state.as_ref()
                .is_some_and(|s| s.focus_side == Side::Remote);
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
            if let Some(sftp) = &mut app.sftp_state {
                sftp.toggle_select_current();
            }
        }
        KeyCode::Char('a') => {
            if let Some(sftp) = &mut app.sftp_state {
                sftp.select_all_current();
                let count = match sftp.focus_side {
                    Side::Local => sftp.local_selected_files.len(),
                    Side::Remote => sftp.remote_selected_files.len(),
                };
                app.notifications.info(&format!("{} arquivo(s) selecionado(s)", count));
            }
        }
        KeyCode::Char('u') => {
            let mut upload_data: Option<(Vec<(String, String, String)>, Option<String>)> = None;
            if let Some(sftp) = &mut app.sftp_state {
                if matches!(sftp.transfer_state, TransferState::Transferring { .. }) {
                    app.notifications.warning("Transferência em andamento!");
                } else {
                    let files: Vec<(String, u64)> = match sftp.focus_side {
                        Side::Local => {
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
                        Side::Remote => vec![],
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

            if let Some((ref paths, _)) = upload_data {
                if let Some(server) = app.selected_server().cloned() {
                    app.notifications.info("Enviando... (barra de progresso em breve)");
                    let server_clone = server.clone();
                    let paths_clone = paths.clone();
                    let (result_tx2, result_rx2) = tokio::sync::mpsc::unbounded_channel();
                    app.sftp_op_rx = Some(result_rx2);
                    tokio::task::spawn_blocking(move || {
                        for (name, local, remote) in &paths_clone {
                            match transfer::upload_via_ssh(&server_clone, local, remote) {
                                Ok(()) => { let _ = result_tx2.send(SftpOpResult::Upload(name.clone())); }
                                Err(e) => { let _ = result_tx2.send(SftpOpResult::Error(format!("SCP erro: {}", e))); }
                            }
                        }
                        let _ = result_tx2.send(SftpOpResult::Error("__done__".to_string()));
                    });
                }
            }
        }
        KeyCode::Char('d') => {
            let mut download_data: Option<(Vec<(String, String, String)>, Option<String>)> = None;
            if let Some(sftp) = &mut app.sftp_state {
                if matches!(sftp.transfer_state, TransferState::Transferring { .. }) {
                    app.notifications.warning("Transferência em andamento!");
                } else {
                    let files: Vec<(String, u64)> = match sftp.focus_side {
                        Side::Remote => {
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
                        Side::Local => vec![],
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

            if let Some((ref paths, _)) = download_data {
                if let Some(server) = app.selected_server().cloned() {
                    app.notifications.info("Baixando... (barra de progresso em breve)");
                    let server_clone = server.clone();
                    let paths_clone = paths.clone();
                    let (result_tx2, result_rx2) = tokio::sync::mpsc::unbounded_channel();
                    app.sftp_op_rx = Some(result_rx2);
                    tokio::task::spawn_blocking(move || {
                        for (name, remote, local) in &paths_clone {
                            match transfer::download_via_ssh(&server_clone, remote, local) {
                                Ok(()) => { let _ = result_tx2.send(SftpOpResult::Upload(name.clone())); }
                                Err(e) => { let _ = result_tx2.send(SftpOpResult::Error(format!("Erro: {}", e))); }
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
            if let Some(sftp) = &mut app.sftp_state {
                if matches!(sftp.transfer_state, TransferState::Transferring { .. }) {
                    app.notifications.warning("Transferência em andamento!");
                } else {
                    sftp.input_mode = SftpInputMode::Mkdir;
                    sftp.input_buffer.clear();
                    sftp.input_cursor = 0;
                }
            }
        }
        KeyCode::Char('R') => {
            if let Some(sftp) = &mut app.sftp_state {
                if matches!(sftp.transfer_state, TransferState::Transferring { .. }) {
                    app.notifications.warning("Transferência em andamento!");
                } else if sftp.focus_side == Side::Remote {
                    if let Some(file) = sftp.remote_files.get(sftp.remote_selected) {
                        sftp.input_mode = SftpInputMode::Rename;
                        sftp.input_buffer = file.name.clone();
                        sftp.input_cursor = sftp.input_buffer.len();
                    }
                }
            }
        }
        KeyCode::Char('x') => {
            if let Some(sftp) = &mut app.sftp_state {
                if matches!(sftp.transfer_state, TransferState::Transferring { .. }) {
                    app.notifications.warning("Transferência em andamento!");
                } else if sftp.focus_side == Side::Remote {
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
            if let Some(sftp) = &mut app.sftp_state {
                if matches!(sftp.transfer_state, TransferState::Transferring { .. }) {
                    app.notifications.warning("Transferência em andamento!");
                } else if sftp.focus_side == Side::Remote {
                    if let Some(file) = sftp.remote_files.get(sftp.remote_selected) {
                        let perm_str = file.permissions
                            .map(|p| format!("{:04o}", p & 0o7777))
                            .unwrap_or_else(|| "????".to_string());
                        sftp.input_mode = SftpInputMode::Chmod;
                        sftp.input_buffer = perm_str;
                        sftp.input_cursor = sftp.input_buffer.len();
                    }
                }
            }
        }
        KeyCode::Char('b') => {
            if let Some(sftp) = &mut app.sftp_state {
                if sftp.focus_side == Side::Remote {
                    sftp.input_mode = SftpInputMode::Bookmark;
                    let default_name = sftp.remote_path
                        .rsplit('/')
                        .next()
                        .unwrap_or("root")
                        .to_string();
                    sftp.input_buffer = default_name;
                    sftp.input_cursor = sftp.input_buffer.len();
                }
            }
        }
        KeyCode::Char('B') => {
            if let Some(sftp) = &app.sftp_state {
                if sftp.focus_side == Side::Remote {
                    let server_idx = app.filtered_indices.get(app.selected).copied();
                    if let Some(idx) = server_idx {
                        if let Some(server) = app.servers.get(idx) {
                            if server.bookmarks.is_empty() {
                                app.notifications.info("Nenhum bookmark salvo. Use 'b' para salvar.");
                            } else {
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
    HandlerResult::None
}

/// Manipula eventos de mouse (scroll, cliques). Retorna `HandlerResult`.
pub fn handle_mouse_event(app: &mut App, mouse: crossterm::event::MouseEvent) -> HandlerResult {
    match mouse.kind {
        crossterm::event::MouseEventKind::ScrollUp => {
            if matches!(app.current_view, CurrentView::SftpBrowser) {
                if let Some(sftp) = &mut app.sftp_state {
                    sftp.previous_item();
                }
            } else {
                app.previous();
            }
        }
        crossterm::event::MouseEventKind::ScrollDown => {
            if matches!(app.current_view, CurrentView::SftpBrowser) {
                if let Some(sftp) = &mut app.sftp_state {
                    sftp.next_item();
                }
            } else {
                app.next();
            }
        }
        crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
            if matches!(app.current_view, CurrentView::ServerList)
                && app.form_state.is_none()
            {
                if mouse.row >= 4 {
                    let clicked_index = (mouse.row - 4) as usize;
                    if clicked_index < app.filtered_indices.len() {
                        app.selected = clicked_index;
                    }
                }
            }
        }
        crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Right) => {
            if matches!(app.current_view, CurrentView::ServerList)
                && app.form_state.is_none()
            {
                if mouse.row >= 4 {
                    let clicked_index = (mouse.row - 4) as usize;
                    if clicked_index < app.filtered_indices.len() {
                        app.selected = clicked_index;
                        if let Some(server) = app.selected_server().cloned() {
                            return HandlerResult::ConnectSsh(server);
                        }
                    }
                }
            }
        }
        _ => {}
    }
    HandlerResult::None
}
