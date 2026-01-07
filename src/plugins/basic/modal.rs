//! BASIC Runner plugin modal rendering
//!
//! UI for the BASIC interpreter plugin.

use super::state::{BasicInterpreter, BasicState, BasicView};
use crate::app::ThemeColors;
use crate::ui::components::{FullScreenView, ModalFrame};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

/// Calculate a centered modal area from the full screen area
fn centered_modal_area(area: Rect) -> Rect {
    let width = area.width.min(60);
    let height = area.height.min(20);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

pub fn draw_basic_modal(frame: &mut Frame, area: Rect, state: &BasicState, colors: &ThemeColors) {
    match state.view {
        BasicView::Menu => draw_menu(frame, area, state, colors),
        BasicView::Running => draw_running(frame, area, state, colors),
        BasicView::Output => draw_output(frame, area, state, colors),
        BasicView::Error => draw_error(frame, area, state, colors),
    }
}

fn draw_menu(frame: &mut Frame, area: Rect, state: &BasicState, colors: &ThemeColors) {
    let modal_area = centered_modal_area(area);
    let modal = ModalFrame::themed(modal_area, " BASIC Runner ", colors);
    modal.render_frame(frame);
    let content_area = modal.content_area();

    let chunks = Layout::vertical([
        Constraint::Length(3), // Info
        Constraint::Min(5),    // Interpreter list
        Constraint::Length(3), // File info
    ])
    .split(content_area);

    // Info section
    let info_text = if state.available_interpreters.is_empty() {
        vec![
            Line::from(Span::styled(
                "No BASIC interpreters found!",
                Style::default().fg(colors.red()),
            )),
            Line::from(Span::styled(
                "Install one: brew install bas55",
                Style::default().fg(colors.grey()),
            )),
        ]
    } else {
        vec![
            Line::from(Span::styled(
                "Select a BASIC interpreter:",
                Style::default().fg(colors.fg()),
            )),
            Line::from(""),
        ]
    };
    let info = Paragraph::new(info_text);
    frame.render_widget(info, chunks[0]);

    // Interpreter list
    let items: Vec<Line> = if state.available_interpreters.is_empty() {
        BasicInterpreter::all()
            .iter()
            .map(|i| {
                Line::from(vec![
                    Span::styled("  [ ] ", Style::default().fg(colors.grey())),
                    Span::styled(i.name(), Style::default().fg(colors.grey())),
                    Span::styled(
                        format!("  ({})", i.install_hint()),
                        Style::default().fg(colors.grey()),
                    ),
                ])
            })
            .collect()
    } else {
        state
            .available_interpreters
            .iter()
            .enumerate()
            .map(|(idx, interp)| {
                let is_selected = idx == state.selected_interpreter;
                let marker = if is_selected { ">" } else { " " };
                let style = if is_selected {
                    Style::default()
                        .fg(colors.yellow())
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(colors.fg())
                };
                Line::from(vec![
                    Span::styled(format!(" {} ", marker), style),
                    Span::styled(interp.name(), style),
                ])
            })
            .collect()
    };
    let list = Paragraph::new(items).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(colors.fg())),
    );
    frame.render_widget(list, chunks[1]);

    // File info
    let file_info = if let Some(path) = &state.file_path {
        let filename = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        vec![Line::from(vec![
            Span::styled("File: ", Style::default().fg(colors.grey())),
            Span::styled(filename, Style::default().fg(colors.cyan())),
        ])]
    } else {
        vec![Line::from(Span::styled(
            "No file selected - select a .bas file first",
            Style::default().fg(colors.grey()),
        ))]
    };
    let file_para = Paragraph::new(file_info).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(colors.fg())),
    );
    frame.render_widget(file_para, chunks[2]);

    // Help
    let help = if state.available_interpreters.is_empty() {
        vec![("Esc", "close")]
    } else if state.file_path.is_some() {
        vec![
            ("↑↓", "select"),
            ("Enter", "run"),
            ("r", "run"),
            ("Esc", "close"),
        ]
    } else {
        vec![("↑↓", "select"), ("Esc", "close")]
    };
    modal.render_help(frame, help);
}

fn draw_running(frame: &mut Frame, area: Rect, state: &BasicState, colors: &ThemeColors) {
    let modal_area = centered_modal_area(area);
    let modal = ModalFrame::themed(modal_area, " BASIC - Running ", colors);
    modal.render_frame(frame);
    let content_area = modal.content_area();

    let interp_name = state
        .selected()
        .map(|i| i.name())
        .unwrap_or("Unknown interpreter");

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Running with {}...", interp_name),
            Style::default().fg(colors.yellow()),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Please wait...",
            Style::default().fg(colors.grey()),
        )),
    ];

    let para = Paragraph::new(text);
    frame.render_widget(para, content_area);
}

fn draw_output(frame: &mut Frame, area: Rect, state: &BasicState, colors: &ThemeColors) {
    // Use full-screen view for output to show more content
    let screen = FullScreenView::new(area, " BASIC - Output ", colors);
    screen.render_frame(frame);
    let content_area = screen.content_area();

    let visible_height = content_area.height as usize;
    let lines: Vec<Line> = state
        .output
        .iter()
        .skip(state.scroll_offset)
        .take(visible_height)
        .map(|line| Line::from(Span::raw(line)))
        .collect();

    let para = Paragraph::new(lines);
    frame.render_widget(para, content_area);

    // Scrollbar if needed
    if state.output.len() > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        let mut scrollbar_state =
            ScrollbarState::new(state.output.len()).position(state.scroll_offset);
        frame.render_stateful_widget(
            scrollbar,
            content_area.inner(ratatui::layout::Margin {
                vertical: 0,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }

    screen.render_help(
        frame,
        vec![("↑↓", "scroll"), ("r", "run again"), ("Esc", "back")],
    );
}

fn draw_error(frame: &mut Frame, area: Rect, state: &BasicState, colors: &ThemeColors) {
    let modal_area = centered_modal_area(area);
    let modal = ModalFrame::themed(modal_area, " BASIC - Error ", colors);
    modal.render_frame(frame);
    let content_area = modal.content_area();

    let error_msg = state.error.as_deref().unwrap_or("Unknown error");

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Error running BASIC program:",
            Style::default().fg(colors.red()),
        )),
        Line::from(""),
        Line::from(Span::styled(error_msg, Style::default().fg(colors.fg()))),
    ];

    let para = Paragraph::new(text);
    frame.render_widget(para, content_area);

    modal.render_help(frame, vec![("Enter/Esc", "back")]);
}
