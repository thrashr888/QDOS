//! Jj Plugin Modal Rendering
//!
//! UI rendering for the jj (Jujutsu) VCS plugin modal.

use super::state::{GitAction, JjMenuItem, JjState, JjView};
use crate::app::ThemeColors;
use crate::ui::components::FullScreenView;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Draw the jj modal
pub fn draw_jj_modal(frame: &mut Frame, area: Rect, state: &JjState, colors: &ThemeColors) {
    let title = match state.view {
        JjView::Menu => " JJ - JUJUTSU VCS ",
        JjView::Status => " JJ STATUS ",
        JjView::Log => " JJ LOG ",
        JjView::Diff => " JJ DIFF ",
        JjView::Describe => " JJ DESCRIBE ",
        JjView::Bookmark => " JJ BOOKMARKS ",
        JjView::Operations => " JJ OPERATIONS ",
        JjView::Git => " JJ GIT ",
    };

    let view = FullScreenView::new(area, title, colors);
    view.render_frame(frame);
    let content_area = view.content_area();

    if !state.is_repo {
        render_not_repo(frame, content_area, colors);
        return;
    }

    if let Some(ref error) = state.error {
        render_error(frame, content_area, error, colors);
        return;
    }

    match state.view {
        JjView::Menu => render_menu(frame, content_area, state, colors),
        JjView::Status => render_status(frame, content_area, state, colors),
        JjView::Log => render_log(frame, content_area, state, colors),
        JjView::Diff => render_diff(frame, content_area, state, colors),
        JjView::Describe => render_describe(frame, content_area, state, colors),
        JjView::Bookmark => render_bookmark(frame, content_area, state, colors),
        JjView::Operations => render_operations(frame, content_area, state, colors),
        JjView::Git => render_git(frame, content_area, state, colors),
    }

    // Render help hints
    let help = get_help_hints(state);
    view.render_help(frame, help);
}

fn render_not_repo(frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Not a jj repository",
            Style::default().fg(colors.red()),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Run 'jj git init' or 'jj init' to create one",
            Style::default().fg(colors.grey()),
        )),
    ];
    frame.render_widget(Paragraph::new(lines), area);
}

fn render_error(frame: &mut Frame, area: Rect, error: &str, colors: &ThemeColors) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  Error: {}", error),
            Style::default().fg(colors.red()),
        )),
    ];
    frame.render_widget(Paragraph::new(lines), area);
}

