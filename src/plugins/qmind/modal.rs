//! Q-MIND plugin modal rendering
//!
//! UI components for the AI Intelligence Layer.
//! Uses FullScreenView for full-screen modal display.

use super::state::{QMindState, QMindView};
use crate::app::ThemeColors;
use crate::ui::components::FullScreenView;
use ratatui::{layout::Rect, style::Style, text::Span, Frame};

/// Draw the Q-MIND modal
pub fn draw_qmind_modal(
    frame: &mut Frame,
    area: Rect,
    state: &QMindState,
    loading: bool,
    colors: &ThemeColors,
) {
    // Show loading screen if still loading
    if loading {
        draw_loading(frame, area, colors);
        return;
    }

    match state.view {
        QMindView::Overview => draw_overview(frame, area, state, colors),
        QMindView::CommandPalette => draw_command_palette(frame, area, state, colors),
        QMindView::SemanticSearch => draw_semantic_search(frame, area, state, colors),
        QMindView::IndexStatus => draw_index_status(frame, area, state, colors),
        QMindView::FileSummary => draw_file_summary(frame, area, state, colors),
    }
}

/// Draw loading screen
fn draw_loading(frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Q-MIND ", colors);
    view.render_frame(frame);

    view.render_row(
        frame,
        5,
        vec![Span::styled(
            "Initializing Q-MIND...",
            Style::default().fg(colors.yellow()).bg(colors.bg()),
        )],
    );

    view.render_row(
        frame,
        7,
        vec![Span::styled(
            "Checking API availability",
            Style::default().fg(colors.grey()).bg(colors.bg()),
        )],
    );
}

/// Draw the overview showing Q-MIND features
fn draw_overview(frame: &mut Frame, area: Rect, state: &QMindState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Q-MIND Intelligence Layer ", colors);
    view.render_frame(frame);

    let mut row = 0;

    // API status
    let api_text = if state.api_available {
        "API Ready"
    } else {
        "No API Key (set OPENAI_API_KEY or ANTHROPIC_API_KEY)"
    };
    let api_color = if state.api_available {
        colors.green()
    } else {
        colors.red()
    };
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            api_text,
            Style::default().fg(api_color).bg(colors.bg()),
        )],
    );
    row += 2;

    // Features header
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "Features:",
            Style::default().fg(colors.green()).bg(colors.bg()),
        )],
    );
    row += 1;

    // Feature list
    view.render_row(
        frame,
        row,
        vec![
            Span::styled("  C ", Style::default().fg(colors.yellow()).bg(colors.bg())),
            Span::styled(
                "Command palette (natural language)",
                Style::default().fg(colors.fg()).bg(colors.bg()),
            ),
        ],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![
            Span::styled("  S ", Style::default().fg(colors.yellow()).bg(colors.bg())),
            Span::styled(
                "Semantic search",
                Style::default().fg(colors.fg()).bg(colors.bg()),
            ),
        ],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![
            Span::styled("  I ", Style::default().fg(colors.yellow()).bg(colors.bg())),
            Span::styled(
                "Index status",
                Style::default().fg(colors.fg()).bg(colors.bg()),
            ),
        ],
    );
    row += 2;

    // Show indexed file count if any
    if state.indexed_count > 0 {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("Indexed: {} files", state.indexed_count),
                Style::default().fg(colors.cyan()).bg(colors.bg()),
            )],
        );
        row += 2;
    }

    // Show error if any
    if let Some(ref error) = state.error {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("Error: {}", error),
                Style::default().fg(colors.red()).bg(colors.bg()),
            )],
        );
        row += 2;
    }

    // Examples
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "Examples:",
            Style::default().fg(colors.grey()).bg(colors.bg()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "  'copy *.txt to backup'",
            Style::default().fg(colors.grey()).bg(colors.bg()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "  'find that config file for rust'",
            Style::default().fg(colors.grey()).bg(colors.bg()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "  'delete old log files'",
            Style::default().fg(colors.grey()).bg(colors.bg()),
        )],
    );

    view.render_help(frame, vec![("C", "command"), ("S", "search"), ("Esc", "close")]);
}

