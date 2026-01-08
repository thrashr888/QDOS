//! Google Drive Plugin Modal Rendering

use super::state::{GDriveState, GDriveView};
use crate::app::ThemeColors;
use crate::plugins::cloud::ui::status_span;
use crate::plugins::cloud::{StorageInfo, SyncStatus};
use crate::ui::components::ModalFrame;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Draw the Google Drive modal
pub fn draw_gdrive_modal(frame: &mut Frame, area: Rect, state: &GDriveState, colors: &ThemeColors) {
    let modal = ModalFrame::themed(area, " Google Drive ", colors);
    modal.render_frame(frame);

    let content_area = modal.content_area();

    match state.view {
        GDriveView::Browser => draw_browser_view(frame, content_area, state, colors),
        GDriveView::Info => draw_info_view(frame, content_area, state, colors),
    }

    let help = vec![
        ("Enter", "open"),
        ("Esc", "close"),
        ("I", "info"),
        ("W", "web"),
        ("R", "refresh"),
    ];
    modal.render_help(frame, help);
}

fn draw_browser_view(frame: &mut Frame, area: Rect, state: &GDriveState, colors: &ThemeColors) {
    let chunks = Layout::vertical([
        Constraint::Length(2), // Status bar
        Constraint::Min(1),    // File list
    ])
    .split(area);

    draw_status_bar(frame, chunks[0], state, colors);
    draw_file_list(frame, chunks[1], state, colors);
}

fn draw_status_bar(frame: &mut Frame, area: Rect, state: &GDriveState, colors: &ThemeColors) {
    let path_str = state.current_dir.to_string_lossy();
    let truncated_path = if path_str.len() > area.width as usize - 15 {
        format!(
            "...{}",
            &path_str[path_str.len() - (area.width as usize - 18)..]
        )
    } else {
        path_str.to_string()
    };

    let status_indicator = if state.is_running {
        Span::styled("● ", Style::default().fg(colors.green()))
    } else {
        Span::styled("○ ", Style::default().fg(colors.red()))
    };

    let line1 = Line::from(vec![
        status_indicator,
        Span::styled(truncated_path, Style::default().fg(colors.fg())),
    ]);

    let count_str = format!("{} items", state.files.len());
    let line2 = Line::from(vec![Span::styled(
        count_str,
        Style::default().fg(colors.grey()),
    )]);

    let para = Paragraph::new(vec![line1, line2]);
    frame.render_widget(para, area);
}

fn draw_file_list(frame: &mut Frame, area: Rect, state: &GDriveState, colors: &ThemeColors) {
    let visible_height = area.height as usize;

    if state.files.is_empty() {
        let para =
            Paragraph::new("No files in this directory").style(Style::default().fg(colors.grey()));
        frame.render_widget(para, area);
        return;
    }

    let scroll_offset = if state.selected >= visible_height {
        state.selected - visible_height + 1
    } else {
        0
    };

    let mut lines: Vec<Line> = Vec::new();

    for (i, file) in state
        .files
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(visible_height)
    {
        let is_selected = i == state.selected;

        let sync_status: SyncStatus = file.sync_state.into();
        let status_char = status_span(sync_status, colors, true);

        let type_indicator = if file.is_dir {
            Span::styled("<DIR>", Style::default().fg(colors.blue()))
        } else if file.is_google_doc {
            Span::styled("<DOC>", Style::default().fg(colors.cyan()))
        } else {
            Span::styled("     ", Style::default())
        };

        let name_style = if is_selected {
            Style::default()
                .fg(colors.yellow())
                .bg(colors.red())
                .add_modifier(Modifier::BOLD)
        } else if file.is_dir {
            Style::default().fg(colors.blue())
        } else if file.is_google_doc {
            Style::default().fg(colors.cyan())
        } else {
            Style::default().fg(colors.fg())
        };

        let name = if file.name.len() > 40 {
            format!("{}...", &file.name[..37])
        } else {
            file.name.clone()
        };

        let size_str = file
            .size
            .map(|s| format!("{:>8}", StorageInfo::format_bytes(s)))
            .unwrap_or_else(|| "        ".to_string());

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

fn draw_info_view(frame: &mut Frame, area: Rect, state: &GDriveState, colors: &ThemeColors) {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![Span::styled(
        "Google Drive Status",
        Style::default()
            .fg(colors.yellow())
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    // Status
    let status_line = if state.is_running {
        Line::from(vec![
            Span::styled("Status:     ", Style::default().fg(colors.grey())),
            Span::styled("Running", Style::default().fg(colors.green())),
        ])
    } else if state.is_installed {
        Line::from(vec![
            Span::styled("Status:     ", Style::default().fg(colors.grey())),
            Span::styled(
                "Installed (not running)",
                Style::default().fg(colors.yellow()),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled("Status:     ", Style::default().fg(colors.grey())),
            Span::styled("Not Installed", Style::default().fg(colors.red())),
        ])
    };
    lines.push(status_line);

    // Variant
    let variant_str = match state.drive_variant {
        super::state::GDriveVariant::None => "None",
        super::state::GDriveVariant::VolumesMount => "Volume Mount (/Volumes/GoogleDrive)",
        super::state::GDriveVariant::HomeFolder => "Home Folder (~/Google Drive)",
        super::state::GDriveVariant::Stream => "Drive Stream",
    };
    lines.push(Line::from(vec![
        Span::styled("Type:       ", Style::default().fg(colors.grey())),
        Span::styled(variant_str, Style::default().fg(colors.fg())),
    ]));

    lines.push(Line::from(""));

    // Storage
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
    let gdocs = state.files.iter().filter(|f| f.is_google_doc).count();
    let dirs = state.files.iter().filter(|f| f.is_dir).count();
    let files = state.files.len() - dirs - gdocs;

    lines.push(Line::from(vec![
        Span::styled("Contents:   ", Style::default().fg(colors.grey())),
        Span::styled(
            format!("{} folders", dirs),
            Style::default().fg(colors.blue()),
        ),
        Span::raw(", "),
        Span::styled(format!("{} files", files), Style::default().fg(colors.fg())),
        Span::raw(", "),
        Span::styled(
            format!("{} Google Docs", gdocs),
            Style::default().fg(colors.cyan()),
        ),
    ]));

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}
