//! Emulator plugin modal rendering

use super::state::{EmulatorState, EmulatorView};
use crate::app::ThemeColors;
use crate::ui::components::FullScreenView;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    Frame,
};

/// Draw the emulator modal
pub fn draw_emulator_modal(
    frame: &mut Frame,
    area: Rect,
    state: &EmulatorState,
    colors: &ThemeColors,
) {
    let view = FullScreenView::new(area, " Emulator ", colors);
    view.render_frame(frame);

    match state.view {
        EmulatorView::Menu => draw_menu(frame, &view, state, colors),
        EmulatorView::FileSelect => draw_file_select(frame, &view, state, colors),
        EmulatorView::Running => draw_running(frame, &view, state, colors),
        EmulatorView::NotAvailable => draw_not_available(frame, &view, state, colors),
    }
}

fn draw_menu(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &EmulatorState,
    colors: &ThemeColors,
) {
    // Header
    view.render_row(
        frame,
        1,
        vec![Span::styled(
            "╔═══════════════════════════════════════════════════════════╗",
            Style::default().fg(colors.cyan()),
        )],
    );
    view.render_row(
        frame,
        2,
        vec![Span::styled(
            "║                      E M U L A T O R                      ║",
            Style::default().fg(colors.cyan()),
        )],
    );
    view.render_row(
        frame,
        3,
        vec![Span::styled(
            "╚═══════════════════════════════════════════════════════════╝",
            Style::default().fg(colors.cyan()),
        )],
    );

    // Emulator status
    let dosbox_status = if state.dosbox_available {
        Span::styled("● DOSBox-X", Style::default().fg(colors.green()))
    } else {
        Span::styled(
            "○ DOSBox-X (not installed)",
            Style::default().fg(colors.grey()),
        )
    };

    view.render_row(
        frame,
        5,
        vec![Span::styled(
            "Available Emulators:",
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        )],
    );
    view.render_row(frame, 7, vec![Span::raw("  "), dosbox_status]);

    // Description
    view.render_row(
        frame,
        9,
        vec![Span::styled(
            "    Run DOS executables (.EXE, .COM, .BAT)",
            Style::default().fg(colors.grey()),
        )],
    );

    // Current file info
    if let Some(ref path) = state.file_path {
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());

        view.render_row(
            frame,
            12,
            vec![
                Span::styled("Selected: ", Style::default().fg(colors.grey())),
                Span::styled(file_name, Style::default().fg(colors.yellow())),
            ],
        );

        if state.can_run(path).is_some() {
            view.render_row(
                frame,
                13,
                vec![Span::styled(
                    "Press Enter to run in DOSBox-X",
                    Style::default().fg(colors.green()),
                )],
            );
        } else {
            view.render_row(
                frame,
                13,
                vec![Span::styled(
                    "This file cannot be run in an emulator",
                    Style::default().fg(colors.red()),
                )],
            );
        }
    } else {
        view.render_row(
            frame,
            12,
            vec![Span::styled(
                "Select a .EXE, .COM, or .BAT file to run",
                Style::default().fg(colors.grey()),
            )],
        );
    }

    // Error display
    if let Some(ref error) = state.error {
        view.render_row(
            frame,
            15,
            vec![Span::styled(
                format!("Error: {}", error),
                Style::default().fg(colors.red()),
            )],
        );
    }

    // Installation help
    if !state.dosbox_available {
        view.render_row(
            frame,
            17,
            vec![Span::styled(
                "To install DOSBox-X:",
                Style::default().fg(colors.yellow()),
            )],
        );
        view.render_row(
            frame,
            18,
            vec![Span::styled(
                "  brew install dosbox-x",
                Style::default().fg(colors.cyan()),
            )],
        );
    }

    let help = if state.file_path.is_some() && state.dosbox_available {
        vec![("Enter", "run"), ("Esc", "close")]
    } else {
        vec![("Esc", "close")]
    };
    view.render_help(frame, help);
}

fn draw_file_select(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &EmulatorState,
    colors: &ThemeColors,
) {
    // Header
    view.render_row(
        frame,
        1,
        vec![Span::styled(
            "Select a DOS Executable",
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        )],
    );

    if state.entries.is_empty() {
        view.render_row(
            frame,
            3,
            vec![Span::styled(
                "No DOS executables found in this directory",
                Style::default().fg(colors.grey()),
            )],
        );
    } else {
        // File list
        let visible_height = view.content_height() as usize - 5;
        let start = state.scroll_offset;
        let end = (start + visible_height).min(state.entries.len());

        for (i, entry) in state.entries[start..end].iter().enumerate() {
            let is_selected = start + i == state.selected;

            let style = if is_selected {
                Style::default().fg(colors.yellow()).bg(colors.red())
            } else {
                Style::default().fg(colors.fg())
            };

            let emulator_style = if is_selected {
                Style::default().fg(colors.cyan()).bg(colors.red())
            } else {
                Style::default().fg(colors.cyan())
            };

            view.render_row(
                frame,
                (i + 3) as u16,
                vec![
                    Span::styled(if is_selected { "► " } else { "  " }, style),
                    Span::styled(format!("{:<30}", entry.name), style),
                    Span::styled(format!(" [{}]", entry.emulator.name()), emulator_style),
                ],
            );
        }
    }

    let help = vec![("↑↓", "select"), ("Enter", "run"), ("Esc", "back")];
    view.render_help(frame, help);
}

fn draw_running(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &EmulatorState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height();
    let center = content_height / 2;

    view.render_row(
        frame,
        center,
        vec![Span::styled(
            "Running in DOSBox-X...",
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );

    if let Some(ref path) = state.file_path {
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        view.render_row(
            frame,
            center + 2,
            vec![Span::styled(file_name, Style::default().fg(colors.cyan()))],
        );
    }

    view.render_row(
        frame,
        center + 4,
        vec![Span::styled(
            "Close DOSBox-X window to return",
            Style::default().fg(colors.grey()),
        )],
    );

    let help = vec![("", "waiting for DOSBox-X to close")];
    view.render_help(frame, help);
}

fn draw_not_available(
    frame: &mut Frame,
    view: &FullScreenView,
    _state: &EmulatorState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height();
    let center = content_height / 2;

    view.render_row(
        frame,
        center - 2,
        vec![Span::styled(
            "No Emulators Available",
            Style::default()
                .fg(colors.red())
                .add_modifier(Modifier::BOLD),
        )],
    );

    view.render_row(
        frame,
        center,
        vec![Span::styled(
            "Install DOSBox-X to run DOS executables:",
            Style::default().fg(colors.yellow()),
        )],
    );

    view.render_row(
        frame,
        center + 2,
        vec![Span::styled(
            "  brew install dosbox-x",
            Style::default().fg(colors.cyan()),
        )],
    );

    let help = vec![("Esc", "close")];
    view.render_help(frame, help);
}
