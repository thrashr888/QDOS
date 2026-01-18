//! DUNGEON Modal Rendering
//!
//! Renders the dungeon maze game with fog of war and ANSI art style.

use super::super::dungeon::{
    DungeonState, EnemyType, FriendlyType, ItemType, Tile, BOARD_HEIGHT, BOARD_WIDTH,
};
use super::super::platform::GameEngine;
use qdos_plugin_api::ui::FullScreenView;
use qdos_plugin_api::ThemeColors;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::Frame;

/// Draw the dungeon game
pub fn draw_dungeon(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DungeonState,
    colors: &ThemeColors,
) {
    if state.game_over {
        draw_game_over(frame, view, state, colors);
        return;
    }

    // Draw the map
    draw_map(frame, view, state, colors);

    // Draw the HUD at bottom
    draw_hud(frame, view, state, colors);

    // Draw messages
    draw_messages(frame, view, state, colors);

    // Draw help
    view.render_help(
        frame,
        vec![
            ("←↑↓→/HJKL", "move"),
            (">", "descend"),
            (".", "wait"),
            ("Esc", "quit"),
        ],
    );
}

/// Draw the game map
fn draw_map(frame: &mut Frame, view: &FullScreenView, state: &DungeonState, colors: &ThemeColors) {
    // Calculate offset to center the board
    let x_offset = 1;

    for y in 0..BOARD_HEIGHT {
        let mut spans: Vec<Span> = Vec::new();

        // Add left padding
        spans.push(Span::raw(" ".repeat(x_offset)));

        for x in 0..BOARD_WIDTH {
            let ch: char;
            let style: Style;

            // Check visibility
            let visible = state.visible[y][x];
            let explored = state.explored[y][x];

            if !visible && !explored {
                // Completely dark
                ch = ' ';
                style = Style::default();
            } else {
                // Check for player
                if x == state.player_x && y == state.player_y {
                    ch = '@';
                    style = Style::default()
                        .fg(colors.yellow())
                        .add_modifier(Modifier::BOLD);
                }
                // Check for enemies
                else if let Some(enemy) = state.enemies.iter().find(|e| e.x == x && e.y == y) {
                    if visible {
                        ch = enemy.enemy_type.char();
                        style = match enemy.enemy_type {
                            EnemyType::Snake => Style::default().fg(colors.green()),
                            EnemyType::Goblin => Style::default().fg(colors.red()),
                            EnemyType::Ghost => Style::default()
                                .fg(colors.cyan())
                                .add_modifier(Modifier::DIM),
                            EnemyType::Skeleton => Style::default().fg(colors.fg()),
                            EnemyType::Troll => Style::default()
                                .fg(colors.red())
                                .add_modifier(Modifier::BOLD),
                            EnemyType::Boss => Style::default()
                                .fg(colors.red())
                                .add_modifier(Modifier::BOLD),
                        };
                    } else {
                        // Not visible, show tile
                        ch = state.board[y][x].char();
                        style = Style::default()
                            .fg(colors.grey())
                            .add_modifier(Modifier::DIM);
                    }
                }
                // Check for friendlies
                else if let Some(friendly) =
                    state.friendlies.iter().find(|f| f.x == x && f.y == y)
                {
                    if visible {
                        ch = friendly.friendly_type.char();
                        style = match friendly.friendly_type {
                            FriendlyType::Sheep => Style::default().fg(colors.fg()),
                            FriendlyType::Merchant => Style::default()
                                .fg(colors.yellow())
                                .add_modifier(Modifier::BOLD),
                            FriendlyType::Fairy => Style::default()
                                .fg(colors.cyan())
                                .add_modifier(Modifier::BOLD),
                        };
                    } else {
                        ch = state.board[y][x].char();
                        style = Style::default()
                            .fg(colors.grey())
                            .add_modifier(Modifier::DIM);
                    }
                }
                // Check for items
                else if let Some(item) = state.items.iter().find(|i| i.x == x && i.y == y) {
                    if visible {
                        ch = item.item_type.char();
                        style = match item.item_type {
                            ItemType::Gold => Style::default()
                                .fg(colors.yellow())
                                .add_modifier(Modifier::BOLD),
                            ItemType::Food => Style::default().fg(colors.green()),
                            ItemType::Key => Style::default().fg(colors.cyan()),
                            ItemType::Potion => Style::default().fg(colors.blue()),
                            ItemType::Weapon => Style::default().fg(colors.red()),
                        };
                    } else {
                        ch = state.board[y][x].char();
                        style = Style::default()
                            .fg(colors.grey())
                            .add_modifier(Modifier::DIM);
                    }
                }
                // Draw tile
                else {
                    let tile = state.board[y][x];
                    ch = tile.char();

                    if visible {
                        style = match tile {
                            Tile::Wall => Style::default().fg(colors.grey()),
                            Tile::Floor => Style::default()
                                .fg(colors.grey())
                                .add_modifier(Modifier::DIM),
                            Tile::Exit => Style::default()
                                .fg(colors.green())
                                .add_modifier(Modifier::BOLD),
                            Tile::Door => Style::default().fg(colors.yellow()),
                            Tile::DoorOpen => Style::default().fg(colors.grey()),
                            Tile::Trap => Style::default().fg(colors.red()),
                            Tile::TrapHidden => Style::default()
                                .fg(colors.grey())
                                .add_modifier(Modifier::DIM),
                            Tile::Water => Style::default().fg(colors.blue()),
                        };
                    } else {
                        // Explored but not visible - dimmed
                        style = Style::default()
                            .fg(colors.grey())
                            .add_modifier(Modifier::DIM);
                    }
                }
            }

            spans.push(Span::styled(ch.to_string(), style));
        }

        view.render_row(frame, y as u16, spans);
    }
}

