//! CRAPS modal rendering
//!
//! Renders the casino craps dice game.

use super::super::craps::{BetType, CrapsState, CrapsView};
use qdos_plugin_api::ui::FullScreenView;
use qdos_plugin_api::ThemeColors;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    Frame,
};

// =============================================================================
// DICE ASCII ART
// =============================================================================

const DICE_ART: [[&str; 3]; 6] = [
    ["┌───┐", "│ o │", "└───┘"], // 1
    ["┌───┐", "│o o│", "└───┘"], // 2
    ["┌───┐", "│ooo│", "└───┘"], // 3
    ["┌───┐", "│oo │", "└oo─┘"], // 4 (simplified)
    ["┌───┐", "│ooo│", "└oo─┘"], // 5 (simplified)
    ["┌───┐", "│ooo│", "└ooo┘"], // 6 (simplified)
];

// =============================================================================
// MAIN DRAW FUNCTION
// =============================================================================

pub fn draw(frame: &mut Frame, area: Rect, state: &CrapsState, colors: &ThemeColors) {
    match state.view {
        CrapsView::Menu => draw_menu(frame, area, state, colors),
        CrapsView::Betting => draw_betting(frame, area, state, colors),
        CrapsView::Rolling => draw_rolling(frame, area, state, colors),
        CrapsView::PointPhase => draw_point_phase(frame, area, state, colors),
        CrapsView::Result => draw_result(frame, area, state, colors),
    }
}

// =============================================================================
// MENU SCREEN
// =============================================================================

fn draw_menu(frame: &mut Frame, area: Rect, state: &CrapsState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " CRAPS ", colors);
    view.render_frame(frame);

    let title_style = Style::default()
        .fg(colors.green())
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(colors.fg());
    let highlight = Style::default()
        .fg(colors.yellow())
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
            "  ║    [1][2][3] CRAPS [4][5][6]           ║",
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

    // Rules
    view.render_row(frame, 6, vec![Span::styled("  BETS:", highlight)]);
    view.render_row(
        frame,
        7,
        vec![Span::styled(
            "  Pass Line - 7/11 win, 2/3/12 lose (come-out)",
            text_style,
        )],
    );
    view.render_row(
        frame,
        8,
        vec![Span::styled(
            "  Don't Pass - Opposite (12 = push)",
            text_style,
        )],
    );
    view.render_row(
        frame,
        9,
        vec![Span::styled(
            "  Field - One roll: 2,3,4,9,10,11,12 win",
            text_style,
        )],
    );
    view.render_row(
        frame,
        10,
        vec![Span::styled(
            "  Any 7 - 7 pays 4:1  |  Any Craps - 2,3,12 pays 7:1",
            text_style,
        )],
    );

    // How to play
    view.render_row(frame, 12, vec![Span::styled("  HOW TO PLAY:", highlight)]);
    view.render_row(
        frame,
        13,
        vec![Span::styled(
            "  Roll dice. Pass/Don't Pass establish a POINT (4-10)",
            text_style,
        )],
    );
    view.render_row(
        frame,
        14,
        vec![Span::styled(
            "  Then roll again: hit POINT to win, 7 to lose (Pass)",
            text_style,
        )],
    );

    // Credits
    view.render_row(
        frame,
        16,
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

fn draw_betting(frame: &mut Frame, area: Rect, state: &CrapsState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " CRAPS ", colors);
    view.render_frame(frame);

    let title_style = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(colors.fg());

    // Table display
    view.render_row(
        frame,
        2,
        vec![Span::styled(
            "  ╔══════════════════════════════════════════╗",
            text_style,
        )],
    );
    view.render_row(
        frame,
        3,
        vec![Span::styled(
            "  ║        C  R  A  P  S    T  A  B  L  E     ║",
            title_style,
        )],
    );
    view.render_row(
        frame,
        4,
        vec![Span::styled(
            "  ╠══════════════════════════════════════════╣",
            text_style,
        )],
    );
    view.render_row(
        frame,
        5,
        vec![Span::styled(
            "  ║ PASS LINE │ DON'T PASS │ FIELD │ PROP   ║",
            text_style,
        )],
    );
    view.render_row(
        frame,
        6,
        vec![Span::styled(
            "  ╚══════════════════════════════════════════╝",
            text_style,
        )],
    );

    // Bet selection
    view.render_row(
        frame,
        8,
        vec![Span::styled("  Select bet (←→):", title_style)],
    );

    view.render_row(
        frame,
        10,
        vec![Span::styled(
            format!("  ► {} ◄", state.bet_type.name()),
            Style::default()
                .fg(colors.cyan())
                .add_modifier(Modifier::BOLD),
        )],
    );
    view.render_row(
        frame,
        11,
        vec![Span::styled(
            format!("    {}", state.bet_type.description()),
            Style::default().fg(colors.grey()),
        )],
    );

    // Bet amount
    view.render_row(
        frame,
        13,
        vec![Span::styled(
            format!(
                "  Credits: ${}    Bet: ${}",
                state.available_credits, state.current_bet
            ),
            text_style,
        )],
    );

    // Stats
    view.render_row(
        frame,
        15,
        vec![Span::styled(
            format!(
                "  Rolls: {}  7s: {}  Points made: {}",
                state.rolls, state.sevens, state.points_made
            ),
            Style::default().fg(colors.grey()),
        )],
    );

    view.render_help(
        frame,
        vec![
            ("Enter", "roll"),
            ("←→", "bet type"),
            ("↑↓", "amount"),
            ("Esc", "back"),
        ],
    );
}

