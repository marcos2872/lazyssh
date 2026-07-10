# LazySSH

Gerenciador de conexoes SSH/SFTP em TUI (Terminal UI) escrito em Rust.

## Funcionalidades

- Lista de servidores persistente em TOML com busca fuzzy
- Terminal SSH interativo via shell externo (native handoff)
- Navegador SFTP dual-pane (local + remoto) com upload/download via SSH
- Autenticacao por chave SSH (Ed25519, RSA, ECDSA) ou senha
- Senhas armazenadas no keyring do OS (GNOME Keyring, KDE Wallet, macOS Keychain)
- Suporte a tags e favoritos (pinned)
- Clipboard integrado (Wayland, X11, arboard)
- Import de ~/.ssh/config
- Teste de conectividade TCP
- SSH Agent Forwarding (-A) e ProxyJump (-J)
- Modal de ajuda com (?) — mostra atalhos disponiveis em cada view
- Footer contextual com dicas de teclas
- Ordenacao de servidores por nome/porta/frequencia
- Upload/download via SSH (sem limite de 1GB do SFTP)
- Timer de transferencia (MM:SS)
- Input com cursor navigation (setas, Home, End, Delete)

## Instalacao

### Instalacao rapida (Linux e macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/marcos2872/lazyssh/master/install.sh | bash
```

O script detecta automaticamente a plataforma (Linux/macOS, x86_64/arm64), baixa o binario e configura o PATH.

**Suporte a shells:** bash, zsh, fish

### Download manual

Acesse a [releases page](https://github.com/marcos2872/lazyssh/releases) e baixe o arquivo correspondente a sua plataforma:

| Arquivo | Plataforma |
|---------|-----------|
| `lazyssh-v0.1.0-linux-x86_64.tar.gz` | Linux x86_64 |
| `lazyssh-v0.1.0-linux-aarch64.tar.gz` | Linux ARM64 (quando disponivel) |
| `lazyssh-v0.1.0-macos-x86_64.tar.gz` | macOS Intel (quando disponivel) |
| `lazyssh-v0.1.0-macos-aarch64.tar.gz` | macOS Apple Silicon (quando disponivel) |

```bash
# Exemplo: Linux x86_64
tar xzf lazyssh-v0.1.0-linux-x86_64.tar.gz
chmod +x lazyssh
sudo mv lazyssh /usr/local/bin/
```

### Compilar a partir do codigo

```bash
git clone https://github.com/marcos2872/lazyssh
cd lazyssh
cargo build --release
```

O binario estara em `./target/release/lazyssh`.

### Dependencias

- `sshpass` para autenticacao por senha em upload/download via SSH
- Keyring do OS (GNOME Keyring, KDE Wallet, ou macOS Keychain) para armazenamento seguro de senhas

## Configuracao

Arquivo: `~/.config/lazyssh/servers.toml`

```toml
[[servers]]
name = "meu-servidor"
host = "192.168.1.100"
port = 22
user = "root"
tags = ["dev", "internal"]
pinned = true
agent_forwarding = false
proxy_jump = "user@bastion.com"

[servers.auth]
type = "key"
path = "~/.ssh/id_ed25519"
passphrase = "opcional"

[[servers]]
name = "dev-box"
host = "dev.internal.com"
port = 2222
user = "admin"

