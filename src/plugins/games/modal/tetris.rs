//! Tetris game modal rendering
//!
//! This module handles the rendering of the Tetris game within the games plugin modal.

use super::super::tetris::{self, TetrisState};
use qdos_plugin_api::ui::FullScreenView;
use qdos_plugin_api::ThemeColors;
use ratatui::{
    style::{Modifier, Style},
    text::Span,
    Frame,
};

/// Draws the Tetris game state to the modal view.
///
/// Renders the game board, current piece, ghost piece preview, score/level info,
/// next piece preview, and help text.
pub fn draw_tetris(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &TetrisState,
    colors: &ThemeColors,
) {
    // Board offset for centering
    let board_start_x = 5;

    // Draw score/level info
    view.render_row(
        frame,
        0,
        vec![
            Span::styled(
                format!("Score: {:<8}", state.score),
                Style::default().fg(colors.green()),
            ),
            Span::styled(
                format!("  Level: {:<3}", state.level),
                Style::default().fg(colors.cyan()),
            ),
            Span::styled(
                format!("  Lines: {}", state.lines_cleared),
                Style::default().fg(colors.yellow()),
            ),
        ],
    );

    // Get ghost piece blocks for preview
    let ghost_blocks = state.ghost_blocks();

    // Draw board with ghost piece
    for y in 0..tetris::BOARD_HEIGHT {
        let mut row_spans: Vec<Span> = vec![
            Span::raw(" ".repeat(board_start_x)),
            Span::styled("║", Style::default().fg(colors.blue())),
        ];

        for x in 0..tetris::BOARD_WIDTH {
            let coord = (x as i32, y as i32);

            // Priority 1: Active piece (solid, bright)
            if let Some(piece) = &state.current_piece {
                if piece.blocks().contains(&coord) {
                    row_spans.push(Span::styled("██", Style::default().fg(colors.cyan())));
                    continue;
                }
            }

            // Priority 2: Ghost piece (textured, dim)
            if ghost_blocks.contains(&coord) {
                row_spans.push(Span::styled(
                    "░░",
                    Style::default()
                        .fg(colors.grey())
                        .add_modifier(Modifier::DIM),
                ));
                continue;
            }

            // Priority 3: Placed blocks (solid)
            if state.board[y][x].is_some() {
                row_spans.push(Span::styled("▓▓", Style::default().fg(colors.fg())));
                continue;
            }

            // Priority 4: Empty space (subtle checkerboard pattern)
            if (x + y) % 2 == 0 {
                row_spans.push(Span::styled(
                    "· ",
                    Style::default()
                        .fg(colors.grey())
                        .add_modifier(Modifier::DIM),
                ));
            } else {
                row_spans.push(Span::raw("  "));
            }
        }

        row_spans.push(Span::styled("║", Style::default().fg(colors.blue())));

        view.render_row(frame, 1 + y as u16, row_spans);
    }

    // Bottom border
    view.render_row(
        frame,
        1 + tetris::BOARD_HEIGHT as u16,
        vec![
            Span::raw(" ".repeat(board_start_x)),
            Span::styled(
                format!("╚{}╝", "══".repeat(tetris::BOARD_WIDTH)),
                Style::default().fg(colors.cyan()),
            ),
        ],
    );

    // Next piece preview
    let preview_x = board_start_x + 2 + tetris::BOARD_WIDTH * 2 + 4;
    view.render_row(
        frame,
        2,
        vec![
            Span::raw(" ".repeat(preview_x)),
            Span::styled("Next:", Style::default().fg(colors.grey())),
        ],
    );

    let next_piece = crate::plugins::games::tetris::Piece::new(state.next_piece);
    for (dx, dy) in next_piece.piece_type.shape(0) {
        let px = (preview_x as i32 + 2 + dx * 2) as usize;
        let py = (4 + dy) as u16;
        if py < view.content_height() {
            view.render_row(
                frame,
                py,
                vec![
                    Span::raw(" ".repeat(px)),
                    Span::styled("██", Style::default().fg(colors.yellow())),
                ],
            );
        }
    }

    let help = vec![
        ("←→", "move"),
        ("↑", "rotate"),
        ("↓", "drop"),
        ("Space", "hard drop"),
        ("P", "pause"),
        ("Esc", "quit"),
    ];
    view.render_help(frame, help);
}
