//\! Modal UI components
//\!
//\! This module contains all modal drawing functions including:
//\! - Help modal, Status modal, Quit modal
//\! - File operation modals (Copy, Move, Erase, Rename)
//\! - Search and Find modals
//\! - Directory Map, Batch Rename, Attribute modals
//\! - Error, Success, Progress modals

#[allow(unused_imports)]
use crate::app::{
    App, AttrValue, AttributeState, BatchRenameState, ColorTheme, ColorThemeState,
    DirectoryMapState, FindPhase, FindState, HelpState, Modal, ProgressState, QdstartField,
    QdstartState, SearchSpecState,
};
use crate::file_ops::get_disk_space;
use humansize::{format_size, DECIMAL};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::{
    format_size_short, viewer::{draw_file_viewer, draw_shell_command},
    COLOR_BG, COLOR_BLUE, COLOR_CYAN, COLOR_FG, COLOR_GREEN, COLOR_GREY, COLOR_RED, COLOR_YELLOW,
};

/// Create a centered rectangle
pub(super) fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}


pub(super) fn draw_modal(frame: &mut Frame, app: &App, area: Rect) {
    // Modals that handle their own area/clearing (overlay directly like quit modal)
    match &app.modal {
        Modal::Quit => {
            draw_quit_modal(frame, area, app);
            return;
        }
        Modal::Space => {
            draw_space_modal(frame, area, app);
            return;
        }
        Modal::Error(msg) => {
            draw_error_modal(frame, area, msg, app);
            return;
        }
        Modal::Success(msg) => {
            draw_success_modal(frame, area, msg, app);
            return;
        }
        Modal::Progress(state) => {
            draw_progress_modal(frame, area, state);
            return;
        }
        _ => {}
    }

    let modal_area = centered_rect(60, 50, area);

    // Clear the modal area for other modals
    frame.render_widget(Clear, modal_area);

    match &app.modal {
        Modal::Help(state) => draw_help_modal(frame, area, state),
        Modal::Status(info) => draw_status_modal(frame, modal_area, info),
        Modal::Quit => {} // Handled above
        Modal::SearchSpec(state) => draw_search_spec_modal(frame, modal_area, state),
        Modal::Space => {} // Handled above
        Modal::Error(_) => {} // Handled above
        Modal::Success(_) => {} // Handled above
        Modal::PathInput(path) => draw_path_input_modal(frame, modal_area, path),
        Modal::CopyTo(dest) => draw_copy_modal(frame, modal_area, dest, app),
        Modal::MoveTo(dest) => draw_move_modal(frame, modal_area, dest, app),
        Modal::EraseConfirm => draw_erase_modal(frame, modal_area, app),
        Modal::RenameInput(name) => draw_rename_modal(frame, modal_area, name),
        Modal::ShellCommand(state) => draw_shell_command(frame, area, state, app),
        Modal::FileViewer(state) => draw_file_viewer(frame, area, state),
        Modal::DirectoryMap(state) => draw_directory_map(frame, area, state),
        Modal::Find(state) => draw_find_modal(frame, area, state),
        Modal::BatchRename(state) => draw_batch_rename_modal(frame, area, state),
        Modal::Attribute(state) => draw_attribute_modal(frame, modal_area, state),
        Modal::Progress(_) => {} // Handled above
        Modal::ColorTheme(state) => draw_color_theme_modal(frame, modal_area, state, app),
        Modal::Qdstart(state) => draw_qdstart_modal(frame, area, state, app),
        Modal::None => {}
    }
}

/// Draw help modal with multi-page support (full-page view)
pub(super) fn draw_help_modal(frame: &mut Frame, area: Rect, state: &HelpState) {
    // Clear the full area first
    frame.render_widget(Clear, area);

    // Split into content area and status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let content_area = chunks[0];
    let status_area = chunks[1];

    if state.current_topic == 0 {
        // Index page - show list of topics
        let help_block = Block::default()
            .title(" Help - Index ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_BLUE))
            .style(Style::default().bg(COLOR_BG));

        let mut lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "R-DOS File Manager Help",
                Style::default().fg(COLOR_FG).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press a key to view topic:",
                Style::default().fg(COLOR_BLUE),
            )),
            Line::from(""),
        ];

        for topic in &state.topics {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {}  ", topic.key),
                    Style::default().fg(COLOR_YELLOW).add_modifier(Modifier::BOLD),
                ),
                Span::styled(&topic.title, Style::default().fg(COLOR_FG)),
            ]));
        }

        let paragraph = Paragraph::new(lines)
            .block(help_block)
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, content_area);

        // Status bar
        let status = Paragraph::new(Line::from(vec![
            Span::styled(" ESC ", Style::default().fg(COLOR_BG).bg(COLOR_GREEN)),
            Span::styled(" Close", Style::default().fg(COLOR_FG).bg(COLOR_BG)),
        ]))
        .style(Style::default().bg(COLOR_BG));

        frame.render_widget(status, status_area);
    } else {
        // Topic page - show topic content
        let topic = &state.topics[state.current_topic - 1];
        let title = format!(" Help - {} ", topic.title);

        let help_block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_BLUE))
            .style(Style::default().bg(COLOR_BG));

        let inner_height = content_area.height.saturating_sub(2) as usize;
        let total_lines = topic.content.lines().count();

        // Parse content into lines with scroll offset
        let content_lines: Vec<Line> = topic
            .content
            .lines()
            .skip(state.scroll_offset)
            .take(inner_height)
            .map(|line| {
                if line.starts_with("##") {
                    // Section header
                    Line::from(Span::styled(
                        line.trim_start_matches('#').trim(),
                        Style::default().fg(COLOR_BLUE).add_modifier(Modifier::BOLD),
                    ))
                } else if line.starts_with("  ") || line.starts_with("- ") {
                    // Indented or list item
                    Line::from(Span::styled(line, Style::default().fg(COLOR_FG)))
                } else if line.is_empty() {
                    Line::from("")
                } else {
                    Line::from(Span::styled(line, Style::default().fg(COLOR_FG)))
                }
            })
            .collect();

        let paragraph = Paragraph::new(content_lines)
            .block(help_block)
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, content_area);

        // Status bar with scroll indicator
        let can_scroll = total_lines > inner_height;
        let mut status_spans = vec![
            Span::styled(" ESC ", Style::default().fg(COLOR_BG).bg(COLOR_GREEN)),
            Span::styled(" Back  ", Style::default().fg(COLOR_FG).bg(COLOR_BG)),
            Span::styled(" ↑↓ ", Style::default().fg(COLOR_BG).bg(COLOR_CYAN)),
            Span::styled(" Scroll", Style::default().fg(COLOR_FG).bg(COLOR_BG)),
        ];

        if can_scroll {
            let max_scroll = total_lines.saturating_sub(inner_height);
            status_spans.push(Span::styled(
                format!("  [{}/{}]", state.scroll_offset + 1, max_scroll + 1),
                Style::default().fg(COLOR_CYAN).bg(COLOR_BG),
            ));
        }

        let status = Paragraph::new(Line::from(status_spans)).style(Style::default().bg(COLOR_BG));

        frame.render_widget(status, status_area);
    }
}

