//! SLOTS modal rendering
//!
//! Renders the classic 3-reel slot machine game.

use super::super::slots::{SlotsState, SlotsView, Symbol};
use qdos_plugin_api::ui::FullScreenView;
use qdos_plugin_api::ThemeColors;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    Frame,
};

// =============================================================================
// MAIN DRAW FUNCTION
// =============================================================================

pub fn draw(frame: &mut Frame, area: Rect, state: &SlotsState, colors: &ThemeColors) {
    match state.view {
        SlotsView::Menu => draw_menu(frame, area, state, colors),
        SlotsView::Betting => draw_betting(frame, area, state, colors),
        SlotsView::Spinning => draw_spinning(frame, area, state, colors),
        SlotsView::Result => draw_result(frame, area, state, colors),
    }
}

// =============================================================================
// MENU SCREEN
// =============================================================================

fn draw_menu(frame: &mut Frame, area: Rect, state: &SlotsState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " SLOTS ", colors);
    view.render_frame(frame);

    let title_style = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(colors.fg());
    let highlight = Style::default()
        .fg(colors.green())
        .add_modifier(Modifier::BOLD);

    // Title
    view.render_row(
        frame,
        2,
        vec![Span::styled(
            "  ╔═════════════════════════════════════════╗",
            title_style,
        )],
    );
    view.render_row(
        frame,
        3,
        vec![Span::styled(
            "  ║    * * *  LUCKY SLOTS  * * *           ║",
            title_style,
        )],
    );
    view.render_row(
        frame,
        4,
        vec![Span::styled(
            "  ╚═════════════════════════════════════════╝",
            title_style,
        )],
    );

    // Machine display
    view.render_row(
        frame,
        6,
        vec![Span::styled("       ╔═══════╦═══════╦═══════╗", text_style)],
    );
    view.render_row(
        frame,
        7,
        vec![Span::styled(
            "       ║  7    ║  7    ║  7    ║",
            Style::default().fg(colors.red()),
        )],
    );
    view.render_row(
        frame,
        8,
        vec![Span::styled("       ╚═══════╩═══════╩═══════╝", text_style)],
    );

    // Payouts
    view.render_row(frame, 10, vec![Span::styled("  PAYOUTS:", highlight)]);
    view.render_row(
        frame,
        11,
        vec![Span::styled("  ### Diamond = 500x (JACKPOT!)", text_style)],
    );
    view.render_row(
        frame,
        12,
        vec![Span::styled("  777 Lucky 7 = 100x", text_style)],
    );
    view.render_row(
        frame,
        13,
        vec![Span::styled("  ▬▬▬ BAR     = 50x", text_style)],
    );
    view.render_row(
        frame,
        14,
        vec![Span::styled("  🔔🔔🔔 Bell    = 25x", text_style)],
    );
    view.render_row(
        frame,
        15,
        vec![Span::styled(
            "  Any Cherry  = 1x  |  Two Cherries = 2x",
            text_style,
        )],
    );

    // Credits
    view.render_row(
        frame,
        17,
        vec![Span::styled(
            format!("  Credits: ${}", state.available_credits),
            Style::default()
                .fg(colors.cyan())
                .add_modifier(Modifier::BOLD),
        )],
    );

    view.render_help(frame, vec![("Enter", "play"), ("Esc", "quit")]);
}

// =============================================================================
// BETTING SCREEN
// =============================================================================

fn draw_betting(frame: &mut Frame, area: Rect, state: &SlotsState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " SLOTS ", colors);
    view.render_frame(frame);

    let title_style = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(colors.fg());

    // Machine display
    draw_reels(frame, &view, state, colors, false);

    // Bet info
    view.render_row(
        frame,
        11,
        vec![Span::styled(
            format!(
                "  Credits: ${}    Bet: ${}",
                state.available_credits, state.current_bet
            ),
            text_style,
        )],
    );

    view.render_row(
        frame,
        13,
        vec![Span::styled(
            "  Use ↑↓ to adjust bet, M for max bet",
            title_style,
        )],
    );

    if let Some(msg) = &state.message {
        view.render_row(
            frame,
            15,
            vec![Span::styled(
                format!("  {}", msg),
                Style::default().fg(colors.red()),
            )],
        );
    }

    view.render_help(
        frame,
        vec![
            ("Enter", "spin"),
            ("↑↓", "bet"),
            ("M", "max"),
            ("Esc", "back"),
        ],
    );
}

// =============================================================================
// SPINNING SCREEN
// =============================================================================

