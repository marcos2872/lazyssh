# LazySSH — Fase 4: Terminal Avançado

**Spec:** `docs/compose/specs/2026-07-09-lazyssh-feature-roadmap.md` §S3.1-S3.3

---

## Tarefa 4.1 — Múltiplas Abas de Sessão (§S3.1)

**Objetivo:** Suportar múltiplas sessões SSH simultâneas com abas.

### Arquivos
- `src/tui/app.rs` — reestruturar estado SSH
- `src/main.rs` — handlers de Tab/Shift+Tab/Ctrl+W, renderização
- `src/tui/ssh_terminal.rs` — abas visuais

### Implementação
1. Criar struct `SshTab`:
   ```rust
   pub struct SshTab {
       pub state: SshTerminalState,
       pub output_rx: Option<mpsc::UnboundedReceiver<String>>,
       pub label: String,
   }
   ```
2. No App, substituir `ssh_state: Option<SshTerminalState>` e `ssh_output_rx` por:
   - `ssh_tabs: Vec<SshTab>`
   - `active_ssh_tab: usize`
3. Handlers:
   - `Enter` na ServerList: abre nova aba (máx 8)
   - `Tab`: próxima aba
   - `Shift+Tab` (ou `Ctrl+Tab`): aba anterior
   - `Ctrl+W`: fecha aba ativa (mantém outras)
4. Barra de abas no topo do SSH view:
   - `[1: server1] [2: server2] [3: server3]`
   - Aba ativa com cor de destaque
   - Fechar ícone `×` quando hover
5. Cada aba tem seu próprio `mpsc::Receiver` — drain na loop principal
6. Limitar a 8 abas — notificação se exceder

### Testes
- Teste de criação/remoção de abas
- Teste de navegação Tab/Shift+Tab
- Teste de limite de abas

---

## Tarefa 4.2 — Log de Sessão (§S3.2)

**Objetivo:** Gravar output do terminal em arquivo para referência futura.

### Arquivos
- `src/tui/app.rs` — estado de log
- `src/main.rs` — handler de `L`, gravação
- `src/config/models.rs` — campos no Server

### Implementação
1. Adicionar ao Server: `log_enabled: bool` (serde default false)
2. Criar diretório `~/.local/share/lazyssh/logs/` se não existir
3. Por sessão SSH ativa, manter `Option<File>` para log
4. Handler de `L` no SSH: toggle `log_enabled` na aba atual
   - Ao ativar: criar arquivo `<server>_<YYYYMMDD_HHMMSS>.log`
   - Ao desativar: fechar arquivo, notificação "Log salvo: <path>"
5. Gravar todo output recebido do PTY no arquivo
6. A cada 5 minutos de inatividade, escrever marker temporal
7. Mostrar indicador `[L]` na status bar quando log está ativo

### Testes
- Teste de criação do arquivo de log
- Teste de toggle on/off
- Teste de formatação do marker temporal

---

## Tarefa 4.3 — Histórico de Comandos (§S3.3)

**Objetivo:** Registrar comandos executados e permitir busca/reexecução.

### Arquivos
- `src/tui/app.rs` — estado de histórico
- `src/main.rs` — handler de Ctrl+R, captura de comandos
- `src/tui/history.rs` — novo módulo de renderização

### Implementação
1. Adicionar ao App: `command_history: Vec<CommandEntry>`
   ```rust
   pub struct CommandEntry {
       pub command: String,
       pub timestamp: String,
       pub server_name: String,
   }
   ```
2. Em `key_event_to_bytes()`, interceptar linhas completas (após Enter):
   - Bufferizar caracteres até `\r` ou `\n`
   - Ao completar linha, adicionar ao `command_history`
3. Handler de `Ctrl+R` no SSH:
   - Abre modal de busca sobre o terminal
   - Input para fuzzy search no histórico
   - Lista de resultados com scroll
   - Selecionar → envia comando ao terminal
   - Esc → fecha modal
4. Persistir histórico em `~/.local/share/lazyssh/history.toml`
5. Limite de 500 entradas (FIFO)

### Testes
- Teste de captura de comandos
- Teste de fuzzy search no histórico
- Teste de persistência e loading
- Teste de limite de 500 entradas
