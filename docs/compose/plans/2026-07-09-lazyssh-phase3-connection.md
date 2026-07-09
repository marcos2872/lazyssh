# LazySSH — Fase 3: Conexão Inteligente

**Spec:** `docs/compose/specs/2026-07-09-lazyssh-feature-roadmap.md` §S1.1-S1.4

---

## Tarefa 3.1 — Importar do ~/.ssh/config (§S1.1)

**Objetivo:** Parsear o ssh_config do sistema e importar servidores automaticamente.

### Arquivos
- `src/config/ssh_config.rs` — novo módulo de parse
- `src/config/mod.rs` — re-export
- `src/main.rs` — handler de `i`
- `src/tui/app.rs` — método `import_ssh_config()`

### Implementação
1. Criar módulo `src/config/ssh_config.rs`:
   - Função `parse_ssh_config(path: &Path) -> Vec<Server>`
   - Parser manual de `~/.ssh/config` (formato simples: Host, HostName, Port, User, IdentityFile, ProxyJump)
   - Cada bloco `Host` vira um `Server` com tags: `["imported"]`
   - HostName mapeia para `host`, Host para `name`
   - Valores default: port=22, user=current_user, auth=Key(~/.ssh/id_rsa)
2. Handler de `i` na ServerList:
   - Chama `parse_ssh_config(~/.ssh/config)`
   - Deduplica por `host:port:user` contra lista existente
   - Adiciona novos servidores
   - Notificação: "Importados X servidores, Y ignorados"
3. Tags `["imported"]` para diferenciar de servidores manuais

### Testes
- Testes de parse com arquivos SSH config mock
- Teste de deduplicação
- Teste de valores default

---

## Tarefa 3.2 — Teste de Conexão (§S1.4)

**Objetivo:** Testar conectividade do servidor antes de conectar.

### Arquivos
- `src/ssh/service.rs` — método `test_connection`
- `src/tui/app.rs` — método `test_server_connection`
- `src/main.rs` — handler de `t`

### Implementação
1. Adicionar método `SshService::test_connection(host, port, timeout_secs)`:
   - Usa `tokio::net::TcpStream::connect_timeout()` com timeout de 5s
   - Retorna `Result<(), ConnectionError>` com detalhes
2. Handler de `t` na ServerList:
   - Chama `test_connection` em background (não bloqueia UI)
   - Notificação amarela "Testando conexão..."
   - Quando completa: verde "Servidor acessível", vermelho "Não foi possível conectar"
3. Indicador visual na lista: `🟢` ou `🔴` ao lado do servidor testado (cache em memória)

### Testes
- Teste de timeout
- Teste de sucesso/falha
- Teste de cache de resultado

---

## Tarefa 3.3 — SSH Agent Forwarding (§S1.2)

**Objetivo:** Opção para habilitar `-A` no SSH.

### Arquivos
- `src/config/models.rs` — campo no Server
- `src/main.rs` — `native_shell_command()` adicionar `-A`
- `src/tui/app.rs` — modal Insert/Edit

### Implementação
1. Adicionar `agent_forwarding: bool` ao Server (serde default false)
2. Em `native_shell_command()`: se `agent_forwarding`, adicionar `-A` aos args
3. No modal de Insert/Edit:
   - Campo "Agent Forwarding" com toggle On/Off
   - Posicionado após Auth Type
4. Visual: ícone de agent ao lado do nome na lista quando ativo

### Testes
- Teste de `native_shell_command` com agent forwarding
- Teste de serialização do campo

---

## Tarefa 3.4 — ProxyJump (§S1.3)

**Objetivo:** Suporte a `-J` para saltos SSH intermediários.

### Arquivos
- `src/config/models.rs` — campo no Server
- `src/main.rs` — `native_shell_command()` adicionar `-J`
- `src/tui/app.rs` — modal Insert/Edit

### Implementação
1. Adicionar `proxy_jump: Option<String>` ao Server (serde default None)
2. Em `native_shell_command()`: se `proxy_jump` definido, adicionar `-J <value>` aos args
3. No modal de Insert/Edit:
   - Campo "Proxy Jump" (aparece após Auth Type)
   - Placeholder: "user@bastion.example.com"
   - Se vazio, campo não é salvo (None)
4. Validação: não permite proxy_jump == host do próprio servidor

### Testes
- Teste de `native_shell_command` com proxy jump
- Teste de auto-proxy (servidor não pode ser seu próprio proxy)
- Teste de serialização do campo
