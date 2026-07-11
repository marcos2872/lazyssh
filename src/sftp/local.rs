use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

use super::FileInfo;

/// Abstração do filesystem local para o painel local do navegador SFTP.
#[derive(Debug)]
pub struct LocalFs {
    /// Diretório de trabalho atual sendo navegado localmente.
    pub current_dir: PathBuf,
}

impl LocalFs {
    /// Cria um novo `LocalFs` começando no diretório home do usuário.
    pub fn new() -> Self {
        Self {
            current_dir: dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
        }
    }

    /// Lista arquivos e diretórios no diretório de trabalho atual.
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
                permissions: None,
            });
        }

        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    /// Muda o diretório atual. Suporta caminhos absolutos e relativos.
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

    /// Retorna uma referência ao caminho do diretório atual.
    pub fn current_dir(&self) -> &Path {
        &self.current_dir
    }

    /// Grava conteúdo em um arquivo no diretório atual.
    pub fn download(&self, remote_content: &str, local_filename: &str) -> Result<()> {
        let local_path = self.current_dir.join(local_filename);
        fs::write(local_path, remote_content)?;
        Ok(())
    }

    /// Retorna o tamanho em bytes de um arquivo no diretório atual.
    pub fn get_file_size(&self, filename: &str) -> Result<u64> {
        let path = self.current_dir.join(filename);
        let metadata = fs::metadata(path)?;
        Ok(metadata.len())
    }

    /// Retorna o caminho absoluto de um arquivo no diretório atual.
    pub fn get_full_path(&self, filename: &str) -> String {
        self.current_dir.join(filename).to_string_lossy().to_string()
    }
}
