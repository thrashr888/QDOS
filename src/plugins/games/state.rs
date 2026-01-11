//! Games plugin state

use super::breakout::BreakoutState;
use super::rogue::RogueState;
use super::snake::SnakeState;
use super::tetris::TetrisState;
use super::trek::TrekState;

/// Available games
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameType {
    Tetris,
    Snake,
    Breakout,
    Rogue,
    Trek,
}

impl GameType {
    pub fn all() -> &'static [GameType] {
        &[
            GameType::Tetris,
            GameType::Snake,
            GameType::Breakout,
            GameType::Rogue,
            GameType::Trek,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            GameType::Tetris => "Tetris",
            GameType::Snake => "Snake",
            GameType::Breakout => "Breakout",
            GameType::Rogue => "Rogue",
            GameType::Trek => "Star Trek",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            GameType::Tetris => "Stack falling blocks to clear lines",
            GameType::Snake => "Eat food and grow without hitting yourself",
            GameType::Breakout => "Bounce the ball to break all the bricks",
            GameType::Rogue => "Explore the dungeon and defeat monsters",
            GameType::Trek => "Command the Enterprise, destroy Klingons",
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
}

/// Main games plugin state
pub struct GamesState {
    pub view: GamesView,
    pub selected_game: usize,
    pub current_game: Option<GameType>,
    pub score: u32,
    pub high_scores: [u32; 5], // One for each game

    // Individual game states
    pub tetris: TetrisState,
    pub snake: SnakeState,
    pub breakout: BreakoutState,
    pub rogue: RogueState,
    pub trek: TrekState,
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
            high_scores: [0; 5],
            tetris: TetrisState::new(),
            snake: SnakeState::new(),
            breakout: BreakoutState::new(),
            rogue: RogueState::new(),
            trek: TrekState::new(),
        }
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
        self.view = GamesView::GameOver;

        // Update high score
        if let Some(game) = self.current_game {
            let idx = match game {
                GameType::Tetris => 0,
                GameType::Snake => 1,
                GameType::Breakout => 2,
                GameType::Rogue => 3,
                GameType::Trek => 4,
            };
            if self.score > self.high_scores[idx] {
                self.high_scores[idx] = self.score;
            }
        }
    }

    pub fn return_to_menu(&mut self) {
        self.view = GamesView::Menu;
        self.current_game = None;
    }
}
