//! Full Screen View Component
//!
//! A full-screen layout with title, separators, content, and footer.
//! Used by plugins that take over the entire screen.
//!
//! # Pattern
//! ```text
//! +------------------------------------------+
//! | TITLE BAR                               |  <- Row 0: Title
//! |=========================================|  <- Row 1: Top separator
//! |                                         |
//! |              CONTENT AREA               |  <- Rows 2..n-2: Content
//! |                                         |
//! |=========================================|  <- Row n-1: Bottom separator
//! | Key help  Key help  Key help            |  <- Row n: Footer
//! +------------------------------------------+
//! ```
//!
//! # Example
//! ```ignore
//! let view = FullScreenView::new(area, " FIND FILES ", colors);
//! view.render_frame(frame);
//!
//! // Render content
//! let content = view.content_area();
//! for (i, item) in items.iter().enumerate() {
//!     frame.render_widget(..., Rect::new(content.x, content.y + i as u16, ...));
//! }
//!
//! // Render help
//! view.render_help(frame, vec![("Up/Dn", "select"), ("Enter", "open"), ("Esc", "close")]);
//! ```

use crate::ThemeColors;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

/// A full-screen view with title, separators, content, and footer.
///
/// Unlike `ModalFrame` which uses box-drawing borders, this component
/// uses horizontal line separators for a cleaner full-screen look.
pub struct FullScreenView {
    /// The full screen area
    pub area: Rect,
    /// The title to display
    pub title: String,
    /// Title style
    pub title_style: Style,
    /// Separator style
    pub separator_style: Style,
    /// Content style (for background fill)
    pub content_style: Style,
    /// Help key color
    pub help_key_color: Color,
    /// Separator character
    pub separator_char: char,
}

impl FullScreenView {
    /// Create a new full-screen view with theme colors.
    pub fn new(area: Rect, title: &str, colors: &ThemeColors) -> Self {
        Self {
            area,
            title: title.to_string(),
            title_style: Style::default()
                .fg(colors.fg())
                .bg(colors.bg())
                .add_modifier(Modifier::BOLD),
            separator_style: Style::default().fg(colors.fg()).bg(colors.bg()),
            content_style: Style::default().fg(colors.fg()).bg(colors.bg()),
            help_key_color: colors.green(),
            separator_char: '═',
        }
    }

    /// Set custom title style.
    pub fn title_style(mut self, style: Style) -> Self {
        self.title_style = style;
        self
    }

    /// Set custom separator character.
    pub fn separator_char(mut self, c: char) -> Self {
        self.separator_char = c;
        self
    }

    /// Get the Y position of the title row.
    pub fn title_y(&self) -> u16 {
        self.area.y
    }

    /// Get the Y position of the top separator.
    pub fn top_separator_y(&self) -> u16 {
        self.area.y + 1
    }

    /// Get the Y position of the content start.
    pub fn content_start_y(&self) -> u16 {
        self.area.y + 2
    }

    /// Get the Y position of the bottom separator.
    pub fn bottom_separator_y(&self) -> u16 {
        self.area.y + self.area.height.saturating_sub(2)
    }

    /// Get the Y position of the footer/help row.
    pub fn footer_y(&self) -> u16 {
        self.area.y + self.area.height.saturating_sub(1)
    }

    /// Get the height of the content area.
    pub fn content_height(&self) -> u16 {
        // Total height - title - top_sep - bottom_sep - footer
        self.area.height.saturating_sub(4)
    }

    /// Get the content area as a Rect.
    pub fn content_area(&self) -> Rect {
        Rect::new(
            self.area.x,
            self.content_start_y(),
            self.area.width,
            self.content_height(),
        )
    }

    /// Render the frame (clears screen, draws title and separators).
    /// Call this first, then render content, then render_help.
    pub fn render_frame(&self, frame: &mut Frame) {
        // Clear the entire area
        frame.render_widget(Clear, self.area);

        // Render title
        self.render_title(frame);

        // Render top separator
        self.render_separator(frame, self.top_separator_y());

        // Render bottom separator
        self.render_separator(frame, self.bottom_separator_y());
    }

