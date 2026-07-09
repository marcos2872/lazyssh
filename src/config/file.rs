use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use super::{AppConfig, Server};

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

pub fn load_config(path: &std::path::Path) -> Result<AppConfig> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config from {}", path.display()))?;
    let config: AppConfig = toml::from_str(&content)
        .with_context(|| "Failed to parse config TOML")?;
    Ok(config)
}

pub fn save_config(config: &AppConfig, path: &std::path::Path) -> Result<()> {
    let content = toml::to_string_pretty(config)
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

pub fn load_or_default() -> AppConfig {
    let path = get_config_path();
    if path.exists() {
        load_config(&path).unwrap_or_else(|_| AppConfig { servers: vec![], sort_by: None })
    } else {
        AppConfig { servers: vec![], sort_by: None }
    }
}

pub fn add_server(server: Server) -> Result<()> {
    let path = get_config_path();
    let mut config = load_or_default();
    config.servers.push(server);
    save_config(&config, &path)?;
    Ok(())
}

pub fn remove_server(name: &str) -> Result<()> {
    let path = get_config_path();
    let mut config = load_or_default();
    config.servers.retain(|s| s.name != name);
    save_config(&config, &path)?;
    Ok(())
}

pub fn update_server(name: &str, updated: Server) -> Result<()> {
    let path = get_config_path();
    let mut config = load_or_default();
    if let Some(server) = config.servers.iter_mut().find(|s| s.name == name) {
        *server = updated;
        save_config(&config, &path)?;
    }
    Ok(())
}
