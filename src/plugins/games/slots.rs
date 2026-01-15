//! SLOTS - Casino slot machine game
//!
//! Classic 3-reel slot machine with fruit symbols and jackpot!

use super::platform::{GameEngine, GameEvent, KeyHandleResult};
use crossterm::event::{KeyCode, KeyEvent};
use rand::prelude::*;

// =============================================================================
// CONSTANTS
// =============================================================================

const STARTING_BET: i64 = 10;
const MIN_BET: i64 = 5;
const MAX_BET: i64 = 100;
const SPIN_DURATION: u32 = 45; // Ticks for reels to spin

// =============================================================================
// SYMBOLS
// =============================================================================

/// Slot machine symbols
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Symbol {
    Cherry,
    Lemon,
    Orange,
    Plum,
    Bell,
    Bar,
    Seven,
    Diamond,
}

impl Symbol {
    pub fn char(&self) -> char {
        match self {
            Symbol::Cherry => 'C',
            Symbol::Lemon => 'L',
            Symbol::Orange => 'O',
            Symbol::Plum => 'P',
            Symbol::Bell => 'B',
            Symbol::Bar => '=',
            Symbol::Seven => '7',
            Symbol::Diamond => '#',
        }
    }

    pub fn ascii(&self) -> &'static str {
        match self {
            Symbol::Cherry => "CHR",
            Symbol::Lemon => "LMN",
            Symbol::Orange => "ORG",
            Symbol::Plum => "PLM",
            Symbol::Bell => "BEL",
            Symbol::Bar => "BAR",
            Symbol::Seven => " 7 ",
            Symbol::Diamond => "DIA",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Symbol::Cherry => "Cherry",
            Symbol::Lemon => "Lemon",
            Symbol::Orange => "Orange",
            Symbol::Plum => "Plum",
            Symbol::Bell => "Bell",
            Symbol::Bar => "BAR",
            Symbol::Seven => "Lucky 7",
            Symbol::Diamond => "Diamond",
        }
    }

    /// Weight for random selection (lower = rarer)
    pub fn weight(&self) -> u32 {
        match self {
            Symbol::Cherry => 20,
            Symbol::Lemon => 18,
            Symbol::Orange => 16,
            Symbol::Plum => 14,
            Symbol::Bell => 10,
            Symbol::Bar => 6,
            Symbol::Seven => 3,
            Symbol::Diamond => 1,
        }
    }

    /// All symbols in order
    pub fn all() -> &'static [Symbol] {
        &[
            Symbol::Cherry,
            Symbol::Lemon,
            Symbol::Orange,
            Symbol::Plum,
            Symbol::Bell,
            Symbol::Bar,
            Symbol::Seven,
            Symbol::Diamond,
        ]
    }

    /// Get payout multiplier for 3 of a kind
    pub fn payout(&self) -> i64 {
        match self {
            Symbol::Cherry => 5,
            Symbol::Lemon => 8,
            Symbol::Orange => 10,
            Symbol::Plum => 15,
            Symbol::Bell => 25,
            Symbol::Bar => 50,
            Symbol::Seven => 100,
            Symbol::Diamond => 500, // JACKPOT!
        }
    }
}

// =============================================================================
// GAME STATE
// =============================================================================

/// Current phase of the game
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SlotsView {
    #[default]
    Menu,
    Betting,
    Spinning,
    Result,
}

/// Slot machine state
#[derive(Debug, Clone)]
pub struct SlotsState {
    pub view: SlotsView,
    pub reels: [Symbol; 3],       // Current reel symbols
    pub spinning_reels: [u32; 3], // Remaining spin ticks per reel
    pub current_bet: i64,
    pub available_credits: i64,
    pub last_win: i64,
    pub message: Option<String>,
    pub spin_count: u32,
    pub total_won: i64,
    pub total_bet: i64,
    pub jackpots_hit: u32,
    pub tick_count: u32,
    pub game_over: bool,
    pub pending_events: Vec<GameEvent>,
}

impl Default for SlotsState {
    fn default() -> Self {
        Self::new()
    }
}

impl SlotsState {
    pub fn new() -> Self {
        Self {
            view: SlotsView::Menu,
            reels: [Symbol::Seven, Symbol::Seven, Symbol::Seven],
            spinning_reels: [0, 0, 0],
            current_bet: STARTING_BET,
            available_credits: 0,
            last_win: 0,
            message: None,
            spin_count: 0,
            total_won: 0,
            total_bet: 0,
            jackpots_hit: 0,
            tick_count: 0,
            game_over: false,
            pending_events: Vec::new(),
        }
    }

