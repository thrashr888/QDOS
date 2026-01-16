//! Audio Player modal rendering

use super::soundfont;
use super::state::{AudioState, AudioType, AudioView, PlayState};
use qdos_plugin_api::prelude::ModalFrame;
use qdos_plugin_api::prelude::ThemeColors;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;

/// Calculate centered modal area
fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width.saturating_sub(2));
    let h = height.min(area.height.saturating_sub(2));
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}

/// Check if soundfont is available (for MIDI playback)
pub fn has_soundfont() -> bool {
    soundfont::get_soundfont().is_some()
}

/// Draw the audio player modal
pub fn draw_audio_modal(frame: &mut Frame, area: Rect, state: &AudioState, colors: &ThemeColors) {
    match state.view {
        AudioView::Menu => draw_menu_view(frame, area, state, colors),
        AudioView::Player => draw_player_view(frame, area, state, colors),
        AudioView::ExternalPlaying => draw_external_playing(frame, area, state, colors),
        AudioView::NeedsSoundFont => draw_needs_soundfont(frame, area, state, colors),
        AudioView::DownloadingSoundFont => draw_downloading_soundfont(frame, area, state, colors),
        AudioView::Error => draw_error_view(frame, area, state, colors),
    }
}

fn draw_player_view(frame: &mut Frame, area: Rect, state: &AudioState, colors: &ThemeColors) {
    let modal_area = centered_rect(area, 55, 12);
    let modal = ModalFrame::themed(modal_area, " Audio Player ", colors);
    modal.render_frame(frame);

    let content_area = modal.content_area();
    let mut lines: Vec<Line> = Vec::new();

    // File name
    if state.file_name.is_empty() {
        lines.push(Line::from(Span::styled(
            "No file loaded",
            Style::default().fg(colors.grey()),
        )));
    } else {
        lines.push(Line::from(vec![
            Span::styled("Now Playing: ", Style::default().fg(colors.blue())),
            Span::styled(
                &state.file_name,
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    }
    lines.push(Line::from(""));

    // Play state icon
    let state_icon = match state.play_state {
        PlayState::Playing => "\u{25b6}", // ▶
        PlayState::Paused => "\u{23f8}",  // ⏸
        PlayState::Stopped => "\u{23f9}", // ⏹
    };

    // Progress bar
    let progress = state.progress();
    let bar_width: usize = 30;
    let filled = (progress * bar_width as f32) as usize;
    let empty = bar_width.saturating_sub(filled);
    let progress_bar = format!("{}{}", "=".repeat(filled), "-".repeat(empty));

    lines.push(Line::from(vec![
        Span::styled(
            format!(" {} ", state_icon),
            Style::default().fg(colors.green()),
        ),
        Span::styled(
            format!("[{}]", progress_bar),
            Style::default().fg(colors.blue()),
        ),
    ]));

    lines.push(Line::from(Span::styled(
        format!(
            "    {} / {}",
            state.format_position(),
            state.format_duration()
        ),
        Style::default().fg(colors.fg()),
    )));
    lines.push(Line::from(""));

    // Volume bar
    let vol_width: usize = 20;
    let vol_filled = (state.volume * vol_width as f32) as usize;
    let vol_empty = vol_width.saturating_sub(vol_filled);
    let vol_bar = format!("{}{}", "#".repeat(vol_filled), ".".repeat(vol_empty));

    lines.push(Line::from(vec![
        Span::styled(" Volume: ", Style::default().fg(colors.fg())),
        Span::styled(
            format!("[{}]", vol_bar),
            Style::default().fg(colors.green()),
        ),
        Span::styled(
            format!(" {:>3}%", (state.volume * 100.0) as u8),
            Style::default().fg(colors.fg()),
        ),
    ]));
    lines.push(Line::from(""));

    // Status line
    let status = match state.play_state {
        PlayState::Playing => "Playing",
        PlayState::Paused => "Paused",
        PlayState::Stopped => "Stopped",
    };
    lines.push(Line::from(Span::styled(
        format!(" Status: {}", status),
        Style::default().fg(colors.grey()),
    )));

    frame.render_widget(Paragraph::new(lines), content_area);

    // Build help with prev/next if available
    let mut help = vec![("Space", "play/pause"), ("\u{2191}\u{2193}", "volume")];
    if state.has_prev() || state.has_next() {
        help.push(("[/]", "prev/next"));
    }
    help.push(("S", "stop"));
    help.push(("Esc", "close"));
    modal.render_help(frame, help);
}

fn draw_menu_view(frame: &mut Frame, area: Rect, state: &AudioState, colors: &ThemeColors) {
    let title = format!(" Audio Player - {} ", state.file_name.as_str());
    let modal_area = centered_rect(area, 55, 16);
    let modal = ModalFrame::themed(modal_area, &title, colors);
    modal.render_frame(frame);

    let content_area = modal.content_area();
    let mut lines: Vec<Line> = Vec::new();

    let type_label = match state.audio_type {
        AudioType::Native => "Audio",
        AudioType::Midi => "MIDI",
    };
    lines.push(Line::from(Span::styled(
        format!("Select {} Player:", type_label),
        Style::default()
            .fg(colors.blue())
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    if state.available_players.is_empty() {
        lines.push(Line::from(Span::styled(
            " No players available",
            Style::default().fg(colors.red()),
        )));
        if state.audio_type == AudioType::Midi {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                " Install: brew install fluid-synth",
                Style::default().fg(colors.grey()),
            )));
            lines.push(Line::from(Span::styled(
                "          brew install timidity",
                Style::default().fg(colors.grey()),
            )));
        }
    } else {
        for (i, player) in state.available_players.iter().enumerate() {
            let is_selected = i == state.selected_player;
            let prefix = if is_selected { "> " } else { "  " };

            let style = if is_selected {
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.fg())
            };

            lines.push(Line::from(Span::styled(
                format!("{}{}", prefix, player.name()),
                style,
            )));
            // Show description
            lines.push(Line::from(Span::styled(
                format!("    {}", player.description()),
                Style::default().fg(colors.grey()),
            )));
        }

        // Show soundfont status for MIDI files
        if state.audio_type == AudioType::Midi {
            lines.push(Line::from(""));
            if has_soundfont() {
                lines.push(Line::from(Span::styled(
                    " ✓ SoundFont available",
                    Style::default().fg(colors.green()),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    " ! No SoundFont - press D to download",
                    Style::default().fg(colors.red()),
                )));
            }
        }
    }

    frame.render_widget(Paragraph::new(lines), content_area);

    if state.available_players.is_empty() {
        modal.render_help(frame, vec![("Esc", "close")]);
    } else {
        let mut help = vec![("\u{2191}\u{2193}", "select"), ("Enter", "play")];
        // Add download hint for MIDI without soundfont
        if state.audio_type == AudioType::Midi && !has_soundfont() {
            help.push(("D", "download SF"));
        }
        help.push(("Esc", "close"));
        modal.render_help(frame, help);
    }
}

fn draw_external_playing(frame: &mut Frame, area: Rect, state: &AudioState, colors: &ThemeColors) {
    let modal_area = centered_rect(area, 55, 10);
    let modal = ModalFrame::themed(modal_area, " Audio Player ", colors);
    modal.render_frame(frame);

    let content_area = modal.content_area();
    let player_name = state
        .selected()
        .map(|p| p.name())
        .unwrap_or("External Player");

    // Playing animation
    let playing_icon = "\u{25b6}"; // ▶

    // File position indicator
    let position_str = state.file_position();

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!(" {} ", playing_icon),
                Style::default().fg(colors.green()),
            ),
            Span::styled(
                &state.file_name,
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            ),
            if !position_str.is_empty() {
                Span::styled(
                    format!("  [{}]", position_str),
                    Style::default().fg(colors.grey()),
                )
            } else {
                Span::raw("")
            },
        ]),
        Line::from(""),
        Line::from(Span::styled(
            format!(" Using: {}", player_name),
            Style::default().fg(colors.fg()),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " (Playing via external synthesizer)",
            Style::default().fg(colors.grey()),
        )),
    ];

    frame.render_widget(Paragraph::new(lines), content_area);

    // Build help with prev/next if available
    let mut help = vec![];
    if state.has_prev() || state.has_next() {
        help.push(("[/]", "prev/next"));
    }
    help.push(("S", "stop"));
    help.push(("Esc", "close"));
    modal.render_help(frame, help);
}

