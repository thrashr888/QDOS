//! Proc plugin modal rendering
//!
//! Rendering functions for process monitoring views.

use super::state::{ProcState, ProcView};
use crate::app::ThemeColors;
use crate::ui::components::FullScreenView;
use humansize::{format_size, DECIMAL};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

/// Format CPU time from milliseconds to HH:MM:SS
pub fn format_cpu_time(ms: u64) -> String {
    let total_secs = ms / 1000;
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, mins, secs)
    } else {
        format!("{:02}:{:02}", mins, secs)
    }
}

/// Draw CPU view: Name, % CPU, CPU Time, PID, User
pub fn draw_cpu_view(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &ProcState,
    height: usize,
    colors: &ThemeColors,
) {
    let bg = colors.bg();
    let fg = colors.fg();
    let blue = colors.blue();
    let yellow = colors.yellow();
    let red = colors.red();

    let header_style = Style::default().fg(blue).bg(bg);
    let normal_style = Style::default().fg(fg).bg(bg);
    let highlight_style = Style::default().fg(yellow).bg(red);

    // Header row
    let sort_indicator = state.sort.as_str();
    view.render_row(
        frame,
        0,
        vec![Span::styled(
            format!(
                " {:<30}  {:>8}  {:>10}  {:>7}  {:<12}  ({})",
                "Name", "% CPU", "CPU Time", "PID", "User", sort_indicator
            ),
            header_style,
        )],
    );

    // Visible processes
    let visible_height = height.saturating_sub(1);
    let mut scroll_offset = state.scroll_offset;
    if state.selected >= scroll_offset + visible_height {
        scroll_offset = state.selected.saturating_sub(visible_height - 1);
    }
    if state.selected < scroll_offset {
        scroll_offset = state.selected;
    }

    for (i, proc) in state
        .processes
        .iter()
        .skip(scroll_offset)
        .take(visible_height)
        .enumerate()
    {
        let row = i as u16 + 1;
        let actual_idx = scroll_offset + i;
        let is_selected = actual_idx == state.selected;

        let style = if is_selected {
            highlight_style
        } else {
            normal_style
        };

        // Color CPU usage based on value
        let cpu_style = if proc.cpu_usage > 50.0 {
            style.fg(red)
        } else if proc.cpu_usage > 10.0 {
            style.fg(yellow)
        } else {
            style
        };

        let name = if proc.name.len() > 30 {
            &proc.name[..30]
        } else {
            &proc.name
        };
        let user = if proc.user.len() > 12 {
            &proc.user[..12]
        } else {
            &proc.user
        };

        view.render_row(
            frame,
            row,
            vec![
                Span::styled(format!(" {:<30}  ", name), style),
                Span::styled(format!("{:>8.1}  ", proc.cpu_usage), cpu_style),
                Span::styled(
                    format!("{:>10}  ", format_cpu_time(proc.cpu_time_ms)),
                    style,
                ),
                Span::styled(format!("{:>7}  ", proc.pid), style),
                Span::styled(format!("{:<12}", user), style),
            ],
        );
    }

    // Fill remaining lines
    let displayed = state.processes.len().min(visible_height);
    for i in displayed..visible_height {
        view.render_row(frame, i as u16 + 1, vec![Span::styled("", normal_style)]);
    }
}

/// Draw Memory view: Name, Memory, PID, User
pub fn draw_memory_view(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &ProcState,
    height: usize,
    colors: &ThemeColors,
) {
    let bg = colors.bg();
    let fg = colors.fg();
    let blue = colors.blue();
    let yellow = colors.yellow();
    let red = colors.red();

    let header_style = Style::default().fg(blue).bg(bg);
    let normal_style = Style::default().fg(fg).bg(bg);
    let highlight_style = Style::default().fg(yellow).bg(red);

    // Header row
    let sort_indicator = state.sort.as_str();
    view.render_row(
        frame,
        0,
        vec![Span::styled(
            format!(
                " {:<30}  {:>12}  {:>7}  {:<12}  ({})",
                "Name", "Memory", "PID", "User", sort_indicator
            ),
            header_style,
        )],
    );

    // Visible processes
    let visible_height = height.saturating_sub(1);
    let mut scroll_offset = state.scroll_offset;
    if state.selected >= scroll_offset + visible_height {
        scroll_offset = state.selected.saturating_sub(visible_height - 1);
    }
    if state.selected < scroll_offset {
        scroll_offset = state.selected;
    }

    for (i, proc) in state
        .processes
        .iter()
        .skip(scroll_offset)
        .take(visible_height)
        .enumerate()
    {
        let row = i as u16 + 1;
        let actual_idx = scroll_offset + i;
        let is_selected = actual_idx == state.selected;

        let style = if is_selected {
            highlight_style
        } else {
            normal_style
        };

        let name = if proc.name.len() > 30 {
            &proc.name[..30]
        } else {
            &proc.name
        };
        let user = if proc.user.len() > 12 {
            &proc.user[..12]
        } else {
            &proc.user
        };

        view.render_row(
            frame,
            row,
            vec![
                Span::styled(format!(" {:<30}  ", name), style),
                Span::styled(
                    format!("{:>12}  ", format_size(proc.memory, DECIMAL)),
                    style,
                ),
                Span::styled(format!("{:>7}  ", proc.pid), style),
                Span::styled(format!("{:<12}", user), style),
            ],
        );
    }

    // Fill remaining lines
    let displayed = state.processes.len().min(visible_height);
    for i in displayed..visible_height {
        view.render_row(frame, i as u16 + 1, vec![Span::styled("", normal_style)]);
    }
}

