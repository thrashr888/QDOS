//! AI Assistant plugin modal rendering
//!
//! UI components for displaying AI CLI tool status.

use super::state::{AIMenuItem, AIState, AIView};
use crate::app::ThemeColors;
use crate::ui::components::ModalFrame;
use ratatui::{layout::Rect, style::Style, text::Span, Frame};

/// Draw the AI Assistant modal
pub fn draw_ai_modal(frame: &mut Frame, area: Rect, state: &AIState, colors: &ThemeColors) {
    match state.view {
        AIView::Overview => draw_overview(frame, area, state, colors),
        AIView::Claude => draw_claude_view(frame, area, state, colors),
        AIView::Codex => draw_codex_view(frame, area, state, colors),
        AIView::Gemini => draw_gemini_view(frame, area, state, colors),
    }
}

/// Draw the overview showing all providers
fn draw_overview(frame: &mut Frame, area: Rect, state: &AIState, colors: &ThemeColors) {
    let modal = ModalFrame::themed(area, " AI Assistants ", colors);
    modal.render_frame(frame);

    // Menu items
    for (i, item) in AIMenuItem::ALL.iter().enumerate() {
        let is_selected = i == state.menu_index;
        let prefix = if is_selected { ">" } else { " " };
        let style = if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            Style::default().fg(colors.fg()).bg(colors.bg())
        };

        modal.render_row(
            frame,
            i as u16,
            vec![Span::styled(
                format!("{} {}  {}", prefix, item.key(), item.label()),
                style,
            )],
        );
    }

    // Separator
    modal.render_row(frame, 5, vec![]);

    // Provider status summary
    let claude_status = if state.claude.available {
        if let Some(ref today) = state.claude.today {
            format!("{} msgs today", today.message_count)
        } else {
            "Ready".to_string()
        }
    } else {
        "Not installed".to_string()
    };

    let codex_status = if state.codex.available {
        state
            .codex
            .model
            .clone()
            .unwrap_or_else(|| "Ready".to_string())
    } else {
        "Not installed".to_string()
    };

    let gemini_status = if state.gemini.available {
        state
            .gemini
            .auth_type
            .clone()
            .unwrap_or_else(|| "Ready".to_string())
    } else {
        "Not installed".to_string()
    };

    modal.render_row(
        frame,
        6,
        vec![Span::styled(
            format!("Claude: {}", if state.claude.available { "✓" } else { "✗" }),
            Style::default()
                .fg(if state.claude.available {
                    colors.green()
                } else {
                    colors.grey()
                })
                .bg(colors.bg()),
        )],
    );
    modal.render_row(
        frame,
        7,
        vec![Span::styled(
            format!("  {}", claude_status),
            Style::default().fg(colors.fg()).bg(colors.bg()),
        )],
    );

    modal.render_row(
        frame,
        9,
        vec![Span::styled(
            format!("Codex: {}", if state.codex.available { "✓" } else { "✗" }),
            Style::default()
                .fg(if state.codex.available {
                    colors.green()
                } else {
                    colors.grey()
                })
                .bg(colors.bg()),
        )],
    );
    modal.render_row(
        frame,
        10,
        vec![Span::styled(
            format!("  {}", codex_status),
            Style::default().fg(colors.fg()).bg(colors.bg()),
        )],
    );

    modal.render_row(
        frame,
        12,
        vec![Span::styled(
            format!("Gemini: {}", if state.gemini.available { "✓" } else { "✗" }),
            Style::default()
                .fg(if state.gemini.available {
                    colors.green()
                } else {
                    colors.grey()
                })
                .bg(colors.bg()),
        )],
    );
    modal.render_row(
        frame,
        13,
        vec![Span::styled(
            format!("  {}", gemini_status),
            Style::default().fg(colors.fg()).bg(colors.bg()),
        )],
    );

    // Help footer
    modal.render_help(
        frame,
        vec![
            ("↑↓", "select"),
            ("Enter", "view"),
            ("R", "refresh"),
            ("Esc", "close"),
        ],
    );
}

/// Draw Claude Code detailed view
fn draw_claude_view(frame: &mut Frame, area: Rect, state: &AIState, colors: &ThemeColors) {
    let modal = ModalFrame::themed(area, " Claude Code Status ", colors);
    modal.render_frame(frame);

    if !state.claude.available {
        modal.render_row(
            frame,
            1,
            vec![Span::styled(
                "Claude Code not installed",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            )],
        );
        modal.render_row(
            frame,
            3,
            vec![Span::styled(
                "Install: npm install -g @anthropic-ai/claude-code",
                Style::default().fg(colors.blue()).bg(colors.bg()),
            )],
        );
        modal.render_help(frame, vec![("Esc", "back")]);
        return;
    }

    let mut row = 0;

    // Today's stats
    if let Some(ref today) = state.claude.today {
        modal.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("Today ({})", today.date),
                Style::default().fg(colors.green()).bg(colors.bg()),
            )],
        );
        row += 1;
        modal.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("  Messages:   {:>6}", today.message_count),
                Style::default().fg(colors.fg()).bg(colors.bg()),
            )],
        );
        row += 1;
        modal.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("  Sessions:   {:>6}", today.session_count),
                Style::default().fg(colors.fg()).bg(colors.bg()),
            )],
        );
        row += 1;
        modal.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("  Tool calls: {:>6}", today.tool_call_count),
                Style::default().fg(colors.fg()).bg(colors.bg()),
            )],
        );
        row += 2;
    }

    // Recent activity
    if !state.claude.recent_days.is_empty() {
        modal.render_row(
            frame,
            row,
            vec![Span::styled(
                "Recent Activity",
                Style::default().fg(colors.green()).bg(colors.bg()),
            )],
        );
        row += 1;

        // Header
        modal.render_row(
            frame,
            row,
            vec![Span::styled(
                "  Date        Msgs   Sessions  Tools",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            )],
        );
        row += 1;

        for day in state.claude.recent_days.iter().take(5) {
            modal.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!(
                        "  {} {:>6}   {:>6}   {:>6}",
                        day.date, day.message_count, day.session_count, day.tool_call_count
                    ),
                    Style::default().fg(colors.fg()).bg(colors.bg()),
                )],
            );
            row += 1;
        }
    }

    // Last computed
    if let Some(ref date) = state.claude.last_computed {
        row += 1;
        modal.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("Stats computed: {}", date),
                Style::default().fg(colors.grey()).bg(colors.bg()),
            )],
        );
    }

    modal.render_help(frame, vec![("R", "refresh"), ("Esc", "back")]);
}

