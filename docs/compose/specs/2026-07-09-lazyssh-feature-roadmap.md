# LazySSH — Roadmap de Funcionalidades

**Objetivo:** Tornar o LazySSH um gerenciador SSH/SFTP completo e polido, com 25 funcionalidades novas organizadas em 6 grupos.

---

## [S1] Grupo 1 — Conexão e Autenticação

### [S1.1] Importar do `~/.ssh/config`

**Problema:** Usuários já têm servidores configurados no `~/.ssh/config` do sistema. Re-entrar tudo manualmente é repetitivo.

**Solução:**
- Função `import_from_ssh_config()` que parseia `~/.ssh/config`
- Suporta: `Host`, `HostName`, `Port`, `User`, `IdentityFile`, `ProxyJump`
- Cria servidores no TOML com `tags: ["imported"]`
- Menu/atalho `i` na ServerList para importar (não substitui, apenas adiciona novos)
- Deduplicação por `host:port:user` — não recria servidores já existentes
- Parse do arquivo nativo do OpenSSH (não dependência externa)

### [S1.2] SSH Agent Forwarding

**Problema:** Usuários com múltiplas chaves no agent precisam de `-A` no SSH.

**Solução:**
- Campo `agent_forwarding: bool` no model `Server` (serde default false)
- No `native_shell_command()`, adicionar `-A` quando ativo
- Checkbox no modal de Insert/Edit (campo toggle)
- UI mostra ícone de agent quando ativo

### [S1.3] ProxyJump (Múltiplos saltos)

**Problema:** Servidores em redes privadas precisam de bastion host.

**Solução:**
- Campo `proxy_jump: Option<String>` no model `Server` — hostname do bastion
- No `native_shell_command()`, adicionar `-J <bastion>` quando definido
- No modal de Insert/Edit, campo "Proxy Jump" (aparece após Auth)
- Validação: não permite proxy jump para o próprio servidor

### [S1.4] Teste de Conexão

**Problema:** Usuário não sabe se o servidor está acessível antes de conectar.

**Solução:**
- Atalho `t` na ServerList para testar servidor selecionado
- Executa `nc -z -w 3 <host> <port>` ou tentativa de conexão SSH com timeout de 5s
- Mostra resultado via notificação: verde (ok), vermelho (falhou), amarelo (timeout)
- Não abre sessão, apenas testa conectividade

---

## [S2] Grupo 2 — SFTP

### [S2.1] Barra de Progresso Visual

**Problema:** Transferências grandes não têm feedback visual de progresso.

**Solução:**
- `TransferProgress` já existe — precisa de renderização visual
- Barra de progresso animada no painel inferior do SFTP
- Mostra: nome do arquivo, bytes/total, porcentagem, ETA estimado
- Cor verde durante upload, azul durante download
- Animação de pulso com tachyonfx durante transferência

### [S2.2] Feedback Visual de Transferência

**Problema:** Usuário não vê claramente o que está sendo transferido.

**Solução:**
- Highlight verde nos arquivos selecionados para transferir
- Indicação visual de direção (seta para cima = upload, para baixo = download)
- Notificação toast ao completar transferência com resumo
- shake effect no arquivo se transferência falhar

### [S2.3] Operações de Filesystem Remoto

**Problema:** Não é possível criar pastas, renomear ou remover arquivos pelo SFTP.

**Solução:**
- `mkdir` — atalho `M` (maiúsculo), abre mini-input para nome da pasta
- `rm` — atalho `x`, remove arquivo/pasta selecionado (com confirmação)
- `rename` — atalho `R` (maiúsculo), abre input com nome atual para edição
- Implementar via `russh_sftp` protocol: `rename`, `mkdir`, `rmdir`, `unlink`
- Confirmação visual antes de deletar

### [S2.4] Permissões de Arquivo

**Problema:** Não é possível ver ou alterar permissões de arquivos remotos.

**Solução:**
- Mostrar permissões (`rwxr-xr-x`) na coluna de detalhes do painel remoto
- Atalho `m` para abrir modal de chmod com campos octal
- Aplicar via `russh_sftp` `setstat` com `Permissions`
- Parse de string octal para bitmask e vice-versa

