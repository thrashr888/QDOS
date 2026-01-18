//! Asteroid game modal rendering

use super::super::asteroid::{self, AsteroidState};
use qdos_plugin_api::ui::FullScreenView;
use qdos_plugin_api::ThemeColors;
use ratatui::{style::Style, text::Span, Frame};

/// Draws the Asteroid game board
pub fn draw_asteroid(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &AsteroidState,
    colors: &ThemeColors,
) {
    // Score, lives, and level
    view.render_row(
        frame,
        0,
        vec![
            Span::styled(
                format!("Score: {:<8}", state.score),
                Style::default().fg(colors.green()),
            ),
            Span::styled(
                format!("  Lives: {}", "♦".repeat(state.lives as usize)),
                Style::default().fg(colors.red()),
            ),
            Span::styled(
                format!("  Level: {}", state.level),
                Style::default().fg(colors.yellow()),
            ),
        ],
    );

    // Draw top border
    view.render_row(
        frame,
        1,
        vec![Span::styled(
            format!("╔{}╗", "═".repeat(asteroid::BOARD_WIDTH)),
            Style::default().fg(colors.cyan()),
        )],
    );

    // Build display buffer
    let mut display: Vec<Vec<(char, ratatui::style::Color)>> =
        vec![vec![(' ', colors.bg()); asteroid::BOARD_WIDTH]; asteroid::BOARD_HEIGHT];

    // Draw asteroids
    for ast in &state.asteroids {
        let (ax, ay) = ast.pos();
        if ax >= 0
            && ax < asteroid::BOARD_WIDTH as i32
            && ay >= 0
            && ay < asteroid::BOARD_HEIGHT as i32
        {
            display[ay as usize][ax as usize] = (ast.size.char(), colors.grey());
        }
    }

    // Draw bullets
    for bullet in &state.bullets {
        let bx = bullet.x.round() as i32;
        let by = bullet.y.round() as i32;
        if bx >= 0
            && bx < asteroid::BOARD_WIDTH as i32
            && by >= 0
            && by < asteroid::BOARD_HEIGHT as i32
        {
            display[by as usize][bx as usize] = ('.', colors.yellow());
        }
    }

    // Draw ship (blink if invincible)
    let show_ship = state.invincible_frames == 0 || state.invincible_frames % 6 < 3;
    if show_ship {
        let (sx, sy) = state.ship.pos();
        if sx >= 0
            && sx < asteroid::BOARD_WIDTH as i32
            && sy >= 0
            && sy < asteroid::BOARD_HEIGHT as i32
        {
            display[sy as usize][sx as usize] = (state.ship.direction_char(), colors.green());
        }
    }

    // Render board
    for (y, row) in display.iter().enumerate() {
        let mut row_spans: Vec<Span> = vec![Span::styled("║", Style::default().fg(colors.cyan()))];

        for &(ch, color) in row.iter() {
            if ch == ' ' {
                row_spans.push(Span::raw(" "));
            } else {
                row_spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
            }
        }

        row_spans.push(Span::styled("║", Style::default().fg(colors.cyan())));
        view.render_row(frame, 2 + y as u16, row_spans);
    }

    // Draw bottom border
    view.render_row(
        frame,
        2 + asteroid::BOARD_HEIGHT as u16,
        vec![Span::styled(
            format!("╚{}╝", "═".repeat(asteroid::BOARD_WIDTH)),
            Style::default().fg(colors.cyan()),
        )],
    );

    // Game over overlay
    if state.game_over {
        view.render_row(
            frame,
            asteroid::BOARD_HEIGHT as u16 / 2 + 2,
            vec![Span::styled(
                format!(
                    "{:^width$}",
                    "GAME OVER - Press SPACE to restart",
                    width = asteroid::BOARD_WIDTH + 2
                ),
                Style::default().fg(colors.red()),
            )],
        );
    }

    // Help
    view.render_help(
        frame,
        vec![
            ("←/→", "rotate"),
            ("↑", "thrust"),
            ("Space", "fire"),
            ("Esc", "quit"),
        ],
    );
}
