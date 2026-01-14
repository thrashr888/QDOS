//! NEON DRIVE - Cyberpunk Racing Modal Rendering
//!
//! Pseudo-3D road rendering with perspective scaling.

use crate::app::ThemeColors;
use crate::plugins::games::neondrive::{NeondriveState, NeondriveView, ObstacleKind};
use crate::ui::components::FullScreenView;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    Frame,
};

// Road rendering constants
const ROAD_ROWS: usize = 15;
const BASE_ROAD_WIDTH: usize = 30;
const HORIZON_WIDTH: usize = 10;

pub fn draw(frame: &mut Frame, area: Rect, state: &NeondriveState, colors: &ThemeColors) {
    match state.view {
        NeondriveView::Menu => draw_menu(frame, area, colors),
        NeondriveView::Playing => draw_game(frame, area, state, colors),
        NeondriveView::GameOver => draw_game_over(frame, area, state, colors),
    }
}

fn draw_menu(frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " NEON DRIVE ", colors);
    view.render_frame(frame);

    let mut row = 2;

    // Title art
    let title_lines = [
        "  ███╗   ██╗███████╗ ██████╗ ███╗   ██╗",
        "  ████╗  ██║██╔════╝██╔═══██╗████╗  ██║",
        "  ██╔██╗ ██║█████╗  ██║   ██║██╔██╗ ██║",
        "  ██║╚██╗██║██╔══╝  ██║   ██║██║╚██╗██║",
        "  ██║ ╚████║███████╗╚██████╔╝██║ ╚████║",
        "  ╚═╝  ╚═══╝╚══════╝ ╚═════╝ ╚═╝  ╚═══╝",
        "",
        "       ██████╗ ██████╗ ██╗██╗   ██╗███████╗",
        "       ██╔══██╗██╔══██╗██║██║   ██║██╔════╝",
        "       ██║  ██║██████╔╝██║██║   ██║█████╗  ",
        "       ██║  ██║██╔══██╗██║╚██╗ ██╔╝██╔══╝  ",
        "       ██████╔╝██║  ██║██║ ╚████╔╝ ███████╗",
        "       ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═══╝  ╚══════╝",
    ];

    for line in &title_lines {
        view.render_row(
            frame,
            row,
            vec![Span::styled(*line, Style::default().fg(colors.cyan()))],
        );
        row += 1;
    }

    row += 1;

    // Subtitle
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "          CYBERPUNK RACING",
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );
    row += 2;

    // Instructions
    let instructions = [
        ("←→ / A D", "Change lanes"),
        ("↑↓ / W S", "Accelerate / Brake"),
        ("Space", "Nitro boost"),
    ];

    for (key, desc) in &instructions {
        view.render_row(
            frame,
            row,
            vec![
                Span::styled(
                    format!("     {:^12}", key),
                    Style::default()
                        .fg(colors.green())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!(" - {}", desc), Style::default().fg(colors.fg())),
            ],
        );
        row += 1;
    }

    view.render_help(frame, vec![("Enter", "start"), ("Esc", "quit")]);
}

fn draw_game(frame: &mut Frame, area: Rect, state: &NeondriveState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " NEON DRIVE ", colors);
    view.render_frame(frame);

    // HUD - Row 0
    draw_hud(frame, &view, state, colors);

    // Road - Rows 1-16
    draw_road(frame, &view, state, colors);

    // Controls
    view.render_help(
        frame,
        vec![
            ("←→", "steer"),
            ("↑↓", "speed"),
            ("Space", "nitro"),
            ("P", "pause"),
            ("Esc", "quit"),
        ],
    );
}

