//! iCloud Drive Plugin Modal Rendering

use super::state::{ICloudState, ICloudSyncState, ICloudView};
use qdos_plugin_api::ui::FullScreenView;
use qdos_plugin_api::ThemeColors;
use qdos_plugin_cloud::ui::status_span;
use qdos_plugin_cloud::{StorageInfo, SyncStatus};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Draw the iCloud modal
pub fn draw_icloud_modal(frame: &mut Frame, area: Rect, state: &ICloudState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " iCloud Drive ", colors);
    view.render_frame(frame);

    let content_area = view.content_area();

    match state.view {
        ICloudView::Browser => draw_browser_view(frame, content_area, state, colors),
        ICloudView::Info => draw_info_view(frame, content_area, state, colors),
    }

    // Help line
    let help = vec![
        ("Enter", "open"),
        ("Esc", "close"),
        ("F", "filter"),
        ("I", "info"),
        ("D", "download"),
        ("E", "evict"),
    ];
    view.render_help(frame, help);
}

/// Draw the file browser view
fn draw_browser_view(frame: &mut Frame, area: Rect, state: &ICloudState, colors: &ThemeColors) {
    let chunks = Layout::vertical([
        Constraint::Length(2), // Status bar
        Constraint::Min(1),    // File list
    ])
    .split(area);

    draw_status_bar(frame, chunks[0], state, colors);
    draw_file_list(frame, chunks[1], state, colors);
}

/// Draw the status bar
fn draw_status_bar(frame: &mut Frame, area: Rect, state: &ICloudState, colors: &ThemeColors) {
    let path_str = state.current_dir.to_string_lossy();

    // Shorten path for display
    let display_path = if path_str.contains("com~apple~CloudDocs") {
        path_str
            .replace("Library/Mobile Documents/com~apple~CloudDocs", "iCloud")
            .to_string()
    } else {
        path_str.to_string()
    };

    let truncated_path = if display_path.len() > area.width as usize - 15 {
        format!(
            "...{}",
            &display_path[display_path.len() - (area.width as usize - 18)..]
        )
    } else {
        display_path
    };

    let status_indicator = if state.is_available {
        Span::styled("● ", Style::default().fg(colors.green()))
    } else {
        Span::styled("○ ", Style::default().fg(colors.red()))
    };

    let line1 = Line::from(vec![
        status_indicator,
        Span::styled(truncated_path, Style::default().fg(colors.fg())),
    ]);

    let filter_str = format!("[Filter: {}]", state.filter.as_str());
    let file_count = state.filtered_files().len();
    let total_count = state.files.len();
    let count_str = if file_count == total_count {
        format!("{} items", total_count)
    } else {
        format!("{}/{} items", file_count, total_count)
    };

    let line2 = Line::from(vec![
        Span::styled(filter_str, Style::default().fg(colors.yellow())),
        Span::raw("  "),
        Span::styled(count_str, Style::default().fg(colors.grey())),
    ]);

    let para = Paragraph::new(vec![line1, line2]);
    frame.render_widget(para, area);
}

