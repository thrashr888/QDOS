//! Games plugin state

use super::adventure::AdventureState;
use super::artillery::ArtilleryState;
use super::biolab::BiolabState;
use super::blackjack::BlackjackState;
use super::brainiac::BrainiacState;
use super::breakout::BreakoutState;
use super::caverns::CavernsState;
use super::clicker::ClickerState;
use super::dopewars::DopeWarsState;
use super::dungeon::DungeonState;
use super::gumshoe::GumshoeState;
use super::junglerun::JungleRunState;
use super::micropolis::MicropolisState;
use super::mindgames::MindgamesState;
use super::minesweeper::MinesweeperState;
use super::neondrive::NeondriveState;
use super::rogue::RogueState;
use super::roulette::RouletteState;
use super::snake::SnakeState;
use super::storyweaver::StoryweaverState;
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
    #[serde(default)]
    pub brainiac: GameLeaderboard,
    #[serde(default)]
    pub storyweaver: GameLeaderboard,
    #[serde(default)]
    pub dopewars: GameLeaderboard,
    #[serde(default)]
    pub minesweeper: GameLeaderboard,
    #[serde(default)]
    pub artillery: GameLeaderboard,
    #[serde(default)]
    pub mindgames: GameLeaderboard,
    #[serde(default)]
    pub gumshoe: GameLeaderboard,
    #[serde(default)]
    pub dungeon: GameLeaderboard,
    #[serde(default)]
    pub caverns: GameLeaderboard,
    #[serde(default)]
    pub biolab: GameLeaderboard,
    #[serde(default)]
    pub neondrive: GameLeaderboard,
    #[serde(default)]
    pub micropolis: GameLeaderboard,
    #[serde(default)]
    pub junglerun: GameLeaderboard,
    #[serde(default)]
    pub adventure: GameLeaderboard,
    #[serde(default)]
    pub blackjack: GameLeaderboard,
    #[serde(default)]
    pub roulette: GameLeaderboard,
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
            GameType::Brainiac => &self.brainiac,
            GameType::Storyweaver => &self.storyweaver,
            GameType::DopeWars => &self.dopewars,
            GameType::Minesweeper => &self.minesweeper,
            GameType::Artillery => &self.artillery,
            GameType::Mindgames => &self.mindgames,
            GameType::Gumshoe => &self.gumshoe,
            GameType::Dungeon => &self.dungeon,
            GameType::Caverns => &self.caverns,
            GameType::Biolab => &self.biolab,
            GameType::Neondrive => &self.neondrive,
            GameType::Micropolis => &self.micropolis,
            GameType::JungleRun => &self.junglerun,
            GameType::Adventure => &self.adventure,
            GameType::Blackjack => &self.blackjack,
            GameType::Roulette => &self.roulette,
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
            GameType::Brainiac => &mut self.brainiac,
            GameType::Storyweaver => &mut self.storyweaver,
            GameType::DopeWars => &mut self.dopewars,
            GameType::Minesweeper => &mut self.minesweeper,
            GameType::Artillery => &mut self.artillery,
            GameType::Mindgames => &mut self.mindgames,
            GameType::Gumshoe => &mut self.gumshoe,
            GameType::Dungeon => &mut self.dungeon,
            GameType::Caverns => &mut self.caverns,
            GameType::Biolab => &mut self.biolab,
            GameType::Neondrive => &mut self.neondrive,
            GameType::Micropolis => &mut self.micropolis,
            GameType::JungleRun => &mut self.junglerun,
            GameType::Adventure => &mut self.adventure,
            GameType::Blackjack => &mut self.blackjack,
            GameType::Roulette => &mut self.roulette,
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
    Brainiac,
    Storyweaver,
    DopeWars,
    Minesweeper,
    Artillery,
    Mindgames,
    Gumshoe,
    Dungeon,
    Caverns,
    Biolab,
    Neondrive,
    Micropolis,
    JungleRun,
    Adventure,
    Blackjack,
    Roulette,
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
            GameType::Brainiac,
            GameType::Storyweaver,
            GameType::DopeWars,
            GameType::Minesweeper,
            GameType::Artillery,
            GameType::Mindgames,
            GameType::Gumshoe,
            GameType::Dungeon,
            GameType::Caverns,
            GameType::Biolab,
            GameType::Neondrive,
            GameType::Micropolis,
            GameType::JungleRun,
            GameType::Adventure,
            GameType::Blackjack,
            GameType::Roulette,
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
            GameType::Brainiac => "Brainiac",
            GameType::Storyweaver => "Storyweaver",
            GameType::DopeWars => "Dope Wars",
            GameType::Minesweeper => "Minesweeper",
            GameType::Artillery => "Artillery",
            GameType::Mindgames => "Mindgames",
            GameType::Gumshoe => "Gumshoe",
            GameType::Dungeon => "Dungeon",
            GameType::Caverns => "Caverns",
            GameType::Biolab => "Biolab",
            GameType::Neondrive => "Neon Drive",
            GameType::Micropolis => "Micropolis",
            GameType::JungleRun => "Jungle Run",
            GameType::Adventure => "Adventure",
            GameType::Blackjack => "Blackjack",
            GameType::Roulette => "Roulette",
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
            GameType::Brainiac => "AI trivia - test your knowledge!",
            GameType::Storyweaver => "AI choose-your-own-adventure books",
            GameType::DopeWars => "Buy low, sell high, pay off your debt",
            GameType::Minesweeper => "Find all mines without triggering them",
            GameType::Artillery => "Tank battle with physics and explosions",
            GameType::Mindgames => "Brain training - patterns, memory, math",
            GameType::Gumshoe => "Chase criminals across the globe!",
            GameType::Dungeon => "Explore dark mazes, fight monsters",
            GameType::Caverns => "Text adventure - explore, solve puzzles",
            GameType::Biolab => "Learn biology - cells, DNA, anatomy",
            GameType::Neondrive => "Outrun-style cyberpunk racing",
            GameType::Micropolis => "Build your real estate empire!",
            GameType::JungleRun => "Pitfall-style jungle platformer",
            GameType::Adventure => "Dragon Quest - find the chalice!",
            GameType::Blackjack => "Beat the dealer to 21!",
            GameType::Roulette => "Spin the wheel, place your bets!",
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
    Stats,            // Viewing player statistics
    Achievements,     // Viewing achievements
}

