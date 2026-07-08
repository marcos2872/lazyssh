use std::process::Command;

use crate::config::models::{Auth, Server};

pub fn execute_ssh_command(server: &Server, command: &str) -> Result<String, String> {
    let mut args = vec![
        "-o".to_string(),
        "StrictHostKeyChecking=no".to_string(),
        "-o".to_string(),
        "ConnectTimeout=5".to_string(),
    ];

    // Adicionar porta se não for 22
    if server.port != 22 {
        args.push("-p".to_string());
        args.push(server.port.to_string());
    }

    // Adicionar chave se existir
    if let Auth::Key { path, .. } = &server.auth {
        args.push("-i".to_string());
        args.push(path.clone());
    }

    // Adicionar usuário e host
    let user_host = format!("{}@{}", server.user, server.host);
    args.push(user_host);
    args.push(command.to_string());

    let output = Command::new("ssh")
        .args(&args)
        .output()
        .map_err(|e| format!("Falha ao executar SSH: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if stderr.is_empty() {
            Ok(String::new())
        } else {
            Err(stderr)
        }
    }
}
