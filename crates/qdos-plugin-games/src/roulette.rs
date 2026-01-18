//! ROULETTE - Casino wheel game
//!
//! Place your bets and watch the wheel spin! European style (0-36).

use super::platform::{GameEngine, GameEvent, KeyHandleResult};
use crossterm::event::{KeyCode, KeyEvent};
use rand::prelude::*;

// =============================================================================
// CONSTANTS
// =============================================================================

const STARTING_BET: i64 = 10;
const MIN_BET: i64 = 5;
const MAX_BET: i64 = 500;
const SPIN_DURATION: u32 = 90; // Ticks for wheel to spin

// European roulette wheel order
const WHEEL_ORDER: [u8; 37] = [
    0, 32, 15, 19, 4, 21, 2, 25, 17, 34, 6, 27, 13, 36, 11, 30, 8, 23, 10, 5, 24, 16, 33, 1, 20,
    14, 31, 9, 22, 18, 29, 7, 28, 12, 35, 3, 26,
];

// Red numbers
const RED_NUMBERS: [u8; 18] = [
    1, 3, 5, 7, 9, 12, 14, 16, 18, 19, 21, 23, 25, 27, 30, 32, 34, 36,
];

// =============================================================================
// BET TYPES
// =============================================================================

/// Types of bets available
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BetType {
    // Inside bets
    Straight(u8), // Single number (0-36), pays 35:1

    // Outside bets
    Red,        // Red numbers, pays 1:1
    Black,      // Black numbers, pays 1:1
    Odd,        // Odd numbers, pays 1:1
    Even,       // Even numbers (not 0), pays 1:1
    Low,        // 1-18, pays 1:1
    High,       // 19-36, pays 1:1
    Dozen(u8),  // 1-12, 13-24, 25-36, pays 2:1
    Column(u8), // Columns 1, 2, 3, pays 2:1
}

impl BetType {
    pub fn name(&self) -> String {
        match self {
            BetType::Straight(n) => {
                if *n == 0 {
                    "0 (Green)".to_string()
                } else {
                    format!("{}", n)
                }
            }
            BetType::Red => "Red".to_string(),
            BetType::Black => "Black".to_string(),
            BetType::Odd => "Odd".to_string(),
            BetType::Even => "Even".to_string(),
            BetType::Low => "Low (1-18)".to_string(),
            BetType::High => "High (19-36)".to_string(),
            BetType::Dozen(d) => match d {
                1 => "1st 12 (1-12)".to_string(),
                2 => "2nd 12 (13-24)".to_string(),
                _ => "3rd 12 (25-36)".to_string(),
            },
            BetType::Column(c) => format!("Column {}", c),
        }
    }

    pub fn payout_ratio(&self) -> u8 {
        match self {
            BetType::Straight(_) => 35,
            BetType::Red | BetType::Black | BetType::Odd | BetType::Even => 1,
            BetType::Low | BetType::High => 1,
            BetType::Dozen(_) | BetType::Column(_) => 2,
        }
    }

    pub fn wins(&self, number: u8) -> bool {
        match self {
            BetType::Straight(n) => *n == number,
            BetType::Red => number > 0 && RED_NUMBERS.contains(&number),
            BetType::Black => number > 0 && !RED_NUMBERS.contains(&number),
            BetType::Odd => number > 0 && !number.is_multiple_of(2),
            BetType::Even => number > 0 && number.is_multiple_of(2),
            BetType::Low => (1..=18).contains(&number),
            BetType::High => (19..=36).contains(&number),
            BetType::Dozen(d) => match d {
                1 => (1..=12).contains(&number),
                2 => (13..=24).contains(&number),
                _ => (25..=36).contains(&number),
            },
            BetType::Column(c) => number > 0 && (number - 1) % 3 + 1 == *c,
        }
    }
}

/// A placed bet
#[derive(Debug, Clone)]
pub struct Bet {
    pub bet_type: BetType,
    pub amount: i64,
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

pub fn is_red(number: u8) -> bool {
    RED_NUMBERS.contains(&number)
}

pub fn is_black(number: u8) -> bool {
    number > 0 && !RED_NUMBERS.contains(&number)
}

pub fn get_color_name(number: u8) -> &'static str {
    if number == 0 {
        "Green"
    } else if is_red(number) {
        "Red"
    } else {
        "Black"
    }
}

// =============================================================================
// GAME STATE
// =============================================================================

/// Current phase of the game
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RouletteView {
    #[default]
    Menu,
    Betting,
    Spinning,
    Result,
}

/// Bet selection mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BetMode {
    #[default]
    Outside, // Red/Black, Odd/Even, etc.
    Number, // Single number selection
}

