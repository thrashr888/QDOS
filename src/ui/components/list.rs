//! Scrollable List Component
//!
//! A reusable scrollable list widget with selection highlighting.
//!
//! # Example
//! ```ignore
//! let items = vec!["Item 1", "Item 2", "Item 3"];
//! let list = ScrollableList::new(&items, selected_index, visible_height);
//! list.render(frame, area, colors, |item, is_selected, style| {
//!     vec![Span::styled(item.to_string(), style)]
//! });
//! ```

use crate::app::ThemeColors;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// A scrollable list with selection highlighting.
///
/// Handles:
/// - Automatic scroll offset to keep selection visible
/// - Selection highlighting (yellow on red by default)
/// - Item rendering via callback
/// - Scroll position indicator
pub struct ScrollableList<'a, T> {
    /// The items to display
    items: &'a [T],
    /// Currently selected index
    selected: usize,
    /// Number of visible rows
    visible_height: usize,
    /// Whether to show scroll indicator
    show_scroll_indicator: bool,
    /// Left padding for items
    left_padding: u16,
}

impl<'a, T> ScrollableList<'a, T> {
    /// Create a new scrollable list.
    pub fn new(items: &'a [T], selected: usize, visible_height: usize) -> Self {
        Self {
            items,
            selected: selected.min(items.len().saturating_sub(1)),
            visible_height,
            show_scroll_indicator: true,
            left_padding: 1,
        }
    }

    /// Hide the scroll indicator.
    pub fn no_scroll_indicator(mut self) -> Self {
        self.show_scroll_indicator = false;
        self
    }

    /// Set left padding for items.
    pub fn left_padding(mut self, padding: u16) -> Self {
        self.left_padding = padding;
        self
    }

    /// Calculate the scroll offset to keep selection visible.
    pub fn scroll_offset(&self) -> usize {
        if self.items.is_empty() || self.visible_height == 0 {
            return 0;
        }

        if self.selected >= self.visible_height {
            self.selected - self.visible_height + 1
        } else {
            0
        }
    }

    /// Get the visible items with their indices.
    pub fn visible_items(&self) -> impl Iterator<Item = (usize, &T)> {
        let offset = self.scroll_offset();
        self.items
            .iter()
            .enumerate()
            .skip(offset)
            .take(self.visible_height)
    }

    /// Check if an index is currently selected.
    pub fn is_selected(&self, index: usize) -> bool {
        index == self.selected
    }

    /// Get the selection style (yellow on red).
    pub fn selection_style(&self, colors: &ThemeColors) -> Style {
        Style::default().fg(colors.yellow()).bg(colors.red())
    }

    /// Get the normal style.
    pub fn normal_style(&self, colors: &ThemeColors) -> Style {
        Style::default().fg(colors.fg()).bg(colors.bg())
    }

    /// Get the appropriate style for an item.
    pub fn item_style(&self, index: usize, colors: &ThemeColors) -> Style {
        if self.is_selected(index) {
            self.selection_style(colors)
        } else {
            self.normal_style(colors)
        }
    }

    /// Render the list.
    ///
    /// The `render_item` callback receives:
    /// - The item
    /// - Whether it's selected
    /// - The base style to use
    ///
    /// It should return a Vec of Spans for the line.
    pub fn render<F>(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors, render_item: F)
    where
        F: Fn(&T, bool, Style) -> Vec<Span<'static>>,
    {
        if self.items.is_empty() {
            return;
        }

        let offset = self.scroll_offset();

        for (display_row, (index, item)) in self.visible_items().enumerate() {
            let is_selected = self.is_selected(index);
            let style = self.item_style(index, colors);

            // Get spans from callback
            let mut spans = render_item(item, is_selected, style);

            // Add left padding
            if self.left_padding > 0 {
                spans.insert(
                    0,
                    Span::styled(" ".repeat(self.left_padding as usize), style),
                );
            }

            // Calculate row position
            let y = area.y + display_row as u16;

            // Pad to full width
            let content_width: usize = spans.iter().map(|s| s.width()).sum();
            let pad_width = (area.width as usize).saturating_sub(content_width);
            if pad_width > 0 {
                spans.push(Span::styled(" ".repeat(pad_width), style));
            }

            frame.render_widget(
                Paragraph::new(Line::from(spans)),
                Rect::new(area.x, y, area.width, 1),
            );
        }

        // Render scroll indicator if needed
        if self.show_scroll_indicator && self.items.len() > self.visible_height {
            self.render_scroll_indicator(frame, area, colors, offset);
        }
    }

