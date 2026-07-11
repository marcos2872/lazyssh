use std::collections::VecDeque;
use std::time::{Duration, Instant};

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::theme::Theme;

/// Tipo de notificação (determina a cor e duração).
#[derive(Debug, Clone)]
pub enum NotificationType {
    /// Mensagem informativa (azul, 3s).
    Info,
    /// Operação concluída com sucesso (verde, 3s).
    Success,
    /// Aviso não-crítico (amarelo, 4s).
    Warning,
    /// Erro que o usuário precisa ver (vermelho, 5s).
    Error,
}

/// Uma notificação individual com mensagem, tipo e tempo de expiração.
#[derive(Debug, Clone)]
pub struct Notification {
    /// Texto da notificação.
    pub message: String,
    /// Tipo da notificação (determina cor e duração).
    pub notification_type: NotificationType,
    /// Instante em que a notificação foi criada.
    pub created_at: Instant,
    /// Duração máxima antes de expirar.
    pub duration: Duration,
}

impl Notification {
    /// Cria uma notificação informativa (expira em 3 segundos).
    pub fn info(message: &str) -> Self {
        Self {
            message: message.to_string(),
            notification_type: NotificationType::Info,
            created_at: Instant::now(),
            duration: Duration::from_secs(3),
        }
    }

    /// Cria uma notificação de sucesso (expira em 3 segundos).
    pub fn success(message: &str) -> Self {
        Self {
            message: message.to_string(),
            notification_type: NotificationType::Success,
            created_at: Instant::now(),
            duration: Duration::from_secs(3),
        }
    }

    /// Cria uma notificação de aviso (expira em 4 segundos).
    pub fn warning(message: &str) -> Self {
        Self {
            message: message.to_string(),
            notification_type: NotificationType::Warning,
            created_at: Instant::now(),
            duration: Duration::from_secs(4),
        }
    }

    /// Cria uma notificação de erro (expira em 5 segundos).
    pub fn error(message: &str) -> Self {
        Self {
            message: message.to_string(),
            notification_type: NotificationType::Error,
            created_at: Instant::now(),
            duration: Duration::from_secs(5),
        }
    }

    /// Retorna `true` se a notificação já expirou.
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.duration
    }

    /// Retorna os segundos restantes antes da expiração.
    pub fn remaining_secs(&self) -> u64 {
        let elapsed = self.created_at.elapsed();
        if elapsed >= self.duration {
            0
        } else {
            (self.duration - elapsed).as_secs()
        }
    }
}

/// Fila de notificações exibidas na barra inferior da interface.
#[derive(Debug)]
pub struct NotificationQueue {
    notifications: VecDeque<Notification>,
    max_visible: usize,
}

impl NotificationQueue {
    /// Cria uma nova fila de notificações.
    pub fn new() -> Self {
        Self {
            notifications: VecDeque::new(),
            max_visible: 3,
        }
    }

    /// Adiciona uma notificação à fila. Remove as mais antigas se exceder o limite.
    pub fn push(&mut self, notification: Notification) {
        self.notifications.push_back(notification);
        // Manter apenas as notificações mais recentes
        while self.notifications.len() > self.max_visible + 5 {
            self.notifications.pop_front();
        }
    }

    /// Atalho para adicionar uma notificação informativa.
    pub fn info(&mut self, message: &str) {
        self.push(Notification::info(message));
    }

    /// Atalho para adicionar uma notificação de sucesso.
    pub fn success(&mut self, message: &str) {
        self.push(Notification::success(message));
    }

    /// Atalho para adicionar uma notificação de aviso.
    pub fn warning(&mut self, message: &str) {
        self.push(Notification::warning(message));
    }

    /// Atalho para adicionar uma notificação de erro.
    pub fn error(&mut self, message: &str) {
        self.push(Notification::error(message));
    }

    /// Remove todas as notificações que já expiraram.
    pub fn clear_expired(&mut self) {
        self.notifications.retain(|n| !n.is_expired());
    }

    /// Retorna as notificações visíveis (não expiradas, no máximo `max_visible`).
    pub fn visible_notifications(&self) -> Vec<&Notification> {
        self.notifications
            .iter()
            .filter(|n| !n.is_expired())
            .take(self.max_visible)
            .collect()
    }
}

