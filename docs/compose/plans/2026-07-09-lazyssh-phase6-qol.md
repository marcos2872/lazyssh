# LazySSH — Fase 6: Qualidade de Vida

**Spec:** `docs/compose/specs/2026-07-09-lazyssh-feature-roadmap.md` §S6.2-S6.4

---

## Tarefa 6.1 — Exportar/Importar Config (§S6.2)

**Objetivo:** Backup e compartilhamento de configurações.

### Arquivos
- `src/config/file.rs` — funções de export/import
- `src/main.rs` — handlers de Ctrl+E e Ctrl+I

### Implementação
1. Função `export_config(config, path) -> Result<()>`:
   - Serializa `AppConfig` para TOML
   - Salva em `<path>` ou `~/lazyssh-export-<YYYYMMDD>.toml`
   - Inclui timestamp no header do TOML: `# Exported by LazySSH on 2026-07-09 14:30:00`
2. Função `import_config(path) -> Result<ImportResult>`:
   - Parseia TOML do arquivo
   - `ImportResult { added: usize, skipped: usize, errors: Vec<String> }`
   - Deduplica por `host:port:user` contra config existente
   - Não sobrescreve servidores existentes (mantém versão local)
3. Handler de `Ctrl+E` (ServerList):
   - Exporta para `~/lazyssh-export-<YYYYMMDD>.toml`
   - Notificação: "Exportado para <path>"
4. Handler de `Ctrl+I` (ServerList):
   - Importa de arquivo (precisa de path via input)
   - Mini-input para caminho do arquivo
   - Notificação: "Importados X, Y ignorados"
5. Validação: arquivo deve ser TOML válido com `servers` array

### Testes
- Teste de export round-trip (export → import → same data)
- Teste de deduplicação na importação
- Teste de TOML inválido na importação
- Teste de arquivo inexistente

---

## Tarefa 6.2 — Health Check Periódico (§S6.3)

**Objetivo:** Monitorar status online/offline dos servidores em background.

### Arquivos
- `src/ssh/service.rs` — health check task
- `src/tui/app.rs` — estado de health check
- `src/tui/server_list.rs` — indicadores visuais
- `src/main.rs` — handler de `H`, integração com event loop

### Implementação
1. Criar struct `HealthStatus`:
   ```rust
   pub struct HealthStatus {
       pub online: Option<bool>,  // None = nunca checado
       pub last_check: Option<String>,
   }
   ```
2. Campo `health_statuses: HashMap<String, HealthStatus>` no App
   - Key: `host:port` do servidor
3. Background task com `tokio::spawn`:
   - A cada 60 segundos, testa conectividade de todos os servidores
   - Usa `TcpStream::connect_timeout` com 3s de timeout
   - Atualiza `health_statuses` via `Arc<Mutex>`
   - Não bloqueia a UI principal
4. Handler de `H`: toggle `health_check_enabled: bool`
5. Indicadores visuais na ServerList:
   - 🟢 verde = online
   - 🔴 vermelho = offline
   - ⚪ cinza = nunca checado
   - Posicionado após o ícone de OS (se existir)
6. Notificação ao mudar status: "Servidor 'X' ficou offline"

### Testes
- Teste de detecção online/offline
- Teste de cache de status
- Teste de toggle health check
- Teste de background task (não bloqueia)

---

## Tarefa 6.3 — Comandos Customizados (§S6.4)

**Objetivo:** Salvar e executar comandos frequentes por servidor.

### Arquivos
- `src/config/models.rs` — `CustomCommand` e campo no Server
- `src/tui/app.rs` — estado de comandos customizados
- `src/tui/ssh_terminal.rs` — UI de seleção de comando
- `src/main.rs` — handler de `x` (executar)

### Implementação
1. Criar struct `CustomCommand`:
   ```rust
   pub struct CustomCommand {
       pub name: String,
       pub command: String,
   }
   ```
2. Adicionar `custom_commands: Vec<CustomCommand>` ao Server (serde default)
3. Handler de `x` no SSH terminal:
   - Abre modal com lista de comandos do servidor atual
   - Selecionar → envia comando ao terminal + Enter
   - `n` para novo comando (mini-input nome + comando)
   - `e` para editar comando selecionado
   - `d` para deletar comando
   - Esc fecha modal
4. No modal de Edit do servidor:
   - Seção "Comandos Customizados" abaixo dos campos de auth
   - Lista de comandos com editar/remover
   - Botão "Adicionar comando"
5. Enviar comando ao terminal via shell_writer (não ao TUI)

### Testes
- Teste de serialização de CustomCommand
- Teste de envio de comando ao terminal
- Teste de CRUD de comandos
- Teste de persistência no TOML
