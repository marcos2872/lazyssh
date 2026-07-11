//! Operações de filesystem SFTP — filesystem local, serviço SFTP remoto e transferência via SSH.

pub mod local;
mod service;
pub mod transfer;

/// Metadados de um arquivo ou diretório.
#[derive(Debug)]
pub struct FileInfo {
    /// Nome do arquivo ou diretório (sem caminho).
    pub name: String,
    /// Se esta entrada é um diretório.
    pub is_dir: bool,
    /// Tamanho do arquivo em bytes (0 para diretórios).
    pub size: u64,
    /// Permissões Unix em formato octal, quando disponível do remoto.
    pub permissions: Option<u32>,
}

pub use local::LocalFs;
pub use service::{SftpService, SftpServiceSession, SessionStatus, upload_file};
