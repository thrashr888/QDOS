//! Breakout game modal rendering
//!
//! This module handles the drawing of the Breakout game UI including
//! the paddle, ball, bricks, score, lives, and game borders.

use super::super::breakout::{self, BreakoutState};
use qdos_plugin_api::ui::FullScreenView;
use qdos_plugin_api::ThemeColors;
use ratatui::{style::Style, text::Span, Frame};

/// Draws the Breakout game board including paddle, ball, bricks, and UI elements.
pub fn draw_breakout(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &BreakoutState,
    colors: &ThemeColors,
) {
    // Score and lives
    view.render_row(
        frame,
        0,
        vec![
            Span::styled(
                format!("Score: {:<8}", state.score),
                Style::default().fg(colors.green()),
            ),
            Span::styled(
                format!("  Lives: {}", "♥".repeat(state.lives as usize)),
                Style::default().fg(colors.red()),
            ),
        ],
    );

    // Draw top border
    view.render_row(
        frame,
        1,
        vec![Span::styled(
            format!("╔{}╗", "═".repeat(breakout::BOARD_WIDTH)),
            Style::default().fg(colors.cyan()),
        )],
    );

    // Draw board
    let brick_colors = [colors.red(), colors.yellow(), colors.green(), colors.cyan()];
    let (ball_x, ball_y) = state.ball.pos();

    for y in 0..breakout::BOARD_HEIGHT {
        let mut row_spans: Vec<Span> = vec![Span::styled("║", Style::default().fg(colors.cyan()))];

        for x in 0..breakout::BOARD_WIDTH {
            let (bx, by) = (ball_x as usize, ball_y as usize);

            if x == bx && y == by {
                // Ball
                row_spans.push(Span::styled("●", Style::default().fg(colors.fg())));
            } else if y == breakout::BOARD_HEIGHT - 2 && state.paddle_positions().any(|px| px == x)
            {
                // Paddle
                row_spans.push(Span::styled("═", Style::default().fg(colors.yellow())));
            } else if let Some(row) = state.brick_at(x, y) {
                // Brick
                row_spans.push(Span::styled("█", Style::default().fg(brick_colors[row])));
            } else {
                row_spans.push(Span::raw(" "));
            }
        }

        row_spans.push(Span::styled("║", Style::default().fg(colors.cyan())));

        view.render_row(frame, 2 + y as u16, row_spans);
    }

    // Draw bottom border
    view.render_row(
        frame,
        2 + breakout::BOARD_HEIGHT as u16,
        vec![Span::styled(
            format!("╚{}╝", "═".repeat(breakout::BOARD_WIDTH)),
            Style::default().fg(colors.cyan()),
        )],
    );

    // Launch hint
    if !state.ball_launched {
        view.render_row(
            frame,
            3 + breakout::BOARD_HEIGHT as u16,
            vec![Span::styled(
                "Press SPACE to launch!",
                Style::default().fg(colors.yellow()),
            )],
        );
    }

    let help = if state.ball_launched {
        vec![("←→", "move"), ("P", "pause"), ("Esc", "quit")]
    } else {
        vec![("←→", "move"), ("Space", "launch"), ("Esc", "quit")]
    };
    view.render_help(frame, help);
}
