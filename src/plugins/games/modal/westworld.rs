//! WESTWORLD modal rendering
//!
//! Renders the Contra/Shinobi-style side-scrolling action game.

use crate::app::ThemeColors;
use crate::plugins::games::westworld::{
    EnemyState, TileType, WeaponType, WestworldState, WestworldView, SCREEN_HEIGHT, SCREEN_WIDTH,
};
use crate::ui::components::FullScreenView;
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
    "██╗    ██╗███████╗███████╗████████╗██╗    ██╗ ██████╗ ██████╗ ██╗     ██████╗ ",
    "██║    ██║██╔════╝██╔════╝╚══██╔══╝██║    ██║██╔═══██╗██╔══██╗██║     ██╔══██╗",
    "██║ █╗ ██║█████╗  ███████╗   ██║   ██║ █╗ ██║██║   ██║██████╔╝██║     ██║  ██║",
    "██║███╗██║██╔══╝  ╚════██║   ██║   ██║███╗██║██║   ██║██╔══██╗██║     ██║  ██║",
    "╚███╔███╔╝███████╗███████║   ██║   ╚███╔███╔╝╚██████╔╝██║  ██║███████╗██████╔╝",
    " ╚══╝╚══╝ ╚══════╝╚══════╝   ╚═╝    ╚══╝╚══╝  ╚═════╝ ╚═╝  ╚═╝╚══════╝╚═════╝ ",
];

// =============================================================================
// MAIN DRAW FUNCTION
// =============================================================================

pub fn draw_westworld(frame: &mut Frame, area: Rect, state: &WestworldState, colors: &ThemeColors) {
    match state.view {
        WestworldView::Menu => draw_menu(frame, area, state, colors),
        WestworldView::Playing => draw_game(frame, area, state, colors),
        WestworldView::Paused => draw_paused(frame, area, state, colors),
        WestworldView::GameOver => draw_game_over(frame, area, state, colors),
        WestworldView::Victory => draw_victory(frame, area, state, colors),
    }
}

// =============================================================================
// MENU SCREEN
// =============================================================================

