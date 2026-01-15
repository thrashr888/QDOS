//! ROULETTE modal rendering
//!
//! Renders the wheel and betting table.

use crate::app::ThemeColors;
use crate::ui::components::FullScreenView;
use ratatui::{
    prelude::*,
    style::{Modifier, Style},
    text::Span,
    Frame,
};

use super::super::roulette::{get_color_name, is_red, BetMode, RouletteState, RouletteView};

/// Main draw function for ROULETTE
pub fn draw(frame: &mut Frame, area: Rect, state: &RouletteState, colors: &ThemeColors) {
    let credits = state.available_credits;
    match state.view {
        RouletteView::Menu => draw_menu(frame, area, state, colors, credits),
        RouletteView::Betting => draw_betting(frame, area, state, colors, credits),
        RouletteView::Spinning => draw_spinning(frame, area, state, colors, credits),
        RouletteView::Result => draw_result(frame, area, state, colors, credits),
    }
}

fn draw_menu(
    frame: &mut Frame,
    area: Rect,
    state: &RouletteState,
    colors: &ThemeColors,
    credits: i64,
) {
    let view = FullScreenView::new(area, " Roulette ", colors);
    view.render_frame(frame);

    let yellow = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let white = Style::default().fg(colors.fg());
    let green = Style::default().fg(colors.green());
    let red = Style::default().fg(colors.red());

    // Title
    view.render_row(frame, 1, vec![Span::styled("      * ROULETTE *", yellow)]);

    // Wheel art
    view.render_row(frame, 3, vec![Span::styled("       ╭───────╮", white)]);
    view.render_row(
        frame,
        4,
        vec![
            Span::styled("      ╱", white),
            Span::styled(" 0 ", green),
            Span::styled("│", white),
            Span::styled("32", red),
            Span::styled("│", white),
            Span::styled("15", Style::default().fg(colors.fg())),
            Span::styled("╲", white),
        ],
    );
    view.render_row(
        frame,
        5,
        vec![
            Span::styled("     │", white),
            Span::styled("19", red),
            Span::styled("│", white),
            Span::styled(" 4", Style::default().fg(colors.fg())),
            Span::styled("│", white),
            Span::styled("21", red),
            Span::styled("│", white),
        ],
    );
    view.render_row(frame, 6, vec![Span::styled("      ╲─────────╱", white)]);

    // Instructions
    view.render_row(
        frame,
        8,
        vec![Span::styled("  Place bets and spin!", white)],
    );
    view.render_row(
        frame,
        9,
        vec![Span::styled("  Straight up pays 35:1", green)],
    );

    // Stats
    if state.spins_played > 0 {
        view.render_row(
            frame,
            11,
            vec![Span::styled(
                format!("  Spins: {} | Won: {}", state.spins_played, state.spins_won),
                white,
            )],
        );
    }

    // Last numbers
    if !state.last_numbers.is_empty() {
        let mut spans: Vec<Span> = vec![Span::styled("  Last: ", white)];
        for (i, &num) in state.last_numbers.iter().take(8).enumerate() {
            let style = if num == 0 {
                green
            } else if is_red(num) {
                red
            } else {
                white
            };
            spans.push(Span::styled(format!("{:2}", num), style));
            if i < 7 && i < state.last_numbers.len() - 1 {
                spans.push(Span::styled(" ", white));
            }
        }
        view.render_row(frame, 13, spans);
    }

    // Credits
    view.render_row(
        frame,
        15,
        vec![Span::styled(format!("  Credits: {}", credits), yellow)],
    );

    // Message
    if let Some(msg) = &state.message {
        view.render_row(frame, 17, vec![Span::styled(format!("  {}", msg), red)]);
    }

    view.render_help(frame, vec![("Enter", "play"), ("Esc", "quit")]);
}

