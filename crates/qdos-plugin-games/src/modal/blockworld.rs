//! BLOCKWORLD modal rendering
//!
//! Renders the Terraria-style 2D mining game with blocks, creatures, and UI.

use super::super::blockworld::{
    BlockType, BlockworldState, BlockworldView, TimeOfDay, VIEW_HEIGHT, VIEW_WIDTH,
};
use qdos_plugin_api::ui::FullScreenView;
use qdos_plugin_api::ThemeColors;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    Frame,
};

// =============================================================================
// ASCII ART
// =============================================================================

const TITLE_ART: &[&str] = &[
    "██████╗ ██╗      ██████╗  ██████╗██╗  ██╗██╗    ██╗ ██████╗ ██████╗ ██╗     ██████╗ ",
    "██╔══██╗██║     ██╔═══██╗██╔════╝██║ ██╔╝██║    ██║██╔═══██╗██╔══██╗██║     ██╔══██╗",
    "██████╔╝██║     ██║   ██║██║     █████╔╝ ██║ █╗ ██║██║   ██║██████╔╝██║     ██║  ██║",
    "██╔══██╗██║     ██║   ██║██║     ██╔═██╗ ██║███╗██║██║   ██║██╔══██╗██║     ██║  ██║",
    "██████╔╝███████╗╚██████╔╝╚██████╗██║  ██╗╚███╔███╔╝╚██████╔╝██║  ██║███████╗██████╔╝",
    "╚═════╝ ╚══════╝ ╚═════╝  ╚═════╝╚═╝  ╚═╝ ╚══╝╚══╝  ╚═════╝ ╚═╝  ╚═╝╚══════╝╚═════╝ ",
];

// =============================================================================
// MAIN DRAW FUNCTION
// =============================================================================

pub fn draw_blockworld(
    frame: &mut Frame,
    area: Rect,
    state: &BlockworldState,
    colors: &ThemeColors,
) {
    match state.view {
        BlockworldView::Menu => draw_menu(frame, area, state, colors),
        BlockworldView::Playing => draw_game(frame, area, state, colors),
        BlockworldView::Inventory => draw_inventory(frame, area, state, colors),
        BlockworldView::Crafting => draw_crafting(frame, area, state, colors),
        BlockworldView::Paused => draw_paused(frame, area, state, colors),
        BlockworldView::GameOver => draw_game_over(frame, area, state, colors),
    }
}

// =============================================================================
// MENU SCREEN
// =============================================================================

fn draw_menu(frame: &mut Frame, area: Rect, state: &BlockworldState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " BLOCKWORLD ", colors);
    view.render_frame(frame);

    let text_style = Style::default().fg(colors.fg());
    let highlight = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);

    // Title with animated grass-block gradient
    let phase = (state.tick_count / 10) % 3;
    for (i, line) in TITLE_ART.iter().enumerate() {
        // Cycle colors while maintaining gradient feel
        let row_color = match (i, phase) {
            (0, 0) => colors.green(),  // Grass green
            (0, 1) => colors.cyan(),   // Grass highlight
            (0, _) => colors.green(),  // Grass green
            (_, 0) => colors.red(),    // Dirt
            (_, 1) => colors.yellow(), // Dirt highlight
            (_, _) => colors.red(),    // Dirt
        };
        let style = Style::default().fg(row_color).add_modifier(Modifier::BOLD);
        view.render_row(frame, 1 + i as u16, vec![Span::styled(*line, style)]);
    }

    // Subtitle
    view.render_row(
        frame,
        7,
        vec![Span::styled(
            "          A Terraria-Style Mining Adventure",
            text_style,
        )],
    );

    // Instructions
    view.render_row(frame, 10, vec![Span::styled("  CONTROLS:", highlight)]);
    view.render_row(
        frame,
        11,
        vec![Span::styled(
            "  WASD/Arrows - Move    Space/W - Jump",
            text_style,
        )],
    );
    view.render_row(
        frame,
        12,
        vec![Span::styled(
            "  HJKL - Aim cursor     Z - Mine    X - Place",
            text_style,
        )],
    );
    view.render_row(
        frame,
        13,
        vec![Span::styled(
            "  C - Attack            E - Eat     I - Inventory",
            text_style,
        )],
    );
    view.render_row(
        frame,
        14,
        vec![Span::styled("  1-9 - Select hotbar slot", text_style)],
    );

    view.render_row(
        frame,
        17,
        vec![Span::styled("Press ENTER or SPACE to start", highlight)],
    );

    view.render_help(frame, vec![("Enter", "start"), ("Esc", "quit")]);
}

