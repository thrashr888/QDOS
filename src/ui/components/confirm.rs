//! Confirm Dialog Component
//!
//! A reusable confirmation dialog with Yes/No buttons.
//!
//! # Example
//! ```ignore
//! let dialog = ConfirmDialog::new("Delete 5 files?")
//!     .with_warning("This cannot be undone.")
//!     .with_command("rm -rf ./files")  // Shows "$ rm -rf ./files"
//!     .yes_label("Delete")
//!     .no_label("Cancel");
//!
//! // Handle key
//! match key.code {
//!     KeyCode::Char('y') | KeyCode::Char('Y') => return Some(true),
//!     KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => return Some(false),
//!     KeyCode::Left | KeyCode::Right | KeyCode::Tab => dialog.toggle_selection(),
//!     KeyCode::Enter => return Some(dialog.is_yes_selected()),
//!     _ => {}
//! }
//!
//! // Render inside a modal frame
//! dialog.render(frame, area, colors);
//! ```

use crate::app::ThemeColors;
use crate::ui::components::ModalFrame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// A confirmation dialog with Yes/No buttons.
#[derive(Debug, Clone)]
pub struct ConfirmDialog {
    /// Main message/question
    message: String,
    /// Optional warning or details (shown in yellow)
    warning: Option<String>,
    /// Optional command preview (shown in yellow with $ prefix)
    command: Option<String>,
    /// Label for the "yes" action
    yes_label: String,
    /// Label for the "no" action
    no_label: String,
    /// True if "yes" is selected, false if "no" is selected
    yes_selected: bool,
    /// Title for the modal
    title: String,
}

impl ConfirmDialog {
    /// Create a new confirmation dialog with a message.
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
            warning: None,
            command: None,
            yes_label: "Yes".to_string(),
            no_label: "No".to_string(),
            yes_selected: false, // Default to "No" for safety
            title: " CONFIRM ".to_string(),
        }
    }

    /// Set the dialog title.
    pub fn title(mut self, title: &str) -> Self {
        self.title = format!(" {} ", title.trim());
        self
    }

    /// Add a warning message (shown in yellow).
    pub fn with_warning(mut self, warning: &str) -> Self {
        self.warning = Some(warning.to_string());
        self
    }

    /// Add a command preview (shown in yellow with $ prefix).
    /// Use this to show the user what command will be executed.
    pub fn with_command(mut self, command: &str) -> Self {
        self.command = Some(command.to_string());
        self
    }

    /// Set the "yes" button label.
    pub fn yes_label(mut self, label: &str) -> Self {
        self.yes_label = label.to_string();
        self
    }

    /// Set the "no" button label.
    pub fn no_label(mut self, label: &str) -> Self {
        self.no_label = label.to_string();
        self
    }

    /// Start with "yes" selected (use carefully).
    pub fn select_yes(mut self) -> Self {
        self.yes_selected = true;
        self
    }

    /// Toggle between yes and no selection.
    pub fn toggle_selection(&mut self) {
        self.yes_selected = !self.yes_selected;
    }

    /// Select "yes".
    pub fn select_yes_button(&mut self) {
        self.yes_selected = true;
    }

    /// Select "no".
    pub fn select_no_button(&mut self) {
        self.yes_selected = false;
    }

    /// Check if "yes" is currently selected.
    pub fn is_yes_selected(&self) -> bool {
        self.yes_selected
    }

    /// Get the result based on current selection.
    pub fn confirm(&self) -> bool {
        self.yes_selected
    }

    /// Calculate the required height for the dialog.
    pub fn required_height(&self) -> u16 {
        let mut height = 5; // Title + message + buttons + borders
        if self.warning.is_some() {
            height += 2; // Warning line + spacing
        }
        if self.command.is_some() {
            height += 2; // Command line + spacing
        }
        height
    }

    /// Calculate the required width for the dialog.
    pub fn required_width(&self) -> u16 {
        let message_len = self.message.len();
        let warning_len = self.warning.as_ref().map_or(0, |w| w.len());
        let command_len = self.command.as_ref().map_or(0, |c| c.len() + 2); // +2 for "$ " prefix
        let buttons_len = self.yes_label.len() + self.no_label.len() + 10; // [Yes]  [No] + padding

        (message_len
            .max(warning_len)
            .max(command_len)
            .max(buttons_len)
            + 6) as u16
    }

    /// Render the dialog inside a centered modal.
    pub fn render(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        let width = self.required_width().min(area.width.saturating_sub(4));
        let height = self.required_height().min(area.height.saturating_sub(2));

        // Center the dialog
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        let dialog_area = Rect::new(x, y, width, height);

        // Create modal frame
        let modal = ModalFrame::themed(dialog_area, &self.title, colors);
        modal.render_frame(frame);

        let content_area = modal.content_area();
        self.render_content(frame, content_area, colors);
    }

    /// Render just the content (message, warning, command, buttons) within a given area.
    pub fn render_content(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        let mut y = area.y;

        // Render message
        let msg_style = Style::default().fg(colors.fg()).bg(colors.bg());
        let msg_x = area.x + (area.width.saturating_sub(self.message.len() as u16)) / 2;
        frame.render_widget(
            Paragraph::new(Span::styled(&self.message, msg_style)),
            Rect::new(msg_x, y, self.message.len() as u16, 1),
        );
        y += 1;

        // Render warning if present
        if let Some(ref warning) = self.warning {
            y += 1; // Empty line
            let warn_style = Style::default().fg(colors.yellow()).bg(colors.bg());
            let warn_x = area.x + (area.width.saturating_sub(warning.len() as u16)) / 2;
            frame.render_widget(
                Paragraph::new(Span::styled(warning, warn_style)),
                Rect::new(warn_x, y, warning.len() as u16, 1),
            );
            y += 1;
        }

        // Render command preview if present
        if let Some(ref command) = self.command {
            y += 1; // Empty line
            let cmd_text = format!("$ {}", command);
            let cmd_style = Style::default().fg(colors.yellow()).bg(colors.bg());
            let cmd_x = area.x + (area.width.saturating_sub(cmd_text.len() as u16)) / 2;
            frame.render_widget(
                Paragraph::new(Span::styled(&cmd_text, cmd_style)),
                Rect::new(cmd_x, y, cmd_text.len() as u16, 1),
            );
            y += 1;
        }

        y += 1; // Empty line before buttons

        // Render buttons
        self.render_buttons(frame, Rect::new(area.x, y, area.width, 1), colors);
    }

    fn render_buttons(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        let yes_btn = format!(" {} ", self.yes_label);
        let no_btn = format!(" {} ", self.no_label);

        let yes_style = if self.yes_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            Style::default().fg(colors.fg()).bg(colors.bg())
        };

        let no_style = if !self.yes_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            Style::default().fg(colors.fg()).bg(colors.bg())
        };

        let bracket_style = Style::default().fg(colors.fg()).bg(colors.bg());

        let total_width = yes_btn.len() + no_btn.len() + 8; // [ ] + [ ] + spacing
        let start_x = area.x + (area.width.saturating_sub(total_width as u16)) / 2;

        let spans = vec![
            Span::styled("[", bracket_style),
            Span::styled(yes_btn, yes_style),
            Span::styled("]", bracket_style),
            Span::styled("  ", bracket_style),
            Span::styled("[", bracket_style),
            Span::styled(no_btn, no_style),
            Span::styled("]", bracket_style),
        ];

        frame.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect::new(start_x, area.y, total_width as u16, 1),
        );
    }

    /// Render help text for the dialog.
    pub fn render_help(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        let help_style = Style::default().fg(colors.green()).bg(colors.bg());
        let key_style = Style::default().fg(colors.green()).bg(colors.bg());

        let spans = vec![
            Span::styled("Y", key_style),
            Span::styled("/", help_style),
            Span::styled("N", key_style),
            Span::styled(" yes/no  ", help_style),
            Span::styled("←→", key_style),
            Span::styled(" select  ", help_style),
            Span::styled("Enter", key_style),
            Span::styled(" confirm  ", help_style),
            Span::styled("Esc", key_style),
            Span::styled(" cancel", help_style),
        ];

        let total_width: usize = spans.iter().map(|s| s.width()).sum();
        let x = area.x + (area.width.saturating_sub(total_width as u16)) / 2;

        frame.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect::new(x, area.y, total_width as u16, 1),
        );
    }
}

