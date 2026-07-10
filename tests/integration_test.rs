use lazyssh::config::{self, AppConfig, Auth, Server};
use lazyssh::vault::crypto::{derive_key, encrypt_password, decrypt_password};
use std::fs;

#[test]
fn test_full_config_workflow() {
    let test_dir = std::env::temp_dir().join("lazyssh_integration_test");
    fs::create_dir_all(&test_dir).unwrap();

    // Create config
    let config = AppConfig {
        servers: vec![
            Server {
                name: "server1".to_string(),
                host: "192.168.1.1".to_string(),
                port: 22,
                user: "user1".to_string(),
                auth: Auth::Key {
                    path: "~/.ssh/id_rsa".to_string(),
                    passphrase: None,
                },
                tags: vec!["prod".to_string()],
                pinned: true,
                last_connected: None,
                connection_count: 0,
            bookmarks: vec![],
            agent_forwarding: false,
            proxy_jump: None,
            log_enabled: false,
            },
            Server {
                name: "server2".to_string(),
                host: "192.168.1.2".to_string(),
                port: 22,
                user: "user2".to_string(),
                auth: Auth::Password {
                    vault_key: "server2_pass".to_string(),
                },
                tags: vec!["dev".to_string()],
                pinned: false,
                last_connected: None,
                connection_count: 0,
            bookmarks: vec![],
            agent_forwarding: false,
            proxy_jump: None,
            log_enabled: false,
            },
        ],
        sort_by: None,
    };

    let config_path = test_dir.join("servers.toml");
    config::save_config(&config, &config_path).unwrap();

    // Load and verify
    let loaded = config::load_config(&config_path).unwrap();
    assert_eq!(loaded.servers.len(), 2);
    assert_eq!(loaded.servers[0].name, "server1");
    assert_eq!(loaded.servers[1].name, "server2");

    // Test vault with password auth
    let master_pass = "integration_test_master";
    let salt = [1u8; 16];
    let key = derive_key(master_pass, &salt);

    if let Auth::Password { vault_key } = &loaded.servers[1].auth {
        let encrypted = encrypt_password(vault_key, &key).unwrap();
        let decrypted = decrypt_password(&encrypted, &key).unwrap();
        assert_eq!(decrypted, "server2_pass");
    }

    fs::remove_dir_all(&test_dir).unwrap();
}