### [S2.5] Bookmarks de Diretórios

**Problema:** Navegar até diretórios profundos toda vez é tedioso.

**Solução:**
- Campo `bookmarks: Vec<ServerBookmark>` no model `Server`
- `ServerBookmark { name: String, path: String }`
- Atalho `B` para abrir lista de bookmarks e navegar
- Atalho `b` para adicionar bookmark do diretório remoto atual
- Persistidos no TOML junto com o servidor

---

## [S3] Grupo 3 — Terminal

### [S3.1] Múltiplas Abas de Sessão

**Problema:** Só é possível ter uma sessão SSH ativa por vez.

**Solução:**
- `Vec<SshSession>` no `App` em vez de `Option<SshTerminalState>`
- Tab/Shift+Tab para alternar entre sessões ativas
- Indicador visual na barra superior: `[1: server1] [2: server2]`
- Fechar aba com Ctrl+W (mantém outras abas)
- Máximo de 8 abas simultâneas
- Cada aba tem seu próprio `mpsc::Receiver`

### [S3.2] Log de Sessão

**Problema:** Output do terminal se perde ao fechar a sessão.

**Solução:**
- Campo `log_enabled: bool` e `log_path: Option<String>` no model `Server`
- Gravar output em `~/.local/share/lazyssh/logs/<server>_<timestamp>.log`
- Atalho `L` para toggle log durante sessão
- Formato: texto puro com timestamp a cada 5 minutos de inatividade
- Notificação quando log é salvo

### [S3.3] Histórico de Comandos

**Problema:** Não há como ver comandos executados recentemente.

**Solução:**
- Campo `command_history: Vec<String>` no `App`
- Registrar cada comando enviado ao PTY (bufferizado por linha)
- Atalho `Ctrl+R` para abrir modal de histórico com fuzzy search
- Selecionar item reenvia o comando ao terminal
- Histórico persistido em `~/.local/share/lazyssh/history.toml`
- Limite de 500 entradas por sessão

---

## [S4] Grupo 4 — UI/UX

### [S4.1] Status Bar Inferior

**Problema:** Usuário não vê informações contextuais (servidor conectado, uptime, etc).

**Solução:**
- Barra fixa na parte inferior da tela (3 linhas)
- Mostra: nome do servidor, IP, porta, protocolo ativo
- Indicador de uptime do app (HH:MM:SS)
- Indicador de sessões ativas (SSH: 2, SFTP: 1)
- Cor de fundo diferente da área de conteúdo

### [S4.2] Confirmação ao Deletar

**Problema:** Deletar servidor com `d` não pede confirmação — perigoso.

**Solução:**
- Ao pressionar `d`, abrir modal de confirmação: "Remover 'nome'?"
- Opções: `Enter` confirma, `Esc` cancela
- Overlay escuro + modal centralizado (padrão já existe no Insert/Edit)
- Notificação de sucesso/remoção após confirmação

### [S4.3] Scroll com Mouse

**Problema:** Scroll do mouse só funciona no SSH terminal e SFTP, não na lista de servidores.

**Solução:**
- Mapear `MouseEventKind::ScrollUp/Down` no ServerList (já parcialmente implementado)
- Garantir que funciona com e sem modal aberto
- Velocidade de scroll: 3 itens por evento (consistente com SSH)
- Indicador visual de scroll quando a lista é maior que a tela

### [S4.4] Transições Animadas

**Problema:** Mudanças de tela são abruptas, sem feedback visual.

**Solução:**
- Usar tachyonfx (já no projeto) para:
  - Fade-in ao entrar no SSH terminal
  - Slide ao abrir SFTP browser
  - Shake ao receber notificação de erro
  - Pulse no servidor selecionado
- Duração: 150-300ms para transições, 500ms para efeitos
- Respeitar flag ` reduced_motion ` se o terminal não suporta

### [S4.5] Ícones por Tipo de OS

**Problema:** Todos os servidores parecem iguais, sem diferenciação visual.

