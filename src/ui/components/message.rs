//! Simple message modal component
//!
//! Provides a reusable modal for displaying messages (error, success, info).

use crate::app::ThemeColors;
use ratatui::{
    layout::Rect,
    style::Style,
    text::Span,
    Frame,
};

use super::ModalFrame;

/// Message modal type for styling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    /// Error message (red title)
    Error,
    /// Success message (green title)
    Success,
    /// Info message (blue title)
    Info,
    /// Warning message (yellow title)
    Warning,
}

/// Simple message modal for displaying text messages
///
/// # Example
/// ```ignore
/// use crate::ui::components::MessageModal;
///
/// // Error modal
/// MessageModal::error("Failed to copy file")
///     .render(frame, area, &colors);
///
/// // Success modal
/// MessageModal::success("File copied successfully")
///     .render(frame, area, &colors);
///
/// // Custom modal with hint
/// MessageModal::new("Custom Title", "Your message here")
///     .hint("Press Enter to continue")
///     .render(frame, area, &colors);
/// ```
pub struct MessageModal {
    title: String,
    message: String,
    hint: String,
    message_type: MessageType,
}

impl MessageModal {
    /// Create a new message modal with custom title
    pub fn new(title: &str, message: &str) -> Self {
        Self {
            title: title.to_string(),
            message: message.to_string(),
            hint: "Press any key to close".to_string(),
            message_type: MessageType::Info,
        }
    }

    /// Create an error modal
    pub fn error(message: &str) -> Self {
        Self {
            title: "Error".to_string(),
            message: message.to_string(),
            hint: "Press any key to close".to_string(),
            message_type: MessageType::Error,
        }
    }

    /// Create a success modal
    pub fn success(message: &str) -> Self {
        Self {
            title: "Success".to_string(),
            message: message.to_string(),
            hint: "Press any key to close".to_string(),
            message_type: MessageType::Success,
        }
    }

    /// Create an info modal
    pub fn info(message: &str) -> Self {
        Self {
            title: "Info".to_string(),
            message: message.to_string(),
            hint: "Press any key to close".to_string(),
            message_type: MessageType::Info,
        }
    }

    /// Create a warning modal
    pub fn warning(message: &str) -> Self {
        Self {
            title: "Warning".to_string(),
            message: message.to_string(),
            hint: "Press any key to close".to_string(),
            message_type: MessageType::Warning,
        }
    }

    /// Set custom hint text
    pub fn hint(mut self, hint: &str) -> Self {
        self.hint = hint.to_string();
        self
    }

    /// Set custom title (overrides type-based title)
    pub fn title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    /// Calculate modal dimensions based on content
    fn dimensions(&self) -> (u16, u16) {
        let msg_width = self.message.len().min(60);
        let hint_width = self.hint.len();
        let title_width = self.title.len() + 4;

        let width = msg_width.max(hint_width).max(title_width).max(20) + 6;
        let height: u16 = 8;

        (width as u16, height)
    }

    /// Get centered area for modal
    pub fn centered_area(&self, area: Rect) -> Rect {
        let (width, height) = self.dimensions();
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        Rect::new(x, y, width.min(area.width), height.min(area.height))
    }

    /// Render the message modal
    pub fn render(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        let modal_area = self.centered_area(area);
        let title = format!(" {} ", self.title);

        let modal = ModalFrame::themed(modal_area, &title, colors).no_footer_separator();
        modal.render_frame(frame);

        // Message style based on type
        let msg_color = match self.message_type {
            MessageType::Error => colors.red(),
            MessageType::Warning => colors.yellow(),
            _ => colors.fg(),
        };

        // Content
        modal.render_row(frame, 0, vec![]);
        modal.render_row(
            frame,
            1,
            vec![Span::styled(&self.message, Style::default().fg(msg_color))],
        );
        modal.render_row(frame, 2, vec![]);
        modal.render_row(
            frame,
            3,
            vec![Span::styled(&self.hint, Style::default().fg(colors.green()))],
        );
    }
}