/// Draw status modal
pub(super) fn draw_status_modal(frame: &mut Frame, area: Rect, info: &crate::file_ops::SystemInfo) {
    let status_block = Block::default()
        .title(" System Status ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BLUE))
        .style(Style::default().bg(COLOR_BG));

    let label_style = Style::default().fg(COLOR_GREEN);
    let value_style = Style::default().fg(COLOR_FG);

    let status_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Hostname: ", label_style),
            Span::styled(&info.hostname, value_style),
        ]),
        Line::from(vec![
            Span::styled("OS: ", label_style),
            Span::styled(format!("{} {}", info.os_name, info.os_version), value_style),
        ]),
        Line::from(vec![
            Span::styled("CPUs: ", label_style),
            Span::styled(format!("{}", info.cpu_count), value_style),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Total Memory: ", label_style),
            Span::styled(format_size(info.total_memory, DECIMAL), value_style),
        ]),
        Line::from(vec![
            Span::styled("Used Memory: ", label_style),
            Span::styled(format_size(info.used_memory, DECIMAL), value_style),
        ]),
        Line::from(vec![
            Span::styled("Total Swap: ", label_style),
            Span::styled(format_size(info.total_swap, DECIMAL), value_style),
        ]),
        Line::from(vec![
            Span::styled("Used Swap: ", label_style),
            Span::styled(format_size(info.used_swap, DECIMAL), value_style),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to close",
            Style::default().fg(COLOR_GREEN),
        )),
    ];

    let paragraph = Paragraph::new(status_text)
        .block(status_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Draw quit confirmation modal (Q-DOS II style with double-line box)
pub(super) fn draw_quit_modal(frame: &mut Frame, area: Rect, app: &App) {
    let colors = app.colors();

    // Modal is 60 chars wide, 8 lines tall (matching spec/ui.md)
    let width: u16 = 60;
    let height: u16 = 8;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let quit_area = Rect::new(x, y, width.min(area.width), height.min(area.height));

    let style = Style::default().fg(colors.fg()).bg(colors.bg());

    // Build the modal line by line with double-line box characters
    // Line 0: Top border
    let top = format!("╔{}╗", "═".repeat(width as usize - 2));
    frame.render_widget(
        Paragraph::new(Span::styled(&top, style)),
        Rect::new(quit_area.x, quit_area.y, width, 1),
    );

    // Line 1: Title row
    let title = "F10 - Quit Q-DOS II";
    let title_line = format!("║{:^w$}║", title, w = width as usize - 2);
    frame.render_widget(
        Paragraph::new(Span::styled(&title_line, style)),
        Rect::new(quit_area.x, quit_area.y + 1, width, 1),
    );

    // Line 2: Separator (double line)
    let sep = format!("╠{}╣", "═".repeat(width as usize - 2));
    frame.render_widget(
        Paragraph::new(Span::styled(&sep, style)),
        Rect::new(quit_area.x, quit_area.y + 2, width, 1),
    );

    // Line 3: Empty row
    let empty = format!("║{:w$}║", "", w = width as usize - 2);
    frame.render_widget(
        Paragraph::new(Span::styled(&empty, style)),
        Rect::new(quit_area.x, quit_area.y + 3, width, 1),
    );

    // Line 4: First message
    let msg1 = "Press F10 again to quit, or RETURN for options";
    let msg1_line = format!("║{:^w$}║", msg1, w = width as usize - 2);
    frame.render_widget(
        Paragraph::new(Span::styled(&msg1_line, style)),
        Rect::new(quit_area.x, quit_area.y + 4, width, 1),
    );

    // Line 5: Empty row
    frame.render_widget(
        Paragraph::new(Span::styled(&empty, style)),
        Rect::new(quit_area.x, quit_area.y + 5, width, 1),
    );

    // Line 6: Second message
    let msg2 = "Press ESC to return to Q-DOS II";
    let msg2_line = format!("║{:^w$}║", msg2, w = width as usize - 2);
    frame.render_widget(
        Paragraph::new(Span::styled(&msg2_line, style)),
        Rect::new(quit_area.x, quit_area.y + 6, width, 1),
    );

    // Line 7: Bottom border
    let bottom = format!("╚{}╝", "═".repeat(width as usize - 2));
    frame.render_widget(
        Paragraph::new(Span::styled(&bottom, style)),
        Rect::new(quit_area.x, quit_area.y + 7, width, 1),
    );
}

/// Draw search specification modal
pub(super) fn draw_search_spec_modal(frame: &mut Frame, area: Rect, state: &SearchSpecState) {
    let title = if state.phase == 0 {
        " Set Search Specification "
    } else {
        " Search Attributes "
    };

    let search_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BLUE))
        .style(Style::default().bg(COLOR_BG));

    let mut lines = vec![Line::from("")];

    if state.phase == 0 {
        // Phase 0: Pattern input
        lines.push(Line::from(Span::styled(
            "Enter file search specification:",
            Style::default().fg(COLOR_GREEN),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Pattern: ", Style::default().fg(COLOR_BLUE)),
            Span::styled(
                &state.pattern,
                Style::default().fg(COLOR_YELLOW).bg(COLOR_RED),
            ),
            Span::styled("█", Style::default().fg(COLOR_YELLOW).bg(COLOR_RED)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Examples: *.*  *.txt  *.rs  config.*",
            Style::default().fg(COLOR_GREY),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Enter", Style::default().fg(COLOR_BLUE)),
            Span::raw(" next  "),
            Span::styled("ESC", Style::default().fg(COLOR_BLUE)),
            Span::raw(" cancel"),
        ]));
    } else {
        // Phase 1: Attribute selection
        lines.push(Line::from(vec![
            Span::styled("Pattern: ", Style::default().fg(COLOR_GREEN)),
            Span::styled(&state.pattern, Style::default().fg(COLOR_FG)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Select which file types to display:",
            Style::default().fg(COLOR_GREEN),
        )));
        lines.push(Line::from(""));

        // Build attribute bar
        let mut attr_spans: Vec<Span> = vec![Span::raw("  ")];
        for i in 0..6 {
            let name = SearchSpecState::attr_name(i);
            let is_on = state.attrs[i];
            let is_selected = i == state.selected_attr;

            let style = if is_selected {
                Style::default().fg(COLOR_YELLOW).bg(COLOR_RED)
            } else if is_on {
                Style::default().fg(COLOR_GREEN)
            } else {
                Style::default().fg(COLOR_GREY)
            };

            let indicator = if is_on { " ✓ " } else { "   " };
            attr_spans.push(Span::styled(format!("[{}{}]", name, indicator), style));
            attr_spans.push(Span::raw(" "));
        }
        lines.push(Line::from(attr_spans));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("←→", Style::default().fg(COLOR_BLUE)),
            Span::raw(" select  "),
            Span::styled("SPACE", Style::default().fg(COLOR_BLUE)),
            Span::raw(" toggle  "),
            Span::styled("Enter", Style::default().fg(COLOR_BLUE)),
            Span::raw(" apply  "),
            Span::styled("ESC", Style::default().fg(COLOR_BLUE)),
            Span::raw(" back"),
        ]));
    }

    let paragraph = Paragraph::new(lines)
        .block(search_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Draw Q-DOS II style modal with header separator (non-themed version for backwards compatibility)
#[allow(dead_code)]
pub(super) fn draw_qdos_modal_colored(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    content: Vec<Line>,
    _border_color: Color,
) {
    // Fixed modal size for consistency
    let modal_width: u16 = 50;
    let content_lines = content.len() as u16;
    let modal_height = content_lines + 4;

    let x = area.x + (area.width.saturating_sub(modal_width)) / 2;
    let y = area.y + (area.height.saturating_sub(modal_height)) / 2;
    let modal_area = Rect::new(x, y, modal_width.min(area.width), modal_height.min(area.height));

    frame.render_widget(Clear, modal_area);

    let width = modal_area.width as usize;
    let border_style = Style::default().fg(COLOR_FG).bg(COLOR_BG);
    let content_style = Style::default().fg(COLOR_FG).bg(COLOR_BG);

    let bg_block = ratatui::widgets::Block::default().style(Style::default().bg(COLOR_BG));
    frame.render_widget(bg_block, modal_area);

    let top = format!("╔{}╗", "═".repeat(width.saturating_sub(2)));
    frame.render_widget(
        Paragraph::new(Span::styled(&top, border_style)),
        Rect::new(modal_area.x, modal_area.y, modal_area.width, 1),
    );

    let title_padded = format!("{:^width$}", title, width = width.saturating_sub(2));
    let title_line = format!("║{}║", title_padded);
    frame.render_widget(
        Paragraph::new(Span::styled(&title_line, border_style)),
        Rect::new(modal_area.x, modal_area.y + 1, modal_area.width, 1),
    );

    let sep = format!("╠{}╣", "═".repeat(width.saturating_sub(2)));
    frame.render_widget(
        Paragraph::new(Span::styled(&sep, border_style)),
        Rect::new(modal_area.x, modal_area.y + 2, modal_area.width, 1),
    );

    for (i, line) in content.iter().enumerate() {
        let row_y = modal_area.y + 3 + i as u16;
        let left_border = Span::styled("║", border_style);
        let right_border = Span::styled("║", border_style);

        let mut row_spans = vec![left_border];
        for span in line.spans.iter() {
            row_spans.push(span.clone());
        }
        let content_width = line.width();
        let padding = width.saturating_sub(2).saturating_sub(content_width);
        let left_pad = padding / 2;
        let right_pad = padding - left_pad;
        row_spans.insert(1, Span::styled(" ".repeat(left_pad), content_style));
        row_spans.push(Span::styled(" ".repeat(right_pad), content_style));
        row_spans.push(right_border);

        frame.render_widget(
            Paragraph::new(Line::from(row_spans)).style(content_style),
            Rect::new(modal_area.x, row_y, modal_area.width, 1),
        );
    }

    let bottom = format!("╚{}╝", "═".repeat(width.saturating_sub(2)));
    frame.render_widget(
        Paragraph::new(Span::styled(&bottom, border_style)),
        Rect::new(
            modal_area.x,
            modal_area.y + modal_area.height - 1,
            modal_area.width,
            1,
        ),
    );
}

/// Draw Q-DOS II style modal with header separator and dynamic height (themed version)
/// Uses fixed sizes and preserves individual span colors in content.
/// Layout:
/// ╔════════════════════════════════╗
/// ║            Title               ║
/// ╠════════════════════════════════╣
/// ║           Content              ║
/// ╚════════════════════════════════╝
pub(super) fn draw_qdos_modal_themed(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    content: Vec<Line>,
    border_color: Color,
    app: &App,
) {
    let colors = app.colors();

    // Fixed modal size
    let modal_width: u16 = 50;
    let content_lines = content.len() as u16;
    let modal_height = content_lines + 4; // top + title + separator + bottom

    // Center the modal within the given area
    let x = area.x + (area.width.saturating_sub(modal_width)) / 2;
    let y = area.y + (area.height.saturating_sub(modal_height)) / 2;
    let modal_area = Rect::new(x, y, modal_width.min(area.width), modal_height.min(area.height));

    // Clear only the exact modal area
    frame.render_widget(Clear, modal_area);

    let width = modal_area.width as usize;
    let inner_w = width.saturating_sub(2);

    // Border style uses the border_color for fg, with theme background
    let border_style = Style::default().fg(border_color).bg(colors.bg());
    // Style for padding/empty space
    let pad_style = Style::default().fg(colors.fg()).bg(colors.bg());

    // Top border: ╔═══╗
    let top = format!("╔{}╗", "═".repeat(inner_w));
    frame.render_widget(
        Paragraph::new(Span::styled(&top, border_style)),
        Rect::new(modal_area.x, modal_area.y, modal_area.width, 1),
    );

    // Title row: ║ Title ║
    let title_padded = format!("{:^width$}", title, width = inner_w);
    let title_line = format!("║{}║", title_padded);
    frame.render_widget(
        Paragraph::new(Span::styled(&title_line, border_style)),
        Rect::new(modal_area.x, modal_area.y + 1, modal_area.width, 1),
    );

    // Header separator: ╠═══╣
    let sep = format!("╠{}╣", "═".repeat(inner_w));
    frame.render_widget(
        Paragraph::new(Span::styled(&sep, border_style)),
        Rect::new(modal_area.x, modal_area.y + 2, modal_area.width, 1),
    );

    // Content area - preserve individual span colors
    for (i, line) in content.iter().enumerate() {
        let row_y = modal_area.y + 3 + i as u16;

        // Calculate padding for centering
        let content_width = line.width();
        let padding = inner_w.saturating_sub(content_width);
        let left_pad = padding / 2;
        let right_pad = padding - left_pad;

        // Build row: ║ [padding] [content spans] [padding] ║
        let mut row_spans: Vec<Span> = Vec::with_capacity(line.spans.len() + 4);
        row_spans.push(Span::styled("║", border_style));
        row_spans.push(Span::styled(" ".repeat(left_pad), pad_style));

        // Add content spans with background applied
        for span in line.spans.iter() {
            let span_style = span.style.bg(colors.bg());
            row_spans.push(Span::styled(span.content.clone(), span_style));
        }

        row_spans.push(Span::styled(" ".repeat(right_pad), pad_style));
        row_spans.push(Span::styled("║", border_style));

        frame.render_widget(
            Paragraph::new(Line::from(row_spans)),
            Rect::new(modal_area.x, row_y, modal_area.width, 1),
        );
    }

    // Bottom border: ╚═══╝
    let bottom = format!("╚{}╝", "═".repeat(inner_w));
    frame.render_widget(
        Paragraph::new(Span::styled(&bottom, border_style)),
        Rect::new(modal_area.x, modal_area.y + modal_height - 1, modal_area.width, 1),
    );
}

/// Draw disk space modal
pub(super) fn draw_space_modal(frame: &mut Frame, area: Rect, app: &App) {
    let colors = app.colors();

    // Get disk name from current path (use first component or root)
    let disk_name = app
        .current_path
        .components()
        .next()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .unwrap_or_else(|| "/".to_string());

    let title = format!("Space On Disk {}", disk_name);

    let (available, total) = get_disk_space(&app.current_path).unwrap_or((0, 0));
    let used = total.saturating_sub(available);
    let used_percent = if total > 0 {
        used as f64 / total as f64 * 100.0
    } else {
        0.0
    };

    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Total space:      ", Style::default().fg(colors.yellow())),
            Span::styled(format_size_short(total), Style::default().fg(colors.cyan())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Total used:       ", Style::default().fg(colors.yellow())),
            Span::styled(
                format!("{} ({:.1}%)", format_size_short(used), used_percent),
                Style::default().fg(colors.cyan()),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Total available:  ", Style::default().fg(colors.yellow())),
            Span::styled(format_size_short(available), Style::default().fg(colors.cyan())),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to continue",
            Style::default().fg(colors.green()),
        )),
    ];

    draw_qdos_modal_themed(frame, area, &title, content, colors.fg(), app);
}

