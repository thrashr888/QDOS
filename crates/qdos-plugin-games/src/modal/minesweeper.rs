use super::super::minesweeper::{CellState, MinesweeperState};
use super::super::platform::GameEngine;
use qdos_plugin_api::ui::FullScreenView;
use qdos_plugin_api::ThemeColors;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    Frame,
};

pub fn draw(frame: &mut Frame, area: Rect, state: &MinesweeperState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " MINESWEEPER ", colors);
    view.render_frame(frame);

    let mut row = 1;

    // Game status
    if state.game_won {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "*** YOU WIN! ***",
                Style::default()
                    .fg(colors.green())
                    .add_modifier(Modifier::BOLD),
            )],
        );
        row += 1;
        view.render_row(
            frame,
            row,
            vec![
                Span::styled("Time: ", Style::default().fg(colors.grey())),
                Span::styled(
                    format!("{}s", state.time_elapsed),
                    Style::default().fg(colors.yellow()),
                ),
                Span::styled("  Score: ", Style::default().fg(colors.grey())),
                Span::styled(
                    format!("{}", state.get_score()),
                    Style::default().fg(colors.yellow()),
                ),
            ],
        );
        row += 2;
    } else if state.game_over {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "*** GAME OVER - Mine Hit! ***",
                Style::default()
                    .fg(colors.red())
                    .add_modifier(Modifier::BOLD),
            )],
        );
        row += 2;
    } else {
        // Status bar
        view.render_row(
            frame,
            row,
            vec![
                Span::styled("Flags: ", Style::default().fg(colors.grey())),
                Span::styled(
                    format!("{}/40", state.flags_placed),
                    Style::default().fg(colors.yellow()),
                ),
                Span::styled("  Time: ", Style::default().fg(colors.grey())),
                Span::styled(
                    format!("{}s", state.time_elapsed),
                    Style::default().fg(colors.yellow()),
                ),
                Span::styled("  Revealed: ", Style::default().fg(colors.grey())),
                Span::styled(
                    format!("{}/216", state.cells_revealed),
                    Style::default().fg(colors.cyan()),
                ),
            ],
        );
        row += 2;
    }

    // Render grid
    for (y, grid_row) in state.grid.iter().enumerate() {
        let mut line_spans = Vec::new();
        line_spans.push(Span::raw("  ")); // Left padding

        for (x, cell) in grid_row.iter().enumerate() {
            let is_cursor = x == state.cursor_x && y == state.cursor_y;

            let (ch, style) = match cell.state {
                CellState::Hidden => {
                    if is_cursor {
                        (
                            '□',
                            Style::default()
                                .fg(colors.yellow())
                                .add_modifier(Modifier::BOLD),
                        )
                    } else {
                        ('□', Style::default().fg(colors.grey()))
                    }
                }
                CellState::Flagged => {
                    if is_cursor {
                        (
                            'F',
                            Style::default()
                                .fg(colors.red())
                                .add_modifier(Modifier::BOLD),
                        )
                    } else {
                        ('F', Style::default().fg(colors.red()))
                    }
                }
                CellState::Revealed => {
                    if cell.is_mine {
                        let mut style = Style::default()
                            .fg(colors.red())
                            .add_modifier(Modifier::BOLD);
                        if is_cursor {
                            style = style.bg(colors.grey());
                        }
                        ('*', style)
                    } else if cell.adjacent_mines == 0 {
                        let mut style = Style::default().fg(colors.grey());
                        if is_cursor {
                            style = style.bg(colors.grey()).add_modifier(Modifier::REVERSED);
                        }
                        (' ', style)
                    } else {
                        let digit_char = char::from_digit(cell.adjacent_mines as u32, 10).unwrap();
                        let color = match cell.adjacent_mines {
                            1 => colors.blue(),
                            2 => colors.green(),
                            3 => colors.red(),
                            4 => colors.cyan(),
                            _ => colors.yellow(),
                        };
                        let mut style = Style::default().fg(color);
                        if is_cursor {
                            style = style.bg(colors.grey()).add_modifier(Modifier::BOLD);
                        }
                        (digit_char, style)
                    }
                }
            };

            line_spans.push(Span::styled(ch.to_string(), style));
            line_spans.push(Span::raw(" ")); // Spacing between cells
        }

        view.render_row(frame, row, line_spans);
        row += 1;
    }

    // Help text
    if state.game_over || state.game_won {
        view.render_help(frame, vec![("R", "restart"), ("Esc", "quit")]);
    } else {
        view.render_help(
            frame,
            vec![
                ("Arrows/HJKL", "move"),
                ("Space/Enter", "reveal"),
                ("F", "flag"),
                ("P", "pause"),
                ("Esc", "quit"),
            ],
        );
    }
}
