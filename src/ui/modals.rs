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
    App, AttrValue, AttributeState, BatchRenameState, BeadsMenuItem, BeadsState, BeadsView,
    ColorTheme, ColorThemeState, DirectoryMapState, FindPhase, FindState, GitMenuItem, GitState,
    GitView, HelpState, Modal, ProgressState, QdstartField, QdstartState, RemoteAction,
    SearchSpecState,
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
    format_size_short,
    viewer::{draw_file_viewer, draw_shell_command},
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
    // Minimum size check for modals - avoid crashes on very small terminals
    if area.width < 20 || area.height < 10 {
        return; // Too small to render any modal safely
    }

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
        Modal::Space => {}      // Handled above
        Modal::Error(_) => {}   // Handled above
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
        Modal::Git(state) => draw_git_modal(frame, area, state, app),
        Modal::Beads(state) => draw_beads_modal(frame, area, state, app),
        Modal::Plugin(_plugin_id) => {
            // Delegate to the plugin manager to draw the active plugin's modal
            app.plugin_manager.draw_modal(frame, area);
        }
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
                    Style::default()
                        .fg(COLOR_YELLOW)
                        .add_modifier(Modifier::BOLD),
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
    let inner_width = (width as usize).saturating_sub(2);

    // Line 0: Top border
    let top = format!("╔{}╗", "═".repeat(inner_width));
    frame.render_widget(
        Paragraph::new(Span::styled(&top, style)),
        Rect::new(quit_area.x, quit_area.y, quit_area.width, 1),
    );

    // Line 1: Title row
    let title = "F10 - Quit Q-DOS II";
    let title_line = format!("║{:^w$}║", title, w = inner_width);
    frame.render_widget(
        Paragraph::new(Span::styled(&title_line, style)),
        Rect::new(quit_area.x, quit_area.y + 1, quit_area.width, 1),
    );

    // Line 2: Separator (double line)
    let sep = format!("╠{}╣", "═".repeat(inner_width));
    frame.render_widget(
        Paragraph::new(Span::styled(&sep, style)),
        Rect::new(quit_area.x, quit_area.y + 2, quit_area.width, 1),
    );

    // Line 3: Empty row
    let empty = format!("║{:w$}║", "", w = inner_width);
    frame.render_widget(
        Paragraph::new(Span::styled(&empty, style)),
        Rect::new(quit_area.x, quit_area.y + 3, quit_area.width, 1),
    );

    // Line 4: First message
    let msg1 = "Press F10 again to quit, or RETURN for options";
    let msg1_line = format!("║{:^w$}║", msg1, w = inner_width);
    frame.render_widget(
        Paragraph::new(Span::styled(&msg1_line, style)),
        Rect::new(quit_area.x, quit_area.y + 4, quit_area.width, 1),
    );

    // Line 5: Empty row
    frame.render_widget(
        Paragraph::new(Span::styled(&empty, style)),
        Rect::new(quit_area.x, quit_area.y + 5, quit_area.width, 1),
    );

    // Line 6: Second message
    let msg2 = "Press ESC to return to Q-DOS II";
    let msg2_line = format!("║{:^w$}║", msg2, w = inner_width);
    frame.render_widget(
        Paragraph::new(Span::styled(&msg2_line, style)),
        Rect::new(quit_area.x, quit_area.y + 6, quit_area.width, 1),
    );

    // Line 7: Bottom border
    let bottom = format!("╚{}╝", "═".repeat(inner_width));
    frame.render_widget(
        Paragraph::new(Span::styled(&bottom, style)),
        Rect::new(quit_area.x, quit_area.y + 7, quit_area.width, 1),
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
    let modal_area = Rect::new(
        x,
        y,
        modal_width.min(area.width),
        modal_height.min(area.height),
    );

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
        let right_pad = padding.saturating_sub(left_pad);
        row_spans.insert(1, Span::styled(" ".repeat(left_pad), content_style));
        row_spans.push(Span::styled(" ".repeat(right_pad), content_style));
        row_spans.push(right_border);

        frame.render_widget(
            Paragraph::new(Line::from(row_spans)).style(content_style),
            Rect::new(modal_area.x, row_y, modal_area.width, 1),
        );
    }

    let bottom = format!("╚{}╝", "═".repeat(width.saturating_sub(2)));
    let bottom_y = modal_area
        .y
        .saturating_add(modal_area.height.saturating_sub(1));
    frame.render_widget(
        Paragraph::new(Span::styled(&bottom, border_style)),
        Rect::new(modal_area.x, bottom_y, modal_area.width, 1),
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
    let modal_area = Rect::new(
        x,
        y,
        modal_width.min(area.width),
        modal_height.min(area.height),
    );

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
        let right_pad = padding.saturating_sub(left_pad);

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
    let bottom_y = modal_area.y.saturating_add(modal_height.saturating_sub(1));
    frame.render_widget(
        Paragraph::new(Span::styled(&bottom, border_style)),
        Rect::new(modal_area.x, bottom_y, modal_area.width, 1),
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
            Span::styled(
                format_size_short(available),
                Style::default().fg(colors.cyan()),
            ),
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
        Line::from(Span::styled(progress_bar, Style::default().fg(COLOR_BLUE))),
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
        format!("Completed: {}  Failed: {}", state.completed, state.failed),
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
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
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

/// Draw Git modal
fn draw_git_modal(frame: &mut Frame, area: Rect, state: &GitState, app: &App) {
    let colors = app.colors();

    // Clear the entire area
    frame.render_widget(Clear, area);

    // Layout: title, separator, content, separator, help
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title
            Constraint::Length(1), // Separator
            Constraint::Min(10),   // Content
            Constraint::Length(1), // Separator
            Constraint::Length(1), // Help line
        ])
        .split(area);

    // Title based on current view
    let title = match state.view {
        GitView::Menu => " GIT INTEGRATION ",
        GitView::Status => " GIT STATUS ",
        GitView::Log => " GIT LOG ",
        GitView::Diff => " GIT DIFF ",
        GitView::Commit => " GIT COMMIT ",
        GitView::Branch => " GIT BRANCHES ",
        GitView::Stash => " GIT STASH ",
        GitView::Tag => " GIT TAGS ",
        GitView::Remote => match state.remote_action {
            RemoteAction::Push => " GIT PUSH TO REMOTE ",
            RemoteAction::Pull => " GIT PULL FROM REMOTE ",
        },
        GitView::Config => " GIT CONFIG ",
        GitView::Conflicts => " MERGE CONFLICTS ",
        GitView::Submodules => " GIT SUBMODULES ",
    };
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

    if !state.is_repo {
        // Not a git repo
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "Not a Git repository",
                Style::default().fg(colors.yellow()),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Initialize a git repository with 'git init'",
                Style::default().fg(colors.grey()),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press any key to close",
                Style::default().fg(colors.green()),
            )),
        ];
        frame.render_widget(Paragraph::new(lines), content_area);
    } else {
        match state.view {
            GitView::Menu => {
                let mut lines = vec![Line::from("")];

                for (i, item) in GitMenuItem::ALL.iter().enumerate() {
                    let is_selected = i == state.menu_selected;
                    let style = if is_selected {
                        Style::default().fg(colors.yellow()).bg(colors.red())
                    } else {
                        Style::default().fg(colors.fg())
                    };

                    let key = match item {
                        GitMenuItem::Status => "S",
                        GitMenuItem::Log => "L",
                        GitMenuItem::Diff => "D",
                        GitMenuItem::Commit => "C",
                        GitMenuItem::Push => "P",
                        GitMenuItem::Pull => "U",
                        GitMenuItem::Branch => "B",
                        GitMenuItem::Stash => "H",
                        GitMenuItem::Tag => "T",
                        GitMenuItem::Config => "G",
                        GitMenuItem::Conflicts => "X",
                        GitMenuItem::Submodules => "M",
                    };

                    lines.push(Line::from(vec![
                        Span::styled("  ", style),
                        Span::styled(
                            format!("[{}] ", key),
                            if is_selected {
                                style
                            } else {
                                Style::default().fg(colors.blue())
                            },
                        ),
                        Span::styled(format!("{:<12}", item.as_str()), style),
                        Span::styled(
                            item.description(),
                            if is_selected {
                                style
                            } else {
                                Style::default().fg(colors.grey())
                            },
                        ),
                    ]));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            GitView::Status => {
                let visible_height = content_area.height as usize;
                let mut lines: Vec<Line> = vec![];

                if state.files.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Working tree clean",
                        Style::default().fg(colors.green()),
                    )));
                } else {
                    for (i, file) in state.files.iter().enumerate().take(visible_height) {
                        let is_selected = i == state.selected_file;
                        let status_char = match file.status {
                            'M' => "M",
                            'A' => "A",
                            'D' => "D",
                            'R' => "R",
                            '?' => "?",
                            _ => " ",
                        };
                        let staged_indicator = if file.staged { "+" } else { " " };

                        let style = if is_selected {
                            Style::default().fg(colors.yellow()).bg(colors.red())
                        } else {
                            Style::default().fg(colors.fg())
                        };

                        let status_style = if is_selected {
                            style
                        } else {
                            match file.status {
                                'M' => Style::default().fg(colors.yellow()),
                                'A' => Style::default().fg(colors.green()),
                                'D' => Style::default().fg(colors.red()),
                                '?' => Style::default().fg(colors.grey()),
                                _ => Style::default().fg(colors.fg()),
                            }
                        };

                        lines.push(Line::from(vec![
                            Span::styled(format!(" {} ", staged_indicator), status_style),
                            Span::styled(format!("{} ", status_char), status_style),
                            Span::styled(&file.path, style),
                        ]));
                    }
                }

                if let Some(ref err) = state.error {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("Error: {}", err),
                        Style::default().fg(colors.red()),
                    )));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            GitView::Log => {
                let visible_height = content_area.height as usize / 2; // Each entry takes 2 lines
                let mut lines: Vec<Line> = vec![];

                // Calculate scroll offset based on selection
                let scroll = if state.selected_log >= visible_height {
                    state.selected_log - visible_height + 1
                } else {
                    0
                };

                for (i, entry) in state
                    .log_entries
                    .iter()
                    .enumerate()
                    .skip(scroll)
                    .take(visible_height)
                {
                    let is_selected = i == state.selected_log;
                    let prefix = if is_selected { "▶ " } else { "  " };
                    let hash_style = if is_selected {
                        Style::default()
                            .fg(colors.yellow())
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(colors.yellow())
                    };
                    let msg_style = if is_selected {
                        Style::default()
                            .fg(colors.fg())
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(colors.fg())
                    };

                    lines.push(Line::from(vec![
                        Span::styled(prefix, hash_style),
                        Span::styled(format!("{} ", entry.hash), hash_style),
                        Span::styled(&entry.message, msg_style),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("         ", Style::default()),
                        Span::styled(
                            format!("{} - {}", entry.author, entry.date),
                            Style::default().fg(colors.grey()),
                        ),
                    ]));
                }

                if let Some(ref err) = state.error {
                    lines.push(Line::from(Span::styled(
                        format!("Error: {}", err),
                        Style::default().fg(colors.red()),
                    )));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            GitView::Diff => {
                let visible_height = content_area.height as usize;
                let mut lines: Vec<Line> = vec![];

                for line_text in state
                    .diff_content
                    .iter()
                    .skip(state.scroll_offset)
                    .take(visible_height)
                {
                    let style = if line_text.starts_with('+') && !line_text.starts_with("+++") {
                        Style::default().fg(colors.green())
                    } else if line_text.starts_with('-') && !line_text.starts_with("---") {
                        Style::default().fg(colors.red())
                    } else if line_text.starts_with("@@") {
                        Style::default().fg(colors.cyan())
                    } else if line_text.starts_with("diff") || line_text.starts_with("index") {
                        Style::default().fg(colors.yellow())
                    } else {
                        Style::default().fg(colors.fg())
                    };

                    lines.push(Line::from(Span::styled(line_text.clone(), style)));
                }

                if let Some(ref err) = state.error {
                    lines.push(Line::from(Span::styled(
                        format!("Error: {}", err),
                        Style::default().fg(colors.red()),
                    )));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            GitView::Commit => {
                let mut lines = vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "Enter commit message:",
                        Style::default().fg(colors.green()),
                    )),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled(
                            &state.commit_message,
                            Style::default().fg(colors.yellow()).bg(colors.red()),
                        ),
                        Span::styled("█", Style::default().fg(colors.yellow()).bg(colors.red())),
                    ]),
                ];

                if let Some(ref err) = state.error {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("Error: {}", err),
                        Style::default().fg(colors.red()),
                    )));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            GitView::Branch => {
                let visible_height = content_area.height as usize;
                let mut lines: Vec<Line> = vec![];

                if state.branch_input_mode {
                    // Show branch name input
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Create new branch:",
                        Style::default().fg(colors.green()),
                    )));
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled(
                            &state.branch_name_input,
                            Style::default().fg(colors.yellow()).bg(colors.red()),
                        ),
                        Span::styled("█", Style::default().fg(colors.yellow()).bg(colors.red())),
                    ]));
                } else if state.branches.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "No branches found",
                        Style::default().fg(colors.grey()),
                    )));
                } else {
                    // List branches
                    for (i, branch) in state.branches.iter().enumerate() {
                        if i >= visible_height {
                            break;
                        }

                        let is_selected = i == state.selected_branch;
                        let style = if is_selected {
                            Style::default().fg(colors.yellow()).bg(colors.red())
                        } else {
                            Style::default().fg(colors.fg())
                        };

                        let marker = if branch.is_current { "* " } else { "  " };
                        let marker_style = if branch.is_current {
                            Style::default().fg(colors.green())
                        } else {
                            style
                        };

                        // Truncate commit message
                        let commit_display = if branch.last_commit.len() > 40 {
                            format!("{}...", &branch.last_commit[..37])
                        } else {
                            branch.last_commit.clone()
                        };

                        lines.push(Line::from(vec![
                            Span::styled(marker, marker_style),
                            Span::styled(format!("{:<20} ", branch.name), style),
                            Span::styled(
                                commit_display,
                                if is_selected {
                                    style
                                } else {
                                    Style::default().fg(colors.grey())
                                },
                            ),
                        ]));
                    }
                }

                if let Some(ref err) = state.error {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("Error: {}", err),
                        Style::default().fg(colors.red()),
                    )));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            GitView::Stash => {
                let visible_height = content_area.height as usize;
                let mut lines: Vec<Line> = vec![];

                if state.stash_input_mode {
                    // Show stash message input
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Stash message (optional):",
                        Style::default().fg(colors.green()),
                    )));
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled(
                            &state.stash_message_input,
                            Style::default().fg(colors.yellow()).bg(colors.red()),
                        ),
                        Span::styled("█", Style::default().fg(colors.yellow()).bg(colors.red())),
                    ]));
                } else if state.stashes.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "No stashes",
                        Style::default().fg(colors.grey()),
                    )));
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Press S to stash current changes",
                        Style::default().fg(colors.green()),
                    )));
                } else {
                    // List stashes
                    for (i, stash) in state.stashes.iter().enumerate() {
                        if i >= visible_height {
                            break;
                        }

                        let is_selected = i == state.selected_stash;
                        let style = if is_selected {
                            Style::default().fg(colors.yellow()).bg(colors.red())
                        } else {
                            Style::default().fg(colors.fg())
                        };

                        // Truncate message
                        let msg_display = if stash.message.len() > 50 {
                            format!("{}...", &stash.message[..47])
                        } else {
                            stash.message.clone()
                        };

                        lines.push(Line::from(vec![
                            Span::styled(format!("  stash@{{{}}}: ", stash.index), style),
                            Span::styled(
                                msg_display,
                                if is_selected {
                                    style
                                } else {
                                    Style::default().fg(colors.grey())
                                },
                            ),
                        ]));
                    }
                }

                if let Some(ref err) = state.error {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("Error: {}", err),
                        Style::default().fg(colors.red()),
                    )));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            GitView::Tag => {
                let visible_height = content_area.height as usize;
                let mut lines: Vec<Line> = vec![];

                if state.tag_input_mode {
                    // Show tag name input
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Create new tag:",
                        Style::default().fg(colors.green()),
                    )));
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled(
                            &state.tag_name_input,
                            Style::default().fg(colors.yellow()).bg(colors.red()),
                        ),
                        Span::styled("█", Style::default().fg(colors.yellow()).bg(colors.red())),
                    ]));
                } else if state.tags.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "No tags",
                        Style::default().fg(colors.grey()),
                    )));
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Press N to create a new tag",
                        Style::default().fg(colors.green()),
                    )));
                } else {
                    // List tags
                    for (i, tag) in state.tags.iter().enumerate() {
                        if i >= visible_height {
                            break;
                        }

                        let is_selected = i == state.selected_tag;
                        let style = if is_selected {
                            Style::default().fg(colors.yellow()).bg(colors.red())
                        } else {
                            Style::default().fg(colors.fg())
                        };

                        let msg = tag
                            .message
                            .as_ref()
                            .map(|m| {
                                if m.len() > 40 {
                                    format!(" - {}...", &m[..37])
                                } else {
                                    format!(" - {}", m)
                                }
                            })
                            .unwrap_or_default();

                        lines.push(Line::from(vec![
                            Span::styled(format!("  {:<20} ", tag.name), style),
                            Span::styled(
                                tag.commit.clone(),
                                if is_selected {
                                    style
                                } else {
                                    Style::default().fg(colors.blue())
                                },
                            ),
                            Span::styled(
                                msg,
                                if is_selected {
                                    style
                                } else {
                                    Style::default().fg(colors.grey())
                                },
                            ),
                        ]));
                    }
                }

                if let Some(ref err) = state.error {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("Error: {}", err),
                        Style::default().fg(colors.red()),
                    )));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            GitView::Remote => {
                let visible_height = content_area.height as usize;
                let mut lines: Vec<Line> = vec![];

                let action_text = match state.remote_action {
                    RemoteAction::Push => "Push to",
                    RemoteAction::Pull => "Pull from",
                };

                if state.remotes.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "No remotes configured",
                        Style::default().fg(colors.grey()),
                    )));
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Use 'git remote add <name> <url>' to add a remote",
                        Style::default().fg(colors.green()),
                    )));
                } else {
                    lines.push(Line::from(Span::styled(
                        format!("Select remote to {}:", action_text),
                        Style::default().fg(colors.green()),
                    )));
                    lines.push(Line::from(""));

                    // List remotes
                    for (i, remote) in state.remotes.iter().enumerate() {
                        if i + 2 >= visible_height {
                            break;
                        }

                        let is_selected = i == state.selected_remote;
                        let style = if is_selected {
                            Style::default().fg(colors.yellow()).bg(colors.red())
                        } else {
                            Style::default().fg(colors.fg())
                        };

                        // Truncate URL if too long
                        let url = if remote.url.len() > 50 {
                            format!("{}...", &remote.url[..47])
                        } else {
                            remote.url.clone()
                        };

                        lines.push(Line::from(vec![
                            Span::styled(format!("  {:<12} ", remote.name), style),
                            Span::styled(
                                url,
                                if is_selected {
                                    style
                                } else {
                                    Style::default().fg(colors.blue())
                                },
                            ),
                        ]));
                    }
                }

                if let Some(ref err) = state.error {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("Error: {}", err),
                        Style::default().fg(colors.red()),
                    )));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            GitView::Config => {
                let visible_height = content_area.height as usize;
                let mut lines: Vec<Line> = vec![];

                if state.config_entries.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "No config entries found",
                        Style::default().fg(colors.grey()),
                    )));
                } else {
                    // Calculate scroll offset
                    let start = if state.selected_config >= visible_height {
                        state.selected_config - visible_height + 1
                    } else {
                        0
                    };

                    // List config entries
                    for (idx, entry) in state.config_entries.iter().enumerate().skip(start) {
                        if idx >= start + visible_height {
                            break;
                        }

                        let is_selected = idx == state.selected_config;
                        let style = if is_selected {
                            Style::default().fg(colors.yellow()).bg(colors.red())
                        } else {
                            Style::default().fg(colors.fg())
                        };

                        // Format: [scope] key = value
                        let scope_color = match entry.scope.as_str() {
                            "local" => colors.green(),
                            "global" => colors.blue(),
                            _ => colors.grey(),
                        };

                        // Truncate value if too long
                        let max_val_len = 40;
                        let value = if entry.value.len() > max_val_len {
                            format!("{}...", &entry.value[..max_val_len - 3])
                        } else {
                            entry.value.clone()
                        };

                        lines.push(Line::from(vec![
                            Span::styled(
                                format!("[{:6}] ", entry.scope),
                                if is_selected {
                                    style
                                } else {
                                    Style::default().fg(scope_color)
                                },
                            ),
                            Span::styled(format!("{:<30} ", entry.key), style),
                            Span::styled("= ", Style::default().fg(colors.grey())),
                            Span::styled(
                                value,
                                if is_selected {
                                    style
                                } else {
                                    Style::default().fg(colors.cyan())
                                },
                            ),
                        ]));
                    }
                }

                if let Some(ref err) = state.error {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("Error: {}", err),
                        Style::default().fg(colors.red()),
                    )));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            GitView::Conflicts => {
                let mut lines = vec![];

                if state.conflict_files.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  No merge conflicts detected",
                        Style::default().fg(colors.green()),
                    )));
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  All conflicts have been resolved or there is no merge in progress.",
                        Style::default().fg(colors.grey()),
                    )));
                } else {
                    // Header
                    lines.push(Line::from(vec![Span::styled(
                        format!(
                            "  {} conflicting file(s) - ←→ to switch files",
                            state.conflict_files.len()
                        ),
                        Style::default().fg(colors.yellow()),
                    )]));
                    lines.push(Line::from(""));

                    // Current file info
                    let file = &state.conflict_files[state.selected_conflict_file];
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!(
                                "  File [{}/{}]: ",
                                state.selected_conflict_file + 1,
                                state.conflict_files.len()
                            ),
                            Style::default().fg(colors.grey()),
                        ),
                        Span::styled(&file.path, Style::default().fg(colors.blue())),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::styled(
                            format!("{} conflict section(s)", file.sections.len()),
                            Style::default().fg(colors.red()),
                        ),
                    ]));
                    lines.push(Line::from(""));

                    // Show sections
                    let visible_height = content_area.height.saturating_sub(8) as usize;
                    for (i, section) in file.sections.iter().enumerate().take(visible_height / 6) {
                        let is_selected = i == file.selected_section;
                        let bg = if is_selected {
                            colors.red()
                        } else {
                            Color::Reset
                        };
                        let prefix = if is_selected { "▶ " } else { "  " };

                        // Section header
                        lines.push(Line::from(vec![
                            Span::styled(prefix, Style::default().fg(colors.yellow()).bg(bg)),
                            Span::styled(
                                format!("Conflict {} (line {})", i + 1, section.start_line),
                                Style::default()
                                    .fg(colors.cyan())
                                    .bg(bg)
                                    .add_modifier(Modifier::BOLD),
                            ),
                        ]));

                        // Ours section (green)
                        lines.push(Line::from(vec![
                            Span::styled("    ", Style::default().bg(bg)),
                            Span::styled(
                                "<<<< OURS (current branch)",
                                Style::default().fg(colors.green()).bg(bg),
                            ),
                        ]));
                        for (j, line) in section.ours.iter().take(3).enumerate() {
                            let truncated = if line.len() > 60 {
                                format!("{}...", &line[..57])
                            } else {
                                line.clone()
                            };
                            lines.push(Line::from(vec![
                                Span::styled("      ", Style::default().bg(bg)),
                                Span::styled(
                                    format!("{}: {}", j + 1, truncated),
                                    Style::default().fg(colors.green()).bg(bg),
                                ),
                            ]));
                        }
                        if section.ours.len() > 3 {
                            lines.push(Line::from(vec![
                                Span::styled("      ", Style::default().bg(bg)),
                                Span::styled(
                                    format!("... and {} more lines", section.ours.len() - 3),
                                    Style::default().fg(colors.grey()).bg(bg),
                                ),
                            ]));
                        }

                        // Theirs section (red/yellow)
                        lines.push(Line::from(vec![
                            Span::styled("    ", Style::default().bg(bg)),
                            Span::styled(
                                ">>>> THEIRS (incoming)",
                                Style::default().fg(colors.yellow()).bg(bg),
                            ),
                        ]));
                        for (j, line) in section.theirs.iter().take(3).enumerate() {
                            let truncated = if line.len() > 60 {
                                format!("{}...", &line[..57])
                            } else {
                                line.clone()
                            };
                            lines.push(Line::from(vec![
                                Span::styled("      ", Style::default().bg(bg)),
                                Span::styled(
                                    format!("{}: {}", j + 1, truncated),
                                    Style::default().fg(colors.yellow()).bg(bg),
                                ),
                            ]));
                        }
                        if section.theirs.len() > 3 {
                            lines.push(Line::from(vec![
                                Span::styled("      ", Style::default().bg(bg)),
                                Span::styled(
                                    format!("... and {} more lines", section.theirs.len() - 3),
                                    Style::default().fg(colors.grey()).bg(bg),
                                ),
                            ]));
                        }

                        lines.push(Line::from(""));
                    }
                }

                if let Some(ref err) = state.error {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("Error: {}", err),
                        Style::default().fg(colors.red()),
                    )));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            GitView::Submodules => {
                let visible_height = content_area.height as usize;
                let mut lines: Vec<Line> = vec![];

                if state.submodules.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "No submodules found",
                        Style::default().fg(colors.grey()),
                    )));
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Add submodules with 'git submodule add <url> <path>'",
                        Style::default().fg(colors.green()),
                    )));
                } else {
                    // List submodules
                    for (i, submodule) in state.submodules.iter().enumerate() {
                        if i >= visible_height {
                            break;
                        }

                        let is_selected = i == state.selected_submodule;
                        let style = if is_selected {
                            Style::default().fg(colors.yellow()).bg(colors.red())
                        } else {
                            Style::default().fg(colors.fg())
                        };

                        // Status indicator
                        let status_indicator = match submodule.status {
                            crate::app::SubmoduleStatus::Initialized => "+",
                            crate::app::SubmoduleStatus::Uninitialized => "-",
                            crate::app::SubmoduleStatus::Modified => "*",
                            crate::app::SubmoduleStatus::Conflict => "!",
                            crate::app::SubmoduleStatus::OutOfDate => "^",
                        };

                        let status_color = match submodule.status {
                            crate::app::SubmoduleStatus::Initialized => colors.green(),
                            crate::app::SubmoduleStatus::Uninitialized => colors.grey(),
                            crate::app::SubmoduleStatus::Modified => colors.yellow(),
                            crate::app::SubmoduleStatus::Conflict => colors.red(),
                            crate::app::SubmoduleStatus::OutOfDate => colors.blue(),
                        };

                        // Truncate path if needed
                        let path_display = if submodule.path.len() > 40 {
                            format!("{}...", &submodule.path[..37])
                        } else {
                            submodule.path.clone()
                        };

                        lines.push(Line::from(vec![
                            Span::styled(
                                format!(" {} ", status_indicator),
                                Style::default().fg(status_color),
                            ),
                            Span::styled(format!("{:<42}", path_display), style),
                            Span::styled(
                                format!(" {}", &submodule.commit[..7.min(submodule.commit.len())]),
                                if is_selected {
                                    style
                                } else {
                                    Style::default().fg(colors.grey())
                                },
                            ),
                        ]));
                    }
                }

                if let Some(ref err) = state.error {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("Error: {}", err),
                        Style::default().fg(colors.red()),
                    )));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
        }
    }

    // Bottom separator
    frame.render_widget(
        Paragraph::new(Span::styled(sep, Style::default().fg(colors.fg()))),
        chunks[3],
    );

    // Help line based on view
    let help_text = if !state.is_repo {
        "Press any key to close"
    } else {
        match state.view {
            GitView::Menu => "↑↓ select  Enter open  S/L/D/C/B/H/T/G quick select  ESC close",
            GitView::Status => "↑↓ navigate  Enter view diff  A stage/unstage  R refresh  ESC back",
            GitView::Log => "↑↓ select  Enter view diff  PgUp/PgDn fast scroll  ESC back",
            GitView::Diff => "↑↓ scroll  PgUp/PgDn fast scroll  ESC back",
            GitView::Commit => "Type message  Shift+Enter newline  Enter commit  ESC cancel",
            GitView::Branch => if state.branch_input_mode {
                "Type branch name  Enter create  ESC cancel"
            } else {
                "↑↓ select  Enter switch  N new  D delete  R refresh  ESC back"
            },
            GitView::Stash => if state.stash_input_mode {
                "Type stash message  Enter create  ESC cancel"
            } else {
                "↑↓ select  S stash  P pop  A apply  D drop  R refresh  ESC back"
            },
            GitView::Tag => if state.tag_input_mode {
                "Type tag name  Enter create  ESC cancel"
            } else {
                "↑↓ select  N new  D delete  P push tags  R refresh  ESC back"
            },
            GitView::Remote => "↑↓ select  Enter execute  ESC back",
            GitView::Config => "↑↓ scroll  PgUp/PgDn fast scroll  R refresh  ESC back",
            GitView::Conflicts => "←→ files  ↑↓ sections  O ours  T theirs  B both  M mark resolved  A abort  ESC back",
            GitView::Submodules => "↑↓ select  I init  U update  S sync  R refresh  ESC back",
        }
    };
    frame.render_widget(
        Paragraph::new(Span::styled(help_text, Style::default().fg(colors.green()))),
        chunks[4],
    );
}

