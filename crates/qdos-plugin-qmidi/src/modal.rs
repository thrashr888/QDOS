//! Q-MIDI Modal Rendering
//!
//! UI rendering for all Q-MIDI views.

use crate::state::{FileAction, QMidiState, QMidiView};
use qdos_plugin_api::prelude::*;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    Frame,
};

/// Main modal draw dispatcher
pub fn draw_qmidi_modal(frame: &mut Frame, area: Rect, state: &QMidiState, colors: &ThemeColors) {
    match state.view {
        QMidiView::PianoRoll => draw_piano_roll(frame, area, state, colors),
        QMidiView::EventList => draw_event_list(frame, area, state, colors),
        QMidiView::TrackList => draw_track_list(frame, area, state, colors),
        QMidiView::MidiDevices => draw_midi_devices(frame, area, state, colors),
        QMidiView::FileMenu => draw_file_menu(frame, area, state, colors),
        QMidiView::Help => draw_help(frame, area, state, colors),
    }
}

/// Draw piano roll view (main editor)
fn draw_piano_roll(frame: &mut Frame, area: Rect, state: &QMidiState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, &format!(" Q-MIDI: {} ", state.display_name()), colors);
    view.render_frame(frame);

    let content = view.content_area();
    let mut row: u16 = 0;

    // Track header strip (3 rows)
    draw_track_header(frame, &view, state, colors, &mut row);

    // Separator
    row += 1;

    // Piano roll grid
    let grid_height = content.height.saturating_sub(row + 2) as usize;
    draw_piano_grid(frame, &view, state, colors, row, grid_height);

    // Status bar
    draw_status_bar(frame, &view, state, colors);

    // Help footer
    let help = if state.playing {
        vec![
            ("Space", "stop"),
            ("R", "rec"),
            ("Tab", "view"),
            ("D", "dev"),
            ("Esc", "exit"),
        ]
    } else {
        vec![
            ("Space", "play"),
            ("R", "rec"),
            ("Tab", "view"),
            ("Enter", "note"),
            ("D", "dev"),
            ("Esc", "exit"),
        ]
    };
    view.render_help(frame, help);
}

/// Draw track header strip
fn draw_track_header(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &QMidiState,
    colors: &ThemeColors,
    row: &mut u16,
) {
    // Header row
    let header = format!(
        "  {:>3}  {:<12} {:>2}  {:>3}  {:>3}  [M][S]",
        "Trk", "Name", "Ch", "Vol", "Pan"
    );
    view.render_row(
        frame,
        *row,
        vec![Span::styled(header, Style::default().fg(colors.blue()))],
    );
    *row += 1;

    // Track rows (show up to 3)
    let visible_tracks: Vec<_> = state
        .tracks
        .iter()
        .enumerate()
        .skip(state.track_scroll)
        .take(3)
        .collect();

    for (idx, track) in visible_tracks {
        let selected = idx == state.current_track;
        let marker = if selected { ">" } else { " " };
        let mute = if track.muted { "M" } else { " " };
        let solo = if track.solo { "*" } else { " " };

        let line = format!(
            "{} {:>3}  {:<12} {:>2}  {:>3}  {:>3}  [{}][{}]",
            marker,
            idx + 1,
            truncate(&track.name, 12),
            track.channel + 1,
            track.volume,
            pan_display(track.pan),
            mute,
            solo
        );

        let style = if selected {
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.fg())
        };

        view.render_row(frame, *row, vec![Span::styled(line, style)]);
        *row += 1;
    }

    // Pad if fewer than 3 tracks
    while *row < 4 {
        view.render_row(frame, *row, vec![Span::raw("")]);
        *row += 1;
    }
}