/// Result of handling a key event in a confirmation dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmResult {
    /// User hasn't made a decision yet
    Pending,
    /// User confirmed (yes)
    Confirmed,
    /// User cancelled (no/escape)
    Cancelled,
}

impl ConfirmDialog {
    /// Handle a key event and return the result.
    ///
    /// Returns `ConfirmResult::Pending` if no decision was made.
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> ConfirmResult {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => ConfirmResult::Confirmed,
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => ConfirmResult::Cancelled,
            KeyCode::Left | KeyCode::Char('h') => {
                self.select_yes_button();
                ConfirmResult::Pending
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.select_no_button();
                ConfirmResult::Pending
            }
            KeyCode::Tab => {
                self.toggle_selection();
                ConfirmResult::Pending
            }
            KeyCode::Enter => {
                if self.yes_selected {
                    ConfirmResult::Confirmed
                } else {
                    ConfirmResult::Cancelled
                }
            }
            _ => ConfirmResult::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialog_creation() {
        let dialog = ConfirmDialog::new("Delete files?");
        assert!(!dialog.is_yes_selected()); // Default to no
        assert_eq!(dialog.message, "Delete files?");
    }

    #[test]
    fn test_selection_toggle() {
        let mut dialog = ConfirmDialog::new("Test?");
        assert!(!dialog.is_yes_selected());

        dialog.toggle_selection();
        assert!(dialog.is_yes_selected());

        dialog.toggle_selection();
        assert!(!dialog.is_yes_selected());
    }

    #[test]
    fn test_builder_pattern() {
        let dialog = ConfirmDialog::new("Save changes?")
            .title("SAVE")
            .with_warning("Unsaved data will be lost.")
            .yes_label("Save")
            .no_label("Discard")
            .select_yes();

        assert!(dialog.is_yes_selected());
        assert_eq!(dialog.yes_label, "Save");
        assert_eq!(dialog.no_label, "Discard");
        assert!(dialog.warning.is_some());
    }

    #[test]
    fn test_required_dimensions() {
        let dialog = ConfirmDialog::new("Short");
        assert!(dialog.required_width() >= 15);
        assert!(dialog.required_height() >= 5);

        let dialog_with_warning = ConfirmDialog::new("Test").with_warning("Warning text");
        assert!(dialog_with_warning.required_height() > dialog.required_height());
    }
}