/// Draw error modal
pub(super) fn draw_error_modal(frame: &mut Frame, area: Rect, message: &str, app: &App) {
    let colors = app.colors();
    let content = vec![
        Line::from(""),
        Line::from(Span::styled(message, Style::default().fg(colors.fg()))),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to close",
            Style::default().fg(colors.green()),
        )),
    ];

    draw_qdos_modal_themed(frame, area, "Error", content, colors.fg(), app);
}

/// Draw path input modal
pub(super) fn draw_path_input_modal(frame: &mut Frame, area: Rect, path: &str) {
    // Use the modal area directly (already centered by draw_modal)
    let input_block = Block::default()
        .title(" Change Directory ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BLUE))
        .style(Style::default().bg(COLOR_BG));

    let input_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Enter path (Tab to complete):",
            Style::default().fg(COLOR_GREEN),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("{}_", path),
            Style::default().fg(COLOR_FG),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Tab", Style::default().fg(COLOR_BLUE)),
            Span::raw(" complete, "),
            Span::styled("Enter", Style::default().fg(COLOR_BLUE)),
            Span::raw(" confirm, "),
            Span::styled("Esc", Style::default().fg(COLOR_BLUE)),
            Span::raw(" cancel"),
        ]),
    ];

    let paragraph = Paragraph::new(input_text)
        .block(input_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Draw success modal
pub(super) fn draw_success_modal(frame: &mut Frame, area: Rect, message: &str, app: &App) {
    let colors = app.colors();
    let content = vec![
        Line::from(""),
        Line::from(Span::styled(message, Style::default().fg(colors.fg()))),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to close",
            Style::default().fg(colors.green()),
        )),
    ];

    draw_qdos_modal_themed(frame, area, "Success", content, colors.fg(), app);
}

