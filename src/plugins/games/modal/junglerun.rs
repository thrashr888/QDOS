//! JUNGLE RUN modal rendering
//!
//! Renders the side-scrolling platformer with hazards and treasures.

use qdos_plugin_api::ui::FullScreenView;
use qdos_plugin_api::ThemeColors;
use ratatui::{
    prelude::*,
    style::{Modifier, Style},
    text::Span,
    Frame,
};

use super::super::junglerun::{HazardType, JungleRunState, JungleRunView, PlayerState};

/// Main draw function for JUNGLE RUN
pub fn draw(frame: &mut Frame, area: Rect, state: &JungleRunState, colors: &ThemeColors) {
    match state.view {
        JungleRunView::Menu => draw_menu(frame, area, colors),
        JungleRunView::Playing => draw_game(frame, area, state, colors),
        JungleRunView::Paused => draw_paused(frame, area, state, colors),
        JungleRunView::GameOver => draw_game_over(frame, area, state, colors),
    }
}

/// Draw the menu screen
fn draw_menu(frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " JUNGLE RUN ", colors);
    view.render_frame(frame);

    let title_style = Style::default()
        .fg(colors.green())
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(colors.fg());
    let highlight = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);

    // Title art
    let title = [
        r"     ██╗██╗   ██╗███╗   ██╗ ██████╗ ██╗     ███████╗",
        r"     ██║██║   ██║████╗  ██║██╔════╝ ██║     ██╔════╝",
        r"     ██║██║   ██║██╔██╗ ██║██║  ███╗██║     █████╗  ",
        r"██   ██║██║   ██║██║╚██╗██║██║   ██║██║     ██╔══╝  ",
        r"╚█████╔╝╚██████╔╝██║ ╚████║╚██████╔╝███████╗███████╗",
        r" ╚════╝  ╚═════╝ ╚═╝  ╚═══╝ ╚═════╝ ╚══════╝╚══════╝",
    ];

    for (i, line) in title.iter().enumerate() {
        view.render_row(frame, i as u16 + 1, vec![Span::styled(*line, title_style)]);
    }

    view.render_row(
        frame,
        8,
        vec![Span::styled("Pitfall-Style Platformer", highlight)],
    );
    view.render_row(
        frame,
        10,
        vec![Span::styled(
            "Jump over pits, avoid hazards, collect treasure!",
            text_style,
        )],
    );

    // Instructions
    view.render_row(frame, 12, vec![Span::styled("HOW TO PLAY:", title_style)]);
    view.render_row(
        frame,
        13,
        vec![Span::styled(
            "  Arrow keys or WASD to move left/right",
            text_style,
        )],
    );
    view.render_row(
        frame,
        14,
        vec![Span::styled("  Space or Up arrow to jump", text_style)],
    );
    view.render_row(
        frame,
        15,
        vec![Span::styled(
            "  Collect treasures: $ Gold (+200) * Diamond (+500)",
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

/// Draw the main game screen
fn draw_game(frame: &mut Frame, area: Rect, state: &JungleRunState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " JUNGLE RUN ", colors);
    view.render_frame(frame);

    let text_style = Style::default().fg(colors.fg());
    let yellow = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let red = Style::default().fg(colors.red());

    // HUD - Row 1
    let lives_str = "♥".repeat(state.lives as usize);
    let time_color = if state.time_remaining < 300 {
        red
    } else if state.time_remaining < 600 {
        yellow
    } else {
        text_style
    };

    view.render_row(
        frame,
        1,
        vec![
            Span::styled("SCORE: ", text_style),
            Span::styled(format!("{:<8}", state.score), yellow),
            Span::styled("LIVES: ", text_style),
            Span::styled(format!("{:<6}", lives_str), red),
            Span::styled("TIME: ", text_style),
            Span::styled(format!("{:<8}", state.time_string()), time_color),
            Span::styled("SCREEN: ", text_style),
            Span::styled(format!("{}/{}", state.current_screen + 1, 32), text_style),
        ],
    );

    // Get screen data
    let screen = match state.current_screen_data() {
        Some(s) => s,
        None => return,
    };

    // Sky with clouds and sun - Row 3
    let sky_row = draw_sky(state.tick_count, area.width as usize);
    view.render_row(frame, 3, sky_row);

    // Trees/jungle canopy - Row 4
    let canopy = draw_canopy(state.current_screen, area.width as usize, colors);
    view.render_row(frame, 4, canopy);

    // Treasure row - Row 6
    let treasure_row = draw_treasures(screen, state, area.width as usize, colors);
    view.render_row(frame, 6, treasure_row);

    // Platform/ground row - Rows 8-10
    let (ground_top, ground_mid, water_row) =
        draw_ground(screen, state, area.width as usize, colors);
    view.render_row(frame, 8, ground_top);
    view.render_row(frame, 9, ground_mid);
    view.render_row(frame, 10, water_row);

    // Player - positioned based on player_y
    let player_row = 8 + (state.player_y - 8.0).clamp(0.0, 3.0) as u16;
    let player_char = get_player_char(state);
    let player_x = (state.player_x * 2.0) as usize; // Scale to screen

    // Build player row
    let mut player_spans = Vec::new();
    if player_x > 0 {
        player_spans.push(Span::raw(" ".repeat(player_x.min(70))));
    }
    player_spans.push(Span::styled(player_char, yellow));
    view.render_row(frame, player_row, player_spans);

    // Message display
    if let Some(msg) = &state.message {
        view.render_row(
            frame,
            12,
            vec![Span::styled(
                format!("{:^40}", msg),
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            )],
        );
    }

    // Ground/foundation - Row 14
    view.render_row(
        frame,
        14,
        vec![Span::styled(
            "▓".repeat(area.width as usize - 4),
            Style::default().fg(colors.grey()),
        )],
    );

    view.render_help(
        frame,
        vec![
            ("←→", "move"),
            ("Space", "jump"),
            ("P", "pause"),
            ("Esc", "menu"),
        ],
    );
}

/// Draw paused overlay
fn draw_paused(frame: &mut Frame, area: Rect, state: &JungleRunState, colors: &ThemeColors) {
    // Draw game underneath
    draw_game(frame, area, state, colors);

    let view = FullScreenView::new(area, " PAUSED ", colors);

    let yellow = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);

    view.render_row(
        frame,
        10,
        vec![Span::styled("╔══════════════════════════╗", yellow)],
    );
    view.render_row(
        frame,
        11,
        vec![Span::styled("║        PAUSED            ║", yellow)],
    );
    view.render_row(
        frame,
        12,
        vec![Span::styled("║   Press P to continue    ║", yellow)],
    );
    view.render_row(
        frame,
        13,
        vec![Span::styled("╚══════════════════════════╝", yellow)],
    );
}

/// Draw game over screen
fn draw_game_over(frame: &mut Frame, area: Rect, state: &JungleRunState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " JUNGLE RUN ", colors);
    view.render_frame(frame);

    let text_style = Style::default().fg(colors.fg());
    let yellow = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let green = Style::default()
        .fg(colors.green())
        .add_modifier(Modifier::BOLD);
    let red = Style::default()
        .fg(colors.red())
        .add_modifier(Modifier::BOLD);

    if state.game_won {
        view.render_row(
            frame,
            5,
            vec![Span::styled("╔══════════════════════════════════╗", green)],
        );
        view.render_row(
            frame,
            6,
            vec![Span::styled("║       JUNGLE CONQUERED!          ║", green)],
        );
        view.render_row(
            frame,
            7,
            vec![Span::styled("╚══════════════════════════════════╝", green)],
        );
    } else {
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
    }

    view.render_row(frame, 9, vec![Span::styled("FINAL STATS", yellow)]);
    view.render_row(frame, 10, vec![Span::styled("───────────", text_style)]);

    view.render_row(
        frame,
        11,
        vec![Span::styled(
            format!("Final Score: {}", state.score),
            text_style,
        )],
    );
    view.render_row(
        frame,
        12,
        vec![Span::styled(
            format!("Screens Cleared: {}/32", state.current_screen),
            text_style,
        )],
    );
    view.render_row(
        frame,
        13,
        vec![Span::styled(
            format!("Treasures Collected: {}", state.treasures_collected),
            text_style,
        )],
    );

    view.render_row(
        frame,
        15,
        vec![Span::styled("Press ENTER to try again", yellow)],
    );

    view.render_help(frame, vec![("Enter", "restart"), ("Esc", "quit")]);
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Draw the sky with clouds
fn draw_sky(tick: u32, _width: usize) -> Vec<Span<'static>> {
    let cloud_offset = (tick / 10) % 40;
    let mut sky = String::new();

    // Simple animated clouds
    for i in 0..40 {
        let pos = (i + cloud_offset as usize) % 40;
        if pos == 5 || pos == 6 {
            sky.push_str("= ");
        } else if pos == 15 {
            sky.push_str("* ");
        } else if pos == 25 || pos == 26 {
            sky.push_str("= ");
        } else {
            sky.push_str("  ");
        }
    }

    vec![Span::styled(
        sky,
        Style::default().fg(Color::Rgb(135, 206, 235)),
    )]
}

