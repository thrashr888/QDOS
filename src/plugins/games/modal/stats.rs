//! Stats UI Modal - Display player statistics
//!
//! Shows lifetime and per-game statistics in a formatted table with scrolling.

use super::super::platform::{PlayerStats, StatsTracker};
use super::super::state::GameType;
use qdos_plugin_api::ui::FullScreenView;
use qdos_plugin_api::ThemeColors;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::Frame;

/// Draw the stats modal with scrolling support
pub fn draw_stats(
    frame: &mut Frame,
    view: &FullScreenView,
    stats: &PlayerStats,
    casino_credits: i64,
    scroll_offset: usize,
    session_secs: u64,
    colors: &ThemeColors,
) {
    // Build all content lines first, then apply scrolling
    let mut lines: Vec<Vec<Span>> = Vec::new();

    let yellow_bold = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let grey = Style::default().fg(colors.grey());
    let white = Style::default().fg(colors.fg());
    let cyan = Style::default().fg(colors.cyan());
    let green = Style::default().fg(colors.green());
    let blue_bold = Style::default()
        .fg(colors.blue())
        .add_modifier(Modifier::BOLD);

    // Header section - Lifetime stats
    lines.push(vec![Span::styled("LIFETIME STATISTICS", yellow_bold)]);
    lines.push(vec![Span::styled("─".repeat(40), grey)]);

    // Total play time
    lines.push(vec![
        Span::styled("Total Play Time:    ", white),
        Span::styled(
            StatsTracker::format_playtime(stats.total_playtime_secs),
            cyan,
        ),
    ]);

    // Games played
    lines.push(vec![
        Span::styled("Games Played:       ", white),
        Span::styled(format!("{}", stats.total_games_played), cyan),
    ]);

    // Games won
    lines.push(vec![
        Span::styled("Games Won:          ", white),
        Span::styled(format!("{}", stats.total_games_won), cyan),
    ]);

    // First played
    let first_played_str = stats
        .first_played
        .map(|dt| dt.format("%b %d, %Y").to_string())
        .unwrap_or_else(|| "Never".to_string());
    lines.push(vec![
        Span::styled("Playing Since:      ", white),
        Span::styled(first_played_str, cyan),
    ]);

    // Session time
    lines.push(vec![
        Span::styled("Session Time:       ", white),
        Span::styled(StatsTracker::format_playtime(session_secs), green),
    ]);

    // Casino credits
    lines.push(vec![
        Span::styled("Casino Credits:     ", white),
        Span::styled(format!("{}", casino_credits), yellow_bold),
    ]);

    // Blank line
    lines.push(vec![]);

    // Per-game stats header
    lines.push(vec![Span::styled("PER-GAME STATISTICS", yellow_bold)]);
    lines.push(vec![Span::styled("─".repeat(72), grey)]);

    // Column headers
    lines.push(vec![Span::styled(
        format!(
            "{:<14} {:>8} {:>6} {:>12} {:>10} {:>10}",
            "Game", "Played", "Won", "High Score", "Best Lvl", "Time"
        ),
        blue_bold,
    )]);
    lines.push(vec![Span::styled("─".repeat(72), grey)]);

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
            white
        } else {
            grey
        };

        lines.push(vec![Span::styled(line, style)]);
    }

    // Calculate visible area (leaving room for help footer)
    let visible_rows = 19usize; // Approximate visible content rows
    let total_lines = lines.len();
    let max_scroll = total_lines.saturating_sub(visible_rows);
    let effective_scroll = scroll_offset.min(max_scroll);

    // Render visible lines with scroll offset
    for (i, line) in lines
        .into_iter()
        .skip(effective_scroll)
        .take(visible_rows)
        .enumerate()
    {
        view.render_row(frame, i as u16, line);
    }

    // Show scroll indicator if there's more content
    if total_lines > visible_rows {
        let scroll_indicator = format!(
            " [{}/{}] ",
            effective_scroll + 1,
            max_scroll.saturating_add(1)
        );
        view.render_row(
            frame,
            visible_rows as u16,
            vec![Span::styled(scroll_indicator, grey)],
        );
    }

    // Help footer with scroll hints
    if total_lines > visible_rows {
        view.render_help(
            frame,
            vec![("↑/↓", "scroll"), ("PgUp/Dn", "page"), ("Esc", "close")],
        );
    } else {
        view.render_help(frame, vec![("Esc", "close")]);
    }
}
