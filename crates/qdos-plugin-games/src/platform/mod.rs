//! Game Platform - Core services for the R-DOS Games plugin
//!
//! Provides unified interfaces, statistics tracking, and achievements
//! across all games.

pub mod achievements;
pub mod engine;
pub mod events;
pub mod stats;

pub use achievements::{AchievementManager, AchievementState};
pub use engine::{GameEngine, KeyHandleResult};
pub use events::GameEvent;
pub use stats::{PlayerStats, StatsTracker};
