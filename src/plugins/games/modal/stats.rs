//! Stats UI Modal - Display player statistics
//!
//! Shows lifetime and per-game statistics in a formatted table.

use crate::app::ThemeColors;
use crate::plugins::games::platform::{PlayerStats, StatsTracker};
use crate::plugins::games::state::GameType;
use crate::ui::components::FullScreenView;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::Frame;

/// Draw the stats modal
pub fn draw_stats(
    frame: &mut Frame,
    view: &FullScreenView,
    stats: &PlayerStats,
    session_secs: u64,
    colors: &ThemeColors,
) {
    let mut row = 0;

    // Header section - Lifetime stats
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "LIFETIME STATISTICS",
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 1;

    // Separator
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "─".repeat(40),
            Style::default().fg(colors.grey()),
        )],
    );
    row += 1;

    // Total play time
    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Total Play Time:    ", Style::default().fg(colors.fg())),
            Span::styled(
                StatsTracker::format_playtime(stats.total_playtime_secs),
                Style::default().fg(colors.cyan()),
            ),
        ],
    );
    row += 1;

    // Games played
    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Games Played:       ", Style::default().fg(colors.fg())),
            Span::styled(
                format!("{}", stats.total_games_played),
                Style::default().fg(colors.cyan()),
            ),
        ],
    );
    row += 1;

    // Games won
    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Games Won:          ", Style::default().fg(colors.fg())),
            Span::styled(
                format!("{}", stats.total_games_won),
                Style::default().fg(colors.cyan()),
            ),
        ],
    );
    row += 1;

    // First played
    let first_played_str = stats
        .first_played
        .map(|dt| dt.format("%b %d, %Y").to_string())
        .unwrap_or_else(|| "Never".to_string());
    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Playing Since:      ", Style::default().fg(colors.fg())),
            Span::styled(first_played_str, Style::default().fg(colors.cyan())),
        ],
    );
    row += 1;

    // Session time
    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Session Time:       ", Style::default().fg(colors.fg())),
            Span::styled(
                StatsTracker::format_playtime(session_secs),
                Style::default().fg(colors.green()),
            ),
        ],
    );
    row += 2;

    // Per-game stats header
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "PER-GAME STATISTICS",
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 1;

    // Table header
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "─".repeat(72),
            Style::default().fg(colors.grey()),
        )],
    );
    row += 1;

    // Column headers
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!(
                "{:<14} {:>8} {:>6} {:>12} {:>10} {:>10}",
                "Game", "Played", "Won", "High Score", "Best Lvl", "Time"
            ),
            Style::default()
                .fg(colors.blue())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "─".repeat(72),
            Style::default().fg(colors.grey()),
        )],
    );
    row += 1;

    // Game rows
    for game_type in GameType::all() {
        let game_stats = stats.games.get(game_type.name());

        let (played, won, high_score, best_level, time) = match game_stats {
            Some(gs) => (
                format!("{}", gs.times_played),
                format!("{}", gs.times_won),
                format!("{}", gs.high_score),
                gs.best_level
                    .map(|l| format!("{}", l))
                    .unwrap_or_else(|| "-".to_string()),
                StatsTracker::format_playtime(gs.total_playtime_secs),
            ),
            None => (
                "-".to_string(),
                "-".to_string(),
                "-".to_string(),
                "-".to_string(),
                "-".to_string(),
            ),
        };

        let line = format!(
            "{:<14} {:>8} {:>6} {:>12} {:>10} {:>10}",
            game_type.name(),
            played,
            won,
            high_score,
            best_level,
            time
        );

        // Highlight games that have been played
        let style = if game_stats.is_some() && game_stats.unwrap().times_played > 0 {
            Style::default().fg(colors.fg())
        } else {
            Style::default().fg(colors.grey())
        };

        view.render_row(frame, row, vec![Span::styled(line, style)]);
        row += 1;
    }

    // Help footer
    view.render_help(frame, vec![("Esc", "close")]);
}