/// Draw piano roll grid
fn draw_piano_grid(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &QMidiState,
    colors: &ThemeColors,
    start_row: u16,
    height: usize,
) {
    let content = view.content_area();
    let grid_width = content.width.saturating_sub(5) as usize; // 5 chars for pitch label

    // Time ruler
    let mut ruler = String::from("    ");
    let ticks_per_cell = state.ppqn as u32 / state.zoom_x as u32;
    let cells_per_beat = state.zoom_x as usize;

    for col in 0..grid_width {
        let tick = state.scroll_x + (col as u32 * ticks_per_cell);
        let beat = tick / state.ppqn as u32;
        let beat_in_measure = beat % state.time_signature.0 as u32;

        if col % cells_per_beat == 0 {
            if beat_in_measure == 0 {
                // Measure start
                let measure = beat / state.time_signature.0 as u32 + 1;
                let label = format!("{}", measure);
                if col + label.len() <= grid_width {
                    ruler.push_str(&label);
                    for _ in label.len()..cells_per_beat.min(grid_width - col) {
                        ruler.push(' ');
                    }
                    continue;
                }
            }
            ruler.push('|');
        } else {
            ruler.push(' ');
        }
    }
    view.render_row(
        frame,
        start_row,
        vec![Span::styled(
            truncate(&ruler, content.width as usize),
            Style::default().fg(colors.grey()),
        )],
    );

    // Piano grid rows
    let current_track = state.current_track();

    for row_offset in 1..height {
        let pitch = state
            .scroll_y
            .saturating_add((height - row_offset - 1) as u8);
        if pitch > 127 {
            continue;
        }

        // Pitch label
        let note_names = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];
        let octave = (pitch / 12) as i8 - 1;
        let note_idx = (pitch % 12) as usize;
        let label = format!("{:>2}{}", note_names[note_idx], octave);

        let is_black_key = matches!(note_idx, 1 | 3 | 6 | 8 | 10);
        let is_cursor_row = pitch == state.cursor_pitch;

        let mut spans = vec![Span::styled(
            format!("{:>4}", label),
            if is_cursor_row {
                Style::default().fg(colors.yellow())
            } else {
                Style::default().fg(colors.grey())
            },
        )];

        // Grid cells
        let mut grid_line = String::new();
        for col in 0..grid_width {
            let tick = state.scroll_x + (col as u32 * ticks_per_cell);
            let tick_end = tick + ticks_per_cell;

            // Check for notes at this position
            let has_note = current_track
                .map(|t| {
                    t.notes
                        .iter()
                        .any(|n| n.pitch == pitch && n.start_tick < tick_end && n.end_tick() > tick)
                })
                .unwrap_or(false);

            let is_cursor = pitch == state.cursor_pitch
                && tick <= state.cursor_tick
                && tick_end > state.cursor_tick;

            let is_playhead = state.playing && tick <= state.position && tick_end > state.position;

            let ch = if has_note {
                '#' // Note block
            } else if is_black_key {
                '-'
            } else {
                ' '
            };

            if is_cursor {
                grid_line.push('[');
            } else if is_playhead {
                grid_line.push('|');
            } else if has_note {
                grid_line.push(ch);
            } else {
                // Beat/measure lines
                let beat = tick / state.ppqn as u32;
                let beat_in_measure = beat % state.time_signature.0 as u32;
                let on_beat = tick.is_multiple_of(state.ppqn as u32);

                if on_beat && beat_in_measure == 0 {
                    grid_line.push('|');
                } else if on_beat {
                    grid_line.push(':');
                } else if is_black_key {
                    grid_line.push('-');
                } else {
                    grid_line.push('.');
                }
            }
        }

        let grid_style = if is_cursor_row {
            Style::default().fg(colors.yellow())
        } else if is_black_key {
            Style::default().fg(colors.grey())
        } else {
            Style::default().fg(colors.fg())
        };

        spans.push(Span::styled(grid_line, grid_style));

        view.render_row(frame, start_row + row_offset as u16, spans);
    }
}

/// Draw status bar
fn draw_status_bar(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &QMidiState,
    colors: &ThemeColors,
) {
    let play_symbol = if state.playing {
        if state.recording {
            "REC"
        } else {
            ">>>"
        }
    } else {
        "|||"
    };

    let output = state.output_port.as_deref().unwrap_or("No device");

    let status = format!(
        " {} {:>3} BPM  {}/{}  | {}  | Out: {}",
        play_symbol,
        state.tempo,
        state.time_signature.0,
        state.time_signature.1,
        state.format_position(state.position),
        truncate(output, 25)
    );

    view.render_footer(
        frame,
        vec![Span::styled(status, Style::default().fg(colors.green()))],
    );
}

