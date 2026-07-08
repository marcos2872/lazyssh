use std::process::Command;

use crate::config::models::{Auth, Server};

pub fn execute_ssh_command(server: &Server, command: &str) -> Result<String, String> {
    // Verificar se temos sshpass para autenticação por senha
    let has_sshpass = Command::new("which")
        .arg("sshpass")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    // Verificar autenticação por senha antes de construir args
    let password = match &server.auth {
        Auth::Password { vault_key } => {
            if !has_sshpass {
                return Err(
                    "Autenticação por senha requer 'sshpass'. \
                     Instale com: sudo apt install sshpass (Debian/Ubuntu) \
                     ou sudo yum install sshpass (Fedora/RHEL)"
                        .to_string(),
                );
            }
            if vault_key.is_empty() {
                return Err("Senha não configurada. Configure a senha no servidor.".to_string());
            }
            Some(vault_key.clone())
        }
        _ => None,
    };

    // Construir argumentos do SSH
    let mut ssh_args = vec![
        "-o".to_string(),
        "StrictHostKeyChecking=no".to_string(),
        "-o".to_string(),
        "ConnectTimeout=5".to_string(),
    ];

    // Adicionar porta se não for 22
    if server.port != 22 {
        ssh_args.push("-p".to_string());
        ssh_args.push(server.port.to_string());
    }

    // Adicionar chave se for autenticação por chave
    if let Auth::Key { path, .. } = &server.auth {
        ssh_args.push("-i".to_string());
        ssh_args.push(path.clone());
    }

    // Adicionar usuário e host
    let user_host = format!("{}@{}", server.user, server.host);
    ssh_args.push(user_host);
    ssh_args.push(command.to_string());

    // Construir comando final
    let (program, final_args) = if let Some(pass) = password {
        // sshpass -p <password> ssh <args>
        let mut args = vec!["-p".to_string(), pass];
        args.extend(ssh_args);
        ("sshpass".to_string(), args)
    } else {
        // ssh <args>
        ("ssh".to_string(), ssh_args)
    };

    let output = Command::new(&program)
        .args(&final_args)
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
