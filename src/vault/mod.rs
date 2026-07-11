//! Cofre de credenciais — integração com keyring do sistema e criptografia AES-256-GCM de senhas.

pub mod crypto;
pub mod keyring;

pub use crypto::*;
pub use keyring::*;
