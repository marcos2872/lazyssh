//! Conexões SSH — construção de argumentos, autenticação por chave e serviço de sessões.

pub mod args;
pub mod auth;
pub mod connection;
pub mod service;

pub use connection::SshSession;
pub use service::{SshService, SessionStatus, ShellChannel};