/// Draw progress modal for file operations
pub(super) fn draw_progress_modal(frame: &mut Frame, area: Rect, state: &ProgressState) {
    let total = state.files.len();
    let current = state.current_index.min(total);
    let percentage = if total > 0 {
        (current * 100) / total
    } else {
        100
    };

    // Get current filename being processed
    let current_file = state
        .current_file()
        .map(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| p.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "Complete".to_string());

    // Build progress bar (40 chars wide)
    let bar_width = 40;
    let filled = (bar_width * percentage) / 100;
    let empty = bar_width - filled;
    let progress_bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(empty));

    let title = format!("{} Files", state.operation_name());

    let mut content = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("{} {} of {}", state.operation_name(), current, total),
            Style::default().fg(COLOR_FG),
        )),
        Line::from(""),
        Line::from(Span::styled(
            progress_bar,
            Style::default().fg(COLOR_BLUE),
        )),
        Line::from(Span::styled(
            format!("{}%", percentage),
            Style::default().fg(COLOR_GREEN),
        )),
        Line::from(""),
        Line::from(Span::styled(
            current_file,
            Style::default().fg(COLOR_YELLOW),
        )),
    ];

    // Show error if any
    if let Some(ref err) = state.last_error {
        content.push(Line::from(""));
        content.push(Line::from(Span::styled(
            format!("Error: {}", err),
            Style::default().fg(COLOR_RED),
        )));
    }

    // Show stats
    content.push(Line::from(""));
    content.push(Line::from(Span::styled(
        format!(
            "Completed: {}  Failed: {}",
            state.completed, state.failed
        ),
        Style::default().fg(COLOR_GREEN),
    )));
    content.push(Line::from(""));
    content.push(Line::from(Span::styled(
        "Press ESC to cancel",
        Style::default().fg(COLOR_GREY),
    )));

    draw_qdos_modal_colored(frame, area, &title, content, COLOR_BLUE);
}

