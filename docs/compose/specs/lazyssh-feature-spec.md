# LazySSH — Feature Specification

**Versão:** 0.1.0
**Última atualização:** 2026-07-09

---

## Visão Geral

LazySSH é um gerenciador SSH/SFTP em TUI (Terminal User Interface) escrito em Rust. Permite gerenciar múltiplos servidores SSH com autenticação por chave ou senha, navegação SFTP dual-pane, e transferência de arquivos via SSH.

---

## 1. ServerList — Lista de Servidores

### Campos do servidor

| Campo | Tipo | Default | Descrição |
|-------|------|---------|-----------|
| `name` | string | — | Nome de exibição |
| `host` | string | — | IP ou hostname |
| `port` | u16 | 22 | Porta SSH |
| `user` | string | root | Usuário SSH |
| `auth` | enum | — | key ou password |
| `tags` | string[] | [] | Tags para organização |
| `pinned` | bool | false | Fixar no topo |
| `last_connected` | option | null | Último acesso |
| `connection_count` | u32 | 0 | Contagem de conexões |
| `bookmarks` | object[] | [] | Bookmarks SFTP |
| `agent_forwarding` | bool | false | Flag -A no SSH |
| `proxy_jump` | option | null | Flag -J no SSH |

### Atalhos

| Tecla | Ação |
|-------|------|
| `j` / `↓` | Próximo servidor |
| `k` / `↑` | Servidor anterior |
| `Enter` | Conectar SSH (shell externo) |
| `s` | Abrir SFTP |
| `a` | Adicionar servidor (modal) |
| `e` | Editar servidor (modal) |
| `d` | Deletar servidor (com confirmação) |
| `p` | Fixar/desafixar no topo |
| `/` | Busca fuzzy |
| `i` | Importar de ~/.ssh/config |
| `t` | Testar conectividade TCP |
| `y` | Copiar hostname para clipboard |
| `Y` | Copiar user@host:port para clipboard |
| `O` | Ciclar ordenação (nome/porta/frequência) |
| `q` | Sair do app |
| `?` | Modal de ajuda |
| Mouse scroll | Navegar lista |
| Mouse click | Selecionar servidor |
| Mouse right-click | Conectar SSH |

### Modal de Adicionar/Editar

**Campos (ordem de navegação):**
1. Nome
2. Host
3. Porta
4. Usuário
5. Auth — toggle `[key]`/`[password]` com `←` `→` ou `Space`
6. Se Key: Caminho da chave, Passphrase
7. Se Password: Senha
8. Tags — separadas por vírgula

**Atalhos do modal:**
- `Tab`/`↓`: próximo campo
- `↑`: campo anterior
- `Enter`: salvar (no campo Tags)
- `Esc`: cancelar

### Busca Fuzzy

- Filtra servidores por nome, host ou tags
- Usa `fuzzy-matcher` (SkimMatcherV2)
- `Enter` confirma busca, `Esc` limpa filtro

### Ordenação

Cicla entre: Nenhuma → Nome → Porta → Último acesso → Frequência → Nenhuma

---

## 2. SSH — Conexão Shell

### Método: Shell Externo (Native Handoff)

A conexão SSH **não** usa a TUI interna. Em vez disso:

1. `leave_tui()` — sai do modo raw e alternate screen
2. `\x1bc` — reseta o terminal (limpa scrollback)
3. Spawns `ssh` ou `sshpass ssh` como processo externo
4. Usuário interage com o shell SSH real do sistema
5. Ao digitar `exit`, `reenter_tui()` restaura a TUI

### Comando SSH construído

| Flag | Condição |
|------|----------|
| `-p <porta>` | porta ≠ 22 |
| `-A` | agent_forwarding = true |
| `-J <host>` | proxy_jump definido |
| `-i <caminho>` | Auth=key |
| `sshpass -p <senha>` | Auth=password com vault_key não vazio |

### Autenticação

