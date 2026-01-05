//! Q-DOS style progress indicator components
//!
//! Provides various progress display styles for file operations.

use crate::app::ThemeColors;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Progress bar display style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProgressStyle {
    /// Arrow-style: `Copying FILE.TXT ====> dest`
    Arrow,
    /// Bar-style: `[████████░░░░] 66%`
    #[default]
    Bar,
    /// Indeterminate spinner: `|/-\`
    Spinner,
}

/// Q-DOS style progress indicator
///
/// # Example
/// ```ignore
/// use crate::ui::components::{ProgressBar, ProgressStyle};
///
/// // Bar-style progress
/// let progress = ProgressBar::new(0.66)
///     .message("Copying files...")
///     .style(ProgressStyle::Bar);
/// progress.render(frame, area, &colors);
///
/// // Arrow-style for file operations
/// let progress = ProgressBar::arrow("FILE.TXT", "dest/")
///     .progress(0.5);
/// progress.render(frame, area, &colors);
///
/// // Spinner for indeterminate operations
/// let mut spinner = ProgressBar::spinner("Loading...");
/// spinner.tick(); // Advance spinner animation
/// spinner.render(frame, area, &colors);
/// ```
pub struct ProgressBar {
    /// Progress value (0.0 to 1.0)
    progress: f32,
    /// Message to display
    message: String,
    /// Display style
    style: ProgressStyle,
    /// Source item (for arrow style)
    source: Option<String>,
    /// Destination (for arrow style)
    dest: Option<String>,
    /// Spinner frame (0-3)
    spinner_frame: usize,
    /// Bar width in characters
    bar_width: usize,
    /// Show percentage
    show_percent: bool,
}

impl Default for ProgressBar {
    fn default() -> Self {
        Self {
            progress: 0.0,
            message: String::new(),
            style: ProgressStyle::Bar,
            source: None,
            dest: None,
            spinner_frame: 0,
            bar_width: 40,
            show_percent: true,
        }
    }
}

impl ProgressBar {
    /// Create a new progress bar with given progress (0.0 to 1.0)
    pub fn new(progress: f32) -> Self {
        Self {
            progress: progress.clamp(0.0, 1.0),
            ..Default::default()
        }
    }

    /// Create an arrow-style progress bar for file operations
    pub fn arrow(source: &str, dest: &str) -> Self {
        Self {
            style: ProgressStyle::Arrow,
            source: Some(source.to_string()),
            dest: Some(dest.to_string()),
            ..Default::default()
        }
    }

    /// Create an indeterminate spinner
    pub fn spinner(message: &str) -> Self {
        Self {
            style: ProgressStyle::Spinner,
            message: message.to_string(),
            ..Default::default()
        }
    }

    /// Set the progress value (0.0 to 1.0)
    pub fn progress(mut self, progress: f32) -> Self {
        self.progress = progress.clamp(0.0, 1.0);
        self
    }

    /// Set the display message
    pub fn message(mut self, message: &str) -> Self {
        self.message = message.to_string();
        self
    }

