//! Dino Jump game modal rendering

use super::super::dinojump::{self, DinoJumpState, ObstacleType};
use qdos_plugin_api::ui::FullScreenView;
use qdos_plugin_api::ThemeColors;
use ratatui::{style::Style, text::Span, Frame};

/// Draws the Dino Jump game
pub fn draw_dinojump(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DinoJumpState,
    colors: &ThemeColors,
) {
    // Score and high score
    view.render_row(
        frame,
        0,
        vec![
            Span::styled(
                format!("Score: {:<8}", state.score),
                Style::default().fg(colors.green()),
            ),
            Span::styled(
                format!("  HI: {:<8}", state.high_score),
                Style::default().fg(colors.yellow()),
            ),
            Span::styled(
                format!("  Speed: {:.1}", state.speed),
                Style::default().fg(colors.cyan()),
            ),
        ],
    );

    // Draw top border
    view.render_row(
        frame,
        1,
        vec![Span::styled(
            format!("╔{}╗", "═".repeat(dinojump::BOARD_WIDTH)),
            Style::default().fg(colors.cyan()),
        )],
    );

    // Build display buffer
    let mut display: Vec<Vec<(char, ratatui::style::Color)>> =
        vec![vec![(' ', colors.bg()); dinojump::BOARD_WIDTH]; dinojump::BOARD_HEIGHT];

    // Draw ground
    let ground = state.ground_chars();
    for (x, ch) in ground.chars().enumerate() {
        if x < dinojump::BOARD_WIDTH {
            display[dinojump::GROUND_Y][x] = (ch, colors.grey());
        }
    }

    // Draw obstacles
    for obstacle in &state.obstacles {
        let ox = obstacle.x as i32;
        let chars = obstacle.obstacle_type.chars();
        let y_base = dinojump::GROUND_Y as i32 + obstacle.obstacle_type.y_offset();

        for (row_idx, row) in chars.iter().enumerate() {
            let y = y_base - (chars.len() as i32 - 1 - row_idx as i32);
            if y >= 0 && y < dinojump::BOARD_HEIGHT as i32 {
                for (col_idx, ch) in row.chars().enumerate() {
                    let x = ox + col_idx as i32;
                    if x >= 0 && x < dinojump::BOARD_WIDTH as i32 {
                        let color = match obstacle.obstacle_type {
                            ObstacleType::Bird => colors.cyan(),
                            _ => colors.green(),
                        };
                        display[y as usize][x as usize] = (ch, color);
                    }
                }
            }
        }
    }

    // Draw dino
    let dino_x = 5;
    let dino_y = state.dino.y as usize;

    if state.dino.is_ducking {
        // Ducking dino (shorter)
        if dino_y < dinojump::BOARD_HEIGHT {
            display[dino_y][dino_x] = ('>', colors.yellow());
            display[dino_y][dino_x + 1] = ('=', colors.yellow());
        }
    } else {
        // Standing/jumping dino
        if dino_y > 0 && dino_y - 1 < dinojump::BOARD_HEIGHT {
            display[dino_y - 1][dino_x] = ('o', colors.yellow());
            display[dino_y - 1][dino_x + 1] = ('<', colors.yellow());
        }
        if dino_y < dinojump::BOARD_HEIGHT {
            display[dino_y][dino_x] = ('|', colors.yellow());
            display[dino_y][dino_x + 1] = ('/', colors.yellow());
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
        2 + dinojump::BOARD_HEIGHT as u16,
        vec![Span::styled(
            format!("╚{}╝", "═".repeat(dinojump::BOARD_WIDTH)),
            Style::default().fg(colors.cyan()),
        )],
    );

    // Game over overlay
    if state.game_over {
        view.render_row(
            frame,
            dinojump::BOARD_HEIGHT as u16 / 2 + 2,
            vec![Span::styled(
                format!(
                    "{:^width$}",
                    "GAME OVER - Press SPACE to restart",
                    width = dinojump::BOARD_WIDTH + 2
                ),
                Style::default().fg(colors.red()),
            )],
        );
    }

    // Help
    view.render_help(
        frame,
        vec![("↑/Space", "jump"), ("↓", "duck"), ("Esc", "quit")],
    );
}
