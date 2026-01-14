//! BLACKJACK modal rendering
//!
//! Renders the card game with ASCII card art.

use crate::app::ThemeColors;
use crate::ui::components::FullScreenView;
use ratatui::{
    prelude::*,
    style::{Modifier, Style},
    text::Span,
    Frame,
};

use super::super::blackjack::{calculate_hand_value, BlackjackState, BlackjackView, Card};

/// Main draw function for BLACKJACK
pub fn draw(frame: &mut Frame, area: Rect, state: &BlackjackState, colors: &ThemeColors) {
    let credits = state.available_credits;
    match state.view {
        BlackjackView::Menu => draw_menu(frame, area, state, colors, credits),
        BlackjackView::Betting => draw_betting(frame, area, state, colors, credits),
        BlackjackView::PlayerTurn | BlackjackView::DealerTurn => {
            draw_game(frame, area, state, colors, credits)
        }
        BlackjackView::Result => draw_result(frame, area, state, colors, credits),
    }
}

fn draw_menu(
    frame: &mut Frame,
    area: Rect,
    state: &BlackjackState,
    colors: &ThemeColors,
    credits: i64,
) {
    let view = FullScreenView::new(area, " Blackjack ", colors);
    view.render_frame(frame);

    let yellow = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let white = Style::default().fg(colors.fg());
    let green = Style::default().fg(colors.green());

    // Title art
    view.render_row(
        frame,
        1,
        vec![Span::styled("     ♠ ♥ BLACKJACK ♦ ♣", yellow)],
    );

    // ASCII card art
    view.render_row(frame, 3, vec![Span::styled("      ┌─────┐ ┌─────┐", white)]);
    view.render_row(frame, 4, vec![Span::styled("      │A    │ │?    │", white)]);
    view.render_row(frame, 5, vec![Span::styled("      │  ♠  │ │  ?  │", white)]);
    view.render_row(frame, 6, vec![Span::styled("      │    A│ │    ?│", white)]);
    view.render_row(frame, 7, vec![Span::styled("      └─────┘ └─────┘", white)]);

    // Instructions
    view.render_row(
        frame,
        9,
        vec![Span::styled("  Beat the dealer to 21!", white)],
    );
    view.render_row(frame, 10, vec![Span::styled("  Blackjack pays 3:2", green)]);

    // Stats
    if state.hands_played > 0 {
        view.render_row(
            frame,
            12,
            vec![Span::styled(
                format!(
                    "  Hands: {} | Won: {} | Blackjacks: {}",
                    state.hands_played, state.hands_won, state.blackjacks
                ),
                white,
            )],
        );
    }

    // Credits
    view.render_row(
        frame,
        14,
        vec![Span::styled(format!("  Credits: {}", credits), yellow)],
    );

    // Message
    if let Some(msg) = &state.message {
        let red = Style::default().fg(colors.red());
        view.render_row(frame, 16, vec![Span::styled(format!("  {}", msg), red)]);
    }

    view.render_help(frame, vec![("Enter", "play"), ("Esc", "quit")]);
}

fn draw_betting(
    frame: &mut Frame,
    area: Rect,
    state: &BlackjackState,
    colors: &ThemeColors,
    credits: i64,
) {
    let view = FullScreenView::new(area, " Place Your Bet ", colors);
    view.render_frame(frame);

    let yellow = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let white = Style::default().fg(colors.fg());
    let green = Style::default().fg(colors.green());

    view.render_row(
        frame,
        2,
        vec![Span::styled("  Select your bet amount:", white)],
    );

    // Bet display
    view.render_row(
        frame,
        4,
        vec![Span::styled("       ╔═══════════╗".to_string(), yellow)],
    );
    view.render_row(
        frame,
        5,
        vec![Span::styled(
            format!("       ║  BET: {:>3} ║", state.current_bet),
            yellow,
        )],
    );
    view.render_row(
        frame,
        6,
        vec![Span::styled("       ╚═══════════╝".to_string(), yellow)],
    );

    view.render_row(
        frame,
        8,
        vec![Span::styled(format!("  Credits: {}", credits), green)],
    );

    view.render_row(frame, 10, vec![Span::styled("  ↑/↓: ±5  ←/→: ±50", white)]);

    // Message
    if let Some(msg) = &state.message {
        let red = Style::default().fg(colors.red());
        view.render_row(frame, 12, vec![Span::styled(format!("  {}", msg), red)]);
    }

    view.render_help(frame, vec![("Enter", "deal"), ("Esc", "back")]);
}

