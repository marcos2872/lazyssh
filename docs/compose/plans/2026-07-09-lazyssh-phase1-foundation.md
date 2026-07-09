# LazySSH — Fase 1: Fundação

**Spec:** `docs/compose/specs/2026-07-09-lazyssh-feature-roadmap.md` §S7.1-S7.3, §S5.2, §S4.2, §S6.1, §S4.1, §S5.3

---

## Tarefa 1.1 — Modal de Ajuda `?` (§S7.1)

**Objetivo:** Modal centralizado com dicionário de teclas da view atual.

### Arquivos
- `src/tui/help.rs` — novo módulo de renderização
- `src/tui/mod.rs` — re-export
- `src/tui/app.rs` — estado `help_visible: bool`
- `src/main.rs` — handler de `?`

### Implementação
1. Criar `src/tui/help.rs` com função `render_help_modal(f: &mut Frame, view: &CurrentView)`
2. Criar enum `HelpEntry { key: String, desc: String }`
3. Função `help_entries_for_view(view: &CurrentView) -> Vec<HelpEntry>` retorna atalhos da view
4. Layout do modal:
   - Título: `── Atalhos — <nome da view> ──`
   - Duas colunas: tecla (largura fixa 16) + descrição
   - Fundo preto semi-transparente (overlay)
   - Borda: `Theme::modal_border_style()`
   - Centralizado na tela, largura 50, altura dinâmica
5. Adicionar `help_visible: bool` ao App (default false)
6. Handler de `?` em qualquer view: toggle `help_visible`
7. Quando modal aberto: capturar `j`/`k` para scroll, `Esc`/`?` para fechar
8. Conteúdo exato das teclas está no spec §S7.1

### Testes
- Teste de `help_entries_for_view` retorna entradas corretas para cada view
- Teste de toggle help_visible

---

## Tarefa 1.2 — Footer Contextual (§S7.2)

**Objetivo:** Barra inferior com 3-4 atalhos relevantes da view atual.

### Arquivos
- `src/tui/help.rs` — função de footer
- `src/main.rs` — integrar footer na renderização

### Implementação
1. Criar função `footer_hint_for_view(view: &CurrentView) -> String` em `help.rs`
2. Retornar string compacta por view:
   - ServerList: `j/k:Navegar  Enter:Conectar  a:Novo  ?:Ajuda`
   - SSH: `Ctrl+Q:Sair  Ctrl+R:Histórico  ?:Ajuda`
   - SFTP: `Tab:Alternar  u:Upload  d:Download  ?:Ajuda`
3. Renderizar na status bar (já existe §S4.1), à esquerda
4. Cor: `Theme::dim_style()` para não competir com conteúdo
5. Atualizar dinamicamente ao trocar de view

### Testes
- Teste de retorno correto para cada view

---

## Tarefa 1.3 — Tooltips Inline em Modais (§S7.3)

**Objetivo:** Mostrar teclas disponíveis no rodapé de cada modal.

### Arquivos
- `src/main.rs` — adicionar linha de hints nos modais Insert/Edit/Confirm

### Implementação
1. No modal Insert, adicionar linha inferior: `Tab:Próximo  ↑:Anterior  Enter:Salvar  Esc:Cancelar`
2. No modal Edit: mesma linha
3. No modal Confirm: `Enter:Confirmar  Esc:Cancelar`
4. Formato: 1 linha, cor `Theme::dim_style()`, centralizada abaixo do modal
5. Espaçamento: 1 linha em branco entre conteúdo do modal e hints

### Testes
- Verificar que cada modal renderiza a linha de hints

---

## Tarefa 1.4 — Tags Visuais (§S5.2)

**Objetivo:** Renderizar as tags que já existem no model `Server` como badges coloridos na lista.

### Arquivos
- `src/tui/server_list.rs` — renderização
- `src/tui/theme.rs` — cores derivadas de hash

### Implementação
1. Criar função `tag_color(name: &str) -> Color` que gera cor consistente a partir do hash do nome
2. Em `render_server_list()`, após renderizar nome do servidor, adicionar spans de tags
3. Formato: `[tag1] [tag2]` com fundo colorido e texto contraste
4. Máximo 5 tags visíveis — excedentes mostram `+N`

