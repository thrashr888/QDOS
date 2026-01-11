//! Games plugin state

use super::breakout::BreakoutState;
use super::clicker::ClickerState;
use super::rogue::RogueState;
use super::snake::SnakeState;
use super::tetris::TetrisState;
use super::trek::TrekState;
use serde::{Deserialize, Serialize};

/// Maximum entries per game leaderboard
const MAX_LEADERBOARD_ENTRIES: usize = 10;

/// A single leaderboard entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub initials: String, // 3 characters, e.g., "AAA"
    pub score: u32,
}

impl LeaderboardEntry {
    pub fn new(initials: &str, score: u32) -> Self {
        // Ensure exactly 3 uppercase characters
        let initials = initials
            .chars()
            .filter(|c| c.is_ascii_alphabetic())
            .take(3)
            .collect::<String>()
            .to_uppercase();
        let initials = if initials.len() < 3 {
            format!("{:A<3}", initials) // Pad with 'A' if needed
        } else {
            initials
        };
        Self { initials, score }
    }
}

/// Leaderboard for a single game
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameLeaderboard {
    pub entries: Vec<LeaderboardEntry>,
}

impl GameLeaderboard {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Check if a score qualifies for the leaderboard
    pub fn is_high_score(&self, score: u32) -> bool {
        if score == 0 {
            return false;
        }
        if self.entries.len() < MAX_LEADERBOARD_ENTRIES {
            return true;
        }
        self.entries.last().map(|e| score > e.score).unwrap_or(true)
    }

    /// Add a score to the leaderboard (maintains sorted order, top 10)
    pub fn add_score(&mut self, initials: &str, score: u32) {
        let entry = LeaderboardEntry::new(initials, score);

        // Find insertion point (sorted high to low)
        let pos = self
            .entries
            .iter()
            .position(|e| score > e.score)
            .unwrap_or(self.entries.len());

        self.entries.insert(pos, entry);

        // Keep only top 10
        if self.entries.len() > MAX_LEADERBOARD_ENTRIES {
            self.entries.truncate(MAX_LEADERBOARD_ENTRIES);
        }
    }

    /// Get the top score, if any
    pub fn top_score(&self) -> Option<u32> {
        self.entries.first().map(|e| e.score)
    }
}

/// All game leaderboards (persisted)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Leaderboards {
    pub tetris: GameLeaderboard,
    pub snake: GameLeaderboard,
    pub breakout: GameLeaderboard,
    pub rogue: GameLeaderboard,
    pub trek: GameLeaderboard,
    pub clicker: GameLeaderboard,
}

impl Leaderboards {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get leaderboard for a specific game
    pub fn get(&self, game: GameType) -> &GameLeaderboard {
        match game {
            GameType::Tetris => &self.tetris,
            GameType::Snake => &self.snake,
            GameType::Breakout => &self.breakout,
            GameType::Rogue => &self.rogue,
            GameType::Trek => &self.trek,
            GameType::Clicker => &self.clicker,
        }
    }

    /// Get mutable leaderboard for a specific game
    pub fn get_mut(&mut self, game: GameType) -> &mut GameLeaderboard {
        match game {
            GameType::Tetris => &mut self.tetris,
            GameType::Snake => &mut self.snake,
            GameType::Breakout => &mut self.breakout,
            GameType::Rogue => &mut self.rogue,
            GameType::Trek => &mut self.trek,
            GameType::Clicker => &mut self.clicker,
        }
    }
}

/// Available games
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameType {
    Tetris,
    Snake,
    Breakout,
    Rogue,
    Trek,
    Clicker,
}

impl GameType {
    pub fn all() -> &'static [GameType] {
        &[
            GameType::Tetris,
            GameType::Snake,
            GameType::Breakout,
            GameType::Rogue,
            GameType::Trek,
            GameType::Clicker,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            GameType::Tetris => "Tetris",
            GameType::Snake => "Snake",
            GameType::Breakout => "Breakout",
            GameType::Rogue => "Rogue",
            GameType::Trek => "Star Trek",
            GameType::Clicker => "Clicker",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            GameType::Tetris => "Stack falling blocks to clear lines",
            GameType::Snake => "Eat food and grow without hitting yourself",
            GameType::Breakout => "Bounce the ball to break all the bricks",
            GameType::Rogue => "Explore the dungeon and defeat monsters",
            GameType::Trek => "Command the Enterprise, destroy Klingons",
            GameType::Clicker => "Kill monsters, gain gold, buy upgrades",
        }
    }
}

