//\! Modal UI components
//\!
//\! This module contains all modal drawing functions including:
//\! - Help modal, Status modal, Quit modal
//\! - File operation modals (Copy, Move, Erase, Rename)
//\! - Search and Find modals
//\! - Directory Map, Batch Rename, Attribute modals
//\! - Error, Success, Progress modals

use crate::app::{App, Modal, ProgressState};
use ratatui::{layout::Rect, style::Style, text::Span, Frame};

/// Create a centered rectangle with fixed width and height
/// All modals should use this instead of percentage-based sizing
pub(super) fn centered_fixed(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
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
            // Taller modal to show z suggestions
            let modal_area = centered_fixed(55, 16, area);
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

/// Draw error modal
pub(super) fn draw_error_modal(frame: &mut Frame, area: Rect, message: &str, app: &App) {
    use crate::ui::components::MessageModal;
    MessageModal::error(message).render(frame, area, &app.colors());
}

/// Draw path input modal with z suggestions
pub(super) fn draw_path_input_modal(frame: &mut Frame, area: Rect, path: &str, app: &App) {
    let colors = app.colors();
    use crate::ui::components::ModalFrame;

    // Use ModalFrame for consistent double-line border styling
    let modal = ModalFrame::themed(area, " Change Directory ", &colors)
        .no_title_separator()
        .no_footer_separator();
    modal.render_frame(frame);

    // Build instruction text
    let z_count = app.z_db.as_ref().map(|db| db.len()).unwrap_or(0);
    let instruction = if z_count > 0 {
        "Enter path or type to search directories:".to_string()
    } else {
        "Enter path (Tab to complete):".to_string()
    };

    // Get jump suggestions based on current input
    let suggestions: Vec<String> = if let Some(db) = app.z_db.as_ref() {
        if path.is_empty() {
            // Show top directories when empty
            db.top_dirs(5)
                .iter()
                .map(|e| e.path.to_string_lossy().to_string())
                .collect()
        } else if !path.contains('/') && !path.contains('\\') {
            // Search jump database for matching dirs
            db.search(path)
                .iter()
                .take(5)
                .map(|e| e.path.to_string_lossy().to_string())
                .collect()
        } else {
            Vec::new() // Don't show suggestions for paths
        }
    } else {
        Vec::new()
    };

    // Content rows
    modal.render_row(frame, 0, vec![]); // Empty row
    modal.render_row(
        frame,
        1,
        vec![Span::styled(
            instruction,
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
    modal.render_row(frame, 4, vec![]); // Spacer

    // Show z suggestions
    if !suggestions.is_empty() {
        modal.render_row(
            frame,
            5,
            vec![Span::styled(
                "Suggestions:",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            )],
        );
        for (i, suggestion) in suggestions.iter().enumerate() {
            // Truncate long paths to fit
            let display = if suggestion.len() > 44 {
                format!("...{}", &suggestion[suggestion.len() - 41..])
            } else {
                suggestion.clone()
            };
            modal.render_row(
                frame,
                (6 + i) as u16,
                vec![Span::styled(
                    format!("  {}", display),
                    Style::default().fg(colors.blue()).bg(colors.bg()),
                )],
            );
        }
    }

    // Help line
    let help = if z_count > 0 {
        vec![("Tab", "complete"), ("Enter", "go"), ("Esc", "cancel")]
    } else {
        vec![("Tab", "complete"), ("Enter", "confirm"), ("Esc", "cancel")]
    };
    modal.render_help(frame, help);
}

/// Draw success modal
pub(super) fn draw_success_modal(frame: &mut Frame, area: Rect, message: &str, app: &App) {
    use crate::ui::components::MessageModal;
    MessageModal::success(message).render(frame, area, &app.colors());
}

/// Draw progress modal for file operations
pub(super) fn draw_progress_modal(frame: &mut Frame, area: Rect, state: &ProgressState, app: &App) {
    use crate::ui::components::ModalFrame;

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

    let title = format!(" {} Files ", state.operation_name());

    // Calculate height based on whether there's an error
    let height: u16 = if state.last_error.is_some() { 14 } else { 12 };
    let width: u16 = 50;
    let modal_area = centered_fixed(width, height, area);

    let modal = ModalFrame::themed(modal_area, &title, &colors).no_footer_separator();
    modal.render_frame(frame);

    // Content rows
    modal.render_row(frame, 0, vec![]);
    modal.render_row(
        frame,
        1,
        vec![Span::styled(
            format!("{} {} of {}", state.operation_name(), current, total),
            Style::default().fg(colors.fg()),
        )],
    );
    modal.render_row(frame, 2, vec![]);
    modal.render_row(
        frame,
        3,
        vec![Span::styled(
            &progress_bar,
            Style::default().fg(colors.blue()),
        )],
    );
    modal.render_row(
        frame,
        4,
        vec![Span::styled(
            format!("{}%", percentage),
            Style::default().fg(colors.green()),
        )],
    );
    modal.render_row(frame, 5, vec![]);
    modal.render_row(
        frame,
        6,
        vec![Span::styled(
            &current_file,
            Style::default().fg(colors.yellow()),
        )],
    );

    let mut row = 7;

    // Show error if any
    if let Some(ref err) = state.last_error {
        modal.render_row(frame, row, vec![]);
        row += 1;
        modal.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("Error: {}", err),
                Style::default().fg(colors.red()),
            )],
        );
        row += 1;
    }

    // Show stats and cancel hint
    modal.render_row(frame, row, vec![]);
    modal.render_row(
        frame,
        row + 1,
        vec![Span::styled(
            format!("Completed: {}  Failed: {}", state.completed, state.failed),
            Style::default().fg(colors.green()),
        )],
    );

    // Cancel hint in footer area
    modal.render_help(frame, vec![("Esc", "cancel")]);
}
