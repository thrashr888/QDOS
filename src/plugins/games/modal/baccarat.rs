//! BACCARAT modal rendering
//!
//! Renders the punto banco baccarat card game.

use super::super::baccarat::{hand_value, BaccaratState, BaccaratView, BetType};
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

pub fn draw(frame: &mut Frame, area: Rect, state: &BaccaratState, colors: &ThemeColors) {
    match state.view {
        BaccaratView::Menu => draw_menu(frame, area, state, colors),
        BaccaratView::Betting | BaccaratView::BetSelect => draw_betting(frame, area, state, colors),
        BaccaratView::Dealing => draw_dealing(frame, area, state, colors),
        BaccaratView::Result => draw_result(frame, area, state, colors),
    }
}

// =============================================================================
// MENU SCREEN
// =============================================================================

fn draw_menu(frame: &mut Frame, area: Rect, state: &BaccaratState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " BACCARAT ", colors);
    view.render_frame(frame);

    let title_style = Style::default()
        .fg(colors.red())
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
            "  ║      ♠ ♥ BACCARAT ♦ ♣                   ║",
            title_style,
        )],
    );
    view.render_row(
        frame,
        4,
        vec![Span::styled(
            "  ║         Punto Banco                     ║",
            title_style,
        )],
    );
    view.render_row(
        frame,
        5,
        vec![Span::styled(
            "  ╚═════════════════════════════════════════╝",
            title_style,
        )],
    );

    // Rules
    view.render_row(frame, 7, vec![Span::styled("  RULES:", highlight)]);
    view.render_row(
        frame,
        8,
        vec![Span::styled("  Bet on Player, Banker, or Tie", text_style)],
    );
    view.render_row(
        frame,
        9,
        vec![Span::styled(
            "  Cards dealt automatically following casino rules",
            text_style,
        )],
    );
    view.render_row(
        frame,
        10,
        vec![Span::styled("  Hand closest to 9 wins", text_style)],
    );

    // Payouts
    view.render_row(frame, 12, vec![Span::styled("  PAYOUTS:", highlight)]);
    view.render_row(
        frame,
        13,
        vec![Span::styled("  Player bet:  1:1", text_style)],
    );
    view.render_row(
        frame,
        14,
        vec![Span::styled(
            "  Banker bet:  0.95:1 (5% commission)",
            text_style,
        )],
    );
    view.render_row(
        frame,
        15,
        vec![Span::styled("  Tie bet:     8:1", text_style)],
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

fn draw_betting(frame: &mut Frame, area: Rect, state: &BaccaratState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " BACCARAT ", colors);
    view.render_frame(frame);

    let title_style = Style::default()
        .fg(colors.yellow())
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(colors.fg());

    // Table layout
    view.render_row(
        frame,
        2,
        vec![Span::styled(
            "  ╔═══════════════════════════════════════╗",
            text_style,
        )],
    );
    view.render_row(
        frame,
        3,
        vec![Span::styled(
            "  ║   PLAYER           BANKER             ║",
            text_style,
        )],
    );
    view.render_row(
        frame,
        4,
        vec![Span::styled(
            "  ║   ┌───┐ ┌───┐     ┌───┐ ┌───┐        ║",
            text_style,
        )],
    );
    view.render_row(
        frame,
        5,
        vec![Span::styled(
            "  ║   │   │ │   │     │   │ │   │        ║",
            text_style,
        )],
    );
    view.render_row(
        frame,
        6,
        vec![Span::styled(
            "  ║   └───┘ └───┘     └───┘ └───┘        ║",
            text_style,
        )],
    );
    view.render_row(
        frame,
        7,
        vec![Span::styled(
            "  ╚═══════════════════════════════════════╝",
            text_style,
        )],
    );

    // Bet type selection
    view.render_row(
        frame,
        9,
        vec![Span::styled("  Select your bet:", title_style)],
    );

    let player_style = if state.bet_type == BetType::Player {
        Style::default()
            .fg(colors.yellow())
            .add_modifier(Modifier::BOLD)
    } else {
        text_style
    };
    let banker_style = if state.bet_type == BetType::Banker {
        Style::default()
            .fg(colors.yellow())
            .add_modifier(Modifier::BOLD)
    } else {
        text_style
    };
    let tie_style = if state.bet_type == BetType::Tie {
        Style::default()
            .fg(colors.yellow())
            .add_modifier(Modifier::BOLD)
    } else {
        text_style
    };

    view.render_row(
        frame,
        11,
        vec![
            Span::styled(
                if state.bet_type == BetType::Player {
                    "  ► [P]LAYER 1:1 ◄    "
                } else {
                    "    [P]layer 1:1      "
                },
                player_style,
            ),
            Span::styled(
                if state.bet_type == BetType::Banker {
                    "► [B]ANKER 0.95:1 ◄    "
                } else {
                    "  [B]anker 0.95:1      "
                },
                banker_style,
            ),
            Span::styled(
                if state.bet_type == BetType::Tie {
                    "► [T]IE 8:1 ◄"
                } else {
                    "  [T]ie 8:1  "
                },
                tie_style,
            ),
        ],
    );

    // Bet amount
    view.render_row(
        frame,
        14,
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
        16,
        vec![Span::styled(
            format!(
                "  Player: {}  Banker: {}  Tie: {}  (last {} hands)",
                state.player_wins, state.banker_wins, state.ties, state.hands_played
            ),
            Style::default().fg(colors.grey()),
        )],
    );

    view.render_help(
        frame,
        vec![
            ("Enter", "deal"),
            ("←→/P/B/T", "bet type"),
            ("↑↓", "amount"),
            ("Esc", "back"),
        ],
    );
}