/// Draw natural language command palette
fn draw_command_palette(frame: &mut Frame, area: Rect, state: &QMindState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Q-MIND Command ", colors);
    view.render_frame(frame);

    let input = &state.command_input;
    let mut row = 0;

    // Prompt
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "Enter a natural language command:",
            Style::default().fg(colors.green()).bg(colors.bg()),
        )],
    );
    row += 2;

    // Input field with cursor
    let input_display = if input.is_empty() {
        "Type your command... (e.g., 'copy *.txt to backup')".to_string()
    } else {
        // Show input with cursor
        let mut display = input.input.clone();
        if input.cursor <= display.len() {
            display.insert(input.cursor, '|');
        }
        display
    };

    let input_style = if input.is_empty() {
        Style::default().fg(colors.grey()).bg(colors.bg())
    } else {
        Style::default().fg(colors.yellow()).bg(colors.bg())
    };

    view.render_row(
        frame,
        row,
        vec![Span::styled(format!("> {}", input_display), input_style)],
    );
    row += 2;

    // Status/help text
    if !state.api_available {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "API not configured - set OPENAI_API_KEY or ANTHROPIC_API_KEY",
                Style::default().fg(colors.red()).bg(colors.bg()),
            )],
        );
    } else if input.is_empty() {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "Press Enter to parse command",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            )],
        );
    }

    // Show error if any
    if let Some(ref error) = state.error {
        row += 1;
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("Error: {}", error),
                Style::default().fg(colors.red()).bg(colors.bg()),
            )],
        );
    }

    view.render_help(frame, vec![("Enter", "parse"), ("Esc", "back")]);
}

/// Draw semantic search view
fn draw_semantic_search(frame: &mut Frame, area: Rect, state: &QMindState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Semantic Search ", colors);
    view.render_frame(frame);

    let input = &state.search_input;
    let mut row = 0;

    // Prompt
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "Search files by meaning:",
            Style::default().fg(colors.green()).bg(colors.bg()),
        )],
    );
    row += 2;

    // Input field with cursor
    let input_display = if input.is_empty() {
        "Describe what you're looking for...".to_string()
    } else {
        // Show input with cursor
        let mut display = input.input.clone();
        if input.cursor <= display.len() {
            display.insert(input.cursor, '|');
        }
        display
    };

    let input_style = if input.is_empty() {
        Style::default().fg(colors.grey()).bg(colors.bg())
    } else {
        Style::default().fg(colors.yellow()).bg(colors.bg())
    };

    view.render_row(
        frame,
        row,
        vec![Span::styled(format!("> {}", input_display), input_style)],
    );
    row += 2;

    // Show search results if any
    if !state.search_results.is_empty() {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("Results ({}):", state.search_results.len()),
                Style::default().fg(colors.green()).bg(colors.bg()),
            )],
        );
        row += 1;

        for (i, result) in state.search_results.iter().take(10).enumerate() {
            let is_selected = i == state.search_selected;
            let prefix = if is_selected { ">" } else { " " };
            let style = if is_selected {
                Style::default().fg(colors.yellow()).bg(colors.red())
            } else {
                Style::default().fg(colors.fg()).bg(colors.bg())
            };

            let path_str = result.path.to_string_lossy();
            let score_str = format!("{:.0}%", result.score * 100.0);

            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!("{} {:5} {}", prefix, score_str, path_str),
                    style,
                )],
            );
            row += 1;
        }
    } else if !input.is_empty() && state.api_available {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "Press Enter to search",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            )],
        );
    } else if !state.api_available {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "API not configured",
                Style::default().fg(colors.red()).bg(colors.bg()),
            )],
        );
    }

    view.render_help(frame, vec![("Enter", "search"), ("Esc", "back")]);
}

/// Draw index status view
fn draw_index_status(frame: &mut Frame, area: Rect, state: &QMindState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Index Status ", colors);
    view.render_frame(frame);

    let mut row = 0;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "Embedding Index Status",
            Style::default().fg(colors.green()).bg(colors.bg()),
        )],
    );
    row += 2;

    // API Status
    let api_status = if state.api_available {
        ("Ready", colors.green())
    } else {
        ("No API Key", colors.red())
    };

    view.render_row(
        frame,
        row,
        vec![
            Span::styled(
                "  API Status:     ",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            ),
            Span::styled(
                api_status.0,
                Style::default().fg(api_status.1).bg(colors.bg()),
            ),
        ],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![
            Span::styled(
                "  Indexed Files:  ",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            ),
            Span::styled(
                format!("{}", state.indexed_count),
                Style::default().fg(colors.fg()).bg(colors.bg()),
            ),
        ],
    );
    row += 2;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "Press R to rebuild index",
            Style::default().fg(colors.grey()).bg(colors.bg()),
        )],
    );

    view.render_help(frame, vec![("R", "rebuild"), ("Esc", "back")]);
}

/// Draw file summary view
fn draw_file_summary(frame: &mut Frame, area: Rect, state: &QMindState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " File Summary ", colors);
    view.render_frame(frame);

    let mut row = 0;

    if let Some(ref summary) = state.current_summary {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "AI-Generated Summary",
                Style::default().fg(colors.green()).bg(colors.bg()),
            )],
        );
        row += 2;

        view.render_row(
            frame,
            row,
            vec![Span::styled(
                summary.as_str(),
                Style::default().fg(colors.fg()).bg(colors.bg()),
            )],
        );
    } else {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "No file selected",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            )],
        );
        row += 2;

        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "Select a file to generate an AI summary",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            )],
        );
    }

    view.render_help(frame, vec![("Esc", "back")]);
}
