use serde::{Deserialize, Serialize};

/// Configuração principal da aplicação, serializada em TOML.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    /// Lista de servidores SSH configurados.
    pub servers: Vec<Server>,
    /// Modo de ordenação ativo (ex: "name", "port", "last_connected", "frequency").
    #[serde(default)]
    pub sort_by: Option<String>,
}

/// Um marcador de diretório salvo para navegação rápida no navegador SFTP.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerBookmark {
    /// Nome de exibição do marcador.
    pub name: String,
    /// Caminho absoluto remoto que o marcador aponta.
    pub path: String,
}

/// Definição de uma conexão de servidor SSH.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Server {
    /// Nome de exibição amigável.
    pub name: String,
    /// Hostname ou endereço IP.
    pub host: String,
    /// Porta SSH (padrão 22).
    pub port: u16,
    /// Usuário SSH.
    pub user: String,
    /// Método de autenticação (chave ou senha).
    pub auth: Auth,
    /// Tags definidas pelo usuário para filtragem e organização.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Se este servidor está fixado no topo da lista.
    #[serde(default)]
    pub pinned: bool,
    /// Timestamp ISO da última conexão SSH bem-sucedida.
    #[serde(default)]
    pub last_connected: Option<String>,
    /// Número total de vezes que este servidor foi conectado.
    #[serde(default)]
    pub connection_count: u32,
    /// Marcadores de diretório salvos para o navegador SFTP.
    #[serde(default)]
    pub bookmarks: Vec<ServerBookmark>,
    /// Se deve encaminhar o agent SSH local (flag `-A`).
    #[serde(default)]
    pub agent_forwarding: bool,
    /// Host ProxyJump para acesso via servidor bastião (flag `-J`).
    #[serde(default)]
    pub proxy_jump: Option<String>,
}

/// Método de autenticação SSH para um servidor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Auth {
    /// Autenticação por chave pública.
    Key {
        /// Caminho para o arquivo de chave privada (suporta expansão `~`).
        path: String,
        /// Frase secreta opcional para chaves criptografadas.
        passphrase: Option<String>,
    },
    /// Autenticação por senha (via `sshpass` + variável de ambiente `SSHPASS`).
    Password {
        /// A senha, armazenada no keyring do sistema quando possível.
        vault_key: String,
    },
}
