//! AI Assistant plugin modal rendering
//!
//! UI components for displaying AI CLI tool status.
//! Uses FullScreenView for full-screen modal display.

use super::state::{AIMenuItem, AIState, AIView};
use crate::app::ThemeColors;
use crate::ui::components::FullScreenView;
use ratatui::{layout::Rect, style::Style, text::Span, Frame};

/// Draw the AI Assistant modal
pub fn draw_ai_modal(
    frame: &mut Frame,
    area: Rect,
    state: &AIState,
    loading: bool,
    colors: &ThemeColors,
) {
    // Show loading screen if still loading
    if loading {
        draw_loading(frame, area, colors);
        return;
    }

    match state.view {
        AIView::Overview => draw_overview(frame, area, state, colors),
        AIView::Claude => draw_claude_view(frame, area, state, colors),
        AIView::Codex => draw_codex_view(frame, area, state, colors),
        AIView::Gemini => draw_gemini_view(frame, area, state, colors),
        AIView::Cursor => draw_cursor_view(frame, area, state, colors),
        AIView::Copilot => draw_copilot_view(frame, area, state, colors),
    }
}

/// Draw loading screen
fn draw_loading(frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " AI Assistants ", colors);
    view.render_frame(frame);

    view.render_row(
        frame,
        5,
        vec![Span::styled(
            "Loading AI assistant data...",
            Style::default().fg(colors.yellow()).bg(colors.bg()),
        )],
    );

    view.render_row(
        frame,
        7,
        vec![Span::styled(
            "Reading session files from:",
            Style::default().fg(colors.grey()).bg(colors.bg()),
        )],
    );
    view.render_row(
        frame,
        8,
        vec![Span::styled(
            "  ~/.claude/projects/",
            Style::default().fg(colors.fg()).bg(colors.bg()),
        )],
    );
    view.render_row(
        frame,
        9,
        vec![Span::styled(
            "  ~/.codex/sessions/",
            Style::default().fg(colors.fg()).bg(colors.bg()),
        )],
    );
}