| Método | Via TUI | Via Shell |
|--------|---------|-----------|
| Chave SSH | russh `authenticate_publickey` | `ssh -i <path>` |
| Senha | russh `authenticate_password` | `sshpass -p <pw> ssh` |

### Segurança de Senhas (Keyring)

- Senhas são armazenadas no keyring do OS (GNOME Keyring, KDE Wallet, macOS Keychain)
- No TOML, `vault_key` fica vazio quando a senha está no keyring
- Fallback: se keyring indisponível, senha fica em plaintext no TOML
- Account key: `lazyssh` service, `<user>@<host>:<port>` account

---

## 3. SFTP — Navegador de Arquivos

### Layout

Dual-pane:
- **Esquerda:** Arquivos locais
- **Direita:** Arquivos remotos
- `Tab` alterna foco entre painéis

### Navegação

| Tecla | Ação |
|-------|------|
| `Tab` | Alternar foco |
| `j` / `↓` | Próximo item |
| `k` / `↑` | Item anterior |
| `Enter` | Entrar no diretório |
| `Backspace` | Voltar ao pai |
| `Space` | Selecionar/desselecionar |
| `a` | Selecionar todos |
| `r` | Atualizar listagem |
| `q` / `Esc` | Voltar à lista de servidores |

### Transferência (via SSH)

| Tecla | Ação | Método |
|-------|------|--------|
| `u` | Upload | `cat local \| ssh user@host 'cat > remote'` |
| `d` | Download | `ssh user@host 'cat remote' > local` |

**Design:** Usa SSH direto (não SFTP) para evitar o limite de ~1GB do SFTP. TUI fica responsiva via `spawn_blocking`.

### Operações Remotas

| Tecla | Operação | Descrição |
|-------|----------|-----------|
| `M` | Mkdir | Criar diretório remoto |
| `R` | Rename | Renomear arquivo/diretório |
| `x` | Remove | Deletar arquivo/diretório |
| `m` | Chmod | Alterar permissões (octal) |
| `b` | Bookmark | Salvar diretório como bookmark |
| `B` | Bookmarks | Navegar para primeiro bookmark |

### Input Mode (Mkdir/Rename/Chmod/Bookmark)

| Tecla | Ação |
|-------|------|
| `Char` | Adicionar caractere |
| `Backspace` | Remover último caractere |
| `Enter` | Executar operação |
| `Esc` | Cancelar |

### Progresso de Transferência

- Timer MM:SS desde o início
- Mensagem "Enviando..." (verde) ou "Baixando..." (azul)
- Barra de progresso planejada (暂時 removida)

### SFTP Service

Operaçoes nativas via `russh-sftp`:
- `read_dir`, `list_dir` — listar diretório
- `create_dir` — criar diretório
- `remove_file` / `remove_dir` — remover
- `rename` — renomear
- `set_metadata` com `FileAttributes { permissions }` — chmod
- `open_with_flags(CREATE | WRITE | TRUNCATE)` + `write_all` — upload

---

## 4. Configuração

### Arquivo: `~/.config/lazyssh/servers.toml`

```toml
[[servers]]
name = "meu-servidor"
host = "192.168.1.100"
port = 22
user = "root"
tags = ["dev"]
pinned = true
agent_forwarding = false
proxy_jump = "user@bastion.com"

[servers.auth]
type = "key"
path = "~/.ssh/id_ed25519"
passphrase = null

[[servers]]
name = "producao"
host = "10.0.0.1"
port = 22
user = "admin"

[servers.auth]
type = "password"
vault_key = ""
```

### Backup

Antes de cada salvamento, `servers.toml.backup` é criado.

### Import SSH Config (`~/.ssh/config`)

- Parse manual: `Host`, `HostName`, `Port`, `User`, `IdentityFile`
- Wildcards (`*`, `?`) são ignorados
- Todos importados recebem tag `["imported"]`
- Deduplicação por `host:port:user`

