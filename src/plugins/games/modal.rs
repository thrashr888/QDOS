//! Games plugin modal rendering

use super::breakout::{self, BreakoutState};
use super::clicker::{Buff, ClickerState, ClickerView, Item, Scenery, ShopItem, SoulUpgrade};
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
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

// =============================================================================
// CLICKER GAME STRINGS & ASCII ART
// =============================================================================

/// Death screen title
const CLICKER_DEATH_TITLE: &str = "☠  Y O U   D I E D  ☠";

/// Castle silhouette - dark and foreboding
const CASTLE_ART: &[&str] = &[
    "                  │▌              │▌              │▌                  ",
    "       ▄█▄       ███▄            ███▄            ███▄       ▄█▄       ",
    "      █████     █████           █████           █████     █████      ",
    "     ███████   ███████    ▄    ███████    ▄    ███████   ███████     ",
    "    ▄███████▄▄█████████▄▄███▄▄█████████▄▄███▄▄█████████▄▄███████▄    ",
    "   ██████████████████████████████████████████████████████████████   ",
    "   ██▓▓██░░██▓▓██░░██▓▓██░░██▓▓██░░██▓▓██░░██▓▓██░░██▓▓██░░██▓▓██   ",
    "   ██████████████████████████████████████████████████████████████   ",
    "   ██  ██  ██  ██  ██  ████████████████████  ██  ██  ██  ██  ██   ",
    "   ██  ██  ██  ██  ██  ██▓▓▓▓▓▓▓▓▓▓▓▓▓▓██  ██  ██  ██  ██  ██   ",
    "   ██████████████████████              ██████████████████████████   ",
];

/// Flag animation frames (waving)
const FLAG_WAVE_1: &str = "▀▄";
const FLAG_WAVE_2: &str = "▄▀";
const FLAG_WAVE_3: &str = "▀▄";
const FLAG_WAVE_4: &str = " ▀";

/// Skull with glowing eyes
const SKULL_SMALL: &[&str] = &[
    "    ▄▄███▄▄    ",
    "   ███●█●███   ",
    "   ██▄███▄██   ",
    "    ▀█▀▀▀█▀    ",
];

/// Rogue with knife ASCII art - fallen hero
const ROGUE_FALLEN: &[&str] = &["      ╪═─  O   ", "        \\ /|\\  ", "          / \\ "];

// =============================================================================
// GAMES MENU ASCII ART
// =============================================================================

/// Large GAMES title - line 1
const GAMES_TITLE_1: &str = " ▄▄▄▄▄  ▄▄▄▄▄  ▄   ▄  ▄▄▄▄▄  ▄▄▄▄▄ ";
/// Large GAMES title - line 2
const GAMES_TITLE_2: &str = "█       █   █  ██ ██  █      █     ";
/// Large GAMES title - line 3
const GAMES_TITLE_3: &str = "█  ▀▀▀  █████  █ █ █  █████  ▀▀▀▀█ ";
/// Large GAMES title - line 4
const GAMES_TITLE_4: &str = "█    █  █   █  █   █  █          █ ";
/// Large GAMES title - line 5
const GAMES_TITLE_5: &str = " ▀▀▀▀   ▀   ▀  ▀   ▀  ▀▀▀▀▀  ▀▀▀▀  ";

/// Decorative separator
const GAMES_SEPARATOR: &str = "─═══════════════════════════════════─";

/// Soul shop title
const CLICKER_SOUL_SHOP_TITLE: &str = "~ S O U L   S H O P ~";

/// Elite monster prefix
const CLICKER_ELITE_PREFIX: &str = "◆ ";

/// Floor boss prefix
const CLICKER_FLOOR_BOSS_PREFIX: &str = "⚔ FLOOR BOSS ⚔ ";

/// Boss prefix
const CLICKER_BOSS_PREFIX: &str = "★ ";

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
            Some(GameType::Clicker) => " Clicker ",
            None => " Games ",
        },
        GamesView::GameOver => " Game Over ",
        GamesView::EnteringInitials => " High Score! ",
        GamesView::Leaderboard => " Leaderboard ",
    };

    let view = FullScreenView::new(area, title, colors);
    view.render_frame(frame);

    match state.view {
        GamesView::Menu => draw_menu(frame, &view, state, colors),
        GamesView::Playing => draw_game(frame, &view, state, colors),
        GamesView::Paused => draw_paused(frame, &view, state, colors),
        GamesView::GameOver => draw_game_over(frame, &view, state, colors),
        GamesView::EnteringInitials => draw_initials_entry(frame, &view, state, colors),
        GamesView::Leaderboard => draw_leaderboard(frame, &view, state, colors),
    }
}