/// Draw the overview showing all providers
fn draw_overview(frame: &mut Frame, area: Rect, state: &AIState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " AI Assistants ", colors);
    view.render_frame(frame);

    // Menu items
    for (i, item) in AIMenuItem::ALL.iter().enumerate() {
        let is_selected = i == state.menu_index;
        let prefix = if is_selected { ">" } else { " " };
        let style = if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            Style::default().fg(colors.fg()).bg(colors.bg())
        };

        view.render_row(
            frame,
            i as u16,
            vec![Span::styled(
                format!("{} {}  {}", prefix, item.key(), item.label()),
                style,
            )],
        );
    }

    // Separator
    view.render_row(frame, 5, vec![]);

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

    view.render_row(
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
    view.render_row(
        frame,
        7,
        vec![Span::styled(
            format!("  {}", claude_status),
            Style::default().fg(colors.fg()).bg(colors.bg()),
        )],
    );

    view.render_row(
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
    view.render_row(
        frame,
        10,
        vec![Span::styled(
            format!("  {}", codex_status),
            Style::default().fg(colors.fg()).bg(colors.bg()),
        )],
    );

    view.render_row(
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
    view.render_row(
        frame,
        13,
        vec![Span::styled(
            format!("  {}", gemini_status),
            Style::default().fg(colors.fg()).bg(colors.bg()),
        )],
    );

    // Cursor status
    let cursor_status = if state.cursor.available {
        if state.cursor.code_generations > 0 {
            format!("{} code generations", state.cursor.code_generations)
        } else {
            "Ready".to_string()
        }
    } else {
        "Not installed".to_string()
    };

    view.render_row(
        frame,
        15,
        vec![Span::styled(
            format!("Cursor: {}", if state.cursor.available { "✓" } else { "✗" }),
            Style::default()
                .fg(if state.cursor.available {
                    colors.green()
                } else {
                    colors.grey()
                })
                .bg(colors.bg()),
        )],
    );
    view.render_row(
        frame,
        16,
        vec![Span::styled(
            format!("  {}", cursor_status),
            Style::default().fg(colors.fg()).bg(colors.bg()),
        )],
    );

    // Copilot status
    let copilot_status = if state.copilot.available {
        state
            .copilot
            .github_user
            .as_ref()
            .map(|u| format!("@{}", u))
            .unwrap_or_else(|| "Authenticated".to_string())
    } else {
        "Not installed".to_string()
    };

    view.render_row(
        frame,
        18,
        vec![Span::styled(
            format!(
                "Copilot: {}",
                if state.copilot.available {
                    "✓"
                } else {
                    "✗"
                }
            ),
            Style::default()
                .fg(if state.copilot.available {
                    colors.green()
                } else {
                    colors.grey()
                })
                .bg(colors.bg()),
        )],
    );
    view.render_row(
        frame,
        19,
        vec![Span::styled(
            format!("  {}", copilot_status),
            Style::default().fg(colors.fg()).bg(colors.bg()),
        )],
    );

    // Help footer
    view.render_help(
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
    let view = FullScreenView::new(area, " Claude Code Status ", colors);
    view.render_frame(frame);

    if !state.claude.available {
        view.render_row(
            frame,
            1,
            vec![Span::styled(
                "Claude Code not installed",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            )],
        );
        view.render_row(
            frame,
            3,
            vec![Span::styled(
                "Install: npm install -g @anthropic-ai/claude-code",
                Style::default().fg(colors.blue()).bg(colors.bg()),
            )],
        );
        view.render_help(frame, vec![("Esc", "back")]);
        return;
    }

    let mut row = 0;

    // Today's stats
    if let Some(ref today) = state.claude.today {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("Today ({})", today.date),
                Style::default().fg(colors.green()).bg(colors.bg()),
            )],
        );
        row += 1;
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("  Messages:   {:>6}", today.message_count),
                Style::default().fg(colors.fg()).bg(colors.bg()),
            )],
        );
        row += 1;
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("  Sessions:   {:>6}", today.session_count),
                Style::default().fg(colors.fg()).bg(colors.bg()),
            )],
        );
        row += 1;
        view.render_row(
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
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "Recent Activity",
                Style::default().fg(colors.green()).bg(colors.bg()),
            )],
        );
        row += 1;

        // Header
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "  Date        Msgs   Sessions  Tools",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            )],
        );
        row += 1;

        for day in state.claude.recent_days.iter().take(5) {
            view.render_row(
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

    // Token usage and costs (from session JSONL files)
    let usage = &state.claude.token_usage;
    if usage.input_tokens > 0 || usage.output_tokens > 0 {
        row += 1;
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("Token Usage ({} sessions)", state.claude.session_count),
                Style::default().fg(colors.green()).bg(colors.bg()),
            )],
        );
        row += 1;

        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("  Input:         {:>12}", format_tokens(usage.input_tokens)),
                Style::default().fg(colors.fg()).bg(colors.bg()),
            )],
        );
        row += 1;

        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!(
                    "  Output:        {:>12}",
                    format_tokens(usage.output_tokens)
                ),
                Style::default().fg(colors.fg()).bg(colors.bg()),
            )],
        );
        row += 1;

        if usage.cache_read_tokens > 0 {
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!(
                        "  Cache read:    {:>12}",
                        format_tokens(usage.cache_read_tokens)
                    ),
                    Style::default().fg(colors.fg()).bg(colors.bg()),
                )],
            );
            row += 1;
        }

        if usage.cache_creation_tokens > 0 {
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!(
                        "  Cache write:   {:>12}",
                        format_tokens(usage.cache_creation_tokens)
                    ),
                    Style::default().fg(colors.fg()).bg(colors.bg()),
                )],
            );
            row += 1;
        }

        row += 1;
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("  Est. cost:      ${:.2}", usage.total_cost_usd),
                Style::default()
                    .fg(if usage.total_cost_usd > 10.0 {
                        colors.red()
                    } else if usage.total_cost_usd > 1.0 {
                        colors.yellow()
                    } else {
                        colors.green()
                    })
                    .bg(colors.bg()),
            )],
        );
        row += 1;
    }

    // Last computed
    if let Some(ref date) = state.claude.last_computed {
        row += 1;
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("Stats computed: {}", date),
                Style::default().fg(colors.grey()).bg(colors.bg()),
            )],
        );
    }

    view.render_help(frame, vec![("R", "refresh"), ("Esc", "back")]);
}