fn draw_betting(
    frame: &mut Frame,
    area: Rect,
    state: &RouletteState,
    colors: &ThemeColors,
    credits: i64,
) {
    let view = FullScreenView::new(area, " Place Your Bets ", colors);
    view.render_frame(frame);

    let yellow = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let white = Style::default().fg(colors.fg());
    let green = Style::default().fg(colors.green());
    let red = Style::default().fg(colors.red());
    let grey = Style::default().fg(colors.grey());

    // Mode indicator
    let mode_str = match state.bet_mode {
        BetMode::Outside => "OUTSIDE BETS",
        BetMode::Number => "STRAIGHT UP",
    };
    view.render_row(
        frame,
        1,
        vec![Span::styled(
            format!("  {} (Tab to switch)", mode_str),
            yellow,
        )],
    );

    match state.bet_mode {
        BetMode::Outside => {
            // Outside bet options
            let options = RouletteState::outside_bet_options();
            for (i, bet_type) in options.iter().enumerate() {
                let is_selected = i == state.selected_bet_index;
                let prefix = if is_selected { "▸ " } else { "  " };
                let style = if is_selected { yellow } else { white };

                let payout = format!("({}:1)", bet_type.payout_ratio());
                view.render_row(
                    frame,
                    3 + i as u16,
                    vec![Span::styled(
                        format!("  {}{:<20} {}", prefix, bet_type.name(), payout),
                        style,
                    )],
                );
            }
        }
        BetMode::Number => {
            // Number grid (simplified)
            view.render_row(
                frame,
                3,
                vec![Span::styled("  Select a number (0-36):", white)],
            );

            // Show selected number prominently
            let num = state.selected_number;
            let num_color = if num == 0 {
                green
            } else if is_red(num) {
                red
            } else {
                white
            };

            view.render_row(
                frame,
                5,
                vec![Span::styled("       ╔═══════════╗".to_string(), num_color)],
            );
            view.render_row(
                frame,
                6,
                vec![Span::styled(
                    format!("       ║    {:>2}     ║", num),
                    num_color,
                )],
            );
            view.render_row(
                frame,
                7,
                vec![Span::styled(
                    format!("       ║   ({})   ║", get_color_name(num)),
                    num_color,
                )],
            );
            view.render_row(
                frame,
                8,
                vec![Span::styled("       ╚═══════════╝".to_string(), num_color)],
            );

            view.render_row(frame, 10, vec![Span::styled("  Pays 35:1", green)]);
        }
    }

    // Current bet amount
    view.render_row(
        frame,
        16,
        vec![Span::styled(
            format!("  Bet amount: {} (←/→ to adjust)", state.current_bet_amount),
            white,
        )],
    );

    // Current bets
    if !state.bets.is_empty() {
        view.render_row(frame, 17, vec![Span::styled("  Current bets:", grey)]);
        let bet_str: String = state
            .bets
            .iter()
            .map(|b| format!("{}:{}", b.bet_type.name(), b.amount))
            .collect::<Vec<_>>()
            .join(", ");
        let truncated = if bet_str.len() > 60 {
            format!("{}...", &bet_str[..57])
        } else {
            bet_str
        };
        view.render_row(
            frame,
            18,
            vec![Span::styled(format!("  {}", truncated), grey)],
        );
    }

    // Credits and total bet
    view.render_row(
        frame,
        19,
        vec![Span::styled(
            format!("  Credits: {} | Total bet: {}", credits, state.total_bet()),
            yellow,
        )],
    );

    // Message
    if let Some(msg) = &state.message {
        view.render_row(frame, 20, vec![Span::styled(format!("  {}", msg), red)]);
    }

    view.render_help(
        frame,
        vec![
            ("Enter", "bet"),
            ("Space", "spin"),
            ("C", "clear"),
            ("Esc", "back"),
        ],
    );
}