/// Draw event list view
fn draw_event_list(frame: &mut Frame, area: Rect, state: &QMidiState, colors: &ThemeColors) {
    let title = format!(" Q-MIDI: Event List - {} ", state.display_name());
    let view = FullScreenView::new(area, &title, colors);
    view.render_frame(frame);

    let content = view.content_area();
    let mut row = 0;

    // Track info
    let track_name = state
        .current_track()
        .map(|t| t.name.as_str())
        .unwrap_or("No track");
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!(" Track: {} - {}", state.current_track + 1, track_name),
            Style::default().fg(colors.blue()),
        )],
    );
    row += 1;

    // Header
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "   #     Time        Event       Ch  Note   Vel",
            Style::default().fg(colors.grey()),
        )],
    );
    row += 1;

    // Events
    let visible_height = content.height.saturating_sub(4) as usize;
    if let Some(track) = state.current_track() {
        for (idx, note) in track
            .notes
            .iter()
            .enumerate()
            .skip(state.event_scroll)
            .take(visible_height)
        {
            let selected = idx == state.event_selected;
            let marker = if selected { ">" } else { " " };

            let line = format!(
                " {} {:>4}  {}  Note On     {:>2}  {:<5}  {:>3}",
                marker,
                idx + 1,
                state.format_position(note.start_tick),
                track.channel + 1,
                note.name(),
                note.velocity
            );

            let style = if selected {
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.fg())
            };

            view.render_row(frame, row, vec![Span::styled(line, style)]);
            row += 1;
        }
    }

    // Footer info
    let event_count = state.event_count();
    view.render_footer(
        frame,
        vec![Span::styled(
            format!(
                " Events: {}  |  Selected: {}",
                event_count,
                state.event_selected + 1
            ),
            Style::default().fg(colors.green()),
        )],
    );

    view.render_help(
        frame,
        vec![
            ("Enter", "edit"),
            ("Del", "delete"),
            ("Tab", "piano"),
            ("Esc", "exit"),
        ],
    );
}

/// Draw track list view
fn draw_track_list(frame: &mut Frame, area: Rect, state: &QMidiState, colors: &ThemeColors) {
    let title = format!(" Q-MIDI: Track List - {} ", state.display_name());
    let view = FullScreenView::new(area, &title, colors);
    view.render_frame(frame);

    let content = view.content_area();
    let mut row = 0;

    // Header
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "   #   Name            Ch   Vol  Pan   Notes  [M][S]",
            Style::default().fg(colors.grey()),
        )],
    );
    row += 1;

    // Tracks
    let visible_height = content.height.saturating_sub(3) as usize;
    for (idx, track) in state
        .tracks
        .iter()
        .enumerate()
        .skip(state.track_scroll)
        .take(visible_height)
    {
        let selected = idx == state.current_track;
        let marker = if selected { ">" } else { " " };
        let mute = if track.muted { "M" } else { " " };
        let solo = if track.solo { "*" } else { " " };

        let line = format!(
            " {} {:>3}  {:<14} {:>2}   {:>3}  {:>3}   {:>5}  [{}][{}]",
            marker,
            idx + 1,
            truncate(&track.name, 14),
            track.channel + 1,
            track.volume,
            pan_display(track.pan),
            track.notes.len(),
            mute,
            solo
        );

        let style = if selected {
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.fg())
        };

        view.render_row(frame, row, vec![Span::styled(line, style)]);
        row += 1;
    }

    view.render_footer(
        frame,
        vec![Span::styled(
            format!(" Tracks: {}", state.tracks.len()),
            Style::default().fg(colors.green()),
        )],
    );

    view.render_help(
        frame,
        vec![
            ("N", "new"),
            ("Del", "delete"),
            ("M", "mute"),
            ("S", "solo"),
            ("Tab", "piano"),
            ("Esc", "exit"),
        ],
    );
}