/// Format token count with K/M suffixes
fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

/// Draw Codex detailed view
fn draw_codex_view(frame: &mut Frame, area: Rect, state: &AIState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " OpenAI Codex Status ", colors);
    view.render_frame(frame);

    if !state.codex.available {
        view.render_row(
            frame,
            1,
            vec![Span::styled(
                "OpenAI Codex CLI not installed",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            )],
        );
        view.render_row(
            frame,
            3,
            vec![Span::styled(
                "Install: npm install -g @openai/codex",
                Style::default().fg(colors.blue()).bg(colors.bg()),
            )],
        );
        view.render_help(frame, vec![("Esc", "back")]);
        return;
    }

    let mut row = 0;

    // Model info
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "Configuration",
            Style::default().fg(colors.green()).bg(colors.bg()),
        )],
    );
    row += 1;

    if let Some(ref model) = state.codex.model {
        view.render_row(
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
        view.render_row(
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
        view.render_row(
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
        view.render_row(
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
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "Trusted Projects",
                Style::default().fg(colors.green()).bg(colors.bg()),
            )],
        );
        row += 1;

        for project in &state.codex.trusted_projects {
            view.render_row(
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

    // Token usage and costs (from session JSONL files)
    let usage = &state.codex.token_usage;
    if usage.input_tokens > 0 || usage.output_tokens > 0 {
        row += 1;
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("Token Usage ({} sessions)", state.codex.session_count),
                Style::default().fg(colors.green()).bg(colors.bg()),
            )],
        );
        row += 1;

        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("  Input:         {:>12}", format_tokens(usage.input_tokens)),
                Style::default().fg(colors.fg()).bg(colors.bg()),
            )],
        );
        row += 1;

        if usage.cached_input_tokens > 0 {
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!(
                        "  Cached:        {:>12}",
                        format_tokens(usage.cached_input_tokens)
                    ),
                    Style::default().fg(colors.fg()).bg(colors.bg()),
                )],
            );
            row += 1;
        }

        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!(
                    "  Output:        {:>12}",
                    format_tokens(usage.output_tokens)
                ),
                Style::default().fg(colors.fg()).bg(colors.bg()),
            )],
        );
        row += 1;

        if usage.reasoning_tokens > 0 {
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!(
                        "  Reasoning:     {:>12}",
                        format_tokens(usage.reasoning_tokens)
                    ),
                    Style::default().fg(colors.fg()).bg(colors.bg()),
                )],
            );
            row += 1;
        }

        row += 1;
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("  Est. cost:      ${:.2}", usage.total_cost_usd),
                Style::default()
                    .fg(if usage.total_cost_usd > 10.0 {
                        colors.red()
                    } else if usage.total_cost_usd > 1.0 {
                        colors.yellow()
                    } else {
                        colors.green()
                    })
                    .bg(colors.bg()),
            )],
        );
    }

    view.render_help(frame, vec![("R", "refresh"), ("Esc", "back")]);
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

