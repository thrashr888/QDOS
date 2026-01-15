//! ADVENTURE modal rendering
//!
//! Renders the room-based exploration game with dragons and items.

use crate::app::ThemeColors;
use crate::ui::components::FullScreenView;
use ratatui::{
    prelude::*,
    style::{Modifier, Style},
    text::Span,
    Frame,
};

use super::super::adventure::{AdventureState, AdventureView, DragonType, ItemType};

const ROOM_WIDTH: usize = 38;
const ROOM_HEIGHT: usize = 14;

/// Main draw function for ADVENTURE
pub fn draw(frame: &mut Frame, area: Rect, state: &AdventureState, colors: &ThemeColors) {
    match state.view {
        AdventureView::Menu => draw_menu(frame, area, state, colors),
        AdventureView::Playing => {
            if state.eaten_by.is_some() {
                draw_eaten(frame, area, state, colors);
            } else {
                draw_room(frame, area, state, colors);
            }
        }
        AdventureView::Victory => draw_victory(frame, area, state, colors),
        AdventureView::GameOver => draw_game_over(frame, area, state, colors),
    }
}

/// Draw the menu screen
fn draw_menu(frame: &mut Frame, area: Rect, state: &AdventureState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " ADVENTURE ", colors);
    view.render_frame(frame);

    let text_style = Style::default().fg(colors.fg());
    let highlight = Style::default()
        .fg(colors.green())
        .add_modifier(Modifier::BOLD);

    // Title art
    let title = [
        r"   █████╗ ██████╗ ██╗   ██╗███████╗███╗   ██╗████████╗██╗   ██╗██████╗ ███████╗",
        r"  ██╔══██╗██╔══██╗██║   ██║██╔════╝████╗  ██║╚══██╔══╝██║   ██║██╔══██╗██╔════╝",
        r"  ███████║██║  ██║██║   ██║█████╗  ██╔██╗ ██║   ██║   ██║   ██║██████╔╝█████╗  ",
        r"  ██╔══██║██║  ██║╚██╗ ██╔╝██╔══╝  ██║╚██╗██║   ██║   ██║   ██║██╔══██╗██╔══╝  ",
        r"  ██║  ██║██████╔╝ ╚████╔╝ ███████╗██║ ╚████║   ██║   ╚██████╔╝██║  ██║███████╗",
        r"  ╚═╝  ╚═╝╚═════╝   ╚═══╝  ╚══════╝╚═╝  ╚═══╝   ╚═╝    ╚═════╝ ╚═╝  ╚═╝╚══════╝",
    ];

    // Title with animated fantasy gradient
    let phase = (state.tick_count / 10) % 3;
    for (i, line) in title.iter().enumerate() {
        let row_color = match (i, phase) {
            (0..=1, 0) => colors.yellow(), // Golden treasure
            (0..=1, 1) => colors.fg(),     // Sparkle
            (0..=1, _) => colors.yellow(),
            (2..=3, 0) => colors.cyan(), // Magic
            (2..=3, 1) => colors.blue(), // Mystical
            (2..=3, _) => colors.cyan(),
            (_, 0) => colors.green(), // Forest
            (_, 1) => colors.cyan(),  // Forest highlight
            (_, _) => colors.green(),
        };
        let style = Style::default().fg(row_color).add_modifier(Modifier::BOLD);
        view.render_row(frame, i as u16 + 1, vec![Span::styled(*line, style)]);
    }

    view.render_row(
        frame,
        8,
        vec![Span::styled("Dragon Quest - ASCII Edition", highlight)],
    );

    // Instructions
    view.render_row(frame, 10, vec![Span::styled("THE QUEST:", highlight)]);
    view.render_row(
        frame,
        11,
        vec![Span::styled(
            "  Return the Enchanted Chalice to the Gold Castle!",
            text_style,
        )],
    );

    view.render_row(frame, 13, vec![Span::styled("ITEMS:", highlight)]);
    view.render_row(
        frame,
        14,
        vec![Span::styled(
            "  + Sword (slay dragons)  & Gold Key (open gates)",
            text_style,
        )],
    );
    view.render_row(
        frame,
        15,
        vec![Span::styled(
            "  = Bridge (cross gaps)   Y Chalice (the goal!)",
            text_style,
        )],
    );

    view.render_row(
        frame,
        17,
        vec![Span::styled("Press ENTER or SPACE to start", highlight)],
    );

    view.render_help(frame, vec![("Enter", "start"), ("Esc", "quit")]);
}

