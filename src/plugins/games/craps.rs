//! CRAPS - Casino dice game
//!
//! Classic casino craps with pass/don't pass and odds bets.

use super::platform::{GameEngine, GameEvent, KeyHandleResult};
use crossterm::event::{KeyCode, KeyEvent};
use rand::prelude::*;

// =============================================================================
// CONSTANTS
// =============================================================================

const STARTING_BET: i64 = 10;
const MIN_BET: i64 = 5;
const MAX_BET: i64 = 500;
const ROLL_DURATION: u32 = 30; // Ticks for dice animation

// =============================================================================
// BET TYPES
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BetType {
    #[default]
    Pass, // Come-out: 7/11 win, 2/3/12 lose. Point: hit point wins, 7 loses
    DontPass, // Opposite of pass (12 is push)
    Field,    // One-roll: 2,3,4,9,10,11,12 win (2 and 12 pay double)
    Any7,     // One-roll: 7 pays 4:1
    AnyCraps, // One-roll: 2,3,12 pays 7:1
}

impl BetType {
    pub fn name(&self) -> &'static str {
        match self {
            BetType::Pass => "Pass Line",
            BetType::DontPass => "Don't Pass",
            BetType::Field => "Field",
            BetType::Any7 => "Any 7",
            BetType::AnyCraps => "Any Craps",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            BetType::Pass => "7/11 win, 2/3/12 lose on come-out",
            BetType::DontPass => "2/3 win, 7/11 lose on come-out",
            BetType::Field => "2,3,4,9,10,11,12 win (2,12 = 2x)",
            BetType::Any7 => "7 wins, pays 4:1",
            BetType::AnyCraps => "2,3,12 wins, pays 7:1",
        }
    }

    pub fn all() -> &'static [BetType] {
        &[
            BetType::Pass,
            BetType::DontPass,
            BetType::Field,
            BetType::Any7,
            BetType::AnyCraps,
        ]
    }
}