fn draw_spinning(frame: &mut Frame, area: Rect, state: &SlotsState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " SLOTS ", colors);
    view.render_frame(frame);

    let text_style = Style::default().fg(colors.fg());

    // Machine display with spinning animation
    draw_reels(frame, &view, state, colors, true);

    view.render_row(
        frame,
        11,
        vec![Span::styled(
            format!(
                "  Credits: ${}    Bet: ${}",
                state.available_credits, state.current_bet
            ),
            text_style,
        )],
    );

    view.render_row(
        frame,
        13,
        vec![Span::styled(
            "       * * *  SPINNING...  * * *",
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );

    view.render_help(frame, vec![("", "spinning...")]);
}

// =============================================================================
// RESULT SCREEN
// =============================================================================

fn draw_result(frame: &mut Frame, area: Rect, state: &SlotsState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " SLOTS ", colors);
    view.render_frame(frame);

    let text_style = Style::default().fg(colors.fg());

    // Machine display
    draw_reels(frame, &view, state, colors, false);

    view.render_row(
        frame,
        11,
        vec![Span::styled(
            format!(
                "  Credits: ${}    Bet: ${}",
                state.available_credits, state.current_bet
            ),
            text_style,
        )],
    );

    // Win message
    if state.last_win > 0 {
        let win_style = if state.last_win >= state.current_bet * 50 {
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(colors.green())
                .add_modifier(Modifier::BOLD)
        };

        if let Some(msg) = &state.message {
            view.render_row(
                frame,
                13,
                vec![Span::styled(format!("  {}", msg), win_style)],
            );
        }
        view.render_row(
            frame,
            14,
            vec![Span::styled(
                format!("  WIN: ${}!", state.last_win),
                win_style,
            )],
        );
    } else if let Some(msg) = &state.message {
        view.render_row(
            frame,
            13,
            vec![Span::styled(
                format!("  {}", msg),
                Style::default().fg(colors.grey()),
            )],
        );
    }

    // Stats
    view.render_row(
        frame,
        16,
        vec![Span::styled(
            format!(
                "  Spins: {}  Total Won: ${}  Jackpots: {}",
                state.spin_count, state.total_won, state.jackpots_hit
            ),
            text_style,
        )],
    );

    if state.available_credits >= 5 {
        view.render_help(frame, vec![("Enter", "spin again"), ("Esc", "quit")]);
    } else {
        view.render_help(frame, vec![("Esc", "out of credits")]);
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// All symbols for animation cycling
const SPIN_SYMBOLS: [&str; 8] = ["CHR", "LMN", "ORG", "PLM", "BEL", "BAR", " 7 ", "DIA"];

fn draw_reels(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &SlotsState,
    colors: &ThemeColors,
    spinning: bool,
) {
    let border_style = Style::default().fg(colors.fg());

    // Draw machine frame
    view.render_row(
        frame,
        4,
        vec![Span::styled(
            "  ╔═══════════════════════════════════════╗",
            border_style,
        )],
    );
    view.render_row(
        frame,
        5,
        vec![Span::styled(
            "  ║      ╔═══════╦═══════╦═══════╗        ║",
            border_style,
        )],
    );

    // Reel symbols
    let mut reel_spans = vec![Span::styled("  ║      ║ ", border_style)];

    for i in 0..3 {
        let symbol = state.reels[i];
        let is_spinning = spinning && state.spinning_reels[i] > 0;

        let (text, style) = if is_spinning {
            // Animate with cycling symbols - each reel spins at different speed
            let spin_offset = (state.tick_count as usize + i * 3) % SPIN_SYMBOLS.len();
            let spin_text = SPIN_SYMBOLS[spin_offset];
            // Alternate colors for spinning effect
            let spin_color = if (state.tick_count / 2).is_multiple_of(2) {
                colors.grey()
            } else {
                colors.fg()
            };
            (spin_text, Style::default().fg(spin_color))
        } else {
            let sym_style = match symbol {
                Symbol::Seven => Style::default()
                    .fg(colors.red())
                    .add_modifier(Modifier::BOLD),
                Symbol::Diamond => Style::default()
                    .fg(colors.cyan())
                    .add_modifier(Modifier::BOLD),
                Symbol::Bar => Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
                Symbol::Bell => Style::default().fg(colors.yellow()),
                Symbol::Cherry => Style::default().fg(colors.red()),
                _ => Style::default().fg(colors.green()),
            };
            (symbol.ascii(), sym_style)
        };

        reel_spans.push(Span::styled(format!(" {} ", text), style));
        if i < 2 {
            reel_spans.push(Span::styled(" ║ ", border_style));
        }
    }
    reel_spans.push(Span::styled(" ║        ║", border_style));
    view.render_row(frame, 6, reel_spans);

    view.render_row(
        frame,
        7,
        vec![Span::styled(
            "  ║      ╚═══════╩═══════╩═══════╝        ║",
            border_style,
        )],
    );
    view.render_row(
        frame,
        8,
        vec![Span::styled(
            "  ╚═══════════════════════════════════════╝",
            border_style,
        )],
    );
}
