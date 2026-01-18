//! Rogue game modal rendering
//!
//! This module contains the rendering logic for the roguelike dungeon crawler game.
//! It renders the dungeon map with fog of war, player status, monsters, items,
//! and game messages using the FullScreenView component.

use super::super::rogue::{self, RogueState};
use qdos_plugin_api::ui::FullScreenView;
use qdos_plugin_api::ThemeColors;
use ratatui::{
    style::{Modifier, Style},
    text::Span,
    Frame,
};

/// Renders the Rogue game modal with dungeon map, status bar, and messages.
///
/// The display consists of:
/// - Status bar (top): HP, Strength, Gold, Level, Dungeon Level, Armor, Hunger
/// - Dungeon map: Tiles with fog of war (visible, explored, unexplored)
/// - Message area: Recent game messages (up to 2 lines)
/// - Help footer: Control hints
pub fn draw_rogue(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &RogueState,
    colors: &ThemeColors,
) {
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
        ("\u{2190}\u{2191}\u{2193}\u{2192}", "move"),
        ("hjkl", "move"),
        ("yubn", "diag"),
        ("s", "search"),
        (">", "stairs"),
        ("P", "pause"),
        ("Esc", "quit"),
    ];
    view.render_help(frame, help);
}