/// Draw the jungle canopy
fn draw_canopy(screen: usize, _width: usize, colors: &ThemeColors) -> Vec<Span<'static>> {
    let green = Style::default().fg(colors.green());

    // Vary canopy based on screen - ASCII palm trees
    let pattern = match screen % 4 {
        0 => "Y    YY      Y        Y    YY    Y        Y    YY",
        1 => "  Y      YY      YY      Y    Y    YY      Y    ",
        2 => "YY    Y      Y    YY    Y      YY    Y      YY",
        _ => "    YY    Y      YY    YY      Y    YY      Y  ",
    };

    vec![Span::styled(pattern.to_string(), green)]
}

/// Draw treasures on the screen
fn draw_treasures(
    screen: &super::super::junglerun::Screen,
    _state: &JungleRunState,
    width: usize,
    colors: &ThemeColors,
) -> Vec<Span<'static>> {
    let mut row = vec![' '; width.min(80)];
    let yellow = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);

    for treasure in &screen.treasures {
        if treasure.collected {
            continue;
        }

        let x = (treasure.x * 2.0) as usize;
        if x < row.len() {
            row[x] = treasure.treasure_type.char();
        }
    }

    let row_str: String = row.iter().collect();
    vec![Span::styled(row_str, yellow)]
}

/// Draw the ground, platforms, and water
fn draw_ground(
    screen: &super::super::junglerun::Screen,
    state: &JungleRunState,
    width: usize,
    colors: &ThemeColors,
) -> (Vec<Span<'static>>, Vec<Span<'static>>, Vec<Span<'static>>) {
    let ground_style = Style::default().fg(colors.green());
    let water_style = Style::default().fg(colors.blue());

    let mut top_row = vec!['█'; width.min(80)];
    let mut mid_row = vec!['█'; width.min(80)];
    let mut water_row = vec!['▓'; width.min(80)];

    let is_croc_open = state.is_croc_open();
    let log_offset = state.log_offset();

    for hazard in &screen.hazards {
        let x = (hazard.x * 2.0) as usize;
        let w = (hazard.width * 2.0) as usize;

        match hazard.hazard_type {
            HazardType::Pit => {
                // Create gap in ground
                for i in x..(x + w).min(top_row.len()) {
                    if i < top_row.len() {
                        top_row[i] = ' ';
                        mid_row[i] = ' ';
                        water_row[i] = '░';
                    }
                }
            }
            HazardType::Crocodile => {
                // Water area with crocodile
                for i in x..(x + w).min(top_row.len()) {
                    if i < top_row.len() {
                        top_row[i] = ' ';
                        mid_row[i] = '≈';
                    }
                }
                // Draw croc
                let croc_x = x + 1;
                if croc_x < water_row.len() {
                    if is_croc_open {
                        water_row[croc_x] = '<';
                        if croc_x + 1 < water_row.len() {
                            water_row[croc_x + 1] = '>';
                        }
                    } else {
                        water_row[croc_x] = '=';
                        if croc_x + 1 < water_row.len() {
                            water_row[croc_x + 1] = '=';
                        }
                    }
                }
            }
            HazardType::RollingLog => {
                // Rolling log position
                let log_x = (x as f32 + log_offset) as usize;
                if log_x > 0 && log_x < top_row.len() {
                    top_row[log_x] = '○';
                    if log_x + 1 < top_row.len() {
                        top_row[log_x + 1] = '═';
                    }
                    if log_x + 2 < top_row.len() {
                        top_row[log_x + 2] = '○';
                    }
                }
            }
        }
    }

    let top_str: String = top_row.iter().collect();
    let mid_str: String = mid_row.iter().collect();
    let water_str: String = water_row.iter().collect();

    (
        vec![Span::styled(top_str, ground_style)],
        vec![Span::styled(mid_str, ground_style)],
        vec![Span::styled(water_str, water_style)],
    )
}

/// Get the player character based on state
fn get_player_char(state: &JungleRunState) -> &'static str {
    match state.player_state {
        PlayerState::Idle => "@",
        PlayerState::Running => match state.run_frame % 4 {
            0 => "@",
            1 => "Ø",
            2 => "@",
            _ => "Ø",
        },
        PlayerState::Jumping => "^",
        PlayerState::Falling => "v",
        PlayerState::Dead => "X",
    }
}
