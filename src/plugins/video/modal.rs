//! Video Player plugin modal rendering
//!
//! UI for the video player plugin.

use super::state::{VideoPlayer, VideoState, VideoView};
use crate::app::ThemeColors;
use crate::ui::components::ModalFrame;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Calculate a centered modal area from the full screen area
fn centered_modal_area(area: Rect) -> Rect {
    let width = area.width.min(50);
    let height = area.height.min(14);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

pub fn draw_video_modal(frame: &mut Frame, area: Rect, state: &VideoState, colors: &ThemeColors) {
    match state.view {
        VideoView::Menu => draw_menu(frame, area, state, colors),
        VideoView::Playing => draw_playing(frame, area, state, colors),
        VideoView::Error => draw_error(frame, area, state, colors),
    }
}

fn draw_menu(frame: &mut Frame, area: Rect, state: &VideoState, colors: &ThemeColors) {
    let modal_area = centered_modal_area(area);
    let modal = ModalFrame::themed(modal_area, " Video Player ", colors);
    modal.render_frame(frame);
    let content_area = modal.content_area();

    let chunks = Layout::vertical([
        Constraint::Length(2), // Info
        Constraint::Min(4),    // Player list
        Constraint::Length(2), // File info
    ])
    .split(content_area);

    // Info section
    let info_text = if state.available_players.is_empty() {
        vec![
            Line::from(Span::styled(
                "No video players found!",
                Style::default().fg(colors.red()),
            )),
            Line::from(Span::styled(
                "Install: brew install mpv",
                Style::default().fg(colors.grey()),
            )),
        ]
    } else {
        vec![Line::from(Span::styled(
            "Select a video player:",
            Style::default().fg(colors.fg()),
        ))]
    };
    let info = Paragraph::new(info_text);
    frame.render_widget(info, chunks[0]);

    // Player list
    let items: Vec<Line> = if state.available_players.is_empty() {
        VideoPlayer::all()
            .iter()
            .map(|p| {
                Line::from(vec![
                    Span::styled("  [ ] ", Style::default().fg(colors.grey())),
                    Span::styled(p.name(), Style::default().fg(colors.grey())),
                    Span::styled(
                        format!("  ({})", p.install_hint()),
                        Style::default().fg(colors.grey()),
                    ),
                ])
            })
            .collect()
    } else {
        state
            .available_players
            .iter()
            .enumerate()
            .map(|(idx, player)| {
                let is_selected = idx == state.selected_player;
                let is_recommended = idx == 0;
                let marker = if is_selected { ">" } else { " " };
                let style = if is_selected {
                    Style::default()
                        .fg(colors.yellow())
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(colors.fg())
                };
                let mut spans = vec![
                    Span::styled(format!(" {} ", marker), style),
                    Span::styled(player.name(), style),
                ];
                if is_recommended && state.available_players.len() > 1 {
                    spans.push(Span::styled(
                        "  * recommended",
                        Style::default().fg(colors.green()),
                    ));
                }
                Line::from(spans)
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
            "No file selected - select a video file",
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
    let help = if state.available_players.is_empty() {
        vec![("Esc", "close")]
    } else if state.file_path.is_some() {
        vec![("↑↓", "select"), ("Enter", "play"), ("Esc", "close")]
    } else {
        vec![("↑↓", "select"), ("Esc", "close")]
    };
    modal.render_help(frame, help);
}

fn draw_playing(frame: &mut Frame, area: Rect, state: &VideoState, colors: &ThemeColors) {
    let modal_area = centered_modal_area(area);
    let modal = ModalFrame::themed(modal_area, " Video - Playing ", colors);
    modal.render_frame(frame);
    let content_area = modal.content_area();

    let player_name = state
        .selected()
        .map(|p| p.name())
        .unwrap_or("Unknown player");

    let filename = state
        .file_path
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Playing: {}", filename),
            Style::default().fg(colors.cyan()),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("Using: {}", player_name),
            Style::default().fg(colors.grey()),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Video will open in external player",
            Style::default().fg(colors.yellow()),
        )),
    ];

    let para = Paragraph::new(text);
    frame.render_widget(para, content_area);

    modal.render_help(frame, vec![("Esc", "close")]);
}

fn draw_error(frame: &mut Frame, area: Rect, state: &VideoState, colors: &ThemeColors) {
    let modal_area = centered_modal_area(area);
    let modal = ModalFrame::themed(modal_area, " Video - Error ", colors);
    modal.render_frame(frame);
    let content_area = modal.content_area();

    let error_msg = state.error.as_deref().unwrap_or("Unknown error");

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Error playing video:",
            Style::default().fg(colors.red()),
        )),
        Line::from(""),
        Line::from(Span::styled(error_msg, Style::default().fg(colors.fg()))),
    ];

    let para = Paragraph::new(text);
    frame.render_widget(para, content_area);

    modal.render_help(frame, vec![("Enter/Esc", "back")]);
}
