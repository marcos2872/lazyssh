# Documentacao Tecnica — LazySSH

## Visao geral

LazySSH e um cliente SSH/SFTP em TUI (Terminal User Interface) escrito em Rust. Usa [ratatui](https://github.com/ratatui/ratatui) para renderizacao e [russh](https://github.com/warp-tech/russh) como pilha SSH nativa.

Upload/download de arquivos usa SSH direto (`cat local | ssh user@host cat > remote`) — sem o limite de 1GB do SFTP. Operacoes de filesystem (list, mkdir, rename, chmod) continuam usando SFTP nativo.

## Stack tecnologica

| Camada | Tecnologia | Funcao |
|--------|-----------|--------|
| TUI | ratatui 0.29 | Widgets, layout, renderizacao |
| Terminal raw | crossterm 0.28 | Modo raw, captura de teclado/mouse, alternate screen |
| SSH | russh 0.50.0-beta.7 | Conexao SSH, autenticacao, canal PTY |
| SFTP | russh-sftp 2.1.2 | Subsistema SFTP (filesystem ops) |
| Async runtime | tokio 1 (full) | Tasks de IO, canais mpsc |
| Criptografia | ring 0.17 + pbkdf2 0.12 | AES-256-GCM, PBKDF2-HMAC-SHA256 |
| Config | toml 0.8 + serde 1 | Parse/serialize TOML |
| Efeitos | tachyonfx 0.25 | Animacoes (dissolve, fade, pulse) |
| Clipboard | arboard 3 + wl-copy/xclip | Copia de texto selecionado |

## Arquitetura de modulos

```
src/
├── main.rs            # Entry point, event loop, dispatch de views
├── lib.rs             # Re-exports
├── config/
│   ├── models.rs      # Structs Server, Auth, AppConfig
│   └── file.rs        # CRUD do arquivo ~/.config/lazyssh/servers.toml
├── ssh/
│   ├── service.rs     # SshService (nativo, russh) — SESSION PRINCIPAL
│   ├── auth.rs        # Carregamento de chaves (russh-keys)
│   └── exec.rs        # execute_ssh_command() — legado, nao usado
├── sftp/
│   ├── service.rs     # SftpService + SftpServiceSession (nativo, russh-sftp)
│   ├── local.rs       # LocalFs — navegacao do filesystem local
│   └── remote.rs      # RemoteFs — legado, nao usado
├── vault/
│   └── crypto.rs      # AES-256-GCM + PBKDF2
└── tui/
    ├── app.rs         # App (estado global), CurrentView, InputMode
    ├── server_list.rs # Render da lista de servidores
    ├── ssh_terminal.rs# Terminal SSH com parsing ANSI, selecao, clipboard
    ├── sftp_browser.rs# Navegador dual-pane SFTP com bookmarks
    ├── notifications.rs# Fila de notificacoes
    ├── help.rs        # Modal de ajuda, footer contextual, status bar
    ├── effects.rs     # Animacoes tachyonfx
    └── theme.rs       # Paleta de cores
```

## Fluxo de dados

### SSH Terminal

```
Remote server (SSH PTY)
    │
    │ canal SSH (russh)
    v
tokio task: le ChannelMsg::Data do reader
    │
    │ mpsc::unbounded_channel (Vec<u8> -> String)
    v
main loop: drain do SshTerminalState::feed_output()
    │
    │ processa \r, \n, \b e CSI clear sequences
    v
SshTerminalState.output: Vec<String>
    │
    │ terminal.draw() -> render_ssh_terminal()
    │   └─── parse_ansi_spans(linha) -> Vec<(String, Style)>
    │        └─── Style::fg/bg correta com bold/italic/underline
    v
ratatui Paragraph com Spans coloridos
```

### Upload/Download via SSH

```
SFTP Browser (user presses u/d)
    │
    │ spawn_blocking (evita bloquear TUI)
    v
cat local | ssh user@host cat > remote   (upload)
ssh user@host cat remote > local         (download)
    │
    │ Resultado via mpsc::unbounded_channel
    v
main loop: drain sftp_op_rx -> refresh listing
```

Upload e download usam SSH direto, sem SFTP. Isso evita o limite de ~1GB do buffer SFTP do servidor. A TUI fica responsiva porque a transferencia roda em `spawn_blocking`.

Para autenticacao por senha, usa-se `sshpass -p <senha> ssh ...`.

### SFTP (filesystem ops)

```
TUI (ratatui sync)
    │
    │ block_in_place + block_on (operacoes curtas)
    v
SftpServiceSession (russh -> russh_sftp)
    │
    │ subsistema SFTP sobre SSH
    v
Remote server: read_dir, mkdir, rename, chmod, remove
```

## Parsing ANSI

O parser `parse_ansi_spans()` converte sequences ANSI SGR diretamente para `ratatui::style::Style`, preservando cores do servidor remoto.

### Sequences suportadas

- **SGR 0 / 0m a 107m:** 16 cores base (30-37 fg, 40-47 bg), bright (90-97, 100-107)
- **256 cores:** `38;5;N` / `48;5;N` -> `Color::Indexed(N)`
- **Atributos:** 1 bold, 3 italic, 4 underline, 22/23/24 reset
- **Reset:** 0 (ou variante com leading zero como `00`)

### Implementacao

O parser opera em duas etapas:

1. **feed_output()**: processa a stream de caracteres do PTY, detecta CSI de clear e descarta sequences de controle.
2. **parse_ansi_spans()**: na renderizacao, converte SGR em estilos e descarta CSI nao-SGR silenciosamente.

## Gerenciamento de estado

### App (estado global)

```rust
pub struct App {
    pub servers: Vec<Server>,
    pub current_view: CurrentView,      // ServerList | SshTerminal | SftpBrowser
    pub ssh_state: Option<SshTerminalState>,
    pub sftp_state: Option<SftpState>,
    pub ssh_service: SshService,
    pub sftp_service: Arc<Mutex<SftpService>>,
    pub ssh_output_rx: Option<UnboundedReceiver<String>>,
    pub sftp_op_rx: Option<UnboundedReceiver<SftpOpResult>>,
    pub sftp_progress_rx: Option<UnboundedReceiver<u64>>,
    pub notifications: NotificationQueue,
    pub insert_state: Option<InsertState>,
    pub edit_state: Option<EditState>,
    pub help_visible: bool,
    pub confirm_state: Option<ConfirmState>,
    pub start_time: std::time::Instant,
    pub sort_by: SortBy,
    pub effects: Effects,
}
```

### SftpState

```rust
pub struct SftpState {
    pub remote_entries: Vec<DirEntry>,
    pub local_entries: Vec<DirEntry>,
    pub remote_path: String,
    pub local_path: String,
    pub focus_side: Side,
    pub is_transferring: bool,
    pub transfer_progress: Option<TransferProgress>,
    pub input_mode: SftpInputMode,
    pub input_buffer: String,
    pub transfer_start: Option<Instant>,
}
```

Upload/download sao executados via SSH (spawn_blocking), NAO via SFTP. SFTP e usado apenas para operacoes de filesystem: read_dir, mkdir, rename, chmod, remove.

## Async + sync bridge

Operacoes de rede rodam em tasks tokio assincronas. O event loop do ratatui e sincrono. A sincronizacao usa dois padroes:

**Operacoes curtas (SFTP filesystem ops):**
```rust
tokio::task::block_in_place(|| {
    tokio::runtime::Handle::current().block_on(async {
        sftp_session.read_dir(&path).await
    })
})
```

**Operacoes longas (upload/download):**
```rust
tokio::task::spawn_blocking(move || {
    // SSH transfer em thread separada
    // Resultado via mpsc::unbounded_channel
});
```

O SSH PTY envia dados para a TUI via canal:
```rust
let (tx, rx) = mpsc::unbounded_channel::<String>();
// tx vai para a task SSH (envia linhas de output)
// rx vai para App.ssh_output_rx (drenado no inicio de cada frame)
```

## SFTP Operations

### Operacoes de filesystem

Todas as operacoes de filesystem usam o SFTP nativo:

| Operacao | Metodo SFTP | Notas |
|----------|-------------|-------|
| Listar diretorio | `read_dir()` | Retorna `Vec<DirEntry>` |
| Criar diretorio | `create_dir()` | Cria no servidor remoto |
| Renomear | `rename()` | Arquivo ou diretorio |
| Remover | `remove_file()` / `remove_dir()` | Com confirmacao |
| Chmod | `set_metadata()` com `FileAttributes` | Modo octal (ex: 755) |

### Upload/Download

Usam SSH direto para evitar o limite de 1GB do SFTP:

```rust
// Upload: cat local | ssh user@host cat > remote
std::process::Command::new("ssh")
    .args(&["user@host", "cat >", remote_path])
    .stdin(local_file)
    .spawn()

// Download: ssh user@host cat remote > local
std::process::Command::new("ssh")
    .args(&["user@host", "cat", remote_path])
    .stdout(local_file)
    .spawn()
```

Autenticacao por senha usa `sshpass -p <senha>` antes do `ssh`.

## Configuracao (TOML)

Arquivo: `~/.config/lazyssh/servers.toml`

- Structs serde com `#[serde(deny_unknown_fields)]`
- Auth e tagged enum: `#[serde(tag = "type")]` — `"key"` ou `"password"`
- Backup automatico: antes de `save()`, copia `servers.toml` -> `servers.toml.backup`
- Bookmarks de diretorios salvos por servidor

## TUI (ratatui)

### Views

```
ServerList ──Enter──> SshTerminal
    │                      │
    │ s                    │ Ctrl+Q / Esc / exit
    v                      v
SftpBrowser          ServerList
```

### Help system

- `?` abre modal de ajuda com atalhos da view atual
- Footer mostra 3-4 dicas de teclas contextuais
- Status bar mostra informacoes do servidor

### SFTP Browser

- Dual-pane (local + remoto) lado a lado
- Selecao multipla com Space
- Upload/download via SSH com timer
- Operacoes: mkdir (M), rename (R), remove (x), chmod (m)
- Bookmarks: b adiciona, B abre gerenciador

### Cores e thema

Todas as cores sao centralizadas em `theme.rs` via statics `Theme::*()`. Nunca usar cores hardcoded nos widgets — sempre usar Theme.

## Licenca

MIT