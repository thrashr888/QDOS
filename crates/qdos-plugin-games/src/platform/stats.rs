//! Stats Tracking - Player statistics across all games
//!
//! Tracks lifetime and per-game statistics with persistent storage.
//! Stats are updated on game start/end and saved to config.

use super::GameEvent;
use crate::state::GameType;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

/// Global player statistics across all games
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlayerStats {
    /// Total playtime across all games in seconds
    pub total_playtime_secs: u64,

    /// Total number of games started
    pub total_games_played: u32,

    /// Total number of games won
    pub total_games_won: u32,

    /// First time any game was played
    pub first_played: Option<DateTime<Utc>>,

    /// Per-game statistics (keyed by game name)
    #[serde(default)]
    pub games: HashMap<String, GameStats>,
}

/// Statistics for a single game type
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GameStats {
    /// Number of times this game was played
    pub times_played: u32,

    /// Number of times the player won
    pub times_won: u32,

    /// Total time spent playing this game in seconds
    pub total_playtime_secs: u64,

    /// Highest score achieved
    pub high_score: u32,

    /// Best level/floor reached (if applicable)
    pub best_level: Option<u32>,

    /// Last time this game was played
    pub last_played: Option<DateTime<Utc>>,

    /// Game-specific counters (flexible per-game tracking)
    #[serde(default)]
    pub counters: HashMap<String, u64>,
}

/// Manages stats tracking during gameplay
pub struct StatsTracker {
    /// Current session start time
    session_start: Option<Instant>,

    /// Current game session start time
    game_start: Option<Instant>,

    /// Currently playing game type
    current_game: Option<GameType>,

    /// Player stats (loaded from config)
    pub stats: PlayerStats,

    /// Whether stats have been modified and need saving
    dirty: bool,
}

impl StatsTracker {
    /// Create a new stats tracker with loaded stats
    pub fn new(stats: PlayerStats) -> Self {
        Self {
            session_start: Some(Instant::now()),
            game_start: None,
            current_game: None,
            stats,
            dirty: false,
        }
    }

    /// Create a new stats tracker with empty stats
    pub fn empty() -> Self {
        Self::new(PlayerStats::default())
    }

    /// Get current session duration in seconds
    pub fn session_duration_secs(&self) -> u64 {
        self.session_start
            .map(|start| start.elapsed().as_secs())
            .unwrap_or(0)
    }

    /// Called when a game session starts
    pub fn on_game_start(&mut self, game: GameType) {
        let now = Utc::now();

        // Track first ever play
        if self.stats.first_played.is_none() {
            self.stats.first_played = Some(now);
        }

        // Start tracking game session
        self.game_start = Some(Instant::now());
        self.current_game = Some(game);

        // Increment play counts
        self.stats.total_games_played += 1;

        let game_stats = self.get_or_create_game_stats(game);
        game_stats.times_played += 1;
        game_stats.last_played = Some(now);

        self.dirty = true;
    }

    /// Called when a game session ends
    pub fn on_game_end(&mut self, game: GameType, score: u32, won: bool, level: Option<u32>) {
        // Calculate session playtime
        if let Some(start) = self.game_start.take() {
            let duration_secs = start.elapsed().as_secs();

            // Update global playtime
            self.stats.total_playtime_secs += duration_secs;

            // Update game-specific playtime
            let game_stats = self.get_or_create_game_stats(game);
            game_stats.total_playtime_secs += duration_secs;
        }

        // Track win
        if won {
            self.stats.total_games_won += 1;
            let game_stats = self.get_or_create_game_stats(game);
            game_stats.times_won += 1;
        }

        // Update high score
        let game_stats = self.get_or_create_game_stats(game);
        if score > game_stats.high_score {
            game_stats.high_score = score;
        }

        // Update best level
        if let Some(lvl) = level {
            match game_stats.best_level {
                Some(best) if lvl > best => game_stats.best_level = Some(lvl),
                None => game_stats.best_level = Some(lvl),
                _ => {}
            }
        }

        self.current_game = None;
        self.dirty = true;
    }

