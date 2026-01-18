//! BLACKJACK - Casino card game
//!
//! Classic 21 card game. Beat the dealer without going over 21!

use super::platform::{GameEngine, GameEvent, KeyHandleResult};
use crossterm::event::{KeyCode, KeyEvent};
use rand::prelude::*;

// =============================================================================
// CONSTANTS
// =============================================================================

const STARTING_BET: i64 = 10;
const MIN_BET: i64 = 5;
const MAX_BET: i64 = 500;

// =============================================================================
// CARD TYPES
// =============================================================================

/// Card suits
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

/// Card ranks
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

    pub fn value(&self) -> u8 {
        match self {
            Rank::Ace => 11, // Can also be 1
            Rank::Two => 2,
            Rank::Three => 3,
            Rank::Four => 4,
            Rank::Five => 5,
            Rank::Six => 6,
            Rank::Seven => 7,
            Rank::Eight => 8,
            Rank::Nine => 9,
            Rank::Ten | Rank::Jack | Rank::Queen | Rank::King => 10,
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

/// A playing card
#[derive(Debug, Clone, Copy)]
pub struct Card {
    pub rank: Rank,
    pub suit: Suit,
    pub face_up: bool,
}

impl Card {
    pub fn new(rank: Rank, suit: Suit) -> Self {
        Self {
            rank,
            suit,
            face_up: true,
        }
    }

    pub fn value(&self) -> u8 {
        self.rank.value()
    }
}

/// A deck of cards
#[derive(Debug, Clone)]
pub struct Deck {
    cards: Vec<Card>,
}

impl Deck {
    pub fn new() -> Self {
        let mut cards = Vec::with_capacity(52);
        for suit in [Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades] {
            for rank in Rank::all() {
                cards.push(Card::new(*rank, suit));
            }
        }
        Self { cards }
    }

    pub fn shuffle(&mut self, rng: &mut impl Rng) {
        self.cards.shuffle(rng);
    }

    pub fn draw(&mut self) -> Option<Card> {
        self.cards.pop()
    }
}

impl Default for Deck {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// HAND CALCULATION
// =============================================================================

/// Calculate the best hand value, accounting for aces
pub fn calculate_hand_value(cards: &[Card]) -> u8 {
    let mut value: u16 = 0;
    let mut aces = 0;

    for card in cards {
        if card.face_up {
            value += card.value() as u16;
            if card.rank == Rank::Ace {
                aces += 1;
            }
        }
    }

    // Convert aces from 11 to 1 as needed
    while value > 21 && aces > 0 {
        value -= 10;
        aces -= 1;
    }

    value.min(255) as u8
}

/// Check if hand is a blackjack (21 with 2 cards)
pub fn is_blackjack(cards: &[Card]) -> bool {
    cards.len() == 2 && calculate_hand_value(cards) == 21
}

/// Check if hand is busted (over 21)
pub fn is_busted(cards: &[Card]) -> bool {
    calculate_hand_value(cards) > 21
}

// =============================================================================
// GAME STATE
// =============================================================================

/// Current phase of the game
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlackjackView {
    #[default]
    Menu,
    Betting,
    PlayerTurn,
    DealerTurn,
    Result,
}

/// Result of a round
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundResult {
    PlayerBlackjack, // 3:2 payout
    PlayerWins,      // 1:1 payout
    DealerWins,      // Lose bet
    Push,            // Tie, bet returned
    PlayerBusts,     // Lose bet
}

impl RoundResult {
    pub fn message(&self) -> &'static str {
        match self {
            RoundResult::PlayerBlackjack => "BLACKJACK! You win 3:2!",
            RoundResult::PlayerWins => "You win!",
            RoundResult::DealerWins => "Dealer wins.",
            RoundResult::Push => "Push - tie game.",
            RoundResult::PlayerBusts => "BUST! You lose.",
        }
    }
}

/// Main game state
pub struct BlackjackState {
    pub view: BlackjackView,
    pub deck: Deck,
    pub player_hand: Vec<Card>,
    pub dealer_hand: Vec<Card>,
    pub current_bet: i64,
    pub available_credits: i64, // Casino wallet balance
    pub result: Option<RoundResult>,
    pub message: Option<String>,
    pub message_timer: u32,
    pub hands_played: u32,
    pub hands_won: u32,
    pub total_winnings: i64,
    pub biggest_win: i64,
    pub blackjacks: u32,
    pub game_over: bool,
    pub tick_count: u32,
    pub dealer_reveal_timer: u32,
    events: Vec<GameEvent>,
}

impl Default for BlackjackState {
    fn default() -> Self {
        Self::new()
    }
}

impl BlackjackState {
    pub fn new() -> Self {
        Self {
            view: BlackjackView::Menu,
            deck: Deck::new(),
            player_hand: Vec::new(),
            dealer_hand: Vec::new(),
            current_bet: STARTING_BET,
            available_credits: 0,
            result: None,
            message: None,
            message_timer: 0,
            hands_played: 0,
            hands_won: 0,
            total_winnings: 0,
            biggest_win: 0,
            blackjacks: 0,
            game_over: false,
            tick_count: 0,
            dealer_reveal_timer: 0,
            events: Vec::new(),
        }
    }

    /// Set available credits (called by plugin before starting)
    pub fn set_credits(&mut self, credits: i64) {
        self.available_credits = credits;
        self.current_bet = STARTING_BET.min(credits).max(MIN_BET);
    }

    /// Get net winnings for this session (for updating wallet)
    pub fn get_net_winnings(&self) -> i64 {
        self.total_winnings
    }

    pub fn start_game(&mut self) {
        self.view = BlackjackView::Betting;
        self.current_bet = self.current_bet.min(self.available_credits).max(MIN_BET);
        self.game_over = false;
        self.result = None;
        self.message = None;
    }

    pub fn deal(&mut self) {
        // Fresh deck and shuffle
        self.deck = Deck::new();
        let mut rng = rand::thread_rng();
        self.deck.shuffle(&mut rng);

        // Clear hands
        self.player_hand.clear();
        self.dealer_hand.clear();

        // Deduct bet from credits
        self.available_credits -= self.current_bet;

        // Deal 2 cards each (player, dealer, player, dealer)
        if let Some(card) = self.deck.draw() {
            self.player_hand.push(card);
        }
        if let Some(mut card) = self.deck.draw() {
            card.face_up = true;
            self.dealer_hand.push(card);
        }
        if let Some(card) = self.deck.draw() {
            self.player_hand.push(card);
        }
        if let Some(mut card) = self.deck.draw() {
            card.face_up = false; // Hole card face down
            self.dealer_hand.push(card);
        }

        self.view = BlackjackView::PlayerTurn;
        self.result = None;

        // Check for immediate blackjack
        if is_blackjack(&self.player_hand) {
            self.reveal_dealer_hand();
            if is_blackjack(&self.dealer_hand) {
                self.end_round(RoundResult::Push);
            } else {
                self.blackjacks += 1;
                self.end_round(RoundResult::PlayerBlackjack);
            }
        }
    }

    pub fn hit(&mut self) {
        if let Some(card) = self.deck.draw() {
            self.player_hand.push(card);
        }

        if is_busted(&self.player_hand) {
            self.end_round(RoundResult::PlayerBusts);
        }
    }

    pub fn stand(&mut self) {
        self.view = BlackjackView::DealerTurn;
        self.reveal_dealer_hand();
        self.dealer_reveal_timer = 0;
    }

    pub fn reveal_dealer_hand(&mut self) {
        for card in &mut self.dealer_hand {
            card.face_up = true;
        }
    }

    pub fn dealer_play(&mut self) -> bool {
        let dealer_value = calculate_hand_value(&self.dealer_hand);

        // Dealer hits on 16 or less, stands on 17+
        if dealer_value < 17 {
            if let Some(card) = self.deck.draw() {
                self.dealer_hand.push(card);
            }
            return false; // Not done yet
        }

        // Dealer is done, determine result
        if is_busted(&self.dealer_hand) {
            self.end_round(RoundResult::PlayerWins);
        } else {
            let player_value = calculate_hand_value(&self.player_hand);
            if player_value > dealer_value {
                self.end_round(RoundResult::PlayerWins);
            } else if dealer_value > player_value {
                self.end_round(RoundResult::DealerWins);
            } else {
                self.end_round(RoundResult::Push);
            }
        }
        true // Done
    }

    fn end_round(&mut self, result: RoundResult) {
        let old_score = self.get_score();

        self.result = Some(result);
        self.view = BlackjackView::Result;
        self.hands_played += 1;

        // Calculate winnings and update credits
        let payout = match result {
            RoundResult::PlayerBlackjack => self.current_bet + (self.current_bet * 3) / 2, // Bet + 3:2
            RoundResult::PlayerWins => self.current_bet * 2, // Bet + 1:1
            RoundResult::Push => self.current_bet,           // Bet returned
            RoundResult::DealerWins | RoundResult::PlayerBusts => 0, // Already deducted
        };

        self.available_credits += payout;

        // Track winnings (net of this hand)
        let net = payout - self.current_bet;
        if net > 0 {
            self.hands_won += 1;
            self.total_winnings += net;
            if net > self.biggest_win {
                self.biggest_win = net;
            }
        } else if net < 0 {
            self.total_winnings += net;
        }

        let new_score = self.get_score();
        self.events.push(GameEvent::ScoreChanged {
            old: old_score,
            new: new_score,
        });
    }

    pub fn player_value(&self) -> u8 {
        calculate_hand_value(&self.player_hand)
    }

    pub fn dealer_value(&self) -> u8 {
        calculate_hand_value(&self.dealer_hand)
    }

    pub fn show_message(&mut self, msg: &str) {
        self.message = Some(msg.to_string());
        self.message_timer = 60;
    }

    pub fn adjust_bet(&mut self, delta: i64) {
        self.current_bet =
            (self.current_bet + delta).clamp(MIN_BET, MAX_BET.min(self.available_credits));
    }
}

// =============================================================================
// GAME ENGINE IMPLEMENTATION
// =============================================================================

impl GameEngine for BlackjackState {
    fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);

        // Message timer
        if self.message_timer > 0 {
            self.message_timer -= 1;
            if self.message_timer == 0 {
                self.message = None;
            }
        }

        // Dealer turn - draw cards with delay
        if self.view == BlackjackView::DealerTurn {
            self.dealer_reveal_timer += 1;
            if self.dealer_reveal_timer >= 15 {
                // Every ~0.5 seconds
                self.dealer_reveal_timer = 0;
                self.dealer_play();
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match self.view {
            BlackjackView::Menu => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if self.available_credits >= MIN_BET {
                        self.start_game();
                        KeyHandleResult::Handled
                    } else {
                        self.show_message("Not enough credits!");
                        KeyHandleResult::Handled
                    }
                }
                KeyCode::Esc | KeyCode::Char('q') => KeyHandleResult::RequestQuit,
                _ => KeyHandleResult::NotHandled,
            },

            BlackjackView::Betting => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if self.current_bet <= self.available_credits {
                        self.deal();
                        KeyHandleResult::Handled
                    } else {
                        self.show_message("Not enough credits!");
                        KeyHandleResult::Handled
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.adjust_bet(5);
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.adjust_bet(-5);
                    KeyHandleResult::Handled
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    self.adjust_bet(50);
                    KeyHandleResult::Handled
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    self.adjust_bet(-50);
                    KeyHandleResult::Handled
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.view = BlackjackView::Menu;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::NotHandled,
            },

            BlackjackView::PlayerTurn => match key.code {
                KeyCode::Char('h') | KeyCode::Enter => {
                    self.hit();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('s') | KeyCode::Char(' ') => {
                    self.stand();
                    KeyHandleResult::Handled
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    // Forfeit hand
                    self.end_round(RoundResult::DealerWins);
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::NotHandled,
            },

            BlackjackView::DealerTurn => {
                // No input during dealer turn
                KeyHandleResult::Handled
            }

            BlackjackView::Result => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if self.available_credits >= MIN_BET {
                        self.view = BlackjackView::Betting;
                        self.result = None;
                        KeyHandleResult::Handled
                    } else {
                        self.game_over = true;
                        self.show_message("Out of credits!");
                        KeyHandleResult::GameOver
                    }
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.game_over = true;
                    KeyHandleResult::GameOver
                }
                _ => KeyHandleResult::NotHandled,
            },
        }
    }

    fn get_score(&self) -> u32 {
        self.total_winnings.max(0) as u32
    }

    fn is_game_over(&self) -> bool {
        self.game_over
    }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }
}
