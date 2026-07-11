use anyhow::{Context, Result};
use russh::keys::{key::PrivateKeyWithHashAlg, load_secret_key};
use std::path::Path;
use std::sync::Arc;

use crate::config::models::Auth;

/// Carrega uma chave privada SSH do disco para autenticação por chave pública.
///
/// Suporta expansão de `~` no caminho da chave e frases secretas opcionais.
/// Retorna erro se a variante for `Password` ou se o arquivo não puder ser lido.
pub fn load_key(auth: &Auth) -> Result<PrivateKeyWithHashAlg> {
    match auth {
        Auth::Key { path, passphrase } => {
            let key_path = Path::new(path);
            let lossy = key_path.to_string_lossy();
            let expanded = shellexpand::tilde(&lossy);
            let key = load_secret_key(expanded.as_ref(), passphrase.as_deref())
                .context("Failed to load SSH key")?;
            Ok(PrivateKeyWithHashAlg::new(Arc::new(key), None))
        }
        Auth::Password { .. } => Err(anyhow::anyhow!("Use password auth flow")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::models::Auth;

    #[test]
    fn test_load_key_password_auth_returns_error() {
        let auth = Auth::Password {
            vault_key: "test".to_string(),
        };
        let result = load_key(&auth);
        assert!(result.is_err(), "Password auth should return an error, not attempt to load key");
        assert!(result.unwrap_err().to_string().contains("password"),
            "Error should mention password flow");
    }

    #[test]
    fn test_load_key_nonexistent_path() {
        let auth = Auth::Key {
            path: "/tmp/nonexistent_key_that_does_not_exist".to_string(),
            passphrase: None,
        };
        let result = load_key(&auth);
        assert!(result.is_err(), "Nonexistent key path should fail");
    }
}
