//! CAVERNS modal rendering
//!
//! Renders the text adventure game UI with room descriptions,
//! inventory, encounters, and ASCII art.

use super::super::caverns::{
    get_creature_def, get_item_def, get_room, CavernsState, CavernsView, ExamineTarget,
};
use qdos_plugin_api::ui::FullScreenView;
use qdos_plugin_api::ThemeColors;
use ratatui::prelude::*;

// =============================================================================
// ASCII ART
// =============================================================================

const ENTRANCE_ART: &[&str] = &[
    r"          /\            /\          ",
    r"         /  \    *    /  \         ",
    r"        /    \      /    \        ",
    r"       /______\    /______\       ",
    r"      |   __   |  |   __   |      ",
    r"      |  |  |  ####  |  |  |      ",
    r"      |  |  |  ####  |  |  |      ",
    r"   ===+==+==+======+==+==+===   ",
    r"        C A V E R N S            ",
];

const CRYSTAL_ART: &[&str] = &[
    r"    +  *  +     +  +  *    ",
    r"   /^\   /^\  /^\   /^\   ",
    r"  /<>\  /<><>/><\  /><\  ",
    r" /<><>\/><><><><\/><><\ ",
    r" ---------------------- ",
    r"    *  +  +     +  *  +    ",
];

const DRAGON_ART: &[&str] = &[
    r"              __===~~`--..__        ",
    r"           .=`    .--.  `.  `-._    ",
    r"          /      (O  O)   \    `.   ",
    r"         |    .-' \__/`-.  |    ;  ",
    r"         \   /    `==`   \ |   /   ",
    r"          `.;  __.---._   '/  /    ",
    r"            `-`        `--`-`      ",
    r"       ^ The Dragon awaits... ^   ",
];

const LAKE_ART: &[&str] = &[
    r" ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ ",
    r" ▓▓   ≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈   ▓▓ ",
    r" ▓ ≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈ ▓ ",
    r" ▓ ≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈ ▓ ",
    r" ▓▓   ≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈≈   ▓▓ ",
    r" ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ ",
];

const EXIT_ART: &[&str] = &[
    r"                                  ",
    r"     +--------------------+     ",
    r"     |   *  DAYLIGHT  *   |     ",
    r"     |                    |     ",
    r"     |    FREEDOM AT     |     ",
    r"     |       LAST!        |     ",
    r"     +--------------------+     ",
    r"                                  ",
];

fn get_room_art(room_id: usize) -> Option<&'static [&'static str]> {
    match room_id {
        0 => Some(ENTRANCE_ART),
        4 => Some(CRYSTAL_ART),
        8 => Some(LAKE_ART),
        18 => Some(DRAGON_ART),
        19 => Some(EXIT_ART),
        _ => None,
    }
}

// =============================================================================
// MAIN DISPATCHER
// =============================================================================

pub fn draw_caverns(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &CavernsState,
    colors: &ThemeColors,
) {
    match state.view {
        CavernsView::Playing => draw_playing(frame, view, state, colors),
        CavernsView::Inventory => draw_inventory(frame, view, state, colors),
        CavernsView::ItemSelect => draw_item_select(frame, view, state, colors),
        CavernsView::Encounter => draw_encounter(frame, view, state, colors),
        CavernsView::Examining => draw_examine(frame, view, state, colors),
        CavernsView::GameOver => draw_game_over(frame, view, state, colors),
        CavernsView::Victory => draw_victory(frame, view, state, colors),
    }
}

// =============================================================================
// PLAYING VIEW
// =============================================================================