fn draw_menu(frame: &mut Frame, view: &FullScreenView, state: &GamesState, colors: &ThemeColors) {
    // === LARGE COLORFUL "GAMES" TITLE WITH ANIMATION ===
    let title_lines = [
        GAMES_TITLE_1,
        GAMES_TITLE_2,
        GAMES_TITLE_3,
        GAMES_TITLE_4,
        GAMES_TITLE_5,
    ];

    // Animation: wave offset changes over time, color cycles
    let tick = state.menu_tick;
    let wave_offset = (tick / 2) % 20; // Wave moves every 2 ticks
    let color_phase = (tick / 4) % 4; // Color cycles every 4 ticks

    // Color cycle: cyan -> blue -> magenta -> red -> back
    let cycle_colors = [colors.cyan(), colors.blue(), colors.red(), colors.yellow()];

    for (row, line) in title_lines.iter().enumerate() {
        let mut spans: Vec<Span> = Vec::new();

        // Wave effect: shift each row left/right based on sine-like pattern
        let row_wave = match (wave_offset as i32 + row as i32) % 4 {
            0 => 0,
            1 => 1,
            2 => 0,
            3 => -1,
            _ => 0,
        };
        let margin = (2 + row_wave).max(0) as usize;
        spans.push(Span::raw(" ".repeat(margin)));

        for (i, ch) in line.chars().enumerate() {
            let color = if ch == ' ' {
                colors.bg()
            } else {
                // Animated color cycling with position-based offset
                let phase = ((i + tick as usize + row * 3) / 4) % 4;
                let adjusted_phase = (phase + color_phase as usize) % 4;
                cycle_colors[adjusted_phase]
            };

            let style = if ch == '█' || ch == '▄' || ch == '▀' {
                Style::default().fg(color).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(color)
            };

            spans.push(Span::styled(ch.to_string(), style));
        }
        view.render_row(frame, row as u16, spans);
    }

    // === DECORATIVE SEPARATOR ===
    view.render_row(
        frame,
        5,
        vec![
            Span::raw("  "),
            Span::styled(GAMES_SEPARATOR, Style::default().fg(colors.blue())),
        ],
    );

    // === "R-DOS ARCADE" subtitle ===
    view.render_row(
        frame,
        6,
        vec![
            Span::raw("       "),
            Span::styled(
                "▒▓█",
                Style::default()
                    .fg(colors.red())
                    .add_modifier(Modifier::DIM),
            ),
            Span::styled(
                " R-DOS ARCADE ",
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "█▓▒",
                Style::default()
                    .fg(colors.red())
                    .add_modifier(Modifier::DIM),
            ),
        ],
    );

    view.render_row(
        frame,
        7,
        vec![
            Span::raw("  "),
            Span::styled(GAMES_SEPARATOR, Style::default().fg(colors.blue())),
        ],
    );

    // === GAME LIST ===
    let start_row = 9;
    for (i, game) in GameType::all().iter().enumerate() {
        let is_selected = i == state.selected_game;
        let high_score = state.high_scores[i];

        // Number prefix with color
        let num_style = if is_selected {
            Style::default()
                .fg(colors.yellow())
                .bg(colors.red())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.cyan())
        };

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

        let arrow = if is_selected { "►" } else { " " };

        view.render_row(
            frame,
            start_row + (i as u16 * 2),
            vec![
                Span::styled("   ", Style::default()),
                Span::styled(arrow, num_style),
                Span::styled(format!(" {}) ", i + 1), num_style),
                Span::styled(format!("{:<12}", game.name()), name_style),
                Span::styled(format!(" - {}", game.description()), desc_style),
            ],
        );

        if high_score > 0 {
            view.render_row(
                frame,
                start_row + (i as u16 * 2) + 1,
                vec![
                    Span::styled("        ", Style::default()),
                    Span::styled("★ ", Style::default().fg(colors.yellow())),
                    Span::styled(
                        format!("High Score: {}", high_score),
                        Style::default().fg(colors.green()),
                    ),
                ],
            );
        }
    }

    let help = vec![
        ("↑↓/1-6", "select"),
        ("Enter", "play"),
        ("L", "scores"),
        ("Esc", "close"),
    ];
    view.render_help(frame, help);
}

