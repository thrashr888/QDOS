//! Game Platform - Core services for the R-DOS Games plugin
//!
//! Provides unified interfaces, statistics tracking, and achievements
//! across all games.

pub mod engine;
pub mod events;

pub use engine::{GameEngine, KeyHandleResult};
pub use events::GameEvent;
