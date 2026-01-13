//! GameEngine trait - Unified interface for all games
//!
//! All games implement this trait to provide consistent behavior
//! for the platform's tick, input, and event systems.

use super::GameEvent;
use crossterm::event::KeyEvent;

/// Result of handling a key input in a game
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyHandleResult {
    /// Input was processed successfully
    Handled,
    /// Input was not recognized by this game
    NotHandled,
    /// Game ended as a result of this input
    GameOver,
    /// Game requests to be paused
    RequestPause,
    /// Game requests to quit to menu
    RequestQuit,
}

/// Core trait that all games must implement
///
/// This trait provides a unified interface for the game platform to:
/// - Process game ticks (~10Hz)
/// - Handle keyboard input
/// - Track scores and game state
/// - Collect events for stats/achievements
pub trait GameEngine {
    // === Required Methods ===

    /// Process one game tick (called ~10Hz for most games)
    ///
    /// This is where game logic like movement, physics, timers,
    /// and AI updates should happen.
    fn tick(&mut self);

    /// Handle keyboard input
    ///
    /// Returns a `KeyHandleResult` indicating how the input was processed.
    /// Games should handle their specific controls here and return
    /// `RequestPause` for 'P', `RequestQuit` for Esc, etc.
    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult;

    /// Get the current score for leaderboard/stats tracking
    fn get_score(&self) -> u32;

    /// Check if the game is over (either won or lost)
    fn is_game_over(&self) -> bool;

    // === Optional Methods (with sensible defaults) ===

    /// Check if the player won the game
    ///
    /// Returns `false` by default for endless games like Tetris/Snake.
    /// Games with win conditions (Breakout, Rogue, Trek) should override.
    fn is_game_won(&self) -> bool {
        false
    }

    /// Get the current level/floor/chapter
    ///
    /// Returns `None` by default for level-less games.
    /// Games with progression should override.
    fn get_level(&self) -> Option<u32> {
        None
    }

    /// Collect and clear pending events for platform processing
    ///
    /// The platform calls this after tick() and handle_key() to
    /// collect events for stats tracking and achievement checking.
    fn drain_events(&mut self) -> Vec<GameEvent> {
        Vec::new()
    }

    /// Get a game-specific statistic by key
    ///
    /// Used for achievement conditions and detailed stats tracking.
    /// Keys are game-specific, e.g., "lines_cleared" for Tetris.
    fn get_stat(&self, _key: &str) -> Option<u64> {
        None
    }
}
