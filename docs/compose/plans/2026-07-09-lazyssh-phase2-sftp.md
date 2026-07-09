# LazySSH — Fase 2: SFTP Completo

**Spec:** `docs/compose/specs/2026-07-09-lazyssh-feature-roadmap.md` §S2.1-S2.5

---

## Tarefa 2.1 — Barra de Progresso Visual (§S2.1)

**Objetivo:** Renderizar barra de progresso animada durante transferências SFTP.

### Arquivos
- `src/tui/sftp_browser.rs` — renderização da barra
- `src/tui/theme.rs` — cores de progresso

### Implementação
1. Em `render_sftp_browser()`, na área inferior, renderizar barra quando `transfer_progress` é Some
2. Layout da barra: `[████████░░░░░░░░] 52% — arquivo.txt (1.2MB / 2.3MB)`
3. Cor: verde para upload, azul para download (via `is_upload`)
4. Animação de pulse com tachyonfx durante transferência
5. Calcular ETA: `(bytes_total - bytes_done) / velocity` (velocity = bytesDone / tempoDecorrido)
6. Esconder barra quando `is_transferring` é false

### Testes
- Teste de formatação da barra (porcentagem, ETA)
- Teste de cores por direção (upload vs download)

---

## Tarefa 2.2 — Feedback Visual de Transferência (§S2.2)

**Objetivo:** Indicar visualmente quais arquivos estão sendo transferidos.

### Arquivos
- `src/tui/sftp_browser.rs` — renderização de highlights
- `src/tui/effects.rs` — efeito de shake

### Implementação
1. Highlight verde nos arquivos em `local_selected_files` / `remote_selected_files`
2. Setas de direção: `↑` para upload, `↓` para download ao lado dos arquivos selecionados
3. Notificação toast ao completar: "Transferido: 3 arquivos (4.5MB)"
4. Efeito shake no arquivo se `transfer_progress` indicar erro
5. Animação de checkmark (✓) ao completar cada arquivo individual

### Testes
- Teste de highlight de seleção
- Teste de notificação pós-transferência

---

## Tarefa 2.3 — Operações de Filesystem Remoto (§S2.3)

**Objetivo:** Criar pasta, renomear e remover arquivos no servidor remoto.

### Arquivos
- `src/sftp/service.rs` — novos métodos
- `src/tui/sftp_browser.rs` — handlers e mini-inputs
- `src/main.rs` — handlers de teclado no SFTP

### Implementação
1. Adicionar métodos ao `SftpServiceSession`:
   - `mkdir(path)` → `sftp.mkdir(path, 0o755).await`
   - `rmdir(path)` → `sftp.rmdir(path).await`
   - `unlink(path)` → `sftp.unlink(path).await`
   - `rename(old, new)` → `sftp.rename(old, new).await`
2. Adicionar ao `App`:
   - `sftp_mkdir(session_id, path) -> Result<(), String>`
   - `sftp_rmdir(session_id, path) -> Result<(), String>`
   - `sftp_unlink(session_id, path) -> Result<(), String>`
   - `sftp_rename(session_id, old, new) -> Result<(), String>`
3. Handlers no SFTP view:
   - `M` → mini-input para nome da pasta → `sftp_mkdir`
   - `x` → modal confirmação → `sftp_unlink` ou `sftp_rmdir`
   - `R` → mini-input com nome atual → `sftp_rename`
4. Mini-input: modal compacto (1 linha) sobre o painel remoto
5. Refresh automático do diretório após operação

### Testes
- Testes unitários para cada operação com mock SFTP
- Teste de refresh pós-operação

---

## Tarefa 2.4 — Permissões de Arquivo (§S2.4)

**Objetivo:** Visualizar e alterar permissões de arquivos remotos.

### Arquivos
- `src/tui/sftp_browser.rs` — coluna de permissões e modal chmod
- `src/sftp/service.rs` — método `set_permissions`

### Implementação
1. Na renderização do painel remoto, adicionar coluna de permissões:
   - Formato: `rwxr-xr-x` (9 caracteres)
   - Cores: verde para owner, azul para group, amarelo para other
2. Adicionar método `sftp_set_permissions(session_id, path, mode)` ao App
3. Atalho `m` abre modal de chmod:
   - Mostra permissão atual em octal (755) e simbólica (rwxr-xr-x)
   - Input para novo valor octal
   - Aplica via `sftp.setstat(path, FileAttr { permissions: Some(mode) })`
4. Atalho `M` (maiúsculo) para mkdir — não confundir com `m` minúsculo

### Testes
- Teste de conversão octal ↔ simbólico
- Teste de formatação da coluna de permissões

---

## Tarefa 2.5 — Bookmarks de Diretórios (§S2.5)

**Objetivo:** Salvar e navegar rapidamente para diretórios frequentes.

### Arquivos
- `src/config/models.rs` — `ServerBookmark`
- `src/tui/app.rs` — métodos de bookmark
- `src/tui/sftp_browser.rs` — UI de bookmarks
- `src/main.rs` — handlers `b` e `B`

### Implementação
1. Adicionar struct `ServerBookmark { name: String, path: String }` ao models
2. Adicionar `bookmarks: Vec<ServerBookmark>` ao `Server` (serde default)
3. Atalho `b` no SFTP: salva diretório remoto atual como bookmark
   - Abre mini-input para nome (default: nome da pasta)
   - Adiciona ao servidor atual e salva config
4. Atalho `B` no SFTP: abre lista de bookmarks do servidor
   - Selecionar bookmark navega para o path
   - `x` para remover bookmark da lista
5. Renderizar bookmarks como seção especial no topo do painel remoto

### Testes
- Teste de serialização/deserialização de bookmarks no TOML
- Teste de navegação via bookmark