/// Draw disk view: Per-process disk I/O (Name, Bytes Written, Bytes Read, PID, User)
pub fn draw_disk_view(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &ProcState,
    height: usize,
    colors: &ThemeColors,
) {
    let bg = colors.bg();
    let fg = colors.fg();
    let blue = colors.blue();
    let yellow = colors.yellow();
    let red = colors.red();

    let header_style = Style::default().fg(blue).bg(bg);
    let normal_style = Style::default().fg(fg).bg(bg);
    let highlight_style = Style::default().fg(yellow).bg(red);

    // Header row
    view.render_row(
        frame,
        0,
        vec![Span::styled(
            format!(
                " {:<30}  {:>14}  {:>14}  {:>7}  {:<12}",
                "Name", "Bytes Written", "Bytes Read", "PID", "User"
            ),
            header_style,
        )],
    );

    // Visible processes (sorted by disk activity)
    let visible_height = height.saturating_sub(1);
    let mut scroll_offset = state.scroll_offset;
    if state.selected >= scroll_offset + visible_height {
        scroll_offset = state.selected.saturating_sub(visible_height - 1);
    }
    if state.selected < scroll_offset {
        scroll_offset = state.selected;
    }

    for (i, proc) in state
        .processes
        .iter()
        .skip(scroll_offset)
        .take(visible_height)
        .enumerate()
    {
        let row = i as u16 + 1;
        let actual_idx = scroll_offset + i;
        let is_selected = actual_idx == state.selected;

        let style = if is_selected {
            highlight_style
        } else {
            normal_style
        };

        let name = if proc.name.len() > 30 {
            &proc.name[..30]
        } else {
            &proc.name
        };
        let user = if proc.user.len() > 12 {
            &proc.user[..12]
        } else {
            &proc.user
        };

        view.render_row(
            frame,
            row,
            vec![
                Span::styled(format!(" {:<30}  ", name), style),
                Span::styled(
                    format!("{:>14}  ", format_size(proc.bytes_written, DECIMAL)),
                    style,
                ),
                Span::styled(
                    format!("{:>14}  ", format_size(proc.bytes_read, DECIMAL)),
                    style,
                ),
                Span::styled(format!("{:>7}  ", proc.pid), style),
                Span::styled(format!("{:<12}", user), style),
            ],
        );
    }

    // Fill remaining lines
    let displayed = state.processes.len().min(visible_height);
    for i in displayed..visible_height {
        view.render_row(frame, i as u16 + 1, vec![Span::styled("", normal_style)]);
    }
}

