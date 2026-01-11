//! Streamed log viewer component
//!
//! Provides a scrollable log viewer with auto-follow, status display,
//! and keyboard navigation.
//!
//! # Example
//!
//! ```ignore
//! use crate::ui::components::{LogViewer, LogViewerState, LogStatus};
//!
//! // Create state
//! let mut state = LogViewerState::new();
//!
//! // Add log lines (typically from a background process)
//! state.push_line("Starting build...");
//! state.push_line("Step 1/5: Compiling");
//!
//! // Create viewer and render
//! let viewer = LogViewer::new(&state)
//!     .title("Build Output")
//!     .status(LogStatus::Running);
//! viewer.render(frame, &view, &colors);
//!
//! // Handle keys
//! match key.code {
//!     KeyCode::Up => state.scroll_up(),
//!     KeyCode::Down => state.scroll_down(),
//!     KeyCode::PageUp => state.page_up(visible_height),
//!     KeyCode::PageDown => state.page_down(visible_height),
//!     KeyCode::Home => state.scroll_to_top(),
//!     KeyCode::End => state.scroll_to_bottom(),
//!     _ => {}
//! }
//! ```

use crate::app::ThemeColors;
use crate::ui::components::FullScreenView;
use ratatui::{style::Style, text::Span, Frame};

/// Log viewer status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogStatus {
    #[default]
    Idle,
    Running,
    Success,
    Failed,
}

impl LogStatus {
    /// Check if the operation is still running
    pub fn is_running(&self) -> bool {
        matches!(self, LogStatus::Running)
    }

    /// Check if the operation has completed (success or failure)
    pub fn is_done(&self) -> bool {
        matches!(self, LogStatus::Success | LogStatus::Failed)
    }

    /// Get display text for status
    pub fn text(&self) -> &'static str {
        match self {
            LogStatus::Idle => "Ready",
            LogStatus::Running => "Running...",
            LogStatus::Success => "Complete",
            LogStatus::Failed => "Failed",
        }
    }
}

/// State for log viewer
#[derive(Debug, Clone, Default)]
pub struct LogViewerState {
    /// Log lines
    pub lines: Vec<String>,
    /// Current scroll offset
    pub scroll: usize,
    /// Whether following (auto-scroll to bottom)
    pub following: bool,
    /// Visible height (for scroll calculations)
    visible_height: usize,
}

impl LogViewerState {
    /// Create new log viewer state
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            scroll: 0,
            following: true,
            visible_height: 18,
        }
    }

    /// Set visible height for scroll calculations
    pub fn with_visible_height(mut self, height: usize) -> Self {
        self.visible_height = height;
        self
    }

    /// Clear all lines
    pub fn clear(&mut self) {
        self.lines.clear();
        self.scroll = 0;
        self.following = true;
    }

    /// Add a line to the log
    pub fn push_line(&mut self, line: impl Into<String>) {
        self.lines.push(line.into());
        if self.following {
            self.scroll_to_bottom();
        }
    }

    /// Add multiple lines
    pub fn push_lines(&mut self, lines: impl IntoIterator<Item = impl Into<String>>) {
        for line in lines {
            self.lines.push(line.into());
        }
        if self.following {
            self.scroll_to_bottom();
        }
    }

    /// Scroll up one line (disables following)
    pub fn scroll_up(&mut self) {
        if self.scroll > 0 {
            self.scroll -= 1;
            self.following = false;
        }
    }

    /// Scroll down one line
    pub fn scroll_down(&mut self) {
        let max_scroll = self.max_scroll();
        if self.scroll < max_scroll {
            self.scroll += 1;
        }
        // Re-enable following if at bottom
        if self.scroll >= max_scroll {
            self.following = true;
        }
    }

    /// Page up (disables following)
    pub fn page_up(&mut self, page_size: usize) {
        self.scroll = self.scroll.saturating_sub(page_size);
        self.following = false;
    }

    /// Page down
    pub fn page_down(&mut self, page_size: usize) {
        let max_scroll = self.max_scroll();
        self.scroll = (self.scroll + page_size).min(max_scroll);
        if self.scroll >= max_scroll {
            self.following = true;
        }
    }

    /// Scroll to top (disables following)
    pub fn scroll_to_top(&mut self) {
        self.scroll = 0;
        self.following = false;
    }

    /// Scroll to bottom (enables following)
    pub fn scroll_to_bottom(&mut self) {
        self.scroll = self.max_scroll();
        self.following = true;
    }

    /// Get maximum scroll offset
    fn max_scroll(&self) -> usize {
        self.lines.len().saturating_sub(self.visible_height)
    }

    /// Get visible lines based on current scroll
    pub fn visible_lines(&self) -> impl Iterator<Item = &String> {
        let start = self.scroll;
        let end = (start + self.visible_height).min(self.lines.len());
        self.lines[start..end].iter()
    }

    /// Get scroll position info string
    pub fn scroll_info(&self) -> String {
        if self.lines.is_empty() {
            String::new()
        } else {
            format!(
                "[{}/{}]{}",
                (self.scroll + 1).min(self.lines.len()),
                self.lines.len(),
                if self.following { " ▼" } else { "" }
            )
        }
    }
}