fn draw_game(
    frame: &mut Frame,
    area: Rect,
    state: &BlackjackState,
    colors: &ThemeColors,
    credits: i64,
) {
    let view = FullScreenView::new(area, " Blackjack ", colors);
    view.render_frame(frame);

    let yellow = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let white = Style::default().fg(colors.fg());
    let green = Style::default().fg(colors.green());
    let red = Style::default().fg(colors.red());
    let grey = Style::default().fg(colors.grey());

    // Dealer section
    view.render_row(frame, 1, vec![Span::styled("  DEALER", grey)]);
    let dealer_cards = render_hand(&state.dealer_hand, colors);
    for (i, row) in dealer_cards.iter().enumerate() {
        view.render_row(frame, 2 + i as u16, row.clone());
    }

    let dealer_val = calculate_hand_value(&state.dealer_hand);
    let dealer_str = if state.dealer_hand.iter().any(|c| !c.face_up) {
        "  Value: ?".to_string()
    } else {
        format!("  Value: {}", dealer_val)
    };
    view.render_row(frame, 7, vec![Span::styled(dealer_str, white)]);

    // Player section
    view.render_row(frame, 9, vec![Span::styled("  YOUR HAND", green)]);
    let player_cards = render_hand(&state.player_hand, colors);
    for (i, row) in player_cards.iter().enumerate() {
        view.render_row(frame, 10 + i as u16, row.clone());
    }

    let player_val = calculate_hand_value(&state.player_hand);
    let val_style = if player_val > 21 { red } else { yellow };
    view.render_row(
        frame,
        15,
        vec![Span::styled(format!("  Value: {}", player_val), val_style)],
    );

    // Bet info
    view.render_row(
        frame,
        17,
        vec![Span::styled(
            format!("  Bet: {} | Credits: {}", state.current_bet, credits),
            white,
        )],
    );

    // Message or status
    if let Some(msg) = &state.message {
        view.render_row(frame, 19, vec![Span::styled(format!("  {}", msg), red)]);
    } else if state.view == BlackjackView::DealerTurn {
        view.render_row(
            frame,
            19,
            vec![Span::styled("  Dealer is playing...", grey)],
        );
    }

    // Help based on state
    if state.view == BlackjackView::PlayerTurn {
        view.render_help(
            frame,
            vec![("H", "hit"), ("S", "stand"), ("Esc", "forfeit")],
        );
    } else {
        view.render_help(frame, vec![("", "dealer playing...")]);
    }
}

fn draw_result(
    frame: &mut Frame,
    area: Rect,
    state: &BlackjackState,
    colors: &ThemeColors,
    credits: i64,
) {
    let view = FullScreenView::new(area, " Round Result ", colors);
    view.render_frame(frame);

    let yellow = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let white = Style::default().fg(colors.fg());
    let green = Style::default().fg(colors.green());
    let red = Style::default().fg(colors.red());

    // Show final hands
    let dealer_cards = render_hand(&state.dealer_hand, colors);
    view.render_row(
        frame,
        1,
        vec![Span::styled(
            format!("  DEALER: {}", state.dealer_value()),
            white,
        )],
    );
    for (i, row) in dealer_cards.iter().enumerate() {
        view.render_row(frame, 2 + i as u16, row.clone());
    }

    let player_cards = render_hand(&state.player_hand, colors);
    view.render_row(
        frame,
        8,
        vec![Span::styled(
            format!("  YOU: {}", state.player_value()),
            white,
        )],
    );
    for (i, row) in player_cards.iter().enumerate() {
        view.render_row(frame, 9 + i as u16, row.clone());
    }

    // Result message
    if let Some(result) = &state.result {
        let (msg, style) = match result {
            super::super::blackjack::RoundResult::PlayerBlackjack => (result.message(), green),
            super::super::blackjack::RoundResult::PlayerWins => (result.message(), green),
            super::super::blackjack::RoundResult::Push => (result.message(), yellow),
            _ => (result.message(), red),
        };
        view.render_row(frame, 15, vec![Span::styled(format!("  {}", msg), style)]);
    }

    // Credits
    view.render_row(
        frame,
        17,
        vec![Span::styled(format!("  Credits: {}", credits), yellow)],
    );

    view.render_help(frame, vec![("Enter", "continue"), ("Esc", "quit")]);
}

/// Render a hand of cards as ASCII art rows
fn render_hand(cards: &[Card], colors: &ThemeColors) -> Vec<Vec<Span<'static>>> {
    let mut rows: Vec<Vec<Span<'static>>> = vec![Vec::new(); 5];

    let white = Style::default().fg(colors.fg());
    let red_style = Style::default().fg(colors.red());
    let grey = Style::default().fg(colors.grey());

    for card in cards {
        if card.face_up {
            let suit = card.suit.symbol();
            let rank = card.rank.symbol();
            let style = if card.suit.is_red() { red_style } else { white };

            // Pad rank to 2 chars for alignment
            let rank_left = format!("{:<2}", rank);
            let rank_right = format!("{:>2}", rank);

            rows[0].push(Span::styled("  ┌─────┐", white));
            rows[1].push(Span::styled(format!("  │{}   │", rank_left), style));
            rows[2].push(Span::styled(format!("  │  {}  │", suit), style));
            rows[3].push(Span::styled(format!("  │   {}│", rank_right), style));
            rows[4].push(Span::styled("  └─────┘", white));
        } else {
            // Face down card
            rows[0].push(Span::styled("  ┌─────┐", grey));
            rows[1].push(Span::styled("  │░░░░░│", grey));
            rows[2].push(Span::styled("  │░░░░░│", grey));
            rows[3].push(Span::styled("  │░░░░░│", grey));
            rows[4].push(Span::styled("  └─────┘", grey));
        }
    }

    rows
}
