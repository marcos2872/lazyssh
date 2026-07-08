# LazySSH - Refatoração UI com tachyonfx

> **For agentic workers:** REQUIRED SUB-SKILL: Use compose:subagent (recommended) or compose:execute to implement this plan task-by-task.

**Goal:** Refatorar a UI do LazySSH usando tachyonfx para modais com efeitos, animações e cores mais destacadas.

**Architecture:** Adicionar tachyonfx como dependência, criar módulo de efeitos, refatorar views para usar efeitos.

**Tech Stack:** Rust, Ratatui, tachyonfx, crossterm

---

## [S1] Visão Geral das Mudanças

### Arquivos a Criar
- `src/tui/effects.rs` - Gerenciador de efeitos
- `src/tui/theme.rs` - Tema e cores

### Arquivos a Modificar
- `Cargo.toml` - Adicionar tachyonfx
- `src/tui/mod.rs` - Exportar novos módulos
- `src/tui/app.rs` - Adicionar state de animação
- `src/tui/server_list.rs` - Efeitos na lista
- `src/tui/ssh_terminal.rs` - Efeitos no terminal
- `src/main.rs` - Integrar effect manager

---

## [S2] Task 23: Configuração do tachyonfx

**Files:**
- Modify: `Cargo.toml`
- Create: `src/tui/theme.rs`

**Steps:**

1. Adicionar tachyonfx ao Cargo.toml:
```toml
tachyonfx = "0.25"
```

2. Criar `src/tui/theme.rs`:
```rust
use ratatui::style::{Color, Modifier, Style};

pub struct Theme;

impl Theme {
    // Cores principais
    pub fn primary() -> Color { Color::Cyan }
    pub fn secondary() -> Color { Color::Blue }
    pub fn accent() -> Color { Color::Magenta }
    pub fn success() -> Color { Color::Green }
    pub fn warning() -> Color { Color::Yellow }
    pub fn error() -> Color { Color::Red }
    pub fn background() -> Color { Color::DarkGray }
    pub fn text() -> Color { Color::White }

    // Estilos
    pub fn title_style() -> Style {
        Style::default()
            .fg(Self::primary())
            .add_modifier(Modifier::BOLD)
    }

    pub fn selected_style() -> Style {
        Style::default()
            .bg(Self::primary())
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
    }

    pub fn border_style() -> Style {
        Style::default().fg(Self::secondary())
    }

    pub fn modal_border_style() -> Style {
        Style::default()
            .fg(Self::accent())
            .add_modifier(Modifier::BOLD)
    }

    pub fn input_style() -> Style {
        Style::default()
            .fg(Self::text())
            .bg(Color::Black)
    }

    pub fn input_active_style() -> Style {
        Style::default()
            .fg(Self::primary())
            .bg(Color::Black)
            .add_modifier(Modifier::BOLD)
    }
}
```

3. Commit: `feat: add tachyonfx dependency and theme module`

---

## [S3] Task 24: Gerenciador de Efeitos

**Files:**
- Create: `src/tui/effects.rs`
- Modify: `src/tui/mod.rs`

**Steps:**

1. Criar `src/tui/effects.rs`:
```rust
use std::time::Duration;
use tachyonfx::{EffectManager, fx, EffectTimer, Interpolation, CellFilter};
use ratatui::layout::Rect;
use ratatui::buffer::Buffer;

pub struct AppEffects {
    pub manager: EffectManager<()>,
}

impl AppEffects {
    pub fn new() -> Self {
        Self {
            manager: EffectManager::default(),
        }
    }

    pub fn add_modal_open_effect(&mut self, area: Rect) {
        // Efeito de fade-in para modais
        let fade = fx::fade_to(
            ratatui::style::Color::Reset,
            ratatui::style::Color::Reset,
            (200, Interpolation::QuadOut),
        );
        self.manager.add_effect(fade);

        // Efeito de dissolve para o conteúdo
        let dissolve = fx::dissolve(300);
        self.manager.add_effect(dissolve);
    }

    pub fn add_modal_close_effect(&mut self) {
        let fade = fx::fade_to(
            ratatui::style::Color::Black,
            ratatui::style::Color::Reset,
            (150, Interpolation::QuadIn),
        );
        self.manager.add_effect(fade);
    }

    pub fn add_notification_effect(&mut self) {
        let slide = fx::slide_in(
            tachyonfx::Motion::TopToBottom,
            0,
            1,
            ratatui::style::Color::Reset,
            (300, Interpolation::BackOut),
        );
        self.manager.add_effect(slide);
    }

    pub fn add_server_select_effect(&mut self) {
        let pulse = fx::fade_to(
            ratatui::style::Color::Reset,
            ratatui::style::Color::Cyan,
            (200, Interpolation::SineInOut),
        );
        self.manager.add_effect(pulse);
    }

    pub fn add_ssh_connect_effect(&mut self) {
        let sweep = fx::sweep_in(
            tachyonfx::Motion::LeftToRight,
            0,
            0,
            ratatui::style::Color::Black,
            (500, Interpolation::QuadOut),
        );
        self.manager.add_effect(sweep);
    }

    pub fn process(&mut self, elapsed: Duration, buf: &mut Buffer, area: Rect) {
        self.manager.process_effects(elapsed.into(), buf, area);
    }

    pub fn has_active_effects(&self) -> bool {
        !self.manager.is_idle()
    }
}
```

