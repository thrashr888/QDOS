//! Cloud Storage UI Components
//!
//! Shared UI components for rendering cloud storage status and information.

use super::state::{CloudFileEntry, StorageInfo, SyncStatus};
use crate::app::ThemeColors;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge},
    Frame,
};

/// Render a storage usage bar
pub fn render_storage_bar(
    frame: &mut Frame,
    area: Rect,
    info: &StorageInfo,
    colors: &ThemeColors,
    title: &str,
) {
    let percent = info.usage_percent().unwrap_or(0);

    // Choose color based on usage
    let bar_color = if percent >= 90 {
        colors.red()
    } else if percent >= 75 {
        colors.yellow()
    } else {
        colors.green()
    };

    let label = match (info.used_bytes, info.total_bytes) {
        (Some(used), Some(total)) => {
            format!(
                "{} / {} ({}%)",
                StorageInfo::format_bytes(used),
                StorageInfo::format_bytes(total),
                percent
            )
        }
        _ => "Unknown".to_string(),
    };

    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::NONE)
                .title(format!(" {} ", title)),
        )
        .gauge_style(Style::default().fg(bar_color).bg(colors.bg()))
        .percent(percent as u16)
        .label(label);

    frame.render_widget(gauge, area);
}

/// Get the color for a sync status
pub fn status_color(status: SyncStatus, colors: &ThemeColors) -> Color {
    match status {
        SyncStatus::Synced => colors.green(),
        SyncStatus::Syncing => colors.cyan(),
        SyncStatus::Pending => colors.yellow(),
        SyncStatus::Error => colors.red(),
        SyncStatus::CloudOnly => colors.blue(),
        SyncStatus::Offline => colors.grey(),
        SyncStatus::Excluded => colors.grey(),
        SyncStatus::Unknown => colors.grey(),
    }
}

/// Create a styled span for a sync status indicator
pub fn status_span(status: SyncStatus, colors: &ThemeColors, use_ascii: bool) -> Span<'static> {
    let indicator = if use_ascii {
        status.ascii_indicator()
    } else {
        status.indicator()
    };
    let color = status_color(status, colors);
    Span::styled(format!("{}", indicator), Style::default().fg(color))
}

/// Create a line with file name and sync status
pub fn file_line_with_status(
    entry: &CloudFileEntry,
    colors: &ThemeColors,
    selected: bool,
    use_ascii: bool,
) -> Line<'static> {
    let status_indicator = status_span(entry.status, colors, use_ascii);

    let name_style = if selected {
        Style::default().fg(colors.yellow()).bg(colors.red())
    } else if entry.is_dir {
        Style::default().fg(colors.blue())
    } else {
        Style::default().fg(colors.fg())
    };

    let shared_indicator = if entry.is_shared {
        Span::styled(" [shared]", Style::default().fg(colors.cyan()))
    } else {
        Span::raw("")
    };

    let size_str = entry
        .size
        .map(|s| format!(" {:>8}", StorageInfo::format_bytes(s)))
        .unwrap_or_default();

    Line::from(vec![
        status_indicator,
        Span::raw(" "),
        Span::styled(entry.name.clone(), name_style),
        shared_indicator,
        Span::styled(size_str, Style::default().fg(colors.grey())),
    ])
}

/// Render a sync status legend
pub fn render_status_legend(frame: &mut Frame, area: Rect, colors: &ThemeColors, use_ascii: bool) {
    let statuses = [
        (SyncStatus::Synced, "Synced"),
        (SyncStatus::Syncing, "Syncing"),
        (SyncStatus::Pending, "Pending"),
        (SyncStatus::CloudOnly, "Cloud"),
        (SyncStatus::Error, "Error"),
    ];

    let spans: Vec<Span> = statuses
        .iter()
        .flat_map(|(status, label)| {
            vec![
                status_span(*status, colors, use_ascii),
                Span::styled(format!("={} ", label), Style::default().fg(colors.grey())),
            ]
        })
        .collect();

    let line = Line::from(spans);
    frame.render_widget(
        ratatui::widgets::Paragraph::new(line).style(Style::default()),
        area,
    );
}

/// Format account info for display
pub fn format_account_info(info: &StorageInfo) -> String {
    match &info.account {
        Some(account) => {
            if info.connected {
                format!("Connected: {}", account)
            } else {
                format!("Disconnected: {}", account)
            }
        }
        None => {
            if info.connected {
                "Connected".to_string()
            } else {
                "Not connected".to_string()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_colors() -> ThemeColors {
        ThemeColors {
            background: (0, 0, 0),
            foreground: (255, 255, 255),
            blue: (0, 0, 170),
            green: (0, 170, 0),
            red: (170, 0, 0),
            yellow: (255, 255, 0),
            grey: (128, 128, 128),
            cyan: (0, 170, 170),
            magenta: (170, 0, 170),
        }
    }

    #[test]
    fn test_status_color_mapping() {
        let colors = test_colors();
        // Just verify it doesn't panic for all statuses
        for status in [
            SyncStatus::Synced,
            SyncStatus::Syncing,
            SyncStatus::Pending,
            SyncStatus::Error,
            SyncStatus::CloudOnly,
            SyncStatus::Offline,
            SyncStatus::Excluded,
            SyncStatus::Unknown,
        ] {
            let _ = status_color(status, &colors);
        }
    }
}
