# LazySSH

Gerenciador de conexoes SSH/SFTP em TUI (Terminal UI) escrito em Rust.

## Funcionalidades

- Lista de servidores persistente em TOML com busca fuzzy
- Terminal SSH interativo com PTY (cores ANSI, scroll, selecao de texto)
- Navegador SFTP dual-pane (local + remoto) com upload/download
- Autenticacao por chave SSH (Ed25519, RSA, ECDSA) ou senha
- Criptografia de senhas com AES-256-GCM + PBKDF2
- Suporte a tags e favoritos (pinned)
- Clipboard integrado (Wayland e X11)
- Operacao nativa via russh (sem depender de ssh/sshpass externo)

## Instalacao

### Dependencias

- Rust toolchain (edition 2021)
- Opcional: `sshpass` para fallback de autenticacao por senha via SSH externo

### Compilar

```bash
git clone https://github.com/marcos2872/lazyssh
cd lazyssh
cargo build --release
```

### Executar

```bash
cargo run --release
```

O binario estara em `./target/release/lazyssh`.

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
vault_key = "minha-senha-aqui"
```

### Schema

| Campo | Tipo | Descricao |
|-------|------|-----------|
| `name` | string | Nome do servidor (usado para exibicao na lista) |
| `host` | string | Endereco IP ou hostname |
| `port` | int | Porta SSH (padrao 22) |
| `user` | string | Usuario de login |
| `tags` | string[] | Tags para filtro (opcional) |
| `pinned` | bool | Fixar no topo da lista (opcional) |
| `auth` | Auth | Autenticacao (ver abaixo) |

### Auth

**Chave SSH (`type = "key"`):**

| Campo | Tipo | Descricao |
|-------|------|-----------|
| `path` | string | Caminho para chave privada (suporta `~`) |
| `passphrase` | string? | Passphrase da chave (opcional) |

**Senha (`type = "password"`):**

| Campo | Tipo | Descricao |
|-------|------|-----------|
| `vault_key` | string | Senha (armazenada criptografada no vault) |

O arquivo de configuracao tem backup automatico em `.toml.backup` antes de qualquer salvamento.

## Uso

### Tela inicial (ServerList)

| Tecla | Acao |
|-------|------|
| `j` / `Down` | Navegar para baixo |
| `k` / `Up` | Navegar para cima |
| `Enter` | Conectar SSH no servidor selecionado |
| `s` | Abrir SFTP (navegador de arquivos) |
| `a` | Adicionar novo servidor |
| `e` | Editar servidor selecionado |
| `d` | Deletar servidor selecionado |
| `p` | Fixar/desselecionar servidor no topo |
| `/` | Ativar busca fuzzy |
| `q` | Sair |

### Terminal SSH

| Tecla | Acao |
|-------|------|
| `Ctrl+Q` / `Esc` | Desconectar e voltar a lista |
| `PageUp` / `PageDown` | Rolar output |
| Scroll do mouse | Rolar output |
| Mouse drag | Selecionar texto (copia automatica) |
| Qualquer outra tecla | Enviada ao shell remoto |

Ao digitar `exit` no shell remoto, a conexao encerra e voce volta automaticamente a lista de servidores.

A saida do terminal e renderizada com cores ANSI preservadas (SGR sequences com suporte a 256 cores, bold, italic, underline). Sequencias de controle nao-SGR (bracketed paste, cursor movement, OSC) sao descartadas silenciosamente.

### Navegador SFTP

| Tecla | Acao |
|-------|------|
| `Tab` | Alternar foco entre painel Local e Remoto |
| `Enter` | Entrar no diretorio |
| `Backspace` | Voltar ao diretorio pai |
| `Space` | Selecionar/desselecionar arquivo |
| `a` | Selecionar todos os arquivos |
| `u` | Upload (envia arquivos selecionados do Local) |
| `d` | Download (baixa arquivos selecionados do Remoto) |
| `r` | Atualizar listagem |
| `q` / `Esc` | Voltar a lista de servidores |
| `j`/`k` ou `Down`/`Up` | Navegar |

## Arquitetura

```
src/
├── main.rs              # Entry point, event loop, key/mouse dispatch
├── lib.rs               # Re-exports modulos
├── config/              # Configuracao TOML (models, CRUD file)
├── ssh/
│   └── service.rs       # Cliente SSH nativo (russh), PTY, sessao persistente
├── sftp/
│   ├── service.rs       # Cliente SFTP nativo (russh-sftp)
│   └── local.rs         # Navegacao do filesystem local
├── vault/               # Criptografia AES-256-GCM + PBKDF2
└── tui/
    ├── app.rs           # Estado global e navegacao entre views
    ├── server_list.rs   # Render da lista de servidores
    ├── ssh_terminal.rs  # Terminal SSH com parsing ANSI e selecao
    ├── sftp_browser.rs  # Navegador dual-pane SFTP
    ├── notifications.rs # Fila de notificacoes com timeout
    ├── effects.rs       # Efeitos visuais (tachyonfx)
    └── theme.rs         # Paleta de cores e estilos
```

### Decisoes de design

- **SSH nativo (russh):** substitui ssh/sshpass externo por cliente SSH em Rust puro, com sessao persistente e PTY real.
- **Async + sync bridge:** SSS/SFTP rodam em tasks tokio assincronas; o event loop do ratatui e sincrono. A comunicacao entre eles e feita via `mpsc::unbounded_channel` com `block_in_place()` para sincronizar operacoes async.
- **ANSI parseado, nao stripped:** o parser `parse_ansi_spans()` converte sequences SGR diretamente para Styles do ratatui, preservando cores e formatacao do servidor remoto. Sequencias de controle nao-SGR sao descartadas.
- **Dual-pane SFTP:** navegacao local e remota lado a lado, com selecao multipla e barra de progresso de transferencia.
- **TOML como config:** formato simples, editavel manualmente, com backup automatico.

## Licenca

MIT
