use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use super::{AppConfig, Auth, Server};
use crate::vault::keyring;

/// Retorna o caminho para o arquivo de configuração TOML.
///
/// Usa a variável de ambiente `LAZYSSH_TEST_CONFIG_PATH` se definida (para testes),
/// caso contrário resolve para `~/.config/lazyssh/servers.toml`.
pub fn get_config_path() -> PathBuf {
    if let Ok(path) = std::env::var("LAZYSSH_TEST_CONFIG_PATH") {
        return PathBuf::from(path);
    }

    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lazyssh");
    fs::create_dir_all(&config_dir).ok();
    config_dir.join("servers.toml")
}

/// Carrega e analisa a configuração TOML do caminho informado.
///
/// Após a análise, tenta restaurar senhas do keyring do sistema
/// para qualquer servidor cujo campo `vault_key` esteja vazio.
pub fn load_config(path: &std::path::Path) -> Result<AppConfig> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config from {}", path.display()))?;
    let mut config: AppConfig = toml::from_str(&content)
        .with_context(|| "Failed to parse config TOML")?;

    // Try to restore passwords from keyring for servers with empty vault_key
    for server in &mut config.servers {
        let needs_keyring = matches!(&server.auth, Auth::Password { vault_key } if vault_key.is_empty());
        if needs_keyring {
            if let Some(password) = keyring::get_password(server) {
                if let Auth::Password { ref mut vault_key } = server.auth {
                    *vault_key = password;
                }
            }
        }
    }

    Ok(config)
}

/// Serializa a configuração em TOML e grava no disco.
///
/// Senhas são primeiro movidas para o keyring do sistema (quando disponível),
/// depois removidas do TOML antes da escrita. Um `.toml.backup` é criado
/// como rede de segurança antes de sobrescrever.
pub fn save_config(config: &AppConfig, path: &std::path::Path) -> Result<()> {
    // Try to store passwords in keyring and clear them from TOML
    let mut config_for_save = config.clone();
    for server in &mut config_for_save.servers {
        let should_clear = if let Auth::Password { ref vault_key } = server.auth {
            if !vault_key.is_empty() {
                keyring::store_password(server, vault_key).is_ok()
            } else {
                false
            }
        } else {
            false
        };
        if should_clear {
            if let Auth::Password { ref mut vault_key } = server.auth {
                vault_key.clear();
            }
        }
    }

    let content = toml::to_string_pretty(&config_for_save)
        .with_context(|| "Failed to serialize config")?;

    // Backup before write
    if path.exists() {
        let backup = path.with_extension("toml.backup");
        fs::copy(path, backup).ok();
    }

    fs::write(path, content)
        .with_context(|| format!("Failed to write config to {}", path.display()))?;
    Ok(())
}

/// Carrega a configuração, ou retorna uma config vazia se o arquivo não existir.
pub fn load_or_default() -> AppConfig {
    let path = get_config_path();
    if path.exists() {
        load_config(&path).unwrap_or_else(|_| AppConfig { servers: vec![], sort_by: None })
    } else {
        AppConfig { servers: vec![], sort_by: None }
    }
}

/// Adiciona um novo servidor à configuração e persiste.
pub fn add_server(server: Server) -> Result<()> {
    let path = get_config_path();
    let mut config = load_or_default();
    config.servers.push(server);
    save_config(&config, &path)?;
    Ok(())
}

/// Remove um servidor por nome da configuração e deleta sua entrada no keyring.
pub fn remove_server(name: &str) -> Result<()> {
    let path = get_config_path();
    let mut config = load_or_default();
    // Delete from keyring before removing from config
    for server in config.servers.iter() {
        if server.name == name {
            keyring::delete_password(server);
        }
    }
    config.servers.retain(|s| s.name != name);
    save_config(&config, &path)?;
    Ok(())
}

/// Substitui os dados de um servidor existente pelo nome. Entradas antigas do keyring são limpas.
pub fn update_server(name: &str, updated: Server) -> Result<()> {
    let path = get_config_path();
    let mut config = load_or_default();
    if let Some(server) = config.servers.iter_mut().find(|s| s.name == name) {
        // Delete old keyring entry before replacing
        let old_server = server.clone();
        if let Auth::Password { .. } = &old_server.auth {
            keyring::delete_password(&old_server);
        }
        *server = updated;
        save_config(&config, &path)?;
    }
    Ok(())
}