**Solução:**
- Detectar OS via SSH: `uname -s` na conexão (com cache)
- Mapear ícones: 🐧 Linux, 🍎 macOS, 🔷 FreeBSD, 🌀 Windows, ❓ desconhecido
- Mostrar ícone ao lado do nome do servidor na lista
- Cache do resultado em `~/.local/share/lazyssh/os_cache.toml`
- Fallback para ícone genérico se detecção falhar

---

## [S5] Grupo 5 — Organização

### [S5.1] Grupos/Pastas

**Problema:** Lista plana de servidores fica bagunçada com muitos servidores.

**Solução:**
- Campo `group: Option<String>` no model `Server`
- Renderizar servidores agrupados com cabeçalho de grupo
- Colapsar/expandir grupos com Enter ou Space no cabeçalho
- Grupo padrão: "Sem grupo" para servidores sem definição
- Atalho `G` para mover servidor para grupo (abre seleção)

### [S5.2] Tags Visuais

**Problema:** Campo `tags` existe no model mas não é renderizado.

**Solução:**
- Renderizar tags como badges coloridos ao lado do nome
- Cores derivadas do hash do nome da tag (consistente entre sessões)
- Filtrar por tag: atalho `#` abre busca por tag
- Tags clicáveis no modal de edição (toggle)
- Máximo 5 tags visíveis por servidor

### [S5.3] Ordenação

**Problema:** Servidores estão em ordem de inserção, sem ordenação flexível.

**Solução:**
- Atalho `O` para abrir menu de ordenação
- Opções: Nome (A-Z), Porta, Último acesso, Frequência de uso
- `last_connected: Option<chrono::DateTime>` no model `Server`
- `connection_count: u32` no model `Server`
- Ordenação persistida (campo `sort_by` no AppConfig)
- Pinned servers sempre no topo (independente da ordenação)

### [S5.4] Favoritos Recentes

**Problema:** Não há atalho rápido para os servidores mais usados.

**Solução:**
- Atalho `f` para toggle "mostrar apenas favoritos"
- Favoritos = top 5 servidores por `connection_count`
- Seção separada no topo da lista: "Recentes"
- Ícone de estrela ao lado dos favoritos
- `connection_count` incrementado a cada conexão (Enter ou double-click)

---

## [S6] Grupo 6 — Qualidade de Vida

### [S6.1] Clipboard Melhorado

**Problema:** Copiar IP/hostname não é intuitivo.

**Solução:**
- Atalho `y` (yank) copia hostname do servidor selecionado
- Atalho `Y` (maiúsculo) copia `user@host:port`
- Notificação confirmando cópia
- Já usa a cadeia `wl-copy` → `xclip` → `arboard` — reutilizar

### [S6.2] Exportar/Importar Config

**Problema:** Não há como fazer backup ou compartilhar configurações.

**Solução:**
- Atalho `Ctrl+E` exporta TOML para `~/lazyssh-export-<date>.toml`
- Atalho `Ctrl+I` importa de arquivo (seleciona caminho)
- Importação merge: não sobrescreve, adiciona novos (deduplica por host:port)
- Notificação com resumo: "Importados X servidores, Y ignorados (duplicados)"

### [S6.3] Health Check Periódico

**Problema:** Usuário não sabe quais servidores estão online.

**Solução:**
- Background task que pinga todos os servidores a cada 60s
- Resultado: 🟢 online, 🔴 offline, ⚪ desconhecido (nunca checado)
- Indicador na lista ao lado de cada servidor
- Cache de status em memória (não persistido)
- Toggle com atalho `H` para ativar/desativar health check
- Timeout de 3s por servidor (não bloqueia UI)

### [S6.4] Comandos Customizados

**Problema:** Usuários executam sempre os mesmos comandos nos mesmos servidores.

**Solução:**
- Campo `custom_commands: Vec<CustomCommand>` no model `Server`
- `CustomCommand { name: String, command: String }`
- Atalho `x` (executar) abre lista de comandos do servidor
- Selecionar envia o comando ao terminal nativo (não à TUI)
- Editar comandos no modal de edição do servidor
- Persistidos no TOML

---

## Ordem de Implementação Recomendada

