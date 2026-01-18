//! POKER - Video Poker (Jacks or Better)
//!
//! Classic video poker with standard payouts. Hold cards and draw!

use super::platform::{GameEngine, GameEvent, KeyHandleResult};
use crossterm::event::{KeyCode, KeyEvent};
use rand::prelude::*;

// =============================================================================
// CONSTANTS
// =============================================================================

const STARTING_BET: i64 = 10;
const MIN_BET: i64 = 5;
const MAX_BET: i64 = 100;

// =============================================================================
// CARD TYPES (reuse pattern from blackjack)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rank {
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
    Seven = 7,
    Eight = 8,
    Nine = 9,
    Ten = 10,
    Jack = 11,
    Queen = 12,
    King = 13,
    Ace = 14,
}

impl Rank {
    pub fn symbol(&self) -> &'static str {
        match self {
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
            Rank::Ace => "A",
        }
    }

    pub fn all() -> &'static [Rank] {
        &[
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
            Rank::Ace,
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
}

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

    pub fn shuffle(&mut self) {
        let mut rng = rand::thread_rng();
        self.cards.shuffle(&mut rng);
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
// HAND RANKINGS
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HandRank {
    HighCard,
    JacksOrBetter, // Pair of Jacks, Queens, Kings, or Aces
    TwoPair,
    ThreeOfAKind,
    Straight,
    Flush,
    FullHouse,
    FourOfAKind,
    StraightFlush,
    RoyalFlush,
}

impl HandRank {
    pub fn name(&self) -> &'static str {
        match self {
            HandRank::HighCard => "High Card",
            HandRank::JacksOrBetter => "Jacks or Better",
            HandRank::TwoPair => "Two Pair",
            HandRank::ThreeOfAKind => "Three of a Kind",
            HandRank::Straight => "Straight",
            HandRank::Flush => "Flush",
            HandRank::FullHouse => "Full House",
            HandRank::FourOfAKind => "Four of a Kind",
            HandRank::StraightFlush => "Straight Flush",
            HandRank::RoyalFlush => "ROYAL FLUSH!",
        }
    }

    /// Payout multiplier for each hand
    pub fn payout(&self) -> i64 {
        match self {
            HandRank::HighCard => 0,
            HandRank::JacksOrBetter => 1,
            HandRank::TwoPair => 2,
            HandRank::ThreeOfAKind => 3,
            HandRank::Straight => 4,
            HandRank::Flush => 6,
            HandRank::FullHouse => 9,
            HandRank::FourOfAKind => 25,
            HandRank::StraightFlush => 50,
            HandRank::RoyalFlush => 250,
        }
    }
}