/// Draw the file list
fn draw_file_list(frame: &mut Frame, area: Rect, state: &ICloudState, colors: &ThemeColors) {
    let files = state.filtered_files();
    let visible_height = area.height as usize;

    if files.is_empty() {
        let empty_msg = if state.files.is_empty() {
            "No files in this directory"
        } else {
            "No files match the current filter"
        };
        let para = Paragraph::new(empty_msg).style(Style::default().fg(colors.grey()));
        frame.render_widget(para, area);
        return;
    }

    let scroll_offset = if state.selected >= visible_height {
        state.selected - visible_height + 1
    } else {
        0
    };

    let mut lines: Vec<Line> = Vec::new();

    for (i, file) in files
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(visible_height)
    {
        let is_selected = i == state.selected;

        // Sync status indicator
        let sync_status: SyncStatus = file.sync_state.into();
        let status_char = status_span(sync_status, colors, true);

        // File type indicator
        let type_indicator = if file.is_dir {
            Span::styled("<DIR>", Style::default().fg(colors.blue()))
        } else {
            Span::styled("     ", Style::default())
        };

        // File name
        let name_style = if is_selected {
            Style::default()
                .fg(colors.yellow())
                .bg(colors.red())
                .add_modifier(Modifier::BOLD)
        } else if file.is_dir {
            Style::default().fg(colors.blue())
        } else if file.sync_state == ICloudSyncState::CloudOnly {
            Style::default().fg(colors.cyan())
        } else {
            Style::default().fg(colors.fg())
        };

        let name = if file.name.len() > 40 {
            format!("{}...", &file.name[..37])
        } else {
            file.name.clone()
        };

        // Size
        let size_str = file
            .size
            .map(|s| format!("{:>8}", StorageInfo::format_bytes(s)))
            .unwrap_or_else(|| "   Cloud".to_string());

        lines.push(Line::from(vec![
            status_char,
            Span::raw(" "),
            type_indicator,
            Span::raw(" "),
            Span::styled(format!("{:<40}", name), name_style),
            Span::styled(size_str, Style::default().fg(colors.grey())),
        ]));
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}

/// Draw the info view
fn draw_info_view(frame: &mut Frame, area: Rect, state: &ICloudState, colors: &ThemeColors) {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![Span::styled(
        "iCloud Drive Status",
        Style::default()
            .fg(colors.yellow())
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    // Availability status
    let status_line = if state.is_available {
        Line::from(vec![
            Span::styled("Status:     ", Style::default().fg(colors.grey())),
            Span::styled("Available", Style::default().fg(colors.green())),
        ])
    } else {
        Line::from(vec![
            Span::styled("Status:     ", Style::default().fg(colors.grey())),
            Span::styled("Not Available", Style::default().fg(colors.red())),
        ])
    };
    lines.push(status_line);

    lines.push(Line::from(""));

    // Storage info
    if let Some(total) = state.storage_info.total_bytes {
        let used = state.storage_info.used_bytes.unwrap_or(0);
        let free = total.saturating_sub(used);
        let percent = state.storage_info.usage_percent().unwrap_or(0);

        lines.push(Line::from(vec![
            Span::styled("Storage:    ", Style::default().fg(colors.grey())),
            Span::styled(
                format!(
                    "{} / {}",
                    StorageInfo::format_bytes(used),
                    StorageInfo::format_bytes(total)
                ),
                Style::default().fg(colors.fg()),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Free:       ", Style::default().fg(colors.grey())),
            Span::styled(
                StorageInfo::format_bytes(free),
                Style::default().fg(colors.fg()),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Usage:      ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}%", percent),
                Style::default().fg(if percent >= 90 {
                    colors.red()
                } else if percent >= 75 {
                    colors.yellow()
                } else {
                    colors.green()
                }),
            ),
        ]));
    }

    lines.push(Line::from(""));

    // File counts
    let downloaded = state
        .files
        .iter()
        .filter(|f| f.sync_state == ICloudSyncState::Downloaded)
        .count();
    let cloud_only = state
        .files
        .iter()
        .filter(|f| f.sync_state == ICloudSyncState::CloudOnly)
        .count();
    let syncing = state
        .files
        .iter()
        .filter(|f| {
            f.sync_state == ICloudSyncState::Downloading
                || f.sync_state == ICloudSyncState::Uploading
        })
        .count();

    lines.push(Line::from(vec![
        Span::styled("Files:      ", Style::default().fg(colors.grey())),
        Span::styled(
            format!("{} downloaded", downloaded),
            Style::default().fg(colors.green()),
        ),
        Span::raw(", "),
        Span::styled(
            format!("{} cloud-only", cloud_only),
            Style::default().fg(colors.cyan()),
        ),
        Span::raw(", "),
        Span::styled(
            format!("{} syncing", syncing),
            Style::default().fg(colors.yellow()),
        ),
    ]));

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}