/// Draw copy modal
pub(super) fn draw_copy_modal(frame: &mut Frame, area: Rect, dest: &str, app: &App) {
    // Use the modal area directly (already centered by draw_modal)
    let copy_block = Block::default()
        .title(" Copy Files ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BLUE))
        .style(Style::default().bg(COLOR_BG));

    let copy_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Copying {} tagged file(s)", app.tagged_files.len()),
            Style::default().fg(COLOR_YELLOW),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Destination (Tab to complete):",
            Style::default().fg(COLOR_GREEN),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("{}_", dest),
            Style::default().fg(COLOR_FG),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Tab", Style::default().fg(COLOR_BLUE)),
            Span::raw(" complete, "),
            Span::styled("Enter", Style::default().fg(COLOR_BLUE)),
            Span::raw(" copy, "),
            Span::styled("Esc", Style::default().fg(COLOR_BLUE)),
            Span::raw(" cancel"),
        ]),
    ];

    let paragraph = Paragraph::new(copy_text)
        .block(copy_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Draw move modal
pub(super) fn draw_move_modal(frame: &mut Frame, area: Rect, dest: &str, app: &App) {
    // Use the modal area directly (already centered by draw_modal)
    let move_block = Block::default()
        .title(" Move Files ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BLUE))
        .style(Style::default().bg(COLOR_BG));

    let move_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Moving {} tagged file(s)", app.tagged_files.len()),
            Style::default().fg(COLOR_YELLOW),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Destination (Tab to complete):",
            Style::default().fg(COLOR_GREEN),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("{}_", dest),
            Style::default().fg(COLOR_FG),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Tab", Style::default().fg(COLOR_BLUE)),
            Span::raw(" complete, "),
            Span::styled("Enter", Style::default().fg(COLOR_BLUE)),
            Span::raw(" move, "),
            Span::styled("Esc", Style::default().fg(COLOR_BLUE)),
            Span::raw(" cancel"),
        ]),
    ];

    let paragraph = Paragraph::new(move_text)
        .block(move_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Draw erase confirmation modal
pub(super) fn draw_erase_modal(frame: &mut Frame, area: Rect, app: &App) {
    // Use the modal area directly (already centered by draw_modal)
    let erase_block = Block::default()
        .title(" Erase Files ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_RED))
        .style(Style::default().bg(COLOR_BG));

    let erase_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Delete {} tagged file(s)?", app.tagged_files.len()),
            Style::default().fg(COLOR_YELLOW),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "This cannot be undone!",
            Style::default().fg(COLOR_RED),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Y]", Style::default().fg(COLOR_BLUE)),
            Span::raw("es  "),
            Span::styled("[N]", Style::default().fg(COLOR_BLUE)),
            Span::raw("o"),
        ]),
    ];

    let paragraph = Paragraph::new(erase_text)
        .block(erase_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Draw rename modal
pub(super) fn draw_rename_modal(frame: &mut Frame, area: Rect, name: &str) {
    // Use the modal area directly (already centered by draw_modal)
    let rename_block = Block::default()
        .title(" Rename File ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BLUE))
        .style(Style::default().bg(COLOR_BG));

    let rename_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Enter new name:",
            Style::default().fg(COLOR_GREEN),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("{}_", name),
            Style::default().fg(COLOR_FG),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Enter", Style::default().fg(COLOR_BLUE)),
            Span::raw(" rename, "),
            Span::styled("Esc", Style::default().fg(COLOR_BLUE)),
            Span::raw(" cancel"),
        ]),
    ];

    let paragraph = Paragraph::new(rename_text)
        .block(rename_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

pub(super) fn draw_directory_map(frame: &mut Frame, area: Rect, state: &DirectoryMapState) {
    // Clear the entire screen
    frame.render_widget(Clear, area);

    // Layout: title bar, separator, tree content, separator, help/input
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title bar
            Constraint::Length(1), // Separator
            Constraint::Min(5),    // Tree content
            Constraint::Length(1), // Separator
            Constraint::Length(1), // Help/input line
        ])
        .split(area);

    // Title bar
    let title = " DIRECTORY MAP - Tree View ";
    frame.render_widget(
        Paragraph::new(Span::styled(
            title,
            Style::default().fg(COLOR_FG).add_modifier(Modifier::BOLD),
        )),
        chunks[0],
    );

    // Separator (double line)
    let sep = "═".repeat(area.width as usize);
    frame.render_widget(
        Paragraph::new(Span::styled(sep.clone(), Style::default().fg(COLOR_FG))),
        chunks[1],
    );

    // Tree content area
    let tree_area = chunks[2];
    let visible_height = tree_area.height as usize;

    // Calculate scroll position to keep selected item visible
    let scroll_offset = if state.selected_index >= visible_height {
        state.selected_index - visible_height + 1
    } else {
        0
    };

    // Render tree lines
    let mut lines: Vec<Line> = Vec::new();
    for (i, (path, depth, expanded, has_children)) in state
        .flat_list
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(visible_height)
    {
        let is_selected = i == state.selected_index;

        // Build the tree line with indentation and expand/collapse indicator
        let indent = "  ".repeat(*depth);
        let indicator = if *has_children {
            if *expanded {
                "▼ "
            } else {
                "▶ "
            }
        } else {
            "  "
        };

        // Get the directory name (last component of path)
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        let line_text = format!("{}{}{}", indent, indicator, name);

        let style = if is_selected {
            Style::default().fg(COLOR_YELLOW).bg(COLOR_RED)
        } else {
            Style::default().fg(COLOR_FG)
        };

        // Pad to full width for selection highlighting
        let padded = format!("{:<width$}", line_text, width = tree_area.width as usize);
        lines.push(Line::from(Span::styled(padded, style)));
    }

    frame.render_widget(Paragraph::new(lines), tree_area);

    // Bottom separator
    frame.render_widget(
        Paragraph::new(Span::styled(sep, Style::default().fg(COLOR_FG))),
        chunks[3],
    );

    // Help/input line
    let (help_text, help_style) = if let Some(ref path) = state.confirm_delete {
        let dir_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        (
            format!("Delete '{}'? (Y)es / (N)o / ESC", dir_name),
            Style::default().fg(COLOR_YELLOW),
        )
    } else if let Some(ref mode) = state.input_mode {
        (
            format!("{}: {}█", mode, state.input_buffer),
            Style::default().fg(COLOR_GREEN),
        )
    } else {
        ("↑↓ Navigate  Enter/→ Expand  ←/Backspace Collapse  M Make Dir  D Delete Dir  ESC Close".to_string(), Style::default().fg(COLOR_GREEN))
    };
    frame.render_widget(
        Paragraph::new(Span::styled(help_text, help_style)),
        chunks[4],
    );
}