/// Draw network view
pub fn draw_network_view(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &ProcState,
    height: usize,
    colors: &ThemeColors,
) {
    let bg = colors.bg();
    let fg = colors.fg();
    let blue = colors.blue();
    let yellow = colors.yellow();
    let red = colors.red();

    let header_style = Style::default().fg(blue).bg(bg);
    let normal_style = Style::default().fg(fg).bg(bg);
    let highlight_style = Style::default().fg(yellow).bg(red);

    // Header row
    view.render_row(
        frame,
        0,
        vec![Span::styled(
            format!(
                " {:<20}  {:>14}  {:>14}  {:>12}  {:>12}",
                "Interface", "Received", "Transmitted", "Pkts In", "Pkts Out"
            ),
            header_style,
        )],
    );

    // Visible networks
    let visible_height = height.saturating_sub(1);

    for (i, net) in state.networks.iter().take(visible_height).enumerate() {
        let row = i as u16 + 1;
        let is_selected = i == state.selected;

        let style = if is_selected {
            highlight_style
        } else {
            normal_style
        };

        let iface_name = if net.name.len() > 20 {
            &net.name[..20]
        } else {
            &net.name
        };

        view.render_row(
            frame,
            row,
            vec![
                Span::styled(format!(" {:<20}  ", iface_name), style),
                Span::styled(
                    format!("{:>14}  ", format_size(net.received, DECIMAL)),
                    style,
                ),
                Span::styled(
                    format!("{:>14}  ", format_size(net.transmitted, DECIMAL)),
                    style,
                ),
                Span::styled(format!("{:>12}  ", net.packets_in), style),
                Span::styled(format!("{:>12}", net.packets_out), style),
            ],
        );
    }

    // Fill remaining lines
    let displayed = state.networks.len().min(visible_height);
    for i in displayed..visible_height {
        view.render_row(frame, i as u16 + 1, vec![Span::styled("", normal_style)]);
    }
}

/// Draw process detail overlay
pub fn draw_detail_overlay(
    frame: &mut Frame,
    parent_area: Rect,
    state: &ProcState,
    colors: &ThemeColors,
) {
    let Some(ref detail) = state.detail_info else {
        return;
    };

    let bg = colors.bg();
    let fg = colors.fg();
    let blue = colors.blue();
    let green = colors.green();
    let yellow = colors.yellow();

    let border_style = Style::default().fg(fg).bg(bg);
    let title_style = Style::default()
        .fg(yellow)
        .bg(bg)
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(blue).bg(bg);
    let value_style = Style::default().fg(fg).bg(bg);

    // Center overlay - 80% width, 60% height
    let overlay_width = (parent_area.width as f32 * 0.8) as u16;
    let overlay_height = (parent_area.height as f32 * 0.6) as u16;
    let overlay_x = parent_area.x + (parent_area.width - overlay_width) / 2;
    let overlay_y = parent_area.y + (parent_area.height - overlay_height) / 2;
    let overlay_area = Rect::new(overlay_x, overlay_y, overlay_width, overlay_height);

    // Clear overlay area
    frame.render_widget(Clear, overlay_area);

    let inner_width = overlay_width.saturating_sub(2) as usize;

    // Build content lines
    let mut content_lines: Vec<(String, String)> = vec![
        ("PID".to_string(), detail.pid.to_string()),
        ("Name".to_string(), detail.name.clone()),
        ("Status".to_string(), detail.status.clone()),
        ("User".to_string(), detail.user.clone()),
        ("CPU Usage".to_string(), format!("{:.1}%", detail.cpu_usage)),
        ("Memory".to_string(), format_size(detail.memory, DECIMAL)),
        (
            "Parent PID".to_string(),
            detail
                .parent_pid
                .map(|p| p.to_string())
                .unwrap_or_else(|| "-".to_string()),
        ),
        (
            "Start Time".to_string(),
            chrono::DateTime::from_timestamp(detail.start_time as i64, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "-".to_string()),
        ),
        ("Working Dir".to_string(), detail.cwd.clone()),
    ];

    // Add command line (may span multiple lines)
    if !detail.cmd.is_empty() {
        content_lines.push(("Command".to_string(), detail.cmd.join(" ")));
    }

    // Draw top border
    let mut y = overlay_y;
    let top_border = format!("╔{}╗", "═".repeat(inner_width));
    frame.render_widget(
        Paragraph::new(Span::styled(&top_border, border_style)),
        Rect::new(overlay_x, y, overlay_width, 1),
    );
    y += 1;

    // Draw title row
    let mut title_spans = vec![Span::styled("║", border_style)];
    title_spans.push(Span::styled(
        format!(" Process Info: {} (PID {}) ", detail.name, detail.pid),
        title_style,
    ));
    let title_content: usize = title_spans.iter().map(|s| s.width()).sum();
    let title_pad = (overlay_width as usize).saturating_sub(title_content + 1);
    title_spans.push(Span::styled(" ".repeat(title_pad), value_style));
    title_spans.push(Span::styled("║", border_style));
    frame.render_widget(
        Paragraph::new(Line::from(title_spans)),
        Rect::new(overlay_x, y, overlay_width, 1),
    );
    y += 1;

    // Draw separator
    let sep = format!("╠{}╣", "═".repeat(inner_width));
    frame.render_widget(
        Paragraph::new(Span::styled(&sep, border_style)),
        Rect::new(overlay_x, y, overlay_width, 1),
    );
    y += 1;

    // Content area height
    let content_area_height = overlay_height.saturating_sub(5) as usize;

    // Draw content with scrolling
    for (i, (label, value)) in content_lines
        .iter()
        .skip(state.detail_scroll)
        .take(content_area_height)
        .enumerate()
    {
        let line_y = y + i as u16;
        let mut spans = vec![Span::styled("║ ", border_style)];
        spans.push(Span::styled(format!("{:<12}: ", label), label_style));

        // Truncate value if too long
        let max_val_len = inner_width.saturating_sub(16);
        let val_display = if value.len() > max_val_len {
            format!("{}...", &value[..max_val_len.saturating_sub(3)])
        } else {
            value.clone()
        };
        spans.push(Span::styled(&val_display, value_style));

        let content_width: usize = spans.iter().map(|s| s.width()).sum();
        let padding = (overlay_width as usize).saturating_sub(content_width + 1);
        spans.push(Span::styled(" ".repeat(padding), value_style));
        spans.push(Span::styled("║", border_style));

        frame.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect::new(overlay_x, line_y, overlay_width, 1),
        );
    }

    // Fill remaining content lines
    for i in content_lines.len().min(content_area_height)..content_area_height {
        let line_y = y + i as u16;
        let empty_line = format!("║{:width$}║", "", width = inner_width);
        frame.render_widget(
            Paragraph::new(Span::styled(&empty_line, value_style)),
            Rect::new(overlay_x, line_y, overlay_width, 1),
        );
    }
    y += content_area_height as u16;

    // Draw footer separator
    let sep = format!("╠{}╣", "═".repeat(inner_width));
    frame.render_widget(
        Paragraph::new(Span::styled(&sep, border_style)),
        Rect::new(overlay_x, y, overlay_width, 1),
    );
    y += 1;

    // Draw footer
    let mut footer_spans = vec![Span::styled("║ ", border_style)];
    footer_spans.push(Span::styled("↑↓", Style::default().fg(green).bg(bg)));
    footer_spans.push(Span::styled(" scroll  ", value_style));
    footer_spans.push(Span::styled("Esc", Style::default().fg(green).bg(bg)));
    footer_spans.push(Span::styled(" close", value_style));

    let footer_width: usize = footer_spans.iter().map(|s| s.width()).sum();
    let footer_padding = (overlay_width as usize).saturating_sub(footer_width + 1);
    footer_spans.push(Span::styled(" ".repeat(footer_padding), value_style));
    footer_spans.push(Span::styled("║", border_style));

    frame.render_widget(
        Paragraph::new(Line::from(footer_spans)),
        Rect::new(overlay_x, y, overlay_width, 1),
    );
    y += 1;

    // Draw bottom border
    let bottom_border = format!("╚{}╝", "═".repeat(inner_width));
    frame.render_widget(
        Paragraph::new(Span::styled(&bottom_border, border_style)),
        Rect::new(overlay_x, y, overlay_width, 1),
    );
}

