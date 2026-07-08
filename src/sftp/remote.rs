use anyhow::Result;

use crate::ssh::SshSession;

#[derive(Debug)]
pub struct FileInfo {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

pub struct RemoteFs {
    current_dir: String,
}

impl RemoteFs {
    pub fn new() -> Self {
        Self {
            current_dir: "/".to_string(),
        }
    }

    pub async fn list(&self, session: &SshSession) -> Result<Vec<FileInfo>> {
        let output = session.execute(&format!("ls -la {}", self.current_dir)).await?;
        // Parse ls output - simplified
        let entries = output
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 9 {
                    let is_dir = parts[0].starts_with('d');
                    let name = parts[8..].join(" ");
                    let size: u64 = parts[4].parse().unwrap_or(0);
                    Some(FileInfo {
                        name,
                        is_dir,
                        size,
                    })
                } else {
                    None
                }
            })
            .collect();
        Ok(entries)
    }

    pub async fn cd(&mut self, session: &SshSession, path: &str) -> Result<()> {
        let output = session
            .execute(&format!("cd {} && pwd", path))
            .await?;
        self.current_dir = output.trim().to_string();
        Ok(())
    }

    pub fn current_dir(&self) -> &str {
        &self.current_dir
    }
}