fn draw_hud(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &NeondriveState,
    colors: &ThemeColors,
) {
    // Speed gauge
    let speed_pct = (state.speed / 350.0 * 10.0) as usize;
    let nitro_charges = state.nitro.floor() as usize;
    let nitro_partial = ((state.nitro - state.nitro.floor()) * 3.0) as usize;

    // Heat stars
    let heat_filled = state.heat as usize;
    let heat_empty = 5 - heat_filled;

    view.render_row(
        frame,
        0,
        vec![
            Span::styled(" SPEED: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{:3.0}", state.speed),
                Style::default()
                    .fg(if state.nitro_active {
                        colors.yellow()
                    } else {
                        colors.cyan()
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" KPH ", Style::default().fg(colors.grey())),
            Span::styled(
                "█".repeat(speed_pct),
                Style::default().fg(if state.nitro_active {
                    colors.yellow()
                } else {
                    colors.cyan()
                }),
            ),
            Span::styled(
                "░".repeat(10_usize.saturating_sub(speed_pct)),
                Style::default().fg(colors.grey()),
            ),
            Span::styled("  NITRO: ", Style::default().fg(colors.grey())),
            Span::styled(
                "███".repeat(nitro_charges),
                Style::default().fg(colors.green()),
            ),
            Span::styled(
                "█".repeat(nitro_partial),
                Style::default().fg(colors.green()),
            ),
            Span::styled(
                "░".repeat(9_usize.saturating_sub(nitro_charges * 3 + nitro_partial)),
                Style::default().fg(colors.grey()),
            ),
            Span::styled("  HEAT: ", Style::default().fg(colors.grey())),
            Span::styled("★".repeat(heat_filled), Style::default().fg(colors.red())),
            Span::styled("☆".repeat(heat_empty), Style::default().fg(colors.grey())),
            Span::styled(
                format!("  SCORE: {:>8}", state.score),
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            ),
        ],
    );
}

fn draw_road(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &NeondriveState,
    colors: &ThemeColors,
) {
    let content_width: usize = 78; // Standard content area width
    let center = content_width / 2;

    // Draw rows from horizon (row 1) to near player (row 15)
    for row_idx in 0..ROAD_ROWS {
        let row = (row_idx + 1) as u16; // Offset for HUD

        // Perspective scaling: further = narrower
        let t = row_idx as f32 / ROAD_ROWS as f32;
        let road_width = HORIZON_WIDTH as f32 + (BASE_ROAD_WIDTH - HORIZON_WIDTH) as f32 * t;
        let road_half = (road_width / 2.0) as usize;

        // Calculate lane positions
        let lane_width = road_width / 5.0;

        // Build the row
        let mut line_chars: Vec<(char, Style)> = vec![(' ', Style::default()); content_width];

        // Road surface
        let road_start = center.saturating_sub(road_half);
        let road_end = (center + road_half).min(content_width);

        for item in line_chars.iter_mut().take(road_end).skip(road_start) {
            *item = ('═', Style::default().fg(colors.grey()));
        }

        // Lane dividers
        let road_offset_row = (state.road_offset + row_idx as f32 * 2.0) as usize % 10;
        let show_divider = road_offset_row < 6; // Dashed lines

        if show_divider {
            for lane in 1..5 {
                let lane_x = road_start as f32 + lane as f32 * lane_width;
                let x = lane_x as usize;
                if x < content_width {
                    line_chars[x] = ('│', Style::default().fg(colors.yellow()));
                }
            }
        }

        // Road edges (neon glow effect)
        let edge_color = if (state.tick_count / 2 + row_idx as u32).is_multiple_of(2) {
            colors.cyan()
        } else {
            colors.fg() // Use white instead of magenta
        };

        if road_start > 0 {
            line_chars[road_start] = ('╱', Style::default().fg(edge_color));
        }
        if road_end < content_width {
            line_chars[road_end.saturating_sub(1)] = ('╲', Style::default().fg(edge_color));
        }

        // Draw obstacles at this distance
        let distance_at_row = ROAD_ROWS as f32 - row_idx as f32; // 15 at horizon, 0 at player
        let distance_range = 100.0 / ROAD_ROWS as f32;
        let min_dist = distance_at_row * distance_range - distance_range / 2.0;
        let max_dist = distance_at_row * distance_range + distance_range / 2.0;

        for obstacle in &state.obstacles {
            if obstacle.distance >= min_dist && obstacle.distance < max_dist {
                // Calculate obstacle screen position
                let lane_center = road_start as f32 + (obstacle.lane as f32 + 0.5) * lane_width;
                let obs_x = lane_center as usize;

                if obs_x < content_width.saturating_sub(2) {
                    let (char1, char2, obs_color) = match obstacle.kind {
                        ObstacleKind::Car => ('▓', '▓', colors.fg()),
                        ObstacleKind::Van => ('█', '█', colors.grey()),
                        ObstacleKind::Barrier => ('╳', '╳', colors.red()),
                    };
                    line_chars[obs_x] = (char1, Style::default().fg(obs_color));
                    if obs_x + 1 < content_width {
                        line_chars[obs_x + 1] = (char2, Style::default().fg(obs_color));
                    }
                }
            }
        }

        // Draw player car at the bottom rows
        if row_idx >= ROAD_ROWS - 3 {
            let player_lane_x =
                road_start as f32 + (state.lane as f32 + state.lane_offset + 0.5) * lane_width;
            let car_x = player_lane_x as usize;

            if car_x > 0 && car_x < content_width.saturating_sub(3) {
                let car_row = row_idx - (ROAD_ROWS - 3);
                let car_chars: [(char, char, char); 3] = [
                    ('╔', '═', '╗'), // Top: ╔═╗
                    ('║', '◊', '║'), // Middle: ║◊║
                    ('╚', '▓', '╝'), // Bottom: ╚▓╝
                ];

                let (c1, c2, c3) = car_chars[car_row];
                let car_color = if state.nitro_active {
                    colors.yellow()
                } else {
                    colors.cyan()
                };

                line_chars[car_x - 1] = (c1, Style::default().fg(car_color));
                line_chars[car_x] = (c2, Style::default().fg(car_color));
                line_chars[car_x + 1] = (c3, Style::default().fg(car_color));

                // Nitro flames
                if state.nitro_active && car_row == 2 && car_x > 3 {
                    let flame_char = if state.tick_count % 4 < 2 {
                        '🔥'
                    } else {
                        '💨'
                    };
                    // Use ASCII for compatibility
                    line_chars[car_x - 3] = ('*', Style::default().fg(colors.red()));
                    line_chars[car_x - 2] = (
                        if state.tick_count % 4 < 2 { '~' } else { '-' },
                        Style::default().fg(colors.yellow()),
                    );
                    let _ = flame_char; // Suppress unused warning
                }
            }
        }

        // Convert to Spans
        let spans: Vec<Span> = line_chars
            .iter()
            .map(|(c, style)| Span::styled(c.to_string(), *style))
            .collect();

        view.render_row(frame, row, spans);
    }

    // Add some building silhouettes at horizon
    draw_cityscape(frame, view, state, colors);
}

fn draw_cityscape(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &NeondriveState,
    colors: &ThemeColors,
) {
    // Draw neon signs on sides (rows 2-5)
    let signs = [
        ("NEXUS", colors.cyan()),
        ("CYBER", colors.yellow()),
        ("RAMEN", colors.green()),
        ("HOTEL", colors.red()),
    ];

    let content_width: usize = 78;
    let sign_idx = ((state.distance / 50.0) as usize) % signs.len();
    let (sign_text, sign_color) = signs[sign_idx];

    // Left building sign (only on certain rows for depth)
    for row in 2..=4 {
        if row == 3 {
            view.render_row(
                frame,
                row,
                vec![
                    Span::styled(" █ ", Style::default().fg(colors.grey())),
                    Span::styled(
                        sign_text,
                        Style::default().fg(sign_color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        " █".to_string() + &" ".repeat(content_width.saturating_sub(10)),
                        Style::default().fg(colors.grey()),
                    ),
                ],
            );
        }
    }
}

fn draw_game_over(frame: &mut Frame, area: Rect, state: &NeondriveState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " NEON DRIVE ", colors);
    view.render_frame(frame);

    let mut row = 4;

    // Game over text
    let game_over_lines = [
        "   ██████╗  █████╗ ███╗   ███╗███████╗",
        "  ██╔════╝ ██╔══██╗████╗ ████║██╔════╝",
        "  ██║  ███╗███████║██╔████╔██║█████╗  ",
        "  ██║   ██║██╔══██║██║╚██╔╝██║██╔══╝  ",
        "  ╚██████╔╝██║  ██║██║ ╚═╝ ██║███████╗",
        "   ╚═════╝ ╚═╝  ╚═╝╚═╝     ╚═╝╚══════╝",
        "",
        "   ██████╗ ██╗   ██╗███████╗██████╗ ",
        "  ██╔═══██╗██║   ██║██╔════╝██╔══██╗",
        "  ██║   ██║██║   ██║█████╗  ██████╔╝",
        "  ██║   ██║╚██╗ ██╔╝██╔══╝  ██╔══██╗",
        "  ╚██████╔╝ ╚████╔╝ ███████╗██║  ██║",
        "   ╚═════╝   ╚═══╝  ╚══════╝╚═╝  ╚═╝",
    ];

    for line in &game_over_lines {
        view.render_row(
            frame,
            row,
            vec![Span::styled(*line, Style::default().fg(colors.red()))],
        );
        row += 1;
    }

    row += 1;

    // Final score
    view.render_row(
        frame,
        row,
        vec![
            Span::styled("         FINAL SCORE: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}", state.score),
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            ),
        ],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![
            Span::styled("         DISTANCE:    ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{:.0} m", state.distance),
                Style::default().fg(colors.cyan()),
            ),
        ],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![
            Span::styled("         TOP SPEED:   ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{:.0} KPH", state.speed),
                Style::default().fg(colors.green()),
            ),
        ],
    );

    view.render_help(frame, vec![("Enter", "retry"), ("Esc", "quit")]);
}
