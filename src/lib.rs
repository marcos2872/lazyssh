//! LazySSH — Aplicação TUI para gerenciamento de SSH e SFTP.
//!
//! Fornece uma interface de terminal para gerenciar conexões SSH,
//! navegar em arquivos remotos via SFTP e transferir arquivos entre
//! sistemas locais e remotos. Configuração persistida em TOML com
//! integração opcional ao keyring do sistema para armazenamento de senhas.

pub mod config;
pub mod sftp;
pub mod ssh;
pub mod tui;
pub mod vault;
