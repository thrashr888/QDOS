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
        QMindView::DryRun => draw_dry_run(frame, area, state, colors),
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
    row += 1;

    view.render_row(
        frame,
        row,
        vec![
            Span::styled("  F ", Style::default().fg(colors.yellow()).bg(colors.bg())),
            Span::styled(
                "File summary (AI)",
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

    view.render_help(frame, vec![("C", "command"), ("S", "search"), ("F", "summary"), ("Esc", "close")]);
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

    // Show parsed command if available
    if let Some(ref cmd) = state.last_parsed_command {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("Action: {} ({}% confidence)", cmd.action.description(), (cmd.confidence * 100.0) as u32),
                Style::default().fg(colors.green()).bg(colors.bg()),
            )],
        );
        row += 1;

        if !cmd.targets.is_empty() {
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!("Targets: {}", cmd.targets.join(", ")),
                    Style::default().fg(colors.cyan()).bg(colors.bg()),
                )],
            );
            row += 1;
        }

        if let Some(ref dest) = cmd.destination {
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!("Destination: {}", dest),
                    Style::default().fg(colors.cyan()).bg(colors.bg()),
                )],
            );
            row += 1;
        }

        if let Some(ref pattern) = cmd.pattern {
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!("Pattern: {}", pattern),
                    Style::default().fg(colors.cyan()).bg(colors.bg()),
                )],
            );
            row += 1;
        }

        if !cmd.explanation.is_empty() {
            row += 1;
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    &cmd.explanation,
                    Style::default().fg(colors.fg()).bg(colors.bg()),
                )],
            );
        }

        // Show found files if any
        if !state.found_files.is_empty() {
            row += 2;
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!("Found {} files:", state.found_files.len()),
                    Style::default().fg(colors.green()).bg(colors.bg()),
                )],
            );
            row += 1;

            // Show up to 10 files
            for file in state.found_files.iter().take(10) {
                let name = file.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| file.to_string_lossy().to_string());
                let size = std::fs::metadata(file)
                    .map(|m| format!("{} bytes", m.len()))
                    .unwrap_or_default();
                view.render_row(
                    frame,
                    row,
                    vec![Span::styled(
                        format!("  {} {}", name, size),
                        Style::default().fg(colors.cyan()).bg(colors.bg()),
                    )],
                );
                row += 1;
            }

            if state.found_files.len() > 10 {
                view.render_row(
                    frame,
                    row,
                    vec![Span::styled(
                        format!("  ... and {} more", state.found_files.len() - 10),
                        Style::default().fg(colors.grey()).bg(colors.bg()),
                    )],
                );
            }
        }
    } else if !state.api_available {
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
                "Press Enter to parse command (Shift+Enter for newline)",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            )],
        );
    } else {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "Press Enter to parse...",
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

    view.render_help(frame, vec![("Enter", "parse"), ("Shift+Enter", "newline"), ("Esc", "back")]);
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

    view.render_help(frame, vec![("Enter", "search"), ("Shift+Enter", "newline"), ("Esc", "back")]);
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

    if let Some(ref summary) = state.file_summary {
        // Header
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "AI-Generated Summary",
                Style::default().fg(colors.green()).bg(colors.bg()),
            )],
        );
        row += 2;

        // File type
        if !summary.file_type.is_empty() {
            view.render_row(
                frame,
                row,
                vec![
                    Span::styled(
                        "Type: ",
                        Style::default().fg(colors.grey()).bg(colors.bg()),
                    ),
                    Span::styled(
                        &summary.file_type,
                        Style::default().fg(colors.cyan()).bg(colors.bg()),
                    ),
                ],
            );
            row += 1;
        }

        // Brief summary
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                &summary.brief,
                Style::default().fg(colors.yellow()).bg(colors.bg()),
            )],
        );
        row += 2;

        // Detailed summary
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "Description:",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            )],
        );
        row += 1;

        // Word-wrap detailed summary (simple split by ~70 chars)
        for line in wrap_text(&summary.detailed, 70) {
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!("  {}", line),
                    Style::default().fg(colors.fg()).bg(colors.bg()),
                )],
            );
            row += 1;
        }
        row += 1;

        // Key elements
        if !summary.key_elements.is_empty() {
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    "Key Elements:",
                    Style::default().fg(colors.grey()).bg(colors.bg()),
                )],
            );
            row += 1;

            for element in summary.key_elements.iter().take(10) {
                view.render_row(
                    frame,
                    row,
                    vec![Span::styled(
                        format!("  - {}", element),
                        Style::default().fg(colors.cyan()).bg(colors.bg()),
                    )],
                );
                row += 1;
            }

            if summary.key_elements.len() > 10 {
                view.render_row(
                    frame,
                    row,
                    vec![Span::styled(
                        format!("  ... and {} more", summary.key_elements.len() - 10),
                        Style::default().fg(colors.grey()).bg(colors.bg()),
                    )],
                );
                row += 1;
            }
            row += 1;
        }

        // Token usage
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("Tokens used: {}", summary.tokens_used),
                Style::default().fg(colors.grey()).bg(colors.bg()),
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
                "Select a file in the file browser, then press F to summarize",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            )],
        );
    }

    view.render_help(frame, vec![("Esc", "back")]);
}