    /// Process game events to update counters
    pub fn process_event(&mut self, game: GameType, event: &GameEvent) {
        let game_stats = self.get_or_create_game_stats(game);

        match event {
            // Tetris events
            GameEvent::LinesCleared(lines) => {
                *game_stats
                    .counters
                    .entry("lines_cleared".to_string())
                    .or_insert(0) += *lines as u64;
                if *lines == 4 {
                    *game_stats
                        .counters
                        .entry("tetrises".to_string())
                        .or_insert(0) += 1;
                }
            }

            // Snake events
            GameEvent::FoodEaten => {
                *game_stats
                    .counters
                    .entry("food_eaten".to_string())
                    .or_insert(0) += 1;
            }

            // Breakout events
            GameEvent::BrickDestroyed => {
                *game_stats
                    .counters
                    .entry("bricks_destroyed".to_string())
                    .or_insert(0) += 1;
            }

            // Rogue events
            GameEvent::FloorReached(floor) => {
                let current = game_stats
                    .counters
                    .get("floors_reached")
                    .copied()
                    .unwrap_or(0);
                if *floor as u64 > current {
                    game_stats
                        .counters
                        .insert("floors_reached".to_string(), *floor as u64);
                }
            }
            GameEvent::EnemyDefeated { .. } => {
                *game_stats
                    .counters
                    .entry("enemies_killed".to_string())
                    .or_insert(0) += 1;
            }

            // Trek events
            GameEvent::KlingonDestroyed => {
                *game_stats
                    .counters
                    .entry("klingons_destroyed".to_string())
                    .or_insert(0) += 1;
            }

            // Clicker events
            GameEvent::GoldEarned(gold) => {
                *game_stats
                    .counters
                    .entry("total_gold".to_string())
                    .or_insert(0) += gold;
            }
            GameEvent::Prestiged => {
                *game_stats
                    .counters
                    .entry("prestiges".to_string())
                    .or_insert(0) += 1;
            }

            // Brainiac/Mindgames events
            GameEvent::QuestionAnswered { correct } => {
                *game_stats
                    .counters
                    .entry("questions_answered".to_string())
                    .or_insert(0) += 1;
                if *correct {
                    *game_stats
                        .counters
                        .entry("correct_answers".to_string())
                        .or_insert(0) += 1;
                }
            }

            // Storyweaver events
            GameEvent::ChapterCompleted => {
                *game_stats
                    .counters
                    .entry("chapters_read".to_string())
                    .or_insert(0) += 1;
            }
            GameEvent::StoryCompleted => {
                *game_stats
                    .counters
                    .entry("stories_completed".to_string())
                    .or_insert(0) += 1;
            }

            // Level events (update best_level)
            GameEvent::LevelReached(level) => match game_stats.best_level {
                Some(best) if *level > best => game_stats.best_level = Some(*level),
                None => game_stats.best_level = Some(*level),
                _ => {}
            },

            // Custom events
            GameEvent::Custom { key, value } => {
                *game_stats.counters.entry(key.clone()).or_insert(0) += value;
            }

            // COSMOS alien contact
            GameEvent::AlienContact { species } => {
                let counter_key = format!("alien_contact_{}", species);
                *game_stats.counters.entry(counter_key).or_insert(0) += 1;
            }

            // Lifecycle and score events don't update counters
            GameEvent::GameStarted
            | GameEvent::GameEnded { .. }
            | GameEvent::ScoreChanged { .. } => {}
        }

        self.dirty = true;
    }

    /// Get or create game stats for a specific game
    fn get_or_create_game_stats(&mut self, game: GameType) -> &mut GameStats {
        let key = game.name().to_string();
        self.stats.games.entry(key).or_default()
    }

    /// Get stats for a specific game (read-only)
    pub fn get_game_stats(&self, game: GameType) -> Option<&GameStats> {
        self.stats.games.get(game.name())
    }

    /// Check if stats have been modified
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark stats as saved
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Get the most recently played game
    pub fn last_played_game(&self) -> Option<(GameType, DateTime<Utc>)> {
        let mut most_recent: Option<(GameType, DateTime<Utc>)> = None;

        for game_type in GameType::all() {
            if let Some(stats) = self.stats.games.get(game_type.name()) {
                if let Some(last) = stats.last_played {
                    match most_recent {
                        Some((_, ref time)) if last > *time => {
                            most_recent = Some((*game_type, last));
                        }
                        None => {
                            most_recent = Some((*game_type, last));
                        }
                        _ => {}
                    }
                }
            }
        }

        most_recent
    }