/// Main games plugin state
pub struct GamesState {
    pub view: GamesView,
    pub selected_game: usize,
    pub menu_scroll_offset: usize, // For scrolling the games menu
    pub achievements_scroll_offset: usize, // For scrolling the achievements list
    pub current_game: Option<GameType>,
    pub score: u32,
    pub high_scores: [u32; 8], // One for each game (legacy, kept for compatibility)
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
    pub brainiac: BrainiacState,
    pub storyweaver: StoryweaverState,
    pub dopewars: DopeWarsState,
    pub minesweeper: MinesweeperState,
    pub artillery: ArtilleryState,
    pub mindgames: MindgamesState,
    pub gumshoe: GumshoeState,
    pub dungeon: DungeonState,
    pub caverns: CavernsState,
    pub biolab: BiolabState,
    pub neondrive: NeondriveState,
    pub micropolis: MicropolisState,
    pub junglerun: JungleRunState,
    pub adventure: AdventureState,
    pub blackjack: BlackjackState,
    pub roulette: RouletteState,

    // Casino wallet - shared credits for gambling games
    pub casino_credits: i64,
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
            menu_scroll_offset: 0,
            achievements_scroll_offset: 0,
            current_game: None,
            score: 0,
            high_scores: [0; 8],
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
            brainiac: BrainiacState::new(),
            storyweaver: StoryweaverState::new(),
            dopewars: DopeWarsState::new(),
            minesweeper: MinesweeperState::new(),
            artillery: ArtilleryState::new(),
            mindgames: MindgamesState::new(),
            gumshoe: GumshoeState::new(),
            dungeon: DungeonState::new(),
            caverns: CavernsState::new(),
            biolab: BiolabState::new(),
            neondrive: NeondriveState::new(),
            micropolis: MicropolisState::new(),
            junglerun: JungleRunState::new(),
            adventure: AdventureState::new(),
            blackjack: BlackjackState::new(),
            roulette: RouletteState::new(),
            casino_credits: 1000, // Starting casino credits
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
        self.adjust_scroll();
    }

    pub fn select_prev(&mut self) {
        let count = GameType::all().len();
        self.selected_game = (self.selected_game + count - 1) % count;
        self.adjust_scroll();
    }

    /// Adjust scroll offset to keep selected game visible
    /// Visible window shows ~6 games (each takes 2 rows, we have ~14 rows available)
    fn adjust_scroll(&mut self) {
        const MAX_VISIBLE_GAMES: usize = 6;

        // Scroll down if selection is below visible window
        if self.selected_game >= self.menu_scroll_offset + MAX_VISIBLE_GAMES {
            self.menu_scroll_offset = self.selected_game - MAX_VISIBLE_GAMES + 1;
        }

        // Scroll up if selection is above visible window
        if self.selected_game < self.menu_scroll_offset {
            self.menu_scroll_offset = self.selected_game;
        }
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
            GameType::Brainiac => self.brainiac.reset(),
            GameType::Storyweaver => self.storyweaver.reset(),
            GameType::DopeWars => self.dopewars.reset(),
            GameType::Minesweeper => self.minesweeper = MinesweeperState::new(),
            GameType::Artillery => self.artillery = ArtilleryState::new(),
            GameType::Mindgames => self.mindgames.reset(),
            GameType::Gumshoe => self.gumshoe = GumshoeState::new(),
            GameType::Dungeon => self.dungeon = DungeonState::new(),
            GameType::Caverns => self.caverns = CavernsState::new(),
            GameType::Biolab => self.biolab = BiolabState::new(),
            GameType::Neondrive => self.neondrive = NeondriveState::new(),
            GameType::Micropolis => self.micropolis.reset(),
            GameType::JungleRun => self.junglerun = JungleRunState::new(),
            GameType::Adventure => self.adventure = AdventureState::new(),
            GameType::Blackjack => {
                self.blackjack = BlackjackState::new();
                self.blackjack.set_credits(self.casino_credits);
            }
            GameType::Roulette => {
                self.roulette = RouletteState::new();
                self.roulette.set_credits(self.casino_credits);
            }
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
                GameType::Brainiac => 6,
                GameType::Storyweaver => 7,
                GameType::DopeWars => 8,    // No legacy high score storage
                GameType::Minesweeper => 9, // No legacy high score storage
                GameType::Artillery => 10,  // No legacy high score storage
                GameType::Mindgames => 11,  // No legacy high score storage
                GameType::Gumshoe => 12,    // No legacy high score storage
                GameType::Dungeon => 13,    // No legacy high score storage
                GameType::Caverns => 14,    // No legacy high score storage
                GameType::Biolab => 15,     // No legacy high score storage
                GameType::Neondrive => 16,  // No legacy high score storage
                GameType::Micropolis => 17, // No legacy high score storage
                GameType::JungleRun => 18,  // No legacy high score storage
                GameType::Adventure => 19,  // No legacy high score storage
                GameType::Blackjack => 20,  // Casino game
                GameType::Roulette => 21,   // Casino game
            };

            // Update casino credits for gambling games
            match game {
                GameType::Blackjack => {
                    self.casino_credits = self.blackjack.available_credits;
                }
                GameType::Roulette => {
                    self.casino_credits = self.roulette.available_credits;
                }
                _ => {}
            }

            if idx < 8 && self.score > self.high_scores[idx] {
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

    /// Show player statistics
    pub fn show_stats(&mut self) {
        self.view = GamesView::Stats;
    }

    pub fn show_achievements(&mut self) {
        self.achievements_scroll_offset = 0;
        self.view = GamesView::Achievements;
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