/// Simple text wrapping helper
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= max_width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

/// Draw dry run confirmation view
fn draw_dry_run(frame: &mut Frame, area: Rect, state: &QMindState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Confirm Operation ", colors);
    view.render_frame(frame);

    let mut row = 0;

    if let Some(ref dry_run) = state.dry_run {
        // Source description
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                &dry_run.source,
                Style::default().fg(colors.green()).bg(colors.bg()),
            )],
        );
        row += 2;

        // Warning for destructive operations
        if dry_run.has_destructive() {
            let warning = if dry_run.has_deletions() {
                format!(
                    "WARNING: {} destructive operation(s), including deletions!",
                    dry_run.destructive_count()
                )
            } else {
                format!(
                    "WARNING: {} destructive operation(s)",
                    dry_run.destructive_count()
                )
            };
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    warning,
                    Style::default().fg(colors.red()).bg(colors.bg()),
                )],
            );
            row += 2;
        }

        // Operations list
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("Operations ({}):", dry_run.operations.len()),
                Style::default().fg(colors.fg()).bg(colors.bg()),
            )],
        );
        row += 1;

        // List operations with selection
        for (i, op) in dry_run.operations.iter().enumerate().take(15) {
            let is_selected = i == dry_run.selected;
            let prefix = if is_selected { ">" } else { " " };

            // Color based on operation type
            let op_color = if op.op_type.is_destructive() {
                colors.red()
            } else {
                colors.cyan()
            };

            let path_str = op.path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| op.path.to_string_lossy().to_string());

            let line = format!(
                "{} [{:6}] {} - {}",
                prefix,
                op.op_type.label(),
                path_str,
                op.description
            );

            let style = if is_selected {
                Style::default().fg(colors.yellow()).bg(colors.red())
            } else {
                Style::default().fg(op_color).bg(colors.bg())
            };

            view.render_row(frame, row, vec![Span::styled(line, style)]);
            row += 1;
        }

        if dry_run.operations.len() > 15 {
            row += 1;
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!("... and {} more operations", dry_run.operations.len() - 15),
                    Style::default().fg(colors.grey()).bg(colors.bg()),
                )],
            );
        }
    } else {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "No operation to confirm",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            )],
        );
    }

    // Help depends on whether destructive
    let has_destructive = state.dry_run.as_ref().map(|dr| dr.has_destructive()).unwrap_or(false);
    if has_destructive {
        view.render_help(frame, vec![("Y", "confirm"), ("N/Esc", "cancel")]);
    } else {
        view.render_help(frame, vec![("Y/Enter", "confirm"), ("N/Esc", "cancel")]);
    }
}