---

## 5. Vault / Criptografia

### Keyring (`src/vault/keyring.rs`)

| Função | Descrição |
|--------|-----------|
| `store_password(server, password)` | Armazena no keyring do OS |
| `get_password(server)` | Recupera do keyring |
| `delete_password(server)` | Remove do keyring |
| `is_available()` | Verifica se keyring está disponível |

### Crypto (`src/vault/crypto.rs`)

AES-256-GCM + PBKDF2 (100k iterações). Funções existentes mas não integradas:
- `generate_salt()`, `derive_key()`, `encrypt_password()`, `decrypt_password()`

---

## 6. UI / Visual

### Layout

```
┌──────────────────────────────────┐
│ LazySSH v0.1.0  ServerList  HH:MM:SS │  Status bar (topo)
├──────────────────────────────────┤
│                                  │
│   (conteúdo da view)             │
│                                  │
├──────────────────────────────────┤
│ j/k:Navegar Enter:Conectar ...   │  Footer hints
└──────────────────────────────────┘
```

### Tema (`src/tui/theme.rs`)

Cores centralizadas: `Theme::primary()`, `Theme::success()`, `Theme::error()`, `Theme::warning()`, `Theme::text()`, `Theme::text_dim()`, `Theme::border_color()`, etc.

### Notificações

| Tipo | Duração | Cor |
|------|---------|-----|
| Info | 3s | accent |
| Success | 3s | verde |
| Warning | 4s | amarelo |
| Error | 5s | vermelho |

Máximo 3 visíveis, fila de até 8. Posição: canto superior direito.

### Efeitos (`src/tui/effects.rs`)

Animações com `tachyonfx`: dissolve, coalesce, fade. Usadas para abertura/fechamento de modais, notificações, seleção de servidor.

### Modal de Ajuda (`?`)

Mostra atalhos da view atual. Pressione `?` ou `Esc` para fechar.

---

## 7. Clipboard

Três métodos com fallback:
1. `wl-copy` (Wayland)
2. `xclip -selection clipboard` (X11)
3. `arboard::Clipboard` (Rust)

Seleção de texto no terminal SSH: mouse drag → cores invertidas → cópia automática.

---

## 8. ANSI Terminal

### Parsing (`parse_ansi_spans()`)

- Cores SGR: 16 cores base, bright, 256 cores (`38;5;N`)
- Atributos: bold (1), italic (3), underline (4)
- Reset: 0, 22, 23, 24
- OSC sequences: descartadas silenciosamente
- CSI não-SGR: descartados silenciosamente

### Feed (`feed_output()`)

- Detecta CSI clear: `[2J` (limpa buffer), `[J` (limpa linha)
- Processa `\r`, `\n`, `\b`

---

## 9. Arquitetura Técnica

### Stack

| Camada | Tecnologia |
|--------|-----------|
| TUI | ratatui 0.29 |
| Terminal | crossterm 0.28 |
| SSH | russh 0.50.0-beta.7 |
| SFTP | russh-sftp 2.1.2 |
| Runtime | tokio 1 |
| Crypto | ring 0.17 |
| Config | toml 0.8 + serde 1 |
| Efeitos | tachyonfx 0.25 |
| Keyring | keyring 3.x |
| Clipboard | arboard 3 |

### Views

```
ServerList ←──Enter──→ Shell Externo (SSH)
    │
    │ s
    ↓
SftpBrowser
```

### Async + Sync

TUI é sincrono dentro de `#[tokio::main]`. Operações async usam:
- `block_in_place` + `block_on` — operações curtas (SFTP)
- `spawn_blocking` — operações longas (upload/download via SSH)
- `mpsc::unbounded_channel` — comunicação entre tasks

### Dados

- Config: `~/.config/lazyssh/servers.toml`
- Keyring: OS keyring (lazyssh service)
- Crypto: `vault/crypto.rs` (disponível mas não integrado)