/// Draw a room with the player, items, and dragons
fn draw_room(frame: &mut Frame, area: Rect, state: &AdventureState, colors: &ThemeColors) {
    let room_name = state
        .current_room()
        .map(|r| r.room_type.name())
        .unwrap_or("Unknown");

    let view = FullScreenView::new(area, &format!(" {} ", room_name), colors);
    view.render_frame(frame);

    let wall_style = Style::default().fg(colors.blue());
    let floor_style = Style::default().fg(colors.grey());
    let player_style = Style::default()
        .fg(colors.green())
        .add_modifier(Modifier::BOLD);
    let item_style = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let dragon_yellow = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let dragon_green = Style::default()
        .fg(colors.green())
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(colors.fg());
    let exit_style = Style::default().fg(colors.cyan());

    // Get room data
    let room = match state.current_room() {
        Some(r) => r,
        None => return,
    };

    // Render room grid
    for y in 0..ROOM_HEIGHT {
        let mut row_chars = Vec::new();

        for x in 0..ROOM_WIDTH {
            let ch;
            let style;

            // Check for player
            if x == state.player_x && y == state.player_y {
                ch = '@';
                style = player_style;
            }
            // Check for dragons
            else if let Some(dragon) = state
                .dragons
                .iter()
                .find(|d| d.alive && d.room == state.player_room && d.x == x && d.y == y)
            {
                ch = dragon.dragon_type.char();
                style = match dragon.dragon_type {
                    DragonType::Yorgle => dragon_yellow,
                    DragonType::Grundle => dragon_green,
                };
            }
            // Check for items
            else if let Some(item) = state
                .items
                .iter()
                .find(|i| i.room == state.player_room && i.x == x && i.y == y)
            {
                ch = item.item_type.char();
                style = item_style;
            }
            // Check for bridge placement
            else if let Some((bridge_room, bridge_x)) = state.bridge_placed {
                if bridge_room == state.player_room && x >= bridge_x && x <= bridge_x + 2 && y == 7
                {
                    ch = '═';
                    style = item_style;
                } else if room.walls[y][x] {
                    ch = '█';
                    style = wall_style;
                } else if room.is_gap(x, y) {
                    ch = '░';
                    style = floor_style;
                } else {
                    ch = '·';
                    style = floor_style;
                }
            }
            // Check for walls
            else if room.walls[y][x] {
                ch = '█';
                style = wall_style;
            }
            // Check for gap
            else if room.is_gap(x, y) {
                ch = '░';
                style = floor_style;
            }
            // Empty floor
            else {
                ch = '·';
                style = floor_style;
            }

            row_chars.push(Span::styled(ch.to_string(), style));
        }

        view.render_row(frame, y as u16 + 1, row_chars);
    }

    // Draw exit indicators
    if room.exits.north.is_some() {
        view.render_row(
            frame,
            1,
            vec![Span::styled(format!("{:^38}", "↑ North ↑"), exit_style)],
        );
    }
    if room.exits.south.is_some() {
        view.render_row(
            frame,
            ROOM_HEIGHT as u16,
            vec![Span::styled(format!("{:^38}", "↓ South ↓"), exit_style)],
        );
    }

    // HUD
    let held_str = match state.held_item {
        Some(item) => format!("{} {}", item.char(), item.name()),
        None => "nothing".to_string(),
    };

    view.render_row(
        frame,
        ROOM_HEIGHT as u16 + 2,
        vec![
            Span::styled("Holding: ", text_style),
            Span::styled(held_str, item_style),
            Span::styled("  Moves: ", text_style),
            Span::styled(format!("{}", state.moves), text_style),
            Span::styled("  Score: ", text_style),
            Span::styled(format!("{}", state.score), text_style),
        ],
    );

    // Message
    if let Some(msg) = &state.message {
        view.render_row(
            frame,
            ROOM_HEIGHT as u16 + 3,
            vec![Span::styled(
                format!("{:^40}", msg),
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            )],
        );
    }

    view.render_help(
        frame,
        vec![("↑↓←→", "move"), ("Space", "pick up/drop"), ("Esc", "menu")],
    );
}

