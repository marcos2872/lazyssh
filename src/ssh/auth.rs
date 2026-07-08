use anyhow::{Context, Result};
use russh::keys::{key::PrivateKeyWithHashAlg, load_secret_key};
use std::path::Path;
use std::sync::Arc;

use crate::config::models::Auth;

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