fn draw_playing(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &CavernsState,
    colors: &ThemeColors,
) {
    let room = get_room(state.current_room);
    let mut row = 0;

    // Room name header
    let lamp_status = if state.lamp_lit { " [LAMP LIT]" } else { "" };
    view.render_row(
        frame,
        row,
        vec![
            Span::styled(
                format!(" {} ", room.name),
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(lamp_status, Style::default().fg(colors.green())),
        ],
    );
    row += 1;

    // Separator
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "═".repeat(78),
            Style::default().fg(colors.grey()),
        )],
    );
    row += 1;

    // ASCII art if available
    if let Some(art) = get_room_art(state.current_room) {
        for line in art {
            view.render_row(
                frame,
                row,
                vec![Span::styled(*line, Style::default().fg(colors.cyan()))],
            );
            row += 1;
        }
        row += 1;
    }

    // Room description
    let desc = room.description;
    // Word wrap at 76 chars
    for chunk in wrap_text(desc, 76) {
        view.render_row(
            frame,
            row,
            vec![Span::styled(chunk, Style::default().fg(colors.fg()))],
        );
        row += 1;
    }
    row += 1;

    // Items in room
    let items = state.get_room_items();
    if !items.is_empty() {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "Items here: ",
                Style::default().fg(colors.green()),
            )],
        );
        row += 1;
        for item in items {
            let item_def = get_item_def(*item);
            let treasure_mark = if item_def.is_treasure { " *" } else { "" };
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!("  • {}{}", item_def.name, treasure_mark),
                    Style::default().fg(colors.yellow()),
                )],
            );
            row += 1;
        }
    }

    // Exits
    let exits = format_exits(room);
    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Exits: ", Style::default().fg(colors.blue())),
            Span::styled(exits, Style::default().fg(colors.fg())),
        ],
    );
    row += 1;

    // Status line
    row += 1;
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!(
                "Score: {} | Rooms: {}/20 | Treasures: {}/8 | Turns: {}",
                state.calculate_score(),
                state.rooms_discovered,
                state.treasures_deposited.len(),
                state.turns
            ),
            Style::default().fg(colors.grey()),
        )],
    );

    // Messages (at bottom)
    let msg_start = 18u16;
    for (i, msg) in state.messages.iter().enumerate() {
        let style = if i == state.messages.len() - 1 {
            Style::default().fg(colors.fg())
        } else {
            Style::default().fg(colors.grey())
        };
        view.render_row(
            frame,
            msg_start + i as u16,
            vec![Span::styled(format!("> {}", msg), style)],
        );
    }

    // Help footer
    view.render_help(
        frame,
        vec![
            ("N/S/E/W", "move"),
            ("U/D", "up/down"),
            ("G", "get"),
            ("I", "inventory"),
            ("L", "look"),
            ("Enter", "lamp"),
            ("Esc", "quit"),
        ],
    );
}

// =============================================================================
// INVENTORY VIEW
// =============================================================================

fn draw_inventory(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &CavernsState,
    colors: &ThemeColors,
) {
    let mut row = 0;

    // Header
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            " INVENTORY ",
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "═".repeat(78),
            Style::default().fg(colors.grey()),
        )],
    );
    row += 2;

    // Items
    if state.inventory.is_empty() {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "  Your inventory is empty.",
                Style::default().fg(colors.grey()),
            )],
        );
    } else {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!(
                    "  Your belongings ({}/{} slots):",
                    state.inventory.len(),
                    state.max_inventory
                ),
                Style::default().fg(colors.fg()),
            )],
        );
        row += 2;

        for (i, item) in state.inventory.iter().enumerate() {
            let item_def = get_item_def(*item);
            let selected = i == state.selected_item;
            let prefix = if selected { "► " } else { "  " };
            let treasure_mark = if item_def.is_treasure {
                " * TREASURE"
            } else {
                ""
            };
            let lamp_status = if *item == crate::plugins::games::caverns::ItemId::BrassLamp {
                if state.lamp_lit {
                    " (lit)"
                } else {
                    " (off)"
                }
            } else {
                ""
            };

            let style = if selected {
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.fg())
            };

            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    format!(
                        "{}{}{}{}",
                        prefix, item_def.name, lamp_status, treasure_mark
                    ),
                    style,
                )],
            );
            row += 1;
        }
    }

    // Treasures deposited
    row += 2;
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!(
                "  Treasures deposited: {}/8",
                state.treasures_deposited.len()
            ),
            Style::default().fg(colors.green()),
        )],
    );
    row += 1;
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!("  Current score: {}", state.calculate_score()),
            Style::default().fg(colors.green()),
        )],
    );

    // Help footer
    view.render_help(
        frame,
        vec![
            ("↑↓", "select"),
            ("Enter", "use"),
            ("R", "drop"),
            ("X", "examine"),
            ("Esc", "back"),
        ],
    );
}

