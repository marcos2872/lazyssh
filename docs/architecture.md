# Documentacao Tecnica — LazySSH

## Visao geral

LazySSH e um cliente SSH/SFTP em TUI (Terminal User Interface) escrito em Rust. Usa [ratatui](https://github.com/ratatui/ratatui) para renderizacao e [russh](https://github.com/warp-tech/russh) como pilha SSH nativa.

SSH abre em shell externo (native handoff) — o usuario interage com o terminal SSH real do sistema. Upload/download usa SSH direto (`cat local | ssh user@host cat > remote`) — sem o limite de 1GB do SFTP. Operacoes de filesystem (list, mkdir, rename, chmod) continuam usando SFTP nativo.

## Stack tecnologica

| Camada | Tecnologia | Funcao |
|--------|-----------|--------|
| TUI | ratatui 0.29 | Widgets, layout, renderizacao |
| Terminal raw | crossterm 0.28 | Modo raw, captura de teclado/mouse, alternate screen |
| SSH | russh 0.50.0-beta.7 | Conexao SSH, autenticacao (SFTP) |
| SFTP | russh-sftp 2.1.2 | Subsistema SFTP (filesystem ops) |
| Async runtime | tokio 1 (full) | Tasks de IO, canais mpsc |
| Criptografia | ring 0.17 | AES-256-GCM, PBKDF2 |
| Config | toml 0.8 + serde 1 | Parse/serialize TOML |
| Keyring | keyring 3.x | Senhas no keyring do OS |
| Efeitos | tachyonfx 0.25 | Animacoes (dissolve, fade) |
| Clipboard | arboard 3 + wl-copy/xclip | Copia de texto selecionado |

## Arquitetura de modulos

```
src/
├── main.rs            # Entry point, event loop, dispatch de views
├── lib.rs             # Re-exports
├── config/
│   ├── models.rs      # Structs Server, Auth, AppConfig
│   ├── file.rs        # CRUD do arquivo TOML com keyring integration
│   └── ssh_config.rs  # Parser de ~/.ssh/config
├── ssh/
│   ├── service.rs     # SshService (russh), test_connection
│   └── auth.rs        # Carregamento de chaves
├── sftp/
│   ├── service.rs     # SftpService (russh-sftp), filesystem ops
│   └── local.rs       # LocalFs — navegacao do filesystem local
├── vault/
│   ├── crypto.rs      # AES-256-GCM + PBKDF2 (disponivel)
│   └── keyring.rs     # Integracao com keyring do OS
└── tui/
    ├── app.rs         # App (estado global), InsertState, EditState
    ├── server_list.rs # Render da lista de servidores
    ├── ssh_terminal.rs# Terminal SSH com parsing ANSI
    ├── sftp_browser.rs# Navegador dual-pane SFTP
    ├── notifications.rs# Fila de notificacoes
    ├── help.rs        # Modal de ajuda, footer, status bar
    ├── effects.rs     # Animacoes tachyonfx
    └── theme.rs       # Paleta de cores
```

## Fluxo de dados

### SSH Connection (Native Handoff)

```
User presses Enter on ServerList
    │
    │ run_native_shell_handoff()
    v
leave_tui() — disable raw mode, leave alternate screen
    │
    │ \x1bc — full terminal reset
    v
Command::new("ssh") / Command::new("sshpass").args(["ssh", ...])
    │
    │ Process inherits stdin/stdout
    v
User interacts with real SSH terminal
    │
    │ User types "exit" or disconnects
    v
reenter_tui() — restore raw mode + alternate screen
    │
    │ terminal.clear() — force full redraw
    v
Back to ServerList
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

Upload e download usam SSH direto, sem SFTP. Isso evita o limite de ~1GB do buffer SFTP do servidor.

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
    pub filtered_indices: Vec<usize>,
    pub selected: usize,
    pub current_view: CurrentView,
    pub input_mode: InputMode,
    pub should_quit: bool,
    pub sftp_state: Option<SftpState>,
    pub insert_state: Option<InsertState>,
    pub edit_state: Option<EditState>,
    pub notifications: NotificationQueue,
    pub effects: AppEffects,
    pub ssh_service: SshService,
    pub sftp_service: Arc<Mutex<SftpService>>,
    pub sftp_op_rx: Option<UnboundedReceiver<SftpOpResult>>,
    pub sftp_progress_rx: Option<UnboundedReceiver<u64>>,
    pub help_visible: bool,
    pub confirm_state: Option<ConfirmState>,
    pub start_time: std::time::Instant,
    pub sort_by: Option<String>,
    pub command_history: CommandHistory,
}
```

### Server Model

```rust
pub struct Server {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub auth: Auth,                    // Key { path, passphrase } | Password { vault_key }
    pub tags: Vec<String>,
    pub pinned: bool,
    pub last_connected: Option<String>,
    pub connection_count: u32,
    pub bookmarks: Vec<ServerBookmark>,
    pub agent_forwarding: bool,        // -A flag
    pub proxy_jump: Option<String>,    // -J flag
}
```

### Keyring Integration

```
save_config():
    for each server with Auth::Password:
        if vault_key not empty:
            keyring::store_password(server, vault_key)
            vault_key.clear()  // empty in TOML
    write TOML

load_config():
    parse TOML
    for each server with Auth::Password:
        if vault_key is empty:
            vault_key = keyring::get_password(server)
```

## Async + sync bridge

Operacoes de rede rodam em tasks tokio assincronas. O event loop do ratatui e sincrono.

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

**SSH (native handoff):**
```rust
leave_tui()?;
Command::new("ssh").args(args).status()?;
reenter_tui()?;
```

## Configuracao (TOML)

Arquivo: `~/.config/lazyssh/servers.toml`

- Auth e tagged enum: `#[serde(tag = "type")]` — `"key"` ou `"password"`
- Backup automatico: antes de `save()`, copia `servers.toml` -> `servers.toml.backup`
- Keyring: vault_key fica vazio no TOML quando senha esta no keyring

## TUI (ratatui)

### Views

```
ServerList ──Enter──> Shell Externo (SSH)
    │
    │ s
    v
SftpBrowser
```

### Help system

- `?` abre modal de ajuda com atalhos da view atual
- Footer mostra dicas de teclas contextuais
- Status bar mostra informacoes da view

### SFTP Browser

- Dual-pane (local + remoto) lado a lado
- Selecao multipla com Space
- Upload/download via SSH com timer
- Operacoes: mkdir (M), rename (R), remove (x), chmod (m)
- Bookmarks: b adiciona, B navega
- Input com cursor navigation (← → Home End Delete)

### Cores e thema

Todas as cores sao centralizadas em `theme.rs` via statics `Theme::*()`.

## Licenca

MIT