    /// Format playtime as human-readable string
    pub fn format_playtime(secs: u64) -> String {
        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m", secs / 60)
        } else {
            let hours = secs / 3600;
            let mins = (secs % 3600) / 60;
            if mins > 0 {
                format!("{}h {}m", hours, mins)
            } else {
                format!("{}h", hours)
            }
        }
    }

    /// Format "time ago" string
    pub fn format_time_ago(time: DateTime<Utc>) -> String {
        let now = Utc::now();
        let duration = now.signed_duration_since(time);

        if duration.num_seconds() < 60 {
            "just now".to_string()
        } else if duration.num_minutes() < 60 {
            let mins = duration.num_minutes();
            if mins == 1 {
                "1 minute ago".to_string()
            } else {
                format!("{} minutes ago", mins)
            }
        } else if duration.num_hours() < 24 {
            let hours = duration.num_hours();
            if hours == 1 {
                "1 hour ago".to_string()
            } else {
                format!("{} hours ago", hours)
            }
        } else if duration.num_days() < 7 {
            let days = duration.num_days();
            if days == 1 {
                "yesterday".to_string()
            } else {
                format!("{} days ago", days)
            }
        } else {
            time.format("%b %d, %Y").to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tracker() {
        let tracker = StatsTracker::empty();
        assert_eq!(tracker.stats.total_games_played, 0);
        assert!(!tracker.is_dirty());
    }

    #[test]
    fn test_game_start() {
        let mut tracker = StatsTracker::empty();
        tracker.on_game_start(GameType::Tetris);

        assert_eq!(tracker.stats.total_games_played, 1);
        assert!(tracker.stats.first_played.is_some());

        let game_stats = tracker.get_game_stats(GameType::Tetris).unwrap();
        assert_eq!(game_stats.times_played, 1);
        assert!(game_stats.last_played.is_some());
        assert!(tracker.is_dirty());
    }

    #[test]
    fn test_game_end_with_win() {
        let mut tracker = StatsTracker::empty();
        tracker.on_game_start(GameType::Breakout);
        tracker.on_game_end(GameType::Breakout, 1000, true, None);

        assert_eq!(tracker.stats.total_games_won, 1);

        let game_stats = tracker.get_game_stats(GameType::Breakout).unwrap();
        assert_eq!(game_stats.times_won, 1);
        assert_eq!(game_stats.high_score, 1000);
    }

    #[test]
    fn test_high_score_update() {
        let mut tracker = StatsTracker::empty();

        // First game with score 500
        tracker.on_game_start(GameType::Snake);
        tracker.on_game_end(GameType::Snake, 500, false, None);

        // Second game with higher score
        tracker.on_game_start(GameType::Snake);
        tracker.on_game_end(GameType::Snake, 800, false, None);

        // Third game with lower score
        tracker.on_game_start(GameType::Snake);
        tracker.on_game_end(GameType::Snake, 300, false, None);

        let game_stats = tracker.get_game_stats(GameType::Snake).unwrap();
        assert_eq!(game_stats.high_score, 800);
        assert_eq!(game_stats.times_played, 3);
    }

    #[test]
    fn test_format_playtime() {
        assert_eq!(StatsTracker::format_playtime(30), "30s");
        assert_eq!(StatsTracker::format_playtime(90), "1m");
        assert_eq!(StatsTracker::format_playtime(3600), "1h");
        assert_eq!(StatsTracker::format_playtime(3660), "1h 1m");
        assert_eq!(StatsTracker::format_playtime(7200), "2h");
    }

    #[test]
    fn test_process_lines_cleared_event() {
        let mut tracker = StatsTracker::empty();
        tracker.on_game_start(GameType::Tetris);

        tracker.process_event(GameType::Tetris, &GameEvent::LinesCleared(2));
        tracker.process_event(GameType::Tetris, &GameEvent::LinesCleared(4)); // Tetris!

        let game_stats = tracker.get_game_stats(GameType::Tetris).unwrap();
        assert_eq!(game_stats.counters.get("lines_cleared"), Some(&6));
        assert_eq!(game_stats.counters.get("tetrises"), Some(&1));
    }
}