[servers.auth]
type = "password"
vault_key = ""
```

### Schema

| Campo | Tipo | Descricao |
|-------|------|-----------|
| `name` | string | Nome do servidor |
| `host` | string | Endereco IP ou hostname |
| `port` | int | Porta SSH (padrao 22) |
| `user` | string | Usuario de login |
| `tags` | string[] | Tags para filtro (opcional) |
| `pinned` | bool | Fixar no topo da lista (opcional) |
| `auth` | Auth | Autenticacao (ver abaixo) |
| `agent_forwarding` | bool | Habilitar -A no SSH (opcional) |
| `proxy_jump` | string? | Host de salto -J (opcional) |

### Auth

**Chave SSH (`type = "key"`):**

| Campo | Tipo | Descricao |
|-------|------|-----------|
| `path` | string | Caminho para chave privada (suporta `~`) |
| `passphrase` | string? | Passphrase da chave (opcional) |

**Senha (`type = "password"`):**

| Campo | Tipo | Descricao |
|-------|------|-----------|
| `vault_key` | string | Senha (armazenada no keyring do OS; campo fica vazio no TOML) |

O arquivo de configuracao tem backup automatico em `.toml.backup` antes de qualquer salvamento.

## Uso

### Tela inicial (ServerList)

| Tecla | Acao |
|-------|------|
| `j` / `Down` | Navegar para baixo |
| `k` / `Up` | Navegar para cima |
| `Enter` | Conectar SSH (shell externo) |
| `s` | Abrir SFTP |
| `a` | Adicionar servidor (modal) |
| `e` | Editar servidor (modal) |
| `d` | Deletar servidor (com confirmacao) |
| `p` | Fixar/desafixar no topo |
| `/` | Busca fuzzy |
| `i` | Importar de ~/.ssh/config |
| `t` | Testar conectividade TCP |
| `y` | Copiar hostname para clipboard |
| `Y` | Copiar user@host:port para clipboard |
| `O` | Ciclar ordenacao |
| `q` | Sair |
| `?` | Modal de ajuda |

### Modal de Adicionar/Editar

Campos (ordem): Nome, Host, Porta, Usuario, Auth (toggle key/password com ← →), [Chave/Passphrase ou Senha], Tags

| Tecla | Acao |
|-------|------|
| `Tab` / `↓` | Proximo campo |
| `↑` | Campo anterior |
| `←` / `→` / `Space` | Toggle key/password (no campo Auth) |
| `Enter` | Salvar (no campo Tags) |
| `Esc` | Cancelar |

### Terminal SSH

SSH abre em shell externo (native handoff). O terminal e resetado com `\x1bc` antes de conectar.

### Navegador SFTP

| Tecla | Acao |
|-------|------|
| `Tab` | Alternar foco entre painel Local e Remoto |
| `j`/`k` ou `Down`/`Up` | Navegar |
| `Enter` | Entrar no diretorio |
| `Backspace` | Voltar ao diretorio pai |
| `Space` | Selecionar/desselecionar |
| `a` | Selecionar todos |
| `u` | Upload via SSH |
| `d` | Download via SSH |
| `M` | Criar diretorio remoto |
| `R` | Renomear arquivo/diretorio |
| `x` | Remover arquivo/diretorio |
| `m` | Alterar permissoes (chmod) |
| `b` | Salvar bookmark |
| `B` | Navegar para bookmark |
| `r` | Atualizar listagem |
| `q` / `Esc` | Voltar a lista |

### Input SFTP (Mkdir/Rename/Chmod/Bookmark)

| Tecla | Acao |
|-------|------|
| `←` / `→` | Mover cursor |
| `Home` / `End` | Inicio / Final do texto |
| `Delete` | Remover caractere a direita |
| `Backspace` | Remover caractere a esquerda |
| `Enter` | Executar operacao |
| `Esc` | Cancelar |

## Arquitetura

```
src/
├── main.rs              # Entry point, event loop, key/mouse dispatch
├── lib.rs               # Re-exports modulos
├── config/
│   ├── models.rs        # Structs Server, Auth, AppConfig
│   ├── file.rs          # CRUD do arquivo TOML com keyring integration
│   └── ssh_config.rs    # Parser de ~/.ssh/config
├── ssh/
│   ├── service.rs       # SshService (russh), test_connection
│   └── auth.rs          # Carregamento de chaves
├── sftp/
│   ├── service.rs       # SftpService (russh-sftp), filesystem ops
│   └── local.rs         # Navegacao do filesystem local
├── vault/
│   ├── crypto.rs        # AES-256-GCM + PBKDF2 (disponivel)
│   └── keyring.rs       # Integracao com keyring do OS
└── tui/
    ├── app.rs           # Estado global, InsertState, EditState
    ├── server_list.rs   # Render da lista de servidores
    ├── ssh_terminal.rs  # Terminal SSH com parsing ANSI
    ├── sftp_browser.rs  # Navegador dual-pane SFTP
    ├── notifications.rs # Fila de notificacoes
    ├── help.rs          # Modal de ajuda, footer, status bar
    ├── effects.rs       # Efeitos visuais (tachyonfx)
    └── theme.rs         # Paleta de cores
```

### Decisoes de design

- **Shell externo para SSH:** SSH abre em processo externo (native handoff) para maxima compatibilidade. Terminal e resetado com `\x1bc` antes de conectar.
- **Upload/download via SSH:** usa `cat local | ssh user@host cat > remote` — sem limite de 1GB do SFTP.
- **Keyring para senhas:** senhas sao armazenadas no keyring do OS. No TOML, vault_key fica vazio. Fallback para plaintext se keyring indisponivel.
- **ANSI parseado, nao stripped:** o parser converte SGR diretamente para Styles do ratatui.
- **Dual-pane SFTP:** navegacao local e remota lado a lado, com selecao multipla e bookmarks.
- **TOML como config:** formato simples, editavel manualmente, com backup automatico.

## Licenca

MIT
