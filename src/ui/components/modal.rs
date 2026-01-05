//! Modal Frame Component
//!
//! Reusable modal frame with double-line borders and consistent styling.
//!
//! **Note**: This component is designed for small, centered dialog boxes (e.g., confirm dialogs,
//! input prompts). For full-screen plugin modals, use [`FullScreenView`](super::FullScreenView)
//! instead, which provides a cleaner layout with title bar and separators.

#[cfg(test)]
use crate::app::ColorTheme;
use crate::app::ThemeColors;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

/// A reusable modal frame with double-line borders.
///
/// Handles:
/// - Clearing the modal area
/// - Drawing the double-line border (╔═╗║╚╝╠╣)
/// - Drawing the title
/// - Providing content area coordinates
///
/// # Example
/// ```ignore
/// let colors = app.colors();
/// let modal = ModalFrame::themed(area, " MY TITLE ", &colors);
/// modal.render_frame(frame);
///
/// // Render content rows
/// modal.render_row(frame, 0, vec![Span::raw("Content line 1")]);
/// modal.render_row(frame, 1, vec![Span::raw("Content line 2")]);
///
/// // Render help row at bottom
/// modal.render_help(frame, vec![
///     ("Enter", "confirm"),
///     ("Esc", "cancel"),
/// ]);
/// ```
pub struct ModalFrame {
    /// The full modal area
    pub area: Rect,
    /// The title to display
    pub title: String,
    /// Border style
    pub border_style: Style,
    /// Title style
    pub title_style: Style,
    /// Normal content style
    pub content_style: Style,
    /// Help key color (for keyboard shortcuts in help row)
    pub help_key_color: Color,
    /// Whether to show a separator after the title
    pub show_title_separator: bool,
    /// Whether to show a separator before the help row
    pub show_footer_separator: bool,
    /// Inner width (area width minus borders)
    inner_width: usize,
}

impl ModalFrame {
    /// Create a new modal frame with theme colors.
    /// This is the preferred constructor for theme-aware modals.
    pub fn themed(area: Rect, title: &str, colors: &ThemeColors) -> Self {
        Self {
            area,
            title: title.to_string(),
            border_style: Style::default().fg(colors.fg()).bg(colors.bg()),
            title_style: Style::default()
                .fg(colors.yellow())
                .bg(colors.bg())
                .add_modifier(Modifier::BOLD),
            content_style: Style::default().fg(colors.fg()).bg(colors.bg()),
            help_key_color: colors.green(),
            show_title_separator: true,
            show_footer_separator: true,
            inner_width: area.width.saturating_sub(2) as usize,
        }
    }

    /// Set custom title style.
    pub fn title_style(mut self, style: Style) -> Self {
        self.title_style = style;
        self
    }

    /// Disable the title separator.
    pub fn no_title_separator(mut self) -> Self {
        self.show_title_separator = false;
        self
    }

    /// Disable the footer separator.
    pub fn no_footer_separator(mut self) -> Self {
        self.show_footer_separator = false;
        self
    }

    /// Calculate the Y position for a content row (0-indexed from after title).
    pub fn content_y(&self, row: u16) -> u16 {
        let base = self.area.y + 2; // After top border and title
        let offset = if self.show_title_separator { 1 } else { 0 };
        base + offset + row
    }

    /// Get the number of content rows available (excluding borders, title, separators, help).
    pub fn content_height(&self) -> u16 {
        let overhead = 2 // top + bottom borders
            + 1 // title row
            + if self.show_title_separator { 1 } else { 0 }
            + if self.show_footer_separator { 2 } else { 1 }; // separator + help or just help
        self.area.height.saturating_sub(overhead)
    }

    /// Get the content area as a Rect (inside borders, excluding title and help).
    pub fn content_area(&self) -> Rect {
        let y = self.content_y(0);
        let height = self.content_height();
        Rect::new(
            self.area.x + 2, // After left border and space
            y,
            self.area.width.saturating_sub(4), // Minus borders and padding
            height,
        )
    }

