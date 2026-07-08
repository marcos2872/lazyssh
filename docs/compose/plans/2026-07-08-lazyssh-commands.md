# LazySSH - Implementação dos Comandos Pendentes

> **For agentic workers:** REQUIRED SUB-SKILL: Use compose:subagent (recommended) or compose:execute to implement this plan task-by-task.

**Goal:** Implementar os comandos de TUI pendentes: `a` (adicionar), `e` (editar), `d` (deletar), `p` (pin/despin).

**Architecture:** Adicionar handlers de teclado no TUI main loop + views de formulário para add/edit.

**Tech Stack:** Rust, Ratatui, crossterm

---

### Task 13: Comando `p` - Pin/Unpin

**Covers:** [S6]

**Files:**
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `App` from Task 7

- [ ] **Step 1: Adicionar handler para `p`**

Adicionar no match `InputMode::Normal` em `src/main.rs`:

```rust
KeyCode::Char('p') => {
    if let Some(server) = app.selected_server_mut() {
        server.pinned = !server.pinned;
    }
}
```

- [ ] **Step 2: Adicionar método `selected_server_mut` em App**

Em `src/tui/app.rs`:

```rust
pub fn selected_server_mut(&mut self) -> Option<&mut Server> {
    self.filtered_indices.get(self.selected)
        .and_then(|&i| self.servers.get_mut(i))
}
```

- [ ] **Step 3: Verificar compilação**

Run: `cargo build`

- [ ] **Step 4: Commit**

```bash
git add src/main.rs src/tui/app.rs
git commit -m "feat: add pin/unpin command"
```

---

### Task 14: Comando `d` - Deletar Servidor

**Covers:** [S6]

**Files:**
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `App` from Task 7

- [ ] **Step 1: Adicionar handler para `d`**

Adicionar no match `InputMode::Normal` em `src/main.rs`:

```rust
KeyCode::Char('d') => {
    if let Some(server) = app.selected_server() {
        let name = server.name.clone();
        app.servers.retain(|s| s.name != name);
        app.filter(&app.input.clone());
        // Salvar config
        let _ = config::save_config(
            &config::AppConfig { servers: app.servers.clone() },
            &config::get_config_path()
        );
    }
}
```

- [ ] **Step 2: Verificar compilação**

Run: `cargo build`

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: add delete server command"
```

---

### Task 15: Comando `a` - Adicionar Servidor

**Covers:** [S6]

**Files:**
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `App` from Task 7

- [ ] **Step 1: Adicionar handler para `a`**

Adicionar no match `InputMode::Normal` em `src/main.rs`:

```rust
KeyCode::Char('a') => {
    app.input_mode = tui::app::InputMode::Insert;
    app.input.clear();
}
```

- [ ] **Step 2: Adicionar handler para InputMode::Insert**

Adicionar novo match para `InputMode::Insert`:

```rust
tui::app::InputMode::Insert => match key.code {
    KeyCode::Esc => {
        app.input_mode = tui::app::InputMode::Normal;
        app.input.clear();
    }
    KeyCode::Char(c) => {
        app.input.push(c);
    }
    KeyCode::Backspace => {
        app.input.pop();
    }
    KeyCode::Enter => {
        // Criar servidor com nome do input
        let name = app.input.clone();
        if !name.is_empty() {
            let server = config::Server {
                name: name.clone(),
                host: "localhost".to_string(),
                port: 22,
                user: "root".to_string(),
                auth: config::Auth::Key {
                    path: "~/.ssh/id_rsa".to_string(),
                    passphrase: None,
                },
                tags: vec![],
                pinned: false,
            };
            app.servers.push(server);
            app.filter(&app.input.clone());
            let _ = config::save_config(
                &config::AppConfig { servers: app.servers.clone() },
                &config::get_config_path()
            );
        }
        app.input_mode = tui::app::InputMode::Normal;
        app.input.clear();
    }
    _ => {}
},
```

- [ ] **Step 3: Adicionar renderização para Insert mode**

Adicionar no `terminal.draw()`:

```rust
tui::app::CurrentView::ServerList => {
    render_server_list(f, &app);
    if app.input_mode == tui::app::InputMode::Insert {
        // Overlay com input de nome
        let area = f.area();
        let popup = ratatui::widgets::Block::default()
            .borders(ratatui::widgets::Borders::ALL)
            .title("Novo Servidor (nome)");
        let input = ratatui::widgets::Paragraph::new(app.input.as_str())
            .block(popup);
        let rect = ratatui::layout::Rect::new(
            area.width / 4,
            area.height / 2 - 2,
            area.width / 2,
            3,
        );
        f.render_widget(input, rect);
    }
}
```

- [ ] **Step 4: Verificar compilação**

Run: `cargo build`

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: add server command with popup input"
```

---

### Task 16: Comando `e` - Editar Servidor

**Covers:** [S6]

**Files:**
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `App` from Task 7

- [ ] **Step 1: Adicionar handler para `e`**

Adicionar no match `InputMode::Normal` em `src/main.rs`:

```rust
KeyCode::Char('e') => {
    if let Some(server) = app.selected_server() {
        app.input = server.name.clone();
        app.input_mode = tui::app::InputMode::Edit;
    }
}
```

- [ ] **Step 2: Adicionar handler para InputMode::Edit**

Adicionar novo match para `InputMode::Edit`:

```rust
tui::app::InputMode::Edit => match key.code {
    KeyCode::Esc => {
        app.input_mode = tui::app::InputMode::Normal;
        app.input.clear();
    }
    KeyCode::Char(c) => {
        app.input.push(c);
    }
    KeyCode::Backspace => {
        app.input.pop();
    }
    KeyCode::Enter => {
        let new_name = app.input.clone();
        if !new_name.is_empty() {
            if let Some(server) = app.selected_server_mut() {
                server.name = new_name;
                let _ = config::save_config(
                    &config::AppConfig { servers: app.servers.clone() },
                    &config::get_config_path()
                );
            }
        }
        app.input_mode = tui::app::InputMode::Normal;
        app.input.clear();
    }
    _ => {}
},
```

- [ ] **Step 3: Verificar compilação**

Run: `cargo build`

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: edit server command"
```

---

## Summary

4 tasks implementing:
- `p` - Pin/unpin server
- `d` - Delete server
- `a` - Add server with popup input
- `e` - Edit server name
