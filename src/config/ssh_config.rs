use crate::config::models::{Auth, Server};
use std::path::Path;

/// Parse ~/.ssh/config and return a list of Servers.
pub fn parse_ssh_config(path: &Path) -> Vec<Server> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut servers = Vec::new();
    let mut current_host: Option<String> = None;
    let mut host_name: Option<String> = None;
    let mut port: u16 = 22;
    let mut user: String = std::env::var("USER").unwrap_or_else(|_| "root".to_string());
    let mut identity_file: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Parse key-value pairs (case-insensitive key)
        let (key, value) = if let Some(idx) = trimmed.find(char::is_whitespace) {
            let k = trimmed[..idx].to_lowercase();
            let v = trimmed[idx + 1..].trim().to_string();
            (k, v)
        } else {
            continue;
        };

        match key.as_str() {
            "host" => {
                // Save previous host block
                if let Some(name) = current_host.take() {
                    let host = host_name.unwrap_or_else(|| name.clone());
                    servers.push(Server {
                        name,
                        host,
                        port,
                        user: user.clone(),
                        auth: identity_file
                            .as_ref()
                            .map(|p| Auth::Key {
                                path: p.clone(),
                                passphrase: None,
                            })
                            .unwrap_or_else(|| Auth::Key {
                                path: "~/.ssh/id_rsa".to_string(),
                                passphrase: None,
                            }),
                        tags: vec!["imported".to_string()],
                        pinned: false,
                        last_connected: None,
                        connection_count: 0,
                        bookmarks: vec![],
                        agent_forwarding: false,
                        proxy_jump: None,
            log_enabled: false,
            history_enabled: false,
                    });
                }
                // Start new host block (support multiple patterns — use first non-wildcard)
                if !value.starts_with('*') && !value.contains('*') && !value.contains('?') {
                    current_host = Some(value.clone());
                } else {
                    current_host = None;
                }
                host_name = None;
                port = 22;
                user = std::env::var("USER").unwrap_or_else(|_| "root".to_string());
                identity_file = None;
            }
            "hostname" => {
                host_name = Some(value);
            }
            "port" => {
                if let Ok(p) = value.parse::<u16>() {
                    port = p;
                }
            }
            "user" => {
                user = value;
            }
            "identityfile" => {
                // Expand ~ to home dir
                let expanded = if value.starts_with('~') {
                    let home = std::env::var("HOME").unwrap_or_default();
                    value.replacen('~', &home, 1)
                } else {
                    value
                };
                identity_file = Some(expanded);
            }
            _ => {}
        }
    }

    // Save last host block
    if let Some(name) = current_host {
        let host = host_name.unwrap_or_else(|| name.clone());
        servers.push(Server {
            name,
            host,
            port,
            user,
            auth: identity_file
                .as_ref()
                .map(|p| Auth::Key {
                    path: p.clone(),
                    passphrase: None,
                })
                .unwrap_or_else(|| Auth::Key {
                    path: "~/.ssh/id_rsa".to_string(),
                    passphrase: None,
                }),
            tags: vec!["imported".to_string()],
            pinned: false,
            last_connected: None,
            connection_count: 0,
            bookmarks: vec![],
            agent_forwarding: false,
            proxy_jump: None,
            log_enabled: false,
            history_enabled: false,
        });
    }

    servers
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_config(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn test_parse_single_host() {
        let f = make_config(
            r#"
Host myserver
    HostName 192.168.1.100
    Port 22
    User root
    IdentityFile ~/.ssh/id_ed25519
"#,
        );
        let servers = parse_ssh_config(f.path());
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "myserver");
        assert_eq!(servers[0].host, "192.168.1.100");
        assert_eq!(servers[0].port, 22);
        assert_eq!(servers[0].user, "root");
        assert!(servers[0].tags.contains(&"imported".to_string()));
    }

    #[test]
    fn test_parse_multiple_hosts() {
        let f = make_config(
            r#"
Host server1
    HostName 10.0.0.1
    User admin

Host server2
    HostName 10.0.0.2
    Port 2222
    User deploy
    IdentityFile /home/me/.ssh/deploy_key
"#,
        );
        let servers = parse_ssh_config(f.path());
        assert_eq!(servers.len(), 2);
        assert_eq!(servers[0].name, "server1");
        assert_eq!(servers[0].port, 22); // default
        assert_eq!(servers[1].name, "server2");
        assert_eq!(servers[1].port, 2222);
    }

    #[test]
    fn test_parse_wildcard_host_ignored() {
        let f = make_config(
            r#"
Host *
    User default_user

Host realserver
    HostName 10.0.0.3
"#,
        );
        let servers = parse_ssh_config(f.path());
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "realserver");
    }

    #[test]
    fn test_parse_defaults() {
        let f = make_config(
            r#"
Host minimal
    HostName example.com
"#,
        );
        let servers = parse_ssh_config(f.path());
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].port, 22);
        // user should be current username
    }

    #[test]
    fn test_parse_comments_and_empty_lines() {
        let f = make_config(
            r#"
# This is a comment

Host server1
    # inline comment
    HostName 1.2.3.4

    # blank lines above
"#,
        );
        let servers = parse_ssh_config(f.path());
        assert_eq!(servers.len(), 1);
    }

    #[test]
    fn test_parse_nonexistent_file() {
        let servers = parse_ssh_config(Path::new("/nonexistent/config"));
        assert_eq!(servers.len(), 0);
    }

    #[test]
    fn test_parse_hostname_fallback() {
        // If no HostName, use Host value
        let f = make_config(
            r#"
Host myalias
    User root
"#,
        );
        let servers = parse_ssh_config(f.path());
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].host, "myalias");
    }

    #[test]
    fn test_default_auth() {
        let f = make_config(
            r#"
Host s
    HostName h
"#,
        );
        let servers = parse_ssh_config(f.path());
        match &servers[0].auth {
            Auth::Key { path, passphrase } => {
                assert_eq!(path, "~/.ssh/id_rsa");
                assert!(passphrase.is_none());
            }
            _ => panic!("Expected Key auth"),
        }
    }

    #[test]
    fn test_imported_tag_present() {
        let f = make_config(
            r#"
Host server1
    HostName 10.0.0.1
"#,
        );
        let servers = parse_ssh_config(f.path());
        assert!(servers[0].tags.contains(&"imported".to_string()));
    }

    #[test]
    fn test_host_wildcard_skipped() {
        let f = make_config(
            r#"
Host *
    User default

Host server1
    HostName 1.1.1.1

Host *
    User other
"#,
        );
        let servers = parse_ssh_config(f.path());
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "server1");
    }

    #[test]
    fn test_empty_config() {
        let f = make_config("");
        let servers = parse_ssh_config(f.path());
        assert!(servers.is_empty());
    }
}