fn draw_game(frame: &mut Frame, view: &FullScreenView, state: &GamesState, colors: &ThemeColors) {
    match state.current_game {
        Some(GameType::Tetris) => draw_tetris(frame, view, &state.tetris, colors),
        Some(GameType::Snake) => draw_snake(frame, view, &state.snake, colors),
        Some(GameType::Breakout) => draw_breakout(frame, view, &state.breakout, colors),
        Some(GameType::Rogue) => draw_rogue(frame, view, &state.rogue, colors),
        Some(GameType::Trek) => draw_trek(frame, view, &state.trek, colors),
        Some(GameType::Clicker) => draw_clicker(frame, view, &state.clicker, colors),
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

            // Draw tile with enhanced atmosphere
            let tile = state.board[y][x];
            if is_visible {
                // Visible area - high contrast, atmospheric
                let (ch, color, modifier) = match tile {
                    rogue::Tile::Floor => ('·', colors.grey(), Modifier::empty()),
                    rogue::Tile::Corridor => ('▒', colors.grey(), Modifier::DIM),
                    rogue::Tile::Wall => ('█', colors.fg(), Modifier::empty()),
                    rogue::Tile::Door => ('+', colors.yellow(), Modifier::BOLD),
                    rogue::Tile::StairsDown => ('▓', colors.cyan(), Modifier::BOLD),
                    rogue::Tile::StairsUp => ('◄', colors.cyan(), Modifier::BOLD),
                    rogue::Tile::Trap => ('^', colors.red(), Modifier::BOLD),
                    rogue::Tile::HiddenTrap => ('·', colors.grey(), Modifier::empty()),
                };
                row_spans.push(Span::styled(
                    ch.to_string(),
                    Style::default().fg(color).add_modifier(modifier),
                ));
            } else if is_explored {
                // Explored but not visible - "memory" with dim shadows
                let ch = match tile {
                    rogue::Tile::Wall => '░', // Walls look like shadows in memory
                    rogue::Tile::Door => '+',
                    rogue::Tile::StairsDown => '▓',
                    rogue::Tile::StairsUp => '◄',
                    _ => ' ', // Floors fade to nothing
                };
                row_spans.push(Span::styled(
                    ch.to_string(),
                    Style::default()
                        .fg(colors.grey())
                        .add_modifier(Modifier::DIM),
                ));
            } else {
                // True darkness
                row_spans.push(Span::raw(" "));
            }
        }

        view.render_row(frame, 1 + y as u16, row_spans);
    }

    // Message area - show recent messages (up to 2 lines)
    let msg_start_row = (1 + rogue::BOARD_HEIGHT) as u16;
    let messages_to_show = state.messages.len().min(2);
    for (i, msg) in state
        .messages
        .iter()
        .rev()
        .take(messages_to_show)
        .rev()
        .enumerate()
    {
        view.render_row(
            frame,
            msg_start_row + i as u16,
            vec![Span::styled(
                msg.clone(),
                Style::default().fg(if i == messages_to_show - 1 {
                    colors.yellow() // Most recent message is bright
                } else {
                    colors.grey() // Older messages are dimmer
                }),
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

fn draw_clicker(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &ClickerState,
    colors: &ThemeColors,
) {
    // Dispatch based on ClickerView
    match state.view {
        ClickerView::Playing => draw_clicker_playing(frame, view, state, colors),
        ClickerView::Dead => draw_clicker_dead(frame, view, state, colors),
        ClickerView::SoulShop => draw_clicker_soul_shop(frame, view, state, colors),
    }
}

fn draw_clicker_dead(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &ClickerState,
    colors: &ThemeColors,
) {
    let mut row = 0u16;

    // === DARK SKY WITH BLOOD RED GRADIENT ===
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░",
            Style::default()
                .fg(colors.red())
                .add_modifier(Modifier::DIM),
        )],
    );
    row += 1;

    // === CASTLE WITH ANIMATED FLAGS ===
    // Flag animation based on tick
    let flag_frame = (state.tick / 3) % 4;
    let _flag = match flag_frame {
        0 => FLAG_WAVE_1,
        1 => FLAG_WAVE_2,
        2 => FLAG_WAVE_3,
        _ => FLAG_WAVE_4,
    };

    // Draw castle with colored elements and animated flag color
    let flag_color = if flag_frame.is_multiple_of(2) {
        colors.red()
    } else {
        colors.yellow()
    };

    for (i, line) in CASTLE_ART.iter().enumerate() {
        let mut spans: Vec<Span> = Vec::new();

        for ch in line.chars() {
            let (color, modifier) = match ch {
                // Flag poles and flags - animated colors
                '│' | '▌' => {
                    if i == 0 {
                        // Flag pole top - use animated color
                        (flag_color, Modifier::BOLD)
                    } else {
                        (colors.grey(), Modifier::empty())
                    }
                }
                // Castle structure
                '█' | '▄' => (colors.grey(), Modifier::DIM),
                '▓' => (colors.yellow(), Modifier::DIM), // Lit windows
                '░' => (colors.grey(), Modifier::DIM),   // Darker stone
                _ => (colors.grey(), Modifier::DIM),
            };
            spans.push(Span::styled(
                ch.to_string(),
                Style::default().fg(color).add_modifier(modifier),
            ));
        }
        view.render_row(frame, row, spans);
        row += 1;
    }

    // === SKULL IN CENTER ===
    for line in SKULL_SMALL.iter() {
        let mut spans: Vec<Span> = Vec::new();
        for ch in line.chars() {
            let color = match ch {
                '●' => colors.red(), // Glowing red eyes
                '█' | '▄' | '▀' => colors.fg(),
                _ => colors.grey(),
            };
            spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
        }
        view.render_row(frame, row, spans);
        row += 1;
    }

    // === FALLEN ROGUE ===
    for line in ROGUE_FALLEN.iter() {
        let mut spans: Vec<Span> = Vec::new();
        for ch in line.chars() {
            let color = match ch {
                'O' => colors.yellow(),
                '/' | '\\' | '|' => colors.cyan(),
                '─' | '═' | '╪' => colors.fg(),
                _ => colors.grey(),
            };
            spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
        }
        view.render_row(frame, row, spans);
        row += 1;
    }

    let center_row = row + 1;

    // Death title
    view.render_row(
        frame,
        center_row - 1,
        vec![Span::styled(
            "╔══════════════════════════════════════════════════╗",
            Style::default().fg(colors.red()),
        )],
    );
    view.render_row(
        frame,
        center_row,
        vec![Span::styled(
            format!("║{:^50}║", CLICKER_DEATH_TITLE),
            Style::default()
                .fg(colors.red())
                .add_modifier(Modifier::BOLD),
        )],
    );
    view.render_row(
        frame,
        center_row + 1,
        vec![Span::styled(
            "╠══════════════════════════════════════════════════╣",
            Style::default().fg(colors.red()),
        )],
    );

    // Run stats
    view.render_row(
        frame,
        center_row + 2,
        vec![Span::styled(
            format!("║  {:.<23} {:>22}  ║", "Floor Reached", state.dungeon_floor),
            Style::default().fg(colors.fg()),
        )],
    );
    view.render_row(
        frame,
        center_row + 3,
        vec![Span::styled(
            format!("║  {:.<23} {:>22}  ║", "Level Reached", state.level),
            Style::default().fg(colors.fg()),
        )],
    );
    view.render_row(
        frame,
        center_row + 4,
        vec![Span::styled(
            format!(
                "║  {:.<23} {:>22}  ║",
                "Monsters Slain", state.monsters_killed
            ),
            Style::default().fg(colors.fg()),
        )],
    );
    view.render_row(
        frame,
        center_row + 5,
        vec![Span::styled(
            format!(
                "║  {:.<23} {:>22}  ║",
                "Bosses Defeated", state.bosses_killed
            ),
            Style::default().fg(colors.fg()),
        )],
    );
    view.render_row(
        frame,
        center_row + 6,
        vec![Span::styled(
            format!(
                "║  {:.<23} {:>22}  ║",
                "Gold Earned", state.total_gold_earned
            ),
            Style::default().fg(colors.yellow()),
        )],
    );

    // Souls earned - the big reward!
    view.render_row(
        frame,
        center_row + 7,
        vec![Span::styled(
            format!("║{:^50}║", "────────────────────────────"),
            Style::default().fg(colors.red()),
        )],
    );
    view.render_row(
        frame,
        center_row + 8,
        vec![Span::styled(
            format!(
                "║{:^50}║",
                format!("⚔ SOULS EARNED: {} ⚔", state.souls.souls_earned_this_run)
            ),
            Style::default()
                .fg(colors.cyan())
                .add_modifier(Modifier::BOLD),
        )],
    );
    view.render_row(
        frame,
        center_row + 9,
        vec![Span::styled(
            format!(
                "║{:^50}║",
                format!("Total Souls: {}", state.souls.total_souls)
            ),
            Style::default().fg(colors.cyan()),
        )],
    );

    view.render_row(
        frame,
        center_row + 10,
        vec![Span::styled(
            "╚══════════════════════════════════════════════════╝",
            Style::default().fg(colors.red()),
        )],
    );

    let help = vec![
        ("Enter/r", "new run"),
        ("s/Tab", "soul shop"),
        ("Esc", "menu"),
    ];
    view.render_help(frame, help);
}

fn draw_clicker_soul_shop(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &ClickerState,
    colors: &ThemeColors,
) {
    // Header
    view.render_row(
        frame,
        1,
        vec![Span::styled(
            "╔════════════════════════════════════════════════════════════════════════╗",
            Style::default().fg(colors.cyan()),
        )],
    );
    view.render_row(
        frame,
        2,
        vec![Span::styled(
            format!("║{:^72}║", CLICKER_SOUL_SHOP_TITLE),
            Style::default()
                .fg(colors.cyan())
                .add_modifier(Modifier::BOLD),
        )],
    );
    view.render_row(
        frame,
        3,
        vec![Span::styled(
            format!("║{:^72}║", format!("Souls: {}", state.souls.total_souls)),
            Style::default().fg(colors.yellow()),
        )],
    );
    view.render_row(
        frame,
        4,
        vec![Span::styled(
            "╠════════════════════════════════════════════════════════════════════════╣",
            Style::default().fg(colors.cyan()),
        )],
    );

    // Upgrades list
    for (i, upgrade) in SoulUpgrade::all().iter().enumerate() {
        let is_selected = i == state.soul_shop_selected;
        let current_level = state.souls.upgrade_level(*upgrade);
        let max_level = upgrade.max_level();
        let cost = state.souls.upgrade_cost(*upgrade);
        let can_afford = state.souls.can_afford(*upgrade);
        let is_maxed = current_level >= max_level;

        let prefix = if is_selected { "►" } else { " " };
        let level_str = if is_maxed {
            "MAX".to_string()
        } else {
            format!("Lv.{}", current_level)
        };
        let cost_str = if is_maxed {
            "---".to_string()
        } else {
            format!("{} souls", cost)
        };

        let style = if is_maxed {
            Style::default().fg(colors.grey())
        } else if is_selected {
            Style::default()
                .fg(colors.yellow())
                .bg(colors.red())
                .add_modifier(Modifier::BOLD)
        } else if can_afford {
            Style::default().fg(colors.green())
        } else {
            Style::default().fg(colors.grey())
        };

        let line = format!(
            "║ {} {:<18} {:>6}  {:<30} {:>10} ║",
            prefix,
            upgrade.name(),
            level_str,
            upgrade.description(),
            cost_str
        );
        view.render_row(frame, 5 + i as u16, vec![Span::styled(line, style)]);
    }

    // Bottom border
    let footer_row = 5 + SoulUpgrade::all().len() as u16;
    view.render_row(
        frame,
        footer_row,
        vec![Span::styled(
            "╚════════════════════════════════════════════════════════════════════════╝",
            Style::default().fg(colors.cyan()),
        )],
    );

    // Soul bonuses summary
    view.render_row(
        frame,
        footer_row + 2,
        vec![
            Span::styled("Current Bonuses: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("STR+{} ", state.souls.starting_str),
                Style::default().fg(colors.cyan()),
            ),
            Span::styled(
                format!("ARM+{} ", state.souls.starting_arm),
                Style::default().fg(colors.blue()),
            ),
            Span::styled(
                format!("HP+{} ", state.souls.starting_hp * 10),
                Style::default().fg(colors.green()),
            ),
            Span::styled(
                format!("Gold+{} ", state.souls.starting_gold * 50),
                Style::default().fg(colors.yellow()),
            ),
            Span::styled(
                format!("Speed+{}% ", state.souls.attack_speed * 10),
                Style::default().fg(colors.red()),
            ),
        ],
    );

    view.render_row(
        frame,
        footer_row + 3,
        vec![
            Span::styled("               ", Style::default()),
            Span::styled(
                format!("Crit×{:.1} ", state.souls.crit_damage_multiplier()),
                Style::default().fg(colors.yellow()),
            ),
            Span::styled(
                format!("Gold+{}% ", state.souls.soul_gold_multiplier()),
                Style::default().fg(colors.yellow()),
            ),
            Span::styled(
                format!("Drop+{}% ", state.souls.soul_drop_bonus()),
                Style::default().fg(colors.cyan()),
            ),
            Span::styled(
                format!("Floor+{}", state.souls.floor_skip),
                Style::default().fg(colors.blue()),
            ),
        ],
    );

    let help = if state.game_over {
        vec![
            ("↑↓", "select"),
            ("Enter/b", "buy"),
            ("r", "new run"),
            ("Esc", "back"),
        ]
    } else {
        vec![("↑↓", "select"), ("Enter/b", "buy"), ("Esc", "back")]
    };
    view.render_help(frame, help);
}

fn draw_clicker_playing(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &ClickerState,
    colors: &ThemeColors,
) {
    // Helper to get scenery color
    let scenery_color = |s: &Scenery| match s.color_idx {
        1 => colors.yellow(),
        2 => colors.cyan(),
        3 => colors.red(),
        4 => colors.green(),
        _ => colors.grey(),
    };

    // Calculate layout - corridor on left, shop on right
    let content_width = view.area.width as usize;
    let shop_width = 26;
    let corridor_width = content_width.saturating_sub(shop_width + 2);

    // === TOP STATUS BAR ===
    let total_str = state.total_strength();
    let total_arm = state.total_armor();

    // Calculate STR buff amount for display
    let str_buff: i32 = state
        .buffs
        .iter()
        .map(|b| match b {
            Buff::Strength(amt, _) => *amt,
            _ => 0,
        })
        .sum();

    // Calculate ARM gear bonus for display
    let base_arm = state.armor + state.armor_equip.as_ref().map_or(0, |a| a.bonus);
    let arm_gear_bonus = total_arm - base_arm;

    // First row: main stats
    view.render_row(
        frame,
        0,
        vec![
            Span::styled(
                format!("HP:{}/{}", state.hp, state.max_hp),
                Style::default().fg(if state.hp < state.max_hp / 3 {
                    colors.red()
                } else {
                    colors.green()
                }),
            ),
            Span::styled(
                if str_buff > 0 {
                    format!(" STR:{}+{}", total_str - str_buff, str_buff)
                } else {
                    format!(" STR:{}", total_str)
                },
                Style::default().fg(if str_buff > 0 {
                    colors.yellow() // Highlight when buffed
                } else {
                    colors.cyan()
                }),
            ),
            Span::styled(
                if arm_gear_bonus > 0 {
                    format!(" ARM:{}+{}", base_arm, arm_gear_bonus)
                } else {
                    format!(" ARM:{}", total_arm)
                },
                Style::default().fg(if arm_gear_bonus > 0 {
                    colors.cyan() // Highlight when has gear bonuses
                } else {
                    colors.blue()
                }),
            ),
            Span::styled(
                format!(" Gold:{}", state.gold),
                Style::default().fg(colors.yellow()),
            ),
            Span::styled(
                format!(" Lv:{}", state.level),
                Style::default().fg(colors.cyan()),
            ),
            Span::styled(
                format!(" XP:{}/{}", state.xp, state.xp_for_level()),
                Style::default().fg(colors.green()),
            ),
            Span::styled(
                format!(" Food:{}", state.food),
                Style::default().fg(if state.food < 5 {
                    colors.red()
                } else {
                    colors.fg()
                }),
            ),
        ],
    );

    // Second row: biome, floor, class, alchemy, souls, dust
    let class_name = state.souls.selected_class.name();
    let alchemy_tier = state.alchemy_tier();
    let biome_name = state.biome.name();

    view.render_row(
        frame,
        1,
        vec![
            Span::styled(
                format!("Floor:{}", state.dungeon_floor),
                Style::default().fg(colors.cyan()),
            ),
            Span::styled(
                format!(" {}", biome_name),
                Style::default()
                    .fg(match state.biome.color_idx() {
                        3 => colors.red(),
                        4 => colors.green(),
                        5 => colors.blue(),
                        _ => colors.grey(),
                    })
                    .add_modifier(Modifier::DIM),
            ),
            if class_name != "Peasant" {
                Span::styled(
                    format!(" [{}]", class_name),
                    Style::default().fg(colors.yellow()),
                )
            } else {
                Span::styled("", Style::default())
            },
            if state.souls.alchemy_level > 0 {
                Span::styled(
                    format!(" Alch:{}", alchemy_tier.name()),
                    Style::default().fg(colors.green()),
                )
            } else {
                Span::styled("", Style::default())
            },
            if state.souls.total_souls > 0 {
                Span::styled(
                    format!(" Souls:{}", state.souls.total_souls),
                    Style::default().fg(colors.cyan()),
                )
            } else {
                Span::styled("", Style::default())
            },
            if state.souls.dust > 0 {
                Span::styled(
                    format!(" Dust:{}", state.souls.dust),
                    Style::default().fg(colors.blue()),
                )
            } else {
                Span::styled("", Style::default())
            },
            // Monster Zoo event indicator
            if state.zoo_event.active {
                Span::styled(
                    format!(
                        " ZOO! {}left {:0.1}s",
                        state.zoo_event.monsters_remaining,
                        state.zoo_event.time_remaining as f32 / 20.0
                    ),
                    Style::default()
                        .fg(colors.red())
                        .add_modifier(Modifier::BOLD | Modifier::RAPID_BLINK),
                )
            } else {
                Span::styled("", Style::default())
            },
        ],
    );

    // === DUNGEON CORRIDOR (full width minus shop) ===
    let corridor_border = "═".repeat(corridor_width.saturating_sub(2));
    view.render_row(
        frame,
        2,
        vec![Span::styled(
            format!("╔{}╗", corridor_border),
            Style::default().fg(colors.grey()),
        )],
    );

    // Floor with colored scenery, player, and monster
    let mut floor_spans: Vec<Span> = vec![Span::styled("║", Style::default().fg(colors.grey()))];

    let player_pos = 8;
    let monster_pos = corridor_width / 2;

    for (i, scenery) in state
        .floor
        .iter()
        .take(corridor_width.saturating_sub(2))
        .enumerate()
    {
        if i == player_pos {
            floor_spans.push(Span::styled("@", Style::default().fg(colors.yellow())));
        } else if i == monster_pos {
            if let Some(ref monster) = state.current_monster {
                let monster_style = if monster.is_boss {
                    Style::default()
                        .fg(colors.red())
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(colors.red())
                };
                floor_spans.push(Span::styled(monster.char.to_string(), monster_style));
            } else {
                floor_spans.push(Span::styled(
                    scenery.char.to_string(),
                    Style::default().fg(scenery_color(scenery)),
                ));
            }
        } else {
            floor_spans.push(Span::styled(
                scenery.char.to_string(),
                Style::default().fg(scenery_color(scenery)),
            ));
        }
    }
    floor_spans.push(Span::styled("║", Style::default().fg(colors.grey())));

    view.render_row(frame, 3, floor_spans);

    view.render_row(
        frame,
        4,
        vec![Span::styled(
            format!("╚{}╝", corridor_border),
            Style::default().fg(colors.grey()),
        )],
    );

    // === MONSTER INFO ===
    if let Some(ref monster) = state.current_monster {
        // Determine prefix based on monster type (uses constants for easy editing)
        let prefix = if monster.is_floor_boss {
            CLICKER_FLOOR_BOSS_PREFIX
        } else if monster.is_boss {
            CLICKER_BOSS_PREFIX
        } else if !monster.affixes.is_empty() {
            CLICKER_ELITE_PREFIX // Elite indicator
        } else {
            ""
        };

        // Determine color based on monster type
        let name_color = if monster.is_floor_boss {
            colors.red()
        } else if monster.is_boss {
            colors.red()
        } else if !monster.affixes.is_empty() {
            colors.yellow() // Elite = yellow
        } else {
            colors.fg()
        };

        view.render_row(
            frame,
            6,
            vec![
                Span::styled(
                    format!("{}Enemy: {} ", prefix, monster.name),
                    Style::default().fg(name_color).add_modifier(
                        if monster.is_boss || !monster.affixes.is_empty() {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        },
                    ),
                ),
                Span::styled(
                    format!("HP:{}/{}", monster.hp.max(0), monster.max_hp),
                    Style::default().fg(if monster.hp > monster.max_hp / 2 {
                        colors.green()
                    } else if monster.hp > 0 {
                        colors.yellow()
                    } else {
                        colors.red()
                    }),
                ),
                Span::styled(
                    format!("  Next boss: {} kills", state.kills_until_boss),
                    Style::default().fg(colors.grey()),
                ),
            ],
        );

        view.render_row(
            frame,
            7,
            vec![Span::styled(
                monster.description.clone(),
                Style::default().fg(colors.grey()),
            )],
        );

        // HP bar
        let bar_width = 30;
        let hp_pct = (monster.hp.max(0) as f32 / monster.max_hp as f32 * bar_width as f32) as usize;
        let hp_bar = "█".repeat(hp_pct) + &"░".repeat(bar_width - hp_pct);
        view.render_row(
            frame,
            8,
            vec![Span::styled(
                format!("[{}]", hp_bar),
                Style::default().fg(if monster.is_boss {
                    colors.red()
                } else {
                    colors.cyan()
                }),
            )],
        );
    }

    // === MESSAGE AREA ===
    if let Some(ref msg) = state.message {
        let msg_style = if state.last_crit {
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.yellow())
        };
        view.render_row(frame, 10, vec![Span::styled(msg.clone(), msg_style)]);
    }

    // === EQUIPMENT & STATS ===
    let weapon_str = state
        .weapon
        .as_ref()
        .map_or("None".to_string(), |w| w.name.clone());
    let armor_str = state
        .armor_equip
        .as_ref()
        .map_or("None".to_string(), |a| a.name.clone());

    view.render_row(
        frame,
        12,
        vec![
            Span::styled("Weapon: ", Style::default().fg(colors.grey())),
            Span::styled(weapon_str, Style::default().fg(colors.cyan())),
            Span::styled("  Armor: ", Style::default().fg(colors.grey())),
            Span::styled(armor_str, Style::default().fg(colors.blue())),
        ],
    );

    // New gear slots (helm, amulet, cloak, gloves, boots, shield)
    let helm_str = state
        .helm
        .as_ref()
        .map_or("-".to_string(), |g| g.name.clone());
    let amulet_str = state
        .amulet
        .as_ref()
        .map_or("-".to_string(), |g| g.name.clone());
    let cloak_str = state
        .cloak
        .as_ref()
        .map_or("-".to_string(), |g| g.name.clone());
    let gloves_str = state
        .gloves
        .as_ref()
        .map_or("-".to_string(), |g| g.name.clone());
    let boots_str = state
        .boots
        .as_ref()
        .map_or("-".to_string(), |g| g.name.clone());
    let shield_str = state
        .shield
        .as_ref()
        .map_or("-".to_string(), |g| g.name.clone());

    // Only show if player has any gear equipped
    let has_gear = state.helm.is_some()
        || state.amulet.is_some()
        || state.cloak.is_some()
        || state.gloves.is_some()
        || state.boots.is_some()
        || state.shield.is_some();

    if has_gear {
        // Truncate names to fit
        let truncate = |s: String| -> String {
            if s.len() > 12 {
                format!("{}...", &s[..9])
            } else {
                s
            }
        };
        view.render_row(
            frame,
            18,
            vec![
                Span::styled("Gear: ", Style::default().fg(colors.grey())),
                Span::styled(
                    format!("H:{} ", truncate(helm_str)),
                    Style::default().fg(colors.cyan()),
                ),
                Span::styled(
                    format!("A:{} ", truncate(amulet_str)),
                    Style::default().fg(colors.yellow()),
                ),
                Span::styled(
                    format!("C:{} ", truncate(cloak_str)),
                    Style::default().fg(colors.blue()),
                ),
            ],
        );
        view.render_row(
            frame,
            19,
            vec![
                Span::styled("      ", Style::default()),
                Span::styled(
                    format!("G:{} ", truncate(gloves_str)),
                    Style::default().fg(colors.red()),
                ),
                Span::styled(
                    format!("B:{} ", truncate(boots_str)),
                    Style::default().fg(colors.green()),
                ),
                Span::styled(
                    format!("S:{}", truncate(shield_str)),
                    Style::default().fg(colors.cyan()),
                ),
            ],
        );
    }

    // Status indicators - Floor, Level, Kills
    let stairs_indicator = if state.stairs_available { " [%]" } else { "" };
    let mut status_spans = vec![
        Span::styled(
            format!("Floor:{}", state.dungeon_floor),
            Style::default().fg(colors.blue()),
        ),
        Span::styled(
            format!("  Lv:{}", state.dungeon_level),
            Style::default().fg(colors.cyan()),
        ),
        Span::styled(
            format!("  Kills:{}", state.monsters_killed),
            Style::default().fg(colors.grey()),
        ),
    ];
    if state.stairs_available {
        status_spans.push(Span::styled(
            stairs_indicator,
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        ));
    }
    view.render_row(frame, 13, status_spans);

    // Rings and active buffs
    let mut ring_spans = vec![Span::styled("Rings: ", Style::default().fg(colors.grey()))];
    for (i, ring) in state.ring_slots.iter().enumerate() {
        if i > 0 {
            ring_spans.push(Span::styled(" ", Style::default()));
        }
        match ring {
            Some(r) => ring_spans.push(Span::styled(
                format!("={}", r.name()),
                Style::default().fg(colors.cyan()),
            )),
            None => ring_spans.push(Span::styled("=none", Style::default().fg(colors.grey()))),
        }
    }
    // Add active buffs
    if !state.buffs.is_empty() {
        ring_spans.push(Span::styled("  Buffs:", Style::default().fg(colors.grey())));
        for buff in &state.buffs {
            let buff_str = match buff {
                Buff::Strength(amt, _) => format!(" STR+{}", amt),
                Buff::Speed(_) => " FAST".to_string(),
                Buff::GoldRush(k) => format!(" GOLD({})", k),
                Buff::IceSlow(_) => " ICE".to_string(),
            };
            ring_spans.push(Span::styled(
                buff_str,
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            ));
        }
    }
    view.render_row(frame, 14, ring_spans);

    // Inventory display
    let mut inv_spans = vec![Span::styled("Pack: ", Style::default().fg(colors.grey()))];
    for (i, item) in state.inventory.iter().enumerate() {
        let is_selected = i == state.inv_selected;
        let item_char = item.char();
        let style = if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            match item {
                Item::Potion(_) => Style::default().fg(colors.green()),
                Item::Scroll(_) => Style::default().fg(colors.cyan()),
                Item::Ring(_) => Style::default().fg(colors.blue()),
                Item::Wand(_, _) => Style::default().fg(colors.yellow()),
            }
        };
        inv_spans.push(Span::styled(format!("{}{}", i + 1, item_char), style));
        inv_spans.push(Span::styled(" ", Style::default()));
    }
    // Show empty slots
    for i in state.inventory.len()..8 {
        inv_spans.push(Span::styled(
            format!("{}.", i + 1),
            Style::default().fg(colors.grey()),
        ));
        inv_spans.push(Span::styled(" ", Style::default()));
    }
    view.render_row(frame, 15, inv_spans);

    // Selected item description
    if !state.inventory.is_empty() && state.inv_selected < state.inventory.len() {
        let sel_item = &state.inventory[state.inv_selected];
        view.render_row(
            frame,
            16,
            vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    format!("{}: {}", sel_item.name(), sel_item.description()),
                    Style::default().fg(colors.grey()),
                ),
            ],
        );
    }

    // Auto modes
    let mut auto_spans = Vec::new();
    if state.auto_attack {
        auto_spans.push(Span::styled(
            "[AUTO-HIT]",
            Style::default().fg(colors.green()),
        ));
    }
    if state.auto_eat {
        auto_spans.push(Span::styled(
            format!(" [AUTO-EAT@{}%]", state.auto_eat_threshold),
            Style::default().fg(colors.cyan()),
        ));
    }
    if state.auto_quaff {
        auto_spans.push(Span::styled(
            " [AUTO-QUAFF]",
            Style::default().fg(colors.yellow()),
        ));
    }
    if state.auto_equip {
        auto_spans.push(Span::styled(
            " [AUTO-EQUIP]",
            Style::default().fg(colors.blue()),
        ));
    }
    // Show combat lanes if > 1
    if state.combat_lanes > 1 {
        auto_spans.push(Span::styled(
            format!(" [LANES:{}]", state.combat_lanes),
            Style::default()
                .fg(colors.red())
                .add_modifier(Modifier::BOLD),
        ));
    }
    if !auto_spans.is_empty() {
        view.render_row(frame, 17, auto_spans);
    }

    // === SHOP (always visible on right side) ===
    let shop_x = (corridor_width + 2) as u16;
    let content_y = view.content_start_y();
    let shop_header = format!("{:^24}", "═══ SHOP ═══");
    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            shop_header,
            Style::default().fg(colors.yellow()),
        )])),
        Rect::new(shop_x, content_y + 1, shop_width as u16, 1),
    );

    // Shop items
    for (i, item) in ShopItem::all().iter().enumerate() {
        let is_selected = i == state.shop_selected;
        let cost = state.item_cost(*item);
        let can_afford = state.can_afford(*item);
        let is_maxed = state.is_maxed(*item);

        let prefix = if is_selected { "►" } else { " " };

        let style = if is_maxed {
            Style::default().fg(colors.grey())
        } else if is_selected {
            Style::default()
                .fg(colors.yellow())
                .bg(colors.red())
                .add_modifier(Modifier::BOLD)
        } else if can_afford {
            Style::default().fg(colors.green())
        } else {
            Style::default().fg(colors.grey())
        };

        let cost_str = if is_maxed {
            "MAX".to_string()
        } else {
            format!("{}g", cost)
        };

        let item_line = format!("{}{:<14}{:>5}", prefix, item.name(), cost_str);
        frame.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(item_line, style)])),
            Rect::new(shop_x, content_y + 2 + i as u16, shop_width as u16, 1),
        );
    }

    // Shop footer with description
    let selected_item = state.selected_shop_item();
    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            selected_item.description(),
            Style::default().fg(colors.grey()),
        )])),
        Rect::new(
            shop_x,
            content_y + 2 + ShopItem::all().len() as u16 + 1,
            shop_width as u16,
            1,
        ),
    );

    let help = vec![
        ("Space", "hit"),
        ("e", "eat"),
        ("1-8/q", "use"),
        (">", "stairs"),
        ("b", "buy"),
        ("s", "souls"),
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
            GameType::Clicker => 5,
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

    let help = vec![
        ("Enter", "play again"),
        ("L", "leaderboard"),
        ("Esc", "menu"),
    ];
    view.render_help(frame, help);
}

