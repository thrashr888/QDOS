//! Bubbles game modal rendering

use super::super::bubbles::{self, BubbleColor, BubblesState};
use qdos_plugin_api::ui::FullScreenView;
use qdos_plugin_api::ThemeColors;
use ratatui::{style::Style, text::Span, Frame};

/// Draws the Bubbles game board
pub fn draw_bubbles(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &BubblesState,
    colors: &ThemeColors,
) {
    // Score and status
    view.render_row(
        frame,
        0,
        vec![
            Span::styled(
                format!("Score: {:<8}", state.score),
                Style::default().fg(colors.green()),
            ),
            Span::styled(
                format!("  Next: {}", state.next_bubble.to_char()),
                Style::default().fg(bubble_color(state.next_bubble, colors)),
            ),
        ],
    );

    // Draw top border
    view.render_row(
        frame,
        1,
        vec![Span::styled(
            format!("╔{}╗", "═".repeat(bubbles::BOARD_WIDTH * 2)),
            Style::default().fg(colors.cyan()),
        )],
    );

    // Draw board
    for y in 0..bubbles::BOARD_HEIGHT {
        let mut row_spans: Vec<Span> = vec![Span::styled("║", Style::default().fg(colors.cyan()))];

        // Offset odd rows for hex grid effect
        if y % 2 == 1 {
            row_spans.push(Span::raw(" "));
        }

        for x in 0..bubbles::BOARD_WIDTH {
            let bubble = state.board[y][x];

            // Check if flying bubble is here
            let is_flying = state
                .flying_bubble
                .as_ref()
                .is_some_and(|fb| fb.x.round() as usize == x && fb.y.round() as usize == y);

            if is_flying {
                let fb = state.flying_bubble.as_ref().unwrap();
                row_spans.push(Span::styled(
                    "()",
                    Style::default().fg(bubble_color(fb.color, colors)),
                ));
            } else if bubble != BubbleColor::Empty {
                row_spans.push(Span::styled(
                    "()",
                    Style::default().fg(bubble_color(bubble, colors)),
                ));
            } else {
                row_spans.push(Span::raw("  "));
            }
        }

        // Pad odd rows
        if y % 2 == 1 {
            row_spans.push(Span::raw(" "));
        }

        row_spans.push(Span::styled("║", Style::default().fg(colors.cyan())));

        view.render_row(frame, 2 + y as u16, row_spans);
    }

    // Draw bottom border with shooter
    let shooter_row = 2 + bubbles::BOARD_HEIGHT as u16;
    let mid = bubbles::BOARD_WIDTH;

    // Draw shooter angle indicator
    let angle_offset = (state.shooter_angle / 5.0).round() as i32;
    let shooter_x =
        (mid as i32 + angle_offset).clamp(1, (bubbles::BOARD_WIDTH * 2 - 2) as i32) as usize;

    let mut shooter_line = vec![Span::styled("╠", Style::default().fg(colors.cyan()))];
    for i in 0..(bubbles::BOARD_WIDTH * 2) {
        if i == shooter_x - 1 || i == shooter_x || i == shooter_x + 1 {
            shooter_line.push(Span::styled(
                if i == shooter_x { "^" } else { "/" },
                Style::default().fg(colors.yellow()),
            ));
        } else {
            shooter_line.push(Span::styled("═", Style::default().fg(colors.cyan())));
        }
    }
    shooter_line.push(Span::styled("╣", Style::default().fg(colors.cyan())));
    view.render_row(frame, shooter_row, shooter_line);

    // Current bubble indicator
    view.render_row(
        frame,
        shooter_row + 1,
        vec![
            Span::styled("║", Style::default().fg(colors.cyan())),
            Span::raw(format!("{:>width$}", "", width = shooter_x - 1)),
            Span::styled(
                "()",
                Style::default().fg(bubble_color(state.current_bubble, colors)),
            ),
            Span::raw(format!(
                "{:>width$}",
                "",
                width = bubbles::BOARD_WIDTH * 2 - shooter_x - 1
            )),
            Span::styled("║", Style::default().fg(colors.cyan())),
        ],
    );

    // Bottom border
    view.render_row(
        frame,
        shooter_row + 2,
        vec![Span::styled(
            format!("╚{}╝", "═".repeat(bubbles::BOARD_WIDTH * 2)),
            Style::default().fg(colors.cyan()),
        )],
    );

    // Game over overlay
    if state.game_over {
        let msg = if state.game_won {
            "YOU WIN! Press SPACE to play again"
        } else {
            "GAME OVER - Press SPACE to restart"
        };
        view.render_row(
            frame,
            bubbles::BOARD_HEIGHT as u16 / 2 + 2,
            vec![Span::styled(
                format!("{:^width$}", msg, width = bubbles::BOARD_WIDTH * 2 + 2),
                Style::default().fg(colors.yellow()),
            )],
        );
    }

    // Help
    view.render_help(
        frame,
        vec![("←/→", "aim"), ("Space", "shoot"), ("Esc", "quit")],
    );
}

fn bubble_color(color: BubbleColor, colors: &ThemeColors) -> ratatui::style::Color {
    match color {
        BubbleColor::Red => colors.red(),
        BubbleColor::Blue => colors.blue(),
        BubbleColor::Green => colors.green(),
        BubbleColor::Yellow => colors.yellow(),
        BubbleColor::Purple => colors.cyan(),
        BubbleColor::Empty => colors.grey(),
    }
}
