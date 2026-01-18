//! File operations modal drawing functions

use crate::app::{App, BatchRenameState};
use crate::ui::components::ModalFrame;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

/// Draw copy modal using ModalFrame for consistent styling
pub fn draw_copy_modal(frame: &mut Frame, area: Rect, dest: &str, app: &App) {
    let colors = app.colors();
    let modal = ModalFrame::themed(area, " COPY FILES ", &colors);
    modal.render_frame(frame);

    // Row 0: Copy count
    modal.render_row(
        frame,
        0,
        vec![Span::styled(
            format!("Copying {} tagged file(s)", app.tagged_files.len()),
            Style::default().fg(colors.yellow()).bg(colors.bg()),
        )],
    );

    // Row 1: Empty
    modal.render_row(frame, 1, vec![]);

    // Row 2: Destination prompt
    modal.render_row(
        frame,
        2,
        vec![Span::styled(
            "Destination (Tab to complete):",
            Style::default().fg(colors.green()).bg(colors.bg()),
        )],
    );

    // Row 3: Input field
    modal.render_row(
        frame,
        3,
        vec![Span::styled(
            format!("{}_", dest),
            Style::default().fg(colors.fg()).bg(colors.bg()),
        )],
    );

    // Help row
    modal.render_help(
        frame,
        vec![("Tab", "complete"), ("Enter", "copy"), ("Esc", "cancel")],
    );
}

/// Draw move modal using ModalFrame for consistent styling
pub fn draw_move_modal(frame: &mut Frame, area: Rect, dest: &str, app: &App) {
    let colors = app.colors();
    let modal = ModalFrame::themed(area, " MOVE FILES ", &colors);
    modal.render_frame(frame);

    // Row 0: Move count
    modal.render_row(
        frame,
        0,
        vec![Span::styled(
            format!("Moving {} tagged file(s)", app.tagged_files.len()),
            Style::default().fg(colors.yellow()).bg(colors.bg()),
        )],
    );

    // Row 1: Empty
    modal.render_row(frame, 1, vec![]);

    // Row 2: Destination prompt
    modal.render_row(
        frame,
        2,
        vec![Span::styled(
            "Destination (Tab to complete):",
            Style::default().fg(colors.green()).bg(colors.bg()),
        )],
    );

    // Row 3: Input field
    modal.render_row(
        frame,
        3,
        vec![Span::styled(
            format!("{}_", dest),
            Style::default().fg(colors.fg()).bg(colors.bg()),
        )],
    );

    // Help row
    modal.render_help(
        frame,
        vec![("Tab", "complete"), ("Enter", "move"), ("Esc", "cancel")],
    );
}

/// Draw erase confirmation modal using ModalFrame for consistent styling
pub fn draw_erase_modal(frame: &mut Frame, area: Rect, app: &App) {
    let colors = app.colors();
    let modal = ModalFrame::themed(area, " ERASE FILES ", &colors).title_style(
        Style::default()
            .fg(colors.red())
            .bg(colors.bg())
            .add_modifier(Modifier::BOLD),
    );
    modal.render_frame(frame);

    // Row 0: Delete count
    modal.render_row(
        frame,
        0,
        vec![Span::styled(
            format!("Delete {} tagged file(s)?", app.tagged_files.len()),
            Style::default().fg(colors.yellow()).bg(colors.bg()),
        )],
    );

    // Row 1: Empty
    modal.render_row(frame, 1, vec![]);

    // Row 2: Warning
    modal.render_row(
        frame,
        2,
        vec![Span::styled(
            "This cannot be undone!",
            Style::default().fg(colors.red()).bg(colors.bg()),
        )],
    );

    // Help row
    modal.render_help(frame, vec![("Y", "es"), ("N", "o")]);
}

/// Draw rename modal
pub fn draw_rename_modal(frame: &mut Frame, area: Rect, name: &str, app: &App) {
    use ratatui::widgets::{Block, Borders, Wrap};

    let colors = app.colors();

    // Use the modal area directly (already centered by draw_modal)
    let rename_block = Block::default()
        .title(" Rename File ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors.fg()))
        .style(Style::default().bg(colors.bg()));

    let rename_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Enter new name:",
            Style::default().fg(colors.green()),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("{}_", name),
            Style::default().fg(colors.fg()),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Enter", Style::default().fg(colors.fg())),
            Span::raw(" rename, "),
            Span::styled("Esc", Style::default().fg(colors.fg())),
            Span::raw(" cancel"),
        ]),
    ];

    let paragraph = Paragraph::new(rename_text)
        .block(rename_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Draw the Batch Rename modal
pub fn draw_batch_rename_modal(frame: &mut Frame, area: Rect, state: &BatchRenameState, app: &App) {
    let colors = app.colors();
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

    if let Some((path, original_name)) = state.current_file() {
        let mut lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "File to be renamed:",
                Style::default().fg(colors.green()),
            )),
            Line::from(""),
            Line::from(Span::styled(
                original_name.clone(),
                Style::default().fg(colors.fg()),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("Path: {}", path.parent().unwrap_or(path).display()),
                Style::default().fg(colors.grey()),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Enter new name:",
                Style::default().fg(colors.green()),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    &state.input,
                    Style::default().fg(colors.yellow()).bg(colors.red()),
                ),
                Span::styled("█", Style::default().fg(colors.yellow()).bg(colors.red())),
            ]),
        ];

        // Show error if any
        if let Some(ref error) = state.last_error {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                error.clone(),
                Style::default().fg(colors.red()),
            )));
        }

        // Show progress
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("Renamed so far: {}", state.renamed_count),
            Style::default().fg(colors.grey()),
        )));

        frame.render_widget(Paragraph::new(lines), content_area);
    }

    // Bottom separator
    frame.render_widget(
        Paragraph::new(Span::styled(sep, Style::default().fg(colors.fg()))),
        chunks[3],
    );

    // Help line
    let help_text = "Enter: Rename  Tab: Skip  ESC: Exit";
    frame.render_widget(
        Paragraph::new(Span::styled(help_text, Style::default().fg(colors.green()))),
        chunks[4],
    );
}