/// Draw the "eaten by dragon" screen
fn draw_eaten(frame: &mut Frame, area: Rect, state: &AdventureState, colors: &ThemeColors) {
    let dragon_name = state.eaten_by.map(|d| d.name()).unwrap_or("Dragon");

    let view = FullScreenView::new(
        area,
        &format!(" INSIDE {}! ", dragon_name.to_uppercase()),
        colors,
    );
    view.render_frame(frame);

    let red = Style::default()
        .fg(colors.red())
        .add_modifier(Modifier::BOLD);
    let yellow = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(colors.fg());

    view.render_row(
        frame,
        5,
        vec![Span::styled(
            "╔════════════════════════════════════════╗",
            red,
        )],
    );
    view.render_row(
        frame,
        6,
        vec![Span::styled(
            "║                                        ║",
            red,
        )],
    );
    view.render_row(
        frame,
        7,
        vec![Span::styled(
            format!("║    You have been swallowed by {:10}║", dragon_name),
            red,
        )],
    );
    view.render_row(
        frame,
        8,
        vec![Span::styled(
            "║                                        ║",
            red,
        )],
    );
    view.render_row(
        frame,
        9,
        vec![Span::styled(
            "║       It is dark and squishy...        ║",
            red,
        )],
    );
    view.render_row(
        frame,
        10,
        vec![Span::styled(
            "║                                        ║",
            red,
        )],
    );
    view.render_row(
        frame,
        11,
        vec![Span::styled(
            "║                  @                     ║",
            red,
        )],
    );
    view.render_row(
        frame,
        12,
        vec![Span::styled(
            "║                                        ║",
            red,
        )],
    );
    view.render_row(
        frame,
        13,
        vec![Span::styled(
            "╚════════════════════════════════════════╝",
            red,
        )],
    );

    // Hint
    if state.held_item == Some(ItemType::Sword) {
        view.render_row(
            frame,
            15,
            vec![Span::styled(
                "You have the sword! Press SPACE to strike!",
                yellow,
            )],
        );
    } else {
        view.render_row(
            frame,
            15,
            vec![Span::styled("Find the sword to escape!", text_style)],
        );
    }

    // HUD
    let held_str = match state.held_item {
        Some(item) => format!("{} {}", item.char(), item.name()),
        None => "nothing".to_string(),
    };

    view.render_row(
        frame,
        17,
        vec![
            Span::styled("Holding: ", text_style),
            Span::styled(held_str, yellow),
        ],
    );

    view.render_help(frame, vec![("Space", "use sword"), ("Esc", "menu")]);
}

/// Draw victory screen
fn draw_victory(frame: &mut Frame, area: Rect, state: &AdventureState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " VICTORY! ", colors);
    view.render_frame(frame);

    let green = Style::default()
        .fg(colors.green())
        .add_modifier(Modifier::BOLD);
    let yellow = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(colors.fg());

    view.render_row(
        frame,
        4,
        vec![Span::styled(
            "╔══════════════════════════════════════════╗",
            green,
        )],
    );
    view.render_row(
        frame,
        5,
        vec![Span::styled(
            "║                                          ║",
            green,
        )],
    );
    view.render_row(
        frame,
        6,
        vec![Span::styled(
            "║       THE CHALICE HAS BEEN RETURNED!     ║",
            green,
        )],
    );
    view.render_row(
        frame,
        7,
        vec![Span::styled(
            "║                                          ║",
            green,
        )],
    );
    view.render_row(
        frame,
        8,
        vec![Span::styled(
            "║             Y  VICTORY!  Y               ║",
            green,
        )],
    );
    view.render_row(
        frame,
        9,
        vec![Span::styled(
            "║                                          ║",
            green,
        )],
    );
    view.render_row(
        frame,
        10,
        vec![Span::styled(
            "╚══════════════════════════════════════════╝",
            green,
        )],
    );

    view.render_row(frame, 12, vec![Span::styled("FINAL STATS", yellow)]);
    view.render_row(frame, 13, vec![Span::styled("───────────", text_style)]);

    view.render_row(
        frame,
        14,
        vec![Span::styled(
            format!("Total Moves: {}", state.moves),
            text_style,
        )],
    );
    view.render_row(
        frame,
        15,
        vec![Span::styled(
            format!("Final Score: {}", state.score),
            yellow,
        )],
    );

    view.render_row(
        frame,
        17,
        vec![Span::styled("Press ENTER to play again", yellow)],
    );

    view.render_help(frame, vec![("Enter", "restart"), ("Esc", "quit")]);
}

/// Draw game over screen
fn draw_game_over(frame: &mut Frame, area: Rect, state: &AdventureState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " ADVENTURE ", colors);
    view.render_frame(frame);

    let red = Style::default()
        .fg(colors.red())
        .add_modifier(Modifier::BOLD);
    let yellow = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(colors.fg());

    view.render_row(
        frame,
        5,
        vec![Span::styled("╔══════════════════════════════════╗", red)],
    );
    view.render_row(
        frame,
        6,
        vec![Span::styled("║          GAME OVER               ║", red)],
    );
    view.render_row(
        frame,
        7,
        vec![Span::styled("╚══════════════════════════════════╝", red)],
    );

    view.render_row(
        frame,
        9,
        vec![Span::styled("The quest remains unfulfilled...", text_style)],
    );

    view.render_row(frame, 11, vec![Span::styled("STATS", yellow)]);
    view.render_row(frame, 12, vec![Span::styled("─────", text_style)]);
    view.render_row(
        frame,
        13,
        vec![Span::styled(format!("Moves: {}", state.moves), text_style)],
    );
    view.render_row(
        frame,
        14,
        vec![Span::styled(format!("Score: {}", state.score), text_style)],
    );

    view.render_row(
        frame,
        16,
        vec![Span::styled("Press ENTER to try again", yellow)],
    );

    view.render_help(frame, vec![("Enter", "restart"), ("Esc", "quit")]);
}
