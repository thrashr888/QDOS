//! Game Events - Events emitted by games for platform processing
//!
//! Games emit these events during tick() and handle_key() to notify
//! the platform of significant occurrences for stats and achievements.

/// Events that games can emit for platform processing
#[derive(Debug, Clone, PartialEq)]
pub enum GameEvent {
    // === Lifecycle Events ===
    /// Game session started
    GameStarted,

    /// Game session ended
    GameEnded {
        /// Whether the player won
        won: bool,
    },

    // === Progress Events ===
    /// Score changed during gameplay
    ScoreChanged {
        /// Previous score
        old: u32,
        /// New score
        new: u32,
    },

    /// Player reached a new level/floor/chapter
    LevelReached(u32),

    // === Game-Specific Events ===
    // Tetris
    /// Lines cleared in Tetris (1-4)
    LinesCleared(u32),

    // Snake
    /// Food eaten in Snake
    FoodEaten,

    // Breakout
    /// Brick destroyed in Breakout
    BrickDestroyed,

    // Rogue
    /// Floor reached in Rogue
    FloorReached(u32),

    /// Enemy defeated (Rogue, Clicker)
    EnemyDefeated {
        /// Type of enemy
        enemy_type: String,
    },

    // Trek
    /// Klingon destroyed in Trek
    KlingonDestroyed,

    // Brainiac
    /// Question answered in Brainiac
    QuestionAnswered {
        /// Whether the answer was correct
        correct: bool,
    },

    // Storyweaver
    /// Chapter completed in Storyweaver
    ChapterCompleted,

    /// Story completed in Storyweaver
    StoryCompleted,

    // Clicker
    /// Gold earned in Clicker
    GoldEarned(u64),

    /// Prestige performed in Clicker
    Prestiged,

    // === Custom Events ===
    /// Custom event for game-specific tracking
    Custom {
        /// Event key/name
        key: String,
        /// Event value
        value: u64,
    },
}
