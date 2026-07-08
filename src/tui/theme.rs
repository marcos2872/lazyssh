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
    pub fn text_dim() -> Color { Color::Gray }

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

    pub fn modal_title_style() -> Style {
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
}