/// Draw the main proc modal
pub fn draw_proc_modal(frame: &mut Frame, area: Rect, state: &ProcState, colors: &ThemeColors) {
    // Build title with system stats
    let title = format!(
        " PROC - {} | CPU: {:.1}% ({} cores) | Mem: {}/{} ",
        state.view.as_str(),
        state.cpu_usage,
        state.cpu_count,
        format_size(state.used_memory, DECIMAL),
        format_size(state.total_memory, DECIMAL)
    );

    let view = FullScreenView::new(area, &title, colors);
    view.render_frame(frame);

    // Draw content based on view
    let content_area = view.content_area();
    let height = content_area.height as usize;

    match state.view {
        ProcView::Cpu => draw_cpu_view(frame, &view, state, height, colors),
        ProcView::Memory => draw_memory_view(frame, &view, state, height, colors),
        ProcView::Disk => draw_disk_view(frame, &view, state, height, colors),
        ProcView::Network => draw_network_view(frame, &view, state, height, colors),
    }

    // Draw help bar
    let auto_str = if state.auto_refresh { "on" } else { "off" };
    view.render_help(
        frame,
        vec![
            ("Tab", "view"),
            ("↑↓", "select"),
            ("S", "sort"),
            ("R", "refresh"),
            ("A", &format!("auto:{}", auto_str)),
            ("I", "info"),
            ("X", "kill"),
            ("Esc", "close"),
        ],
    );

    // Draw detail overlay if active
    if state.show_detail {
        draw_detail_overlay(frame, area, state, colors);
    }
}
