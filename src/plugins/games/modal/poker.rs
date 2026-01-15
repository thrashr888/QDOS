//! POKER modal rendering
//!
//! Renders the Jacks or Better video poker game.

use crate::app::ThemeColors;
use crate::plugins::games::poker::{HandRank, PokerState, PokerView};
use crate::ui::components::FullScreenView;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    Frame,
};

// =============================================================================
// MAIN DRAW FUNCTION
// =============================================================================

pub fn draw(frame: &mut Frame, area: Rect, state: &PokerState, colors: &ThemeColors) {
    match state.view {
        PokerView::Menu => draw_menu(frame, area, state, colors),
        PokerView::Betting => draw_betting(frame, area, state, colors),
        PokerView::HoldSelect => draw_hold_select(frame, area, state, colors),
        PokerView::FirstDeal | PokerView::Draw => draw_dealing(frame, area, state, colors),
        PokerView::Result => draw_result(frame, area, state, colors),
    }
}

// =============================================================================
// MENU SCREEN
// =============================================================================

fn draw_menu(frame: &mut Frame, area: Rect, state: &PokerState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " VIDEO POKER ", colors);
    view.render_frame(frame);

    let title_style = Style::default()
        .fg(colors.cyan())
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
            "   ╔════════════════════════════════════════╗",
            title_style,
        )],
    );
    view.render_row(
        frame,
        3,
        vec![Span::styled(
            "   ║   ♠ ♥ JACKS OR BETTER ♦ ♣              ║",
            title_style,
        )],
    );
    view.render_row(
        frame,
        4,
        vec![Span::styled(
            "   ╚════════════════════════════════════════╝",
            title_style,
        )],
    );

    // Sample hand display
    view.render_row(
        frame,
        6,
        vec![Span::styled(
            "     ┌───┐ ┌───┐ ┌───┐ ┌───┐ ┌───┐",
            text_style,
        )],
    );
    view.render_row(
        frame,
        7,
        vec![
            Span::styled("     │", text_style),
            Span::styled("A♠ ", Style::default().fg(colors.fg())),
            Span::styled("│ │", text_style),
            Span::styled("K♠ ", Style::default().fg(colors.fg())),
            Span::styled("│ │", text_style),
            Span::styled("Q♠ ", Style::default().fg(colors.fg())),
            Span::styled("│ │", text_style),
            Span::styled("J♠ ", Style::default().fg(colors.fg())),
            Span::styled("│ │", text_style),
            Span::styled("10♠", Style::default().fg(colors.fg())),
            Span::styled("│", text_style),
        ],
    );
    view.render_row(
        frame,
        8,
        vec![Span::styled(
            "     └───┘ └───┘ └───┘ └───┘ └───┘",
            text_style,
        )],
    );

    // Payouts
    view.render_row(frame, 10, vec![Span::styled("  PAYOUTS:", highlight)]);
    view.render_row(
        frame,
        11,
        vec![Span::styled("  Royal Flush     250x", text_style)],
    );
    view.render_row(
        frame,
        12,
        vec![Span::styled("  Straight Flush   50x", text_style)],
    );
    view.render_row(
        frame,
        13,
        vec![Span::styled("  Four of a Kind   25x", text_style)],
    );
    view.render_row(
        frame,
        14,
        vec![Span::styled("  Full House        9x", text_style)],
    );
    view.render_row(
        frame,
        15,
        vec![Span::styled(
            "  Flush/Straight  6x/4x  |  Three/Two Pair  3x/2x",
            text_style,
        )],
    );
    view.render_row(
        frame,
        16,
        vec![Span::styled("  Jacks or Better   1x", text_style)],
    );

    // Credits
    view.render_row(
        frame,
        18,
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

fn draw_betting(frame: &mut Frame, area: Rect, state: &PokerState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " VIDEO POKER ", colors);
    view.render_frame(frame);

    let title_style = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(colors.fg());

    view.render_row(
        frame,
        4,
        vec![Span::styled("  Place your bet", title_style)],
    );

    // Empty card positions
    view.render_row(
        frame,
        7,
        vec![Span::styled(
            "     ┌───┐ ┌───┐ ┌───┐ ┌───┐ ┌───┐",
            text_style,
        )],
    );
    view.render_row(
        frame,
        8,
        vec![Span::styled(
            "     │   │ │   │ │   │ │   │ │   │",
            text_style,
        )],
    );
    view.render_row(
        frame,
        9,
        vec![Span::styled(
            "     └───┘ └───┘ └───┘ └───┘ └───┘",
            text_style,
        )],
    );

    // Bet info
    view.render_row(
        frame,
        12,
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
        14,
        vec![Span::styled(
            "  Use ↑↓ to adjust bet, M for max bet",
            title_style,
        )],
    );

    view.render_help(
        frame,
        vec![
            ("Enter", "deal"),
            ("↑↓", "bet"),
            ("M", "max"),
            ("Esc", "back"),
        ],
    );
}