2. Atualizar `src/tui/mod.rs`:
```rust
pub mod app;
pub mod effects;
pub mod notifications;
pub mod sftp_browser;
pub mod server_list;
pub mod ssh_terminal;
pub mod theme;

pub use app::App;
pub use effects::AppEffects;
pub use notifications::{render_notifications, NotificationQueue};
pub use sftp_browser::{render_sftp_browser, Side, SftpState};
pub use server_list::render_server_list;
pub use ssh_terminal::render_ssh_terminal;
pub use theme::Theme;
```

3. Commit: `feat: add effects manager and theme module`

---

## [S4] Task 25: Refatorar Server List com Efeitos

**Files:**
- Modify: `src/tui/server_list.rs`

**Steps:**

1. Atualizar `render_server_list` para usar Theme:
```rust
use super::theme::Theme;

pub fn render_server_list(f: &mut Frame, app: &App) {
    // ... código existente ...

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Servers ")
        .title_style(Theme::title_style())
        .border_style(Theme::border_style());

    let list = List::new(items)
        .block(block)
        .highlight_style(Theme::selected_style())
        .highlight_symbol("→ ");
}
```

2. Adicionar efeito de seleção no highlight

3. Commit: `feat: refactor server list with theme and effects`

---

## [S5] Task 26: Refatorar Modais de Servidor

**Files:**
- Modify: `src/main.rs` (renderização de modais)

**Steps:**

1. Criar função para renderizar modal com efeito:
```rust
fn render_modal_with_effect(
    f: &mut Frame,
    area: Rect,
    title: &str,
    content: Vec<Line>,
    effects: &mut AppEffects,
) {
    // Calcular área do modal (centralizado)
    let modal_width = 50.min(area.width - 4);
    let modal_height = (content.len() as u16 + 4).min(area.height - 4);
    let x = (area.width - modal_width) / 2;
    let y = (area.height - modal_height) / 2;

    let modal_area = Rect::new(x, y, modal_width, modal_height);

    // Criar bloco do modal com estilo destacado
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", title))
        .title_style(Theme::modal_title_style())
        .border_style(Theme::modal_border_style())
        .style(Style::default().bg(Color::Black));

    // Renderizar com efeito de dissolve
    let paragraph = Paragraph::new(content).block(block);
    f.render_widget(paragraph, modal_area);

    // Aplicar efeito de overlay escuro nas bordas
    let overlay = Block::default()
        .style(Style::default().bg(Color::Black).add_modifier(Modifier::DIM));
    f.render_widget(overlay, area);
}
```

2. Atualizar renderização dos modais Insert e Edit

3. Commit: `feat: refactor modals with visual effects`

---

## [S6] Task 27: Refatorar Terminal SSH

**Files:**
- Modify: `src/tui/ssh_terminal.rs`

**Steps:**

1. Adicionar efeito de cursor pulsante

2. Adicionar efeito de fade-in na conexão

3. Melhorar estilo do prompt com cores do tema

4. Commit: `feat: refactor SSH terminal with effects`

---

## [S7] Task 28: Integrar Effect Manager no Main

**Files:**
- Modify: `src/main.rs`
- Modify: `src/tui/app.rs`

**Steps:**

1. Adicionar `AppEffects` ao struct `App`:
```rust
pub struct App {
    // ... campos existentes ...
    pub effects: AppEffects,
}
```

2. Atualizar loop principal para processar efeitos:
```rust
loop {
    let elapsed = last_frame.elapsed();
    last_frame = Instant::now();

    terminal.draw(|f| {
        // Renderizar conteúdo
        match app.current_view {
            // ...
        }

        // Processar efeitos
        app.effects.process(elapsed, f.buffer_mut(), f.area());
    })?;
}
```

3. Adicionar efeitos nos eventos:
- Ao abrir modal: `app.effects.add_modal_open_effect()`
- Ao selecionar servidor: `app.effects.add_server_select_effect()`
- Ao conectar SSH: `app.effects.add_ssh_connect_effect()`
- Ao mostrar notificação: `app.effects.add_notification_effect()`

4. Commit: `feat: integrate effect manager in main loop`

---

## [S8] Task 29: Efeitos de Notificação Melhorados

**Files:**
- Modify: `src/tui/notifications.rs`

**Steps:**

1. Adicionar animação de slide-in para notificações

2. Adicionar efeito de fade-out antes de desaparecer

3. Melhorar visual com cores do tema

4. Commit: `feat: enhance notification animations`

---

## Resumo

| Task | Descrição | Arquivos |
|------|-----------|----------|
| 23 | Config tachyonfx + theme | Cargo.toml, theme.rs |
| 24 | Gerenciador de efeitos | effects.rs, mod.rs |
| 25 | Server list com efeitos | server_list.rs |
| 26 | Modais de servidor | main.rs |
| 27 | Terminal SSH | ssh_terminal.rs |
| 28 | Integrar no main | main.rs, app.rs |
| 29 | Notificações animadas | notifications.rs |
