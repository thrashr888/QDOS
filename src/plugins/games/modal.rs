//! Games plugin modal rendering

use super::breakout::{self, BreakoutState};
use super::rogue::{self, RogueState};
use super::snake::{self, Direction, Position, SnakeState};
use super::state::{GameType, GamesState, GamesView};
use super::tetris::{self, TetrisState};
use super::trek::{self, TrekState};
use crate::app::ThemeColors;
use crate::ui::components::FullScreenView;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    Frame,
};

/// Draw the games modal
pub fn draw_games_modal(frame: &mut Frame, area: Rect, state: &GamesState, colors: &ThemeColors) {
    let title = match state.view {
        GamesView::Menu => " Games ",
        GamesView::Playing | GamesView::Paused => match state.current_game {
            Some(GameType::Tetris) => " Tetris ",
            Some(GameType::Snake) => " Snake ",
            Some(GameType::Breakout) => " Breakout ",
            Some(GameType::Rogue) => " Rogue ",
            Some(GameType::Trek) => " Star Trek ",
            None => " Games ",
        },
        GamesView::GameOver => " Game Over ",
    };

    let view = FullScreenView::new(area, title, colors);
    view.render_frame(frame);

    match state.view {
        GamesView::Menu => draw_menu(frame, &view, state, colors),
        GamesView::Playing => draw_game(frame, &view, state, colors),
        GamesView::Paused => draw_paused(frame, &view, state, colors),
        GamesView::GameOver => draw_game_over(frame, &view, state, colors),
    }
}

fn draw_menu(frame: &mut Frame, view: &FullScreenView, state: &GamesState, colors: &ThemeColors) {
    // Title art
    view.render_row(
        frame,
        1,
        vec![Span::styled(
            "╔═══════════════════════════════════════════╗",
            Style::default().fg(colors.cyan()),
        )],
    );
    view.render_row(
        frame,
        2,
        vec![Span::styled(
            "║           R - D O S   G A M E S           ║",
            Style::default().fg(colors.cyan()),
        )],
    );
    view.render_row(
        frame,
        3,
        vec![Span::styled(
            "╚═══════════════════════════════════════════╝",
            Style::default().fg(colors.cyan()),
        )],
    );

    // Game list
    let start_row = 5;
    for (i, game) in GameType::all().iter().enumerate() {
        let is_selected = i == state.selected_game;
        let high_score = state.high_scores[i];

        let name_style = if is_selected {
            Style::default()
                .fg(colors.yellow())
                .bg(colors.red())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.fg())
        };

        let desc_style = if is_selected {
            Style::default().fg(colors.grey()).bg(colors.red())
        } else {
            Style::default().fg(colors.grey())
        };

        let prefix = if is_selected { "► " } else { "  " };

        view.render_row(
            frame,
            start_row + (i as u16 * 2),
            vec![
                Span::styled(prefix, name_style),
                Span::styled(format!("{:<12}", game.name()), name_style),
                Span::styled(format!(" - {}", game.description()), desc_style),
            ],
        );

        if high_score > 0 {
            view.render_row(
                frame,
                start_row + (i as u16 * 2) + 1,
                vec![Span::styled(
                    format!("    High Score: {}", high_score),
                    Style::default().fg(colors.green()),
                )],
            );
        }
    }

    let help = vec![("↑↓", "select"), ("Enter", "play"), ("Esc", "close")];
    view.render_help(frame, help);
}

fn draw_game(frame: &mut Frame, view: &FullScreenView, state: &GamesState, colors: &ThemeColors) {
    match state.current_game {
        Some(GameType::Tetris) => draw_tetris(frame, view, &state.tetris, colors),
        Some(GameType::Snake) => draw_snake(frame, view, &state.snake, colors),
        Some(GameType::Breakout) => draw_breakout(frame, view, &state.breakout, colors),
        Some(GameType::Rogue) => draw_rogue(frame, view, &state.rogue, colors),
        Some(GameType::Trek) => draw_trek(frame, view, &state.trek, colors),
        None => {}
    }
}