    /// Set the display style
    pub fn style(mut self, style: ProgressStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the bar width in characters
    pub fn bar_width(mut self, width: usize) -> Self {
        self.bar_width = width;
        self
    }

    /// Hide percentage display
    pub fn hide_percent(mut self) -> Self {
        self.show_percent = false;
        self
    }

    /// Advance the spinner animation
    pub fn tick(&mut self) {
        self.spinner_frame = (self.spinner_frame + 1) % 4;
    }

    /// Get current progress as percentage (0-100)
    pub fn percentage(&self) -> usize {
        (self.progress * 100.0) as usize
    }

    /// Render the progress bar
    pub fn render(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        match self.style {
            ProgressStyle::Arrow => self.render_arrow(frame, area, colors),
            ProgressStyle::Bar => self.render_bar(frame, area, colors),
            ProgressStyle::Spinner => self.render_spinner(frame, area, colors),
        }
    }

    /// Render arrow-style progress: `Copying FILE.TXT ====> dest`
    fn render_arrow(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        let source = self.source.as_deref().unwrap_or("file");
        let dest = self.dest.as_deref().unwrap_or("dest");

        // Calculate arrow length based on progress
        let max_arrows = 10;
        let arrows = ((self.progress * max_arrows as f32) as usize).min(max_arrows);
        let arrow_str = format!("{}>{}", "=".repeat(arrows), " ".repeat(max_arrows - arrows));

        let line = Line::from(vec![
            Span::styled(&self.message, Style::default().fg(colors.fg())),
            Span::styled(" ", Style::default()),
            Span::styled(source, Style::default().fg(colors.yellow())),
            Span::styled(" ", Style::default()),
            Span::styled(arrow_str, Style::default().fg(colors.blue())),
            Span::styled(" ", Style::default()),
            Span::styled(dest, Style::default().fg(colors.green())),
        ]);

        frame.render_widget(Paragraph::new(line), area);
    }

    /// Render bar-style progress: `[████████░░░░] 66%`
    fn render_bar(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        let filled = ((self.bar_width as f32 * self.progress) as usize).min(self.bar_width);
        let empty = self.bar_width - filled;

        let bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(empty));

        let mut spans = vec![Span::styled(bar, Style::default().fg(colors.blue()))];

        if self.show_percent {
            spans.push(Span::styled(
                format!(" {}%", self.percentage()),
                Style::default().fg(colors.green()),
            ));
        }

        if !self.message.is_empty() {
            spans.insert(
                0,
                Span::styled(
                    format!("{} ", self.message),
                    Style::default().fg(colors.fg()),
                ),
            );
        }

        frame.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    /// Render spinner: `Loading... |`
    fn render_spinner(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        let spinner_chars = ['|', '/', '-', '\\'];
        let spinner = spinner_chars[self.spinner_frame % 4];

        let line = Line::from(vec![
            Span::styled(&self.message, Style::default().fg(colors.fg())),
            Span::styled(" ", Style::default()),
            Span::styled(spinner.to_string(), Style::default().fg(colors.yellow())),
        ]);

        frame.render_widget(Paragraph::new(line), area);
    }

    /// Render just the bar portion (for embedding in other components)
    pub fn render_bar_only(&self, colors: &ThemeColors) -> Line<'static> {
        let filled = ((self.bar_width as f32 * self.progress) as usize).min(self.bar_width);
        let empty = self.bar_width - filled;

        let bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(empty));

        let mut spans = vec![Span::styled(bar, Style::default().fg(colors.blue()))];

        if self.show_percent {
            spans.push(Span::styled(
                format!(" {}%", self.percentage()),
                Style::default().fg(colors.green()),
            ));
        }

        Line::from(spans)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_bar_new() {
        let bar = ProgressBar::new(0.5);
        assert_eq!(bar.percentage(), 50);
    }

    #[test]
    fn test_progress_clamp() {
        let bar = ProgressBar::new(1.5);
        assert_eq!(bar.percentage(), 100);

        let bar = ProgressBar::new(-0.5);
        assert_eq!(bar.percentage(), 0);
    }

    #[test]
    fn test_arrow_style() {
        let bar = ProgressBar::arrow("file.txt", "/dest/");
        assert_eq!(bar.style, ProgressStyle::Arrow);
        assert_eq!(bar.source, Some("file.txt".to_string()));
        assert_eq!(bar.dest, Some("/dest/".to_string()));
    }

    #[test]
    fn test_spinner_tick() {
        let mut spinner = ProgressBar::spinner("Loading");
        assert_eq!(spinner.spinner_frame, 0);
        spinner.tick();
        assert_eq!(spinner.spinner_frame, 1);
        spinner.tick();
        spinner.tick();
        spinner.tick();
        assert_eq!(spinner.spinner_frame, 0); // Wraps around
    }

    #[test]
    fn test_builder_pattern() {
        let bar = ProgressBar::new(0.75)
            .message("Copying")
            .bar_width(20)
            .hide_percent();

        assert_eq!(bar.percentage(), 75);
        assert_eq!(bar.message, "Copying");
        assert_eq!(bar.bar_width, 20);
        assert!(!bar.show_percent);
    }
}