/// Current view state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GamesView {
    #[default]
    Menu,
    Playing,
    Paused,
    GameOver,
    EnteringInitials, // High score - enter 3-letter initials
    Leaderboard,      // Viewing the leaderboard
}

/// Main games plugin state
pub struct GamesState {
    pub view: GamesView,
    pub selected_game: usize,
    pub current_game: Option<GameType>,
    pub score: u32,
    pub high_scores: [u32; 6], // One for each game (legacy, kept for compatibility)
    pub menu_tick: u32,        // Animation tick for menu effects

    // Leaderboard system
    pub leaderboards: Leaderboards,
    pub initials_buffer: String, // Current initials being entered (0-3 chars)
    pub initials_cursor: usize,  // Which character is being edited (0-2)
    pub pending_score: Option<u32>, // Score waiting to be added to leaderboard
    pub leaderboard_game: Option<GameType>, // Which game's leaderboard to show

    // Individual game states
    pub tetris: TetrisState,
    pub snake: SnakeState,
    pub breakout: BreakoutState,
    pub rogue: RogueState,
    pub trek: TrekState,
    pub clicker: ClickerState,
}

impl Default for GamesState {
    fn default() -> Self {
        Self::new()
    }
}

impl GamesState {
    pub fn new() -> Self {
        Self {
            view: GamesView::Menu,
            selected_game: 0,
            current_game: None,
            score: 0,
            high_scores: [0; 6],
            menu_tick: 0,
            leaderboards: Leaderboards::new(),
            initials_buffer: String::new(),
            initials_cursor: 0,
            pending_score: None,
            leaderboard_game: None,
            tetris: TetrisState::new(),
            snake: SnakeState::new(),
            breakout: BreakoutState::new(),
            rogue: RogueState::new(),
            trek: TrekState::new(),
            clicker: ClickerState::new(),
        }
    }

    /// Load leaderboards from saved data
    pub fn load_leaderboards(&mut self, leaderboards: Leaderboards) {
        self.leaderboards = leaderboards;
    }

    /// Increment menu animation tick
    pub fn tick_menu(&mut self) {
        self.menu_tick = self.menu_tick.wrapping_add(1);
    }

    pub fn selected_game_type(&self) -> GameType {
        GameType::all()[self.selected_game]
    }

    pub fn select_next(&mut self) {
        let count = GameType::all().len();
        self.selected_game = (self.selected_game + 1) % count;
    }

    pub fn select_prev(&mut self) {
        let count = GameType::all().len();
        self.selected_game = (self.selected_game + count - 1) % count;
    }

    pub fn start_game(&mut self) {
        let game_type = self.selected_game_type();
        self.current_game = Some(game_type);
        self.score = 0;
        self.view = GamesView::Playing;

        // Reset game state
        match game_type {
            GameType::Tetris => self.tetris.reset(),
            GameType::Snake => self.snake.reset(),
            GameType::Breakout => self.breakout.reset(),
            GameType::Rogue => self.rogue.reset(),
            GameType::Trek => self.trek.reset(),
            GameType::Clicker => self.clicker.reset(),
        }
    }

    pub fn toggle_pause(&mut self) {
        match self.view {
            GamesView::Playing => self.view = GamesView::Paused,
            GamesView::Paused => self.view = GamesView::Playing,
            _ => {}
        }
    }

