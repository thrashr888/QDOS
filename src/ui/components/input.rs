//! Input Field Component
//!
//! A reusable text input widget with cursor management.
//!
//! # Example
//! ```ignore
//! let mut input = InputField::new();
//! input.set_placeholder("Enter path...");
//!
//! // Handle keys
//! match key.code {
//!     KeyCode::Char(c) => input.insert(c),
//!     KeyCode::Backspace => input.backspace(),
//!     KeyCode::Delete => input.delete(),
//!     KeyCode::Left => input.move_left(),
//!     KeyCode::Right => input.move_right(),
//!     KeyCode::Home => input.move_home(),
//!     KeyCode::End => input.move_end(),
//!     _ => {}
//! }
//!
//! // Render
//! input.render(frame, area, colors, is_focused);
//! ```

use crate::app::ThemeColors;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// A text input field with cursor support.
#[derive(Debug, Clone, Default)]
pub struct InputField {
    /// The input text content
    content: String,
    /// Cursor position (byte index)
    cursor: usize,
    /// Placeholder text when empty
    placeholder: String,
    /// Maximum length (0 = unlimited)
    max_len: usize,
}

impl InputField {
    /// Create a new empty input field.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an input field with initial content.
    pub fn with_content(content: &str) -> Self {
        let len = content.len();
        Self {
            content: content.to_string(),
            cursor: len,
            placeholder: String::new(),
            max_len: 0,
        }
    }

    /// Set placeholder text.
    pub fn set_placeholder(&mut self, placeholder: &str) {
        self.placeholder = placeholder.to_string();
    }

    /// Set maximum length.
    pub fn set_max_len(&mut self, max_len: usize) {
        self.max_len = max_len;
    }

    /// Get the current content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the content as owned String.
    pub fn take_content(&mut self) -> String {
        self.cursor = 0;
        std::mem::take(&mut self.content)
    }

    /// Set the content and move cursor to end.
    pub fn set_content(&mut self, content: &str) {
        self.content = content.to_string();
        self.cursor = self.content.len();
    }

