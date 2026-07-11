use crate::config::models::Server;

const SERVICE: &str = "lazyssh";

/// Gera a chave da conta no keyring a partir da identidade do servidor (`user@host:port`).
fn account_key(server: &Server) -> String {
    format!("{}@{}:{}", server.user, server.host, server.port)
}

/// Armazena uma senha no keyring do sistema (GNOME Keyring, KDE Wallet, macOS Keychain).
pub fn store_password(server: &Server, password: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE, &account_key(server))
        .map_err(|e| format!("keyring entry: {}", e))?;
    entry.set_password(password)
        .map_err(|e| format!("keyring store: {}", e))
}

/// Recupera uma senha armazenada no keyring do sistema, ou `None` se não encontrada.
pub fn get_password(server: &Server) -> Option<String> {
    match keyring::Entry::new(SERVICE, &account_key(server)) {
        Ok(entry) => entry.get_password().ok(),
        Err(_) => None,
    }
}

/// Deleta uma senha armazenada no keyring do sistema. Sem efeito se não encontrada.
pub fn delete_password(server: &Server) {
    if let Ok(entry) = keyring::Entry::new(SERVICE, &account_key(server)) {
        let _ = entry.delete_credential();
    }
}

/// Verifica se o keyring do sistema está disponível.
pub fn is_available() -> bool {
    keyring::Entry::new("lazyssh-test", "availability-check").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::models::{Auth, Server};

    fn test_server() -> Server {
        Server {
            name: "test".into(),
            host: "10.0.0.1".into(),
            port: 22,
            user: "root".into(),
            auth: Auth::Key {
                path: "~/.ssh/id_rsa".into(),
                passphrase: None,
            },
            tags: vec![],
            pinned: false,
            last_connected: None,
            connection_count: 0,
            bookmarks: vec![],
            agent_forwarding: false,
            proxy_jump: None,
        }
    }

    #[test]
    fn test_account_key_format() {
        let server = test_server();
        assert_eq!(account_key(&server), "root@10.0.0.1:22");
    }

    #[test]
    #[ignore] // requires real keyring
    fn test_store_get_delete_roundtrip() {
        let server = test_server();
        assert!(store_password(&server, "test_pass_123").is_ok());
        assert_eq!(get_password(&server), Some("test_pass_123".into()));
        delete_password(&server);
        assert_eq!(get_password(&server), None);
    }

    #[test]
    #[ignore] // requires real keyring
    fn test_get_nonexistent_returns_none() {
        let mut server = test_server();
        server.host = "nonexistent-host-99999".into();
        assert_eq!(get_password(&server), None);
    }
}
