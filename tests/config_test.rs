use lazyssh::config::file::{add_server, get_config_path, load_config, load_or_default, remove_server, save_config, update_server};
use lazyssh::config::models::{AppConfig, Auth, Server};
use std::{env as std_env, fs, sync::Mutex};

static CONFIG_TEST_LOCK: Mutex<()> = Mutex::new(());

struct TestConfigGuard;
impl Drop for TestConfigGuard {
    fn drop(&mut self) { std_env::remove_var("LAZYSSH_TEST_CONFIG_PATH"); }
}

fn test_config_root() -> std::path::PathBuf {
    let root = std_env::temp_dir().join(format!("lazyssh_cov_{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    root
}

fn with_test_config<T>(f: impl FnOnce() -> T) -> T {
    let _lock = CONFIG_TEST_LOCK.lock().unwrap();
    let root = test_config_root();
    std_env::set_var("LAZYSSH_TEST_CONFIG_PATH", root.join("servers.toml"));
    let _cleanup = TestConfigGuard;
    f()
}

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
        last_connected: None,
        connection_count: 0,
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
            last_connected: None,
            connection_count: 0,
        }],
        sort_by: None,
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
            last_connected: None,
            connection_count: 0,
        }],
        sort_by: None,
    };

    let json = serde_json::to_string(&config).expect("serialization failed");
    let deserialized: AppConfig = serde_json::from_str(&json).expect("deserialization failed");
    assert_eq!(config, deserialized);
    assert!(json.contains("\"type\":\"Password\""));
}

#[test]
fn test_config_path() {
    let path = get_config_path();
    assert!(path.to_string_lossy().contains("lazyssh"));
}

#[test]
fn test_save_and_load_config() {
    let test_dir = std::env::temp_dir().join("lazyssh_test");
    fs::create_dir_all(&test_dir).unwrap();

    let config = AppConfig {
        servers: vec![Server {
            name: "test".to_string(),
            host: "127.0.0.1".to_string(),
            port: 22,
            user: "user".to_string(),
            auth: Auth::Key {
                path: "~/.ssh/id_rsa".to_string(),
                passphrase: None,
            },
            tags: vec![],
            pinned: false,
            last_connected: None,
            connection_count: 0,
        }],
        sort_by: None,
    };

    let path = test_dir.join("servers.toml");
    save_config(&config, &path).unwrap();
    let loaded = load_config(&path).unwrap();

    assert_eq!(loaded.servers.len(), 1);
    assert_eq!(loaded.servers[0].name, "test");

    fs::remove_dir_all(&test_dir).unwrap();
}

#[test]
fn test_load_config_nonexistent_path() {
    let path = std::env::temp_dir().join("lazyssh_test_nonexistent/servers.toml");
    let result = load_config(&path);
    assert!(result.is_err(), "loading nonexistent config should fail");
}

#[test]
fn test_load_config_invalid_toml() {
    let test_dir = std::env::temp_dir().join("lazyssh_test_invalid");
    fs::create_dir_all(&test_dir).unwrap();
    let path = test_dir.join("bad.toml");
    fs::write(&path, "this is not valid toml {{").unwrap();

    let result = load_config(&path);
    assert!(result.is_err(), "invalid TOML should fail to parse");

    fs::remove_dir_all(&test_dir).unwrap();
}

#[test]
fn test_empty_config() {
    let config = AppConfig { servers: vec![], sort_by: None };
    assert!(config.servers.is_empty());
}

#[test]
fn test_config_with_multiple_servers() {
    let config = AppConfig {
        servers: vec![
            Server {
                name: "alpha".to_string(),
                host: "10.0.0.1".to_string(),
                port: 22,
                user: "user1".to_string(),
                auth: Auth::Key { path: "~/.ssh/id_a".to_string(), passphrase: None },
                tags: vec!["tag1".to_string()],
                pinned: false,
                last_connected: None,
                connection_count: 0,
            },
            Server {
                name: "beta".to_string(),
                host: "10.0.0.2".to_string(),
                port: 2222,
                user: "user2".to_string(),
                auth: Auth::Password { vault_key: "beta_key".to_string() },
                tags: vec!["tag2".to_string(), "tag3".to_string()],
                pinned: true,
                last_connected: None,
                connection_count: 0,
            },
        ],
        sort_by: None,
    };

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: AppConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.servers.len(), 2);
    assert_eq!(deserialized.servers[0].name, "alpha");
    assert_eq!(deserialized.servers[1].port, 2222);
    assert!(deserialized.servers[1].pinned);
    assert_eq!(deserialized.servers[1].tags.len(), 2);
}

#[test]
fn test_auth_key_with_passphrase_serialization() {
    let auth = Auth::Key {
        path: "~/.ssh/id_ed25519".to_string(),
        passphrase: Some("hunter2".to_string()),
    };
    let json = serde_json::to_string(&auth).unwrap();
    let deserialized: Auth = serde_json::from_str(&json).unwrap();
    assert_eq!(auth, deserialized);
    assert!(json.contains("\"passphrase\""));
}

#[test]
fn test_server_defaults() {
    // pinned should default to false, tags to empty
    let toml_str = r#"[[servers]]
name = "defaults"
host = "default-host"
port = 22
user = "default-user"
auth = { type = "Key", path = "~/.ssh/id_rsa" }
"#;
    let config: AppConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(config.servers[0].name, "defaults");
    assert!(!config.servers[0].pinned);
    assert!(config.servers[0].tags.is_empty());
}