fn draw_spinning(
    frame: &mut Frame,
    area: Rect,
    state: &RouletteState,
    colors: &ThemeColors,
    _credits: i64,
) {
    let view = FullScreenView::new(area, " Spinning... ", colors);
    view.render_frame(frame);

    let yellow = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let white = Style::default().fg(colors.fg());
    let green = Style::default().fg(colors.green());
    let red = Style::default().fg(colors.red());

    // Animated wheel display
    let wheel_nums = state.wheel_display();

    view.render_row(frame, 4, vec![Span::styled("        ▼", yellow)]);

    // Draw wheel section
    let mut wheel_spans: Vec<Span> = vec![Span::styled("   ", white)];
    for (i, &num) in wheel_nums.iter().enumerate() {
        let is_center = i == 2;
        let num_style = if num == 0 {
            green
        } else if is_red(num) {
            red
        } else {
            white
        };

        let bg_style = if is_center {
            num_style.add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            num_style
        };

        wheel_spans.push(Span::styled(format!(" {:>2} ", num), bg_style));
    }
    view.render_row(frame, 5, wheel_spans);

    // Wheel border
    view.render_row(
        frame,
        6,
        vec![Span::styled("   ═══════════════════", white)],
    );

    // Status
    let spin_pct = 100 - (state.spin_timer * 100 / 90);
    view.render_row(
        frame,
        9,
        vec![Span::styled(
            format!("      Spinning... {}%", spin_pct.min(99)),
            yellow,
        )],
    );

    // Animation dots
    let dots = ".".repeat((state.tick_count / 5 % 4) as usize);
    view.render_row(
        frame,
        11,
        vec![Span::styled(format!("         {}", dots), white)],
    );

    view.render_help(frame, vec![("", "waiting for result...")]);
}

fn draw_result(
    frame: &mut Frame,
    area: Rect,
    state: &RouletteState,
    colors: &ThemeColors,
    credits: i64,
) {
    let view = FullScreenView::new(area, " Result ", colors);
    view.render_frame(frame);

    let yellow = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let white = Style::default().fg(colors.fg());
    let green = Style::default().fg(colors.green());
    let red = Style::default().fg(colors.red());

    // Winning number
    if let Some(num) = state.winning_number {
        let num_style = if num == 0 {
            green
        } else if is_red(num) {
            red
        } else {
            white
        };

        view.render_row(
            frame,
            2,
            vec![Span::styled("  The winning number is:", white)],
        );
        view.render_row(
            frame,
            4,
            vec![Span::styled("       ╔═══════════╗".to_string(), num_style)],
        );
        view.render_row(
            frame,
            5,
            vec![Span::styled(
                format!("       ║    {:>2}     ║", num),
                num_style,
            )],
        );
        view.render_row(
            frame,
            6,
            vec![Span::styled(
                format!("       ║   ({})   ║", get_color_name(num)),
                num_style,
            )],
        );
        view.render_row(
            frame,
            7,
            vec![Span::styled("       ╚═══════════╝".to_string(), num_style)],
        );

        // Calculate winnings
        let total_bet: i64 = state.bets.iter().map(|b| b.amount).sum();
        let total_win = state.calculate_potential_win(num);
        let net = total_win - total_bet;

        let result_style = if net > 0 {
            green
        } else if net < 0 {
            red
        } else {
            yellow
        };
        let result_msg = if net > 0 {
            format!("  YOU WIN {}!", net)
        } else if net < 0 {
            format!("  You lose {}", -net)
        } else {
            "  No change".to_string()
        };

        view.render_row(frame, 9, vec![Span::styled(result_msg, result_style)]);

        // Show winning bets
        let winning_bets: Vec<_> = state.bets.iter().filter(|b| b.bet_type.wins(num)).collect();
        if !winning_bets.is_empty() {
            view.render_row(frame, 11, vec![Span::styled("  Winning bets:", green)]);
            for (i, bet) in winning_bets.iter().take(3).enumerate() {
                let payout = bet.amount * (bet.bet_type.payout_ratio() as i64 + 1);
                view.render_row(
                    frame,
                    12 + i as u16,
                    vec![Span::styled(
                        format!("    {} → {}", bet.bet_type.name(), payout),
                        green,
                    )],
                );
            }
        }
    }

    // Credits
    view.render_row(
        frame,
        17,
        vec![Span::styled(format!("  Credits: {}", credits), yellow)],
    );

    // Message
    if let Some(msg) = &state.message {
        view.render_row(frame, 19, vec![Span::styled(format!("  {}", msg), red)]);
    }

    view.render_help(frame, vec![("Enter", "continue"), ("Esc", "quit")]);
}
