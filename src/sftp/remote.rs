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
        Self {
            current_dir: "~".to_string(),
            server: Some(server),
        }
    }

    pub fn list(&self) -> Vec<FileInfo> {
        let server = match self.server.as_ref() {
            Some(s) => s,
            None => return vec![],
        };

        let output = match execute_ssh_command(server, &format!("ls -la \"{}\"", self.current_dir)) {
            Ok(o) => o,
            Err(_) => return vec![],
        };

        output
            .lines()
            .filter(|line| !line.starts_with("total"))
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 9 {
                    let is_dir = parts[0].starts_with('d');
                    let name = parts[8..].join(" ");
                    let size: u64 = parts[4].parse().unwrap_or(0);

                    if name == "." || name == ".." {
                        return None;
                    }

                    Some(FileInfo { name, is_dir, size })
                } else {
                    None
                }
            })
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

        execute_ssh_command(server, &format!("cat \"{}\" > \"{}\"", local_path, full_remote))
            .map_err(|e| e.to_string())?;
        Ok(())
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