    /// Render a scroll position indicator.
    fn render_scroll_indicator(
        &self,
        frame: &mut Frame,
        area: Rect,
        colors: &ThemeColors,
        _offset: usize,
    ) {
        let total = self.items.len();
        let indicator = format!(" [{}/{}] ", self.selected + 1, total);
        let indicator_len = indicator.len() as u16;

        // Position at bottom-right of area
        let x = area.x + area.width.saturating_sub(indicator_len);
        let y = area.y + area.height.saturating_sub(1);

        frame.render_widget(
            Paragraph::new(Span::styled(
                indicator,
                Style::default().fg(colors.grey()).bg(colors.bg()),
            )),
            Rect::new(x, y, indicator_len, 1),
        );
    }
}

/// State management for a scrollable list.
///
/// Use this to track selection state and handle navigation keys.
#[derive(Debug, Clone, Default)]
pub struct ListState {
    /// Currently selected index
    pub selected: usize,
    /// Total number of items
    pub len: usize,
}

impl ListState {
    /// Create a new list state.
    pub fn new(len: usize) -> Self {
        Self { selected: 0, len }
    }

    /// Update the length and clamp selection.
    pub fn set_len(&mut self, len: usize) {
        self.len = len;
        if self.selected >= len && len > 0 {
            self.selected = len - 1;
        }
    }

    /// Select the next item.
    pub fn select_next(&mut self) {
        if self.len > 0 {
            self.selected = (self.selected + 1).min(self.len - 1);
        }
    }

    /// Select the previous item.
    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// Select the first item.
    pub fn select_first(&mut self) {
        self.selected = 0;
    }

    /// Select the last item.
    pub fn select_last(&mut self) {
        if self.len > 0 {
            self.selected = self.len - 1;
        }
    }

    /// Move selection down by a page.
    pub fn page_down(&mut self, page_size: usize) {
        if self.len > 0 {
            self.selected = (self.selected + page_size).min(self.len - 1);
        }
    }

    /// Move selection up by a page.
    pub fn page_up(&mut self, page_size: usize) {
        self.selected = self.selected.saturating_sub(page_size);
    }
}

/// Helper to truncate text with ellipsis.
pub fn truncate_with_ellipsis(text: &str, max_width: usize) -> String {
    if text.len() <= max_width {
        text.to_string()
    } else if max_width <= 3 {
        ".".repeat(max_width)
    } else {
        format!("{}...", &text[..max_width - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scroll_offset() {
        let items = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        // Selection at top, no scroll
        let list = ScrollableList::new(&items, 0, 5);
        assert_eq!(list.scroll_offset(), 0);

        // Selection in middle, no scroll yet
        let list = ScrollableList::new(&items, 4, 5);
        assert_eq!(list.scroll_offset(), 0);

        // Selection past visible, should scroll
        let list = ScrollableList::new(&items, 5, 5);
        assert_eq!(list.scroll_offset(), 1);

        // Selection at end
        let list = ScrollableList::new(&items, 9, 5);
        assert_eq!(list.scroll_offset(), 5);
    }

    #[test]
    fn test_list_state_navigation() {
        let mut state = ListState::new(10);

        assert_eq!(state.selected, 0);

        state.select_next();
        assert_eq!(state.selected, 1);

        state.select_prev();
        assert_eq!(state.selected, 0);

        state.select_prev(); // Should not go negative
        assert_eq!(state.selected, 0);

        state.select_last();
        assert_eq!(state.selected, 9);

        state.select_next(); // Should not exceed len
        assert_eq!(state.selected, 9);

        state.select_first();
        assert_eq!(state.selected, 0);

        state.page_down(3);
        assert_eq!(state.selected, 3);

        state.page_up(2);
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn test_truncate_with_ellipsis() {
        assert_eq!(truncate_with_ellipsis("hello", 10), "hello");
        assert_eq!(truncate_with_ellipsis("hello world", 8), "hello...");
        assert_eq!(truncate_with_ellipsis("hi", 2), "hi");
        assert_eq!(truncate_with_ellipsis("hello", 3), "...");
    }
}