### Fase 1 — Fundação (melhora imediata)
1. S5.2 Tags Visuais (já tem o campo, falta renderizar)
2. S4.2 Confirmação ao Deletar
3. S6.1 Clipboard Melhorado
4. S4.1 Status Bar
5. S5.3 Ordenação

### Fase 2 — SFTP Completo
6. S2.1 Barra de Progresso
7. S2.2 Feedback Visual
8. S2.3 Operações de Filesystem
9. S2.4 Permissões
10. S2.5 Bookmarks

### Fase 3 — Conexão Inteligente
11. S1.1 Importar SSH Config
12. S1.4 Teste de Conexão
13. S1.2 Agent Forwarding
14. S1.3 ProxyJump

### Fase 4 — Terminal Avançado
15. S3.1 Múltiplas Abas
16. S3.2 Log de Sessão
17. S3.3 Histórico de Comandos

### Fase 5 — Organização e UX
18. S5.1 Grupos/Pastas
19. S5.4 Favoritos Recentes
20. S4.3 Scroll com Mouse
21. S4.4 Transições Animadas
22. S4.5 Ícones por OS

### Fase 6 — Qualidade de Vida
23. S6.2 Exportar/Importar
24. S6.3 Health Check
25. S6.4 Comandos Customizados

---

## Dependências entre Funcionalidades

```
S5.3 Ordenação → S5.4 Favoritos (precisa de connection_count)
S4.1 Status Bar → S6.3 Health Check (mostra status)
S1.1 SSH Config → S1.2/S1.3 (campos extras no parse)
S3.1 Múltiplas Abas → S3.2/S3.3 (cada aba tem seu estado)
S2.1-S2.5 SFTP → independente entre si
S4.4 Transições → pode usar tachyonfx em qualquer feature
```

## Impacto nos Models

### `Server` (src/config/models.rs)
```rust
pub struct Server {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub auth: Auth,
    pub tags: Vec<String>,
    pub pinned: bool,
    // NOVOS:
    pub group: Option<String>,
    pub agent_forwarding: bool,
    pub proxy_jump: Option<String>,
    pub bookmarks: Vec<ServerBookmark>,
    pub custom_commands: Vec<CustomCommand>,
    pub log_enabled: bool,
    pub last_connected: Option<String>,  // ISO 8601
    pub connection_count: u32,
}
```

### `AppConfig` (src/config/models.rs)
```rust
pub struct AppConfig {
    pub servers: Vec<Server>,
    pub sort_by: Option<String>,
}
```

### Novos models
```rust
pub struct ServerBookmark {
    pub name: String,
    pub path: String,
}

pub struct CustomCommand {
    pub name: String,
    pub command: String,
}
```

---

## [S7] Sistema de Ajuda — Dicionário de Teclas

### [S7.1] Modal de Ajuda (`?`)

**Problema:** Com 25+ funcionalidades, o usuário não sabe quais teclas estão disponíveis.

**Solução:**
- Atalho `?` em qualquer view abre modal de ajuda
- Modal mostra APENAS as teclas da view atual (não todas de uma vez)
- Layout: duas colunas — tecla à esquerda, descrição à direita
- Seção com cabeçalho: `── Atalhos — ServerList ──`
- Scroll com `j`/`k` ou PageUp/PageDown se a lista for longa
- Esc fecha o modal
- Não captura其他 teclas enquanto aberto (modal é overlay)

**Conteúdo por view:**

**ServerList:**
```
┌─ Atalhos — Lista de Servidores ─────────────┐
│  j/k ou ↑/↓    Navegar na lista              │
│  Enter          Conectar SSH (shell nativo)   │
│  s              Abrir SFTP                    │
│  a              Adicionar servidor            │
│  e              Editar servidor selecionado   │
│  d              Remover servidor              │
│  p              Fixar/desfixar servidor       │
│  /              Buscar servidores             │
│  i              Importar do ~/.ssh/config     │
│  y              Copiar hostname               │
│  Y              Copiar user@host:port         │
│  t              Testar conexão                │
│  O              Ordenar por...                │
│  f              Mostrar favoritos             │
│  H              Toggle health check           │
│  G              Mover para grupo              │
│  q              Sair                          │
│  ?              Esta ajuda                    │
└──────────────────────────────────────────────┘
```