// =============================================================================
// GAME SCREEN
// =============================================================================

fn draw_game(frame: &mut Frame, area: Rect, state: &BlockworldState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " BLOCKWORLD ", colors);
    view.render_frame(frame);

    // Status bar (row 0)
    draw_status_bar(frame, &view, state, colors);

    // World view (rows 1-18)
    draw_world(frame, &view, state, colors);

    // Hotbar (row 19)
    draw_hotbar(frame, &view, state, colors);

    // Message
    if let Some(msg) = &state.message {
        let msg_style = Style::default()
            .fg(colors.yellow())
            .add_modifier(Modifier::BOLD);
        view.render_row(
            frame,
            19,
            vec![Span::styled(format!("  {}", msg), msg_style)],
        );
    }

    // Help
    view.render_help(
        frame,
        vec![
            ("WASD", "move"),
            ("HJKL", "aim"),
            ("Z", "mine"),
            ("X", "place"),
            ("C", "attack"),
            ("I", "inv"),
        ],
    );
}

fn draw_status_bar(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &BlockworldState,
    colors: &ThemeColors,
) {
    let hp_pct = (state.player_hp as f32 / state.player_max_hp as f32 * 10.0) as usize;
    let hunger_pct = (state.player_hunger as f32 / state.player_max_hunger as f32 * 10.0) as usize;

    let time_icon = match state.get_time_of_day() {
        TimeOfDay::Dawn => "~",
        TimeOfDay::Day => "*",
        TimeOfDay::Dusk => "~",
        TimeOfDay::Night => ".",
    };

    let time_str = format!(
        "Day {} {} {:02}:{:02}",
        state.day_count,
        time_icon,
        (state.time_of_day / 50) % 24,
        (state.time_of_day % 50) * 60 / 50
    );

    let hp_bar = format!(
        "HP:[{}{}]",
        "#".repeat(hp_pct),
        ".".repeat(10 - hp_pct.min(10))
    );

    let hunger_bar = format!(
        "Food:[{}{}]",
        "#".repeat(hunger_pct),
        ".".repeat(10 - hunger_pct.min(10))
    );

    let status = format!(
        " {}  {}  {}  Score:{} ",
        time_str, hp_bar, hunger_bar, state.score
    );

    let hp_color = if hp_pct <= 3 {
        colors.red()
    } else if hp_pct <= 6 {
        colors.yellow()
    } else {
        colors.green()
    };

    view.render_row(
        frame,
        0,
        vec![Span::styled(status, Style::default().fg(hp_color))],
    );
}

fn draw_world(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &BlockworldState,
    colors: &ThemeColors,
) {
    let player_screen_x = (state.player_x as usize).saturating_sub(state.camera_x);
    let player_screen_y = (state.player_y as usize).saturating_sub(state.camera_y);

    let cursor_world_x = state.player_x as i32 + state.cursor_x;
    let cursor_world_y = state.player_y as i32 + state.cursor_y;

    let is_night = state.is_night();

    for row in 0..VIEW_HEIGHT {
        let world_y = state.camera_y + row;
        let mut spans: Vec<Span> = Vec::new();

        for col in 0..VIEW_WIDTH {
            let world_x = state.camera_x + col;

            // Check if this is player position
            if col == player_screen_x && (row == player_screen_y || row == player_screen_y + 1) {
                let ch = if row == player_screen_y {
                    'o' // Head
                } else {
                    '@' // Body
                };
                spans.push(Span::styled(
                    ch.to_string(),
                    Style::default()
                        .fg(colors.cyan())
                        .add_modifier(Modifier::BOLD),
                ));
                continue;
            }

            // Check if cursor
            if world_x as i32 == cursor_world_x && world_y as i32 == cursor_world_y {
                spans.push(Span::styled(
                    "[".to_string(),
                    Style::default()
                        .fg(colors.yellow())
                        .add_modifier(Modifier::BOLD),
                ));
                continue;
            }
            if world_x as i32 == cursor_world_x + 1 && world_y as i32 == cursor_world_y {
                spans.push(Span::styled(
                    "]".to_string(),
                    Style::default()
                        .fg(colors.yellow())
                        .add_modifier(Modifier::BOLD),
                ));
                continue;
            }

            // Check for creatures
            let mut creature_char = None;
            for creature in &state.creatures {
                let cx = (creature.x as usize).saturating_sub(state.camera_x);
                let cy = (creature.y as usize).saturating_sub(state.camera_y);
                if col == cx && row == cy {
                    creature_char = Some((
                        creature.creature_type.char(),
                        creature.creature_type.is_hostile(),
                    ));
                    break;
                }
            }

            if let Some((ch, hostile)) = creature_char {
                let color = if hostile {
                    colors.red()
                } else {
                    colors.green()
                };
                spans.push(Span::styled(
                    ch.to_string(),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ));
                continue;
            }

            // Draw block
            if world_x < state.blocks.len() && world_y < state.blocks[0].len() {
                let block = state.blocks[world_x][world_y];
                let (ch, color) = get_block_display(block, is_night, colors);
                spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
            } else {
                spans.push(Span::raw(" "));
            }
        }

        view.render_row(frame, 1 + row as u16, spans);
    }
}

