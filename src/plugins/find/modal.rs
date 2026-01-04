//! Find modal drawing function

use crate::app::{App, FindPhase, FindState, SearchMode};
use crate::ui::components::FullScreenView;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Draw the Find modal
pub fn draw_find_modal(frame: &mut Frame, area: Rect, state: &FindState, app: &App) {
    let colors = app.colors();

    // Create full-screen view
    let view = FullScreenView::new(area, " FIND FILES ", &colors);
    view.render_frame(frame);

    // Content area
    let content_area = view.content_area();

    match state.phase {
        FindPhase::SelectMode => {
            let resolved_tool = state.search_tool.resolve();
            let tool_name = resolved_tool.name();
            let tool_status = if state.search_tool_available {
                format!("✓ {} available", tool_name)
            } else {
                format!("✗ {} not found", tool_name)
            };

            let mut lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "Select search mode:",
                    Style::default().fg(colors.green()),
                )),
                Line::from(""),
            ];

            // Option 1: By filename (default)
            let name_style = if state.search_mode == SearchMode::ByName {
                Style::default().fg(colors.yellow()).bg(colors.red())
            } else {
                Style::default().fg(colors.fg())
            };
            lines.push(Line::from(Span::styled(
                "  1. Search by filename (glob patterns)",
                name_style,
            )));

            // Option 2: By content (requires configured tool)
            if state.search_tool_available {
                let content_style = if state.search_mode == SearchMode::ByContent {
                    Style::default().fg(colors.yellow()).bg(colors.red())
                } else {
                    Style::default().fg(colors.fg())
                };
                lines.push(Line::from(Span::styled(
                    format!("  2. Search by content ({})", tool_name),
                    content_style,
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    format!("  2. Search by content (requires {})", tool_name),
                    Style::default().fg(colors.grey()),
                )));
            }

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                tool_status,
                Style::default().fg(if state.search_tool_available {
                    colors.green()
                } else {
                    colors.grey()
                }),
            )));

            frame.render_widget(Paragraph::new(lines), content_area);
        }
        FindPhase::InputPattern => {
            // Show different prompts based on search mode
            let resolved_tool = state.search_tool.resolve();
            let tool_name = resolved_tool.name();
            let (mode_label, examples) = match state.search_mode {
                SearchMode::ByName => (
                    "Find File (by name):".to_string(),
                    "Examples: *.txt, foo*.rs, config.*".to_string(),
                ),
                SearchMode::ByContent => (
                    format!("Find File (by content - {}):", tool_name),
                    "Examples: TODO, fn main, error".to_string(),
                ),
            };

            let mut lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    &mode_label,
                    Style::default().fg(colors.green()),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Pattern: ", Style::default().fg(colors.fg())),
                    Span::styled(
                        &state.pattern,
                        Style::default().fg(colors.yellow()).bg(colors.red()),
                    ),
                    Span::styled("█", Style::default().fg(colors.yellow()).bg(colors.red())),
                ]),
                Line::from(""),
                Line::from(Span::styled(&examples, Style::default().fg(colors.grey()))),
                Line::from(Span::styled(
                    "Ctrl+R to recall last pattern",
                    Style::default().fg(colors.grey()),
                )),
            ];
            if !state.last_pattern.is_empty() {
                lines.push(Line::from(Span::styled(
                    format!("Last pattern: {}", state.last_pattern),
                    Style::default().fg(colors.grey()),
                )));
            }
            frame.render_widget(Paragraph::new(lines), content_area);
        }
        FindPhase::AskPause => {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("Searching for: {}", state.pattern),
                    Style::default().fg(colors.green()),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Pause when a match is found?  (Y/N)",
                    Style::default().fg(colors.fg()),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Y = Stop at each match (can Jump/View/Continue)",
                    Style::default().fg(colors.grey()),
                )),
                Line::from(Span::styled(
                    "N = Show all matches at once",
                    Style::default().fg(colors.grey()),
                )),
            ];
            frame.render_widget(Paragraph::new(lines), content_area);
        }
        FindPhase::Searching => {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "Searching...",
                    Style::default().fg(colors.yellow()),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!("Pattern: {}", state.pattern),
                    Style::default().fg(colors.green()),
                )),
            ];
            frame.render_widget(Paragraph::new(lines), content_area);
        }
        FindPhase::ShowResult => {
            if let Some((path, display)) = state.matches.get(state.current_match) {
                let lines = vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        format!(
                            "Match {} of {}",
                            state.current_match + 1,
                            state.matches.len()
                        ),
                        Style::default().fg(colors.green()),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        display.clone(),
                        Style::default().fg(colors.yellow()),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("Path: {}", path.display()),
                        Style::default().fg(colors.grey()),
                    )),
                ];
                frame.render_widget(Paragraph::new(lines), content_area);
            }
        }
        FindPhase::ShowAllResults => {
            let visible_height = content_area.height as usize;
            let mut lines: Vec<Line> = vec![
                Line::from(Span::styled(
                    format!(
                        "Found {} matches for '{}':",
                        state.matches.len(),
                        state.pattern
                    ),
                    Style::default().fg(colors.green()),
                )),
                Line::from(""),
            ];

            for (i, (path, _)) in state
                .matches
                .iter()
                .enumerate()
                .skip(state.scroll_offset)
                .take(visible_height.saturating_sub(2))
            {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string());
                let parent = path
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                let line_text = format!("{:4}. {} - {}", i + 1, name, parent);
                // Highlight the selected item
                let style = if i == state.current_match {
                    Style::default().fg(colors.yellow()).bg(colors.red())
                } else {
                    Style::default().fg(colors.fg())
                };
                lines.push(Line::from(Span::styled(line_text, style)));
            }

            frame.render_widget(Paragraph::new(lines), content_area);
        }
        FindPhase::NoResults => {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("Pattern: {}", state.pattern),
                    Style::default().fg(colors.green()),
                )),
                Line::from(""),
                if state.search_complete && state.matches.is_empty() {
                    Line::from(Span::styled(
                        "No matching files found.",
                        Style::default().fg(colors.yellow()),
                    ))
                } else {
                    Line::from(Span::styled(
                        format!("Finished with FIND -- {} files found", state.matches.len()),
                        Style::default().fg(colors.yellow()),
                    ))
                },
                Line::from(""),
                Line::from(Span::styled(
                    "Press any key to continue",
                    Style::default().fg(colors.green()),
                )),
            ];
            frame.render_widget(Paragraph::new(lines), content_area);
        }
    }

    // Render help based on phase
    let help_hints: Vec<(&str, &str)> = match state.phase {
        FindPhase::SelectMode => vec![
            ("1/2/Tab", "select"),
            ("Enter", "continue"),
            ("ESC", "cancel"),
        ],
        FindPhase::InputPattern => {
            vec![("Enter", "search"), ("Ctrl+R", "recall"), ("ESC", "cancel")]
        }
        FindPhase::AskPause => vec![("Y", "pause"), ("N", "show all"), ("ESC", "cancel")],
        FindPhase::Searching => vec![("", "Searching...")],
        FindPhase::ShowResult => vec![
            ("C", "continue"),
            ("J", "jump"),
            ("V", "view"),
            ("ESC", "quit"),
        ],
        FindPhase::ShowAllResults => vec![
            ("↑↓", "select"),
            ("Enter/J", "jump"),
            ("V", "view"),
            ("ESC", "close"),
        ],
        FindPhase::NoResults => vec![("Any key", "continue")],
    };
    view.render_help(frame, help_hints);
}
