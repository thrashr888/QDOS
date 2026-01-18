//! BACCARAT - Casino card game
//!
//! Punto banco baccarat with player/banker/tie bets.

use super::platform::{GameEngine, GameEvent, KeyHandleResult};
use crossterm::event::{KeyCode, KeyEvent};
use rand::prelude::*;

// =============================================================================
// CONSTANTS
// =============================================================================

const STARTING_BET: i64 = 10;
const MIN_BET: i64 = 5;
const MAX_BET: i64 = 500;
const DEAL_DELAY: u32 = 20; // Ticks between dealing cards

// =============================================================================
// CARD TYPES
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

impl Suit {
    pub fn symbol(&self) -> char {
        match self {
            Suit::Hearts => '♥',
            Suit::Diamonds => '♦',
            Suit::Clubs => '♣',
            Suit::Spades => '♠',
        }
    }

    pub fn is_red(&self) -> bool {
        matches!(self, Suit::Hearts | Suit::Diamonds)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rank {
    Ace,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
}

impl Rank {
    pub fn symbol(&self) -> &'static str {
        match self {
            Rank::Ace => "A",
            Rank::Two => "2",
            Rank::Three => "3",
            Rank::Four => "4",
            Rank::Five => "5",
            Rank::Six => "6",
            Rank::Seven => "7",
            Rank::Eight => "8",
            Rank::Nine => "9",
            Rank::Ten => "10",
            Rank::Jack => "J",
            Rank::Queen => "Q",
            Rank::King => "K",
        }
    }

    /// Baccarat value: A=1, 2-9=face value, 10/J/Q/K=0
    pub fn value(&self) -> u8 {
        match self {
            Rank::Ace => 1,
            Rank::Two => 2,
            Rank::Three => 3,
            Rank::Four => 4,
            Rank::Five => 5,
            Rank::Six => 6,
            Rank::Seven => 7,
            Rank::Eight => 8,
            Rank::Nine => 9,
            Rank::Ten | Rank::Jack | Rank::Queen | Rank::King => 0,
        }
    }

    pub fn all() -> &'static [Rank] {
        &[
            Rank::Ace,
            Rank::Two,
            Rank::Three,
            Rank::Four,
            Rank::Five,
            Rank::Six,
            Rank::Seven,
            Rank::Eight,
            Rank::Nine,
            Rank::Ten,
            Rank::Jack,
            Rank::Queen,
            Rank::King,
        ]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Card {
    pub rank: Rank,
    pub suit: Suit,
}

impl Card {
    pub fn new(rank: Rank, suit: Suit) -> Self {
        Self { rank, suit }
    }

    pub fn value(&self) -> u8 {
        self.rank.value()
    }
}

/// 8-deck shoe for baccarat
#[derive(Debug, Clone)]
pub struct Shoe {
    cards: Vec<Card>,
}

impl Shoe {
    pub fn new() -> Self {
        let mut cards = Vec::with_capacity(52 * 8);
        for _ in 0..8 {
            for suit in [Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades] {
                for rank in Rank::all() {
                    cards.push(Card::new(*rank, suit));
                }
            }
        }
        Self { cards }
    }

    pub fn shuffle(&mut self) {
        let mut rng = rand::thread_rng();
        self.cards.shuffle(&mut rng);
    }

    pub fn draw(&mut self) -> Option<Card> {
        self.cards.pop()
    }

    pub fn remaining(&self) -> usize {
        self.cards.len()
    }
}

impl Default for Shoe {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// BET TYPES
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BetType {
    #[default]
    Player, // Pays 1:1
    Banker, // Pays 0.95:1 (5% commission)
    Tie,    // Pays 8:1
}

impl BetType {
    pub fn name(&self) -> &'static str {
        match self {
            BetType::Player => "Player",
            BetType::Banker => "Banker",
            BetType::Tie => "Tie",
        }
    }

    pub fn payout_str(&self) -> &'static str {
        match self {
            BetType::Player => "1:1",
            BetType::Banker => "0.95:1",
            BetType::Tie => "8:1",
        }
    }
}

// =============================================================================
// HAND CALCULATION
// =============================================================================

/// Calculate baccarat hand value (mod 10)
pub fn hand_value(cards: &[Card]) -> u8 {
    let sum: u8 = cards.iter().map(|c| c.value()).sum();
    sum % 10
}

/// Determine if a third card should be drawn
pub fn player_draws_third(player_value: u8) -> bool {
    player_value <= 5
}

/// Determine if banker draws third card (depends on player's third card)
pub fn banker_draws_third(banker_value: u8, player_third: Option<Card>) -> bool {
    match player_third {
        None => banker_value <= 5, // Player stood, banker uses simple rule
        Some(third) => {
            let p3 = third.value();
            match banker_value {
                0..=2 => true,
                3 => p3 != 8,
                4 => (2..=7).contains(&p3),
                5 => (4..=7).contains(&p3),
                6 => (6..=7).contains(&p3),
                _ => false,
            }
        }
    }
}

// =============================================================================
// GAME STATE
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BaccaratView {
    #[default]
    Menu,
    Betting,
    BetSelect,
    Dealing,
    Result,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Winner {
    Player,
    Banker,
    Tie,
}

#[derive(Debug, Clone)]
pub struct BaccaratState {
    pub view: BaccaratView,
    pub shoe: Shoe,
    pub player_hand: Vec<Card>,
    pub banker_hand: Vec<Card>,
    pub bet_type: BetType,
    pub current_bet: i64,
    pub available_credits: i64,
    pub last_win: i64,
    pub winner: Option<Winner>,
    pub message: Option<String>,
    pub hands_played: u32,
    pub total_won: i64,
    pub total_bet: i64,
    pub deal_phase: u32, // For dealing animation
    pub deal_timer: u32, // Timer for animation
    pub player_wins: u32,
    pub banker_wins: u32,
    pub ties: u32,
    pub tick_count: u32,
    pub game_over: bool,
    pub pending_events: Vec<GameEvent>,
}

impl Default for BaccaratState {
    fn default() -> Self {
        Self::new()
    }
}

impl BaccaratState {
    pub fn new() -> Self {
        let mut shoe = Shoe::new();
        shoe.shuffle();

        Self {
            view: BaccaratView::Menu,
            shoe,
            player_hand: Vec::new(),
            banker_hand: Vec::new(),
            bet_type: BetType::Player,
            current_bet: STARTING_BET,
            available_credits: 0,
            last_win: 0,
            winner: None,
            message: None,
            hands_played: 0,
            total_won: 0,
            total_bet: 0,
            deal_phase: 0,
            deal_timer: 0,
            player_wins: 0,
            banker_wins: 0,
            ties: 0,
            tick_count: 0,
            game_over: false,
            pending_events: Vec::new(),
        }
    }

    pub fn set_credits(&mut self, credits: i64) {
        self.available_credits = credits;
        self.current_bet = self.current_bet.min(credits).max(MIN_BET);
    }

    fn start_dealing(&mut self) {
        if self.current_bet > self.available_credits {
            self.message = Some("Not enough credits!".to_string());
            return;
        }

        // Deduct bet
        self.available_credits -= self.current_bet;
        self.total_bet += self.current_bet;
        self.hands_played += 1;
        self.last_win = 0;
        self.winner = None;
        self.message = None;

        // Reset hands
        self.player_hand.clear();
        self.banker_hand.clear();
        self.deal_phase = 0;
        self.deal_timer = DEAL_DELAY;

        // Check if shoe needs reshuffling
        if self.shoe.remaining() < 20 {
            self.shoe = Shoe::new();
            self.shoe.shuffle();
        }

        self.view = BaccaratView::Dealing;
    }

    fn update_dealing(&mut self) {
        if self.deal_timer > 0 {
            self.deal_timer -= 1;
            return;
        }

        match self.deal_phase {
            0 => {
                // First player card
                if let Some(card) = self.shoe.draw() {
                    self.player_hand.push(card);
                }
                self.deal_phase = 1;
                self.deal_timer = DEAL_DELAY;
            }
            1 => {
                // First banker card
                if let Some(card) = self.shoe.draw() {
                    self.banker_hand.push(card);
                }
                self.deal_phase = 2;
                self.deal_timer = DEAL_DELAY;
            }
            2 => {
                // Second player card
                if let Some(card) = self.shoe.draw() {
                    self.player_hand.push(card);
                }
                self.deal_phase = 3;
                self.deal_timer = DEAL_DELAY;
            }
            3 => {
                // Second banker card
                if let Some(card) = self.shoe.draw() {
                    self.banker_hand.push(card);
                }
                self.deal_phase = 4;
                self.deal_timer = DEAL_DELAY;
            }
            4 => {
                // Check for naturals (8 or 9)
                let player_val = hand_value(&self.player_hand);
                let banker_val = hand_value(&self.banker_hand);

                if player_val >= 8 || banker_val >= 8 {
                    // Natural - no more cards
                    self.determine_winner();
                    return;
                }

                // Check if player draws third
                if player_draws_third(player_val) {
                    self.deal_phase = 5;
                    self.deal_timer = DEAL_DELAY;
                } else {
                    // Player stands, check banker
                    if banker_draws_third(banker_val, None) {
                        self.deal_phase = 6;
                        self.deal_timer = DEAL_DELAY;
                    } else {
                        self.determine_winner();
                    }
                }
            }
            5 => {
                // Player's third card
                if let Some(card) = self.shoe.draw() {
                    self.player_hand.push(card);
                }
                let banker_val = hand_value(&self.banker_hand);
                let player_third = self.player_hand.get(2).copied();

                if banker_draws_third(banker_val, player_third) {
                    self.deal_phase = 6;
                    self.deal_timer = DEAL_DELAY;
                } else {
                    self.determine_winner();
                }
            }
            6 => {
                // Banker's third card
                if let Some(card) = self.shoe.draw() {
                    self.banker_hand.push(card);
                }
                self.determine_winner();
            }
            _ => {
                self.determine_winner();
            }
        }
    }

    fn determine_winner(&mut self) {
        let player_val = hand_value(&self.player_hand);
        let banker_val = hand_value(&self.banker_hand);

        let winner = if player_val > banker_val {
            self.player_wins += 1;
            Winner::Player
        } else if banker_val > player_val {
            self.banker_wins += 1;
            Winner::Banker
        } else {
            self.ties += 1;
            Winner::Tie
        };

        self.winner = Some(winner);

        // Calculate payout
        let win = match (self.bet_type, winner) {
            (BetType::Player, Winner::Player) => self.current_bet * 2,
            (BetType::Banker, Winner::Banker) => {
                // 5% commission
                let gross = self.current_bet * 2;
                let commission = self.current_bet / 20;
                gross - commission
            }
            (BetType::Tie, Winner::Tie) => self.current_bet * 9,
            _ => 0,
        };

        if win > 0 {
            self.last_win = win - self.current_bet; // Net win
            self.available_credits += win;
            self.total_won += self.last_win;
            self.message = Some(format!(
                "{} wins! {} vs {} - You win ${}!",
                match winner {
                    Winner::Player => "Player",
                    Winner::Banker => "Banker",
                    Winner::Tie => "Tie",
                },
                player_val,
                banker_val,
                self.last_win
            ));
        } else {
            self.last_win = 0;
            self.message = Some(format!(
                "{} wins! {} vs {} - You lose.",
                match winner {
                    Winner::Player => "Player",
                    Winner::Banker => "Banker",
                    Winner::Tie => "Tie",
                },
                player_val,
                banker_val
            ));
        }

        self.view = BaccaratView::Result;
    }

    fn adjust_bet(&mut self, delta: i64) {
        self.current_bet =
            (self.current_bet + delta).clamp(MIN_BET, MAX_BET.min(self.available_credits));
    }

    fn cycle_bet_type(&mut self, forward: bool) {
        self.bet_type = match (self.bet_type, forward) {
            (BetType::Player, true) => BetType::Banker,
            (BetType::Banker, true) => BetType::Tie,
            (BetType::Tie, true) => BetType::Player,
            (BetType::Player, false) => BetType::Tie,
            (BetType::Banker, false) => BetType::Player,
            (BetType::Tie, false) => BetType::Banker,
        };
    }
}

// =============================================================================
// GAME ENGINE
// =============================================================================

impl GameEngine for BaccaratState {
    fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);

        if self.view == BaccaratView::Dealing {
            self.update_dealing();
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match self.view {
            BaccaratView::Menu => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if self.available_credits >= MIN_BET {
                        self.view = BaccaratView::Betting;
                        KeyHandleResult::Handled
                    } else {
                        self.message = Some("No credits remaining!".to_string());
                        KeyHandleResult::Handled
                    }
                }
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                    KeyHandleResult::RequestQuit
                }
                _ => KeyHandleResult::Handled,
            },
            BaccaratView::Betting | BaccaratView::BetSelect => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.start_dealing();
                    KeyHandleResult::Handled
                }
                KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                    self.adjust_bet(10);
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.adjust_bet(-10);
                    KeyHandleResult::Handled
                }
                KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') => {
                    self.cycle_bet_type(false);
                    KeyHandleResult::Handled
                }
                KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('D') => {
                    self.cycle_bet_type(true);
                    KeyHandleResult::Handled
                }
                KeyCode::Char('p') | KeyCode::Char('P') => {
                    self.bet_type = BetType::Player;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('b') | KeyCode::Char('B') => {
                    self.bet_type = BetType::Banker;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('t') | KeyCode::Char('T') => {
                    self.bet_type = BetType::Tie;
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => {
                    self.view = BaccaratView::Menu;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            BaccaratView::Dealing => KeyHandleResult::Handled,
            BaccaratView::Result => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if self.available_credits >= MIN_BET {
                        self.view = BaccaratView::Betting;
                        self.current_bet = self.current_bet.min(self.available_credits);
                        KeyHandleResult::Handled
                    } else {
                        self.message = Some("Out of credits!".to_string());
                        self.game_over = true;
                        self.pending_events
                            .push(GameEvent::GameEnded { won: false });
                        KeyHandleResult::GameOver
                    }
                }
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                    KeyHandleResult::RequestQuit
                }
                _ => KeyHandleResult::Handled,
            },
        }
    }

    fn get_score(&self) -> u32 {
        self.total_won.max(0) as u32
    }

    fn is_game_over(&self) -> bool {
        self.game_over
    }

    fn is_game_won(&self) -> bool {
        self.total_won > self.total_bet
    }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.pending_events)
    }
}