// =============================================================================
// DEALING SCREEN
// =============================================================================

fn draw_dealing(frame: &mut Frame, area: Rect, state: &BaccaratState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " BACCARAT ", colors);
    view.render_frame(frame);

    let text_style = Style::default().fg(colors.fg());

    view.render_row(
        frame,
        2,
        vec![Span::styled(
            "  ╔═══════════════════════════════════════╗",
            text_style,
        )],
    );

    // Draw hands
    draw_table(frame, &view, state, colors);

    view.render_row(
        frame,
        7,
        vec![Span::styled(
            "  ╚═══════════════════════════════════════╝",
            text_style,
        )],
    );

    view.render_row(
        frame,
        9,
        vec![Span::styled(
            "  Dealing...",
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        )],
    );

    view.render_row(
        frame,
        11,
        vec![Span::styled(
            format!("  Bet: {} - ${}", state.bet_type.name(), state.current_bet),
            text_style,
        )],
    );
}

// =============================================================================
// RESULT SCREEN
// =============================================================================

fn draw_result(frame: &mut Frame, area: Rect, state: &BaccaratState, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " BACCARAT ", colors);
    view.render_frame(frame);

    let text_style = Style::default().fg(colors.fg());

    view.render_row(
        frame,
        2,
        vec![Span::styled(
            "  ╔═══════════════════════════════════════╗",
            text_style,
        )],
    );

    draw_table(frame, &view, state, colors);

    view.render_row(
        frame,
        7,
        vec![Span::styled(
            "  ╚═══════════════════════════════════════╝",
            text_style,
        )],
    );

    // Values
    let player_val = hand_value(&state.player_hand);
    let banker_val = hand_value(&state.banker_hand);

    view.render_row(
        frame,
        9,
        vec![Span::styled(
            format!("  Player: {}         Banker: {}", player_val, banker_val),
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
                "  Hands: {}  Won: ${}  |  P:{} B:{} T:{}",
                state.hands_played,
                state.total_won,
                state.player_wins,
                state.banker_wins,
                state.ties
            ),
            Style::default().fg(colors.grey()),
        )],
    );

    if state.available_credits >= 5 {
        view.render_help(frame, vec![("Enter", "play again"), ("Esc", "quit")]);
    } else {
        view.render_help(frame, vec![("Esc", "out of credits")]);
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

fn draw_table(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &BaccaratState,
    colors: &ThemeColors,
) {
    let text_style = Style::default().fg(colors.fg());

    // Header
    view.render_row(
        frame,
        3,
        vec![Span::styled(
            "  ║   PLAYER              BANKER            ║",
            text_style,
        )],
    );

    // Card positions
    let mut player_cards = String::new();
    for (i, card) in state.player_hand.iter().enumerate() {
        if i > 0 {
            player_cards.push(' ');
        }
        player_cards.push_str(&format!("{}{}", card.rank.symbol(), card.suit.symbol()));
    }
    while player_cards.len() < 12 {
        player_cards.push(' ');
    }

    let mut banker_cards = String::new();
    for (i, card) in state.banker_hand.iter().enumerate() {
        if i > 0 {
            banker_cards.push(' ');
        }
        banker_cards.push_str(&format!("{}{}", card.rank.symbol(), card.suit.symbol()));
    }
    while banker_cards.len() < 12 {
        banker_cards.push(' ');
    }

    view.render_row(
        frame,
        5,
        vec![
            Span::styled("  ║   ", text_style),
            Span::styled(player_cards, Style::default().fg(colors.cyan())),
            Span::styled("      ", text_style),
            Span::styled(banker_cards, Style::default().fg(colors.yellow())),
            Span::styled("    ║", text_style),
        ],
    );

    // Values if cards dealt
    if !state.player_hand.is_empty() {
        let player_val = hand_value(&state.player_hand);
        let banker_val = if !state.banker_hand.is_empty() {
            hand_value(&state.banker_hand)
        } else {
            0
        };

        view.render_row(
            frame,
            6,
            vec![Span::styled(
                format!(
                    "  ║   Value: {:<5}       Value: {:<5}      ║",
                    player_val, banker_val
                ),
                text_style,
            )],
        );
    } else {
        view.render_row(
            frame,
            6,
            vec![Span::styled(
                "  ║                                         ║",
                text_style,
            )],
        );
    }
}