/// Evaluate a 5-card hand
pub fn evaluate_hand(cards: &[Card; 5]) -> HandRank {
    let mut ranks: Vec<Rank> = cards.iter().map(|c| c.rank).collect();
    ranks.sort();

    let is_flush = cards.iter().all(|c| c.suit == cards[0].suit);

    // Check for straight (including A-2-3-4-5 and 10-J-Q-K-A)
    let is_straight = is_sequential(&ranks);
    let is_ace_low_straight = ranks == [Rank::Two, Rank::Three, Rank::Four, Rank::Five, Rank::Ace];

    // Count rank occurrences
    let mut counts: Vec<(Rank, usize)> = Vec::new();
    for &rank in &ranks {
        if let Some(entry) = counts.iter_mut().find(|(r, _)| *r == rank) {
            entry.1 += 1;
        } else {
            counts.push((rank, 1));
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by count descending

    // Royal flush: A-K-Q-J-10 of same suit
    if is_flush && ranks == [Rank::Ten, Rank::Jack, Rank::Queen, Rank::King, Rank::Ace] {
        return HandRank::RoyalFlush;
    }

    // Straight flush
    if is_flush && (is_straight || is_ace_low_straight) {
        return HandRank::StraightFlush;
    }

    // Four of a kind
    if counts[0].1 == 4 {
        return HandRank::FourOfAKind;
    }

    // Full house: 3 + 2
    if counts[0].1 == 3 && counts.len() > 1 && counts[1].1 == 2 {
        return HandRank::FullHouse;
    }

    // Flush
    if is_flush {
        return HandRank::Flush;
    }

    // Straight
    if is_straight || is_ace_low_straight {
        return HandRank::Straight;
    }

    // Three of a kind
    if counts[0].1 == 3 {
        return HandRank::ThreeOfAKind;
    }

    // Two pair
    if counts[0].1 == 2 && counts.len() > 1 && counts[1].1 == 2 {
        return HandRank::TwoPair;
    }

    // Jacks or better (pair of J, Q, K, or A)
    if counts[0].1 == 2 && counts[0].0 >= Rank::Jack {
        return HandRank::JacksOrBetter;
    }

    HandRank::HighCard
}

fn is_sequential(ranks: &[Rank]) -> bool {
    if ranks.len() != 5 {
        return false;
    }
    for i in 1..5 {
        if ranks[i] as u8 != ranks[i - 1] as u8 + 1 {
            return false;
        }
    }
    true
}

// =============================================================================
// GAME STATE
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PokerView {
    #[default]
    Menu,
    Betting,
    FirstDeal,
    HoldSelect,
    Draw,
    Result,
}

#[derive(Debug, Clone)]
pub struct PokerState {
    pub view: PokerView,
    pub deck: Deck,
    pub hand: [Card; 5],
    pub held: [bool; 5],
    pub current_bet: i64,
    pub available_credits: i64,
    pub last_win: i64,
    pub hand_rank: Option<HandRank>,
    pub message: Option<String>,
    pub hands_played: u32,
    pub total_won: i64,
    pub total_bet: i64,
    pub selected_card: usize,
    pub tick_count: u32,
    pub game_over: bool,
    pub pending_events: Vec<GameEvent>,
}

impl Default for PokerState {
    fn default() -> Self {
        Self::new()
    }
}

impl PokerState {
    pub fn new() -> Self {
        let mut deck = Deck::new();
        deck.shuffle();
        let hand = [
            Card::new(Rank::Ace, Suit::Spades),
            Card::new(Rank::King, Suit::Spades),
            Card::new(Rank::Queen, Suit::Spades),
            Card::new(Rank::Jack, Suit::Spades),
            Card::new(Rank::Ten, Suit::Spades),
        ];

        Self {
            view: PokerView::Menu,
            deck,
            hand,
            held: [false; 5],
            current_bet: STARTING_BET,
            available_credits: 0,
            last_win: 0,
            hand_rank: None,
            message: None,
            hands_played: 0,
            total_won: 0,
            total_bet: 0,
            selected_card: 0,
            tick_count: 0,
            game_over: false,
            pending_events: Vec::new(),
        }
    }

    pub fn set_credits(&mut self, credits: i64) {
        self.available_credits = credits;
        self.current_bet = self.current_bet.min(credits).max(MIN_BET);
    }

    fn deal(&mut self) {
        if self.current_bet > self.available_credits {
            self.message = Some("Not enough credits!".to_string());
            return;
        }

        // Deduct bet
        self.available_credits -= self.current_bet;
        self.total_bet += self.current_bet;
        self.hands_played += 1;
        self.last_win = 0;
        self.hand_rank = None;
        self.message = None;
        self.held = [false; 5];
        self.selected_card = 0;

        // New deck and deal 5 cards
        self.deck = Deck::new();
        self.deck.shuffle();

        for i in 0..5 {
            if let Some(card) = self.deck.draw() {
                self.hand[i] = card;
            }
        }

        self.view = PokerView::HoldSelect;
    }

    fn draw_cards(&mut self) {
        // Replace non-held cards
        for i in 0..5 {
            if !self.held[i] {
                if let Some(card) = self.deck.draw() {
                    self.hand[i] = card;
                }
            }
        }

        // Evaluate hand
        self.hand_rank = Some(evaluate_hand(&self.hand));
        let rank = self.hand_rank.unwrap();
        let payout = rank.payout() * self.current_bet;

        self.last_win = payout;
        self.available_credits += payout;
        self.total_won += payout;

        if payout > 0 {
            self.message = Some(format!("{}! Win: ${}", rank.name(), payout));
        } else {
            self.message = Some("No win. Try again!".to_string());
        }

        self.view = PokerView::Result;
    }

    fn toggle_hold(&mut self, idx: usize) {
        if idx < 5 {
            self.held[idx] = !self.held[idx];
        }
    }

    fn adjust_bet(&mut self, delta: i64) {
        self.current_bet =
            (self.current_bet + delta).clamp(MIN_BET, MAX_BET.min(self.available_credits));
    }
}

// =============================================================================
// GAME ENGINE
// =============================================================================

impl GameEngine for PokerState {
    fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match self.view {
            PokerView::Menu => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if self.available_credits >= MIN_BET {
                        self.view = PokerView::Betting;
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
            PokerView::Betting => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.deal();
                    KeyHandleResult::Handled
                }
                KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                    self.adjust_bet(5);
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.adjust_bet(-5);
                    KeyHandleResult::Handled
                }
                KeyCode::Char('m') | KeyCode::Char('M') => {
                    self.current_bet = MAX_BET.min(self.available_credits);
                    KeyHandleResult::Handled
                }
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                    self.view = PokerView::Menu;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            PokerView::HoldSelect => match key.code {
                KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') => {
                    if self.selected_card > 0 {
                        self.selected_card -= 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('D') => {
                    if self.selected_card < 4 {
                        self.selected_card += 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Up | KeyCode::Down | KeyCode::Char(' ') => {
                    self.toggle_hold(self.selected_card);
                    KeyHandleResult::Handled
                }
                KeyCode::Char('1') => {
                    self.toggle_hold(0);
                    KeyHandleResult::Handled
                }
                KeyCode::Char('2') => {
                    self.toggle_hold(1);
                    KeyHandleResult::Handled
                }
                KeyCode::Char('3') => {
                    self.toggle_hold(2);
                    KeyHandleResult::Handled
                }
                KeyCode::Char('4') => {
                    self.toggle_hold(3);
                    KeyHandleResult::Handled
                }
                KeyCode::Char('5') => {
                    self.toggle_hold(4);
                    KeyHandleResult::Handled
                }
                KeyCode::Enter => {
                    self.draw_cards();
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => {
                    // Can't cancel mid-hand
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            PokerView::FirstDeal | PokerView::Draw => KeyHandleResult::Handled,
            PokerView::Result => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if self.available_credits >= MIN_BET {
                        self.view = PokerView::Betting;
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