fn draw_menu(frame: &mut Frame, area: Rect, state: &WestworldState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " WESTWORLD ", colors);
    view.render_frame(frame);

    let text_style = Style::default().fg(colors.fg());
    let highlight = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);

    // Title art with animated sunset gradient
    let phase = (state.tick_count / 10) % 3;
    for (i, line) in TITLE_ART.iter().enumerate() {
        let row_color = match (i, phase) {
            (0..=1, 0) => colors.yellow(), // Sunset sky
            (0..=1, 1) => colors.fg(),     // Bright flash
            (0..=1, _) => colors.yellow(),
            (2..=3, 0) => colors.red(),    // Sunset red
            (2..=3, 1) => colors.yellow(), // Orange highlight
            (2..=3, _) => colors.red(),
            (_, 0) => colors.grey(), // Dusty horizon
            (_, 1) => colors.red(),  // Dust highlight
            (_, _) => colors.grey(),
        };
        let style = Style::default().fg(row_color).add_modifier(Modifier::BOLD);
        view.render_row(frame, i as u16, vec![Span::styled(*line, style)]);
    }

    view.render_row(
        frame,
        7,
        vec![Span::styled(
            "          Android Uprising - A Contra-Style Action Game",
            text_style,
        )],
    );

    // Instructions
    view.render_row(frame, 10, vec![Span::styled("  CONTROLS:", highlight)]);
    view.render_row(
        frame,
        11,
        vec![Span::styled(
            "  Arrow Keys/WASD - Move    Space/W - Jump",
            text_style,
        )],
    );
    view.render_row(
        frame,
        12,
        vec![Span::styled(
            "  Z - Shoot                 X - Switch Weapon",
            text_style,
        )],
    );

    view.render_row(frame, 14, vec![Span::styled("  OBJECTIVE:", highlight)]);
    view.render_row(
        frame,
        15,
        vec![Span::styled(
            "  Defeat enemies, free Hosts, destroy the Sheriff!",
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

// =============================================================================
// GAME SCREEN
// =============================================================================

fn draw_game(frame: &mut Frame, area: Rect, state: &WestworldState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " WESTWORLD ", colors);
    view.render_frame(frame);

    // Status bar (row 0)
    draw_status_bar(frame, &view, state, colors);

    // Game world (rows 1-18)
    draw_world(frame, &view, state, colors);

    // Message overlay
    if let Some(msg) = &state.message {
        let msg_style = Style::default()
            .fg(colors.yellow())
            .add_modifier(Modifier::BOLD);
        view.render_row(
            frame,
            SCREEN_HEIGHT as u16 - 1,
            vec![Span::styled(format!("  {}", msg), msg_style)],
        );
    }

    view.render_help(
        frame,
        vec![
            ("Arrows", "move"),
            ("Space", "jump"),
            ("Z", "shoot"),
            ("X", "weapon"),
            ("Esc", "pause"),
        ],
    );
}

fn draw_status_bar(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &WestworldState,
    colors: &ThemeColors,
) {
    let hp_pct = (state.player_hp * 10 / state.player_max_hp).max(0) as usize;
    let hp_bar = format!(
        "HP:[{}{}]",
        "#".repeat(hp_pct.min(10)),
        ".".repeat(10 - hp_pct.min(10))
    );

    let weapon_name = match state.current_weapon {
        WeaponType::Revolver => "REV",
        WeaponType::Shotgun => "SHT",
        WeaponType::Rifle => "RIF",
        WeaponType::Katana => "KAT",
    };

    let status = format!(
        " {} AMMO:{:3} [{:3}] Lives:{} Score:{:5} Hosts:{}/{} Zone:{}",
        hp_bar,
        state.ammo,
        weapon_name,
        state.lives,
        state.score,
        state.hosts_freed,
        state.total_hosts,
        state.current_zone.name()
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
    state: &WestworldState,
    colors: &ThemeColors,
) {
    let camera_x = state.camera_x;

    for row in 0..SCREEN_HEIGHT {
        let mut spans: Vec<Span> = Vec::new();

        for col in 0..SCREEN_WIDTH {
            let world_x = camera_x + col;
            let world_y = row;

            // Check for player
            let player_screen_x = (state.player_x as usize).saturating_sub(camera_x);
            let player_screen_y = state.player_y as usize;

            if col == player_screen_x && row == player_screen_y {
                // Draw player
                let player_char = if state.invincible_frames > 0 && state.tick_count % 4 < 2 {
                    ' ' // Blink when invincible
                } else {
                    '@'
                };
                spans.push(Span::styled(
                    player_char.to_string(),
                    Style::default()
                        .fg(colors.cyan())
                        .add_modifier(Modifier::BOLD),
                ));
                continue;
            }

            // Check for bullets
            let mut bullet_char = None;
            for bullet in &state.bullets {
                let bx = (bullet.x as usize).saturating_sub(camera_x);
                let by = bullet.y as usize;
                if col == bx && row == by {
                    bullet_char = Some(if bullet.friendly { '-' } else { '*' });
                    break;
                }
            }

            if let Some(ch) = bullet_char {
                let color = if ch == '-' {
                    colors.yellow()
                } else {
                    colors.red()
                };
                spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
                continue;
            }

            // Check for enemies
            let mut enemy_char = None;
            for enemy in &state.enemies {
                if enemy.state == EnemyState::Dead {
                    continue;
                }
                let ex = (enemy.x as usize).saturating_sub(camera_x);
                let ey = enemy.y as usize;
                if col == ex && row == ey {
                    enemy_char = Some((enemy.enemy_type.char(), enemy.enemy_type.is_boss()));
                    break;
                }
            }

            if let Some((ch, is_boss)) = enemy_char {
                let color = if is_boss {
                    colors.red()
                } else {
                    colors.yellow()
                };
                spans.push(Span::styled(
                    ch.to_string(),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ));
                continue;
            }

            // Check for pickups
            let mut pickup_char = None;
            for pickup in &state.pickups {
                if pickup.collected {
                    continue;
                }
                let px = (pickup.x as usize).saturating_sub(camera_x);
                let py = pickup.y as usize;
                if col == px && row == py {
                    pickup_char = Some(pickup.pickup_type.char());
                    break;
                }
            }

            if let Some(ch) = pickup_char {
                spans.push(Span::styled(
                    ch.to_string(),
                    Style::default()
                        .fg(colors.green())
                        .add_modifier(Modifier::BOLD),
                ));
                continue;
            }

            // Draw tile
            if world_x < state.tiles.len() && world_y < state.tiles[world_x].len() {
                let tile = state.tiles[world_x][world_y];
                let (ch, color) = get_tile_display(tile, colors);
                spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
            } else {
                spans.push(Span::raw(" "));
            }
        }

        view.render_row(frame, 1 + row as u16, spans);
    }
}

fn get_tile_display(tile: TileType, colors: &ThemeColors) -> (char, ratatui::style::Color) {
    match tile {
        TileType::Air => (' ', colors.bg()),
        TileType::Ground => ('=', colors.yellow()),
        TileType::Platform => ('-', colors.grey()),
        TileType::Wall => ('#', colors.fg()),
        TileType::Cactus => ('Y', colors.green()),
        TileType::Building => ('%', colors.cyan()),
        TileType::Saloon => ('M', colors.red()),
    }
}

// =============================================================================
// PAUSED SCREEN
// =============================================================================

fn draw_paused(frame: &mut Frame, area: Rect, state: &WestworldState, colors: &ThemeColors) {
    // Draw game in background
    draw_game(frame, area, state, colors);

    let view = FullScreenView::new(area, " PAUSED ", colors);

    let yellow = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);

    view.render_row(
        frame,
        8,
        vec![Span::styled("  +===================+", yellow)],
    );
    view.render_row(
        frame,
        9,
        vec![Span::styled("  |     PAUSED        |", yellow)],
    );
    view.render_row(
        frame,
        10,
        vec![Span::styled(" +===================+", yellow)],
    );

    view.render_help(frame, vec![("P/Esc", "resume"), ("Q", "quit")]);
}

// =============================================================================
// GAME OVER SCREEN
// =============================================================================

fn draw_game_over(frame: &mut Frame, area: Rect, state: &WestworldState, colors: &ThemeColors) {
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
        vec![Span::styled("|        GAME OVER             |", red)],
    );
    view.render_row(
        frame,
        7,
        vec![Span::styled("+==============================+", red)],
    );

    view.render_row(
        frame,
        9,
        vec![Span::styled(
            format!("  Final Score: {}", state.score),
            yellow,
        )],
    );
    view.render_row(
        frame,
        10,
        vec![Span::styled(
            format!("  Hosts Freed: {}/{}", state.hosts_freed, state.total_hosts),
            text_style,
        )],
    );
    view.render_row(
        frame,
        11,
        vec![Span::styled(
            format!("  Zone Reached: {}", state.current_zone.name()),
            text_style,
        )],
    );

    view.render_row(
        frame,
        14,
        vec![Span::styled(
            "  The machines have won... for now.",
            text_style,
        )],
    );

    view.render_row(
        frame,
        16,
        vec![Span::styled(
            "  Press ENTER to try again",
            Style::default()
                .fg(colors.green())
                .add_modifier(Modifier::BOLD),
        )],
    );

    view.render_help(frame, vec![("Enter", "retry"), ("Esc", "quit")]);
}

// =============================================================================
// VICTORY SCREEN
// =============================================================================

fn draw_victory(frame: &mut Frame, area: Rect, state: &WestworldState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " VICTORY ", colors);
    view.render_frame(frame);

    let green = Style::default()
        .fg(colors.green())
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(colors.fg());
    let yellow = Style::default().fg(colors.yellow());

    view.render_row(
        frame,
        5,
        vec![Span::styled("+==============================+", green)],
    );
    view.render_row(
        frame,
        6,
        vec![Span::styled("|       V I C T O R Y !        |", green)],
    );
    view.render_row(
        frame,
        7,
        vec![Span::styled("+==============================+", green)],
    );

    view.render_row(
        frame,
        9,
        vec![Span::styled(
            format!("  Final Score: {}", state.score),
            yellow,
        )],
    );
    view.render_row(
        frame,
        10,
        vec![Span::styled(
            format!("  Hosts Freed: {}/{}", state.hosts_freed, state.total_hosts),
            text_style,
        )],
    );

    if state.hosts_freed == state.total_hosts {
        view.render_row(
            frame,
            12,
            vec![Span::styled(
                "  TRUE ENDING: All Hosts are free!",
                Style::default()
                    .fg(colors.cyan())
                    .add_modifier(Modifier::BOLD),
            )],
        );
        view.render_row(
            frame,
            13,
            vec![Span::styled("  Bonus: +500 points", yellow)],
        );
    }

    view.render_row(
        frame,
        15,
        vec![Span::styled(
            "  The Sheriff has fallen. Sweetwater is free.",
            text_style,
        )],
    );

    view.render_row(
        frame,
        17,
        vec![Span::styled(
            "  Press ENTER to play again",
            Style::default()
                .fg(colors.green())
                .add_modifier(Modifier::BOLD),
        )],
    );

    view.render_help(frame, vec![("Enter", "play again"), ("Esc", "quit")]);
}