fn get_block_display(
    block: BlockType,
    is_night: bool,
    colors: &ThemeColors,
) -> (char, ratatui::style::Color) {
    match block {
        BlockType::Air => {
            if is_night {
                (' ', colors.bg())
            } else {
                (' ', colors.bg())
            }
        }
        BlockType::Grass => ('"', colors.green()),
        BlockType::Dirt => ('#', colors.yellow()),
        BlockType::Stone => ('%', colors.grey()),
        BlockType::Bedrock => ('@', colors.fg()),
        BlockType::Wood => ('|', colors.yellow()),
        BlockType::Leaves => ('*', colors.green()),
        BlockType::Coal => ('C', colors.grey()),
        BlockType::Iron => ('I', colors.fg()),
        BlockType::Gold => ('G', colors.yellow()),
        BlockType::Diamond => ('D', colors.cyan()),
        BlockType::Water => ('~', colors.blue()),
        BlockType::Sand => ('.', colors.yellow()),
        BlockType::Planks => ('=', colors.yellow()),
        BlockType::Cobblestone => ('+', colors.grey()),
        BlockType::Torch => ('i', colors.yellow()),
        BlockType::Workbench => ('W', colors.yellow()),
        BlockType::Furnace => ('F', colors.red()),
        BlockType::Chest => ('B', colors.yellow()),
    }
}

fn draw_hotbar(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &BlockworldState,
    colors: &ThemeColors,
) {
    let mut spans: Vec<Span> = vec![Span::raw(" ")];

    for i in 0..9 {
        let is_selected = i == state.selected_slot;
        let bracket_style = if is_selected {
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.grey())
        };

        spans.push(Span::styled(
            if is_selected { "[" } else { " " },
            bracket_style,
        ));

        if let Some(slot) = &state.inventory[i] {
            let item_char = slot.item.char();
            let count_str = if slot.count > 1 {
                format!("{}{}", item_char, slot.count.min(99))
            } else {
                format!("{} ", item_char)
            };
            spans.push(Span::styled(count_str, Style::default().fg(colors.fg())));
        } else {
            spans.push(Span::styled("  ", Style::default().fg(colors.grey())));
        }

        spans.push(Span::styled(
            if is_selected { "]" } else { " " },
            bracket_style,
        ));

        spans.push(Span::styled(
            format!("{}", i + 1),
            Style::default().fg(colors.grey()),
        ));
    }

    // Show selected item name
    if let Some(slot) = &state.inventory[state.selected_slot] {
        spans.push(Span::styled(
            format!("  {}", slot.item.name()),
            Style::default().fg(colors.cyan()),
        ));
    }

    view.render_row(frame, 20, spans);
}

// =============================================================================
// INVENTORY SCREEN
// =============================================================================