fn draw_tetris(
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

    // Draw board
    for y in 0..tetris::BOARD_HEIGHT {
        let mut row_content = String::new();

        // Left border
        row_content.push('║');

        for x in 0..tetris::BOARD_WIDTH {
            let cell = if let Some(piece) = &state.current_piece {
                let blocks = piece.blocks();
                if blocks.contains(&(x as i32, y as i32)) {
                    Some(piece.piece_type)
                } else {
                    state.board[y][x]
                }
            } else {
                state.board[y][x]
            };

            if cell.is_some() {
                row_content.push_str("██");
            } else {
                row_content.push_str("  ");
            }
        }

        // Right border
        row_content.push('║');

        view.render_row(
            frame,
            1 + y as u16,
            vec![
                Span::raw(" ".repeat(board_start_x)),
                Span::styled(row_content, Style::default().fg(colors.cyan())),
            ],
        );
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

    let next_piece = super::tetris::Piece::new(state.next_piece);
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

fn draw_snake(frame: &mut Frame, view: &FullScreenView, state: &SnakeState, colors: &ThemeColors) {
    // Score
    view.render_row(
        frame,
        0,
        vec![Span::styled(
            format!("Score: {}", state.score),
            Style::default().fg(colors.green()),
        )],
    );

    // Draw top border
    view.render_row(
        frame,
        1,
        vec![Span::styled(
            format!("╔{}╗", "═".repeat(snake::BOARD_WIDTH)),
            Style::default().fg(colors.cyan()),
        )],
    );

    // Draw board
    for y in 0..snake::BOARD_HEIGHT {
        let mut row_content = String::new();
        row_content.push('║');

        for x in 0..snake::BOARD_WIDTH {
            let pos = Position::new(x as i32, y as i32);

            if state.is_head(pos) {
                row_content.push('@');
            } else if state.is_snake(pos) {
                row_content.push('O');
            } else if pos == state.food {
                row_content.push('*');
            } else {
                row_content.push(' ');
            }
        }

        row_content.push('║');

        let style = Style::default().fg(colors.cyan());
        let content_style = if state.game_over {
            Style::default().fg(colors.red())
        } else {
            Style::default().fg(colors.green())
        };

        // Render border and content separately for colors
        let border_left = "║";
        let content: String = row_content
            .chars()
            .skip(1)
            .take(snake::BOARD_WIDTH)
            .collect();
        let border_right = "║";

        view.render_row(
            frame,
            2 + y as u16,
            vec![
                Span::styled(border_left, style),
                Span::styled(content, content_style),
                Span::styled(border_right, style),
            ],
        );
    }

    // Draw bottom border
    view.render_row(
        frame,
        2 + snake::BOARD_HEIGHT as u16,
        vec![Span::styled(
            format!("╚{}╝", "═".repeat(snake::BOARD_WIDTH)),
            Style::default().fg(colors.cyan()),
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

fn draw_breakout(
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

fn draw_rogue(frame: &mut Frame, view: &FullScreenView, state: &RogueState, colors: &ThemeColors) {
    // Status bar (top row)
    let hunger_status = state.hunger_status();
    let hunger_color = match state.hunger {
        0..=100 => colors.red(),
        101..=300 => colors.red(),
        301..=500 => colors.yellow(),
        _ => colors.green(),
    };

    view.render_row(
        frame,
        0,
        vec![
            Span::styled(
                format!("HP:{}/{}", state.health, state.max_health),
                Style::default().fg(if state.health < state.max_health / 3 {
                    colors.red()
                } else {
                    colors.green()
                }),
            ),
            Span::styled(
                format!("  Str:{}", state.strength),
                Style::default().fg(colors.cyan()),
            ),
            Span::styled(
                format!("  Gold:{}", state.gold),
                Style::default().fg(colors.yellow()),
            ),
            Span::styled(
                format!("  Lv:{}", state.level),
                Style::default().fg(colors.cyan()),
            ),
            Span::styled(
                format!("  Dlv:{}", state.dungeon_level),
                Style::default().fg(colors.blue()),
            ),
            Span::styled(
                format!("  Arm:{}", state.defense),
                Style::default().fg(colors.fg()),
            ),
            if !hunger_status.is_empty() {
                Span::styled(
                    format!("  {}", hunger_status),
                    Style::default().fg(hunger_color),
                )
            } else {
                Span::raw("")
            },
        ],
    );

    // Draw dungeon
    for y in 0..rogue::BOARD_HEIGHT {
        let mut row_spans: Vec<Span> = Vec::new();

        for x in 0..rogue::BOARD_WIDTH {
            let is_visible = state.is_visible(x, y);
            let is_explored = state.explored[y][x];

            // Check for player
            if x == state.player_x && y == state.player_y {
                row_spans.push(Span::styled("@", Style::default().fg(colors.yellow())));
                continue;
            }

            // Check for monsters
            if let Some(monster) = state.monsters.iter().find(|m| m.x == x && m.y == y) {
                if is_visible {
                    let monster_color = match monster.monster_type {
                        rogue::MonsterType::Rat | rogue::MonsterType::Bat => colors.grey(),
                        rogue::MonsterType::Goblin | rogue::MonsterType::Skeleton => colors.green(),
                        rogue::MonsterType::Orc | rogue::MonsterType::Troll => colors.cyan(),
                        rogue::MonsterType::Dragon => colors.red(),
                    };
                    row_spans.push(Span::styled(
                        monster.monster_type.char().to_string(),
                        Style::default().fg(monster_color),
                    ));
                    continue;
                }
            }

            // Check for items
            if let Some(entity) = state.entities.iter().find(|e| e.x == x && e.y == y) {
                if is_visible || is_explored {
                    let item_color = match entity.entity_type {
                        rogue::EntityType::Gold => colors.yellow(),
                        rogue::EntityType::Food => colors.green(),
                        rogue::EntityType::Potion => colors.cyan(),
                        rogue::EntityType::Scroll => colors.fg(),
                        rogue::EntityType::Weapon => colors.blue(),
                        rogue::EntityType::Armor => colors.grey(),
                    };
                    let style = if is_visible {
                        Style::default().fg(item_color)
                    } else {
                        Style::default().fg(item_color).add_modifier(Modifier::DIM)
                    };
                    row_spans.push(Span::styled(entity.entity_type.char().to_string(), style));
                    continue;
                }
            }

            // Draw tile
            let tile = state.board[y][x];
            if is_visible {
                let (ch, color) = match tile {
                    rogue::Tile::Floor | rogue::Tile::Corridor => ('.', colors.grey()),
                    rogue::Tile::Wall => ('#', colors.fg()),
                    rogue::Tile::Door => ('+', colors.yellow()),
                    rogue::Tile::StairsDown => ('%', colors.cyan()),
                    rogue::Tile::StairsUp => ('<', colors.cyan()),
                    rogue::Tile::Trap => ('^', colors.red()),
                    rogue::Tile::HiddenTrap => ('.', colors.grey()), // Looks like floor
                };
                row_spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
            } else if is_explored {
                let ch = tile.char();
                row_spans.push(Span::styled(
                    ch.to_string(),
                    Style::default()
                        .fg(colors.grey())
                        .add_modifier(Modifier::DIM),
                ));
            } else {
                row_spans.push(Span::raw(" "));
            }
        }

        view.render_row(frame, 1 + y as u16, row_spans);
    }

    // Message area
    if let Some(ref msg) = state.message {
        view.render_row(
            frame,
            (1 + rogue::BOARD_HEIGHT) as u16,
            vec![Span::styled(
                msg.clone(),
                Style::default().fg(colors.yellow()),
            )],
        );
    }

    let help = vec![
        ("←↑↓→", "move"),
        ("hjkl", "move"),
        ("yubn", "diag"),
        ("s", "search"),
        (">", "stairs"),
        ("P", "pause"),
        ("Esc", "quit"),
    ];
    view.render_help(frame, help);
}

fn draw_trek(frame: &mut Frame, view: &FullScreenView, state: &TrekState, colors: &ThemeColors) {
    // Status bar
    view.render_row(
        frame,
        0,
        vec![
            Span::styled(
                format!("E:{}", state.energy),
                Style::default().fg(if state.energy < 500 {
                    colors.red()
                } else {
                    colors.green()
                }),
            ),
            Span::styled(
                format!(" S:{}", state.shields),
                Style::default().fg(if state.shields > 0 {
                    colors.cyan()
                } else {
                    colors.grey()
                }),
            ),
            Span::styled(
                format!(" T:{}", state.torpedoes),
                Style::default().fg(if state.torpedoes > 0 {
                    colors.yellow()
                } else {
                    colors.grey()
                }),
            ),
            Span::styled(
                format!(" SD:{:.1}", state.stardate),
                Style::default().fg(colors.fg()),
            ),
            Span::styled(
                format!(" K:{}", state.klingons_remaining),
                Style::default().fg(colors.red()),
            ),
            Span::styled(
                if state.docked { " [DOCKED]" } else { "" }.to_string(),
                Style::default().fg(colors.green()),
            ),
        ],
    );

    // Draw sector grid on the left
    view.render_row(
        frame,
        1,
        vec![Span::styled(
            format!("╔{}╗  Long Range Scan", "═══".repeat(trek::SECTOR_SIZE)),
            Style::default().fg(colors.cyan()),
        )],
    );

    for y in 0..trek::SECTOR_SIZE {
        let mut row_spans: Vec<Span> = vec![Span::styled("║", Style::default().fg(colors.cyan()))];

        for x in 0..trek::SECTOR_SIZE {
            let entity = state.sector[y][x];
            let entity_str = entity.char();
            let entity_color = match entity {
                trek::SectorEntity::Enterprise => colors.green(),
                trek::SectorEntity::Klingon => colors.red(),
                trek::SectorEntity::Starbase => colors.cyan(),
                trek::SectorEntity::Star => colors.yellow(),
                trek::SectorEntity::Empty => colors.grey(),
            };
            row_spans.push(Span::styled(entity_str, Style::default().fg(entity_color)));
        }

        row_spans.push(Span::styled("║", Style::default().fg(colors.cyan())));

        // Long range scan on the right side
        row_spans.push(Span::raw("  "));
        if y < trek::GALAXY_SIZE {
            for x in 0..trek::GALAXY_SIZE {
                let q = &state.galaxy[y][x];
                let is_current = x == state.quadrant_x && y == state.quadrant_y;
                let code = if q.scanned {
                    q.sensor_code()
                } else {
                    "***".to_string()
                };
                let style = if is_current {
                    Style::default()
                        .fg(colors.yellow())
                        .add_modifier(Modifier::BOLD)
                } else if q.klingons > 0 {
                    Style::default().fg(colors.red())
                } else if q.starbases > 0 {
                    Style::default().fg(colors.cyan())
                } else {
                    Style::default().fg(colors.grey())
                };
                row_spans.push(Span::styled(format!("{} ", code), style));
            }
        }

        view.render_row(frame, 2 + y as u16, row_spans);
    }

    // Bottom border of sector
    view.render_row(
        frame,
        2 + trek::SECTOR_SIZE as u16,
        vec![Span::styled(
            format!("╚{}╝", "═══".repeat(trek::SECTOR_SIZE)),
            Style::default().fg(colors.cyan()),
        )],
    );

    // Position info
    view.render_row(
        frame,
        3 + trek::SECTOR_SIZE as u16,
        vec![Span::styled(
            format!(
                "Quadrant {}-{}, Sector {}-{}",
                state.quadrant_x + 1,
                state.quadrant_y + 1,
                state.sector_x + 1,
                state.sector_y + 1
            ),
            Style::default().fg(colors.grey()),
        )],
    );

    // Message/prompt area
    let prompt_row = 4 + trek::SECTOR_SIZE as u16;
    let prompt_style = Style::default().fg(colors.yellow());
    let input_display = format!("{}{}", state.message, state.input_buffer);
    view.render_row(
        frame,
        prompt_row,
        vec![Span::styled(input_display, prompt_style)],
    );

    // Command mode indicator
    let mode_str = match state.mode {
        trek::CommandMode::Main => "",
        trek::CommandMode::Navigation => "[NAV]",
        trek::CommandMode::Phasers => "[PHASER]",
        trek::CommandMode::Torpedoes => "[TORPEDO]",
        trek::CommandMode::Shields => "[SHIELDS]",
        trek::CommandMode::Computer => "[COMPUTER]",
    };
    if !mode_str.is_empty() {
        view.render_row(
            frame,
            prompt_row + 1,
            vec![Span::styled(
                mode_str,
                Style::default()
                    .fg(colors.green())
                    .add_modifier(Modifier::BOLD),
            )],
        );
    }

    let help = vec![
        ("N", "nav"),
        ("S", "scan"),
        ("L", "LRS"),
        ("P", "phasers"),
        ("T", "torpedoes"),
        ("H", "shields"),
        ("C", "computer"),
        ("D", "damage"),
        ("Esc", "quit"),
    ];
    view.render_help(frame, help);
}

fn draw_paused(frame: &mut Frame, view: &FullScreenView, state: &GamesState, colors: &ThemeColors) {
    // Draw the game in background
    draw_game(frame, view, state, colors);

    // Overlay pause message
    let content_height = view.content_height();
    let pause_row = content_height / 2;

    view.render_row(
        frame,
        pause_row,
        vec![Span::styled(
            "       ═══════════════════════       ",
            Style::default().fg(colors.yellow()),
        )],
    );
    view.render_row(
        frame,
        pause_row + 1,
        vec![Span::styled(
            "       ║      P A U S E D      ║       ",
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );
    view.render_row(
        frame,
        pause_row + 2,
        vec![Span::styled(
            "       ═══════════════════════       ",
            Style::default().fg(colors.yellow()),
        )],
    );

    let help = vec![("P", "resume"), ("Esc", "quit")];
    view.render_help(frame, help);
}

fn draw_game_over(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &GamesState,
    colors: &ThemeColors,
) {
    let content_height = view.content_height();
    let center_row = content_height / 2;

    // Check if it's a win (breakout, rogue, or trek)
    let is_win = (matches!(state.current_game, Some(GameType::Breakout))
        && state.breakout.game_won)
        || (matches!(state.current_game, Some(GameType::Rogue)) && state.rogue.game_won)
        || (matches!(state.current_game, Some(GameType::Trek)) && state.trek.game_won);

    let title = if is_win { "YOU WIN!" } else { "GAME OVER" };
    let title_color = if is_win { colors.green() } else { colors.red() };

    view.render_row(
        frame,
        center_row - 2,
        vec![Span::styled(
            "╔═══════════════════════════════════════════╗",
            Style::default().fg(title_color),
        )],
    );
    view.render_row(
        frame,
        center_row - 1,
        vec![Span::styled(
            format!("║{:^43}║", title),
            Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
        )],
    );
    view.render_row(
        frame,
        center_row,
        vec![Span::styled(
            format!("║{:^43}║", format!("Final Score: {}", state.score)),
            Style::default().fg(colors.yellow()),
        )],
    );

    // Check high score
    if let Some(game) = state.current_game {
        let idx = match game {
            GameType::Tetris => 0,
            GameType::Snake => 1,
            GameType::Breakout => 2,
            GameType::Rogue => 3,
            GameType::Trek => 4,
        };
        if state.score >= state.high_scores[idx] && state.score > 0 {
            view.render_row(
                frame,
                center_row + 1,
                vec![Span::styled(
                    format!("║{:^43}║", "NEW HIGH SCORE!"),
                    Style::default()
                        .fg(colors.green())
                        .add_modifier(Modifier::BOLD),
                )],
            );
        } else {
            view.render_row(
                frame,
                center_row + 1,
                vec![Span::styled(
                    format!("║{:^43}║", ""),
                    Style::default().fg(title_color),
                )],
            );
        }
    }

    view.render_row(
        frame,
        center_row + 2,
        vec![Span::styled(
            "╚═══════════════════════════════════════════╝",
            Style::default().fg(title_color),
        )],
    );

    let help = vec![("Enter", "play again"), ("Esc", "menu")];
    view.render_help(frame, help);
}
