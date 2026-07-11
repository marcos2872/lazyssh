//! Camada de interface de terminal — widgets ratatui, manipuladores de eventos e efeitos visuais.

pub mod app;
pub mod effects;
pub mod handlers;
pub mod help;
pub mod modals;
pub mod notifications;
pub mod sftp_browser;
pub mod server_list;
pub mod ssh_terminal;
pub mod theme;

pub use app::{App, SftpOpResult};
pub use effects::AppEffects;
pub use help::{footer_hint_for_view, render_help_modal};
pub use notifications::{render_notifications, NotificationQueue};
pub use sftp_browser::{format_size, render_sftp_browser, Side, SftpState, TransferProgress};
pub use server_list::render_server_list;
pub use ssh_terminal::render_ssh_terminal;
pub use theme::Theme;