/// Draw the initials entry screen for high scores
fn draw_initials_entry(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &GamesState,
    colors: &ThemeColors,
) {
    let center_row = 6u16;
    let title_color = colors.yellow();

    // Celebratory header
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
            "║       ★  N E W   H I G H   S C O R E  ★       ║",
            Style::default()
                .fg(colors.green())
                .add_modifier(Modifier::BOLD),
        )],
    );
    view.render_row(
        frame,
        center_row,
        vec![Span::styled(
            "╠═══════════════════════════════════════════╣",
            Style::default().fg(title_color),
        )],
    );

    // Score display
    view.render_row(
        frame,
        center_row + 1,
        vec![Span::styled(
            format!("║{:^43}║", format!("Score: {}", state.score)),
            Style::default().fg(colors.cyan()),
        )],
    );

    // Initials entry
    view.render_row(
        frame,
        center_row + 3,
        vec![Span::styled(
            format!("║{:^43}║", "Enter your initials:"),
            Style::default().fg(colors.fg()),
        )],
    );

    // Draw the 3-character entry with cursor
    let chars: Vec<char> = state.initials_buffer.chars().collect();
    let mut initials_spans: Vec<Span> = vec![Span::styled(
        "║                    ",
        Style::default().fg(title_color),
    )];

    for (i, ch) in chars.iter().enumerate() {
        let style = if i == state.initials_cursor {
            Style::default()
                .fg(colors.yellow())
                .bg(colors.red())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(colors.cyan())
                .add_modifier(Modifier::BOLD)
        };
        initials_spans.push(Span::styled(format!(" {} ", ch), style));
    }

    initials_spans.push(Span::styled(
        "                    ║",
        Style::default().fg(title_color),
    ));

    view.render_row(frame, center_row + 5, initials_spans);

    // Instructions
    view.render_row(
        frame,
        center_row + 7,
        vec![Span::styled(
            format!("║{:^43}║", "←→ move   ↑↓ change letter"),
            Style::default().fg(colors.grey()),
        )],
    );

    view.render_row(
        frame,
        center_row + 8,
        vec![Span::styled(
            "╚═══════════════════════════════════════════╝",
            Style::default().fg(title_color),
        )],
    );

    let help = vec![("←→", "move"), ("↑↓", "change"), ("Enter", "confirm")];
    view.render_help(frame, help);
}

