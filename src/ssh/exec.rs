use std::process::Command;

use crate::config::models::{Auth, Server};

pub fn execute_ssh_command(server: &Server, command: &str) -> Result<String, String> {
    // Verificar se temos sshpass para autenticação por senha
    let has_sshpass = Command::new("which")
        .arg("sshpass")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

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

    // Tratar autenticação
    match &server.auth {
        Auth::Key { path, .. } => {
            args.push("-i".to_string());
            args.push(path.clone());
        }
        Auth::Password { vault_key } => {
            // Para autenticação por senha, precisamos de sshpass
            if !has_sshpass {
                return Err(
                    "Autenticação por senha requer 'sshpass'. \
                     Instale com: sudo apt install sshpass (Debian/Ubuntu) \
                     ou sudo yum install sshpass (Fedora/RHEL)"
                        .to_string(),
                );
            }
        }
    }

    // Adicionar usuário e host
    let user_host = format!("{}@{}", server.user, server.host);
    args.push(user_host);
    args.push(command.to_string());

    // Se for autenticação por senha, usar sshpass
    let (program, cmd_args) = if let Auth::Password { vault_key } = &server.auth {
        if vault_key.is_empty() {
            return Err("Senha não configurada. Configure a senha no servidor.".to_string());
        }
        let mut sshpass_args = vec!["-p".to_string(), vault_key.clone()];
        sshpass_args.extend(args);
        ("sshpass".to_string(), sshpass_args)
    } else {
        ("ssh".to_string(), args)
    };

    let output = Command::new(&program)
        .args(&cmd_args)
        .output()
        .map_err(|e| format!("Falha ao executar SSH: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        // Filtrar mensagens de password prompt
        let filtered_stderr = stderr
            .lines()
            .filter(|line| !line.contains("password:") && !line.contains("Password:"))
            .collect::<Vec<_>>()
            .join("\n");

        if filtered_stderr.trim().is_empty() {
            if stdout.trim().is_empty() {
                Err("Comando executado sem saída".to_string())
            } else {
                Ok(stdout)
            }
        } else {
            Err(filtered_stderr)
        }
    }
}