/// Draw Codex detailed view
fn draw_codex_view(frame: &mut Frame, area: Rect, state: &AIState, colors: &ThemeColors) {
    let modal = ModalFrame::themed(area, " OpenAI Codex Status ", colors);
    modal.render_frame(frame);

    if !state.codex.available {
        modal.render_row(
            frame,
            1,
            vec![Span::styled(
                "OpenAI Codex CLI not installed",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            )],
        );
        modal.render_row(
            frame,
            3,
            vec![Span::styled(
                "Install: npm install -g @openai/codex",
                Style::default().fg(colors.blue()).bg(colors.bg()),
            )],
        );
        modal.render_help(frame, vec![("Esc", "back")]);
        return;
    }

    let mut row = 0;

    // Model info
    modal.render_row(
        frame,
        row,
        vec![Span::styled(
            "Configuration",
            Style::default().fg(colors.green()).bg(colors.bg()),
        )],
    );
    row += 1;

    if let Some(ref model) = state.codex.model {
        modal.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("  Model: {}", model),
                Style::default().fg(colors.fg()).bg(colors.bg()),
            )],
        );
        row += 1;
    }

    if let Some(ref effort) = state.codex.reasoning_effort {
        modal.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("  Reasoning: {}", effort),
                Style::default().fg(colors.fg()).bg(colors.bg()),
            )],
        );
        row += 1;
    }

    row += 1;

    // Version info
    if let Some(ref version) = state.codex.latest_version {
        modal.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("Version: {}", version),
                Style::default().fg(colors.fg()).bg(colors.bg()),
            )],
        );
        row += 1;
    }

    if let Some(ref checked) = state.codex.last_checked {
        modal.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("Last checked: {}", checked),
                Style::default().fg(colors.grey()).bg(colors.bg()),
            )],
        );
        row += 1;
    }

    row += 1;

    // Trusted projects
    if !state.codex.trusted_projects.is_empty() {
        modal.render_row(
            frame,
            row,
            vec![Span::styled(
                "Trusted Projects",
                Style::default().fg(colors.green()).bg(colors.bg()),
            )],
        );
        row += 1;

        for project in &state.codex.trusted_projects {
            modal.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!("  {}", project),
                    Style::default().fg(colors.fg()).bg(colors.bg()),
                )],
            );
            row += 1;
        }
    }

    modal.render_help(frame, vec![("R", "refresh"), ("Esc", "back")]);
}

/// Draw Gemini detailed view
fn draw_gemini_view(frame: &mut Frame, area: Rect, state: &AIState, colors: &ThemeColors) {
    let modal = ModalFrame::themed(area, " Gemini CLI Status ", colors);
    modal.render_frame(frame);

    if !state.gemini.available {
        modal.render_row(
            frame,
            1,
            vec![Span::styled(
                "Gemini CLI not installed",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            )],
        );
        modal.render_row(
            frame,
            3,
            vec![Span::styled(
                "Install: npm install -g @anthropic-ai/gemini-cli",
                Style::default().fg(colors.blue()).bg(colors.bg()),
            )],
        );
        modal.render_help(frame, vec![("Esc", "back")]);
        return;
    }

    let mut row = 0;

    // Authentication
    modal.render_row(
        frame,
        row,
        vec![Span::styled(
            "Authentication",
            Style::default().fg(colors.green()).bg(colors.bg()),
        )],
    );
    row += 1;

    if let Some(ref auth) = state.gemini.auth_type {
        modal.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("  Type: {}", auth),
                Style::default().fg(colors.fg()).bg(colors.bg()),
            )],
        );
        row += 1;
    }

    row += 1;

    // Settings
    modal.render_row(
        frame,
        row,
        vec![Span::styled(
            "Settings",
            Style::default().fg(colors.green()).bg(colors.bg()),
        )],
    );
    row += 1;

    if let Some(ref editor) = state.gemini.preferred_editor {
        modal.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("  Editor: {}", editor),
                Style::default().fg(colors.fg()).bg(colors.bg()),
            )],
        );
        row += 1;
    }

    if let Some(ref theme) = state.gemini.theme {
        modal.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("  Theme: {}", theme),
                Style::default().fg(colors.fg()).bg(colors.bg()),
            )],
        );
        row += 1;
    }

    modal.render_row(
        frame,
        row,
        vec![Span::styled(
            format!(
                "  Preview features: {}",
                if state.gemini.preview_features {
                    "enabled"
                } else {
                    "disabled"
                }
            ),
            Style::default().fg(colors.fg()).bg(colors.bg()),
        )],
    );

    modal.render_help(frame, vec![("R", "refresh"), ("Esc", "back")]);
}
