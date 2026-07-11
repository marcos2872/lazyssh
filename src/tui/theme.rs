use ratatui::style::{Color, Modifier, Style};

/// Tema visual da aplicação — cores e estilos reutilizáveis para todos os widgets TUI.
pub struct Theme;

impl Theme {
    // === Cores principais ===

    /// Cor primária (ciano) — usada para títulos e elementos de destaque.
    pub fn primary() -> Color { Color::Cyan }
    /// Cor secundária (azul) — usada para bordas e labels.
    pub fn secondary() -> Color { Color::Blue }
    /// Cor de destaque (magenta) — usada para marcadores e bordas modais.
    pub fn accent() -> Color { Color::Magenta }
    /// Cor de sucesso (verde) — usada para mensagens de sucesso e ações confirmadas.
    pub fn success() -> Color { Color::Green }
    /// Cor de aviso (amarelo) — usada para servidores fixados e avisos.
    pub fn warning() -> Color { Color::Yellow }
    /// Cor de erro (vermelho) — usada para erros e cancelamentos.
    pub fn error() -> Color { Color::Red }
    /// Cor de fundo (cinza escuro).
    pub fn background() -> Color { Color::DarkGray }
    /// Cor de texto principal (branco).
    pub fn text() -> Color { Color::White }
    /// Cor de texto secundário/desabilitado (cinza).
    pub fn text_dim() -> Color { Color::Gray }

    // === Estilos ===

    /// Estilo para títulos de seção (ciano, negrito).
    pub fn title_style() -> Style {
        Style::default()
            .fg(Self::primary())
            .add_modifier(Modifier::BOLD)
    }

    /// Estilo para item selecionado na lista (fundo ciano, texto preto).
    pub fn selected_style() -> Style {
        Style::default()
            .bg(Self::primary())
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
    }

    /// Estilo para bordas de widgets (azul).
    pub fn border_style() -> Style {
        Style::default().fg(Self::secondary())
    }

    /// Estilo para bordas de modais (magenta, negrito).
    pub fn modal_border_style() -> Style {
        Style::default()
            .fg(Self::accent())
            .add_modifier(Modifier::BOLD)
    }

    /// Estilo para títulos de modal (magenta, negrito).
    pub fn modal_title_style() -> Style {
        Style::default()
            .fg(Self::accent())
            .add_modifier(Modifier::BOLD)
    }

    /// Estilo para campos de input inativos (texto branco).
    pub fn input_style() -> Style {
        Style::default()
            .fg(Self::text())
    }

    /// Estilo para campos de input ativos (ciano, negrito).
    pub fn input_active_style() -> Style {
        Style::default()
            .fg(Self::primary())
            .add_modifier(Modifier::BOLD)
    }

    /// Estilo para prompts de comando (verde, negrito).
    pub fn prompt_style() -> Style {
        Style::default()
            .fg(Self::success())
            .add_modifier(Modifier::BOLD)
    }

    pub fn error_style() -> Style {
        Style::default()
            .fg(Self::error())
    }

    pub fn success_style() -> Style {
        Style::default()
            .fg(Self::success())
    }

    pub fn warning_style() -> Style {
        Style::default()
            .fg(Self::warning())
    }

    pub fn dim_style() -> Style {
        Style::default()
            .fg(Self::text_dim())
    }

    pub fn tag_color(name: &str) -> Color {
        let hash: u32 = name.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
        let colors = [
            Color::Cyan, Color::Blue, Color::Magenta, Color::Green,
            Color::Yellow, Color::Red, Color::LightCyan, Color::LightBlue,
            Color::LightMagenta, Color::LightGreen,
        ];
        colors[hash as usize % colors.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primary_color() {
        assert_eq!(Theme::primary(), Color::Cyan);
    }

    #[test]
    fn test_secondary_color() {
        assert_eq!(Theme::secondary(), Color::Blue);
    }

    #[test]
    fn test_accent_color() {
        assert_eq!(Theme::accent(), Color::Magenta);
    }

    #[test]
    fn test_success_color() {
        assert_eq!(Theme::success(), Color::Green);
    }

    #[test]
    fn test_warning_color() {
        assert_eq!(Theme::warning(), Color::Yellow);
    }

    #[test]
    fn test_error_color() {
        assert_eq!(Theme::error(), Color::Red);
    }

    #[test]
    fn test_background_color() {
        assert_eq!(Theme::background(), Color::DarkGray);
    }

    #[test]
    fn test_text_color() {
        assert_eq!(Theme::text(), Color::White);
    }

    #[test]
    fn test_text_dim_color() {
        assert_eq!(Theme::text_dim(), Color::Gray);
    }

    #[test]
    fn test_title_style_has_bold() {
        let s = Theme::title_style();
        assert_eq!(s.fg, Some(Color::Cyan));
        assert!(s.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_selected_style() {
        let s = Theme::selected_style();
        assert_eq!(s.bg, Some(Color::Cyan));
        assert_eq!(s.fg, Some(Color::Black));
        assert!(s.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_border_style() {
        let s = Theme::border_style();
        assert_eq!(s.fg, Some(Color::Blue));
    }

    #[test]
    fn test_modal_border_style() {
        let s = Theme::modal_border_style();
        assert_eq!(s.fg, Some(Color::Magenta));
        assert!(s.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_input_style() {
        let s = Theme::input_style();
        assert_eq!(s.fg, Some(Color::White));
    }

    #[test]
    fn test_input_active_style() {
        let s = Theme::input_active_style();
        assert_eq!(s.fg, Some(Color::Cyan));
        assert!(s.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_prompt_style() {
        let s = Theme::prompt_style();
        assert_eq!(s.fg, Some(Color::Green));
        assert!(s.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_error_style() {
        let s = Theme::error_style();
        assert_eq!(s.fg, Some(Color::Red));
    }

    #[test]
    fn test_success_style() {
        let s = Theme::success_style();
        assert_eq!(s.fg, Some(Color::Green));
    }

    #[test]
    fn test_warning_style() {
        let s = Theme::warning_style();
        assert_eq!(s.fg, Some(Color::Yellow));
    }

    #[test]
    fn test_dim_style() {
        let s = Theme::dim_style();
        assert_eq!(s.fg, Some(Color::Gray));
    }
}