fn draw_inventory(frame: &mut Frame, area: Rect, state: &BlockworldState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " INVENTORY ", colors);
    view.render_frame(frame);

    let title_style = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(colors.fg());
    let selected_style = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);

    view.render_row(frame, 0, vec![Span::styled("INVENTORY", title_style)]);

    // Draw inventory grid (9 columns x 4 rows)
    for row in 0..4 {
        let mut spans: Vec<Span> = vec![Span::raw("  ")];

        for col in 0..9 {
            let idx = row * 9 + col;
            let is_selected = idx == state.inventory_cursor;
            let is_hotbar = row == 0;

            let bracket_style = if is_selected {
                selected_style
            } else if is_hotbar {
                Style::default().fg(colors.cyan())
            } else {
                Style::default().fg(colors.grey())
            };

            spans.push(Span::styled(
                if is_selected { ">" } else { "[" },
                bracket_style,
            ));

            if let Some(slot) = &state.inventory[idx] {
                let item_str = format!("{:2}", slot.count.min(99));
                spans.push(Span::styled(
                    format!("{}{}", slot.item.char(), item_str),
                    text_style,
                ));
            } else {
                spans.push(Span::styled("   ", text_style));
            }

            spans.push(Span::styled(
                if is_selected { "<" } else { "]" },
                bracket_style,
            ));
        }

        let row_label = if row == 0 { " HOTBAR" } else { "" };
        spans.push(Span::styled(row_label, Style::default().fg(colors.grey())));

        view.render_row(frame, 2 + row as u16 * 2, spans);
    }

    // Show selected item details
    if let Some(slot) = &state.inventory[state.inventory_cursor] {
        view.render_row(
            frame,
            12,
            vec![Span::styled(
                format!("Selected: {} x{}", slot.item.name(), slot.count),
                title_style,
            )],
        );

        if let Some(dur) = slot.durability {
            view.render_row(
                frame,
                13,
                vec![Span::styled(format!("Durability: {}", dur), text_style)],
            );
        }
    }

    // Stats
    view.render_row(
        frame,
        16,
        vec![Span::styled(
            format!(
                "Blocks mined: {}  Placed: {}  Kills: {}",
                state.blocks_mined, state.blocks_placed, state.creatures_killed
            ),
            text_style,
        )],
    );

    view.render_help(
        frame,
        vec![
            ("Arrows", "move"),
            ("Enter", "select/swap"),
            ("I/Esc", "close"),
        ],
    );
}

// =============================================================================
// CRAFTING SCREEN
// =============================================================================

fn draw_crafting(frame: &mut Frame, area: Rect, _state: &BlockworldState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " CRAFTING ", colors);
    view.render_frame(frame);

    let text_style = Style::default().fg(colors.fg());

    view.render_row(
        frame,
        5,
        vec![Span::styled("Crafting coming soon!", text_style)],
    );

    view.render_help(frame, vec![("Esc", "close")]);
}

// =============================================================================
// PAUSED SCREEN
// =============================================================================

fn draw_paused(frame: &mut Frame, area: Rect, state: &BlockworldState, colors: &ThemeColors) {
    // Draw game in background
    draw_game(frame, area, state, colors);

    // Overlay pause message
    let view = FullScreenView::new(area, " PAUSED ", colors);

    let yellow = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);

    view.render_row(
        frame,
        8,
        vec![Span::styled("+==========================+", yellow)],
    );
    view.render_row(
        frame,
        9,
        vec![Span::styled("|        PAUSED            |", yellow)],
    );
    view.render_row(
        frame,
        10,
        vec![Span::styled("|   ESC to continue        |", yellow)],
    );
    view.render_row(
        frame,
        11,
        vec![Span::styled("|   Q to quit              |", yellow)],
    );
    view.render_row(
        frame,
        12,
        vec![Span::styled("+==========================+", yellow)],
    );
}

// =============================================================================
// GAME OVER SCREEN
// =============================================================================

fn draw_game_over(frame: &mut Frame, area: Rect, state: &BlockworldState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " GAME OVER ", colors);
    view.render_frame(frame);

    let red = Style::default()
        .fg(colors.red())
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(colors.fg());
    let yellow = Style::default().fg(colors.yellow());

    view.render_row(
        frame,
        5,
        vec![Span::styled("+==============================+", red)],
    );
    view.render_row(
        frame,
        6,
        vec![Span::styled("|         GAME OVER            |", red)],
    );
    view.render_row(
        frame,
        7,
        vec![Span::styled("+==============================+", red)],
    );

    view.render_row(
        frame,
        10,
        vec![Span::styled(
            format!("Days Survived: {}", state.day_count),
            text_style,
        )],
    );
    view.render_row(
        frame,
        11,
        vec![Span::styled(
            format!("Blocks Mined: {}", state.blocks_mined),
            text_style,
        )],
    );
    view.render_row(
        frame,
        12,
        vec![Span::styled(
            format!("Final Score: {}", state.score),
            yellow,
        )],
    );

    view.render_row(
        frame,
        15,
        vec![Span::styled("Press ENTER to play again", yellow)],
    );

    view.render_help(frame, vec![("Enter", "restart"), ("Esc", "quit")]);
}
