//! Snake game modal rendering
//!
//! This module handles the visual rendering of the Snake game within
//! the games plugin modal. It displays the game board, snake, food,
//! score, and help text.

use crate::app::ThemeColors;
use crate::plugins::games::snake::{self, Direction, Position, SnakeState};
use crate::ui::components::FullScreenView;
use ratatui::{
    style::{Modifier, Style},
    text::Span,
    Frame,
};

/// Renders the Snake game state to the terminal.
///
/// Draws the game board with borders, the snake (head with directional arrow,
/// body with textured blocks), pulsing food, score display, and help text.
pub fn draw_snake(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &SnakeState,
    colors: &ThemeColors,
) {
    // Score with snake length
    view.render_row(
        frame,
        0,
        vec![
            Span::styled("Score: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}", state.score),
                Style::default()
                    .fg(colors.green())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  Length: {}", state.body.len()),
                Style::default().fg(colors.cyan()),
            ),
        ],
    );

    // Draw solid top border
    let border_color = colors.cyan();
    view.render_row(
        frame,
        1,
        vec![Span::styled(
            format!("╔{}╗", "═".repeat(snake::BOARD_WIDTH)),
            Style::default().fg(border_color),
        )],
    );

    // Draw board with enhanced visuals
    for y in 0..snake::BOARD_HEIGHT {
        let mut row_spans: Vec<Span> = vec![Span::styled("║", Style::default().fg(border_color))];

        for x in 0..snake::BOARD_WIDTH {
            let pos = Position::new(x as i32, y as i32);

            if state.is_head(pos) {
                // Head is solid and bold - show direction
                let head_char = match state.direction {
                    Direction::Up => "▲",
                    Direction::Down => "▼",
                    Direction::Left => "◄",
                    Direction::Right => "►",
                };
                row_spans.push(Span::styled(
                    head_char,
                    Style::default()
                        .fg(colors.green())
                        .add_modifier(Modifier::BOLD),
                ));
            } else if state.is_snake(pos) {
                // Body is textured
                row_spans.push(Span::styled("▓", Style::default().fg(colors.green())));
            } else if pos == state.food {
                // Food pulses based on tick
                let pulse = (state.tick_count / 5).is_multiple_of(2);
                let food_char = if pulse { "●" } else { "○" };
                row_spans.push(Span::styled(
                    food_char,
                    Style::default()
                        .fg(colors.red())
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                // Subtle floor texture
                row_spans.push(Span::styled(
                    "·",
                    Style::default()
                        .fg(colors.grey())
                        .add_modifier(Modifier::DIM),
                ));
            }
        }

        row_spans.push(Span::styled("║", Style::default().fg(border_color)));
        view.render_row(frame, 2 + y as u16, row_spans);
    }

    // Draw solid bottom border
    view.render_row(
        frame,
        2 + snake::BOARD_HEIGHT as u16,
        vec![Span::styled(
            format!("╚{}╝", "═".repeat(snake::BOARD_WIDTH)),
            Style::default().fg(border_color),
        )],
    );

    // Direction indicator
    let dir_char = match state.direction {
        Direction::Up => "↑",
        Direction::Down => "↓",
        Direction::Left => "←",
        Direction::Right => "→",
    };
    view.render_row(
        frame,
        3 + snake::BOARD_HEIGHT as u16,
        vec![Span::styled(
            format!("Direction: {}", dir_char),
            Style::default().fg(colors.grey()),
        )],
    );

    let help = vec![("←↑↓→", "move"), ("P", "pause"), ("Esc", "quit")];
    view.render_help(frame, help);
}