// =============================================================================
// HOLD SELECT SCREEN
// =============================================================================

fn draw_hold_select(frame: &mut Frame, area: Rect, state: &PokerState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " VIDEO POKER ", colors);
    view.render_frame(frame);

    let title_style = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(colors.fg());

    view.render_row(
        frame,
        3,
        vec![Span::styled(
            "  Select cards to HOLD, then press ENTER to draw",
            title_style,
        )],
    );

    // Draw cards with hold indicators
    draw_hand(frame, &view, state, colors, true);

    // Credits info
    view.render_row(
        frame,
        12,
        vec![Span::styled(
            format!(
                "  Credits: ${}    Bet: ${}",
                state.available_credits, state.current_bet
            ),
            text_style,
        )],
    );

    view.render_help(
        frame,
        vec![("←→", "select"), ("Space/1-5", "hold"), ("Enter", "draw")],
    );
}

// =============================================================================
// DEALING ANIMATION
// =============================================================================

fn draw_dealing(frame: &mut Frame, area: Rect, state: &PokerState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " VIDEO POKER ", colors);
    view.render_frame(frame);

    let title_style = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);

    view.render_row(frame, 10, vec![Span::styled("  Dealing...", title_style)]);

    draw_hand(frame, &view, state, colors, false);
}

// =============================================================================
// RESULT SCREEN
// =============================================================================

fn draw_result(frame: &mut Frame, area: Rect, state: &PokerState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " VIDEO POKER ", colors);
    view.render_frame(frame);

    let text_style = Style::default().fg(colors.fg());

    // Draw final hand
    draw_hand(frame, &view, state, colors, false);

    // Hand rank and win
    if let Some(rank) = state.hand_rank {
        let win_style = if rank >= HandRank::FullHouse {
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD)
        } else if state.last_win > 0 {
            Style::default()
                .fg(colors.green())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.grey())
        };

        view.render_row(
            frame,
            11,
            vec![Span::styled(format!("  {}", rank.name()), win_style)],
        );

        if state.last_win > 0 {
            view.render_row(
                frame,
                12,
                vec![Span::styled(
                    format!("  WIN: ${}!", state.last_win),
                    win_style,
                )],
            );
        }
    }

    // Credits info
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
                "  Hands: {}  Total Won: ${}",
                state.hands_played, state.total_won
            ),
            text_style,
        )],
    );

    if state.available_credits >= 5 {
        view.render_help(frame, vec![("Enter", "deal again"), ("Esc", "quit")]);
    } else {
        view.render_help(frame, vec![("Esc", "out of credits")]);
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

fn draw_hand(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &PokerState,
    colors: &ThemeColors,
    show_selection: bool,
) {
    let border_style = Style::default().fg(colors.fg());

    // Card tops
    let mut top_spans = vec![Span::styled("     ", border_style)];
    for i in 0..5 {
        let style = if show_selection && i == state.selected_card {
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD)
        } else {
            border_style
        };
        top_spans.push(Span::styled("┌───┐ ", style));
    }
    view.render_row(frame, 5, top_spans);

    // Card contents
    let mut card_spans = vec![Span::styled("     ", border_style)];
    for i in 0..5 {
        let card = state.hand[i];
        let card_style = if card.suit.is_red() {
            Style::default().fg(colors.red())
        } else {
            Style::default().fg(colors.fg())
        };

        let border_color = if show_selection && i == state.selected_card {
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD)
        } else {
            border_style
        };

        let rank_str = card.rank.symbol();
        let suit_char = card.suit.symbol();
        let content = format!("{:<2}{}", rank_str, suit_char);

        card_spans.push(Span::styled("│", border_color));
        card_spans.push(Span::styled(content, card_style));
        card_spans.push(Span::styled("│ ", border_color));
    }
    view.render_row(frame, 6, card_spans);

    // Card bottoms
    let mut bottom_spans = vec![Span::styled("     ", border_style)];
    for i in 0..5 {
        let style = if show_selection && i == state.selected_card {
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD)
        } else {
            border_style
        };
        bottom_spans.push(Span::styled("└───┘ ", style));
    }
    view.render_row(frame, 7, bottom_spans);

    // Hold indicators
    if show_selection {
        let mut hold_spans = vec![Span::styled("     ", border_style)];
        for i in 0..5 {
            if state.held[i] {
                hold_spans.push(Span::styled(
                    " HOLD ",
                    Style::default()
                        .fg(colors.green())
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                hold_spans.push(Span::styled("      ", border_style));
            }
        }
        view.render_row(frame, 8, hold_spans);

        // Card numbers
        let mut num_spans = vec![Span::styled("     ", Style::default().fg(colors.grey()))];
        for i in 1..=5 {
            num_spans.push(Span::styled(
                format!("  {}   ", i),
                Style::default().fg(colors.grey()),
            ));
        }
        view.render_row(frame, 9, num_spans);
    }
}
