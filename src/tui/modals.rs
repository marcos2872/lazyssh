use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::app::{ConfirmAction, ConfirmState, FormField, FormMode, FormState};
use super::theme::Theme;

/// Renderiza o modal de formulário (inserir/editar servidor) centralizado na tela.
pub fn render_form_modal(f: &mut Frame, form: &FormState) {
    let area = f.area();
    let is_key = form.is_key_auth();
    let is_insert = matches!(form.mode, FormMode::Insert);
    let height: u16 = if is_key { 16 } else { 14 };
    let width: u16 = 50;
    let x = (area.width - width) / 2;
    let y = (area.height - height) / 2;
    let rect = Rect::new(x, y, width, height);

    let mut lines = vec![];

    let title = if is_insert { "  ➕ Novo Servidor  " } else { "  ✏️ Editar Servidor  " };
    lines.push(Line::from(vec![
        Span::styled(title,
            Style::default().fg(Theme::accent()).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from("─".repeat(width as usize - 2)));

    let base_fields = [
        (FormField::Name, "📝 Nome", &form.name),
        (FormField::Host, "🌐 Host", &form.host),
        (FormField::Port, "🔌 Porta", &form.port),
        (FormField::User, "👤 Usuário", &form.user),
    ];

    for (field_type, label, value) in &base_fields {
        let is_active = &form.field == field_type;
        let marker = if is_active { "▶" } else { " " };
        let style = if is_active {
            Style::default().fg(Theme::primary()).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::text())
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{} ", marker), Style::default().fg(Theme::accent())),
            Span::styled(format!("{}: ", label), Style::default().fg(Theme::secondary())),
            Span::styled(*value, style),
        ]));
    }

    {
        let is_active = form.field == FormField::AuthType;
        let marker = if is_active { "▶" } else { " " };
        let auth_label = if is_key { "key" } else { "password" };
        let style = if is_active {
            Style::default().fg(Theme::primary()).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::text())
        };
        let mut spans = vec![
            Span::styled(format!("{} ", marker), Style::default().fg(Theme::accent())),
            Span::styled("🔐 Auth: ", Style::default().fg(Theme::secondary())),
            Span::styled(format!("[{}]", auth_label), style),
        ];
        if is_active {
            spans.push(Span::styled(" ← →", Style::default().fg(Theme::text_dim())));
        }
        lines.push(Line::from(spans));
    }

    if is_key {
        let is_active = form.field == FormField::KeyPath;
        let marker = if is_active { "▶" } else { " " };
        let style = if is_active {
            Style::default().fg(Theme::primary()).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::text())
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{} ", marker), Style::default().fg(Theme::accent())),
            Span::styled("🔑 Chave: ", Style::default().fg(Theme::secondary())),
            Span::styled(&form.key_path, style),
        ]));

        let is_active = form.field == FormField::Passphrase;
        let marker = if is_active { "▶" } else { " " };
        let style = if is_active {
            Style::default().fg(Theme::primary()).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::text())
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{} ", marker), Style::default().fg(Theme::accent())),
            Span::styled("🔑 Senha: ", Style::default().fg(Theme::secondary())),
            Span::styled(&form.passphrase, style),
        ]));
    } else {
        let is_active = form.field == FormField::Password;
        let marker = if is_active { "▶" } else { " " };
        let style = if is_active {
            Style::default().fg(Theme::primary()).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::text())
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{} ", marker), Style::default().fg(Theme::accent())),
            Span::styled("🔑 Senha: ", Style::default().fg(Theme::secondary())),
            Span::styled(&form.password, style),
        ]));
    }

    {
        let is_active = form.field == FormField::Tags;
        let marker = if is_active { "▶" } else { " " };
        let style = if is_active {
            Style::default().fg(Theme::primary()).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::text())
        };
        let tags_display = if form.tags.is_empty() { " (nenhuma)" } else { &form.tags };
        lines.push(Line::from(vec![
            Span::styled(format!("{} ", marker), Style::default().fg(Theme::accent())),
            Span::styled("🏷️ Tags: ", Style::default().fg(Theme::secondary())),
            Span::styled(tags_display, style),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  ↑/↓/Tab: próximo campo  ", Style::default().fg(Theme::text_dim())),
        Span::styled("│  Esc: cancelar", Style::default().fg(Theme::error())),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Enter: salvar (no último campo)  ", Style::default().fg(Theme::success())),
    ]));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::modal_border_style())
        .style(Style::default().bg(Color::Black));

    let input = Paragraph::new(lines).block(block);
    f.render_widget(input, rect);
}

/// Renderiza o modal de confirmação (ex: exclusão de servidor) centralizado na tela.
pub fn render_confirm_modal(f: &mut Frame, confirm: &ConfirmState) {
    let area = f.area();
    let width = 45u16;
    let height = 5u16;
    let x = (area.width - width) / 2;
    let y = (area.height - height) / 2;
    let rect = Rect::new(x, y, width, height);

    let msg = match &confirm.action {
        ConfirmAction::DeleteServer { name } => {
            format!("Remover servidor '{}'?", name)
        }
    };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("  {}  ", msg),
                Style::default().fg(Theme::text()).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Enter:Confirmar  ", Style::default().fg(Theme::success())),
            Span::styled("│  Esc:Cancelar", Style::default().fg(Theme::error())),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" ⚠ Confirmação ")
        .title_style(Theme::modal_title_style())
        .border_style(Theme::modal_border_style())
        .style(Style::default().bg(Color::Black));

    f.render_widget(ratatui::widgets::Clear, rect);
    f.render_widget(Paragraph::new(lines).block(block), rect);
}