/// Draw Cursor detailed view
fn draw_cursor_view(frame: &mut Frame, area: Rect, state: &AIState, colors: &ThemeColors) {
    let modal = ModalFrame::themed(area, " Cursor Status ", colors);
    modal.render_frame(frame);

    if !state.cursor.available {
        modal.render_row(
            frame,
            1,
            vec![Span::styled(
                "Cursor IDE not installed",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            )],
        );
        modal.render_row(
            frame,
            3,
            vec![Span::styled(
                "Install: https://cursor.com",
                Style::default().fg(colors.blue()).bg(colors.bg()),
            )],
        );
        modal.render_help(frame, vec![("Esc", "back")]);
        return;
    }

    let mut row = 0;

    // Configuration
    modal.render_row(
        frame,
        row,
        vec![Span::styled(
            "Configuration",
            Style::default().fg(colors.green()).bg(colors.bg()),
        )],
    );
    row += 1;

    if let Some(ref model) = state.cursor.model {
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

    modal.render_row(
        frame,
        row,
        vec![Span::styled(
            format!(
                "  Vim mode: {}",
                if state.cursor.vim_mode {
                    "enabled"
                } else {
                    "disabled"
                }
            ),
            Style::default().fg(colors.fg()).bg(colors.bg()),
        )],
    );
    row += 2;

    // AI tracking stats
    modal.render_row(
        frame,
        row,
        vec![Span::styled(
            "AI Code Generation Stats",
            Style::default().fg(colors.green()).bg(colors.bg()),
        )],
    );
    row += 1;

    modal.render_row(
        frame,
        row,
        vec![Span::styled(
            format!("  Total generations: {}", state.cursor.code_generations),
            Style::default().fg(colors.fg()).bg(colors.bg()),
        )],
    );
    row += 2;

    // Breakdown by source
    if !state.cursor.generations_by_source.is_empty() {
        modal.render_row(
            frame,
            row,
            vec![Span::styled(
                "By Source",
                Style::default().fg(colors.green()).bg(colors.bg()),
            )],
        );
        row += 1;

        for (source, count) in state.cursor.generations_by_source.iter().take(6) {
            modal.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!("  {:12} {:>8}", source, count),
                    Style::default().fg(colors.fg()).bg(colors.bg()),
                )],
            );
            row += 1;
        }
    }

    modal.render_help(frame, vec![("R", "refresh"), ("Esc", "back")]);
}

/// Draw Copilot detailed view
fn draw_copilot_view(frame: &mut Frame, area: Rect, state: &AIState, colors: &ThemeColors) {
    let modal = ModalFrame::themed(area, " GitHub Copilot Status ", colors);
    modal.render_frame(frame);

    if !state.copilot.available {
        modal.render_row(
            frame,
            1,
            vec![Span::styled(
                "GitHub Copilot not configured",
                Style::default().fg(colors.grey()).bg(colors.bg()),
            )],
        );
        modal.render_row(
            frame,
            3,
            vec![Span::styled(
                "Authenticate: gh auth login",
                Style::default().fg(colors.blue()).bg(colors.bg()),
            )],
        );
        modal.render_help(frame, vec![("Esc", "back")]);
        return;
    }

    let mut row = 0;

    // Authentication status
    modal.render_row(
        frame,
        row,
        vec![Span::styled(
            "Authentication",
            Style::default().fg(colors.green()).bg(colors.bg()),
        )],
    );
    row += 1;

    modal.render_row(
        frame,
        row,
        vec![Span::styled(
            format!(
                "  GitHub CLI: {}",
                if state.copilot.gh_authenticated {
                    "authenticated"
                } else {
                    "not authenticated"
                }
            ),
            Style::default().fg(colors.fg()).bg(colors.bg()),
        )],
    );
    row += 1;

    if let Some(ref user) = state.copilot.github_user {
        modal.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("  User: @{}", user),
                Style::default().fg(colors.fg()).bg(colors.bg()),
            )],
        );
        row += 1;
    }

    row += 1;

    // Info
    modal.render_row(
        frame,
        row,
        vec![Span::styled(
            "Note: Copilot usage data requires API access.",
            Style::default().fg(colors.grey()).bg(colors.bg()),
        )],
    );
    row += 1;
    modal.render_row(
        frame,
        row,
        vec![Span::styled(
            "Only authentication status is shown locally.",
            Style::default().fg(colors.grey()).bg(colors.bg()),
        )],
    );

    modal.render_help(frame, vec![("R", "refresh"), ("Esc", "back")]);
}