/// Log line styling function type
pub type LineStyler = fn(&str, &ThemeColors) -> Style;

/// Default line styler with common patterns
pub fn default_line_style(line: &str, colors: &ThemeColors) -> Style {
    let lower = line.to_lowercase();
    if lower.contains("error") || lower.contains("failed") || lower.contains("fatal") {
        Style::default().fg(colors.red())
    } else if lower.contains("warning") || lower.contains("warn") {
        Style::default().fg(colors.yellow())
    } else if lower.contains("success")
        || lower.contains("complete")
        || lower.contains("done")
        || line.starts_with('+')
    {
        Style::default().fg(colors.green())
    } else if line.starts_with('-') || lower.contains("removed") || lower.contains("deleted") {
        Style::default().fg(colors.red())
    } else if line.starts_with('~') || lower.contains("modified") || lower.contains("changed") {
        Style::default().fg(colors.yellow())
    } else if lower.contains("downloading")
        || lower.contains("fetching")
        || lower.contains("loading")
    {
        Style::default().fg(colors.cyan())
    } else {
        Style::default().fg(colors.fg())
    }
}

/// Log viewer component
pub struct LogViewer<'a> {
    state: &'a LogViewerState,
    title: Option<&'a str>,
    status: LogStatus,
    status_label: Option<&'a str>,
    max_line_width: usize,
    line_styler: LineStyler,
    start_row: u16,
}

impl<'a> LogViewer<'a> {
    /// Create a new log viewer
    pub fn new(state: &'a LogViewerState) -> Self {
        Self {
            state,
            title: None,
            status: LogStatus::Idle,
            status_label: None,
            max_line_width: 78,
            line_styler: default_line_style,
            start_row: 1,
        }
    }

    /// Set title displayed in status line
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    /// Set current status
    pub fn status(mut self, status: LogStatus) -> Self {
        self.status = status;
        self
    }

    /// Set custom status label (overrides status text)
    pub fn status_label(mut self, label: &'a str) -> Self {
        self.status_label = Some(label);
        self
    }

    /// Set maximum line width for truncation
    pub fn max_line_width(mut self, width: usize) -> Self {
        self.max_line_width = width;
        self
    }

    /// Set custom line styler function
    pub fn line_styler(mut self, styler: LineStyler) -> Self {
        self.line_styler = styler;
        self
    }

    /// Set starting row for content (default: 1)
    pub fn start_row(mut self, row: u16) -> Self {
        self.start_row = row;
        self
    }

    /// Render the log viewer
    pub fn render(&self, frame: &mut Frame, view: &FullScreenView, colors: &ThemeColors) {
        // Status header
        let status_text = self.status_label.unwrap_or_else(|| self.status.text());
        let status_color = match self.status {
            LogStatus::Idle => colors.grey(),
            LogStatus::Running => colors.yellow(),
            LogStatus::Success => colors.green(),
            LogStatus::Failed => colors.red(),
        };

        let mut status_spans = vec![
            Span::styled("Status: ", Style::default().fg(colors.grey())),
            Span::styled(status_text, Style::default().fg(status_color)),
        ];

        if let Some(title) = self.title {
            status_spans.push(Span::styled(
                format!("  {}", title),
                Style::default().fg(colors.cyan()),
            ));
        }

        view.render_row(frame, self.start_row, status_spans);

        // Scroll indicator
        let scroll_info = self.state.scroll_info();
        view.render_row(
            frame,
            self.start_row + 1,
            vec![Span::styled(
                format!(" {}", scroll_info),
                Style::default().fg(colors.grey()),
            )],
        );

        // Log lines
        for (i, line) in self.state.visible_lines().enumerate() {
            let style = (self.line_styler)(line, colors);
            let display = if line.len() > self.max_line_width {
                format!("{:.width$}", line, width = self.max_line_width)
            } else {
                line.clone()
            };

            view.render_row(
                frame,
                self.start_row + 2 + i as u16,
                vec![Span::styled(display, style)],
            );
        }
    }

    /// Get help items based on current status
    pub fn help_items(&self) -> Vec<(&'static str, &'static str)> {
        if self.status.is_done() {
            vec![
                ("Tab", "next"),
                ("Enter", "done"),
                ("Esc", "close"),
                ("↑↓", "scroll"),
            ]
        } else {
            vec![("↑↓", "scroll"), ("PgUp/Dn", "page"), ("Home/End", "jump")]
        }
    }
}