**SSH Terminal:**
```
┌─ Atalhos — Terminal SSH ────────────────────┐
│  Qualquer tecla   Enviar ao terminal remoto  │
│  Ctrl+Q ou Esc    Desconectar                │
│  PageUp/PageDown  Rolar 10 linhas            │
│  Mouse scroll     Rolar 3 linhas             │
│  Ctrl+R           Buscar no histórico        │
│  L                Toggle log de sessão        │
│  x                Comandos customizados       │
│  ?                Esta ajuda                 │
└──────────────────────────────────────────────┘
```

**SFTP Browser:**
```
┌─ Atalhos — Navegador SFTP ─────────────────┐
│  Tab              Alternar painel             │
│  j/k ou ↑/↓      Navegar na lista            │
│  Enter            Entrar no diretório         │
│  Backspace        Voltar ao pai               │
│  Space            Selecionar arquivo          │
│  a                Selecionar todos            │
│  u                Upload selecionados         │
│  d                Download selecionados       │
│  r                Atualizar listagem          │
│  M                Criar pasta (mkdir)         │
│  R                Renomear                    │
│  x                Remover arquivo/pasta       │
│  m                Alterar permissões (chmod)  │
│  b                Salvar bookmark             │
│  B                Listar bookmarks            │
│  q/Esc            Voltar à lista              │
│  ?                Esta ajuda                  │
└──────────────────────────────────────────────┘
```

### [S7.2] Footer Contextual

**Problema:** O modal de ajuda é completo mas exige ação extra para ver.

**Solução:**
- Barra inferior (já existe na §S4.1) mostra 3-4 atalhos mais importantes da view atual
- Formato compacto: `a:Novo  s:SFTP  /:Buscar  ?:Ajuda`
- Cor dim para não competir com conteúdo
- Atalho `?` sempre visível no footer como lembrete
- Muda dinamicamente ao trocar de view

**ServerList footer:** `j/k:Navegar  Enter:Conectar  a:Novo  ?:Ajuda`
**SSH footer:** `Ctrl+Q:Sair  Ctrl+R:Histórico  ?:Ajuda`
**SFTP footer:** `Tab:Alternar  u:Upload  d:Download  ?:Ajuda`

### [S7.3] Tooltips Inline

**Problema:** Em modais (Insert/Edit/Confirm), o usuário não sabe quais teclas funcionam.

**Solução:**
- No rodapé de cada modal, mostrar teclas disponíveis
- Insert/Edit: `Tab:Próximo  ↑:Anterior  Enter:Salvar  Esc:Cancelar`
- Confirm: `Enter:Confirmar  Esc:Cancelar`
- Formato: linha fina abaixo do modal com cor dim

---

## Ordem de Implementação Recomendada (atualizada)

### Fase 1 — Fundação (melhora imediata)
1. S7.1 Modal de Ajuda (`?`)
2. S7.2 Footer Contextual
3. S7.3 Tooltips Inline
4. S5.2 Tags Visuais
5. S4.2 Confirmação ao Deletar
6. S6.1 Clipboard Melhorado
7. S4.1 Status Bar
8. S5.3 Ordenação

### Fase 2 — SFTP Completo
9. S2.1 Barra de Progresso
10. S2.2 Feedback Visual
11. S2.3 Operações de Filesystem
12. S2.4 Permissões
13. S2.5 Bookmarks

### Fase 3 — Conexão Inteligente
14. S1.1 Importar SSH Config
15. S1.4 Teste de Conexão
16. S1.2 Agent Forwarding
17. S1.3 ProxyJump

### Fase 4 — Terminal Avançado
18. S3.1 Múltiplas Abas
19. S3.2 Log de Sessão
20. S3.3 Histórico de Comandos

### Fase 5 — Organização e UX
21. S5.1 Grupos/Pastas
22. S5.4 Favoritos Recentes
23. S4.3 Scroll com Mouse
24. S4.4 Transições Animadas
25. S4.5 Ícones por OS

### Fase 6 — Qualidade de Vida
26. S6.2 Exportar/Importar
27. S6.3 Health Check
28. S6.4 Comandos Customizados