/// Draw Beads modal
fn draw_beads_modal(frame: &mut Frame, area: Rect, state: &BeadsState, app: &App) {
    let colors = app.colors();

    // Clear the entire area
    frame.render_widget(Clear, area);

    // Layout: title, separator, content, separator, help
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title
            Constraint::Length(1), // Separator
            Constraint::Min(10),   // Content
            Constraint::Length(1), // Separator
            Constraint::Length(1), // Help line
        ])
        .split(area);

    // Title based on current view
    let title = match state.view {
        BeadsView::Menu => " BEADS ISSUE TRACKER ",
        BeadsView::List => " BEADS - ALL ISSUES ",
        BeadsView::Ready => " BEADS - READY TO WORK ",
        BeadsView::Blocked => " BEADS - BLOCKED ISSUES ",
        BeadsView::Stats => " BEADS - STATISTICS ",
        BeadsView::Create => " BEADS - CREATE ISSUE ",
        BeadsView::Detail => " BEADS - ISSUE DETAIL ",
        BeadsView::Edit => " BEADS - EDIT ISSUE ",
        BeadsView::Comments => " BEADS - COMMENTS ",
        BeadsView::History => " BEADS - ISSUE HISTORY ",
        BeadsView::FileIssues => " BEADS - FILE ISSUES ",
        BeadsView::Dependencies => " BEADS - DEPENDENCY GRAPH ",
        BeadsView::Kanban => " BEADS - KANBAN BOARD ",
        BeadsView::Human => " BEADS - COMMAND HELP ",
        BeadsView::Doctor => " BEADS - HEALTH CHECK ",
    };
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

    if !state.is_beads_project {
        // Not a beads project
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "Not a Beads project",
                Style::default().fg(colors.yellow()),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Initialize beads with 'bd init'",
                Style::default().fg(colors.grey()),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press any key to close",
                Style::default().fg(colors.green()),
            )),
        ];
        frame.render_widget(Paragraph::new(lines), content_area);
    } else {
        match state.view {
            BeadsView::Menu => {
                let mut lines = vec![Line::from("")];

                let items = BeadsMenuItem::items(state.is_beads_project);
                for (i, item) in items.iter().enumerate() {
                    let is_selected = i == state.menu_selected;
                    let style = if is_selected {
                        Style::default().fg(colors.yellow()).bg(colors.red())
                    } else {
                        Style::default().fg(colors.fg())
                    };

                    let number = format!("{}. ", i + 1);

                    lines.push(Line::from(vec![
                        Span::styled("  ", style),
                        Span::styled(
                            number,
                            if is_selected {
                                style
                            } else {
                                Style::default().fg(colors.blue())
                            },
                        ),
                        Span::styled(format!("{:<12}", item.as_str()), style),
                        Span::styled(
                            item.description(),
                            if is_selected {
                                style
                            } else {
                                Style::default().fg(colors.grey())
                            },
                        ),
                    ]));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            BeadsView::List | BeadsView::Ready | BeadsView::Blocked => {
                // Account for search bar and header
                let search_height = if state.search_active || !state.search_query.is_empty() {
                    1
                } else {
                    0
                };
                let visible_height =
                    content_area.height.saturating_sub(1 + search_height as u16) as usize;
                let mut lines: Vec<Line> = vec![];

                // Show search bar if active or has query
                if state.search_active || !state.search_query.is_empty() {
                    let search_style = if state.search_active {
                        Style::default().fg(colors.yellow())
                    } else {
                        Style::default().fg(colors.blue())
                    };
                    let prompt = if state.search_active { "/" } else { "🔍" };
                    lines.push(Line::from(vec![
                        Span::styled(format!("{} ", prompt), search_style),
                        Span::styled(&state.search_query, search_style),
                        if state.search_active {
                            Span::styled("█", search_style)
                        } else {
                            Span::raw("")
                        },
                    ]));
                }

                // Filter issues based on search query
                let query_lower = state.search_query.to_lowercase();
                let filtered_issues: Vec<_> = if state.search_query.is_empty() {
                    state.issues.iter().collect()
                } else {
                    state
                        .issues
                        .iter()
                        .filter(|i| {
                            i.id.to_lowercase().contains(&query_lower)
                                || i.title.to_lowercase().contains(&query_lower)
                                || i.issue_type.to_lowercase().contains(&query_lower)
                                || i.status.to_lowercase().contains(&query_lower)
                        })
                        .collect()
                };

                if filtered_issues.is_empty() {
                    lines.push(Line::from(""));
                    let msg = if !state.search_query.is_empty() {
                        "No matching issues"
                    } else {
                        "No issues found"
                    };
                    lines.push(Line::from(Span::styled(
                        msg,
                        Style::default().fg(colors.grey()),
                    )));
                } else {
                    // Table header
                    let header_style = Style::default()
                        .fg(colors.blue())
                        .add_modifier(Modifier::BOLD);
                    // Column widths: ID=14, Type=8, Status=12, Pri=3, Title=rest
                    let id_w = 14;
                    let type_w = 8;
                    let status_w = 12;
                    let pri_w = 3;
                    let fixed_width = id_w + type_w + status_w + pri_w + 5; // +5 for spacing
                    let title_w = content_area
                        .width
                        .saturating_sub(fixed_width as u16)
                        .max(10) as usize;

                    lines.push(Line::from(vec![
                        Span::styled(format!(" {:<id_w$}", "ID"), header_style),
                        Span::styled(format!("{:<type_w$}", "TYPE"), header_style),
                        Span::styled(format!("{:<status_w$}", "STATUS"), header_style),
                        Span::styled(format!("{:<pri_w$}", "P"), header_style),
                        Span::styled(format!("{:<title_w$}", "TITLE"), header_style),
                    ]));

                    for (i, issue) in filtered_issues
                        .iter()
                        .skip(state.scroll_offset)
                        .enumerate()
                        .take(visible_height)
                    {
                        let is_selected = i == state.selected_issue;
                        let style = if is_selected {
                            Style::default().fg(colors.yellow()).bg(colors.red())
                        } else {
                            Style::default().fg(colors.fg())
                        };

                        let priority_style = if is_selected {
                            style
                        } else {
                            match issue.priority.as_str() {
                                "0" | "P0" => Style::default().fg(colors.red()),
                                "1" | "P1" => Style::default().fg(colors.yellow()),
                                _ => Style::default().fg(colors.grey()),
                            }
                        };

                        let type_style = if is_selected {
                            style
                        } else {
                            match issue.issue_type.as_str() {
                                "bug" => Style::default().fg(colors.red()),
                                "feature" => Style::default().fg(colors.green()),
                                _ => Style::default().fg(colors.grey()),
                            }
                        };

                        let status_style = if is_selected {
                            style
                        } else {
                            match issue.status.as_str() {
                                "open" => Style::default().fg(colors.green()),
                                "in_progress" => Style::default().fg(colors.yellow()),
                                "closed" => Style::default().fg(colors.grey()),
                                _ => Style::default().fg(colors.fg()),
                            }
                        };

                        let id_short = if issue.id.len() > id_w {
                            format!("{}…", &issue.id[..id_w - 1])
                        } else {
                            issue.id.clone()
                        };

                        let type_short = if issue.issue_type.len() > type_w {
                            format!("{}…", &issue.issue_type[..type_w - 1])
                        } else {
                            issue.issue_type.clone()
                        };

                        let status_short = if issue.status.len() > status_w {
                            format!("{}…", &issue.status[..status_w - 1])
                        } else {
                            issue.status.clone()
                        };

                        let pri = issue.priority.chars().last().unwrap_or('2');

                        // For epics, show progress bar
                        let is_epic = issue.issue_type == "epic";
                        let progress_str = if is_epic && !issue.dependents.is_empty() {
                            let total = issue.dependents.len();
                            let closed = issue
                                .dependents
                                .iter()
                                .filter(|d| d.status == "closed")
                                .count();
                            let pct = if total > 0 { (closed * 100) / total } else { 0 };
                            // Create progress bar: [████░░░░] 4/6
                            let bar_width = 8;
                            let filled = (pct * bar_width) / 100;
                            let empty = bar_width - filled;
                            format!(
                                " [{}{}] {}/{}",
                                "█".repeat(filled),
                                "░".repeat(empty),
                                closed,
                                total
                            )
                        } else {
                            String::new()
                        };

                        let progress_len = progress_str.len();
                        let available_title_w = title_w.saturating_sub(progress_len);
                        let title = if issue.title.len() > available_title_w {
                            format!("{}…", &issue.title[..available_title_w.saturating_sub(1)])
                        } else {
                            issue.title.clone()
                        };

                        if is_epic && !issue.dependents.is_empty() {
                            let closed_count = issue
                                .dependents
                                .iter()
                                .filter(|d| d.status == "closed")
                                .count();
                            let total = issue.dependents.len();
                            let pct = (closed_count * 100) / total.max(1);
                            let progress_color = if is_selected {
                                style
                            } else if pct == 100 {
                                Style::default().fg(colors.green())
                            } else if pct >= 50 {
                                Style::default().fg(colors.yellow())
                            } else {
                                Style::default().fg(colors.grey())
                            };

                            lines.push(Line::from(vec![
                                Span::styled(format!(" {:<id_w$}", id_short), style),
                                Span::styled(format!("{:<type_w$}", type_short), type_style),
                                Span::styled(format!("{:<status_w$}", status_short), status_style),
                                Span::styled(format!("{:<pri_w$}", pri), priority_style),
                                Span::styled(title, style),
                                Span::styled(progress_str, progress_color),
                            ]));
                        } else {
                            let title = if issue.title.len() > title_w {
                                format!("{}…", &issue.title[..title_w.saturating_sub(1)])
                            } else {
                                issue.title.clone()
                            };
                            lines.push(Line::from(vec![
                                Span::styled(format!(" {:<id_w$}", id_short), style),
                                Span::styled(format!("{:<type_w$}", type_short), type_style),
                                Span::styled(format!("{:<status_w$}", status_short), status_style),
                                Span::styled(format!("{:<pri_w$}", pri), priority_style),
                                Span::styled(format!("{:<title_w$}", title), style),
                            ]));
                        }
                    }
                }

                if let Some(ref err) = state.error {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("Error: {}", err),
                        Style::default().fg(colors.red()),
                    )));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            BeadsView::Stats => {
                let lines = vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("  Total Issues:   ", Style::default().fg(colors.green())),
                        Span::styled(
                            state.stats.total.to_string(),
                            Style::default().fg(colors.fg()),
                        ),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("  Open:           ", Style::default().fg(colors.green())),
                        Span::styled(
                            state.stats.open.to_string(),
                            Style::default().fg(colors.fg()),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("  In Progress:    ", Style::default().fg(colors.yellow())),
                        Span::styled(
                            state.stats.in_progress.to_string(),
                            Style::default().fg(colors.fg()),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("  Blocked:        ", Style::default().fg(colors.red())),
                        Span::styled(
                            state.stats.blocked.to_string(),
                            Style::default().fg(colors.fg()),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("  Closed:         ", Style::default().fg(colors.grey())),
                        Span::styled(
                            state.stats.closed.to_string(),
                            Style::default().fg(colors.fg()),
                        ),
                    ]),
                ];

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            BeadsView::Create => {
                let issue_types = ["task", "bug", "feature"];
                let priorities = ["P0", "P1", "P2", "P3", "P4"];

                let mut lines = vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "Create New Issue",
                        Style::default()
                            .fg(colors.fg())
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                ];

                // Title field
                let title_style = if state.create_field == 0 {
                    Style::default().fg(colors.yellow()).bg(colors.red())
                } else {
                    Style::default().fg(colors.fg())
                };
                lines.push(Line::from(vec![
                    Span::styled("  Title:    ", Style::default().fg(colors.green())),
                    Span::styled(&state.create_title, title_style),
                    if state.create_field == 0 {
                        Span::styled("█", title_style)
                    } else {
                        Span::styled("", Style::default())
                    },
                ]));

                lines.push(Line::from(""));

                // Type field
                let type_style = if state.create_field == 1 {
                    Style::default().fg(colors.yellow()).bg(colors.red())
                } else {
                    Style::default().fg(colors.fg())
                };
                let type_value = format!("< {} >", issue_types[state.create_type]);
                lines.push(Line::from(vec![
                    Span::styled("  Type:     ", Style::default().fg(colors.green())),
                    Span::styled(type_value, type_style),
                ]));

                lines.push(Line::from(""));

                // Priority field
                let priority_style = if state.create_field == 2 {
                    Style::default().fg(colors.yellow()).bg(colors.red())
                } else {
                    Style::default().fg(colors.fg())
                };
                let priority_value = format!("< {} >", priorities[state.create_priority]);
                lines.push(Line::from(vec![
                    Span::styled("  Priority: ", Style::default().fg(colors.green())),
                    Span::styled(priority_value, priority_style),
                ]));

                if let Some(ref err) = state.error {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("Error: {}", err),
                        Style::default().fg(colors.red()),
                    )));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            BeadsView::Detail => {
                let mut lines = vec![];

                // Use detail_issue if available, otherwise fall back to issues list
                let issue = state
                    .detail_issue
                    .as_ref()
                    .or_else(|| state.issues.get(state.selected_issue));

                if let Some(issue) = issue {
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled("  ID:       ", Style::default().fg(colors.green())),
                        Span::styled(&issue.id, Style::default().fg(colors.fg())),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("  Title:    ", Style::default().fg(colors.green())),
                        Span::styled(&issue.title, Style::default().fg(colors.fg())),
                    ]));

                    // Description if available (with text wrapping)
                    if let Some(ref desc) = issue.description {
                        if !desc.is_empty() {
                            lines.push(Line::from(""));
                            lines.push(Line::from(Span::styled(
                                "  Description:",
                                Style::default().fg(colors.green()),
                            )));
                            // Wrap text to fit content area (subtract 4 for indent)
                            let max_width = (content_area.width as usize).saturating_sub(6);
                            let mut line_count = 0;
                            for paragraph in desc.lines() {
                                if line_count >= 8 {
                                    lines.push(Line::from(vec![
                                        Span::styled("    ", Style::default()),
                                        Span::styled("...", Style::default().fg(colors.grey())),
                                    ]));
                                    break;
                                }
                                // Simple word wrap
                                let words: Vec<&str> = paragraph.split_whitespace().collect();
                                if words.is_empty() {
                                    lines.push(Line::from(""));
                                    line_count += 1;
                                    continue;
                                }
                                let mut current_line = String::new();
                                for word in words {
                                    if current_line.is_empty() {
                                        current_line = word.to_string();
                                    } else if current_line.len() + 1 + word.len() <= max_width {
                                        current_line.push(' ');
                                        current_line.push_str(word);
                                    } else {
                                        // Emit current line and start new one
                                        lines.push(Line::from(vec![
                                            Span::styled("    ", Style::default()),
                                            Span::styled(
                                                current_line.clone(),
                                                Style::default().fg(colors.grey()),
                                            ),
                                        ]));
                                        line_count += 1;
                                        if line_count >= 8 {
                                            break;
                                        }
                                        current_line = word.to_string();
                                    }
                                }
                                if !current_line.is_empty() && line_count < 8 {
                                    lines.push(Line::from(vec![
                                        Span::styled("    ", Style::default()),
                                        Span::styled(
                                            current_line.clone(),
                                            Style::default().fg(colors.grey()),
                                        ),
                                    ]));
                                    line_count += 1;
                                }
                            }
                        }
                    }

                    lines.push(Line::from(""));

                    // Status with color
                    let status_color = match issue.status.as_str() {
                        "open" => colors.green(),
                        "in_progress" => colors.yellow(),
                        "closed" => colors.grey(),
                        _ => colors.fg(),
                    };
                    lines.push(Line::from(vec![
                        Span::styled("  Status:   ", Style::default().fg(colors.green())),
                        Span::styled(&issue.status, Style::default().fg(status_color)),
                    ]));

                    // Type with color
                    let type_color = match issue.issue_type.as_str() {
                        "bug" => colors.red(),
                        "feature" => colors.green(),
                        "epic" => colors.cyan(),
                        _ => colors.fg(),
                    };
                    lines.push(Line::from(vec![
                        Span::styled("  Type:     ", Style::default().fg(colors.green())),
                        Span::styled(&issue.issue_type, Style::default().fg(type_color)),
                    ]));

                    // Priority with color
                    let priority_color = match issue.priority.as_str() {
                        "0" | "P0" => colors.red(),
                        "1" | "P1" => colors.yellow(),
                        _ => colors.fg(),
                    };
                    lines.push(Line::from(vec![
                        Span::styled("  Priority: ", Style::default().fg(colors.green())),
                        Span::styled(&issue.priority, Style::default().fg(priority_color)),
                    ]));

                    // Show blocked by if any
                    if !issue.blocked_by.is_empty() {
                        lines.push(Line::from(""));
                        lines.push(Line::from(vec![
                            Span::styled(
                                "  Blocked by: ",
                                Style::default()
                                    .fg(colors.red())
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                issue.blocked_by.join(", "),
                                Style::default().fg(colors.red()),
                            ),
                        ]));
                    }

                    // Show subtasks for epics
                    if !issue.dependents.is_empty() {
                        lines.push(Line::from(""));
                        let closed_count = issue
                            .dependents
                            .iter()
                            .filter(|d| d.status == "closed")
                            .count();
                        let total_count = issue.dependents.len();
                        lines.push(Line::from(Span::styled(
                            format!("  ─── Subtasks ({}/{}) ───", closed_count, total_count),
                            Style::default()
                                .fg(colors.cyan())
                                .add_modifier(Modifier::BOLD),
                        )));

                        for (i, subtask) in issue.dependents.iter().enumerate() {
                            let is_selected = i == state.selected_subtask;
                            let prefix = if is_selected { "▶ " } else { "  " };

                            let status_char = match subtask.status.as_str() {
                                "closed" => "✓",
                                "in_progress" => "◆",
                                _ => "○",
                            };
                            let status_color = match subtask.status.as_str() {
                                "closed" => colors.grey(),
                                "in_progress" => colors.yellow(),
                                _ => colors.green(),
                            };

                            let bg = if is_selected {
                                colors.red()
                            } else {
                                Color::Reset
                            };

                            lines.push(Line::from(vec![
                                Span::styled(
                                    format!("  {}", prefix),
                                    Style::default().fg(colors.yellow()).bg(bg),
                                ),
                                Span::styled(
                                    format!("{} ", status_char),
                                    Style::default().fg(status_color).bg(bg),
                                ),
                                Span::styled(
                                    format!("{} ", subtask.id),
                                    Style::default().fg(colors.blue()).bg(bg),
                                ),
                                Span::styled(
                                    &subtask.title,
                                    Style::default().fg(colors.fg()).bg(bg),
                                ),
                            ]));
                        }
                    }

                    // Show comments if any
                    if !issue.comments.is_empty() {
                        lines.push(Line::from(""));
                        lines.push(Line::from(Span::styled(
                            format!("  ─── Comments ({}) ───", issue.comments.len()),
                            Style::default()
                                .fg(colors.magenta())
                                .add_modifier(Modifier::BOLD),
                        )));

                        for comment in issue.comments.iter().take(3) {
                            lines.push(Line::from(vec![
                                Span::styled(
                                    format!("  {} ", comment.author),
                                    Style::default().fg(colors.blue()),
                                ),
                                Span::styled(
                                    &comment.created_at[..10], // Just date
                                    Style::default().fg(colors.grey()),
                                ),
                            ]));
                            // Truncate comment text if too long
                            let text = if comment.text.len() > 60 {
                                format!("{}...", &comment.text[..57])
                            } else {
                                comment.text.clone()
                            };
                            lines.push(Line::from(vec![
                                Span::styled("    ", Style::default()),
                                Span::styled(text, Style::default().fg(colors.fg())),
                            ]));
                        }
                        if issue.comments.len() > 3 {
                            lines.push(Line::from(Span::styled(
                                format!("    ... and {} more", issue.comments.len() - 3),
                                Style::default().fg(colors.grey()),
                            )));
                        }
                    }

                    // Actions section
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  ─── Actions ───",
                        Style::default()
                            .fg(colors.blue())
                            .add_modifier(Modifier::BOLD),
                    )));
                    lines.push(Line::from(""));

                    // Show available actions based on status
                    if issue.status == "open" {
                        lines.push(Line::from(vec![
                            Span::styled("  [S] ", Style::default().fg(colors.yellow())),
                            Span::styled("Start working", Style::default().fg(colors.fg())),
                        ]));
                    }
                    if issue.status == "in_progress" || issue.status == "open" {
                        lines.push(Line::from(vec![
                            Span::styled("  [C] ", Style::default().fg(colors.yellow())),
                            Span::styled("Close issue", Style::default().fg(colors.fg())),
                        ]));
                    }
                    if issue.status == "closed" {
                        lines.push(Line::from(vec![
                            Span::styled("  [O] ", Style::default().fg(colors.yellow())),
                            Span::styled("Reopen issue", Style::default().fg(colors.fg())),
                        ]));
                    }
                    // Edit action always available
                    lines.push(Line::from(vec![
                        Span::styled("  [E] ", Style::default().fg(colors.yellow())),
                        Span::styled("Edit issue", Style::default().fg(colors.fg())),
                    ]));
                    // Subtask creation for epics
                    if issue.issue_type == "epic" {
                        lines.push(Line::from(vec![
                            Span::styled("  [N] ", Style::default().fg(colors.yellow())),
                            Span::styled("New subtask", Style::default().fg(colors.fg())),
                        ]));
                    }
                } else {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Issue not found",
                        Style::default().fg(colors.red()),
                    )));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            BeadsView::Edit => {
                let statuses = ["open", "in_progress", "closed"];
                let priorities = ["P0", "P1", "P2", "P3", "P4"];

                let mut lines = vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("Edit Issue: {}", state.edit_issue_id),
                        Style::default()
                            .fg(colors.fg())
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                ];

                // Title field
                let title_style = if state.edit_field == 0 {
                    Style::default().fg(colors.yellow()).bg(colors.red())
                } else {
                    Style::default().fg(colors.fg())
                };
                lines.push(Line::from(vec![
                    Span::styled("  Title:       ", Style::default().fg(colors.green())),
                    Span::styled(&state.edit_title, title_style),
                    if state.edit_field == 0 {
                        Span::styled("█", title_style)
                    } else {
                        Span::styled("", Style::default())
                    },
                ]));

                lines.push(Line::from(""));

                // Description field
                let desc_style = if state.edit_field == 1 {
                    Style::default().fg(colors.yellow()).bg(colors.red())
                } else {
                    Style::default().fg(colors.fg())
                };
                let desc_display = if state.edit_description.len() > 40 {
                    format!("{}...", &state.edit_description[..37])
                } else {
                    state.edit_description.clone()
                };
                lines.push(Line::from(vec![
                    Span::styled("  Description: ", Style::default().fg(colors.green())),
                    Span::styled(&desc_display, desc_style),
                    if state.edit_field == 1 {
                        Span::styled("█", desc_style)
                    } else {
                        Span::styled("", Style::default())
                    },
                ]));

                lines.push(Line::from(""));

                // Status field
                let status_style = if state.edit_field == 2 {
                    Style::default().fg(colors.yellow()).bg(colors.red())
                } else {
                    Style::default().fg(colors.fg())
                };
                let status_value = format!("< {} >", statuses[state.edit_status]);
                lines.push(Line::from(vec![
                    Span::styled("  Status:      ", Style::default().fg(colors.green())),
                    Span::styled(status_value, status_style),
                ]));

                lines.push(Line::from(""));

                // Priority field
                let priority_style = if state.edit_field == 3 {
                    Style::default().fg(colors.yellow()).bg(colors.red())
                } else {
                    Style::default().fg(colors.fg())
                };
                let priority_value = format!("< {} >", priorities[state.edit_priority]);
                lines.push(Line::from(vec![
                    Span::styled("  Priority:    ", Style::default().fg(colors.green())),
                    Span::styled(priority_value, priority_style),
                ]));

                if let Some(ref err) = state.error {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("Error: {}", err),
                        Style::default().fg(colors.red()),
                    )));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            BeadsView::Comments => {
                let mut lines = vec![];

                if let Some(ref issue) = state.detail_issue {
                    // Issue title at top
                    lines.push(Line::from(vec![
                        Span::styled("  Issue: ", Style::default().fg(colors.green())),
                        Span::styled(&issue.id, Style::default().fg(colors.blue())),
                        Span::styled(" - ", Style::default().fg(colors.grey())),
                        Span::styled(&issue.title, Style::default().fg(colors.fg())),
                    ]));
                    lines.push(Line::from(""));

                    // Comment input area at top if active
                    if state.comment_input_active {
                        lines.push(Line::from(Span::styled(
                            "  Add comment:",
                            Style::default()
                                .fg(colors.yellow())
                                .add_modifier(Modifier::BOLD),
                        )));
                        lines.push(Line::from(vec![
                            Span::styled("  > ", Style::default().fg(colors.green())),
                            Span::styled(&state.comment_input, Style::default().fg(colors.fg())),
                            Span::styled("█", Style::default().fg(colors.yellow())),
                        ]));
                        lines.push(Line::from(""));
                    }

                    if issue.comments.is_empty() {
                        lines.push(Line::from(""));
                        lines.push(Line::from(Span::styled(
                            "  No comments yet. Press 'A' to add one.",
                            Style::default().fg(colors.grey()),
                        )));
                    } else {
                        // Header
                        lines.push(Line::from(Span::styled(
                            format!("  ─── Comments ({}) ───", issue.comments.len()),
                            Style::default()
                                .fg(colors.magenta())
                                .add_modifier(Modifier::BOLD),
                        )));
                        lines.push(Line::from(""));

                        // List all comments with scrolling
                        for (i, comment) in issue.comments.iter().enumerate() {
                            let is_selected = i == state.selected_comment;
                            let bg = if is_selected {
                                colors.red()
                            } else {
                                Color::Reset
                            };
                            let prefix = if is_selected { "▶ " } else { "  " };

                            // Author and date line
                            lines.push(Line::from(vec![
                                Span::styled(prefix, Style::default().fg(colors.yellow()).bg(bg)),
                                Span::styled(
                                    format!("{} ", comment.author),
                                    Style::default().fg(colors.blue()).bg(bg),
                                ),
                                Span::styled(
                                    &comment.created_at,
                                    Style::default().fg(colors.grey()).bg(bg),
                                ),
                            ]));

                            // Comment text - wrap if too long
                            let text_style = Style::default().fg(colors.fg()).bg(bg);
                            let max_width = content_area.width.saturating_sub(6) as usize;
                            let text = &comment.text;
                            if text.len() <= max_width {
                                lines.push(Line::from(vec![
                                    Span::styled("    ", Style::default().bg(bg)),
                                    Span::styled(text.clone(), text_style),
                                ]));
                            } else {
                                // Wrap text
                                for chunk in text.as_bytes().chunks(max_width) {
                                    let line_text = String::from_utf8_lossy(chunk).to_string();
                                    lines.push(Line::from(vec![
                                        Span::styled("    ", Style::default().bg(bg)),
                                        Span::styled(line_text, text_style),
                                    ]));
                                }
                            }
                            lines.push(Line::from(""));
                        }
                    }
                } else {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "No issue selected",
                        Style::default().fg(colors.red()),
                    )));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            BeadsView::History => {
                let visible_height = content_area.height.saturating_sub(4) as usize;
                let mut lines = vec![];

                if let Some(ref issue) = state.detail_issue {
                    // Issue title at top
                    lines.push(Line::from(vec![
                        Span::styled("  Issue: ", Style::default().fg(colors.green())),
                        Span::styled(&issue.id, Style::default().fg(colors.blue())),
                        Span::styled(" - ", Style::default().fg(colors.grey())),
                        Span::styled(&issue.title, Style::default().fg(colors.fg())),
                    ]));
                    lines.push(Line::from(""));
                }

                if state.activity_entries.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  No activity history available.",
                        Style::default().fg(colors.grey()),
                    )));
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  Activity is tracked when issues are modified.",
                        Style::default().fg(colors.grey()),
                    )));
                } else {
                    // Timeline header
                    lines.push(Line::from(Span::styled(
                        format!(
                            "  ─── Timeline ({} events) ───",
                            state.activity_entries.len()
                        ),
                        Style::default()
                            .fg(colors.magenta())
                            .add_modifier(Modifier::BOLD),
                    )));
                    lines.push(Line::from(""));

                    // Calculate scroll offset for selected item visibility
                    let start = if state.selected_activity >= visible_height {
                        state.selected_activity - visible_height + 1
                    } else {
                        0
                    };

                    for (i, entry) in state
                        .activity_entries
                        .iter()
                        .enumerate()
                        .skip(start)
                        .take(visible_height)
                    {
                        let is_selected = i == state.selected_activity;
                        let bg = if is_selected {
                            colors.red()
                        } else {
                            Color::Reset
                        };
                        let prefix = if is_selected { "▶ " } else { "  " };

                        // Event type color
                        let event_color = match entry.event_type.as_str() {
                            "created" => colors.green(),
                            "status_change" => colors.yellow(),
                            "closed" => colors.grey(),
                            "reopened" => colors.cyan(),
                            "comment_added" => colors.blue(),
                            "priority_change" => colors.magenta(),
                            "assignment_change" => colors.cyan(),
                            _ => colors.fg(),
                        };

                        // Timeline connector
                        let connector = if i == 0 { "┌" } else { "├" };

                        // Main event line with timestamp
                        lines.push(Line::from(vec![
                            Span::styled(prefix, Style::default().fg(colors.yellow()).bg(bg)),
                            Span::styled(
                                format!("{} ", connector),
                                Style::default().fg(colors.grey()).bg(bg),
                            ),
                            Span::styled(
                                format!("{} ", entry.symbol),
                                Style::default().fg(event_color).bg(bg),
                            ),
                            Span::styled(
                                &entry.timestamp,
                                Style::default().fg(colors.grey()).bg(bg),
                            ),
                        ]));

                        // Event detail line
                        let detail_prefix = if i == state.activity_entries.len() - 1 {
                            "└──"
                        } else {
                            "│  "
                        };
                        lines.push(Line::from(vec![
                            Span::styled("  ", Style::default().bg(bg)),
                            Span::styled(
                                format!("{} ", detail_prefix),
                                Style::default().fg(colors.grey()).bg(bg),
                            ),
                            Span::styled(&entry.message, Style::default().fg(colors.fg()).bg(bg)),
                        ]));

                        // Status transition if present
                        if let (Some(old), Some(new)) = (&entry.old_status, &entry.new_status) {
                            lines.push(Line::from(vec![
                                Span::styled("  ", Style::default().bg(bg)),
                                Span::styled("│     ", Style::default().fg(colors.grey()).bg(bg)),
                                Span::styled(old, Style::default().fg(colors.grey()).bg(bg)),
                                Span::styled(" → ", Style::default().fg(colors.yellow()).bg(bg)),
                                Span::styled(new, Style::default().fg(colors.green()).bg(bg)),
                            ]));
                        }

                        // Actor if present
                        if let Some(ref actor) = entry.actor {
                            lines.push(Line::from(vec![
                                Span::styled("  ", Style::default().bg(bg)),
                                Span::styled("│     ", Style::default().fg(colors.grey()).bg(bg)),
                                Span::styled("by ", Style::default().fg(colors.grey()).bg(bg)),
                                Span::styled(actor, Style::default().fg(colors.blue()).bg(bg)),
                            ]));
                        }

                        lines.push(Line::from(""));
                    }
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            BeadsView::FileIssues => {
                let visible_height = content_area.height.saturating_sub(4) as usize;
                let mut lines = vec![];

                // Show the file being queried
                lines.push(Line::from(vec![
                    Span::styled("  File: ", Style::default().fg(colors.green())),
                    Span::styled(&state.file_query_path, Style::default().fg(colors.blue())),
                ]));
                lines.push(Line::from(""));

                if state.file_related_issues.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  No issues found mentioning this file.",
                        Style::default().fg(colors.grey()),
                    )));
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  Create an issue with the filename to link it.",
                        Style::default().fg(colors.grey()),
                    )));
                } else {
                    lines.push(Line::from(Span::styled(
                        format!(
                            "  ─── Related Issues ({}) ───",
                            state.file_related_issues.len()
                        ),
                        Style::default()
                            .fg(colors.magenta())
                            .add_modifier(Modifier::BOLD),
                    )));
                    lines.push(Line::from(""));

                    for (i, issue) in state
                        .file_related_issues
                        .iter()
                        .enumerate()
                        .take(visible_height)
                    {
                        let is_selected = i == state.file_issue_selected;
                        let bg = if is_selected {
                            colors.red()
                        } else {
                            Color::Reset
                        };
                        let prefix = if is_selected { "▶ " } else { "  " };

                        // Status indicator
                        let status_char = match issue.status.as_str() {
                            "closed" => "✓",
                            "in_progress" => "◆",
                            "open" => "○",
                            _ => "?",
                        };
                        let status_color = match issue.status.as_str() {
                            "closed" => colors.grey(),
                            "in_progress" => colors.yellow(),
                            "open" => colors.green(),
                            _ => colors.fg(),
                        };

                        // Issue line
                        lines.push(Line::from(vec![
                            Span::styled(prefix, Style::default().fg(colors.yellow()).bg(bg)),
                            Span::styled(status_char, Style::default().fg(status_color).bg(bg)),
                            Span::styled(" ", Style::default().bg(bg)),
                            Span::styled(&issue.id, Style::default().fg(colors.blue()).bg(bg)),
                            Span::styled(" ", Style::default().bg(bg)),
                            Span::styled(
                                format!("[{}]", issue.priority),
                                Style::default().fg(colors.cyan()).bg(bg),
                            ),
                            Span::styled(" ", Style::default().bg(bg)),
                            Span::styled(&issue.title, Style::default().fg(colors.fg()).bg(bg)),
                        ]));
                    }
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            BeadsView::Dependencies => {
                let visible_height = content_area.height.saturating_sub(2) as usize;
                let mut lines = vec![];

                if state.issues.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  No issues to display",
                        Style::default().fg(colors.grey()),
                    )));
                } else {
                    lines.push(Line::from(Span::styled(
                        "  Issue ID          Status        Blocked By → Dependents",
                        Style::default()
                            .fg(colors.blue())
                            .add_modifier(Modifier::BOLD),
                    )));
                    lines.push(Line::from(""));

                    for (i, issue) in state
                        .issues
                        .iter()
                        .skip(state.scroll_offset)
                        .enumerate()
                        .take(visible_height)
                    {
                        let is_selected = i == state.selected_issue;
                        let bg = if is_selected {
                            colors.red()
                        } else {
                            Color::Reset
                        };
                        let prefix = if is_selected { "▶ " } else { "  " };

                        // Status indicator
                        let status_char = match issue.status.as_str() {
                            "closed" => "✓",
                            "in_progress" => "◆",
                            "open" => "○",
                            _ => "?",
                        };
                        let status_color = match issue.status.as_str() {
                            "closed" => colors.grey(),
                            "in_progress" => colors.yellow(),
                            "open" => colors.green(),
                            _ => colors.fg(),
                        };

                        // Build dependency info
                        let blocked_by_str = if issue.blocked_by.is_empty() {
                            "none".to_string()
                        } else {
                            issue
                                .blocked_by
                                .iter()
                                .map(|b| {
                                    // Shorten the ID
                                    if b.len() > 8 {
                                        format!("{}…", &b[..7])
                                    } else {
                                        b.clone()
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join(", ")
                        };

                        let dependents_str = if issue.dependents.is_empty() {
                            "none".to_string()
                        } else {
                            format!("{} items", issue.dependents.len())
                        };

                        // ID shortened if needed
                        let id_short = if issue.id.len() > 12 {
                            format!("{}…", &issue.id[..11])
                        } else {
                            issue.id.clone()
                        };

                        // Type indicator
                        let type_symbol = match issue.issue_type.as_str() {
                            "epic" => "⊞",
                            "bug" => "●",
                            "feature" => "★",
                            _ => "□",
                        };
                        let type_color = match issue.issue_type.as_str() {
                            "epic" => colors.cyan(),
                            "bug" => colors.red(),
                            "feature" => colors.green(),
                            _ => colors.grey(),
                        };

                        // First line: ID and status
                        lines.push(Line::from(vec![
                            Span::styled(prefix, Style::default().fg(colors.yellow()).bg(bg)),
                            Span::styled(
                                format!("{} ", type_symbol),
                                Style::default().fg(type_color).bg(bg),
                            ),
                            Span::styled(
                                format!("{:<12} ", id_short),
                                Style::default().fg(colors.blue()).bg(bg),
                            ),
                            Span::styled(
                                format!("{} ", status_char),
                                Style::default().fg(status_color).bg(bg),
                            ),
                            Span::styled(
                                format!("{:<12}", issue.status),
                                Style::default().fg(status_color).bg(bg),
                            ),
                        ]));

                        // Second line: Dependencies
                        let block_color = if issue.blocked_by.is_empty() {
                            colors.grey()
                        } else {
                            colors.red()
                        };
                        let dep_color = if issue.dependents.is_empty() {
                            colors.grey()
                        } else {
                            colors.cyan()
                        };

                        lines.push(Line::from(vec![
                            Span::styled("      ← ", Style::default().fg(block_color).bg(bg)),
                            Span::styled(
                                format!("{:<20}", blocked_by_str),
                                Style::default().fg(block_color).bg(bg),
                            ),
                            Span::styled(" → ", Style::default().fg(dep_color).bg(bg)),
                            Span::styled(dependents_str, Style::default().fg(dep_color).bg(bg)),
                        ]));

                        // Third line: Title (truncated)
                        let max_title_w = content_area.width.saturating_sub(8) as usize;
                        let title = if issue.title.len() > max_title_w {
                            format!("{}…", &issue.title[..max_title_w.saturating_sub(1)])
                        } else {
                            issue.title.clone()
                        };
                        lines.push(Line::from(vec![
                            Span::styled("      ", Style::default().bg(bg)),
                            Span::styled(title, Style::default().fg(colors.fg()).bg(bg)),
                        ]));

                        // Separator
                        lines.push(Line::from(""));
                    }
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            BeadsView::Kanban => {
                // Kanban board with three columns: Open, In Progress, Closed
                let column_width = content_area.width / 3;
                let visible_height = content_area.height.saturating_sub(3) as usize;

                // Filter issues by status
                let open_issues: Vec<_> =
                    state.issues.iter().filter(|i| i.status == "open").collect();
                let in_progress_issues: Vec<_> = state
                    .issues
                    .iter()
                    .filter(|i| i.status == "in_progress")
                    .collect();
                let closed_issues: Vec<_> = state
                    .issues
                    .iter()
                    .filter(|i| i.status == "closed")
                    .collect();

                let columns = [
                    ("OPEN", &open_issues, colors.green()),
                    ("IN PROGRESS", &in_progress_issues, colors.yellow()),
                    ("CLOSED", &closed_issues, colors.grey()),
                ];

                let mut lines = vec![];

                // Header row
                let mut header_spans = vec![];
                for (i, (title, issues, color)) in columns.iter().enumerate() {
                    let selected = i == state.kanban_column;
                    let style = if selected {
                        Style::default()
                            .fg(*color)
                            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                    } else {
                        Style::default().fg(*color).add_modifier(Modifier::BOLD)
                    };
                    header_spans.push(Span::styled(
                        format!(
                            " {:^width$}",
                            format!("{} ({})", title, issues.len()),
                            width = column_width.saturating_sub(2) as usize
                        ),
                        style,
                    ));
                }
                lines.push(Line::from(header_spans));
                lines.push(Line::from(""));

                // Render rows
                let max_items = open_issues
                    .len()
                    .max(in_progress_issues.len())
                    .max(closed_issues.len());

                for row in 0..visible_height.min(max_items) {
                    let mut row_spans = vec![];

                    for (col_idx, (_, issues, color)) in columns.iter().enumerate() {
                        let is_selected_cell =
                            col_idx == state.kanban_column && row == state.kanban_row;
                        let bg = if is_selected_cell {
                            colors.red()
                        } else {
                            Color::Reset
                        };

                        if row < issues.len() {
                            let issue = &issues[row];
                            let type_symbol = match issue.issue_type.as_str() {
                                "epic" => "⊞",
                                "bug" => "●",
                                "feature" => "★",
                                _ => "□",
                            };

                            // Truncate title to fit column
                            let max_title = column_width.saturating_sub(6) as usize;
                            let title = if issue.title.len() > max_title {
                                format!("{}…", &issue.title[..max_title.saturating_sub(1)])
                            } else {
                                issue.title.clone()
                            };

                            row_spans.push(Span::styled(
                                format!(
                                    " {} {:width$}",
                                    type_symbol,
                                    title,
                                    width = column_width.saturating_sub(4) as usize
                                ),
                                Style::default().fg(*color).bg(bg),
                            ));
                        } else {
                            // Empty cell
                            row_spans.push(Span::styled(
                                format!("{:width$}", "", width = column_width as usize),
                                Style::default().bg(bg),
                            ));
                        }
                    }

                    lines.push(Line::from(row_spans));
                }

                // If no issues at all
                if state.issues.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  No issues to display",
                        Style::default().fg(colors.grey()),
                    )));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
            BeadsView::Human | BeadsView::Doctor => {
                let visible_height = content_area.height as usize;
                let mut lines = vec![];

                // Title line
                let title_text = if state.view == BeadsView::Human {
                    "Common beads commands for human users:"
                } else {
                    "Beads installation health check:"
                };
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    title_text,
                    Style::default().fg(colors.yellow()),
                )));
                lines.push(Line::from(""));

                // Show output lines with scrolling
                let total_lines = state.output_lines.len();
                let start = state.scroll_offset;
                let end = (start + visible_height.saturating_sub(4)).min(total_lines);

                for line in state.output_lines.iter().skip(start).take(end - start) {
                    // Color lines based on content
                    let style =
                        if line.contains("✓") || line.contains("OK") || line.contains("passed") {
                            Style::default().fg(colors.green())
                        } else if line.contains("✗")
                            || line.contains("ERROR")
                            || line.contains("failed")
                        {
                            Style::default().fg(colors.red())
                        } else if line.contains("WARNING") {
                            Style::default().fg(colors.yellow())
                        } else if line.starts_with("  ") {
                            Style::default().fg(colors.grey())
                        } else {
                            Style::default().fg(colors.fg())
                        };
                    lines.push(Line::from(Span::styled(line.clone(), style)));
                }

                // Scroll indicator
                if total_lines > visible_height.saturating_sub(4) {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("-- Line {}/{} --", start + 1, total_lines.max(1)),
                        Style::default().fg(colors.grey()),
                    )));
                }

                frame.render_widget(Paragraph::new(lines), content_area);
            }
        }
    }

    // Bottom separator
    frame.render_widget(
        Paragraph::new(Span::styled(sep, Style::default().fg(colors.fg()))),
        chunks[3],
    );

    // Help line based on view
    let help_text = if !state.is_beads_project {
        "Press any key to close"
    } else {
        match state.view {
            BeadsView::Menu => "↑↓ select  Enter open  ESC close",
            BeadsView::List | BeadsView::Ready | BeadsView::Blocked => {
                if state.search_active {
                    "Type to search  Enter finish  ESC cancel"
                } else if !state.search_query.is_empty() {
                    "↑↓ nav  / search  C close  S start  R refresh  ESC clear"
                } else {
                    "↑↓ nav  / search  C close  S start  R refresh  ESC back"
                }
            }
            BeadsView::Stats => "R refresh  ESC back",
            BeadsView::Create => "↑↓ field  ←→ value  Enter create  ESC cancel",
            BeadsView::Detail => {
                "↑↓ subtasks  E edit  N new subtask  M comments  H history  S start  C close"
            }
            BeadsView::Edit => "↑↓ field  ←→ value  Enter save  ESC cancel",
            BeadsView::Comments => {
                if state.comment_input_active {
                    "Type comment  Enter submit  ESC cancel"
                } else {
                    "↑↓ navigate  A add comment  ESC back"
                }
            }
            BeadsView::History => "↑↓ navigate  PgUp/PgDn page  Home/End  R refresh  ESC back",
            BeadsView::FileIssues => "↑↓ navigate  Enter view detail  R refresh  ESC back",
            BeadsView::Dependencies => "↑↓ navigate  Enter view detail  R refresh  ESC back",
            BeadsView::Kanban => "←→ columns  ↑↓ rows  Enter view detail  R refresh  ESC back",
            BeadsView::Human | BeadsView::Doctor => "↑↓ scroll  PgUp/PgDn page  ESC back",
        }
    };
    frame.render_widget(
        Paragraph::new(Span::styled(help_text, Style::default().fg(colors.green()))),
        chunks[4],
    );
}