/// Draw the Find modal
pub(super) fn draw_find_modal(frame: &mut Frame, area: Rect, state: &FindState) {
    // Clear the entire screen
    frame.render_widget(Clear, area);

    // Layout: title, separator, content, separator, help
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title
            Constraint::Length(1), // Separator
            Constraint::Min(5),    // Content
            Constraint::Length(1), // Separator
            Constraint::Length(1), // Help line
        ])
        .split(area);

    // Title
    let title = " FIND FILES ";
    frame.render_widget(
        Paragraph::new(Span::styled(
            title,
            Style::default().fg(COLOR_FG).add_modifier(Modifier::BOLD),
        )),
        chunks[0],
    );

    // Separator
    let sep = "═".repeat(area.width as usize);
    frame.render_widget(
        Paragraph::new(Span::styled(sep.clone(), Style::default().fg(COLOR_FG))),
        chunks[1],
    );

    // Content area based on phase
    let content_area = chunks[2];
    match state.phase {
        FindPhase::InputPattern => {
            let mut lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "Find File -- Search for:",
                    Style::default().fg(COLOR_GREEN),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Pattern: ", Style::default().fg(COLOR_BLUE)),
                    Span::styled(
                        &state.pattern,
                        Style::default().fg(COLOR_YELLOW).bg(COLOR_RED),
                    ),
                    Span::styled("█", Style::default().fg(COLOR_YELLOW).bg(COLOR_RED)),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "Examples: *.txt, foo*.rs, config.*",
                    Style::default().fg(COLOR_GREY),
                )),
                Line::from(Span::styled(
                    "Ctrl+R to recall last pattern",
                    Style::default().fg(COLOR_GREY),
                )),
            ];
            if !state.last_pattern.is_empty() {
                lines.push(Line::from(Span::styled(
                    format!("Last pattern: {}", state.last_pattern),
                    Style::default().fg(COLOR_GREY),
                )));
            }
            frame.render_widget(Paragraph::new(lines), content_area);
        }
        FindPhase::AskPause => {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("Searching for: {}", state.pattern),
                    Style::default().fg(COLOR_GREEN),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Pause when a match is found?  (Y/N)",
                    Style::default().fg(COLOR_FG),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Y = Stop at each match (can Jump/View/Continue)",
                    Style::default().fg(COLOR_GREY),
                )),
                Line::from(Span::styled(
                    "N = Show all matches at once",
                    Style::default().fg(COLOR_GREY),
                )),
            ];
            frame.render_widget(Paragraph::new(lines), content_area);
        }
        FindPhase::Searching => {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "Searching...",
                    Style::default().fg(COLOR_YELLOW),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!("Pattern: {}", state.pattern),
                    Style::default().fg(COLOR_GREEN),
                )),
            ];
            frame.render_widget(Paragraph::new(lines), content_area);
        }
        FindPhase::ShowResult => {
            if let Some((path, display)) = state.matches.get(state.current_match) {
                let lines = vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        format!(
                            "Match {} of {}",
                            state.current_match + 1,
                            state.matches.len()
                        ),
                        Style::default().fg(COLOR_GREEN),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        display.clone(),
                        Style::default().fg(COLOR_YELLOW),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("Path: {}", path.display()),
                        Style::default().fg(COLOR_GREY),
                    )),
                ];
                frame.render_widget(Paragraph::new(lines), content_area);
            }
        }
        FindPhase::ShowAllResults => {
            let visible_height = content_area.height as usize;
            let mut lines: Vec<Line> = vec![
                Line::from(Span::styled(
                    format!(
                        "Found {} matches for '{}':",
                        state.matches.len(),
                        state.pattern
                    ),
                    Style::default().fg(COLOR_GREEN),
                )),
                Line::from(""),
            ];

            for (i, (path, _)) in state
                .matches
                .iter()
                .enumerate()
                .skip(state.scroll_offset)
                .take(visible_height.saturating_sub(2))
            {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string());
                let parent = path
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                let line_text = format!("{:4}. {} - {}", i + 1, name, parent);
                // Highlight the selected item
                let style = if i == state.current_match {
                    Style::default().fg(COLOR_YELLOW).bg(COLOR_RED)
                } else {
                    Style::default().fg(COLOR_FG)
                };
                lines.push(Line::from(Span::styled(line_text, style)));
            }

            frame.render_widget(Paragraph::new(lines), content_area);
        }
        FindPhase::NoResults => {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("Pattern: {}", state.pattern),
                    Style::default().fg(COLOR_GREEN),
                )),
                Line::from(""),
                if state.search_complete && state.matches.is_empty() {
                    Line::from(Span::styled(
                        "No matching files found.",
                        Style::default().fg(COLOR_YELLOW),
                    ))
                } else {
                    Line::from(Span::styled(
                        format!("Finished with FIND -- {} files found", state.matches.len()),
                        Style::default().fg(COLOR_YELLOW),
                    ))
                },
                Line::from(""),
                Line::from(Span::styled(
                    "Press any key to continue",
                    Style::default().fg(COLOR_GREEN),
                )),
            ];
            frame.render_widget(Paragraph::new(lines), content_area);
        }
    }

    // Bottom separator
    frame.render_widget(
        Paragraph::new(Span::styled(sep, Style::default().fg(COLOR_FG))),
        chunks[3],
    );

    // Help line based on phase
    let help_text = match state.phase {
        FindPhase::InputPattern => "Enter pattern, Ctrl+R recall, ESC cancel",
        FindPhase::AskPause => "Y pause on match, N show all, ESC cancel",
        FindPhase::Searching => "Searching...",
        FindPhase::ShowResult => "(C)ontinue (J)ump (V)iew  ESC quit",
        FindPhase::ShowAllResults => "↑↓ select  Enter/(J)ump  (V)iew  ESC close",
        FindPhase::NoResults => "Press any key",
    };
    frame.render_widget(
        Paragraph::new(Span::styled(help_text, Style::default().fg(COLOR_GREEN))),
        chunks[4],
    );
}

