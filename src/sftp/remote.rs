use crate::config::models::Server;
use crate::ssh::execute_ssh_command;

use super::FileInfo;

#[derive(Debug)]
pub struct RemoteFs {
    pub current_dir: String,
    pub server: Option<Server>,
}

impl RemoteFs {
    pub fn new() -> Self {
        Self {
            current_dir: "/".to_string(),
            server: None,
        }
    }

    pub fn with_server(server: Server) -> Self {
        // Obter o diretório home do usuário
        let home_dir = execute_ssh_command(&server, "echo $HOME")
            .unwrap_or_else(|_| "/root".to_string())
            .trim()
            .to_string();

        Self {
            current_dir: home_dir,
            server: Some(server),
        }
    }

    pub fn list(&self) -> Vec<FileInfo> {
        let server = match self.server.as_ref() {
            Some(s) => s,
            None => return vec![],
        };

        // Usar ls -1p para obter lista simples com / ao final de diretórios
        let output = match execute_ssh_command(server, &format!("ls -1p \"{}\"", self.current_dir)) {
            Ok(o) => o,
            Err(_) => return vec![],
        };

        output
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| {
                let name = line.trim_end_matches('/').to_string();
                let is_dir = line.ends_with('/');
                FileInfo {
                    name,
                    is_dir,
                    size: 0, // Tamanho não disponível com ls -1p
                }
            })
            .filter(|f| f.name != "." && f.name != "..")
            .collect()
    }

    pub fn cd(&mut self, path: &str) -> Result<(), String> {
        let server = self.server.as_ref()
            .ok_or_else(|| "No server connected".to_string())?;

        let new_dir = if path.starts_with('/') {
            path.to_string()
        } else if path == "~" || path.starts_with("~/") {
            path.to_string()
        } else if path == ".." {
            let output = execute_ssh_command(server, &format!("dirname \"{}\"", self.current_dir))
                .map_err(|e| e.to_string())?;
            output.trim().to_string()
        } else {
            format!("{}/{}", self.current_dir, path)
        };

        let output = execute_ssh_command(server, &format!("cd \"{}\" && pwd", new_dir))
            .map_err(|e| e.to_string())?;

        if output.trim().is_empty() {
            return Err(format!("Directory not found: {}", path));
        }

        self.current_dir = output.trim().to_string();
        Ok(())
    }

    pub fn current_dir(&self) -> &str {
        &self.current_dir
    }

    pub fn upload(&self, local_path: &str, remote_path: &str) -> Result<(), String> {
        let server = self.server.as_ref()
            .ok_or_else(|| "No server connected".to_string())?;

        let full_remote = if remote_path.starts_with('/') {
            remote_path.to_string()
        } else {
            format!("{}/{}", self.current_dir, remote_path)
        };

        use std::process::Command;

        let user_host = format!("{}@{}", server.user, server.host);
        let remote_target = format!("{}:{}", user_host, full_remote);
        let port_str = server.port.to_string();

        // Verificar se temos sshpass para autenticação por senha
        let has_sshpass = Command::new("which")
            .arg("sshpass")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        // Determinar programa e argumentos
        let (program, mut args) = match &server.auth {
            crate::config::models::Auth::Password { vault_key } => {
                if !has_sshpass {
                    return Err("Password auth requires sshpass".to_string());
                }
                if vault_key.is_empty() {
                    return Err("Password not configured".to_string());
                }
                let mut a = vec!["-p".to_string(), vault_key.clone(), "scp".to_string()];
                a.push("-o".to_string());
                a.push("StrictHostKeyChecking=no".to_string());
                a.push("-P".to_string());
                a.push(port_str);
                a.push(local_path.to_string());
                a.push(remote_target);
                ("sshpass".to_string(), a)
            }
            crate::config::models::Auth::Key { path, .. } => {
                let mut a = vec![
                    "scp".to_string(),
                    "-o".to_string(), "StrictHostKeyChecking=no".to_string(),
                    "-P".to_string(), port_str,
                    "-i".to_string(), path.clone(),
                    local_path.to_string(),
                    remote_target,
                ];
                ("scp".to_string(), a)
            }
        };

        let output = Command::new(&program)
            .args(&args)
            .output()
            .map_err(|e| format!("Failed to run {}: {}", program, e))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("SCP upload failed: {}", stderr))
        }
    }

    pub fn download(&self, remote_path: &str, local_path: &str) -> Result<(), String> {
        let server = self.server.as_ref()
            .ok_or_else(|| "No server connected".to_string())?;

        let full_remote = if remote_path.starts_with('/') {
            remote_path.to_string()
        } else {
            format!("{}/{}", self.current_dir, remote_path)
        };

        use std::process::Command;

        let user_host = format!("{}@{}", server.user, server.host);
        let remote_source = format!("{}:{}", user_host, full_remote);
        let port_str = server.port.to_string();

        // Verificar se temos sshpass para autenticação por senha
        let has_sshpass = Command::new("which")
            .arg("sshpass")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        // Determinar programa e argumentos
        let (program, mut args) = match &server.auth {
            crate::config::models::Auth::Password { vault_key } => {
                if !has_sshpass {
                    return Err("Password auth requires sshpass".to_string());
                }
                if vault_key.is_empty() {
                    return Err("Password not configured".to_string());
                }
                let mut a = vec!["-p".to_string(), vault_key.clone(), "scp".to_string()];
                a.push("-o".to_string());
                a.push("StrictHostKeyChecking=no".to_string());
                a.push("-P".to_string());
                a.push(port_str);
                a.push(remote_source);
                a.push(local_path.to_string());
                ("sshpass".to_string(), a)
            }
            crate::config::models::Auth::Key { path, .. } => {
                let mut a = vec![
                    "scp".to_string(),
                    "-o".to_string(), "StrictHostKeyChecking=no".to_string(),
                    "-P".to_string(), port_str,
                    "-i".to_string(), path.clone(),
                    remote_source,
                    local_path.to_string(),
                ];
                ("scp".to_string(), a)
            }
        };

        let output = Command::new(&program)
            .args(&args)
            .output()
            .map_err(|e| format!("Failed to run {}: {}", program, e))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("SCP download failed: {}", stderr))
        }
    }

    pub fn get_file_size(&self, remote_path: &str) -> Result<u64, String> {
        let server = self.server.as_ref()
            .ok_or_else(|| "No server connected".to_string())?;

        let full_remote = if remote_path.starts_with('/') {
            remote_path.to_string()
        } else {
            format!("{}/{}", self.current_dir, remote_path)
        };

        let output = execute_ssh_command(server, &format!("stat -c %s \"{}\"", full_remote))
            .map_err(|e| e.to_string())?;

        output.trim().parse::<u64>().map_err(|e| format!("Invalid size: {}", e))
    }

    pub fn get_file_content(&self, remote_path: &str) -> Result<String, String> {
        let server = self.server.as_ref()
            .ok_or_else(|| "No server connected".to_string())?;

        let full_remote = if remote_path.starts_with('/') {
            remote_path.to_string()
        } else {
            format!("{}/{}", self.current_dir, remote_path)
        };

        execute_ssh_command(server, &format!("cat \"{}\"", full_remote))
            .map_err(|e| e.to_string())
    }
}
