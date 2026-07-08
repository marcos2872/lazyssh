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

#[derive(Debug, Clone)]
pub enum NotificationType {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub notification_type: NotificationType,
    pub created_at: Instant,
    pub duration: Duration,
}

impl Notification {
    pub fn info(message: &str) -> Self {
        Self {
            message: message.to_string(),
            notification_type: NotificationType::Info,
            created_at: Instant::now(),
            duration: Duration::from_secs(3),
        }
    }

    pub fn success(message: &str) -> Self {
        Self {
            message: message.to_string(),
            notification_type: NotificationType::Success,
            created_at: Instant::now(),
            duration: Duration::from_secs(3),
        }
    }

    pub fn warning(message: &str) -> Self {
        Self {
            message: message.to_string(),
            notification_type: NotificationType::Warning,
            created_at: Instant::now(),
            duration: Duration::from_secs(4),
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            message: message.to_string(),
            notification_type: NotificationType::Error,
            created_at: Instant::now(),
            duration: Duration::from_secs(5),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.duration
    }

    pub fn remaining_secs(&self) -> u64 {
        let elapsed = self.created_at.elapsed();
        if elapsed >= self.duration {
            0
        } else {
            (self.duration - elapsed).as_secs()
        }
    }
}

#[derive(Debug)]
pub struct NotificationQueue {
    notifications: VecDeque<Notification>,
    max_visible: usize,
}

impl NotificationQueue {
    pub fn new() -> Self {
        Self {
            notifications: VecDeque::new(),
            max_visible: 3,
        }
    }

    pub fn push(&mut self, notification: Notification) {
        self.notifications.push_back(notification);
        // Manter apenas as notificações mais recentes
        while self.notifications.len() > self.max_visible + 5 {
            self.notifications.pop_front();
        }
    }

    pub fn info(&mut self, message: &str) {
        self.push(Notification::info(message));
    }

    pub fn success(&mut self, message: &str) {
        self.push(Notification::success(message));
    }

    pub fn warning(&mut self, message: &str) {
        self.push(Notification::warning(message));
    }

    pub fn error(&mut self, message: &str) {
        self.push(Notification::error(message));
    }

    pub fn clear_expired(&mut self) {
        self.notifications.retain(|n| !n.is_expired());
    }

    pub fn visible_notifications(&self) -> Vec<&Notification> {
        self.notifications
            .iter()
            .filter(|n| !n.is_expired())
            .take(self.max_visible)
            .collect()
    }
}

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
