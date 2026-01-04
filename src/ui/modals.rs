//\! Modal UI components
//\!
//\! This module contains all modal drawing functions including:
//\! - Help modal, Status modal, Quit modal
//\! - File operation modals (Copy, Move, Erase, Rename)
//\! - Search and Find modals
//\! - Directory Map, Batch Rename, Attribute modals
//\! - Error, Success, Progress modals

use crate::app::{App, Modal, ProgressState};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

/// Create a centered rectangle with fixed width and height
/// All modals should use this instead of percentage-based sizing
pub(super) fn centered_fixed(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

/// Wrap text lines to fit within a maximum width
/// Simple word-wrapping that preserves span styles
fn wrap_lines(lines: Vec<Line>, max_width: usize) -> Vec<Line> {
    let mut result = Vec::new();

    for line in lines {
        let line_width = line.width();
        if line_width <= max_width {
            result.push(line);
        } else {
            // For simple text lines (single span or plain text), word wrap
            let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            let style = line.spans.first().map(|s| s.style).unwrap_or_default();

            let mut current_line = String::new();
            for word in text.split_whitespace() {
                if current_line.is_empty() {
                    current_line = word.to_string();
                } else if current_line.len() + 1 + word.len() <= max_width {
                    current_line.push(' ');
                    current_line.push_str(word);
                } else {
                    result.push(Line::from(Span::styled(current_line.clone(), style)));
                    current_line = word.to_string();
                }
            }
            if !current_line.is_empty() {
                result.push(Line::from(Span::styled(current_line, style)));
            }
        }
    }

    result
}

pub(super) fn draw_modal(frame: &mut Frame, app: &App, area: Rect) {
    // Minimum size check for modals - avoid crashes on very small terminals
    if area.width < 20 || area.height < 10 {
        return; // Too small to render any modal safely
    }

    // All modals handle their own area/clearing with fixed sizes
    // NO percentage-based sizing - all modals use centered_fixed for exact dimensions
    match &app.modal {
        // Full-screen modals (use entire area)
        Modal::Find(state) => {
            crate::plugins::find::modal::draw_find_modal(frame, area, state, app);
        }
        Modal::BatchRename(state) => {
            crate::plugins::fileops::modal::draw_batch_rename_modal(frame, area, state, app);
        }
        Modal::Git(state) => {
            crate::plugins::git::modal::draw_git_modal(frame, area, state, app);
        }
        Modal::Beads(state) => {
            crate::plugins::beads::modal::draw_beads_modal(frame, area, state, app);
        }
        Modal::Plugin(_) => app.plugin_manager.draw_modal(frame, area, &app.colors()),

        // Fixed-size modals with their own clearing
        Modal::Quit => draw_quit_modal(frame, area, app),
        Modal::Error(msg) => draw_error_modal(frame, area, msg, app),
        Modal::Success(msg) => draw_success_modal(frame, area, msg, app),
        Modal::Progress(state) => draw_progress_modal(frame, area, state, app),

        Modal::CopyTo(dest) => {
            let modal_area = centered_fixed(50, 12, area);
            crate::plugins::fileops::modal::draw_copy_modal(frame, modal_area, dest, app);
        }
        Modal::MoveTo(dest) => {
            let modal_area = centered_fixed(50, 12, area);
            crate::plugins::fileops::modal::draw_move_modal(frame, modal_area, dest, app);
        }
        Modal::EraseConfirm => {
            let modal_area = centered_fixed(50, 10, area);
            crate::plugins::fileops::modal::draw_erase_modal(frame, modal_area, app);
        }
        Modal::PathInput(path) => {
            let modal_area = centered_fixed(50, 10, area);
            draw_path_input_modal(frame, modal_area, path, app);
        }
        Modal::RenameInput(name) => {
            let modal_area = centered_fixed(50, 10, area);
            crate::plugins::fileops::modal::draw_rename_modal(frame, modal_area, name, app);
        }
        Modal::Attribute(state) => {
            let modal_area = centered_fixed(60, 15, area);
            crate::plugins::attribute::modal::draw_attribute_modal(frame, modal_area, state, app);
        }
        Modal::Clipboard(state) => {
            // Clipboard modal height depends on item count
            let height = (state.items.len() + 5).min(15) as u16;
            let modal_area = centered_fixed(50, height, area);
            crate::plugins::clipboard::modal::draw_clipboard_modal(frame, modal_area, state, app);
        }
        Modal::None => {}
    }
}

/// Draw quit confirmation modal (Q-DOS II style with double-line box)
pub(super) fn draw_quit_modal(frame: &mut Frame, area: Rect, app: &App) {
    use crate::ui::components::ModalFrame;

    let colors = app.colors();

    // Modal is 60 chars wide, 8 lines tall (matching spec/ui.md)
    let width: u16 = 60;
    let height: u16 = 8;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let quit_area = Rect::new(x, y, width.min(area.width), height.min(area.height));

    // Use ModalFrame for consistent styling
    let modal =
        ModalFrame::themed(quit_area, " F10 - Quit Q-DOS II ", &colors).no_footer_separator();
    modal.render_frame(frame);

    // Content rows (centered text)
    modal.render_row(frame, 0, vec![]); // Empty row
    modal.render_row(
        frame,
        1,
        vec![Span::styled(
            "Press F10 again to quit, or RETURN for options",
            Style::default().fg(colors.fg()).bg(colors.bg()),
        )],
    );
    modal.render_row(frame, 2, vec![]); // Empty row
    modal.render_row(
        frame,
        3,
        vec![Span::styled(
            "Press ESC to return to Q-DOS II",
            Style::default().fg(colors.green()).bg(colors.bg()),
        )],
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

    // Calculate dynamic modal width based on content
    let title_width = title.len() + 4; // title + some padding
    let max_content_width = content.iter().map(|line| line.width()).max().unwrap_or(0) + 4; // content + padding

    // Use the larger of title or content width, with min/max constraints
    let min_width: u16 = 30;
    let max_width: u16 = (area.width * 8 / 10).min(80); // 80% of area or 80 chars
    let calculated_width = title_width.max(max_content_width) as u16;
    let modal_width = calculated_width.clamp(min_width, max_width);

    // Wrap content that exceeds inner width (modal_width - 2 for borders)
    let inner_width = (modal_width as usize).saturating_sub(4);
    let content = wrap_lines(content, inner_width);

    let content_lines = content.len() as u16;
    let modal_height = (content_lines + 4).min(area.height - 2); // cap at screen height

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
pub(super) fn draw_path_input_modal(frame: &mut Frame, area: Rect, path: &str, app: &App) {
    let colors = app.colors();
    use crate::ui::components::ModalFrame;

    // Use ModalFrame for consistent double-line border styling
    let modal = ModalFrame::themed(area, " Change Directory ", &colors)
        .no_title_separator()
        .no_footer_separator();
    modal.render_frame(frame);

    // Content rows
    modal.render_row(frame, 0, vec![]); // Empty row
    modal.render_row(
        frame,
        1,
        vec![Span::styled(
            "Enter path (Tab to complete):",
            Style::default().fg(colors.green()).bg(colors.bg()),
        )],
    );
    modal.render_row(frame, 2, vec![]); // Empty row
    modal.render_row(
        frame,
        3,
        vec![Span::styled(
            format!("{}_", path),
            Style::default().fg(colors.fg()).bg(colors.bg()),
        )],
    );
    modal.render_row(frame, 4, vec![]); // Empty row

    // Help line
    modal.render_help(
        frame,
        vec![("Tab", "complete"), ("Enter", "confirm"), ("Esc", "cancel")],
    );
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
pub(super) fn draw_progress_modal(frame: &mut Frame, area: Rect, state: &ProgressState, app: &App) {
    let colors = app.colors();
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
            Style::default().fg(colors.fg()),
        )),
        Line::from(""),
        Line::from(Span::styled(
            progress_bar,
            Style::default().fg(colors.blue()),
        )),
        Line::from(Span::styled(
            format!("{}%", percentage),
            Style::default().fg(colors.green()),
        )),
        Line::from(""),
        Line::from(Span::styled(
            current_file,
            Style::default().fg(colors.yellow()),
        )),
    ];

    // Show error if any
    if let Some(ref err) = state.last_error {
        content.push(Line::from(""));
        content.push(Line::from(Span::styled(
            format!("Error: {}", err),
            Style::default().fg(colors.red()),
        )));
    }

    // Show stats
    content.push(Line::from(""));
    content.push(Line::from(Span::styled(
        format!("Completed: {}  Failed: {}", state.completed, state.failed),
        Style::default().fg(colors.green()),
    )));
    content.push(Line::from(""));
    content.push(Line::from(Span::styled(
        "Press ESC to cancel",
        Style::default().fg(colors.grey()),
    )));

    draw_qdos_modal_themed(frame, area, &title, content, colors.blue(), app);
}