/// Draw the HUD
fn draw_hud(frame: &mut Frame, view: &FullScreenView, state: &DungeonState, colors: &ThemeColors) {
    let hud_row = BOARD_HEIGHT as u16;

    // HP bar
    let hp_pct = (state.hp as f32 / state.max_hp as f32 * 10.0).round() as usize;
    let hp_bar: String = format!(
        "[{}{}]",
        "█".repeat(hp_pct),
        "░".repeat(10 - hp_pct.min(10))
    );

    let hp_color = if state.hp <= state.max_hp / 4 {
        colors.red()
    } else if state.hp <= state.max_hp / 2 {
        colors.yellow()
    } else {
        colors.green()
    };

    view.render_row(
        frame,
        hud_row,
        vec![
            Span::styled(" HP:", Style::default().fg(colors.fg())),
            Span::styled(hp_bar, Style::default().fg(hp_color)),
            Span::styled(
                format!(" {}/{}", state.hp, state.max_hp),
                Style::default().fg(hp_color),
            ),
            Span::styled("  ATK:", Style::default().fg(colors.fg())),
            Span::styled(
                format!("{}", state.attack),
                Style::default().fg(colors.red()),
            ),
            Span::styled("  LV:", Style::default().fg(colors.fg())),
            Span::styled(
                format!("{}", state.level),
                Style::default().fg(colors.cyan()),
            ),
            Span::styled("  XP:", Style::default().fg(colors.fg())),
            Span::styled(
                format!("{}/{}", state.xp, state.level * 100),
                Style::default().fg(colors.cyan()),
            ),
        ],
    );

    view.render_row(
        frame,
        hud_row + 1,
        vec![
            Span::styled(" FLOOR:", Style::default().fg(colors.fg())),
            Span::styled(
                format!("{}", state.floor),
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  GOLD:", Style::default().fg(colors.fg())),
            Span::styled(
                format!("${}", state.gold),
                Style::default().fg(colors.yellow()),
            ),
            Span::styled("  KEYS:", Style::default().fg(colors.fg())),
            Span::styled(
                format!("{}", state.keys),
                Style::default().fg(colors.cyan()),
            ),
        ],
    );
}

/// Draw messages
fn draw_messages(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DungeonState,
    colors: &ThemeColors,
) {
    let msg_row = (BOARD_HEIGHT + 2) as u16;

    // Show last few messages
    for (i, msg) in state.messages.iter().rev().take(2).enumerate() {
        let style = if i == 0 {
            Style::default().fg(colors.fg())
        } else {
            Style::default()
                .fg(colors.grey())
                .add_modifier(Modifier::DIM)
        };

        view.render_row(
            frame,
            msg_row + i as u16,
            vec![Span::styled(format!(" {}", msg), style)],
        );
    }
}

/// Draw game over screen
fn draw_game_over(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DungeonState,
    colors: &ThemeColors,
) {
    let center_row = 6u16;

    let (title, title_color) = if state.game_won {
        (
            "╔══════════════════════════════════════════╗",
            colors.green(),
        )
    } else {
        ("╔══════════════════════════════════════════╗", colors.red())
    };

    view.render_row(
        frame,
        center_row,
        vec![Span::styled(title, Style::default().fg(title_color))],
    );

    let header = if state.game_won {
        "║          * VICTORY! *                    ║"
    } else {
        "║           GAME OVER                      ║"
    };

    view.render_row(
        frame,
        center_row + 1,
        vec![Span::styled(
            header,
            Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
        )],
    );

    view.render_row(
        frame,
        center_row + 2,
        vec![Span::styled(
            "╠══════════════════════════════════════════╣",
            Style::default().fg(title_color),
        )],
    );

    // Stats
    view.render_row(
        frame,
        center_row + 3,
        vec![Span::styled(
            format!(
                "║  Floor Reached:  {:>3}                     ║",
                state.max_floor
            ),
            Style::default().fg(colors.fg()),
        )],
    );

    view.render_row(
        frame,
        center_row + 4,
        vec![Span::styled(
            format!("║  Gold Collected: {:>4}                    ║", state.gold),
            Style::default().fg(colors.yellow()),
        )],
    );

    view.render_row(
        frame,
        center_row + 5,
        vec![Span::styled(
            format!("║  XP Earned:      {:>4}                    ║", state.xp),
            Style::default().fg(colors.cyan()),
        )],
    );

    view.render_row(
        frame,
        center_row + 6,
        vec![Span::styled(
            format!(
                "║  Player Level:   {:>3}                     ║",
                state.level
            ),
            Style::default().fg(colors.green()),
        )],
    );

    let score = state.get_score();
    view.render_row(
        frame,
        center_row + 7,
        vec![Span::styled(
            format!("║  FINAL SCORE:    {:>5}                  ║", score),
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );

    view.render_row(
        frame,
        center_row + 8,
        vec![Span::styled(
            "╚══════════════════════════════════════════╝",
            Style::default().fg(title_color),
        )],
    );

    view.render_help(frame, vec![("Enter", "play again"), ("Esc", "menu")]);
}
