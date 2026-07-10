# LazySSH — Roadmap de Features

**Versão atual:** 0.1.0
**Features implementadas:** 25+

---

## ✅ Fase 1 — Foundation (Completa)

| # | Feature | Status |
|---|---------|--------|
| 1.1 | Modal de ajuda (`?`) | ✅ |
| 1.2 | Footer contextual | ✅ |
| 1.3 | Tooltips/labels nos modais | ✅ |
| 1.4 | Tags visuais na lista | ✅ |
| 1.5 | Confirmação ao deletar | ✅ |
| 1.6 | Clipboard (Wayland/X11/arboard) | ✅ |
| 1.7 | Status bar (uptime, hora) | ✅ |
| 1.8 | Ordenação de servidores | ✅ |

## ✅ Fase 2 — SFTP (Completa)

| # | Feature | Status |
|---|---------|--------|
| 2.1 | Barra de progresso visual | ✅ (暂時 removida) |
| 2.2 | Feedback visual de transferência | ✅ |
| 2.3 | Operações de filesystem (mkdir, rename, remove) | ✅ |
| 2.4 | Permissões (chmod) | ✅ |
| 2.5 | Bookmarks de diretórios | ✅ |

## ✅ Fase 3 — Conexão Inteligente (Completa)

| # | Feature | Status |
|---|---------|--------|
| 3.1 | Import SSH config | ✅ |
| 3.2 | Teste de conexão TCP | ✅ |
| 3.3 | SSH Agent Forwarding (`-A`) | ✅ |
| 3.4 | ProxyJump (`-J`) | ✅ |

## ✅ Fase 4 — Terminal Avançado (Parcial)

| # | Feature | Status |
|---|---------|--------|
| 4.1 | Múltiplas abas SSH | ❌ Removida (shell externo) |
| 4.2 | Log de sessão | ❌ Removida |
| 4.3 | Histórico de comandos | ❌ Removida |

## ✅ Fase 5 — Segurança (Completa)

| # | Feature | Status |
|---|---------|--------|
| 5.1 | Keyring OS para senhas | ✅ |
| 5.2 | Fallback plaintext TOML | ✅ |
| 5.3 | Auth como toggle nos modais | ✅ |
| 5.4 | Tags nos modais de add/edit | ✅ |

---

## 🔜 Features Futuras (Prioridade)

### Alta Prioridade

| Feature | Descrição |
|---------|-----------|
| **Barra de progresso real** | Mostrar progresso durante upload/download via SSH |
| **Multi-seleção SFTP com drag** | Selecionar range de arquivos com Shift+Click |
| **Rename inline** | Renomear sem abrir modal (F2 como no midnight commander) |
| **Config SSH extras** | StrictHostKeyChecking, KeepAlive, Compression |

### Média Prioridade

| Feature | Descrição |
|---------|-----------|
| **Groups/Folders** | Organizar servidores em pastas/grupos |
| **Recent connections** | Lista de conexões recentes no topo |
| **Session recording** | Gravar sessões SSH em arquivo (via `script`) |
| **Jump host chain** | Suporte a múltiplos saltos `-J host1,host2` |
| **Port forwarding** |隧道 `-L` / `-R` com UI dedicada |

### Baixa Prioridade

| Feature | Descrição |
|---------|-----------|
| **Sync config** | Sincronizar config entre máquinas (git/crypt) |
| **Theme customizável** | Cores editáveis pelo usuário |
| **Mouse improvements** | Double-click para conectar, context menu |
| **SFTP search** | Busca fuzzy em arquivos remotos |
| **Batch operations** | Operações em lote (chmod, rename com pattern) |

---

## Decisões de Design

### SSH: Shell Externo vs TUI Interno

**Decisão:** SSH abre em shell externo (native handoff).

**Razão:** Shell interno no TUI (via russh PTY) apresentou bugs com o PTY remoto. Shell externo é mais estável e suporta todos os recursos do SSH do sistema.

**Trade-off:** Não é possível ter múltiplas sessões SSH simultâneas na TUI. Cada `Enter` abre uma nova janela de terminal.

### Upload/Download: SSH Pipe vs SFTP

**Decisão:** Upload/download usa `cat | ssh` em vez de SFTP.

**Razão:** SFTP tem limite de ~1GB no buffer do servidor. SSH pipe não tem limite.

**Trade-off:** Não há progresso real durante a transferência (apenas timer).

### Senhas: Keyring vs Vault Crypto

**Decisão:** Keyring do OS com fallback plaintext.

**Razão:** Keyring é a solução padrão de password managers. Funciona em todas as plataformas (Linux, macOS). Vault crypto existe mas não está integrado.

**Trade-off:** Se o keyring quebra, o usuário perde as senhas salvas (vault_key fica vazio no TOML).
