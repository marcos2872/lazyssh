use crate::config::models::{Auth, Server};

/// Constrói os argumentos de linha de comando SSH para uma conexão de servidor.
///
/// Retorna `(programa, argumentos, senha)` onde:
/// - `programa` é `"ssh"` para autenticação por chave ou `"sshpass"` para senha
/// - `argumentos` contém o vetor completo de argumentos
/// - `senha` é `Some(...)` para autenticação por senha (via variável `SSHPASS`)
///
/// Trata porta (`-p`), encaminhamento de agent (`-A`), proxy jump (`-J`)
/// e arquivo de identidade (`-i`) baseado na configuração do servidor.
pub fn build_ssh_args(server: &Server) -> (String, Vec<String>, Option<String>) {
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
                ssh_args.push(format!("{}@{}", server.user, server.host));
                ssh_args.insert(0, "ssh".to_string());
                ssh_args.insert(0, "-e".to_string());
                return ("sshpass".to_string(), ssh_args, Some(vault_key.clone()));
            }
        }
    }

    ssh_args.push(format!("{}@{}", server.user, server.host));
    ("ssh".to_string(), ssh_args, None)
}

/// Gera um subprocesso SSH com a configuração do servidor informado.
///
/// Envelopa `build_ssh_args` e configura redirecionamentos de stdin/stdout/stderr.
/// Para autenticação por senha, define a variável de ambiente `SSHPASS` para que
/// `sshpass -e` possa lê-la sem expor a senha na linha de comando.
pub fn spawn_ssh_process(
    server: &Server,
    extra_args: Vec<String>,
    stdin: Option<std::process::Stdio>,
    stdout: Option<std::process::Stdio>,
    stderr: Option<std::process::Stdio>,
) -> Result<std::process::Child, String> {
    let (cmd, mut args, password) = build_ssh_args(server);
    args.extend(extra_args);

    let mut command = std::process::Command::new(&cmd);
    command.args(&args);
    if let Some(s) = stdin {
        command.stdin(s);
    }
    if let Some(s) = stdout {
        command.stdout(s);
    }
    if let Some(s) = stderr {
        command.stderr(s);
    }

    if let Some(ref pw) = password {
        command.env("SSHPASS", pw);
    }

    command.spawn().map_err(|e| format!("{} erro: {}", cmd, e))
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
        let (cmd, args, _pw) = build_ssh_args(&server);
        assert_eq!(cmd, "ssh");
        assert!(args.contains(&"-i".to_string()));
        assert!(args.contains(&"root@example.com".to_string()));
    }

    #[test]
    fn test_password_auth() {
        let server = make_server(Auth::Password {
            vault_key: "secret123".into(),
        });
        let (cmd, args, pw) = build_ssh_args(&server);
        assert_eq!(cmd, "sshpass");
        assert_eq!(args[0], "-e");
        assert_eq!(args[1], "ssh");
        assert!(args.contains(&"root@example.com".to_string()));
        assert_eq!(pw, Some("secret123".into()));
        // Password must NOT appear in args
        assert!(!args.contains(&"secret123".to_string()));
    }

    #[test]
    fn test_agent_forwarding() {
        let mut server = make_server(Auth::Key {
            path: "~/.ssh/id_rsa".into(),
            passphrase: None,
        });
        server.agent_forwarding = true;
        let (_, args, _) = build_ssh_args(&server);
        assert!(args.contains(&"-A".to_string()));
    }

    #[test]
    fn test_proxy_jump() {
        let mut server = make_server(Auth::Key {
            path: "~/.ssh/id_rsa".into(),
            passphrase: None,
        });
        server.proxy_jump = Some("user@bastion.example.com".into());
        let (_, args, _) = build_ssh_args(&server);
        assert!(args.contains(&"-J".to_string()));
    }

    #[test]
    fn test_empty_proxy_jump_ignored() {
        let mut server = make_server(Auth::Key {
            path: "~/.ssh/id_rsa".into(),
            passphrase: None,
        });
        server.proxy_jump = Some("".into());
        let (_, args, _) = build_ssh_args(&server);
        assert!(!args.contains(&"-J".to_string()));
    }
}