use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct FileInfo {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

pub struct LocalFs {
    current_dir: PathBuf,
}

impl LocalFs {
    pub fn new() -> Self {
        Self {
            current_dir: dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
        }
    }

    pub fn list(&self) -> Result<Vec<FileInfo>> {
        let mut entries = Vec::new();

        for entry in fs::read_dir(&self.current_dir)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let name = entry.file_name().to_string_lossy().to_string();

            entries.push(FileInfo {
                name,
                is_dir: metadata.is_dir(),
                size: metadata.len(),
            });
        }

        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    pub fn cd(&mut self, path: &str) -> Result<()> {
        let new_path = if path.starts_with('/') {
            PathBuf::from(path)
        } else {
            self.current_dir.join(path)
        };

        if new_path.is_dir() {
            self.current_dir = fs::canonicalize(new_path)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Not a directory: {}", path))
        }
    }

    pub fn current_dir(&self) -> &Path {
        &self.current_dir
    }
}