/// Draw MIDI devices view
fn draw_midi_devices(frame: &mut Frame, area: Rect, state: &QMidiState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Q-MIDI: MIDI Devices ", colors);
    view.render_frame(frame);

    let content = view.content_area();
    let mut row = 0;

    // Output devices
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            " Output Devices:",
            Style::default().fg(colors.blue()),
        )],
    );
    row += 1;

    if state.available_outputs.is_empty() {
        // Show helpful message when no devices found
        for help_line in crate::midi_io::no_devices_help() {
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!("   {}", help_line),
                    Style::default().fg(colors.grey()),
                )],
            );
            row += 1;
        }
    } else {
        let visible_height = (content.height as usize).saturating_sub(6);
        for (idx, port) in state
            .available_outputs
            .iter()
            .enumerate()
            .take(visible_height)
        {
            let selected = idx == state.device_selected;
            let current = state.output_port.as_ref() == Some(port);
            let marker = if selected { ">" } else { " " };
            let check = if current { "*" } else { " " };

            let line = format!(" {} {} {}", marker, check, truncate(port, 50));

            let style = if selected {
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD)
            } else if current {
                Style::default().fg(colors.green())
            } else {
                Style::default().fg(colors.fg())
            };

            view.render_row(frame, row, vec![Span::styled(line, style)]);
            row += 1;
        }
    }

    row += 1;

    // Input devices
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            " Input Devices:",
            Style::default().fg(colors.blue()),
        )],
    );
    row += 1;

    if state.available_inputs.is_empty() {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "   (No MIDI inputs found)",
                Style::default().fg(colors.grey()),
            )],
        );
    } else {
        for port in state.available_inputs.iter().take(3) {
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!("   {}", truncate(port, 50)),
                    Style::default().fg(colors.fg()),
                )],
            );
            row += 1;
        }
    }

    // Current selection
    let current = state.output_port.as_deref().unwrap_or("None");
    view.render_footer(
        frame,
        vec![Span::styled(
            format!(" Current Output: {}", current),
            Style::default().fg(colors.green()),
        )],
    );

    view.render_help(
        frame,
        vec![("Enter", "select"), ("R", "refresh"), ("Esc", "back")],
    );
}

/// Draw file menu
fn draw_file_menu(frame: &mut Frame, area: Rect, state: &QMidiState, colors: &ThemeColors) {
    // Centered modal for file menu
    let width = area.width.min(50);
    let height = area.height.min(12);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let modal_area = Rect::new(x, y, width, height);

    let modal = ModalFrame::themed(modal_area, " File ", colors);
    modal.render_frame(frame);

    // File actions
    for (idx, action) in FileAction::all().iter().enumerate() {
        let selected = *action == state.file_action;
        let marker = if selected { ">" } else { " " };

        let style = if selected {
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.fg())
        };

        modal.render_row(
            frame,
            idx as u16,
            vec![Span::styled(
                format!(" {} {}", marker, action.name()),
                style,
            )],
        );
    }

    // Input field for Save As / Open
    if state.file_action == FileAction::SaveAs || state.file_action == FileAction::Open {
        let label = if state.file_action == FileAction::Open {
            "File:"
        } else {
            "Save as:"
        };
        modal.render_row(
            frame,
            5,
            vec![
                Span::styled(format!(" {}", label), Style::default().fg(colors.blue())),
                Span::styled(&state.file_input, Style::default().fg(colors.fg())),
            ],
        );
    }

    modal.render_help(frame, vec![("Enter", "confirm"), ("Esc", "cancel")]);
}

/// Draw help screen
fn draw_help(frame: &mut Frame, area: Rect, state: &QMidiState, colors: &ThemeColors) {
    let _ = state; // Unused but kept for consistency
    let view = FullScreenView::new(area, " Q-MIDI: Help ", colors);
    view.render_frame(frame);

    let help_text = vec![
        "",
        "  Q-MIDI - MIDI Sequencer",
        "",
        "  PLAYBACK:",
        "    Space      Play/Stop",
        "    R          Toggle Record",
        "    [          Set loop start",
        "    ]          Set loop end",
        "    L          Toggle loop",
        "",
        "  NAVIGATION:",
        "    Arrows     Move cursor",
        "    PgUp/PgDn  Scroll octave",
        "    Home/End   Go to start/end",
        "    +/-        Zoom in/out",
        "",
        "  EDITING:",
        "    Enter      Insert note",
        "    Delete     Delete note(s)",
        "    M          Mute track",
        "    S          Solo track",
        "",
        "  VIEWS:",
        "    Tab        Cycle views",
        "    T          Track list",
        "    D          MIDI devices",
        "    F1         This help",
        "    Esc        Exit",
    ];

    for (idx, line) in help_text.iter().enumerate() {
        if idx >= view.content_area().height as usize {
            break;
        }
        view.render_row(
            frame,
            idx as u16,
            vec![Span::styled(*line, Style::default().fg(colors.fg()))],
        );
    }

    view.render_help(frame, vec![("Esc", "back")]);
}

// =============================================================================
// HELPERS
// =============================================================================

/// Truncate string to max length
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}

/// Display pan value as L/C/R
fn pan_display(pan: u8) -> String {
    if pan < 60 {
        format!("L{}", 64 - pan)
    } else if pan > 68 {
        format!("R{}", pan - 64)
    } else {
        "C".to_string()
    }
}
