//! Shell plugin modal rendering
//!
//! Rendering functions for shell/DOS command views.

use super::state::{BackgroundTask, ShellState, TaskStatus};
use crate::app::ThemeColors;
use crate::ui::components::ModalFrame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::Clear;
use ratatui::Frame;
use std::collections::HashMap;
use std::path::PathBuf;

/// Draw the command input view
pub fn draw_command_view(
    frame: &mut Frame,
    area: Rect,
    state: &ShellState,
    current_cwd: &PathBuf,
    tasks: &HashMap<u64, BackgroundTask>,
    colors: &ThemeColors,
) {
    frame.render_widget(Clear, area);

    let modal = ModalFrame::themed(area, " DOS Command ", colors);
    modal.render_frame(frame);

    let content_height = modal.content_height() as usize;

    // Draw prompt and input
    let prompt_style = Style::default().fg(colors.green()).bg(colors.bg());
    let input_style = Style::default().fg(colors.fg()).bg(colors.bg());

    let prompt = format!("{}> ", current_cwd.display());
    let prompt_len = prompt.len().min(area.width.saturating_sub(4) as usize);

    modal.render_row(
        frame,
        0,
        vec![
            Span::styled(&prompt[..prompt_len], prompt_style),
            Span::styled(&state.input, input_style),
            Span::styled(
                "_",
                Style::default()
                    .fg(colors.cyan())
                    .bg(colors.bg())
                    .add_modifier(Modifier::SLOW_BLINK),
            ),
        ],
    );

    // Draw output
    let output_start = 2;
    let output_height = content_height.saturating_sub(3);
    let scroll = state
        .scroll_offset
        .min(state.output.len().saturating_sub(output_height));

    for (i, line) in state
        .output
        .iter()
        .skip(scroll)
        .take(output_height)
        .enumerate()
    {
        let style = if line.starts_with("stderr:") {
            Style::default().fg(colors.yellow()).bg(colors.bg())
        } else {
            Style::default().fg(colors.fg()).bg(colors.bg())
        };
        modal.render_row(
            frame,
            (output_start + i) as u16,
            vec![Span::styled(line, style)],
        );
    }

    // Draw status line
    let status_row = content_height.saturating_sub(1) as u16;
    let exit_str = match state.exit_code {
        Some(0) => "Exit: 0 (OK)".to_string(),
        Some(code) => format!("Exit: {}", code),
        None => String::new(),
    };

    let running_count = tasks.values().filter(|t| t.status == TaskStatus::Running).count();
    let tasks_str = if running_count > 0 {
        format!(" | {} bg", running_count)
    } else {
        String::new()
    };

    modal.render_row(
        frame,
        status_row,
        vec![
            Span::styled(
                &exit_str,
                Style::default().fg(colors.grey()).bg(colors.bg()),
            ),
            Span::styled(
                &tasks_str,
                Style::default().fg(colors.cyan()).bg(colors.bg()),
            ),
            Span::styled(
                " | jobs  fg <id>  kill <id>  cmd&",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            ),
        ],
    );
}

/// Draw the task list view
pub fn draw_task_list_view(
    frame: &mut Frame,
    area: Rect,
    state: &ShellState,
    tasks: &HashMap<u64, BackgroundTask>,
    colors: &ThemeColors,
) {
    frame.render_widget(Clear, area);

    let running = tasks.values().filter(|t| t.status == TaskStatus::Running).count();
    let total = tasks.len();
    let title = format!(" Task List ({} running / {} total) ", running, total);
    let modal = ModalFrame::themed(area, &title, colors);
    modal.render_frame(frame);

    let content_height = modal.content_height() as usize;

    if tasks.is_empty() {
        modal.render_row(
            frame,
            1,
            vec![Span::styled(
                "No background tasks",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            )],
        );
    } else {
        // Header
        modal.render_row(
            frame,
            0,
            vec![
                Span::styled("  ID  ", Style::default().fg(colors.cyan()).bg(colors.bg())),
                Span::styled(
                    "Status   ",
                    Style::default().fg(colors.cyan()).bg(colors.bg()),
                ),
                Span::styled(
                    "Time     ",
                    Style::default().fg(colors.cyan()).bg(colors.bg()),
                ),
                Span::styled(
                    "Command",
                    Style::default().fg(colors.cyan()).bg(colors.bg()),
                ),
            ],
        );

        let task_ids = get_sorted_task_ids(tasks);
        let max_display = content_height.saturating_sub(3);

        for (i, id) in task_ids.iter().take(max_display).enumerate() {
            if let Some(task) = tasks.get(id) {
                let is_selected = i == state.selected_task;
                let bg = if is_selected {
                    colors.red()
                } else {
                    colors.bg()
                };
                let fg = if is_selected {
                    colors.yellow()
                } else {
                    colors.fg()
                };

                let status_style = match task.status {
                    TaskStatus::Running => Style::default().fg(colors.green()).bg(bg),
                    TaskStatus::Completed => Style::default().fg(colors.cyan()).bg(bg),
                    TaskStatus::Failed => Style::default().fg(colors.red()).bg(bg),
                };

                let status_icon = match task.status {
                    TaskStatus::Running => "● ",
                    TaskStatus::Completed => "✓ ",
                    TaskStatus::Failed => "✗ ",
                };

                let elapsed = task.elapsed().as_secs_f32();
                let time_str = if elapsed < 60.0 {
                    format!("{:>5.1}s  ", elapsed)
                } else {
                    format!("{:>5.1}m  ", elapsed / 60.0)
                };

                let cmd_width = area.width.saturating_sub(30) as usize;
                let cmd_display = if task.command.len() > cmd_width {
                    format!("{}...", &task.command[..cmd_width.saturating_sub(3)])
                } else {
                    task.command.clone()
                };

                modal.render_row(
                    frame,
                    (i + 1) as u16,
                    vec![
                        Span::styled(format!(" {:>3}  ", id), Style::default().fg(fg).bg(bg)),
                        Span::styled(status_icon, status_style),
                        Span::styled(format!("{:<6} ", task.status.as_str()), status_style),
                        Span::styled(&time_str, Style::default().fg(colors.grey()).bg(bg)),
                        Span::styled(cmd_display, Style::default().fg(fg).bg(bg)),
                    ],
                );
            }
        }
    }

    // Help line
    let status_row = content_height.saturating_sub(1) as u16;
    modal.render_row(
        frame,
        status_row,
        vec![
            Span::styled("Enter", Style::default().fg(colors.green()).bg(colors.bg())),
            Span::styled(
                ":attach  ",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            ),
            Span::styled("D", Style::default().fg(colors.green()).bg(colors.bg())),
            Span::styled(
                ":kill  ",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            ),
            Span::styled("C", Style::default().fg(colors.green()).bg(colors.bg())),
            Span::styled(
                ":clear  ",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            ),
            Span::styled("Esc", Style::default().fg(colors.green()).bg(colors.bg())),
            Span::styled(":back", Style::default().fg(colors.grey()).bg(colors.bg())),
        ],
    );
}