#[test]
fn test_save_config_creates_backup() {
    let test_dir = std::env::temp_dir().join("lazyssh_test_backup");
    fs::create_dir_all(&test_dir).unwrap();
    let path = test_dir.join("servers.toml");

    // Save once
    let config = AppConfig {
        servers: vec![Server {
            name: "v1".to_string(),
            host: "10.0.0.1".to_string(),
            port: 22,
            user: "user".to_string(),
            auth: Auth::Key { path: "~/.ssh/id_rsa".to_string(), passphrase: None },
            tags: vec![],
            pinned: false,
            last_connected: None,
            connection_count: 0,
        }],
        sort_by: None,
    };
    save_config(&config, &path).unwrap();

    // Save again — should create backup
    let config_v2 = AppConfig {
        servers: vec![Server {
            name: "v2".to_string(),
            host: "10.0.0.2".to_string(),
            port: 22,
            user: "user".to_string(),
            auth: Auth::Key { path: "~/.ssh/id_rsa".to_string(), passphrase: None },
            tags: vec![],
            pinned: false,
            last_connected: None,
            connection_count: 0,
        }],
        sort_by: None,
    };
    save_config(&config_v2, &path).unwrap();

    let backup_path = test_dir.join("servers.toml.backup");
    assert!(backup_path.exists(), "backup file should exist");

    // Backup should contain v1 data
    let backup_content = fs::read_to_string(&backup_path).unwrap();
    assert!(backup_content.contains("v1"));

    fs::remove_dir_all(&test_dir).unwrap();
}

#[test]
fn test_add_server_creates_entry() {
    with_test_config(|| {

    // Clean up any previous test artifacts
    let path = lazyssh::config::file::get_config_path();
    let _ = fs::remove_file(&path);
    let _ = fs::remove_file(path.with_extension("toml.backup"));

    let server = Server {
        name: "test-add".to_string(),
        host: "10.0.0.1".to_string(),
        port: 22,
        user: "u".to_string(),
        auth: Auth::Key { path: "~/.ssh/id_rsa".to_string(), passphrase: None },
        tags: vec![],
        pinned: false,
        last_connected: None,
        connection_count: 0,
    };
    lazyssh::config::file::add_server(server).unwrap();

    let config = lazyssh::config::file::load_or_default();
    assert!(config.servers.iter().any(|s| s.name == "test-add"));

    // Cleanup
    let _ = fs::remove_file(&path);
    let _ = fs::remove_file(path.with_extension("toml.backup"));

    });
}

#[test]
fn test_remove_server_deletes_entry() {
    with_test_config(|| {

    let path = lazyssh::config::file::get_config_path();
    let _ = fs::remove_file(&path);
    let _ = fs::remove_file(path.with_extension("toml.backup"));

    let server = Server {
        name: "to-remove".to_string(),
        host: "10.0.0.2".to_string(),
        port: 22,
        user: "u".to_string(),
        auth: Auth::Key { path: "~/.ssh/id_rsa".to_string(), passphrase: None },
        tags: vec![],
        pinned: false,
        last_connected: None,
        connection_count: 0,
    };
    lazyssh::config::file::add_server(server).unwrap();
    lazyssh::config::file::remove_server("to-remove").unwrap();

    let config = lazyssh::config::file::load_or_default();
    assert!(!config.servers.iter().any(|s| s.name == "to-remove"));

    let _ = fs::remove_file(&path);
    let _ = fs::remove_file(path.with_extension("toml.backup"));

    });
}

#[test]
fn test_update_server_modifies_entry() {
    with_test_config(|| {

    let path = lazyssh::config::file::get_config_path();
    let _ = fs::remove_file(&path);
    let _ = fs::remove_file(path.with_extension("toml.backup"));

    let server = Server {
        name: "to-update".to_string(),
        host: "old".to_string(),
        port: 22,
        user: "u".to_string(),
        auth: Auth::Key { path: "~/.ssh/id_rsa".to_string(), passphrase: None },
        tags: vec![],
        pinned: false,
        last_connected: None,
        connection_count: 0,
    };
    lazyssh::config::file::add_server(server).unwrap();

    let updated = Server {
        name: "to-update".to_string(),
        host: "new".to_string(),
        port: 2222,
        user: "admin".to_string(),
        auth: Auth::Key { path: "~/.ssh/id_ed25519".to_string(), passphrase: Some("s3cret".into()) },
        tags: vec!["updated".into()],
        pinned: true,
        last_connected: None,
        connection_count: 0,
    };
    lazyssh::config::file::update_server("to-update", updated).unwrap();

    let config = lazyssh::config::file::load_or_default();
    let s = config.servers.iter().find(|s| s.name == "to-update").unwrap();
    assert_eq!(s.host, "new");
    assert_eq!(s.port, 2222);
    assert_eq!(s.user, "admin");
    assert!(s.pinned);

    let _ = fs::remove_file(&path);
    let _ = fs::remove_file(path.with_extension("toml.backup"));

    });
}

#[test]
fn test_load_or_default_no_config() {
    with_test_config(|| {

    let path = lazyssh::config::file::get_config_path();
    let _ = fs::remove_file(&path);
    let _ = fs::remove_file(path.with_extension("toml.backup"));

    let config = lazyssh::config::file::load_or_default();
    assert!(config.servers.is_empty());

    });
}