// =============================================================================
// ROLLING SCREEN
// =============================================================================

fn draw_rolling(frame: &mut Frame, area: Rect, state: &CrapsState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " CRAPS ", colors);
    view.render_frame(frame);

    // Draw dice
    draw_dice(frame, &view, state, colors, true);

    view.render_row(
        frame,
        10,
        vec![Span::styled(
            "       * * *  ROLLING...  * * *",
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );

    if let Some(point) = state.point {
        view.render_row(
            frame,
            12,
            vec![Span::styled(
                format!("  Point: {}", point),
                Style::default().fg(colors.cyan()),
            )],
        );
    }
}

// =============================================================================
// POINT PHASE SCREEN
// =============================================================================

fn draw_point_phase(frame: &mut Frame, area: Rect, state: &CrapsState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " CRAPS ", colors);
    view.render_frame(frame);

    let text_style = Style::default().fg(colors.fg());

    // Draw dice
    draw_dice(frame, &view, state, colors, false);

    if let Some(point) = state.point {
        view.render_row(
            frame,
            10,
            vec![Span::styled(
                format!("  POINT: {} - Roll again!", point),
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            )],
        );

        let need_msg = if state.bet_type == BetType::Pass {
            format!("  Need {} to win, 7 to lose", point)
        } else {
            format!("  Need 7 to win, {} to lose", point)
        };
        view.render_row(
            frame,
            11,
            vec![Span::styled(need_msg, Style::default().fg(colors.grey()))],
        );
    }

    if let Some(msg) = &state.message {
        view.render_row(
            frame,
            13,
            vec![Span::styled(format!("  {}", msg), text_style)],
        );
    }

    view.render_row(
        frame,
        15,
        vec![Span::styled(
            format!(
                "  Credits: ${}    Bet: ${}",
                state.available_credits, state.current_bet
            ),
            text_style,
        )],
    );

    view.render_help(frame, vec![("Enter", "roll"), ("Esc", "abandon point")]);
}

// =============================================================================
// RESULT SCREEN
// =============================================================================

fn draw_result(frame: &mut Frame, area: Rect, state: &CrapsState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " CRAPS ", colors);
    view.render_frame(frame);

    let text_style = Style::default().fg(colors.fg());

    // Draw dice
    draw_dice(frame, &view, state, colors, false);

    // Total
    view.render_row(
        frame,
        9,
        vec![Span::styled(
            format!("          Total: {}", state.dice_total()),
            Style::default()
                .fg(colors.cyan())
                .add_modifier(Modifier::BOLD),
        )],
    );

    // Result message
    if let Some(msg) = &state.message {
        let style = if state.last_win > 0 {
            Style::default()
                .fg(colors.green())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.red())
        };
        view.render_row(frame, 11, vec![Span::styled(format!("  {}", msg), style)]);
    }

    // Credits
    view.render_row(
        frame,
        14,
        vec![Span::styled(
            format!("  Credits: ${}", state.available_credits),
            text_style,
        )],
    );

    // Stats
    view.render_row(
        frame,
        16,
        vec![Span::styled(
            format!(
                "  Rolls: {}  Total Won: ${}  7s: {}",
                state.rolls, state.total_won, state.sevens
            ),
            Style::default().fg(colors.grey()),
        )],
    );

    if state.available_credits >= 5 {
        view.render_help(frame, vec![("Enter", "roll again"), ("Esc", "quit")]);
    } else {
        view.render_help(frame, vec![("Esc", "out of credits")]);
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

fn draw_dice(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &CrapsState,
    colors: &ThemeColors,
    rolling: bool,
) {
    let style = if rolling {
        Style::default().fg(colors.grey())
    } else {
        Style::default().fg(colors.fg())
    };

    // Draw two dice side by side
    for (row_idx, row) in (4..7).enumerate() {
        let d1 = state.dice[0] as usize - 1;
        let d2 = state.dice[1] as usize - 1;

        view.render_row(
            frame,
            row as u16,
            vec![Span::styled(
                format!(
                    "          {}  {}",
                    DICE_ART[d1][row_idx], DICE_ART[d2][row_idx]
                ),
                style,
            )],
        );
    }
}