/// Draw the attached to task view
pub fn draw_attached_view(
    frame: &mut Frame,
    area: Rect,
    task_id: u64,
    state: &ShellState,
    tasks: &HashMap<u64, BackgroundTask>,
    colors: &ThemeColors,
) {
    frame.render_widget(Clear, area);

    let task = match tasks.get(&task_id) {
        Some(t) => t,
        None => {
            // Task was removed, go back to command view
            return;
        }
    };

    let status_str = match task.status {
        TaskStatus::Running => "RUNNING",
        TaskStatus::Completed => "DONE",
        TaskStatus::Failed => "FAILED",
    };
    let title = format!(" Task {} - {} - {} ", task_id, status_str, task.command);
    let title_truncated = if title.len() > area.width.saturating_sub(4) as usize {
        format!("{}...", &title[..area.width.saturating_sub(7) as usize])
    } else {
        title
    };

    let modal = ModalFrame::themed(area, &title_truncated, colors);
    modal.render_frame(frame);

    let content_height = modal.content_height() as usize;
    let output = task.output.lock().unwrap();
    let output_len = output.len();

    // Calculate scroll position
    let visible_height = content_height.saturating_sub(2);
    let scroll = state
        .scroll_offset
        .min(output_len.saturating_sub(visible_height));

    // Draw output lines
    for (i, line) in output.iter().skip(scroll).take(visible_height).enumerate() {
        let style = if line.starts_with("stderr:") {
            Style::default().fg(colors.yellow()).bg(colors.bg())
        } else if line == "[Killed]" {
            Style::default().fg(colors.red()).bg(colors.bg())
        } else {
            Style::default().fg(colors.fg()).bg(colors.bg())
        };
        modal.render_row(frame, i as u16, vec![Span::styled(line, style)]);
    }

    // Status line
    let status_row = content_height.saturating_sub(1) as u16;
    let elapsed = task.elapsed().as_secs_f32();
    let time_str = if elapsed < 60.0 {
        format!("{:.1}s", elapsed)
    } else {
        format!("{:.1}m", elapsed / 60.0)
    };

    let exit_str = match task.exit_code {
        Some(code) => format!("Exit: {}", code),
        None => String::new(),
    };

    let scroll_str = format!(" [{}/{}]", scroll + 1, output_len.max(1));

    modal.render_row(
        frame,
        status_row,
        vec![
            Span::styled(
                &time_str,
                Style::default().fg(colors.grey()).bg(colors.bg()),
            ),
            Span::styled("  ", Style::default().bg(colors.bg())),
            Span::styled(
                &exit_str,
                Style::default().fg(colors.grey()).bg(colors.bg()),
            ),
            Span::styled(
                &scroll_str,
                Style::default().fg(colors.grey()).bg(colors.bg()),
            ),
            Span::styled(" | ", Style::default().fg(colors.grey()).bg(colors.bg())),
            Span::styled("↑↓", Style::default().fg(colors.green()).bg(colors.bg())),
            Span::styled(
                ":scroll  ",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            ),
            Span::styled("F", Style::default().fg(colors.green()).bg(colors.bg())),
            Span::styled(
                ":follow  ",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            ),
            Span::styled("q", Style::default().fg(colors.green()).bg(colors.bg())),
            Span::styled(":back", Style::default().fg(colors.grey()).bg(colors.bg())),
        ],
    );
}

/// Get sorted task IDs for display
fn get_sorted_task_ids(tasks: &HashMap<u64, BackgroundTask>) -> Vec<u64> {
    let mut ids: Vec<_> = tasks.keys().copied().collect();
    ids.sort();
    ids
}