    pub fn game_over(&mut self) {
        // Update legacy high score
        if let Some(game) = self.current_game {
            let idx = match game {
                GameType::Tetris => 0,
                GameType::Snake => 1,
                GameType::Breakout => 2,
                GameType::Rogue => 3,
                GameType::Trek => 4,
                GameType::Clicker => 5,
            };
            if self.score > self.high_scores[idx] {
                self.high_scores[idx] = self.score;
            }

            // Check if this is a leaderboard-worthy score
            if self.leaderboards.get(game).is_high_score(self.score) {
                // Enter initials mode
                self.pending_score = Some(self.score);
                self.initials_buffer = "AAA".to_string();
                self.initials_cursor = 0;
                self.view = GamesView::EnteringInitials;
                return;
            }
        }

        self.view = GamesView::GameOver;
    }

    /// Handle initials entry - add a character
    pub fn initials_next_char(&mut self) {
        if self.initials_cursor < 3 {
            let chars: Vec<char> = self.initials_buffer.chars().collect();
            let current = chars.get(self.initials_cursor).unwrap_or(&'A');
            let next = if *current == 'Z' {
                'A'
            } else {
                ((*current as u8) + 1) as char
            };
            let mut new_buffer = String::new();
            for (i, c) in chars.iter().enumerate() {
                if i == self.initials_cursor {
                    new_buffer.push(next);
                } else {
                    new_buffer.push(*c);
                }
            }
            self.initials_buffer = new_buffer;
        }
    }

    /// Handle initials entry - previous character
    pub fn initials_prev_char(&mut self) {
        if self.initials_cursor < 3 {
            let chars: Vec<char> = self.initials_buffer.chars().collect();
            let current = chars.get(self.initials_cursor).unwrap_or(&'A');
            let prev = if *current == 'A' {
                'Z'
            } else {
                ((*current as u8) - 1) as char
            };
            let mut new_buffer = String::new();
            for (i, c) in chars.iter().enumerate() {
                if i == self.initials_cursor {
                    new_buffer.push(prev);
                } else {
                    new_buffer.push(*c);
                }
            }
            self.initials_buffer = new_buffer;
        }
    }

    /// Move cursor left in initials entry
    pub fn initials_cursor_left(&mut self) {
        if self.initials_cursor > 0 {
            self.initials_cursor -= 1;
        }
    }

    /// Move cursor right in initials entry
    pub fn initials_cursor_right(&mut self) {
        if self.initials_cursor < 2 {
            self.initials_cursor += 1;
        }
    }

    /// Confirm initials entry and add to leaderboard
    pub fn confirm_initials(&mut self) {
        if let (Some(game), Some(score)) = (self.current_game, self.pending_score) {
            self.leaderboards
                .get_mut(game)
                .add_score(&self.initials_buffer, score);
            self.pending_score = None;
        }
        self.view = GamesView::GameOver;
    }

    /// Cancel initials entry (don't add to leaderboard)
    pub fn cancel_initials(&mut self) {
        self.pending_score = None;
        self.view = GamesView::GameOver;
    }

    /// Show leaderboard for current or selected game
    pub fn show_leaderboard(&mut self) {
        self.leaderboard_game = self.current_game.or(Some(self.selected_game_type()));
        self.view = GamesView::Leaderboard;
    }

    /// Close leaderboard
    pub fn close_leaderboard(&mut self) {
        if self.current_game.is_some() {
            self.view = GamesView::GameOver;
        } else {
            self.view = GamesView::Menu;
        }
    }

    /// Cycle to next game in leaderboard view
    pub fn next_leaderboard_game(&mut self) {
        let current = self.leaderboard_game.unwrap_or(GameType::Tetris);
        let all = GameType::all();
        let idx = all.iter().position(|g| *g == current).unwrap_or(0);
        let next_idx = (idx + 1) % all.len();
        self.leaderboard_game = Some(all[next_idx]);
    }

    /// Cycle to previous game in leaderboard view
    pub fn prev_leaderboard_game(&mut self) {
        let current = self.leaderboard_game.unwrap_or(GameType::Tetris);
        let all = GameType::all();
        let idx = all.iter().position(|g| *g == current).unwrap_or(0);
        let prev_idx = (idx + all.len() - 1) % all.len();
        self.leaderboard_game = Some(all[prev_idx]);
    }

    pub fn return_to_menu(&mut self) {
        self.view = GamesView::Menu;
        self.current_game = None;
    }
}