    /// Render the modal frame (borders, title, separators).
    /// Call this first, then render content rows.
    pub fn render_frame(&self, frame: &mut Frame) {
        // Clear the area
        frame.render_widget(Clear, self.area);

        let mut y = self.area.y;

        // Top border
        let top = format!("╔{}╗", "═".repeat(self.inner_width));
        frame.render_widget(
            Paragraph::new(Span::styled(&top, self.border_style)),
            Rect::new(self.area.x, y, self.area.width, 1),
        );
        y += 1;

        // Title row
        let title_pad = self.inner_width.saturating_sub(self.title.len());
        let title_line = vec![
            Span::styled("║", self.border_style),
            Span::styled(&self.title, self.title_style),
            Span::styled(" ".repeat(title_pad), self.content_style),
            Span::styled("║", self.border_style),
        ];
        frame.render_widget(
            Paragraph::new(Line::from(title_line)),
            Rect::new(self.area.x, y, self.area.width, 1),
        );
        y += 1;

        // Title separator
        if self.show_title_separator {
            let sep = format!("╠{}╣", "═".repeat(self.inner_width));
            frame.render_widget(
                Paragraph::new(Span::styled(&sep, self.border_style)),
                Rect::new(self.area.x, y, self.area.width, 1),
            );
        }

        // Fill middle rows with empty content (will be overwritten by render_row)
        let content_start = self.content_y(0);
        let footer_y = self.footer_y();
        for row_y in content_start..footer_y {
            self.render_empty_row(frame, row_y);
        }

        // Footer separator
        if self.show_footer_separator {
            let sep = format!("╠{}╣", "═".repeat(self.inner_width));
            frame.render_widget(
                Paragraph::new(Span::styled(&sep, self.border_style)),
                Rect::new(self.area.x, footer_y, self.area.width, 1),
            );
        }

        // Help row placeholder (empty, to be filled by render_help)
        self.render_empty_row(
            frame,
            footer_y + if self.show_footer_separator { 1 } else { 0 },
        );

        // Bottom border
        let bottom = format!("╚{}╝", "═".repeat(self.inner_width));
        frame.render_widget(
            Paragraph::new(Span::styled(&bottom, self.border_style)),
            Rect::new(
                self.area.x,
                self.area.y + self.area.height - 1,
                self.area.width,
                1,
            ),
        );
    }

    /// Get the Y position for the footer separator.
    fn footer_y(&self) -> u16 {
        self.area.y + self.area.height - 3
    }

    /// Render an empty row with borders.
    fn render_empty_row(&self, frame: &mut Frame, y: u16) {
        let fill = " ".repeat(self.inner_width);
        let line = vec![
            Span::styled("║", self.border_style),
            Span::styled(fill, self.content_style),
            Span::styled("║", self.border_style),
        ];
        frame.render_widget(
            Paragraph::new(Line::from(line)),
            Rect::new(self.area.x, y, self.area.width, 1),
        );
    }

    /// Render a content row at the given index (0-indexed from after title separator).
    pub fn render_row(&self, frame: &mut Frame, row: u16, content: Vec<Span>) {
        let y = self.content_y(row);
        self.render_row_at(frame, y, content);
    }

    /// Render a content row at an absolute Y position.
    pub fn render_row_at(&self, frame: &mut Frame, y: u16, content: Vec<Span>) {
        let mut spans = vec![Span::styled("║ ", self.border_style)];
        spans.extend(content);

        // Calculate padding needed
        let content_width: usize = spans.iter().map(|s| s.width()).sum();
        let pad = (self.area.width as usize).saturating_sub(content_width + 1);
        spans.push(Span::styled(" ".repeat(pad), self.content_style));
        spans.push(Span::styled("║", self.border_style));

        frame.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect::new(self.area.x, y, self.area.width, 1),
        );
    }

    /// Render the help row at the bottom with key hints.
    /// Takes pairs of (key, description).
    pub fn render_help(&self, frame: &mut Frame, hints: Vec<(&str, &str)>) {
        let y = self.area.y + self.area.height - 2;
        // Use help_key_color with the same background as content
        let key_style = self.content_style.fg(self.help_key_color);

        let mut spans: Vec<Span> = Vec::new();
        for (i, (key, desc)) in hints.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled("  ", self.content_style));
            }
            spans.push(Span::styled(*key, key_style));
            spans.push(Span::styled(format!(" {}", desc), self.content_style));
        }

        self.render_row_at(frame, y, spans);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modal_frame_dimensions() {
        let area = Rect::new(10, 5, 40, 12);
        let colors = ColorTheme::Default.colors();
        let modal = ModalFrame::themed(area, " TEST ", &colors);

        assert_eq!(modal.area, area);
        assert_eq!(modal.content_y(0), 8); // y + 2 (top+title) + 1 (separator)
        assert!(modal.content_height() > 0);
    }
}