fn draw_error_view(frame: &mut Frame, area: Rect, state: &AudioState, colors: &ThemeColors) {
    let modal_area = centered_rect(area, 60, 10);
    let modal = ModalFrame::themed(modal_area, " Audio Player - Error ", colors);
    modal.render_frame(frame);

    let content_area = modal.content_area();

    let error_text = state.error.as_deref().unwrap_or("Unknown error occurred");

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            " Playback Error",
            Style::default()
                .fg(colors.red())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!(" {}", error_text),
            Style::default().fg(colors.fg()),
        )),
    ];

    let para = Paragraph::new(lines).wrap(Wrap { trim: true });
    frame.render_widget(para, content_area);

    modal.render_help(frame, vec![("Enter/Esc", "close")]);
}

fn draw_needs_soundfont(frame: &mut Frame, area: Rect, state: &AudioState, colors: &ThemeColors) {
    let modal_area = centered_rect(area, 55, 12);
    let modal = ModalFrame::themed(modal_area, " SoundFont Required ", colors);
    modal.render_frame(frame);

    let content_area = modal.content_area();

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            " A General MIDI SoundFont is required for MIDI playback.",
            Style::default().fg(colors.fg()),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " Would you like to download GeneralUser GS (~30MB)?",
            Style::default().fg(colors.yellow()),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!(" File: {}", state.file_name),
            Style::default().fg(colors.grey()),
        )),
    ];

    frame.render_widget(Paragraph::new(lines), content_area);
    modal.render_help(
        frame,
        vec![("D", "download"), ("S", "skip"), ("Esc", "cancel")],
    );
}

fn draw_downloading_soundfont(
    frame: &mut Frame,
    area: Rect,
    _state: &AudioState,
    colors: &ThemeColors,
) {
    let modal_area = centered_rect(area, 50, 8);
    let modal = ModalFrame::themed(modal_area, " Downloading SoundFont ", colors);
    modal.render_frame(frame);

    let content_area = modal.content_area();

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            " Downloading GeneralUser GS SoundFont...",
            Style::default().fg(colors.yellow()),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " (~30MB - please wait)",
            Style::default().fg(colors.grey()),
        )),
    ];

    frame.render_widget(Paragraph::new(lines), content_area);
    // No help - user should wait
}