/// Draw the Batch Rename modal
pub(super) fn draw_batch_rename_modal(frame: &mut Frame, area: Rect, state: &BatchRenameState) {
    // Clear the entire screen
    frame.render_widget(Clear, area);

    // Layout: title, separator, content, separator, help
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title
            Constraint::Length(1), // Separator
            Constraint::Min(5),    // Content
            Constraint::Length(1), // Separator
            Constraint::Length(1), // Help line
        ])
        .split(area);

    // Title
    let title = format!(
        " RENAME FILES - {} of {} ",
        state.current_index + 1,
        state.files.len()
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            title,
            Style::default().fg(COLOR_FG).add_modifier(Modifier::BOLD),
        )),
        chunks[0],
    );

    // Separator
    let sep = "═".repeat(area.width as usize);
    frame.render_widget(
        Paragraph::new(Span::styled(sep.clone(), Style::default().fg(COLOR_FG))),
        chunks[1],
    );

    // Content area
    let content_area = chunks[2];

    if let Some((path, original_name)) = state.current_file() {
        let mut lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "File to be renamed:",
                Style::default().fg(COLOR_GREEN),
            )),
            Line::from(""),
            Line::from(Span::styled(
                original_name.clone(),
                Style::default().fg(COLOR_FG),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("Path: {}", path.parent().unwrap_or(path).display()),
                Style::default().fg(COLOR_GREY),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Enter new name:",
                Style::default().fg(COLOR_GREEN),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    &state.input,
                    Style::default().fg(COLOR_YELLOW).bg(COLOR_RED),
                ),
                Span::styled("█", Style::default().fg(COLOR_YELLOW).bg(COLOR_RED)),
            ]),
        ];

        // Show error if any
        if let Some(ref error) = state.last_error {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                error.clone(),
                Style::default().fg(COLOR_RED),
            )));
        }

        // Show progress
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("Renamed so far: {}", state.renamed_count),
            Style::default().fg(COLOR_GREY),
        )));

        frame.render_widget(Paragraph::new(lines), content_area);
    }

    // Bottom separator
    frame.render_widget(
        Paragraph::new(Span::styled(sep, Style::default().fg(COLOR_FG))),
        chunks[3],
    );

    // Help line
    let help_text = "Enter: Rename  Tab: Skip  ESC: Exit";
    frame.render_widget(
        Paragraph::new(Span::styled(help_text, Style::default().fg(COLOR_GREEN))),
        chunks[4],
    );
}

/// Draw the Attribute modal
pub(super) fn draw_attribute_modal(frame: &mut Frame, area: Rect, state: &AttributeState) {
    let attr_block = Block::default()
        .title(if state.display_only {
            " Display File Attributes "
        } else if state.for_tagged {
            " Change Tagged Files Attributes "
        } else {
            " Change File Attributes "
        })
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BLUE))
        .style(Style::default().bg(COLOR_BG));

    frame.render_widget(attr_block.clone(), area);
    let inner = attr_block.inner(area);

    // Build attribute display lines
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("File: ", Style::default().fg(COLOR_GREEN)),
            Span::styled(&state.name, Style::default().fg(COLOR_FG)),
        ]),
        Line::from(""),
    ];

    // Show current attributes row
    lines.push(Line::from(Span::styled(
        "Current attributes:",
        Style::default().fg(COLOR_GREEN),
    )));
    lines.push(Line::from(""));

    // Show original values
    let orig_text = format!(
        "  Original: {} {} {} {}",
        if state.original[0] { "HID" } else { "   " },
        if state.original[1] { "SYS" } else { "   " },
        if state.original[2] { "R/O" } else { "   " },
        if state.original[3] { "ARC" } else { "   " },
    );
    lines.push(Line::from(Span::styled(
        orig_text,
        Style::default().fg(COLOR_GREY),
    )));
    lines.push(Line::from(""));

    // Build attribute bars
    let mut attr_spans: Vec<Span> = vec![Span::raw("  ")];
    for i in 0..4 {
        let name = AttributeState::attr_name(i);
        let value = state.attrs[i];

        // Determine if this attribute is modifiable
        // Only R/O (index 2) is modifiable on Unix
        let is_modifiable = i == 2 && !state.display_only;
        let is_selected = i == state.selected && !state.display_only;

        let style = if is_selected {
            Style::default().fg(COLOR_YELLOW).bg(COLOR_RED)
        } else if is_modifiable {
            Style::default().fg(COLOR_FG)
        } else {
            Style::default().fg(COLOR_GREY)
        };

        let value_style = if is_selected {
            Style::default().fg(COLOR_YELLOW).bg(COLOR_RED)
        } else {
            match value {
                AttrValue::On => Style::default().fg(COLOR_GREEN),
                AttrValue::Off => Style::default().fg(COLOR_GREY),
                AttrValue::NoChange => Style::default().fg(COLOR_BLUE),
            }
        };

        attr_spans.push(Span::styled(format!("[ {} ", name), style));
        attr_spans.push(Span::styled(value.as_str(), value_style));
        attr_spans.push(Span::styled(" ]  ", style));
    }
    lines.push(Line::from(attr_spans));
    lines.push(Line::from(""));

    // Help text
    if state.display_only {
        lines.push(Line::from(Span::styled(
            "Press any key to close",
            Style::default().fg(COLOR_GREEN),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "Note: Only R/O (Read-Only) can be changed on Unix",
            Style::default().fg(COLOR_GREY),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("←→", Style::default().fg(COLOR_BLUE)),
            Span::raw(" select  "),
            Span::styled("SPACE", Style::default().fg(COLOR_BLUE)),
            Span::raw(" toggle  "),
            Span::styled("Enter", Style::default().fg(COLOR_BLUE)),
            Span::raw(" apply  "),
            Span::styled("ESC", Style::default().fg(COLOR_BLUE)),
            Span::raw(" cancel"),
        ]));
    }

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true });

    frame.render_widget(paragraph, inner);
}