/// Renderiza as notificações visíveis na parte inferior da área informada.
pub fn render_notifications(f: &mut Frame, queue: &NotificationQueue, area: Rect) {
    let visible = queue.visible_notifications();
    if visible.is_empty() {
        return;
    }

    // Posicionar no canto superior direito
    let popup_width = 40.min(area.width.saturating_sub(4));
    let popup_height = (visible.len() as u16 + 2).min(area.height.saturating_sub(4));
    let x = area.x + area.width.saturating_sub(popup_width + 2);
    let y = area.y + 2;

    let popup_area = Rect::new(x, y, popup_width, popup_height);

    let mut lines = vec![];
    for notification in &visible {
        let (icon, color) = match notification.notification_type {
            NotificationType::Info => ("ℹ", Theme::primary()),
            NotificationType::Success => ("✓", Theme::success()),
            NotificationType::Warning => ("⚠", Theme::warning()),
            NotificationType::Error => ("✗", Theme::error()),
        };

        let remaining = notification.remaining_secs();
        let line = Line::from(vec![
            Span::styled(format!("{} ", icon), Style::default().fg(color)),
            Span::styled(&notification.message, Style::default().fg(color)),
            Span::styled(
                format!(" ({}s)", remaining),
                Theme::dim_style(),
            ),
        ]);
        lines.push(line);
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" 🔔 Notificações ")
        .title_style(Theme::title_style())
        .border_style(Theme::border_style())
        .style(Style::default().bg(Color::Black));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, popup_area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_notification_info() {
        let n = Notification::info("test info");
        assert_eq!(n.message, "test info");
        assert!(matches!(n.notification_type, NotificationType::Info));
        assert_eq!(n.duration, Duration::from_secs(3));
    }

    #[test]
    fn test_notification_success() {
        let n = Notification::success("ok");
        assert!(matches!(n.notification_type, NotificationType::Success));
        assert_eq!(n.duration, Duration::from_secs(3));
    }

    #[test]
    fn test_notification_warning() {
        let n = Notification::warning("caution");
        assert!(matches!(n.notification_type, NotificationType::Warning));
        assert_eq!(n.duration, Duration::from_secs(4));
    }

    #[test]
    fn test_notification_error() {
        let n = Notification::error("fail");
        assert!(matches!(n.notification_type, NotificationType::Error));
        assert_eq!(n.duration, Duration::from_secs(5));
    }

    #[test]
    fn test_notification_is_expired() {
        let mut n = Notification::info("test");
        n.created_at = Instant::now() - Duration::from_secs(10);
        n.duration = Duration::from_millis(1);
        assert!(n.is_expired());
    }

    #[test]
    fn test_notification_not_expired() {
        let n = Notification::info("fresh");
        assert!(!n.is_expired());
    }

    #[test]
    fn test_notification_remaining_secs() {
        let mut n = Notification::info("test");
        n.created_at = Instant::now();
        n.duration = Duration::from_secs(5);
        let rem = n.remaining_secs();
        assert!(rem <= 5 && rem >= 4, "remaining should be ~5s, got {}", rem);
    }

    #[test]
    fn test_notification_remaining_expired() {
        let mut n = Notification::info("test");
        n.created_at = Instant::now() - Duration::from_secs(10);
        n.duration = Duration::from_millis(1);
        assert_eq!(n.remaining_secs(), 0);
    }

    #[test]
    fn test_notification_queue_new() {
        let q = NotificationQueue::new();
        assert!(q.visible_notifications().is_empty());
    }

    #[test]
    fn test_notification_queue_push() {
        let mut q = NotificationQueue::new();
        q.info("msg1");
        q.success("msg2");
        assert_eq!(q.visible_notifications().len(), 2);
    }

    #[test]
    fn test_notification_queue_clear_expired() {
        let mut q = NotificationQueue::new();

        // Push a notification that will expire immediately
        let mut expired = Notification::info("expired");
        expired.created_at = Instant::now() - Duration::from_secs(10);
        expired.duration = Duration::from_millis(1);
        q.push(expired);

        q.info("fresh");
        q.clear_expired();
        assert_eq!(q.visible_notifications().len(), 1);
        let visible = q.visible_notifications();
        assert_eq!(visible[0].message, "fresh");
    }

    #[test]
    fn test_notification_queue_max_visible() {
        let mut q = NotificationQueue::new();
        q.info("1");
        q.info("2");
        q.info("3");
        q.info("4");
        assert_eq!(q.visible_notifications().len(), 3);
    }

    #[test]
    fn test_notification_queue_types() {
        let mut q = NotificationQueue::new();
        q.info("info");
        q.success("success");
        q.warning("warning");

        let visible = q.visible_notifications();
        assert_eq!(visible.len(), 3);
        assert!(matches!(visible[0].notification_type, NotificationType::Info));
        assert!(matches!(visible[1].notification_type, NotificationType::Success));
        assert!(matches!(visible[2].notification_type, NotificationType::Warning));
    }
}