### Testes
- Unit test para `tag_color()` retornar cores consistentes
- Unit test para truncamento de tags (>5)

---

## Tarefa 1.5 — Confirmação ao Deletar (§S4.2)

**Objetivo:** Adicionar modal de confirmação antes de remover servidor.

### Arquivos
- `src/tui/app.rs` — novo `ConfirmState`
- `src/main.rs` — handler de `d` e renderização do modal

### Implementação
1. Criar enum `ConfirmAction` com variante `DeleteServer { name: String }`
2. Criar struct `ConfirmState { action: ConfirmAction, server_name: String }`
3. Adicionar `confirm_state: Option<ConfirmState>` no `App`
4. Alterar handler de `d`: em vez de deletar direto, abrir `confirm_state`
5. Renderizar modal de confirmação: "Remover 'nome'?" com Enter/ Esc
6. Enter no modal → executar deleção, Esc → cancelar
7. Incluir tooltips inline (§S7.3): `Enter:Confirmar  Esc:Cancelar`

### Testes
- Teste de state transition: `d` → confirm_state Some → Enter → None + server removido
- Teste de cancelamento: `d` → confirm_state Some → Esc → None + server mantido

---

## Tarefa 1.6 — Clipboard Melhorado (§S6.1)

**Objetivo:** Copiar hostname e user@host:port com atalhos `y` e `Y`.

### Arquivos
- `src/main.rs` — handlers de `y` e `Y`
- `src/tui/ssh_terminal.rs` — reutilizar `copy_selection_to_clipboard()`

### Implementação
1. Extrair lógica de clipboard para função `copy_to_clipboard(text: &str) -> bool`
2. Handler de `y`: copiar `server.host`
3. Handler de `Y`: copiar `format!("{}@{}:{}", server.user, server.host, server.port)`
4. Notificação toast confirmando cópia: "Copiado: hostname"

### Testes
- Teste unitário de formatação da string copiada

---

## Tarefa 1.7 — Status Bar (§S4.1)

**Objetivo:** Barra inferior com informações contextuais e footer contextual (§S7.2).

### Arquivos
- `src/tui/server_list.rs` — adicionar status bar na renderização
- `src/tui/sftp_browser.rs` — status bar no SFTP
- `src/tui/ssh_terminal.rs` — status bar no SSH

### Implementação
1. Criar função `render_status_bar(f: &mut Frame, area: Rect, app: &App)` em módulo dedicado ou `help.rs`
2. Layout: 3 linhas na parte inferior
3. Linha 1 (esquerda): nome do servidor conectado ou "LazySSH v0.1.0"
4. Linha 1 (centro): view ativa (ServerList / SSH / SFTP)
5. Linha 1 (direita): hora atual + uptime
6. Linha 2: footer contextual (§S7.2) — atalhos da view atual
7. Linha 3: indicador de ordenação (se ativo)
8. Fundo: `Theme::background()` com borda superior

### Testes
- Teste de formatação de uptime
- Teste de footer por view

---

## Tarefa 1.8 — Ordenação (§S5.3)

**Objetivo:** Ordenar servidores por nome, porta, último acesso ou frequência.

### Arquivos
- `src/config/models.rs` — campos novos no Server e AppConfig
- `src/tui/app.rs` — método `sort_servers()`
- `src/main.rs` — handler de `O`
- `src/tui/server_list.rs` — indicador de ordenação na UI

### Implementação
1. Adicionar campos ao Server: `last_connected: Option<String>`, `connection_count: u32` (serde default)
2. Adicionar `sort_by: Option<String>` ao AppConfig (serde default)
3. Criar método `App::sort_servers()` que ordena conforme `sort_by`
4. Handler de `O`: ciclo entre Nome/Porta/Último acesso/Frequência
5. Mostrar indicador de ordenação na status bar: "Ordenado: Nome ↑"
6. Pinned servers sempre no topo

### Testes
- Teste de ordenação por cada critério
- Teste de pinned sempre no topo
- Teste de default (None) mantém ordem original