/// Draw color theme selection modal (QDCOLOR)
fn draw_color_theme_modal(frame: &mut Frame, area: Rect, state: &ColorThemeState, app: &App) {
    let colors = app.colors();

    let theme_block = Block::default()
        .title(" R-DOS Color Configuration ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors.blue()))
        .style(Style::default().bg(colors.bg()));

    frame.render_widget(theme_block.clone(), area);
    let inner = theme_block.inner(area);

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Current Theme: ", Style::default().fg(colors.green())),
            Span::styled(
                app.color_theme.name(),
                Style::default().fg(colors.yellow()).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Select a color theme:",
            Style::default().fg(colors.fg()),
        )),
        Line::from(""),
    ];

    // List all themes
    for (i, theme) in ColorTheme::ALL.iter().enumerate() {
        let is_selected = i == state.selected;
        let number = format!("{}. ", i + 1);
        let name = format!("{:<12}", theme.name());
        let desc = theme.description();

        let style = if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            Style::default().fg(colors.fg())
        };

        let number_style = if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            Style::default().fg(colors.blue())
        };

        let desc_style = if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            Style::default().fg(colors.green())
        };

        lines.push(Line::from(vec![
            Span::styled("  ", style),
            Span::styled(number, number_style),
            Span::styled(name, style),
            Span::styled(desc, desc_style),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Theme changes preview live as you select.",
        Style::default().fg(colors.grey()),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("↑↓/1-5", Style::default().fg(colors.blue())),
        Span::raw(" select  "),
        Span::styled("Enter", Style::default().fg(colors.blue())),
        Span::raw(" apply  "),
        Span::styled("ESC", Style::default().fg(colors.blue())),
        Span::raw(" cancel"),
    ]));

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, inner);
}

/// Draw QDSTART configuration modal
fn draw_qdstart_modal(frame: &mut Frame, area: Rect, state: &QdstartState, app: &App) {
    let colors = app.colors();

    // Clear the entire area
    frame.render_widget(Clear, area);

    // Layout: title, separator, content, separator, help
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title
            Constraint::Length(1), // Separator
            Constraint::Min(12),   // Content
            Constraint::Length(1), // Separator
            Constraint::Length(1), // Help line
        ])
        .split(area);

    // Title
    let title = " R-DOS STARTUP CONFIGURATION ";
    frame.render_widget(
        Paragraph::new(Span::styled(
            title,
            Style::default()
                .fg(colors.fg())
                .add_modifier(Modifier::BOLD),
        )),
        chunks[0],
    );

    // Separator
    let sep = "═".repeat(area.width as usize);
    frame.render_widget(
        Paragraph::new(Span::styled(sep.clone(), Style::default().fg(colors.fg()))),
        chunks[1],
    );

    // Content area
    let content_area = chunks[2];
    let mut lines: Vec<Line> = vec![Line::from("")];

    for (i, field) in QdstartField::ALL.iter().enumerate() {
        let is_selected = i == state.selected;
        let is_editing = is_selected && state.editing;

        // Get field name and value
        let name = field.name();
        let value = match field {
            QdstartField::SearchSpec => {
                if is_editing {
                    format!("{}█", state.input_buffer)
                } else {
                    state.search_spec.clone()
                }
            }
            QdstartField::SortMethod => state.sort_method_name().to_string(),
            QdstartField::SortDirection => {
                if state.sort_asc {
                    "Ascending".to_string()
                } else {
                    "Descending".to_string()
                }
            }
            QdstartField::ShowHidden => {
                if state.show_hidden {
                    "Yes".to_string()
                } else {
                    "No".to_string()
                }
            }
            QdstartField::ConfirmDelete => {
                if state.confirm_delete {
                    "Yes".to_string()
                } else {
                    "No".to_string()
                }
            }
            QdstartField::Editor => {
                if is_editing {
                    format!("{}█", state.input_buffer)
                } else {
                    state
                        .editor
                        .clone()
                        .unwrap_or_else(|| "$EDITOR".to_string())
                }
            }
            QdstartField::ColorTheme => state.theme().name().to_string(),
            QdstartField::MouseSupport => {
                if state.mouse_support {
                    "Yes".to_string()
                } else {
                    "No".to_string()
                }
            }
            QdstartField::UppercaseNames => {
                if state.uppercase_names {
                    "Yes".to_string()
                } else {
                    "No".to_string()
                }
            }
        };

        // Style based on selection
        let line_style = if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            Style::default().fg(colors.fg())
        };

        let name_style = if is_selected {
            Style::default()
                .fg(colors.yellow())
                .bg(colors.red())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.blue())
        };

        let value_style = if is_editing {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            Style::default().fg(colors.green())
        };

        // Format as "  Field Name:        Value"
        let padded_name = format!("  {:<22}", format!("{}:", name));
        let padded_value = format!("{:<20}", value);

        lines.push(Line::from(vec![
            Span::styled(padded_name, name_style),
            Span::styled(padded_value, value_style),
            Span::styled(
                " ".repeat(area.width.saturating_sub(44) as usize),
                line_style,
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Settings will be saved to ~/.config/rdos/config.toml",
        Style::default().fg(colors.grey()),
    )));

    frame.render_widget(Paragraph::new(lines), content_area);

    // Bottom separator
    frame.render_widget(
        Paragraph::new(Span::styled(sep, Style::default().fg(colors.fg()))),
        chunks[3],
    );

    // Help line
    let help_text = if state.editing {
        "Type value, Enter to confirm, ESC to cancel"
    } else {
        "↑↓ select  Enter/Space toggle  S save  ESC close"
    };
    frame.render_widget(
        Paragraph::new(Span::styled(help_text, Style::default().fg(colors.green()))),
        chunks[4],
    );
}
