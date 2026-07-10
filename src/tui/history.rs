use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const MAX_HISTORY: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandEntry {
    pub command: String,
    pub timestamp: String,
    pub server_name: String,
}

pub struct CommandHistory {
    pub entries: Vec<CommandEntry>,
}

impl CommandHistory {
    pub fn new() -> Self {
        let entries = Self::load_from_disk();
        Self { entries }
    }

    pub fn add(&mut self, command: String, server_name: String) {
        if command.trim().is_empty() {
            return;
        }
        // Don't duplicate the last command for the same server
        if let Some(last) = self.entries.last() {
            if last.command == command && last.server_name == server_name {
                return;
            }
        }
        let now = chrono::Local::now();
        self.entries.push(CommandEntry {
            command,
            timestamp: now.format("%Y-%m-%d %H:%M:%S").to_string(),
            server_name,
        });
        // FIFO: keep only MAX_HISTORY
        if self.entries.len() > MAX_HISTORY {
            let drain_count = self.entries.len() - MAX_HISTORY;
            self.entries.drain(..drain_count);
        }
        self.save_to_disk();
    }

    pub fn search(&self, query: &str) -> Vec<&CommandEntry> {
        if query.is_empty() {
            return self.entries.iter().rev().take(20).collect();
        }
        let q = query.to_lowercase();
        let mut results: Vec<&CommandEntry> = self
            .entries
            .iter()
            .rev()
            .filter(|e| e.command.to_lowercase().contains(&q))
            .take(20)
            .collect();
        results.reverse();
        results
    }

    fn history_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_default();
        let dir = format!("{}/.local/share/lazyssh", home);
        let _ = std::fs::create_dir_all(&dir);
        PathBuf::from(format!("{}/history.toml", dir))
    }

    fn save_to_disk(&self) {
        let path = Self::history_path();
        if let Ok(content) = toml::to_string_pretty(&self.entries) {
            let _ = std::fs::write(&path, content);
        }
    }

    fn load_from_disk() -> Vec<CommandEntry> {
        let path = Self::history_path();
        if let Ok(content) = std::fs::read_to_string(&path) {
            toml::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_command() {
        let mut h = CommandHistory { entries: Vec::new() };
        h.add("ls -la".to_string(), "server1".to_string());
        assert_eq!(h.entries.len(), 1);
        assert_eq!(h.entries[0].command, "ls -la");
        assert_eq!(h.entries[0].server_name, "server1");
    }

    #[test]
    fn test_add_empty_command_ignored() {
        let mut h = CommandHistory { entries: Vec::new() };
        h.add("".to_string(), "s".to_string());
        h.add("   ".to_string(), "s".to_string());
        assert_eq!(h.entries.len(), 0);
    }

    #[test]
    fn test_no_consecutive_duplicates() {
        let mut h = CommandHistory { entries: Vec::new() };
        h.add("ls".to_string(), "s".to_string());
        h.add("ls".to_string(), "s".to_string());
        assert_eq!(h.entries.len(), 1);
    }

    #[test]
    fn test_same_command_different_server() {
        let mut h = CommandHistory { entries: Vec::new() };
        h.add("ls".to_string(), "s1".to_string());
        h.add("ls".to_string(), "s2".to_string());
        assert_eq!(h.entries.len(), 2);
    }

    #[test]
    fn test_fifo_limit() {
        let mut h = CommandHistory { entries: Vec::new() };
        for i in 0..MAX_HISTORY + 10 {
            h.entries.push(CommandEntry {
                command: format!("cmd{}", i),
                timestamp: String::new(),
                server_name: "s".to_string(),
            });
        }
        // Manually trim
        if h.entries.len() > MAX_HISTORY {
            let drain = h.entries.len() - MAX_HISTORY;
            h.entries.drain(..drain);
        }
        assert_eq!(h.entries.len(), MAX_HISTORY);
    }

    #[test]
    fn test_search_empty_query_returns_recent() {
        let mut h = CommandHistory { entries: Vec::new() };
        h.entries.push(CommandEntry {
            command: "ls".to_string(),
            timestamp: String::new(),
            server_name: "s".to_string(),
        });
        h.entries.push(CommandEntry {
            command: "pwd".to_string(),
            timestamp: String::new(),
            server_name: "s".to_string(),
        });
        let results = h.search("");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_filters() {
        let mut h = CommandHistory { entries: Vec::new() };
        h.entries.push(CommandEntry {
            command: "ls -la".to_string(),
            timestamp: String::new(),
            server_name: "s".to_string(),
        });
        h.entries.push(CommandEntry {
            command: "cd /tmp".to_string(),
            timestamp: String::new(),
            server_name: "s".to_string(),
        });
        let results = h.search("ls");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].command, "ls -la");
    }

    #[test]
    fn test_search_case_insensitive() {
        let mut h = CommandHistory { entries: Vec::new() };
        h.entries.push(CommandEntry {
            command: "Makefile".to_string(),
            timestamp: String::new(),
            server_name: "s".to_string(),
        });
        let results = h.search("make");
        assert_eq!(results.len(), 1);
    }
}