// =============================================================================
// ITEM SELECT VIEW
// =============================================================================

fn draw_item_select(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &CavernsState,
    colors: &ThemeColors,
) {
    let mut row = 0;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            " SELECT ITEM TO TAKE ",
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "═".repeat(78),
            Style::default().fg(colors.grey()),
        )],
    );
    row += 2;

    let items = state.get_room_items();
    for (i, item) in items.iter().enumerate() {
        let item_def = get_item_def(*item);
        let selected = i == state.selected_item;
        let prefix = if selected { "► " } else { "  " };
        let treasure_mark = if item_def.is_treasure { " *" } else { "" };

        let style = if selected {
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.fg())
        };

        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("{}{}{}", prefix, item_def.name, treasure_mark),
                style,
            )],
        );
        row += 1;
    }

    view.render_help(
        frame,
        vec![("↑↓", "select"), ("Enter", "take"), ("Esc", "cancel")],
    );
}

// =============================================================================
// ENCOUNTER VIEW
// =============================================================================

fn draw_encounter(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &CavernsState,
    colors: &ThemeColors,
) {
    let creature_id = match state.encounter_creature {
        Some(c) => c,
        None => return,
    };
    let creature = get_creature_def(creature_id);

    let mut row = 0;

    // Header
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!(" {} ENCOUNTER! ", creature.name.to_uppercase()),
            Style::default()
                .fg(colors.red())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "═".repeat(78),
            Style::default().fg(colors.red()),
        )],
    );
    row += 2;

    // Dragon art for dragon encounter
    if creature_id == crate::plugins::games::caverns::CreatureId::Dragon {
        for line in DRAGON_ART {
            view.render_row(
                frame,
                row,
                vec![Span::styled(*line, Style::default().fg(colors.red()))],
            );
            row += 1;
        }
        row += 1;
    }

    // Description
    for chunk in wrap_text(creature.description, 76) {
        view.render_row(
            frame,
            row,
            vec![Span::styled(chunk, Style::default().fg(colors.fg()))],
        );
        row += 1;
    }
    row += 2;

    // Options
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "What do you do?",
            Style::default().fg(colors.yellow()),
        )],
    );
    row += 2;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "  [A] Attack",
            Style::default().fg(colors.red()),
        )],
    );
    row += 1;

    // Show inventory items that might help
    for (i, item) in state.inventory.iter().enumerate() {
        let item_def = get_item_def(*item);
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                format!("  [{}] Use {}", i + 1, item_def.name),
                Style::default().fg(colors.cyan()),
            )],
        );
        row += 1;
    }

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "  [F] Flee",
            Style::default().fg(colors.green()),
        )],
    );

    view.render_help(
        frame,
        vec![("A", "attack"), ("1-9", "use item"), ("F", "flee")],
    );
}

// =============================================================================
// EXAMINE VIEW
// =============================================================================

fn draw_examine(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &CavernsState,
    colors: &ThemeColors,
) {
    let mut row = 0;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            " EXAMINE ",
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "═".repeat(78),
            Style::default().fg(colors.grey()),
        )],
    );
    row += 2;

    match &state.examine_target {
        Some(ExamineTarget::Room) => {
            let room = get_room(state.current_room);
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    room.name,
                    Style::default()
                        .fg(colors.yellow())
                        .add_modifier(Modifier::BOLD),
                )],
            );
            row += 2;

            for chunk in wrap_text(room.description, 76) {
                view.render_row(
                    frame,
                    row,
                    vec![Span::styled(chunk, Style::default().fg(colors.fg()))],
                );
                row += 1;
            }
        }
        Some(ExamineTarget::Item(item_id)) => {
            let item = get_item_def(*item_id);
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    item.name,
                    Style::default()
                        .fg(colors.yellow())
                        .add_modifier(Modifier::BOLD),
                )],
            );
            row += 2;

            for chunk in wrap_text(item.description, 76) {
                view.render_row(
                    frame,
                    row,
                    vec![Span::styled(chunk, Style::default().fg(colors.fg()))],
                );
                row += 1;
            }

            if item.is_treasure {
                row += 1;
                view.render_row(
                    frame,
                    row,
                    vec![Span::styled(
                        format!("Value: {} points", item.points),
                        Style::default().fg(colors.green()),
                    )],
                );
            }
        }
        None => {}
    }

    view.render_help(frame, vec![("Enter/Esc", "back")]);
}

