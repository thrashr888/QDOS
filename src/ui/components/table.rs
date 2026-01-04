//! Table Component
//!
//! A reusable table widget with column specifications and alignment.
//!
//! # Example
//! ```ignore
//! let table = Table::new(vec![
//!     Column::new("ID", 10).left(),
//!     Column::new("Name", 20).left(),
//!     Column::new("Size", 12).right(),
//! ]);
//!
//! table.render_header(frame, y, colors);
//! for (i, item) in items.iter().enumerate() {
//!     let is_selected = i == selected;
//!     table.render_row(frame, y + 1 + i as u16, &[&item.id, &item.name, &item.size], is_selected, colors);
//! }
//! ```

use crate::app::ThemeColors;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Column alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    #[default]
    Left,
    Right,
    Center,
}

/// A column specification.
#[derive(Debug, Clone)]
pub struct Column {
    /// Column header name
    pub name: String,
    /// Column width in characters
    pub width: usize,
    /// Text alignment
    pub align: Align,
}

impl Column {
    /// Create a new left-aligned column.
    pub fn new(name: &str, width: usize) -> Self {
        Self {
            name: name.to_string(),
            width,
            align: Align::Left,
        }
    }

    /// Set left alignment.
    pub fn left(mut self) -> Self {
        self.align = Align::Left;
        self
    }

    /// Set right alignment.
    pub fn right(mut self) -> Self {
        self.align = Align::Right;
        self
    }

    /// Set center alignment.
    pub fn center(mut self) -> Self {
        self.align = Align::Center;
        self
    }

    /// Format a value according to this column's width and alignment.
    pub fn format(&self, value: &str) -> String {
        let truncated = if value.len() > self.width {
            if self.width <= 3 {
                ".".repeat(self.width)
            } else {
                format!("{}...", &value[..self.width - 3])
            }
        } else {
            value.to_string()
        };

        match self.align {
            Align::Left => format!("{:<width$}", truncated, width = self.width),
            Align::Right => format!("{:>width$}", truncated, width = self.width),
            Align::Center => format!("{:^width$}", truncated, width = self.width),
        }
    }
}

/// A table with columns.
#[derive(Debug, Clone)]
pub struct Table {
    /// Column specifications
    columns: Vec<Column>,
    /// Left margin before first column
    left_margin: usize,
    /// Separator between columns
    separator: String,
}

impl Table {
    /// Create a new table with the given columns.
    pub fn new(columns: Vec<Column>) -> Self {
        Self {
            columns,
            left_margin: 1,
            separator: " ".to_string(),
        }
    }

    /// Set the left margin.
    pub fn left_margin(mut self, margin: usize) -> Self {
        self.left_margin = margin;
        self
    }

    /// Set the column separator.
    pub fn separator(mut self, sep: &str) -> Self {
        self.separator = sep.to_string();
        self
    }

    /// Get the total width of the table.
    pub fn total_width(&self) -> usize {
        self.left_margin
            + self.columns.iter().map(|c| c.width).sum::<usize>()
            + self.separator.len() * self.columns.len().saturating_sub(1)
    }

    /// Render the header row.
    pub fn render_header(&self, frame: &mut Frame, area: Rect, y: u16, colors: &ThemeColors) {
        let header_style = Style::default().fg(colors.blue()).bg(colors.bg());

        let mut spans: Vec<Span> = Vec::new();

        // Left margin
        if self.left_margin > 0 {
            spans.push(Span::styled(" ".repeat(self.left_margin), header_style));
        }

        // Column headers
        for (i, col) in self.columns.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(self.separator.clone(), header_style));
            }
            spans.push(Span::styled(col.format(&col.name), header_style));
        }

        // Pad to full width
        let content_width: usize = spans.iter().map(|s| s.width()).sum();
        let pad = (area.width as usize).saturating_sub(content_width);
        if pad > 0 {
            spans.push(Span::styled(" ".repeat(pad), header_style));
        }

        frame.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect::new(area.x, y, area.width, 1),
        );
    }

    /// Render a data row.
    pub fn render_row(
        &self,
        frame: &mut Frame,
        area: Rect,
        y: u16,
        values: &[&str],
        is_selected: bool,
        colors: &ThemeColors,
    ) {
        let style = if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            Style::default().fg(colors.fg()).bg(colors.bg())
        };

        self.render_row_styled(frame, area, y, values, style);
    }

    /// Render a row with custom style.
    pub fn render_row_styled(
        &self,
        frame: &mut Frame,
        area: Rect,
        y: u16,
        values: &[&str],
        style: Style,
    ) {
        let mut spans: Vec<Span> = Vec::new();

        // Left margin
        if self.left_margin > 0 {
            spans.push(Span::styled(" ".repeat(self.left_margin), style));
        }

        // Column values
        for (i, col) in self.columns.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(self.separator.clone(), style));
            }
            let value = values.get(i).copied().unwrap_or("");
            spans.push(Span::styled(col.format(value), style));
        }

        // Pad to full width
        let content_width: usize = spans.iter().map(|s| s.width()).sum();
        let pad = (area.width as usize).saturating_sub(content_width);
        if pad > 0 {
            spans.push(Span::styled(" ".repeat(pad), style));
        }

        frame.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect::new(area.x, y, area.width, 1),
        );
    }

    /// Render a row with per-column styles.
    pub fn render_row_with_styles(
        &self,
        frame: &mut Frame,
        area: Rect,
        y: u16,
        values: &[(String, Style)],
        base_style: Style,
    ) {
        let mut spans: Vec<Span> = Vec::new();

        // Left margin
        if self.left_margin > 0 {
            spans.push(Span::styled(" ".repeat(self.left_margin), base_style));
        }

        // Column values with individual styles
        for (i, col) in self.columns.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(self.separator.clone(), base_style));
            }
            let (value, style) = values
                .get(i)
                .map(|(v, s)| (v.as_str(), *s))
                .unwrap_or(("", base_style));
            spans.push(Span::styled(col.format(value), style));
        }

        // Pad to full width
        let content_width: usize = spans.iter().map(|s| s.width()).sum();
        let pad = (area.width as usize).saturating_sub(content_width);
        if pad > 0 {
            spans.push(Span::styled(" ".repeat(pad), base_style));
        }

        frame.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect::new(area.x, y, area.width, 1),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_format_left() {
        let col = Column::new("Name", 10).left();
        assert_eq!(col.format("hello"), "hello     ");
        assert_eq!(col.format("hello world"), "hello w...");
    }

    #[test]
    fn test_column_format_right() {
        let col = Column::new("Size", 8).right();
        assert_eq!(col.format("1234"), "    1234");
        assert_eq!(col.format("123456789"), "12345...");
    }

    #[test]
    fn test_column_format_center() {
        let col = Column::new("Status", 10).center();
        assert_eq!(col.format("open"), "   open   ");
    }

    #[test]
    fn test_table_total_width() {
        let table = Table::new(vec![
            Column::new("A", 10),
            Column::new("B", 8),
            Column::new("C", 12),
        ]);
        // 1 (margin) + 10 + 1 (sep) + 8 + 1 (sep) + 12 = 33
        assert_eq!(table.total_width(), 33);
    }
}