/// Draw the leaderboard view
fn draw_leaderboard(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &GamesState,
    colors: &ThemeColors,
) {
    let game = state.leaderboard_game.unwrap_or(state.selected_game_type());
    let leaderboard = state.leaderboards.get(game);
    let title_color = colors.cyan();

    // Game selector tabs at top
    let games = GameType::all();
    let mut tab_spans = Vec::new();
    tab_spans.push(Span::raw("  "));
    for g in games {
        let is_selected = *g == game;
        let style = if is_selected {
            Style::default()
                .fg(colors.yellow())
                .bg(colors.blue())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.grey())
        };
        let name = match g {
            GameType::Tetris => "TET",
            GameType::Snake => "SNK",
            GameType::Breakout => "BRK",
            GameType::Rogue => "ROG",
            GameType::Trek => "TRK",
            GameType::Clicker => "CLK",
        };
        tab_spans.push(Span::styled(format!(" {} ", name), style));
    }
    view.render_row(frame, 0, tab_spans);

    // Header
    view.render_row(
        frame,
        1,
        vec![Span::styled(
            "╔═══════════════════════════════════════════════════╗",
            Style::default().fg(title_color),
        )],
    );
    view.render_row(
        frame,
        2,
        vec![Span::styled(
            format!("║{:^51}║", format!("{} Leaderboard", game.name())),
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );
    view.render_row(
        frame,
        3,
        vec![Span::styled(
            "╠═══════════════════════════════════════════════════╣",
            Style::default().fg(title_color),
        )],
    );

    // Special view for Clicker - show stats instead of just leaderboard
    if game == GameType::Clicker {
        let souls = &state.clicker.souls;

        // Stats header
        view.render_row(
            frame,
            4,
            vec![Span::styled(
                "║                   SOUL STATISTICS                   ║",
                Style::default().fg(colors.cyan()),
            )],
        );
        view.render_row(
            frame,
            5,
            vec![Span::styled(
                "╠═══════════════════════════════════════════════════╣",
                Style::default().fg(title_color),
            )],
        );

        // Stats
        let stats = [
            ("Total Souls", format!("{}", souls.total_souls)),
            ("Total Runs", format!("{}", souls.total_runs)),
            ("Total Deaths", format!("{}", souls.total_deaths)),
            ("Best Floor", format!("{}", souls.best_floor)),
            (
                "Monsters Killed",
                format!("{}", souls.total_monsters_killed),
            ),
            ("Gold Earned", format!("{}", souls.total_gold_earned)),
            ("Zoo Cleared", format!("{}", souls.total_zoo_cleared)),
            ("Arcane Dust", format!("{}", souls.dust)),
            ("Alchemy Level", format!("{}", souls.alchemy_level)),
        ];

        for (i, (label, value)) in stats.iter().enumerate() {
            let row = 6 + i as u16;
            let style = Style::default().fg(if i % 2 == 0 {
                colors.fg()
            } else {
                colors.grey()
            });
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!("║  {:<20}  {:>24}  ║", label, value),
                    style,
                )],
            );
        }

        // Heirloom info
        if let Some(ref heirloom) = souls.heirloom {
            view.render_row(
                frame,
                14,
                vec![Span::styled(
                    format!(
                        "║  Heirloom: {:<36}  ║",
                        format!(
                            "{} (STR+{} CRIT+{}% LS+{}%)",
                            heirloom.name,
                            heirloom.str_bonus,
                            heirloom.crit_bonus,
                            heirloom.life_steal_bonus
                        )
                    ),
                    Style::default()
                        .fg(colors.yellow())
                        .add_modifier(Modifier::BOLD),
                )],
            );
        } else {
            view.render_row(
                frame,
                14,
                vec![Span::styled(
                    "║  Heirloom: None                                   ║",
                    Style::default().fg(colors.grey()),
                )],
            );
        }

        // Show top 3 scores at bottom
        view.render_row(
            frame,
            15,
            vec![Span::styled(
                "╠═══════════════════════════════════════════════════╣",
                Style::default().fg(title_color),
            )],
        );

        let top3: String = leaderboard
            .entries
            .iter()
            .take(3)
            .enumerate()
            .map(|(i, e)| {
                let medal = match i {
                    0 => "🥇",
                    1 => "🥈",
                    _ => "🥉",
                };
                format!("{}{} {}", medal, e.initials, e.score)
            })
            .collect::<Vec<_>>()
            .join("  ");

        view.render_row(
            frame,
            16,
            vec![Span::styled(
                format!("║  Top: {:<42}  ║", top3),
                Style::default().fg(colors.green()),
            )],
        );
    } else {
        // Standard leaderboard for other games
        // Column headers
        view.render_row(
            frame,
            4,
            vec![Span::styled(
                "║  Rank   Initials              Score              ║",
                Style::default().fg(colors.grey()),
            )],
        );
        view.render_row(
            frame,
            5,
            vec![Span::styled(
                "║  ────   ────────              ─────              ║",
                Style::default().fg(colors.grey()),
            )],
        );

        // Entries
        for i in 0..10 {
            let row = 6 + i as u16;
            if let Some(entry) = leaderboard.entries.get(i) {
                let rank_str = format!("{}.", i + 1);
                let medal = match i {
                    0 => "🥇",
                    1 => "🥈",
                    2 => "🥉",
                    _ => "  ",
                };
                let style = match i {
                    0 => Style::default()
                        .fg(colors.yellow())
                        .add_modifier(Modifier::BOLD),
                    1 => Style::default().fg(colors.fg()),
                    2 => Style::default().fg(colors.cyan()),
                    _ => Style::default().fg(colors.grey()),
                };
                view.render_row(
                    frame,
                    row,
                    vec![Span::styled(
                        format!(
                            "║  {:>3} {}  {:<3}              {:>10}              ║",
                            rank_str, medal, entry.initials, entry.score
                        ),
                        style,
                    )],
                );
            } else {
                view.render_row(
                    frame,
                    row,
                    vec![Span::styled(
                        format!(
                            "║  {:>3}     ---              ---------              ║",
                            format!("{}.", i + 1)
                        ),
                        Style::default()
                            .fg(colors.grey())
                            .add_modifier(Modifier::DIM),
                    )],
                );
            }
        }

        view.render_row(
            frame,
            16,
            vec![Span::styled(
                "║                                                   ║",
                Style::default().fg(title_color),
            )],
        );
    }

    // Footer
    view.render_row(
        frame,
        17,
        vec![Span::styled(
            "╚═══════════════════════════════════════════════════╝",
            Style::default().fg(title_color),
        )],
    );

    let help = vec![("←→", "switch game"), ("Esc", "back")];
    view.render_help(frame, help);
}
