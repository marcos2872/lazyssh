use crate::config::models::{Auth, Server};

pub fn build_ssh_args(server: &Server) -> (String, Vec<String>) {
    let mut ssh_args = vec![];

    if server.port != 22 {
        ssh_args.push("-p".to_string());
        ssh_args.push(server.port.to_string());
    }

    if server.agent_forwarding {
        ssh_args.push("-A".to_string());
    }

    if let Some(ref jump) = server.proxy_jump {
        if !jump.is_empty() {
            ssh_args.push("-J".to_string());
            ssh_args.push(jump.clone());
        }
    }

    match &server.auth {
        Auth::Key { path, .. } => {
            let expanded = shellexpand::tilde(path).into_owned();
            ssh_args.push("-i".to_string());
            ssh_args.push(expanded);
        }
        Auth::Password { vault_key } => {
            if !vault_key.is_empty() {
                let mut args = vec![
                    "sshpass".to_string(),
                    "-p".to_string(),
                    vault_key.clone(),
                    "ssh".to_string(),
                ];
                args.append(&mut ssh_args);
                args.push(format!("{}@{}", server.user, server.host));
                return ("sshpass".to_string(), args);
            }
        }
    }

    ssh_args.push(format!("{}@{}", server.user, server.host));
    ("ssh".to_string(), ssh_args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::models::{Auth, Server};

    fn make_server(auth: Auth) -> Server {
        Server {
            name: "test".into(),
            host: "example.com".into(),
            port: 22,
            user: "root".into(),
            auth,
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
    fn test_key_auth() {
        let server = make_server(Auth::Key {
            path: "~/.ssh/id_ed25519".into(),
            passphrase: None,
        });
        let (cmd, args) = build_ssh_args(&server);
        assert_eq!(cmd, "ssh");
        assert!(args.contains(&"-i".to_string()));
        assert!(args.contains(&"root@example.com".to_string()));
    }

    #[test]
    fn test_password_auth() {
        let server = make_server(Auth::Password {
            vault_key: "secret123".into(),
        });
        let (cmd, args) = build_ssh_args(&server);
        assert_eq!(cmd, "sshpass");
        assert_eq!(args[1], "-p");
        assert_eq!(args[2], "secret123");
    }

    #[test]
    fn test_agent_forwarding() {
        let mut server = make_server(Auth::Key {
            path: "~/.ssh/id_rsa".into(),
            passphrase: None,
        });
        server.agent_forwarding = true;
        let (_, args) = build_ssh_args(&server);
        assert!(args.contains(&"-A".to_string()));
    }

    #[test]
    fn test_proxy_jump() {
        let mut server = make_server(Auth::Key {
            path: "~/.ssh/id_rsa".into(),
            passphrase: None,
        });
        server.proxy_jump = Some("user@bastion.example.com".into());
        let (_, args) = build_ssh_args(&server);
        assert!(args.contains(&"-J".to_string()));
    }

    #[test]
    fn test_empty_proxy_jump_ignored() {
        let mut server = make_server(Auth::Key {
            path: "~/.ssh/id_rsa".into(),
            passphrase: None,
        });
        server.proxy_jump = Some("".into());
        let (_, args) = build_ssh_args(&server);
        assert!(!args.contains(&"-J".to_string()));
    }
}