    /// Render the title row.
    fn render_title(&self, frame: &mut Frame) {
        // Pad title to full width (use saturating_sub to prevent overflow)
        let pad = (self.area.width as usize).saturating_sub(self.title.len());
        let title_line = format!("{}{}", self.title, " ".repeat(pad));

        frame.render_widget(
            Paragraph::new(Span::styled(title_line, self.title_style)),
            Rect::new(self.area.x, self.title_y(), self.area.width, 1),
        );
    }

    /// Render a separator line at the given Y position.
    fn render_separator(&self, frame: &mut Frame, y: u16) {
        let sep = self
            .separator_char
            .to_string()
            .repeat(self.area.width as usize);
        frame.render_widget(
            Paragraph::new(Span::styled(sep, self.separator_style)),
            Rect::new(self.area.x, y, self.area.width, 1),
        );
    }

    /// Render a content row at the given offset from content start.
    pub fn render_row(&self, frame: &mut Frame, row: u16, spans: Vec<Span>) {
        let y = self.content_start_y() + row;
        if y >= self.bottom_separator_y() {
            return; // Don't render past content area
        }

        // Pad to full width (use saturating_sub to prevent overflow if content is wider than area)
        let content_width: usize = spans.iter().map(|s| s.width()).sum();
        let mut all_spans = spans;
        let pad = (self.area.width as usize).saturating_sub(content_width);
        if pad > 0 {
            all_spans.push(Span::styled(" ".repeat(pad), self.content_style));
        }

        frame.render_widget(
            Paragraph::new(Line::from(all_spans)),
            Rect::new(self.area.x, y, self.area.width, 1),
        );
    }

    /// Render the help/footer row with key hints.
    /// Takes pairs of (key, description).
    pub fn render_help(&self, frame: &mut Frame, hints: Vec<(&str, &str)>) {
        let key_style = self.content_style.fg(self.help_key_color);

        let mut spans: Vec<Span> = vec![Span::styled(" ", self.content_style)];
        for (i, (key, desc)) in hints.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled("  ", self.content_style));
            }
            spans.push(Span::styled(*key, key_style));
            spans.push(Span::styled(format!(" {}", desc), self.content_style));
        }

        // Pad to full width
        let content_width: usize = spans.iter().map(|s| s.width()).sum();
        let pad = (self.area.width as usize).saturating_sub(content_width);
        if pad > 0 {
            spans.push(Span::styled(" ".repeat(pad), self.content_style));
        }

        frame.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect::new(self.area.x, self.footer_y(), self.area.width, 1),
        );
    }

    /// Render a simple text footer (for custom footer content).
    pub fn render_footer(&self, frame: &mut Frame, spans: Vec<Span>) {
        let content_width: usize = spans.iter().map(|s| s.width()).sum();
        let mut all_spans = spans;
        let pad = (self.area.width as usize).saturating_sub(content_width);
        if pad > 0 {
            all_spans.push(Span::styled(" ".repeat(pad), self.content_style));
        }

        frame.render_widget(
            Paragraph::new(Line::from(all_spans)),
            Rect::new(self.area.x, self.footer_y(), self.area.width, 1),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_colors() -> ThemeColors {
        ThemeColors::default()
    }

    #[test]
    fn test_full_screen_view_dimensions() {
        let area = Rect::new(0, 0, 80, 25);
        let colors = test_colors();
        let view = FullScreenView::new(area, " TEST ", &colors);

        assert_eq!(view.title_y(), 0);
        assert_eq!(view.top_separator_y(), 1);
        assert_eq!(view.content_start_y(), 2);
        assert_eq!(view.bottom_separator_y(), 23); // 25 - 2
        assert_eq!(view.footer_y(), 24); // 25 - 1
        assert_eq!(view.content_height(), 21); // 25 - 4
    }

    #[test]
    fn test_content_area() {
        let area = Rect::new(0, 0, 80, 25);
        let colors = test_colors();
        let view = FullScreenView::new(area, " TEST ", &colors);

        let content = view.content_area();
        assert_eq!(content.x, 0);
        assert_eq!(content.y, 2);
        assert_eq!(content.width, 80);
        assert_eq!(content.height, 21);
    }
}