// =============================================================================
// GAME OVER VIEW
// =============================================================================

fn draw_game_over(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &CavernsState,
    colors: &ThemeColors,
) {
    let mut row = 4;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "╔══════════════════════════════════════════════════════════════════════════╗",
            Style::default().fg(colors.red()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "║                            GAME OVER                                     ║",
            Style::default()
                .fg(colors.red())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "╠══════════════════════════════════════════════════════════════════════════╣",
            Style::default().fg(colors.red()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!(
                "║  Final Score: {:>5}                                                     ║",
                state.calculate_score()
            ),
            Style::default().fg(colors.fg()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!(
                "║  Rooms Discovered: {:>2}/20                                                ║",
                state.rooms_discovered
            ),
            Style::default().fg(colors.fg()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!(
                "║  Treasures Found: {:>1}/8                                                   ║",
                state.treasures_deposited.len()
            ),
            Style::default().fg(colors.fg()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!(
                "║  Turns Taken: {:>4}                                                       ║",
                state.turns
            ),
            Style::default().fg(colors.fg()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "╚══════════════════════════════════════════════════════════════════════════╝",
            Style::default().fg(colors.red()),
        )],
    );

    view.render_help(frame, vec![("Enter", "restart"), ("Esc", "quit")]);
}

// =============================================================================
// VICTORY VIEW
// =============================================================================

fn draw_victory(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &CavernsState,
    colors: &ThemeColors,
) {
    let mut row = 3;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "╔══════════════════════════════════════════════════════════════════════════╗",
            Style::default().fg(colors.green()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "║                                                                          ║",
            Style::default().fg(colors.green()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "║             * * *  CONGRATULATIONS!  * * *                              ║",
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "║                                                                          ║",
            Style::default().fg(colors.green()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "║              You have recovered all the treasures!                       ║",
            Style::default().fg(colors.fg()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "║                                                                          ║",
            Style::default().fg(colors.green()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "╠══════════════════════════════════════════════════════════════════════════╣",
            Style::default().fg(colors.green()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!(
                "║  Final Score: {:>5}   (includes +500 victory bonus)                    ║",
                state.calculate_score()
            ),
            Style::default().fg(colors.yellow()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!(
                "║  Rooms Discovered: {:>2}/20                                                ║",
                state.rooms_discovered
            ),
            Style::default().fg(colors.fg()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!(
                "║  Puzzles Solved: {:>1}/5                                                    ║",
                state.puzzles_solved
            ),
            Style::default().fg(colors.fg()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            format!(
                "║  Turns Taken: {:>4}                                                       ║",
                state.turns
            ),
            Style::default().fg(colors.fg()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "║                                                                          ║",
            Style::default().fg(colors.green()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "╚══════════════════════════════════════════════════════════════════════════╝",
            Style::default().fg(colors.green()),
        )],
    );

    view.render_help(frame, vec![("Enter", "play again"), ("Esc", "quit")]);
}

// =============================================================================
// HELPERS
// =============================================================================

fn format_exits(room: &crate::plugins::games::caverns::Room) -> String {
    let mut exits = Vec::new();
    if room.exits.north.is_some() {
        exits.push("North");
    }
    if room.exits.south.is_some() {
        exits.push("South");
    }
    if room.exits.east.is_some() {
        exits.push("East");
    }
    if room.exits.west.is_some() {
        exits.push("West");
    }
    if room.exits.up.is_some() {
        exits.push("Up");
    }
    if room.exits.down.is_some() {
        exits.push("Down");
    }

    if exits.is_empty() {
        "None".to_string()
    } else {
        exits.join(", ")
    }
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}
