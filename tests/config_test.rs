use lazyssh::config::models::{Auth, Server};

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