    pub fn set_credits(&mut self, credits: i64) {
        self.available_credits = credits;
        self.current_bet = self.current_bet.min(credits).max(MIN_BET);
    }

    fn random_symbol() -> Symbol {
        let mut rng = rand::thread_rng();
        let total_weight: u32 = Symbol::all().iter().map(|s| s.weight()).sum();
        let mut roll = rng.gen_range(0..total_weight);

        for symbol in Symbol::all() {
            if roll < symbol.weight() {
                return *symbol;
            }
            roll -= symbol.weight();
        }
        Symbol::Cherry
    }

    fn start_spin(&mut self) {
        if self.current_bet > self.available_credits {
            self.message = Some("Not enough credits!".to_string());
            return;
        }

        // Deduct bet
        self.available_credits -= self.current_bet;
        self.total_bet += self.current_bet;
        self.spin_count += 1;
        self.last_win = 0;
        self.message = None;

        // Start all reels spinning with staggered stop times
        self.spinning_reels = [SPIN_DURATION, SPIN_DURATION + 10, SPIN_DURATION + 20];
        self.view = SlotsView::Spinning;
    }

    fn update_spinning(&mut self) {
        let mut all_stopped = true;

        for i in 0..3 {
            if self.spinning_reels[i] > 0 {
                self.spinning_reels[i] -= 1;
                // Randomize symbol while spinning
                if self.spinning_reels[i] > 0 {
                    self.reels[i] = Self::random_symbol();
                } else {
                    // Final symbol
                    self.reels[i] = Self::random_symbol();
                }
                all_stopped = false;
            }
        }

        if all_stopped {
            self.calculate_win();
        }
    }

    fn calculate_win(&mut self) {
        let mut win = 0i64;

        // Three of a kind
        if self.reels[0] == self.reels[1] && self.reels[1] == self.reels[2] {
            win = self.current_bet * self.reels[0].payout();
            if self.reels[0] == Symbol::Diamond {
                self.message = Some("*** JACKPOT!!! ***".to_string());
                self.jackpots_hit += 1;
            } else if self.reels[0] == Symbol::Seven {
                self.message = Some("LUCKY SEVENS!".to_string());
            } else {
                self.message = Some(format!("Three {}s!", self.reels[0].name()));
            }
        }
        // Two cherries (partial win)
        else if self.reels.iter().filter(|&&s| s == Symbol::Cherry).count() == 2 {
            win = self.current_bet * 2;
            self.message = Some("Two Cherries!".to_string());
        }
        // Any cherry
        else if self.reels.contains(&Symbol::Cherry) {
            win = self.current_bet;
            self.message = Some("Cherry!".to_string());
        }
        // No win
        else {
            self.message = Some("No win. Try again!".to_string());
        }

        self.last_win = win;
        self.available_credits += win;
        self.total_won += win;
        self.view = SlotsView::Result;
    }

    fn adjust_bet(&mut self, delta: i64) {
        self.current_bet =
            (self.current_bet + delta).clamp(MIN_BET, MAX_BET.min(self.available_credits));
    }
}

// =============================================================================
// GAME ENGINE
// =============================================================================

impl GameEngine for SlotsState {
    fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);

        if self.view == SlotsView::Spinning {
            self.update_spinning();
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match self.view {
            SlotsView::Menu => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if self.available_credits >= MIN_BET {
                        self.view = SlotsView::Betting;
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
            SlotsView::Betting => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.start_spin();
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
                KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') => {
                    self.adjust_bet(-10);
                    KeyHandleResult::Handled
                }
                KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('D') => {
                    self.adjust_bet(10);
                    KeyHandleResult::Handled
                }
                KeyCode::Char('m') | KeyCode::Char('M') => {
                    // Max bet
                    self.current_bet = MAX_BET.min(self.available_credits);
                    KeyHandleResult::Handled
                }
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                    self.view = SlotsView::Menu;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            SlotsView::Spinning => {
                // No input during spin
                KeyHandleResult::Handled
            }
            SlotsView::Result => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if self.available_credits >= MIN_BET {
                        self.view = SlotsView::Betting;
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
