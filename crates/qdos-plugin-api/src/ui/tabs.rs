//! Tab bar component for plugin navigation
//!
//! Provides a horizontal tab bar with keyboard navigation.

use crate::ThemeColors;
use ratatui::{
    style::{Modifier, Style},
    text::Span,
};

/// A horizontal tab bar component
pub struct TabBar<'a> {
    /// Tab labels
    tabs: &'a [&'a str],
    /// Currently selected tab index
    selected: usize,
    /// Separator between tabs
    separator: &'a str,
}

impl<'a> TabBar<'a> {
    /// Create a new tab bar
    pub fn new(tabs: &'a [&'a str], selected: usize) -> Self {
        Self {
            tabs,
            selected,
            separator: " | ",
        }
    }

    /// Set custom separator (default: " | ")
    pub fn separator(mut self, sep: &'a str) -> Self {
        self.separator = sep;
        self
    }

    /// Render the tab bar as spans
    pub fn render(&self, colors: &ThemeColors) -> Vec<Span<'a>> {
        let mut spans = Vec::new();

        for (i, tab) in self.tabs.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(
                    self.separator,
                    Style::default().fg(colors.grey()),
                ));
            }

            let style = if i == self.selected {
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.grey())
            };

            spans.push(Span::styled(*tab, style));
        }

        spans
    }

    /// Get next tab index (wrapping)
    pub fn next_index(&self) -> usize {
        if self.tabs.is_empty() {
            0
        } else {
            (self.selected + 1) % self.tabs.len()
        }
    }

    /// Get previous tab index (wrapping)
    pub fn prev_index(&self) -> usize {
        if self.tabs.is_empty() {
            0
        } else if self.selected == 0 {
            self.tabs.len() - 1
        } else {
            self.selected - 1
        }
    }
}

/// State management for tab navigation
#[derive(Debug, Clone, Default)]
pub struct TabState {
    /// Current selected tab index
    pub selected: usize,
    /// Total number of tabs
    pub count: usize,
}

impl TabState {
    /// Create new tab state
    pub fn new(count: usize) -> Self {
        Self { selected: 0, count }
    }

    /// Move to next tab
    pub fn next(&mut self) {
        if self.count > 0 {
            self.selected = (self.selected + 1) % self.count;
        }
    }

    /// Move to previous tab
    pub fn prev(&mut self) {
        if self.count > 0 {
            self.selected = if self.selected == 0 {
                self.count - 1
            } else {
                self.selected - 1
            };
        }
    }

    /// Set selected tab directly
    pub fn select(&mut self, index: usize) {
        if index < self.count {
            self.selected = index;
        }
    }
}