// =============================================================================
// GAME STATE
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CrapsView {
    #[default]
    Menu,
    Betting,
    Rolling,
    Result,
    PointPhase, // After point is established
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GamePhase {
    ComeOut, // Initial roll
    Point,   // Point established, rolling for point or 7
}

#[derive(Debug, Clone)]
pub struct CrapsState {
    pub view: CrapsView,
    pub phase: GamePhase,
    pub dice: [u8; 2],      // Current dice values
    pub rolling_ticks: u32, // Animation countdown
    pub point: Option<u8>,  // Established point (4,5,6,8,9,10)
    pub bet_type: BetType,
    pub current_bet: i64,
    pub available_credits: i64,
    pub last_win: i64,
    pub message: Option<String>,
    pub rolls: u32,
    pub total_won: i64,
    pub total_bet: i64,
    pub sevens: u32,
    pub points_made: u32,
    pub tick_count: u32,
    pub game_over: bool,
    pub pending_events: Vec<GameEvent>,
}

impl Default for CrapsState {
    fn default() -> Self {
        Self::new()
    }
}

impl CrapsState {
    pub fn new() -> Self {
        Self {
            view: CrapsView::Menu,
            phase: GamePhase::ComeOut,
            dice: [1, 1],
            rolling_ticks: 0,
            point: None,
            bet_type: BetType::Pass,
            current_bet: STARTING_BET,
            available_credits: 0,
            last_win: 0,
            message: None,
            rolls: 0,
            total_won: 0,
            total_bet: 0,
            sevens: 0,
            points_made: 0,
            tick_count: 0,
            game_over: false,
            pending_events: Vec::new(),
        }
    }

    pub fn set_credits(&mut self, credits: i64) {
        self.available_credits = credits;
        self.current_bet = self.current_bet.min(credits).max(MIN_BET);
    }

    pub fn dice_total(&self) -> u8 {
        self.dice[0] + self.dice[1]
    }

    fn start_roll(&mut self) {
        if self.current_bet > self.available_credits {
            self.message = Some("Not enough credits!".to_string());
            return;
        }

        // Deduct bet
        self.available_credits -= self.current_bet;
        self.total_bet += self.current_bet;
        self.rolls += 1;
        self.last_win = 0;
        self.message = None;

        // Start rolling animation
        self.rolling_ticks = ROLL_DURATION;
        self.view = CrapsView::Rolling;
    }

    fn update_rolling(&mut self) {
        if self.rolling_ticks > 0 {
            self.rolling_ticks -= 1;

            // Random dice while rolling
            let mut rng = rand::thread_rng();
            self.dice[0] = rng.gen_range(1..=6);
            self.dice[1] = rng.gen_range(1..=6);

            if self.rolling_ticks == 0 {
                self.resolve_roll();
            }
        }
    }

    fn resolve_roll(&mut self) {
        let total = self.dice_total();

        match self.bet_type {
            BetType::Pass => self.resolve_pass(total),
            BetType::DontPass => self.resolve_dont_pass(total),
            BetType::Field => self.resolve_field(total),
            BetType::Any7 => self.resolve_any7(total),
            BetType::AnyCraps => self.resolve_any_craps(total),
        }
    }

    fn resolve_pass(&mut self, total: u8) {
        match self.phase {
            GamePhase::ComeOut => {
                match total {
                    7 | 11 => {
                        // Natural - win!
                        self.last_win = self.current_bet * 2;
                        self.available_credits += self.last_win;
                        self.total_won += self.current_bet;
                        if total == 7 {
                            self.sevens += 1;
                        }
                        self.message = Some(format!(
                            "{}! NATURAL - You WIN ${}",
                            total, self.current_bet
                        ));
                        self.view = CrapsView::Result;
                    }
                    2 | 3 | 12 => {
                        // Craps - lose
                        self.message = Some(format!("{}! Craps - You lose", total));
                        self.view = CrapsView::Result;
                    }
                    4 | 5 | 6 | 8 | 9 | 10 => {
                        // Point established
                        self.point = Some(total);
                        self.phase = GamePhase::Point;
                        self.message = Some(format!("Point is {}! Roll again...", total));
                        // Return bet for now
                        self.available_credits += self.current_bet;
                        self.total_bet -= self.current_bet;
                        self.view = CrapsView::PointPhase;
                    }
                    _ => {}
                }
            }
            GamePhase::Point => {
                if let Some(point) = self.point {
                    if total == point {
                        // Made the point!
                        self.last_win = self.current_bet * 2;
                        self.available_credits += self.last_win;
                        self.total_won += self.current_bet;
                        self.points_made += 1;
                        self.message = Some(format!(
                            "{}! Made the point - WIN ${}",
                            total, self.current_bet
                        ));
                        self.point = None;
                        self.phase = GamePhase::ComeOut;
                        self.view = CrapsView::Result;
                    } else if total == 7 {
                        // Seven out - lose
                        self.sevens += 1;
                        self.message = Some("Seven Out! You lose".to_string());
                        self.point = None;
                        self.phase = GamePhase::ComeOut;
                        self.view = CrapsView::Result;
                    } else {
                        // Keep rolling
                        self.available_credits += self.current_bet;
                        self.total_bet -= self.current_bet;
                        self.message =
                            Some(format!("{}. Point is still {}. Roll again!", total, point));
                        self.view = CrapsView::PointPhase;
                    }
                }
            }
        }
    }

    fn resolve_dont_pass(&mut self, total: u8) {
        match self.phase {
            GamePhase::ComeOut => {
                match total {
                    2 | 3 => {
                        // Win on craps (not 12)
                        self.last_win = self.current_bet * 2;
                        self.available_credits += self.last_win;
                        self.total_won += self.current_bet;
                        self.message =
                            Some(format!("{}! Craps - You WIN ${}", total, self.current_bet));
                        self.view = CrapsView::Result;
                    }
                    12 => {
                        // Push
                        self.available_credits += self.current_bet;
                        self.total_bet -= self.current_bet;
                        self.message = Some("12! Push - bet returned".to_string());
                        self.view = CrapsView::Result;
                    }
                    7 | 11 => {
                        // Lose on natural
                        if total == 7 {
                            self.sevens += 1;
                        }
                        self.message = Some(format!("{}! Natural - You lose", total));
                        self.view = CrapsView::Result;
                    }
                    4 | 5 | 6 | 8 | 9 | 10 => {
                        // Point established
                        self.point = Some(total);
                        self.phase = GamePhase::Point;
                        self.message = Some(format!("Point is {}. Need 7 to win!", total));
                        self.available_credits += self.current_bet;
                        self.total_bet -= self.current_bet;
                        self.view = CrapsView::PointPhase;
                    }
                    _ => {}
                }
            }
            GamePhase::Point => {
                if let Some(point) = self.point {
                    if total == 7 {
                        // Seven out - win!
                        self.last_win = self.current_bet * 2;
                        self.available_credits += self.last_win;
                        self.total_won += self.current_bet;
                        self.sevens += 1;
                        self.message = Some(format!("7! Seven out - WIN ${}", self.current_bet));
                        self.point = None;
                        self.phase = GamePhase::ComeOut;
                        self.view = CrapsView::Result;
                    } else if total == point {
                        // Made the point - lose
                        self.message = Some(format!("{}! Point made - You lose", total));
                        self.point = None;
                        self.phase = GamePhase::ComeOut;
                        self.view = CrapsView::Result;
                    } else {
                        // Keep rolling
                        self.available_credits += self.current_bet;
                        self.total_bet -= self.current_bet;
                        self.message = Some(format!("{}. Need 7. Point is {}.", total, point));
                        self.view = CrapsView::PointPhase;
                    }
                }
            }
        }
    }

    fn resolve_field(&mut self, total: u8) {
        match total {
            2 | 12 => {
                // Double payout
                self.last_win = self.current_bet * 3;
                self.available_credits += self.last_win;
                self.total_won += self.current_bet * 2;
                self.message = Some(format!(
                    "{}! Field DOUBLE - WIN ${}",
                    total,
                    self.current_bet * 2
                ));
            }
            3 | 4 | 9 | 10 | 11 => {
                // Normal field win
                self.last_win = self.current_bet * 2;
                self.available_credits += self.last_win;
                self.total_won += self.current_bet;
                self.message = Some(format!("{}! Field - WIN ${}", total, self.current_bet));
            }
            _ => {
                // Lose
                if total == 7 {
                    self.sevens += 1;
                }
                self.message = Some(format!("{}. Field loses", total));
            }
        }
        self.view = CrapsView::Result;
    }

    fn resolve_any7(&mut self, total: u8) {
        if total == 7 {
            self.last_win = self.current_bet * 5; // 4:1 payout
            self.available_credits += self.last_win;
            self.total_won += self.current_bet * 4;
            self.sevens += 1;
            self.message = Some(format!("SEVEN! WIN ${}", self.current_bet * 4));
        } else {
            self.message = Some(format!("{}. No seven", total));
        }
        self.view = CrapsView::Result;
    }

    fn resolve_any_craps(&mut self, total: u8) {
        if total == 2 || total == 3 || total == 12 {
            self.last_win = self.current_bet * 8; // 7:1 payout
            self.available_credits += self.last_win;
            self.total_won += self.current_bet * 7;
            self.message = Some(format!("{}! Craps - WIN ${}", total, self.current_bet * 7));
        } else {
            if total == 7 {
                self.sevens += 1;
            }
            self.message = Some(format!("{}. No craps", total));
        }
        self.view = CrapsView::Result;
    }

    fn adjust_bet(&mut self, delta: i64) {
        self.current_bet =
            (self.current_bet + delta).clamp(MIN_BET, MAX_BET.min(self.available_credits));
    }

    fn cycle_bet_type(&mut self, forward: bool) {
        let types = BetType::all();
        let current_idx = types.iter().position(|&t| t == self.bet_type).unwrap_or(0);
        let new_idx = if forward {
            (current_idx + 1) % types.len()
        } else {
            (current_idx + types.len() - 1) % types.len()
        };
        self.bet_type = types[new_idx];
    }
}

// =============================================================================
// GAME ENGINE
// =============================================================================

impl GameEngine for CrapsState {
    fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);

        if self.view == CrapsView::Rolling {
            self.update_rolling();
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match self.view {
            CrapsView::Menu => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if self.available_credits >= MIN_BET {
                        self.view = CrapsView::Betting;
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
            CrapsView::Betting => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.start_roll();
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
                KeyCode::Esc => {
                    self.view = CrapsView::Menu;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            CrapsView::PointPhase => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.start_roll();
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => {
                    // Abandon point phase
                    self.point = None;
                    self.phase = GamePhase::ComeOut;
                    self.view = CrapsView::Betting;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            CrapsView::Rolling => KeyHandleResult::Handled,
            CrapsView::Result => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if self.available_credits >= MIN_BET {
                        self.view = CrapsView::Betting;
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
