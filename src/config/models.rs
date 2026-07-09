use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    pub servers: Vec<Server>,
    #[serde(default)]
    pub sort_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Server {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub auth: Auth,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub last_connected: Option<String>,
    #[serde(default)]
    pub connection_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Auth {
    Key {
        path: String,
        passphrase: Option<String>,
    },
    Password {
        vault_key: String,
    },
}