/// Main game state
pub struct RouletteState {
    pub view: RouletteView,
    pub bet_mode: BetMode,
    pub bets: Vec<Bet>,
    pub current_bet_amount: i64,
    pub available_credits: i64, // Casino wallet balance
    pub selected_bet_index: usize,
    pub selected_number: u8,
    pub winning_number: Option<u8>,
    pub spin_position: usize,
    pub spin_timer: u32,
    pub last_numbers: Vec<u8>,
    pub message: Option<String>,
    pub message_timer: u32,
    pub spins_played: u32,
    pub spins_won: u32,
    pub total_winnings: i64,
    pub biggest_win: i64,
    pub game_over: bool,
    pub tick_count: u32,
    events: Vec<GameEvent>,
}

impl Default for RouletteState {
    fn default() -> Self {
        Self::new()
    }
}

impl RouletteState {
    pub fn new() -> Self {
        Self {
            view: RouletteView::Menu,
            bet_mode: BetMode::Outside,
            bets: Vec::new(),
            current_bet_amount: STARTING_BET,
            available_credits: 0,
            selected_bet_index: 0,
            selected_number: 0,
            winning_number: None,
            spin_position: 0,
            spin_timer: 0,
            last_numbers: Vec::new(),
            message: None,
            message_timer: 0,
            spins_played: 0,
            spins_won: 0,
            total_winnings: 0,
            biggest_win: 0,
            game_over: false,
            tick_count: 0,
            events: Vec::new(),
        }
    }

    /// Set available credits (called by plugin before starting)
    pub fn set_credits(&mut self, credits: i64) {
        self.available_credits = credits;
        self.current_bet_amount = STARTING_BET.min(credits).max(MIN_BET);
    }

    /// Get net winnings for this session (for updating wallet)
    pub fn get_net_winnings(&self) -> i64 {
        self.total_winnings
    }

    pub fn start_game(&mut self) {
        self.view = RouletteView::Betting;
        self.bets.clear();
        self.current_bet_amount = self
            .current_bet_amount
            .min(self.available_credits)
            .max(MIN_BET);
        self.winning_number = None;
        self.game_over = false;
    }

    pub fn outside_bet_options() -> Vec<BetType> {
        vec![
            BetType::Red,
            BetType::Black,
            BetType::Odd,
            BetType::Even,
            BetType::Low,
            BetType::High,
            BetType::Dozen(1),
            BetType::Dozen(2),
            BetType::Dozen(3),
            BetType::Column(1),
            BetType::Column(2),
            BetType::Column(3),
        ]
    }

    pub fn place_bet(&mut self, bet_type: BetType) -> bool {
        let total_bet: i64 = self.bets.iter().map(|b| b.amount).sum();
        if total_bet + self.current_bet_amount > self.available_credits {
            self.show_message("Not enough credits!");
            return false;
        }

        // Check if bet already exists, add to it
        if let Some(existing) = self.bets.iter_mut().find(|b| b.bet_type == bet_type) {
            existing.amount += self.current_bet_amount;
        } else {
            self.bets.push(Bet {
                bet_type,
                amount: self.current_bet_amount,
            });
        }

        self.show_message(&format!(
            "Bet {} on {}",
            self.current_bet_amount,
            bet_type.name()
        ));
        true
    }

    pub fn clear_bets(&mut self) {
        self.bets.clear();
        self.show_message("Bets cleared");
    }

    pub fn spin(&mut self) {
        if self.bets.is_empty() {
            self.show_message("Place a bet first!");
            return;
        }

        // Deduct total bet from credits
        let total_bet: i64 = self.bets.iter().map(|b| b.amount).sum();
        self.available_credits -= total_bet;

        self.view = RouletteView::Spinning;
        self.spin_timer = SPIN_DURATION;

        // Pre-determine winning number
        let mut rng = rand::thread_rng();
        let idx = rng.gen_range(0..37);
        self.winning_number = Some(WHEEL_ORDER[idx]);
        self.spin_position = rng.gen_range(0..37);
    }

    pub fn resolve_spin(&mut self) {
        let old_score = self.get_score();
        let winning = self.winning_number.unwrap_or(0);

        // Add to history
        self.last_numbers.insert(0, winning);
        if self.last_numbers.len() > 10 {
            self.last_numbers.pop();
        }

        // Calculate winnings
        let mut total_win: i64 = 0;
        let total_bet: i64 = self.bets.iter().map(|b| b.amount).sum();

        for bet in &self.bets {
            if bet.bet_type.wins(winning) {
                let win = bet.amount * (bet.bet_type.payout_ratio() as i64 + 1);
                total_win += win;
            }
        }

        // Add winnings to credits
        self.available_credits += total_win;

        let net = total_win - total_bet;
        self.total_winnings += net;
        self.spins_played += 1;

        if net > 0 {
            self.spins_won += 1;
            if net > self.biggest_win {
                self.biggest_win = net;
            }
        }

        let new_score = self.get_score();
        self.events.push(GameEvent::ScoreChanged {
            old: old_score,
            new: new_score,
        });
        self.view = RouletteView::Result;
    }

