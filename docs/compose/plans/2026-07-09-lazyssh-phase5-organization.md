# LazySSH — Fase 5: Organização e UX

**Spec:** `docs/compose/specs/2026-07-09-lazyssh-feature-roadmap.md` §S5.1, §S5.4, §S4.3-S4.5

---

## Tarefa 5.1 — Grupos/Pastas (§S5.1)

**Objetivo:** Agrupar servidores por categoria (dev, staging, prod).

### Arquivos
- `src/config/models.rs` — campo `group` no Server
- `src/tui/app.rs` — lógica de agrupamento
- `src/tui/server_list.rs` — renderização com cabeçalhos de grupo
- `src/main.rs` — handler de `G`

### Implementação
1. Adicionar `group: Option<String>` ao Server (serde default None)
2. Em `render_server_list()`:
   - Agrupar servidores filtrados por `group`
   - Renderizar cabeçalho de grupo: `── Produção ──` com cor secondary
   - Servidores sem grupo: `── Sem grupo ──`
   - Cabeçalho clicável: Enter/Space colapsa/expande o grupo
3. Estado de expansão: `expanded_groups: HashSet<String>` no App
4. Handler de `G` na ServerList:
   - Abre mini-input para nome do grupo
   - Move servidor selecionado para o grupo
   - Salva config
5. Indicador visual: `▸` grupo expandido, `▹` colapsado

### Testes
- Teste de agrupamento por group
- Teste de colapso/expansão
- Teste de mover para grupo

---

## Tarefa 5.2 — Favoritos Recentes (§S5.4)

**Objetivo:** Mostrar os servidores mais usados no topo.

### Arquivos
- `src/tui/app.rs` — método de ordenação por favoritos
- `src/tui/server_list.rs` — seção de recentes
- `src/main.rs` — handler de `f`, incremento de connection_count

### Implementação
1. Campo `connection_count: u32` já adicionado na Fase 1 (§S5.3)
2. Incrementar `connection_count` a cada conexão:
   - `Enter` na ServerList (SSH)
   - `s` no SFTP
   - Double-click direito
3. Handler de `f`: toggle `show_favorites_only: bool` no App
4. Se `show_favorites_only`:
   - Filtrar apenas top 5 por `connection_count`
   - Mostrar seção "Recentes" no topo
5. Se não:
   - Mostrar seção "Recentes" como top 3 antes da lista principal
   - Separador visual: `── Recentes ──`
6. Ícone de estrela `★` ao lado dos top 5

### Testes
- Teste de incremento de connection_count
- Teste de top 5 favoritos
- Teste de toggle show_favorites_only

---

## Tarefa 5.3 — Scroll com Mouse na Lista (§S4.3)

**Objetivo:** Garantir scroll fluido com mouse na ServerList.

### Arquivos
- `src/main.rs` — handlers de mouse no ServerList

### Implementação
1. Verificar handlers existentes de `MouseEventKind::ScrollUp/Down` no ServerList
2. Garantir que funciona com modal aberto (Insert/Edit) — scroll o fundo
3. Velocidade: 3 itens por evento (consistente com SSH terminal)
4. Adicionar indicador de scroll: seta `▲` no topo / `▼` no fundo quando aplicável
5. Auto-scroll: quando navegar com teclado e item sair da área visível, scroll para manter visível

### Testes
- Teste de scroll up/down com mouse
- Teste de auto-scroll com teclado
- Teste de indicador de scroll

---

## Tarefa 5.4 — Transições Animadas (§S4.4)

**Objetivo:** Usar tachyonfx para transições suaves entre views.

### Arquivos
- `src/tui/effects.rs` — novos efeitos
- `src/main.rs` — trigger de efeitos nas mudanças de view

### Implementação
1. Efeitos a implementar:
   - **Fade-in**: ao entrar no SSH terminal (opacity 0→1 em 200ms)
   - **Slide**: ao abrir SFTP browser (desliza da esquerda)
   - **Shake**: ao receber notificação de erro (3 oscillations, 150ms)
   - **Pulse**: no servidor selecionado quando pressionado Enter
2. Integrar com `AppEffects` existente:
   - `ssh_enter_effect()` — fade-in
   - `sftp_enter_effect()` — slide
   - `error_effect()` — shake
3. Trigger no event loop:
   - Após `run_native_shell_handoff` retorna → flash effect
   - Após `app.open_sftp()` → slide effect
   - Após notificação de erro → shake effect
4. Flag `reduced_motion` para desabilitar (verificar `TERM` env)

### Testes
- Teste de criação de cada tipo de efeito
- Teste de duração dos efeitos
- Teste de reduced_motion

---

## Tarefa 5.5 — Ícones por Tipo de OS (§S4.5)

**Objetivo:** Detectar e mostrar ícone do sistema operacional do servidor.

### Arquivos
- `src/ssh/service.rs` — detecção de OS
- `src/tui/server_list.rs` — renderização de ícones
- Cache em `~/.local/share/lazyssh/os_cache.toml`

### Implementação
1. Criar função `detect_os(ssh_service, server) -> OsType`:
   - Executa `uname -s` via SSH (comandos curtos, sem PTY)
   - Timeout de 3s
   - Cacheia resultado em `HashMap<String, OsType>`
2. Enum `OsType`: Linux, MacOS, FreeBSD, Windows, Unknown
3. Mapeamento de ícones: `🐧`, `🍎`, `🔷`, `🌀`, `❓`
4. Renderizar ícone ao lado do nome na ServerList
5. Cache persistido em `~/.local/share/lazyssh/os_cache.toml`
   - Formato: `host:port = "Linux"`
   - Atualizado a cada detecção bem-sucedida
6. Detecção lazy: só detecta quando servidor é selecionado (não ao iniciar)

### Testes
- Teste de mapeamento OS → ícone
- Teste de cache hit/miss
- Teste de parse do uname output
