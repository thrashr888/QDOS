//! Status Color Helpers
//!
//! Centralized color mapping for status indicators across plugins.
//!
//! # Example
//! ```ignore
//! use crate::ui::components::colors;
//!
//! let color = colors::git_status_color('M', &colors);  // Yellow for modified
//! let color = colors::priority_color(1, &colors);      // Red for P1
//! let color = colors::threshold_color(85.0, 50.0, 80.0, &colors);  // Red (> 80%)
//! ```

use crate::app::ThemeColors;
use ratatui::style::Color;

/// Get color for a git file status character.
///
/// | Status | Color | Meaning |
/// |--------|-------|---------|
/// | M | Yellow | Modified |
/// | A | Cyan | Added |
/// | D | Red | Deleted |
/// | R | Cyan | Renamed |
/// | C | Cyan | Copied |
/// | U | Magenta | Unmerged |
/// | ? | Magenta | Untracked |
/// | ! | Grey | Ignored |
pub fn git_status_color(status: char, colors: &ThemeColors) -> Color {
    match status {
        'M' => colors.yellow(),
        'A' | 'R' | 'C' => colors.cyan(),
        'D' => colors.red(),
        'U' => Color::Magenta,
        '?' => Color::Magenta,
        '!' => colors.grey(),
        _ => colors.fg(),
    }
}

/// Get color for an issue status string.
///
/// | Status | Color |
/// |--------|-------|
/// | open | White (fg) |
/// | in_progress | Cyan |
/// | blocked | Magenta |
/// | closed | Grey |
pub fn issue_status_color(status: &str, colors: &ThemeColors) -> Color {
    match status.to_lowercase().as_str() {
        "open" => colors.fg(),
        "in_progress" | "in-progress" => colors.cyan(),
        "blocked" => Color::Magenta,
        "closed" | "done" | "completed" => colors.grey(),
        _ => colors.fg(),
    }
}

/// Get color for an issue type string.
///
/// | Type | Color |
/// |------|-------|
/// | bug | Red |
/// | feature | Green |
/// | task | Blue |
/// | epic | Yellow |
pub fn issue_type_color(issue_type: &str, colors: &ThemeColors) -> Color {
    match issue_type.to_lowercase().as_str() {
        "bug" => colors.red(),
        "feature" => colors.green(),
        "task" => colors.blue(),
        "epic" => colors.yellow(),
        _ => colors.fg(),
    }
}

/// Get color for a priority level (0-4).
///
/// | Priority | Color | Meaning |
/// |----------|-------|---------|
/// | 0 | Red | Critical |
/// | 1 | Red | High |
/// | 2 | Yellow | Medium |
/// | 3 | White | Low |
/// | 4 | Grey | Backlog |
pub fn priority_color(priority: u8, colors: &ThemeColors) -> Color {
    match priority {
        0 | 1 => colors.red(),
        2 => colors.yellow(),
        3 => colors.fg(),
        _ => colors.grey(),
    }
}

/// Get color for a priority string (P0-P4 or named).
pub fn priority_str_color(priority: &str, colors: &ThemeColors) -> Color {
    let p = priority.to_uppercase();
    if p.starts_with("P0") || p.starts_with("P1") || p.contains("CRITICAL") || p.contains("HIGH") {
        colors.red()
    } else if p.starts_with("P2") || p.contains("MEDIUM") {
        colors.yellow()
    } else if p.starts_with("P3") || p.contains("LOW") {
        colors.fg()
    } else {
        colors.grey() // P4, backlog, etc.
    }
}

/// Get color based on a threshold value.
///
/// - `value >= critical` -> Red
/// - `value >= warning` -> Yellow
/// - Otherwise -> Green
///
/// Useful for CPU usage, memory, disk space, etc.
pub fn threshold_color(value: f32, warning: f32, critical: f32, colors: &ThemeColors) -> Color {
    if value >= critical {
        colors.red()
    } else if value >= warning {
        colors.yellow()
    } else {
        colors.green()
    }
}

/// Get color for a process status.
///
/// | Status | Color |
/// |--------|-------|
/// | running | Green |
/// | sleeping | Grey |
/// | stopped | Yellow |
/// | zombie | Red |
/// | dead | Red |
pub fn process_status_color(status: &str, colors: &ThemeColors) -> Color {
    match status.to_lowercase().as_str() {
        "running" | "run" => colors.green(),
        "sleeping" | "sleep" | "idle" => colors.grey(),
        "stopped" | "stop" => colors.yellow(),
        "zombie" | "dead" => colors.red(),
        _ => colors.fg(),
    }
}

/// Get color for a boolean/enabled state.
///
/// - true/enabled/yes -> Green
/// - false/disabled/no -> Grey
pub fn boolean_color(enabled: bool, colors: &ThemeColors) -> Color {
    if enabled {
        colors.green()
    } else {
        colors.grey()
    }
}

/// Get color for file size indicator (relative to threshold).
pub fn file_size_color(bytes: u64, large_threshold: u64, colors: &ThemeColors) -> Color {
    if bytes >= large_threshold {
        colors.yellow()
    } else {
        colors.fg()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ColorTheme;

    #[test]
    fn test_git_status_colors() {
        let colors = ColorTheme::Default.colors();

        assert_eq!(git_status_color('M', &colors), colors.yellow());
        assert_eq!(git_status_color('A', &colors), colors.cyan());
        assert_eq!(git_status_color('D', &colors), colors.red());
        assert_eq!(git_status_color('?', &colors), Color::Magenta);
    }

    #[test]
    fn test_priority_colors() {
        let colors = ColorTheme::Default.colors();

        assert_eq!(priority_color(0, &colors), colors.red());
        assert_eq!(priority_color(1, &colors), colors.red());
        assert_eq!(priority_color(2, &colors), colors.yellow());
        assert_eq!(priority_color(3, &colors), colors.fg());
        assert_eq!(priority_color(4, &colors), colors.grey());
    }

    #[test]
    fn test_threshold_colors() {
        let colors = ColorTheme::Default.colors();

        // CPU usage thresholds: warn at 50%, critical at 80%
        assert_eq!(threshold_color(30.0, 50.0, 80.0, &colors), colors.green());
        assert_eq!(threshold_color(60.0, 50.0, 80.0, &colors), colors.yellow());
        assert_eq!(threshold_color(90.0, 50.0, 80.0, &colors), colors.red());
    }
}
