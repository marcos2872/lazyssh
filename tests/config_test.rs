use lazyssh::config::models::{AppConfig, Auth, Server};

#[test]
fn test_server_creation() {
    let server = Server {
        name: "myserver".to_string(),
        host: "192.168.1.100".to_string(),
        port: 22,
        user: "root".to_string(),
        auth: Auth::Key {
            path: "~/.ssh/id_ed25519".to_string(),
            passphrase: None,
        },
        tags: vec!["prod".to_string()],
        pinned: false,
    };
    assert_eq!(server.name, "myserver");
    assert_eq!(server.port, 22);
}

#[test]
fn test_auth_password_variant() {
    let auth = Auth::Password {
        vault_key: "my-vault-key".to_string(),
    };
    match &auth {
        Auth::Password { vault_key } => assert_eq!(vault_key, "my-vault-key"),
        _ => panic!("Expected Auth::Password variant"),
    }
    assert_eq!(
        auth,
        Auth::Password {
            vault_key: "my-vault-key".to_string()
        }
    );
}

#[test]
fn test_config_serialization_round_trip() {
    let config = AppConfig {
        servers: vec![Server {
            name: "dev".to_string(),
            host: "10.0.0.1".to_string(),
            port: 22,
            user: "deploy".to_string(),
            auth: Auth::Key {
                path: "~/.ssh/id_rsa".to_string(),
                passphrase: Some("secret".to_string()),
            },
            tags: vec!["dev".to_string()],
            pinned: true,
        }],
    };

    let json = serde_json::to_string(&config).expect("serialization failed");
    let deserialized: AppConfig = serde_json::from_str(&json).expect("deserialization failed");
    assert_eq!(config, deserialized);
}

#[test]
fn test_config_password_serialization_round_trip() {
    let config = AppConfig {
        servers: vec![Server {
            name: "prod-db".to_string(),
            host: "db.example.com".to_string(),
            port: 5432,
            user: "admin".to_string(),
            auth: Auth::Password {
                vault_key: "vault/prod/db".to_string(),
            },
            tags: vec![],
            pinned: false,
        }],
    };

    let json = serde_json::to_string(&config).expect("serialization failed");
    let deserialized: AppConfig = serde_json::from_str(&json).expect("deserialization failed");
    assert_eq!(config, deserialized);
    assert!(json.contains("\"type\":\"Password\""));
}
