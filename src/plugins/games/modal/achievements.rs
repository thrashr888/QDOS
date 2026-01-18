//! Achievements UI Modal - Display achievement progress
//!
//! Shows all achievements organized by game with unlock status and progress.

use super::super::platform::achievements::{AchievementManager, AchievementToast};
use super::super::platform::stats::PlayerStats;
use super::super::state::GameType;
use qdos_plugin_api::ui::FullScreenView;
use qdos_plugin_api::ThemeColors;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::Frame;

/// Draw the achievements modal
pub fn draw_achievements(
    frame: &mut Frame,
    view: &FullScreenView,
    manager: &AchievementManager,
    stats: &PlayerStats,
    scroll_offset: usize,
    colors: &ThemeColors,
) {
    let mut row = 0;

    // Header with count
    let unlocked = manager.unlocked_count();
    let total = manager.total_count();
    view.render_row(
        frame,
        row,
        vec![
            Span::styled(
                "ACHIEVEMENTS",
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "                                      {} / {}",
                    unlocked, total
                ),
                Style::default().fg(colors.cyan()),
            ),
        ],
    );
    row += 1;

    // Separator
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "\u{2500}".repeat(76),
            Style::default().fg(colors.grey()),
        )],
    );
    row += 1;

    // Get achievements by game
    let by_game = manager.get_achievements_by_game();
    #[allow(clippy::type_complexity)]
    let mut all_rows: Vec<(
        Option<GameType>,
        &'static str,
        &'static str,
        char,
        bool,
        Option<String>,
    )> = Vec::new();

    for (game, achievements) in &by_game {
        // Add section header
        let game_name = game.map(|g| g.name()).unwrap_or("GLOBAL");
        all_rows.push((*game, game_name, "", ' ', false, None));

        for achievement in achievements {
            let is_unlocked = manager.is_unlocked(achievement.id);
            let progress = manager.get_progress(achievement, stats);

            // Handle hidden achievements
            let (name, desc, icon) = if achievement.hidden && !is_unlocked {
                ("???", "???", ' ')
            } else {
                (achievement.name, achievement.description, achievement.icon)
            };

            all_rows.push((*game, name, desc, icon, is_unlocked, progress));
        }
    }

    // Calculate visible area (leave room for header and help)
    let visible_height = view.content_height().saturating_sub(4) as usize;
    let max_scroll = all_rows.len().saturating_sub(visible_height);
    let scroll = scroll_offset.min(max_scroll);

    // Render visible rows
    for (i, (game, name, desc, icon, is_unlocked, progress)) in
        all_rows.iter().skip(scroll).enumerate()
    {
        if i >= visible_height {
            break;
        }

        // Section header (game name)
        if desc.is_empty() {
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!("\n{}", name),
                    Style::default()
                        .fg(colors.blue())
                        .add_modifier(Modifier::BOLD),
                )],
            );
        } else {
            // Achievement row
            let icon_str = if *is_unlocked {
                format!("[{}]", icon)
            } else {
                "[ ]".to_string()
            };

            let status = progress
                .as_ref()
                .map(|p| p.as_str())
                .unwrap_or(if game.is_some() { "LOCKED" } else { "LOCKED" });

            let name_style = if *is_unlocked {
                Style::default().fg(colors.fg())
            } else {
                Style::default().fg(colors.grey())
            };

            let status_style = if status == "UNLOCKED" {
                Style::default()
                    .fg(colors.green())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.grey())
            };

            // Color the icon part differently
            if *is_unlocked {
                view.render_row(
                    frame,
                    row,
                    vec![
                        Span::styled(
                            format!("{} ", icon_str),
                            Style::default().fg(colors.yellow()),
                        ),
                        Span::styled(format!("{:<18} ", truncate(name, 18)), name_style),
                        Span::styled(
                            format!("{:<38} ", truncate(desc, 38)),
                            Style::default().fg(colors.grey()),
                        ),
                        Span::styled(status.to_string(), status_style),
                    ],
                );
            } else {
                view.render_row(
                    frame,
                    row,
                    vec![
                        Span::styled(format!("{} ", icon_str), Style::default().fg(colors.grey())),
                        Span::styled(format!("{:<18} ", truncate(name, 18)), name_style),
                        Span::styled(
                            format!("{:<38} ", truncate(desc, 38)),
                            Style::default().fg(colors.grey()),
                        ),
                        Span::styled(status.to_string(), status_style),
                    ],
                );
            }
        }
        row += 1;
    }

    // Scroll indicator
    if all_rows.len() > visible_height {
        let indicator = if scroll > 0 && scroll < max_scroll {
            format!("[{}/{}]", scroll + 1, all_rows.len())
        } else if scroll > 0 {
            "[END]".to_string()
        } else {
            "[TOP]".to_string()
        };
        view.render_row(
            frame,
            view.content_height().saturating_sub(2),
            vec![Span::styled(
                format!("{:>76}", indicator),
                Style::default().fg(colors.grey()),
            )],
        );
    }

    // Help footer
    view.render_help(
        frame,
        vec![("\u{2191}\u{2193}", "scroll"), ("Esc", "close")],
    );
}

/// Draw achievement unlock toast notification
pub fn draw_achievement_toast(frame: &mut Frame, toast: &AchievementToast, colors: &ThemeColors) {
    let area = frame.area();

    // Toast dimensions
    let width = 40u16;
    let height = 5u16;

    // Position at top-center
    let x = (area.width.saturating_sub(width)) / 2;
    let y = 2;

    let toast_area = ratatui::layout::Rect::new(x, y, width, height);

    // Draw toast background
    let block = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_style(Style::default().fg(colors.yellow()))
        .style(Style::default().bg(colors.bg()));
    frame.render_widget(block, toast_area);

    // Draw content
    let inner = ratatui::layout::Rect::new(x + 2, y + 1, width - 4, height - 2);

    // Title
    let title = ratatui::widgets::Paragraph::new("ACHIEVEMENT UNLOCKED!").style(
        Style::default()
            .fg(colors.yellow())
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(
        title,
        ratatui::layout::Rect::new(inner.x, inner.y, inner.width, 1),
    );

    // Achievement name with icon
    let achievement = toast.achievement;
    let name_line = format!("[{}] {}", achievement.icon, achievement.name);
    let name = ratatui::widgets::Paragraph::new(name_line).style(Style::default().fg(colors.fg()));
    frame.render_widget(
        name,
        ratatui::layout::Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );

    // Description
    let desc = ratatui::widgets::Paragraph::new(achievement.description)
        .style(Style::default().fg(colors.grey()));
    frame.render_widget(
        desc,
        ratatui::layout::Rect::new(inner.x, inner.y + 2, inner.width, 1),
    );
}

/// Truncate string to max length with ellipsis
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}
