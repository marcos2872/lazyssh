# Documentacao Tecnica — LazySSH

## Visao geral

LazySSH e um cliente SSH/SFTP em TUI (Terminal User Interface) escrito em Rust. Usa [ratatui](https://github.com/ratatui/ratatui) para renderizacao e [russh](https://github.com/warp-tech/russh) como pilha SSH nativa — sem depender de binarios externos como `ssh`, `sshpass` ou `scp`.

O projeto substitui uma implementacao inicial baseada em processos externos (sshpass + scp) por uma arquitetura nativa Rust com sessao SSH persistente, PTY interativo e SFTP sobre subsistema SSH.

## Stack tecnologica

| Camada | Tecnologia | Funcao |
|--------|-----------|--------|
| TUI | ratatui 0.29 | Widgets, layout, renderizacao |
| Terminal raw | crossterm 0.28 | Modo raw, captura de teclado/mouse, alternate screen |
| SSH | russh 0.50.0-beta.7 | Conexao SSH, autenticacao, canal PTY |
| SFTP | russh-sftp 2.1.2 | Subsistema SFTP sobre canal SSH |
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
│   ├── connection.rs  # SshSession legacy (nao usado pelo TUI)
│   └── exec.rs        # execute_ssh_command() — fallback ssh/sshpass externo
├── sftp/
│   ├── service.rs     # SftpService + SftpServiceSession (nativo, russh-sftp)
│   ├── local.rs       # LocalFs — navegacao do filesystem local
│   └── remote.rs      # RemoteFs — fallback scp/sshpass (legado, nao usado)
├── vault/
│   └── crypto.rs      # AES-256-GCM + PBKDF2
└── tui/
    ├── app.rs         # App (estado global), CurrentView, InputMode
    ├── server_list.rs # Render da lista de servidores
    ├── ssh_terminal.rs# Terminal SSH com parsing ANSI, selecao, clipboard
    ├── sftp_browser.rs# Navegador dual-pane SFTP
    ├── notifications.rs# Fila de notificacoes
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

O fluxo inverso (input do usuario):

```
Tecla pressionada (crossterm event)
    │
    │ SshTerminal handler (Ctrl+Q/Esc: volta, PgUp/Dn: scroll,
    │  senao: key_event_to_bytes() -> Vec<u8>)
    v
tokio::spawn: writer.write_all(bytes)
    │
    │ canal SSH (russh)
    v
Remote server (SSH PTY)
```

### SFTP

```
TUI (ratatui sync)
    │
    │ tokio::task::block_in_place + Handle::current().block_on()
    v
SftpServiceSession (russh -> russh_sftp)
    │
    │ subsistema SFTP sobre SSH
    v
Remote server (SSH SFTP)
```

## Parsing ANSI

O parser `parse_ansi_spans()` converte sequences ANSI SGR diretamente para `ratatui::style::Style`, preservando cores do servidor remoto.

### Sequences suportadas

- **SGR 0 / 0m a 107m:** 16 cores base (30-37 fg, 40-47 bg), bright (90-97, 100-107)
- **256 cores:** `38;5;N` / `48;5;N` -> `Color::Indexed(N)`
- **Atributos:** 1 bold, 3 italic, 4 underline, 22/23/24 reset
- **Reset:** 0 (ou variante com leading zero como `00`)

### Sequences descartadas

- **CSI nao-SGR:** qualquer sequence CSI com terminador `h`, `l`, `J`, `K`, `A`-`D`, `H`, etc. (bracketed paste, cursor movement, clear screen, etc.)
- **OSC:** qualquer sequence OSC (`\x1b]...`) ate BEL (`\x07`) ou ST (`\x1b\\`)
- **CSI intermediarios/privados:** `[?2004h`, `[>1;123c`, etc.

### Implementacao

O parser opera em duas etapas:

1. **feed_output()**: processa a stream de caracteres do PTY, detecta CSI de clear (`[2J` limpa buffer, `[J` limpa linha corrente, `[H` ignorado) e descarta sequences de controle antes de chegarem ao buffer de exibicao.

2. **parse_ansi_spans()**: na renderizacao, percorre cada linha do buffer, detecta `\x1b[` seguido de parametros e terminador. Se terminador for `m`, aplica SGR ao estilo corrente. Para qualquer outro terminador, descarta a sequence. O texto entre sequences e acumulado com o estilo corrente.

```rust
// Pseudocodigo do parse_ansi_spans
while let Some(c) = chars.next() {
    if c == '\x1b' && chars.next() == Some('[') {
        let terminator = collect_params_until_command_letter();
        if terminator == 'm' { apply_sgr(params, &mut current_style); }
        // non-SGR: silently discarded
    } else {
        text += c; // texto literal, aplica estilo corrente
    }
}
```

## Gerenciamento de estado

### App (estado global)

```rust
pub struct App {
    pub servers: Vec<Server>,           // lista de servidores
    pub current_view: CurrentView,      // ServerList | SshTerminal | SftpBrowser
    pub ssh_state: Option<SshTerminalState>,
    pub sftp_state: Option<SftpState>,
    pub ssh_service: SshService,
    pub sftp_service: SftpService,
    pub ssh_output_rx: Option<UnboundedReceiver<String>>,
    pub notifications: NotificationQueue,
    pub insert_state: Option<InsertState>,  // modal add server
    pub edit_state: Option<EditState>,      // modal edit server
    pub search_query: String,
    pub input_mode: InputMode,
}
```

### SshTerminalState

```rust
pub struct SshTerminalState {
    pub output: Vec<String>,            // linhas do output do PTY
    pub current_line: String,           // linha sendo acumulada
    pub scroll_offset: usize,
    pub status: SshStatus,              // Connecting | Connected | Error | Disconnected
    pub session_id: Option<String>,
    pub selection: Option<Selection>,
    pub clipboard: Option<arboard::Clipboard>,
    // Offsets de renderizacao (Cell para escrita imutavel no render)
    pub first_visible_line: Cell<usize>,
    pub padding_top: Cell<usize>,
}
```

### Ciclo principal

O event loop em `main()` segue esta estrutura:

1. Drain do SSH output (se conectado) — processa dados do canal mpsc
2. `terminal.draw(|f| { ... })` — renderiza a view atual
3. `event::poll(timeout)` — espera por evento de teclado/mouse
4. Match sobre `app.current_view` + tipo do evento:
   - ServerList: navegacao, search, add/edit/delete server
   - SshTerminal: Ctrl+Q/Esc sai, PgUp/Dn scroll, resto vai ao PTY
   - SftpBrowser: navegacao dual-pane, upload/download

## Async + sync bridge

O ratatui executa em modo sincrono dentro de `#[tokio::main]`. Operacoes de rede (SSH connect, SFTP list/transfer) sao assincronas no tokio. A sincronizacao e feita assim:

```rust
// Chamar async de dentro do event loop sincrono
tokio::task::block_in_place(|| {
    tokio::runtime::Handle::current().block_on(async {
        // operacao SSH/SFTP aqui
    })
})
```

O SSH PTY (background task) envia dados para a TUI via canal:
```rust
let (tx, rx) = mpsc::unbounded_channel::<String>();
// tx vai para a task SSH (envia linhas de output)
// rx vai para App.ssh_output_rx (drenado no inicio de cada frame)
```

## Criptografia (Vault)

Senhas sao criptografadas com AES-256-GCM antes de serem escritas no arquivo de configuracao.

```
senha_plana
    │
    │ derive_key(salt, 100000 iteracoes PBKDF2-HMAC-SHA256)
    v
key (256 bits)
    │
    │ encrypt(key, nonce 12 bytes, senha)
    v
nonce || ciphertext || tag  (armazenado como vault_key no TOML)
```

Funcoes em `vault/crypto.rs`:
- `generate_salt()` -> 16 bytes aleatorios
- `derive_key(password: &str, salt: &[u8])` -> 32 bytes (AES-256)
- `encrypt_password(password: &str, master: &str)` -> String (hex)
- `decrypt_password(encrypted: &str, master: &str)` -> String

## SSH (service.rs)

### Conexao

1. `SshService::connect(session_id, server, auth)` cria uma sessao SSH via russh
2. Autenticacao: tenta `authenticate_publickey()` primeiro. Se falha ou Auth for Password, usa `authenticate_password()`
3. Abre canal e solicita PTY com `xterm-256color` (80x24)
4. Converte o canal em `AsyncRead + AsyncWrite` via `into_stream()`, split em reader/writer
5. Reader: tokio task que le byte por byte, converte para String, envia via mpsc para TUI
6. Writer: `Arc<Mutex<Box<dyn AsyncWrite + Send + Unpin>>>` guardado no `ShellChannel` para uso sincrono

### ShellChannel

```rust
pub struct ShellChannel {
    pub writer: Arc<Mutex<Box<dyn AsyncWrite + Send + Unpin>>>,
    pub data_rx: Arc<Mutex<mpsc::Receiver<Vec<u8>>>>,
    event_rx: Arc<Mutex<mpsc::Receiver<ShellEvent>>>,
}
```

O campo `data_rx` e publico — a TUI consome os dados recebidos do PTY atraves deste canal. O `writer` e usado para enviar teclas do usuario para o shell remoto.

## SFTP (service.rs)

### SftpServiceSession

- Abre canal SSH e solicita subsistema `"sftp"`
- Cria `russh_sftp::client::SftpSession` a partir do stream do canal
- Operacoes nativas: `read_dir`, `metadata`, `try_exists`, `open_with_flags`, `create_dir`, `remove_file`, `remove_dir`, `rename`

### Upload

```rust
let data = tokio::fs::read(local_path).await?;
let mut file = sftp.open_with_flags(remote_path,
    OpenFlags::CREATE | OpenFlags::WRITE | OpenFlags::TRUNCATE).await?;
file.write_all(&data).await?;
// Importante: OpenFlags::CREATE | WRITE | TRUNCATE — nao apenas WRITE
```

### Download

```rust
let data = sftp.read(remote_path).await?;
tokio::fs::write(local_path, &data).await?;
```

## Configuracao (TOML)

Arquivo: `~/.config/lazyssh/servers.toml`

- Structs serde com `#[serde(deny_unknown_fields)]`
- Auth e tagged enum: `#[serde(tag = "type")]` — `"key"` ou `"password"`
- Backup automatico: antes de `save()`, copia `servers.toml` -> `servers.toml.backup`
- Operacoes: load, save, add, remove, update, find por nome

## TUI (ratatui)

### Views

A TUI tem tres estados principais, controlados por `CurrentView`:

```
ServerList ──Enter──> SshTerminal
    │                      │
    │ s                    │ Ctrl+Q / Esc / exit
    v                      v
SftpBrowser          ServerList
```

### Render dispatch

```rust
terminal.draw(|f| {
    match app.current_view {
        CurrentView::ServerList => render_server_list(f, &mut app),
        CurrentView::SshTerminal => render_ssh_terminal(f, &mut app),
        CurrentView::SftpBrowser => render_sftp_browser(f, &mut app),
    }
    render_notifications(f, &app.notifications, f.area());
});
```

### SshTerminal render

O render do terminal SSH e o mais complexo. As linhas do buffer `output` sao processadas por `parse_ansi_spans()` que retorna `Vec<(String, Style)>`. Cada `(texto, estilo)` vira um `Span` colorido dentro de um `Line` do Paragraph.

Se ha selecao de texto ativa, o estilo ANSI e sobrescrito com fundo branco/azul nos indices selecionados.

### Scroll

A navegacao usa `scroll_offset` aplicado ao slice do `output`:
```rust
let start = ssh.output.len().saturating_sub(ssh.scroll_offset + usable_height);
let visible = &ssh.output[start..];
```

### Mouse selection

A posicao do mouse e mapeada para o indice no buffer considerando padding e scroll:
```rust
let output_idx = first_visible_line + (content_row - padding_top);
```
