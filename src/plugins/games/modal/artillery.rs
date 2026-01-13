use crate::app::ThemeColors;
use crate::plugins::games::artillery::{ArtilleryState, GamePhase};
use crate::ui::components::FullScreenView;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    Frame,
};

pub fn draw(frame: &mut Frame, area: Rect, state: &ArtilleryState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " ARTILLERY ", colors);
    view.render_frame(frame);

    let mut row = 0;

    // Status bar
    let player_health_bars = state.player_tank.health / 10;
    let enemy_health_bars = state.enemy_tank.health / 10;

    let wind_symbol = if state.wind > 0 {
        "→".repeat((state.wind.abs() / 5).min(3) as usize)
    } else if state.wind < 0 {
        "←".repeat((state.wind.abs() / 5).min(3) as usize)
    } else {
        " ".to_string()
    };

    view.render_row(
        frame,
        row,
        vec![
            Span::styled(
                "P1: ",
                Style::default()
                    .fg(colors.blue())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "█".repeat(player_health_bars as usize),
                Style::default().fg(colors.green()),
            ),
            Span::styled(
                "░".repeat((10 - player_health_bars) as usize),
                Style::default().fg(colors.grey()),
            ),
            Span::styled("  WIND: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{} {} mph", wind_symbol, state.wind.abs()),
                Style::default().fg(colors.yellow()),
            ),
            Span::styled(
                "  P2: ",
                Style::default()
                    .fg(colors.red())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "█".repeat(enemy_health_bars as usize),
                Style::default().fg(colors.green()),
            ),
            Span::styled(
                "░".repeat((10 - enemy_health_bars) as usize),
                Style::default().fg(colors.grey()),
            ),
        ],
    );
    row += 1;

    // Render battlefield
    for y in 0..state.terrain.len() {
        let mut line = Vec::new();
        line.push(Span::raw(" ")); // Left padding

        for x in 0..state.terrain[y].len() {
            // Check for tanks
            let player_tank_here = x >= state.player_tank.x.saturating_sub(1)
                && x <= state.player_tank.x + 1
                && y >= state.player_tank.y.saturating_sub(1)
                && y <= state.player_tank.y;

            let enemy_tank_here = x >= state.enemy_tank.x.saturating_sub(1)
                && x <= state.enemy_tank.x + 1
                && y >= state.enemy_tank.y.saturating_sub(1)
                && y <= state.enemy_tank.y;

            // Check for projectile
            let projectile_here = if let Some(ref proj) = state.projectile {
                proj.active && (proj.x as usize) == x && (proj.y as usize) == y
            } else {
                false
            };

            if player_tank_here && state.player_tank.is_alive() {
                // Draw player tank
                if y == state.player_tank.y.saturating_sub(1) {
                    if x == state.player_tank.x {
                        line.push(Span::styled(
                            "█",
                            Style::default()
                                .fg(colors.blue())
                                .add_modifier(Modifier::BOLD),
                        ));
                    } else {
                        line.push(Span::styled("▀", Style::default().fg(colors.blue())));
                    }
                } else {
                    line.push(Span::styled("█", Style::default().fg(colors.blue())));
                }
            } else if enemy_tank_here && state.enemy_tank.is_alive() {
                // Draw enemy tank
                if y == state.enemy_tank.y.saturating_sub(1) {
                    if x == state.enemy_tank.x {
                        line.push(Span::styled(
                            "█",
                            Style::default()
                                .fg(colors.red())
                                .add_modifier(Modifier::BOLD),
                        ));
                    } else {
                        line.push(Span::styled("▀", Style::default().fg(colors.red())));
                    }
                } else {
                    line.push(Span::styled("█", Style::default().fg(colors.red())));
                }
            } else if projectile_here {
                line.push(Span::styled(
                    "*",
                    Style::default()
                        .fg(colors.yellow())
                        .add_modifier(Modifier::BOLD),
                ));
            } else if state.terrain[y][x] {
                line.push(Span::styled("▓", Style::default().fg(colors.green())));
            } else {
                line.push(Span::raw(" "));
            }
        }

        view.render_row(frame, row, line);
        row += 1;
    }

    // Controls / Message area
    row += 1;
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            &state.message,
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );

    if state.phase == GamePhase::Aiming && state.current_player {
        row += 1;
        let power_bars = state.power / 10;
        view.render_row(
            frame,
            row,
            vec![
                Span::styled("Angle: ", Style::default().fg(colors.grey())),
                Span::styled(
                    format!("{}°", state.angle),
                    Style::default().fg(colors.yellow()),
                ),
                Span::styled("  Power: ", Style::default().fg(colors.grey())),
                Span::styled(
                    "█".repeat(power_bars as usize),
                    Style::default().fg(colors.cyan()),
                ),
                Span::styled(
                    "░".repeat((10 - power_bars) as usize),
                    Style::default().fg(colors.grey()),
                ),
                Span::styled(
                    format!(" {}%", state.power),
                    Style::default().fg(colors.cyan()),
                ),
            ],
        );
    }

    // Help text
    if state.phase == GamePhase::GameOver {
        view.render_help(frame, vec![("R", "restart"), ("Esc", "quit")]);
    } else if state.current_player && state.phase == GamePhase::Aiming {
        view.render_help(
            frame,
            vec![
                ("←→/AD", "angle"),
                ("↑↓/WS", "power"),
                ("Space", "fire"),
                ("P", "pause"),
                ("Esc", "quit"),
            ],
        );
    } else {
        view.render_help(frame, vec![("P", "pause"), ("Esc", "quit")]);
    }
}
