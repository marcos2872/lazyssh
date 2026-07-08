# LazySSH - Implementação dos Fluxos Completos

**Goal:** Implementar todos os fluxos de uso do LazySSH com formulários completos e navegação funcional.

---

### Task 17: Formulário Completo de Adicionar Servidor

**Files:** Modify `src/main.rs`, Modify `src/tui/app.rs`

**Fluxo:** Usuário pressiona `a` → Popup com campos: nome, host, porta, usuário → Enter salva

- [ ] **Step 1:** Criar enum `InsertField` em `app.rs` para controlar qual campo está sendo editado

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum InsertField {
    Name,
    Host,
    Port,
    User,
}

pub struct InsertState {
    pub field: InsertField,
    pub name: String,
    pub host: String,
    pub port: String,
    pub user: String,
}
```

- [ ] **Step 2:** Atualizar `App` para usar `InsertState`

```rust
pub struct App {
    // ... campos existentes ...
    pub insert_state: Option<InsertState>,
}
```

- [ ] **Step 3:** Atualizar handler `a` para criar `InsertState`

- [ ] **Step 4:** Atualizar `InputMode::Insert` para navegar entre campos com Tab

- [ ] **Step 5:** Atualizar renderização para mostrar formulário com 4 campos

- [ ] **Step 6:** Commit: `feat: complete add server form with all fields`

---

### Task 18: Formulário Completo de Editar Servidor

**Files:** Modify `src/main.rs`, Modify `src/tui/app.rs`

**Fluxo:** Usuário pressiona `e` → Popup com campos preenchidos → Tab navega → Enter salva

- [ ] **Step 1:** Criar enum `EditField` similar ao InsertField

- [ ] **Step 2:** Criar `EditState` com campos do servidor selecionado

- [ ] **Step 3:** Atualizar handler `e` para criar `EditState` com dados atuais

- [ ] **Step 4:** Atualizar `InputMode::Edit` para navegar entre campos

- [ ] **Step 5:** Atualizar renderização para mostrar formulário editável

- [ ] **Step 6:** Commit: `feat: complete edit server form with all fields`

---

### Task 19: Navegação de Diretórios no SFTP

**Files:** Modify `src/main.rs`, Modify `src/sftp/local.rs`, Modify `src/tui/sftp_browser.rs`

**Fluxo:** No SFTP, Enter em diretório → entra, Backspace → volta

- [ ] **Step 1:** Adicionar handler `Enter` no SFTP para `cd` no diretório selecionado

```rust
KeyCode::Enter => {
    if let Some(sftp) = &mut app.sftp_state {
        let files = sftp.local.list().unwrap_or_default();
        if let Some(file) = files.get(sftp.local_selected) {
            if file.is_dir {
                let _ = sftp.local.cd(&file.name);
                sftp.local_selected = 0;
            }
        }
    }
}
```

- [ ] **Step 2:** Adicionar handler `Backspace` para voltar ao diretório pai

```rust
KeyCode::Backspace => {
    if let Some(sftp) = &mut app.sftp_state {
        let _ = sftp.local.cd("..");
        sftp.local_selected = 0;
    }
}
```

- [ ] **Step 3:** Commit: `feat: add directory navigation in SFTP browser`

---

### Task 20: Busca Fuzzy Real

**Files:** Modify `src/tui/app.rs`

**Fluxo:** `/` ativa busca → caracteres fazem matching fuzzy

- [ ] **Step 1:** Importar `fuzzy-matcher` no `app.rs`

```rust
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
```

- [ ] **Step 2:** Atualizar método `filter` para usar fuzzy matching

```rust
pub fn filter(&mut self, query: &str) {
    if query.is_empty() {
        self.filtered_indices = (0..self.servers.len()).collect();
    } else {
        let matcher = SkimMatcherV2::default();
        self.filtered_indices = self.servers.iter()
            .enumerate()
            .filter_map(|(i, s)| {
                let score = matcher.fuzzy_match(&s.name, query)
                    .or_else(|| matcher.fuzzy_match(&s.host, query))
                    .or_else(|| s.tags.iter().find_map(|t| matcher.fuzzy_match(t, query)));
                score.map(|_| i)
            })
            .collect();
    }
    self.selected = 0;
}
```

- [ ] **Step 3:** Commit: `feat: implement fuzzy search`

---

### Task 21: Conexão SSH Real

**Files:** Modify `src/main.rs`, Modify `src/ssh/connection.rs`

**Fluxo:** `Enter` seleciona servidor → abre terminal SSH integrado

- [ ] **Step 1:** Criar view `SshTerminal` com dados da conexão

- [ ] **Step 2:** Atualizar handler `Enter` para conectar e mudar view

- [ ] **Step 3:** Implementar renderização básica do terminal (output do comando)

- [ ] **Step 4:** Commit: `feat: add basic SSH terminal view`

---

### Task 22: Salvar Alterações de Pin/Unpin

**Files:** Modify `src/main.rs`

**Fluxo:** `p` alterna pin → salva no config

- [ ] **Step 1:** Adicionar persistência ao handler `p`

```rust
KeyCode::Char('p') => {
    if let Some(server) = app.selected_server_mut() {
        server.pinned = !server.pinned;
        let _ = config::save_config(
            &config::AppConfig { servers: app.servers.clone() },
            &config::get_config_path(),
        );
    }
}
```

- [ ] **Step 2:** Commit: `feat: persist pin/unpin changes to config`

---

## Summary

6 tasks implementing complete user flows:
- Task 17: Complete add server form (name, host, port, user)
- Task 18: Complete edit server form (all fields)
- Task 19: SFTP directory navigation (Enter/Backspace)
- Task 20: Real fuzzy search
- Task 21: Basic SSH terminal connection
- Task 22: Persist pin/unpin to config
