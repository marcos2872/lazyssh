use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use std::time::Instant;

use super::app::CurrentView;
use super::theme::Theme;

pub struct HelpEntry {
    pub key: String,
    pub desc: String,
}

impl HelpEntry {
    pub fn new(key: &str, desc: &str) -> Self {
        Self {
            key: key.to_string(),
            desc: desc.to_string(),
        }
    }
}

pub fn help_entries_for_view(view: &CurrentView) -> Vec<HelpEntry> {
    match view {
        CurrentView::ServerList => vec![
            HelpEntry::new("j/k ou ↑/↓", "Navegar na lista"),
            HelpEntry::new("Enter", "Conectar SSH (shell nativo)"),
            HelpEntry::new("s", "Abrir SFTP"),
            HelpEntry::new("a", "Adicionar servidor"),
            HelpEntry::new("e", "Editar servidor selecionado"),
            HelpEntry::new("d", "Remover servidor"),
            HelpEntry::new("p", "Fixar/desfixar servidor"),
            HelpEntry::new("/", "Buscar servidores"),
            HelpEntry::new("i", "Importar do ~/.ssh/config"),
            HelpEntry::new("y", "Copiar hostname"),
            HelpEntry::new("Y", "Copiar user@host:port"),
            HelpEntry::new("t", "Testar conexão"),
            HelpEntry::new("Ctrl+L", "Toggle log por servidor"),
            HelpEntry::new("O", "Ordenar por..."),
            HelpEntry::new("f", "Mostrar favoritos"),
            HelpEntry::new("q", "Sair"),
            HelpEntry::new("?", "Esta ajuda"),
        ],
        CurrentView::SshTerminal => vec![
            HelpEntry::new("Qualquer tecla", "Enviar ao terminal remoto"),
            HelpEntry::new("Ctrl+Q ou Esc", "Desconectar"),
            HelpEntry::new("PageUp/PageDown", "Rolar 10 linhas"),
            HelpEntry::new("Mouse scroll", "Rolar 3 linhas"),
            HelpEntry::new("?", "Esta ajuda"),
        ],
        CurrentView::SftpBrowser => vec![
            HelpEntry::new("Tab", "Alternar painel"),
            HelpEntry::new("j/k ou ↑/↓", "Navegar na lista"),
            HelpEntry::new("Enter", "Entrar no diretório"),
            HelpEntry::new("Backspace", "Voltar ao pai"),
            HelpEntry::new("Space", "Selecionar arquivo"),
            HelpEntry::new("a", "Selecionar todos"),
            HelpEntry::new("u", "Upload selecionados"),
            HelpEntry::new("d", "Download selecionados"),
            HelpEntry::new("r", "Atualizar listagem"),
            HelpEntry::new("M", "Criar pasta (mkdir)"),
            HelpEntry::new("R", "Renomear"),
            HelpEntry::new("x", "Remover arquivo/pasta"),
            HelpEntry::new("m", "Alterar permissões (chmod)"),
            HelpEntry::new("b", "Salvar bookmark"),
            HelpEntry::new("B", "Listar bookmarks"),
            HelpEntry::new("q/Esc", "Voltar à lista"),
            HelpEntry::new("?", "Esta ajuda"),
        ],
    }
}

pub fn footer_hint_for_view(view: &CurrentView) -> String {
    match view {
        CurrentView::ServerList => "j/k:Navegar  Enter:Conectar  a:Novo  ?:Ajuda".to_string(),
        CurrentView::SshTerminal => "Ctrl+Q:Sair  ?:Ajuda".to_string(),
        CurrentView::SftpBrowser => "Tab:Alternar  u:Upload  d:Download  ?:Ajuda".to_string(),
    }
}

pub fn render_help_modal(f: &mut Frame, view: &CurrentView) {
    let entries = help_entries_for_view(view);
    let area = f.area();

    let view_name = match view {
        CurrentView::ServerList => "Lista de Servidores",
        CurrentView::SshTerminal => "Terminal SSH",
        CurrentView::SftpBrowser => "Navegador SFTP",
    };

    let title = format!("── Atalhos — {} ──", view_name);
    let width = 50u16;
    let height = (entries.len() as u16) + 4; // title + border + padding + entries
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let modal_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, modal_area);

    let mut lines = vec![];
    for entry in &entries {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<16}", entry.key),
                Style::default()
                    .fg(Theme::primary())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(&entry.desc, Style::default().fg(Theme::text())),
        ]));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Theme::modal_title_style())
        .border_style(Theme::modal_border_style());

    let help_text = Paragraph::new(lines).block(block);
    f.render_widget(help_text, modal_area);
}

pub fn render_status_bar(f: &mut Frame, area: Rect, view: &CurrentView, start_time: Instant) {
    let view_name = match view {
        CurrentView::ServerList => "ServerList",
        CurrentView::SshTerminal => "SSH",
        CurrentView::SftpBrowser => "SFTP",
    };

    let elapsed = start_time.elapsed();
    let uptime = format!(
        "{:02}:{:02}:{:02}",
        elapsed.as_secs() / 3600,
        (elapsed.as_secs() % 3600) / 60,
        elapsed.as_secs() % 60
    );

    let now = chrono::Local::now().format("%H:%M:%S").to_string();

    let footer = footer_hint_for_view(view);

    let status_line = Line::from(vec![
        Span::styled(
            format!(" LazySSH v0.1.0 "),
            Style::default().fg(Theme::primary()).add_modifier(Modifier::BOLD),
        ),
        Span::styled("│", Style::default().fg(Theme::text_dim())),
        Span::styled(
            format!(" {} ", view_name),
            Style::default().fg(Theme::secondary()),
        ),
        Span::styled("│", Style::default().fg(Theme::text_dim())),
        Span::styled(
            format!(" {} ", uptime),
            Style::default().fg(Theme::text_dim()),
        ),
        Span::styled("│", Style::default().fg(Theme::text_dim())),
        Span::styled(
            format!(" {} ", now),
            Style::default().fg(Theme::text_dim()),
        ),
    ]);

    let footer_line = Line::from(vec![
        Span::styled(
            format!(" {} ", footer),
            Style::default().fg(Theme::text_dim()),
        ),
    ]);

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Theme::border_style());

    let status = Paragraph::new(vec![status_line, footer_line]).block(block);
    f.render_widget(status, area);
}