    pub fn total_bet(&self) -> i64 {
        self.bets.iter().map(|b| b.amount).sum()
    }

    pub fn calculate_potential_win(&self, number: u8) -> i64 {
        let mut win: i64 = 0;
        for bet in &self.bets {
            if bet.bet_type.wins(number) {
                win += bet.amount * (bet.bet_type.payout_ratio() as i64 + 1);
            }
        }
        win
    }

    pub fn show_message(&mut self, msg: &str) {
        self.message = Some(msg.to_string());
        self.message_timer = 60;
    }

    pub fn adjust_bet(&mut self, delta: i64) {
        let max_bet = self.available_credits - self.total_bet();
        self.current_bet_amount =
            (self.current_bet_amount + delta).clamp(MIN_BET, MAX_BET.min(max_bet.max(MIN_BET)));
    }

    /// Get the wheel position display (5 numbers centered on current position)
    pub fn wheel_display(&self) -> Vec<u8> {
        let mut numbers = Vec::with_capacity(5);
        for i in 0..5 {
            let idx = (self.spin_position + i + 35) % 37; // -2 to +2 from center
            numbers.push(WHEEL_ORDER[idx]);
        }
        numbers
    }
}

// =============================================================================
// GAME ENGINE IMPLEMENTATION
// =============================================================================

impl GameEngine for RouletteState {
    fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);

        // Message timer
        if self.message_timer > 0 {
            self.message_timer -= 1;
            if self.message_timer == 0 {
                self.message = None;
            }
        }

        // Spinning animation
        if self.view == RouletteView::Spinning {
            if self.spin_timer > 0 {
                self.spin_timer -= 1;

                // Speed decreases as timer decreases
                let speed = if self.spin_timer > 60 {
                    2
                } else if self.spin_timer > 30 {
                    4
                } else {
                    8
                };

                if self.tick_count.is_multiple_of(speed) {
                    self.spin_position = (self.spin_position + 1) % 37;
                }
            } else {
                // Find the winning number's position
                if let Some(winning) = self.winning_number {
                    let target_pos = WHEEL_ORDER.iter().position(|&n| n == winning).unwrap_or(0);
                    self.spin_position = target_pos;
                }
                self.resolve_spin();
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match self.view {
            RouletteView::Menu => match key.code {
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

            RouletteView::Betting => match key.code {
                // Toggle bet mode
                KeyCode::Tab => {
                    self.bet_mode = match self.bet_mode {
                        BetMode::Outside => BetMode::Number,
                        BetMode::Number => BetMode::Outside,
                    };
                    self.selected_bet_index = 0;
                    KeyHandleResult::Handled
                }

                // Navigation
                KeyCode::Up | KeyCode::Char('k') => {
                    match self.bet_mode {
                        BetMode::Outside => {
                            let options = Self::outside_bet_options();
                            if self.selected_bet_index > 0 {
                                self.selected_bet_index -= 1;
                            } else {
                                self.selected_bet_index = options.len() - 1;
                            }
                        }
                        BetMode::Number => {
                            self.selected_number = (self.selected_number + 36) % 37;
                        }
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    match self.bet_mode {
                        BetMode::Outside => {
                            let options = Self::outside_bet_options();
                            self.selected_bet_index = (self.selected_bet_index + 1) % options.len();
                        }
                        BetMode::Number => {
                            self.selected_number = (self.selected_number + 1) % 37;
                        }
                    }
                    KeyHandleResult::Handled
                }

                // Bet amount adjustment
                KeyCode::Left | KeyCode::Char('h') => {
                    self.adjust_bet(-5);
                    KeyHandleResult::Handled
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    self.adjust_bet(5);
                    KeyHandleResult::Handled
                }

                // Place bet
                KeyCode::Enter => {
                    let bet_type = match self.bet_mode {
                        BetMode::Outside => Self::outside_bet_options()[self.selected_bet_index],
                        BetMode::Number => BetType::Straight(self.selected_number),
                    };
                    self.place_bet(bet_type);
                    KeyHandleResult::Handled
                }

                // Clear bets
                KeyCode::Char('c') => {
                    self.clear_bets();
                    KeyHandleResult::Handled
                }

                // Spin!
                KeyCode::Char(' ') | KeyCode::Char('s') => {
                    self.spin();
                    KeyHandleResult::Handled
                }

                KeyCode::Esc | KeyCode::Char('q') => {
                    if self.bets.is_empty() {
                        self.view = RouletteView::Menu;
                    } else {
                        self.show_message("Press C to clear bets first");
                    }
                    KeyHandleResult::Handled
                }

                _ => KeyHandleResult::NotHandled,
            },

            RouletteView::Spinning => {
                // No input during spin
                KeyHandleResult::Handled
            }

            RouletteView::Result => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.bets.clear();
                    if self.available_credits >= MIN_BET {
                        self.view = RouletteView::Betting;
                        self.winning_number = None;
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