fn render_menu(frame: &mut Frame, area: Rect, state: &JjState, colors: &ThemeColors) {
    let mut lines = vec![Line::from("")];

    for (i, item) in JjMenuItem::ALL.iter().enumerate() {
        let is_selected = i == state.menu_selected;
        let style = if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            Style::default().fg(colors.blue())
        };

        let desc_style = if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            Style::default().fg(colors.grey())
        };

        lines.push(Line::from(vec![
            Span::styled(format!("  {:<12}", item.as_str()), style),
            Span::styled(format!(" - {}", item.description()), desc_style),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_status(frame: &mut Frame, area: Rect, state: &JjState, colors: &ThemeColors) {
    let mut lines = vec![Line::from("")];

    // Working copy info
    if let Some(ref wc) = state.working_copy {
        let marker = if wc.is_working_copy { "@" } else { " " };
        let empty_marker = if wc.is_empty { "(empty) " } else { "" };
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {} {} ", marker, wc.change_id),
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{} ", wc.commit_id),
                Style::default().fg(colors.blue()),
            ),
            Span::styled(empty_marker, Style::default().fg(colors.grey())),
            Span::styled(&wc.description, Style::default().fg(colors.fg())),
        ]));
    }

    // Parent info
    if let Some(ref parent) = state.parent {
        let empty_marker = if parent.is_empty { "(empty) " } else { "" };
        lines.push(Line::from(vec![
            Span::styled(
                format!("    {} ", parent.change_id),
                Style::default().fg(colors.green()),
            ),
            Span::styled(
                format!("{} ", parent.commit_id),
                Style::default().fg(colors.blue()),
            ),
            Span::styled(empty_marker, Style::default().fg(colors.grey())),
            Span::styled(&parent.description, Style::default().fg(colors.grey())),
        ]));
    }

    lines.push(Line::from(""));

    // File changes
    if state.files.is_empty() {
        lines.push(Line::from(Span::styled(
            "  The working copy has no changes.",
            Style::default().fg(colors.grey()),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "  Changed files:",
            Style::default().fg(colors.blue()),
        )));
        for file in &state.files {
            let status_color = match file.status {
                'A' => colors.green(),
                'D' => colors.red(),
                _ => colors.yellow(),
            };
            lines.push(Line::from(vec![
                Span::styled(
                    format!("    {} ", file.status),
                    Style::default().fg(status_color),
                ),
                Span::styled(&file.path, Style::default().fg(colors.fg())),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_log(frame: &mut Frame, area: Rect, state: &JjState, colors: &ThemeColors) {
    let visible_height = area.height.saturating_sub(2) as usize;
    let mut lines = vec![Line::from("")];

    if state.changes.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No changes found",
            Style::default().fg(colors.grey()),
        )));
    } else {
        for (i, change) in state
            .changes
            .iter()
            .skip(state.scroll_offset)
            .take(visible_height)
            .enumerate()
        {
            let idx = i + state.scroll_offset;
            let is_selected = idx == state.selected_change;

            let base_style = if is_selected {
                Style::default().fg(colors.yellow()).bg(colors.red())
            } else {
                Style::default()
            };

            let marker = if change.is_working_copy { "@" } else { " " };
            let empty_marker = if change.is_empty { "(empty) " } else { "" };

            let change_style = if is_selected {
                base_style
            } else if change.is_working_copy {
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.green())
            };

            let commit_style = if is_selected {
                base_style
            } else {
                Style::default().fg(colors.blue())
            };

            let desc_style = if is_selected {
                base_style
            } else {
                Style::default().fg(colors.fg())
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  {} {} ", marker, change.change_id), change_style),
                Span::styled(format!("{} ", change.commit_id), commit_style),
                Span::styled(empty_marker, base_style.fg(colors.grey())),
                Span::styled(&change.description, desc_style),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_diff(frame: &mut Frame, area: Rect, state: &JjState, colors: &ThemeColors) {
    let visible_height = area.height.saturating_sub(2) as usize;
    let mut lines = vec![Line::from("")];

    if state.diff_content.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No changes to display",
            Style::default().fg(colors.grey()),
        )));
    } else {
        for line in state
            .diff_content
            .iter()
            .skip(state.scroll_offset)
            .take(visible_height)
        {
            let style = if line.starts_with('+') && !line.starts_with("+++") {
                Style::default().fg(colors.green())
            } else if line.starts_with('-') && !line.starts_with("---") {
                Style::default().fg(colors.red())
            } else if line.starts_with("@@") {
                Style::default().fg(colors.cyan())
            } else if line.starts_with("diff ") || line.starts_with("index ") {
                Style::default().fg(colors.blue())
            } else {
                Style::default().fg(colors.fg())
            };
            lines.push(Line::from(Span::styled(format!("  {}", line), style)));
        }
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_describe(frame: &mut Frame, area: Rect, state: &JjState, colors: &ThemeColors) {
    let mut lines = vec![Line::from("")];

    if let Some(ref wc) = state.working_copy {
        lines.push(Line::from(vec![
            Span::styled("  Change: ", Style::default().fg(colors.blue())),
            Span::styled(&wc.change_id, Style::default().fg(colors.yellow())),
        ]));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "  Description:",
        Style::default().fg(colors.blue()),
    )));

    if state.input_mode {
        lines.push(Line::from(vec![
            Span::styled("  > ", Style::default().fg(colors.green())),
            Span::styled(
                &state.description_input,
                Style::default().fg(colors.yellow()),
            ),
            Span::styled("█", Style::default().fg(colors.yellow())),
        ]));
    } else {
        let desc = if let Some(ref wc) = state.working_copy {
            &wc.description
        } else {
            "(no description set)"
        };
        lines.push(Line::from(Span::styled(
            format!("  {}", desc),
            Style::default().fg(colors.fg()),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Press Enter to edit",
            Style::default().fg(colors.grey()),
        )));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_bookmark(frame: &mut Frame, area: Rect, state: &JjState, colors: &ThemeColors) {
    let visible_height = area.height.saturating_sub(4) as usize;
    let mut lines = vec![Line::from("")];

    if state.bookmark_input_mode {
        lines.push(Line::from(vec![
            Span::styled("  New bookmark: ", Style::default().fg(colors.blue())),
            Span::styled(&state.bookmark_input, Style::default().fg(colors.yellow())),
            Span::styled("█", Style::default().fg(colors.yellow())),
        ]));
        lines.push(Line::from(""));
    }

    if state.bookmarks.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No bookmarks found",
            Style::default().fg(colors.grey()),
        )));
    } else {
        for (i, bookmark) in state.bookmarks.iter().take(visible_height).enumerate() {
            let is_selected = i == state.selected_bookmark;
            let style = if is_selected {
                Style::default().fg(colors.yellow()).bg(colors.red())
            } else {
                Style::default().fg(colors.fg())
            };

            let name_style = if is_selected {
                style
            } else if bookmark.is_remote {
                Style::default().fg(colors.grey())
            } else {
                Style::default().fg(colors.green())
            };

            let conflict_marker = if bookmark.is_conflicted {
                " (conflicted)"
            } else {
                ""
            };
            let remote_marker = if let Some(ref remote) = bookmark.remote {
                format!("@{}", remote)
            } else {
                String::new()
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  {}{}", bookmark.name, remote_marker), name_style),
                Span::styled(format!(" -> {}", bookmark.target), style),
                Span::styled(conflict_marker, Style::default().fg(colors.red())),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_operations(frame: &mut Frame, area: Rect, state: &JjState, colors: &ThemeColors) {
    let visible_height = area.height.saturating_sub(2) as usize;
    let mut lines = vec![Line::from("")];

    if state.operations.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No operations found",
            Style::default().fg(colors.grey()),
        )));
    } else {
        for (i, op) in state.operations.iter().take(visible_height).enumerate() {
            let is_selected = i == state.selected_operation;
            let style = if is_selected {
                Style::default().fg(colors.yellow()).bg(colors.red())
            } else {
                Style::default().fg(colors.fg())
            };

            let marker = if op.is_current { "@" } else { " " };
            let id_style = if is_selected {
                style
            } else if op.is_current {
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.blue())
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  {} {} ", marker, op.id), id_style),
                Span::styled(format!("{} ", op.time), style),
                Span::styled(&op.description, style),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_git(frame: &mut Frame, area: Rect, state: &JjState, colors: &ThemeColors) {
    let mut lines = vec![Line::from("")];

    lines.push(Line::from(Span::styled(
        "  Git Remote Operations",
        Style::default()
            .fg(colors.blue())
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    let fetch_style = if state.git_action == GitAction::Fetch {
        Style::default().fg(colors.yellow()).bg(colors.red())
    } else {
        Style::default().fg(colors.fg())
    };

    let push_style = if state.git_action == GitAction::Push {
        Style::default().fg(colors.yellow()).bg(colors.red())
    } else {
        Style::default().fg(colors.fg())
    };

    lines.push(Line::from(vec![
        Span::styled("  [F] ", Style::default().fg(colors.green())),
        Span::styled("Fetch from remote", fetch_style),
    ]));

    lines.push(Line::from(vec![
        Span::styled("  [P] ", Style::default().fg(colors.green())),
        Span::styled("Push to remote", push_style),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Press Enter to execute selected action",
        Style::default().fg(colors.grey()),
    )));

    frame.render_widget(Paragraph::new(lines), area);
}

fn get_help_hints(state: &JjState) -> Vec<(&'static str, &'static str)> {
    match state.view {
        JjView::Menu => vec![("↑↓", "select"), ("Enter", "open"), ("ESC", "close")],
        JjView::Status => vec![("d", "diff"), ("ESC", "back")],
        JjView::Log => vec![("↑↓", "select"), ("Enter", "diff"), ("ESC", "back")],
        JjView::Diff => vec![("↑↓", "scroll"), ("ESC", "back")],
        JjView::Describe => {
            if state.input_mode {
                vec![("Enter", "save"), ("ESC", "cancel")]
            } else {
                vec![("Enter", "edit"), ("ESC", "back")]
            }
        }
        JjView::Bookmark => {
            if state.bookmark_input_mode {
                vec![("Enter", "create"), ("ESC", "cancel")]
            } else {
                vec![
                    ("↑↓", "select"),
                    ("n", "new"),
                    ("d", "delete"),
                    ("ESC", "back"),
                ]
            }
        }
        JjView::Operations => vec![("↑↓", "select"), ("u", "undo"), ("ESC", "back")],
        JjView::Git => vec![("↑↓", "select"), ("Enter", "execute"), ("ESC", "back")],
    }
}