    /// Check if the input is empty.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Clear the input.
    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor = 0;
    }

    /// Get cursor position.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    // --- Editing operations ---

    /// Insert a character at cursor position.
    pub fn insert(&mut self, c: char) {
        if self.max_len > 0 && self.content.len() >= self.max_len {
            return;
        }
        self.content.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    /// Insert a string at cursor position.
    pub fn insert_str(&mut self, s: &str) {
        if self.max_len > 0 && self.content.len() + s.len() > self.max_len {
            return;
        }
        self.content.insert_str(self.cursor, s);
        self.cursor += s.len();
    }

    /// Delete character before cursor (backspace).
    pub fn backspace(&mut self) -> bool {
        if self.cursor > 0 {
            // Find the previous character boundary
            let prev = self.prev_char_boundary();
            self.content.drain(prev..self.cursor);
            self.cursor = prev;
            true
        } else {
            false
        }
    }

    /// Delete character at cursor (delete key).
    pub fn delete(&mut self) -> bool {
        if self.cursor < self.content.len() {
            let next = self.next_char_boundary();
            self.content.drain(self.cursor..next);
            true
        } else {
            false
        }
    }

    /// Delete word before cursor (Ctrl+Backspace).
    pub fn delete_word_back(&mut self) {
        let start = self.prev_word_boundary();
        self.content.drain(start..self.cursor);
        self.cursor = start;
    }

    /// Delete to end of line (Ctrl+K).
    pub fn delete_to_end(&mut self) {
        self.content.truncate(self.cursor);
    }

    /// Delete entire line (Ctrl+U).
    pub fn delete_line(&mut self) {
        self.clear();
    }

    // --- Cursor movement ---

    /// Move cursor left.
    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.prev_char_boundary();
        }
    }

    /// Move cursor right.
    pub fn move_right(&mut self) {
        if self.cursor < self.content.len() {
            self.cursor = self.next_char_boundary();
        }
    }

    /// Move cursor to start.
    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to end.
    pub fn move_end(&mut self) {
        self.cursor = self.content.len();
    }

    /// Move cursor to previous word boundary (Ctrl+Left).
    pub fn move_word_left(&mut self) {
        self.cursor = self.prev_word_boundary();
    }

    /// Move cursor to next word boundary (Ctrl+Right).
    pub fn move_word_right(&mut self) {
        self.cursor = self.next_word_boundary();
    }

    // --- Helper methods ---

    fn prev_char_boundary(&self) -> usize {
        let mut idx = self.cursor.saturating_sub(1);
        while idx > 0 && !self.content.is_char_boundary(idx) {
            idx -= 1;
        }
        idx
    }

    fn next_char_boundary(&self) -> usize {
        let mut idx = self.cursor + 1;
        while idx < self.content.len() && !self.content.is_char_boundary(idx) {
            idx += 1;
        }
        idx.min(self.content.len())
    }

    fn prev_word_boundary(&self) -> usize {
        let bytes = self.content.as_bytes();
        let mut idx = self.cursor;

        // Skip any trailing whitespace
        while idx > 0 && bytes.get(idx - 1).is_some_and(|&b| b == b' ') {
            idx -= 1;
        }

        // Find start of word
        while idx > 0 && bytes.get(idx - 1).is_some_and(|&b| b != b' ') {
            idx -= 1;
        }

        idx
    }

    fn next_word_boundary(&self) -> usize {
        let bytes = self.content.as_bytes();
        let mut idx = self.cursor;

        // Skip current word
        while idx < bytes.len() && bytes.get(idx).is_some_and(|&b| b != b' ') {
            idx += 1;
        }

        // Skip whitespace
        while idx < bytes.len() && bytes.get(idx).is_some_and(|&b| b == b' ') {
            idx += 1;
        }

        idx
    }

    // --- Rendering ---

    /// Render the input field.
    ///
    /// Shows cursor as a block character when focused.
    pub fn render(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors, focused: bool) {
        let style = if focused {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            Style::default().fg(colors.fg()).bg(colors.bg())
        };

        self.render_styled(frame, area, style, focused);
    }

    /// Render with custom style.
    pub fn render_styled(&self, frame: &mut Frame, area: Rect, style: Style, show_cursor: bool) {
        let display_text = if self.content.is_empty() && !self.placeholder.is_empty() {
            // Show placeholder in grey
            let placeholder_style = style.fg(ratatui::style::Color::DarkGray);
            frame.render_widget(
                Paragraph::new(Span::styled(&self.placeholder, placeholder_style)),
                area,
            );
            return;
        } else {
            &self.content
        };

        if show_cursor {
            // Split text at cursor position for cursor rendering
            let (before, after) = display_text.split_at(self.cursor);

            let cursor_char = after.chars().next().unwrap_or(' ');
            let after_cursor = if after.is_empty() {
                ""
            } else {
                &after[cursor_char.len_utf8()..]
            };

            // Build spans: before + cursor_char (inverted) + after
            let spans = vec![
                Span::styled(before.to_string(), style),
                Span::styled(
                    cursor_char.to_string(),
                    Style::default()
                        .fg(style.bg.unwrap_or(ratatui::style::Color::Black))
                        .bg(style.fg.unwrap_or(ratatui::style::Color::White)),
                ),
                Span::styled(after_cursor.to_string(), style),
            ];

            // Pad to full width
            let content_len = display_text.len() + 1; // +1 for cursor when at end
            let pad = (area.width as usize).saturating_sub(content_len);

            let mut all_spans = spans;
            if pad > 0 {
                all_spans.push(Span::styled(" ".repeat(pad), style));
            }

            frame.render_widget(Paragraph::new(Line::from(all_spans)), area);
        } else {
            // No cursor, just render text
            let pad = (area.width as usize).saturating_sub(display_text.len());
            let padded = format!("{}{}", display_text, " ".repeat(pad));
            frame.render_widget(Paragraph::new(Span::styled(padded, style)), area);
        }
    }

    /// Get visible portion of text for given width, with cursor visible.
    pub fn visible_text(&self, width: usize) -> (&str, usize) {
        if self.content.len() <= width {
            return (&self.content, self.cursor);
        }

        // Scroll to keep cursor visible
        let half_width = width / 2;
        let scroll = if self.cursor > half_width {
            (self.cursor - half_width).min(self.content.len().saturating_sub(width))
        } else {
            0
        };

        let end = (scroll + width).min(self.content.len());
        (&self.content[scroll..end], self.cursor - scroll)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_input() {
        let mut input = InputField::new();
        assert!(input.is_empty());

        input.insert('h');
        input.insert('i');
        assert_eq!(input.content(), "hi");
        assert_eq!(input.cursor(), 2);
    }

    #[test]
    fn test_backspace() {
        let mut input = InputField::with_content("hello");
        assert!(input.backspace());
        assert_eq!(input.content(), "hell");

        input.move_home();
        assert!(!input.backspace()); // At start, nothing to delete
    }

    #[test]
    fn test_cursor_movement() {
        let mut input = InputField::with_content("hello");

        input.move_home();
        assert_eq!(input.cursor(), 0);

        input.move_right();
        assert_eq!(input.cursor(), 1);

        input.move_end();
        assert_eq!(input.cursor(), 5);

        input.move_left();
        assert_eq!(input.cursor(), 4);
    }

    #[test]
    fn test_word_movement() {
        let mut input = InputField::with_content("hello world test");
        input.move_home();

        input.move_word_right();
        assert_eq!(input.cursor(), 6); // After "hello "

        input.move_word_right();
        assert_eq!(input.cursor(), 12); // After "world "

        input.move_word_left();
        assert_eq!(input.cursor(), 6); // Back to "world"
    }

    #[test]
    fn test_delete() {
        let mut input = InputField::with_content("hello");
        input.move_home();

        assert!(input.delete());
        assert_eq!(input.content(), "ello");
        assert_eq!(input.cursor(), 0);
    }

    #[test]
    fn test_insert_in_middle() {
        let mut input = InputField::with_content("hllo");
        input.cursor = 1; // After 'h'
        input.insert('e');
        assert_eq!(input.content(), "hello");
    }

    #[test]
    fn test_max_len() {
        let mut input = InputField::new();
        input.set_max_len(5);

        for c in "hello world".chars() {
            input.insert(c);
        }
        assert_eq!(input.content(), "hello");
    }

    #[test]
    fn test_delete_word() {
        let mut input = InputField::with_content("hello world");
        input.delete_word_back();
        assert_eq!(input.content(), "hello ");
    }

    #[test]
    fn test_visible_text() {
        let input = InputField::with_content("hello world this is a long string");

        let (visible, cursor_pos) = input.visible_text(10);
        // Cursor at end, so should show last portion
        assert!(visible.len() <= 10);
        assert!(cursor_pos <= visible.len());
    }
}
