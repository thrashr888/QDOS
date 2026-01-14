//! Achievements System - Track player accomplishments
//!
//! Provides achievement definitions, condition checking, and unlock tracking.

use crate::plugins::games::state::GameType;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::stats::PlayerStats;

// =============================================================================
// ACHIEVEMENT CONDITION
// =============================================================================

/// Conditions that trigger achievement unlocks
#[derive(Debug, Clone)]
pub enum AchievementCondition {
    /// Player has played N total games
    GamesPlayed(u32),
    /// Player has won N total games
    GamesWon(u32),
    /// Player has played for N seconds total
    TotalPlaytime(u64),

    /// Player reached score N in a single game
    ScoreReached(GameType, u32),
    /// Player's high score is at least N
    HighScoreReached(GameType, u32),

    /// Player reached level N in a game
    LevelReached(GameType, u32),
    /// Player won the specified game at least once
    GameWon(GameType),

    /// Counter reached a threshold
    CounterReached {
        game: GameType,
        counter: &'static str,
        value: u64,
    },

    /// Player has won every game at least once
    AllGamesWon,
    /// Player has played every game at least once
    AllGamesPlayed,

    /// Custom condition checked via game state
    Custom(&'static str),
}

// =============================================================================
// ACHIEVEMENT DEFINITION
// =============================================================================

/// A single achievement definition
#[derive(Debug, Clone)]
pub struct Achievement {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub icon: char,
    pub game: Option<GameType>,
    pub hidden: bool,
    pub condition: AchievementCondition,
}

// =============================================================================
// ACHIEVEMENT REGISTRY
// =============================================================================

/// All achievements in the game
pub const ACHIEVEMENTS: &[Achievement] = &[
    // === GLOBAL ACHIEVEMENTS ===
    Achievement {
        id: "first_steps",
        name: "First Steps",
        description: "Play your first game",
        icon: '*',
        game: None,
        hidden: false,
        condition: AchievementCondition::GamesPlayed(1),
    },
    Achievement {
        id: "dedicated",
        name: "Dedicated",
        description: "Play for 1 hour total",
        icon: '+',
        game: None,
        hidden: false,
        condition: AchievementCondition::TotalPlaytime(3600),
    },
    Achievement {
        id: "veteran",
        name: "Veteran",
        description: "Play for 10 hours total",
        icon: '#',
        game: None,
        hidden: false,
        condition: AchievementCondition::TotalPlaytime(36000),
    },
    Achievement {
        id: "winner",
        name: "Winner",
        description: "Win any game",
        icon: '!',
        game: None,
        hidden: false,
        condition: AchievementCondition::GamesWon(1),
    },
    Achievement {
        id: "completionist",
        name: "Completionist",
        description: "Win every game at least once",
        icon: '@',
        game: None,
        hidden: false,
        condition: AchievementCondition::AllGamesWon,
    },
    Achievement {
        id: "explorer",
        name: "Explorer",
        description: "Play every game at least once",
        icon: '?',
        game: None,
        hidden: false,
        condition: AchievementCondition::AllGamesPlayed,
    },
    // === TETRIS ===
    Achievement {
        id: "tetris_first_line",
        name: "Line Clear",
        description: "Clear your first line in Tetris",
        icon: '-',
        game: Some(GameType::Tetris),
        hidden: false,
        condition: AchievementCondition::CounterReached {
            game: GameType::Tetris,
            counter: "lines_cleared",
            value: 1,
        },
    },
    Achievement {
        id: "tetris_100_lines",
        name: "Tetris Master",
        description: "Clear 100 lines total in Tetris",
        icon: '=',
        game: Some(GameType::Tetris),
        hidden: false,
        condition: AchievementCondition::CounterReached {
            game: GameType::Tetris,
            counter: "lines_cleared",
            value: 100,
        },
    },
    Achievement {
        id: "tetris_level_10",
        name: "Speed Demon",
        description: "Reach level 10 in Tetris",
        icon: '>',
        game: Some(GameType::Tetris),
        hidden: false,
        condition: AchievementCondition::LevelReached(GameType::Tetris, 10),
    },
    Achievement {
        id: "tetris_score_10k",
        name: "Block Party",
        description: "Score 10,000 points in Tetris",
        icon: '%',
        game: Some(GameType::Tetris),
        hidden: false,
        condition: AchievementCondition::ScoreReached(GameType::Tetris, 10000),
    },
    // === SNAKE ===
    Achievement {
        id: "snake_score_100",
        name: "Hungry Snake",
        description: "Score 100 points in Snake",
        icon: '~',
        game: Some(GameType::Snake),
        hidden: false,
        condition: AchievementCondition::ScoreReached(GameType::Snake, 100),
    },
    Achievement {
        id: "snake_score_500",
        name: "Snake Charmer",
        description: "Score 500 points in Snake",
        icon: 'S',
        game: Some(GameType::Snake),
        hidden: false,
        condition: AchievementCondition::ScoreReached(GameType::Snake, 500),
    },
    Achievement {
        id: "snake_score_1000",
        name: "Serpent King",
        description: "Score 1,000 points in Snake",
        icon: '$',
        game: Some(GameType::Snake),
        hidden: true,
        condition: AchievementCondition::ScoreReached(GameType::Snake, 1000),
    },
    // === BREAKOUT ===
    Achievement {
        id: "breakout_win",
        name: "Brick Breaker",
        description: "Clear all bricks in Breakout",
        icon: 'B',
        game: Some(GameType::Breakout),
        hidden: false,
        condition: AchievementCondition::GameWon(GameType::Breakout),
    },
    Achievement {
        id: "breakout_score_10k",
        name: "Demolition Expert",
        description: "Score 10,000 points in Breakout",
        icon: 'D',
        game: Some(GameType::Breakout),
        hidden: false,
        condition: AchievementCondition::ScoreReached(GameType::Breakout, 10000),
    },
    // === ROGUE ===
    Achievement {
        id: "rogue_floor_5",
        name: "Dungeon Crawler",
        description: "Reach floor 5 in Rogue",
        icon: 'v',
        game: Some(GameType::Rogue),
        hidden: false,
        condition: AchievementCondition::LevelReached(GameType::Rogue, 5),
    },
    Achievement {
        id: "rogue_win",
        name: "Rogue Champion",
        description: "Escape the dungeon (floor 10)",
        icon: '^',
        game: Some(GameType::Rogue),
        hidden: false,
        condition: AchievementCondition::GameWon(GameType::Rogue),
    },
    Achievement {
        id: "rogue_gold_1000",
        name: "Treasure Hunter",
        description: "Collect 1,000 gold total in Rogue",
        icon: 'G',
        game: Some(GameType::Rogue),
        hidden: false,
        condition: AchievementCondition::CounterReached {
            game: GameType::Rogue,
            counter: "gold_collected",
            value: 1000,
        },
    },
    // === TREK ===
    Achievement {
        id: "trek_win",
        name: "Final Frontier",
        description: "Destroy all Klingons",
        icon: 'K',
        game: Some(GameType::Trek),
        hidden: false,
        condition: AchievementCondition::GameWon(GameType::Trek),
    },
    Achievement {
        id: "trek_klingons_50",
        name: "Starfleet Captain",
        description: "Destroy 50 Klingons total",
        icon: 'T',
        game: Some(GameType::Trek),
        hidden: false,
        condition: AchievementCondition::CounterReached {
            game: GameType::Trek,
            counter: "klingons_destroyed",
            value: 50,
        },
    },
    // === CLICKER ===
    Achievement {
        id: "clicker_floor_10",
        name: "Dungeon Delver",
        description: "Reach floor 10 in Clicker",
        icon: '1',
        game: Some(GameType::Clicker),
        hidden: false,
        condition: AchievementCondition::LevelReached(GameType::Clicker, 10),
    },
    Achievement {
        id: "clicker_prestige",
        name: "Soul Reaper",
        description: "Prestige for the first time",
        icon: 'P',
        game: Some(GameType::Clicker),
        hidden: false,
        condition: AchievementCondition::CounterReached {
            game: GameType::Clicker,
            counter: "prestiges",
            value: 1,
        },
    },
    Achievement {
        id: "clicker_gold_1m",
        name: "Gold Hoarder",
        description: "Earn 1,000,000 gold total",
        icon: 'M',
        game: Some(GameType::Clicker),
        hidden: false,
        condition: AchievementCondition::CounterReached {
            game: GameType::Clicker,
            counter: "total_gold",
            value: 1_000_000,
        },
    },
    // === BRAINIAC ===
    Achievement {
        id: "brainiac_correct_10",
        name: "Quick Thinker",
        description: "Answer 10 questions correctly",
        icon: '?',
        game: Some(GameType::Brainiac),
        hidden: false,
        condition: AchievementCondition::CounterReached {
            game: GameType::Brainiac,
            counter: "correct_answers",
            value: 10,
        },
    },
    Achievement {
        id: "brainiac_correct_100",
        name: "Trivia Buff",
        description: "Answer 100 questions correctly",
        icon: 'Q',
        game: Some(GameType::Brainiac),
        hidden: false,
        condition: AchievementCondition::CounterReached {
            game: GameType::Brainiac,
            counter: "correct_answers",
            value: 100,
        },
    },
    Achievement {
        id: "brainiac_perfect",
        name: "Perfect Score",
        description: "Complete a quiz with no mistakes",
        icon: 'A',
        game: Some(GameType::Brainiac),
        hidden: false,
        condition: AchievementCondition::Custom("brainiac_perfect_game"),
    },
    // === STORYWEAVER ===
    Achievement {
        id: "storyweaver_complete",
        name: "The End",
        description: "Complete your first story",
        icon: '.',
        game: Some(GameType::Storyweaver),
        hidden: false,
        condition: AchievementCondition::CounterReached {
            game: GameType::Storyweaver,
            counter: "stories_completed",
            value: 1,
        },
    },
    Achievement {
        id: "storyweaver_complete_5",
        name: "Storyteller",
        description: "Complete 5 stories",
        icon: 'W',
        game: Some(GameType::Storyweaver),
        hidden: false,
        condition: AchievementCondition::CounterReached {
            game: GameType::Storyweaver,
            counter: "stories_completed",
            value: 5,
        },
    },
    Achievement {
        id: "storyweaver_all_templates",
        name: "Genre Master",
        description: "Complete all 5 story templates",
        icon: '*',
        game: Some(GameType::Storyweaver),
        hidden: true,
        condition: AchievementCondition::Custom("storyweaver_all_templates"),
    },
    // === DOPE WARS ===
    Achievement {
        id: "dopewars_win",
        name: "Drug Lord",
        description: "End with positive net worth",
        icon: '$',
        game: Some(GameType::DopeWars),
        hidden: false,
        condition: AchievementCondition::GameWon(GameType::DopeWars),
    },
    Achievement {
        id: "dopewars_score_100k",
        name: "Hustler",
        description: "End with $100,000 net worth",
        icon: 'H',
        game: Some(GameType::DopeWars),
        hidden: false,
        condition: AchievementCondition::ScoreReached(GameType::DopeWars, 100000),
    },
    Achievement {
        id: "dopewars_debt_free",
        name: "Clean Slate",
        description: "Pay off your debt completely",
        icon: '0',
        game: Some(GameType::DopeWars),
        hidden: false,
        condition: AchievementCondition::CounterReached {
            game: GameType::DopeWars,
            counter: "debt_paid_off",
            value: 1,
        },
    },
    // === MINESWEEPER ===
    Achievement {
        id: "minesweeper_win",
        name: "Mine Sweeper",
        description: "Clear all safe cells",
        icon: 'F',
        game: Some(GameType::Minesweeper),
        hidden: false,
        condition: AchievementCondition::GameWon(GameType::Minesweeper),
    },
    Achievement {
        id: "minesweeper_wins_10",
        name: "Bomb Squad",
        description: "Win 10 games of Minesweeper",
        icon: 'X',
        game: Some(GameType::Minesweeper),
        hidden: false,
        condition: AchievementCondition::CounterReached {
            game: GameType::Minesweeper,
            counter: "games_won",
            value: 10,
        },
    },
    // === ARTILLERY ===
    Achievement {
        id: "artillery_win",
        name: "Artillery Expert",
        description: "Destroy the enemy fortress",
        icon: 'A',
        game: Some(GameType::Artillery),
        hidden: false,
        condition: AchievementCondition::GameWon(GameType::Artillery),
    },
    Achievement {
        id: "artillery_wins_10",
        name: "Bombardier",
        description: "Win 10 games of Artillery",
        icon: 'B',
        game: Some(GameType::Artillery),
        hidden: false,
        condition: AchievementCondition::CounterReached {
            game: GameType::Artillery,
            counter: "games_won",
            value: 10,
        },
    },
    // === MINDGAMES ===
    Achievement {
        id: "mindgames_perfect",
        name: "Genius",
        description: "Get all 10 questions correct",
        icon: 'G',
        game: Some(GameType::Mindgames),
        hidden: false,
        condition: AchievementCondition::Custom("mindgames_perfect_game"),
    },
    Achievement {
        id: "mindgames_daily_10",
        name: "Daily Devotee",
        description: "Complete 10 daily challenges",
        icon: 'D',
        game: Some(GameType::Mindgames),
        hidden: false,
        condition: AchievementCondition::CounterReached {
            game: GameType::Mindgames,
            counter: "daily_completed",
            value: 10,
        },
    },
];

// =============================================================================
// ACHIEVEMENT MANAGER
// =============================================================================

/// Persistent state for achievements
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AchievementState {
    /// IDs of unlocked achievements
    #[serde(default)]
    pub unlocked: HashSet<String>,
    /// IDs of achievements whose notifications have been dismissed
    #[serde(default)]
    pub seen: HashSet<String>,
}

/// Achievement being displayed as a toast
#[derive(Debug, Clone)]
pub struct AchievementToast {
    pub achievement: &'static Achievement,
    pub ticks_remaining: u32,
}

/// Manages achievement checking and unlocking
pub struct AchievementManager {
    pub state: AchievementState,
    /// Queue of newly unlocked achievements to show
    pub pending_toasts: Vec<&'static Achievement>,
    /// Currently displayed toast
    pub current_toast: Option<AchievementToast>,
    /// Whether state has been modified
    dirty: bool,
}

impl AchievementManager {
    /// Toast display duration in ticks (3 seconds at 10 ticks/sec)
    const TOAST_DURATION: u32 = 30;

    /// Create a new manager with default state
    pub fn new() -> Self {
        Self {
            state: AchievementState::default(),
            pending_toasts: Vec::new(),
            current_toast: None,
            dirty: false,
        }
    }

    /// Create manager from loaded state
    pub fn from_state(state: AchievementState) -> Self {
        Self {
            state,
            pending_toasts: Vec::new(),
            current_toast: None,
            dirty: false,
        }
    }

    /// Check all achievements against current stats and unlock any that are met
    pub fn check_all(&mut self, stats: &PlayerStats, current_score: Option<(GameType, u32)>) {
        for achievement in ACHIEVEMENTS {
            if self.state.unlocked.contains(achievement.id) {
                continue;
            }

            if self.check_condition(&achievement.condition, stats, current_score) {
                self.unlock(achievement);
            }
        }
    }

    /// Check a single condition against stats
    fn check_condition(
        &self,
        condition: &AchievementCondition,
        stats: &PlayerStats,
        current_score: Option<(GameType, u32)>,
    ) -> bool {
        match condition {
            AchievementCondition::GamesPlayed(n) => stats.total_games_played >= *n,
            AchievementCondition::GamesWon(n) => stats.total_games_won >= *n,
            AchievementCondition::TotalPlaytime(secs) => stats.total_playtime_secs >= *secs,

            AchievementCondition::ScoreReached(game, score) => {
                // Check current game score
                if let Some((current_game, current_score)) = current_score {
                    if current_game == *game && current_score >= *score {
                        return true;
                    }
                }
                // Check high score
                stats
                    .games
                    .get(game.name())
                    .is_some_and(|gs| gs.high_score >= *score)
            }

            AchievementCondition::HighScoreReached(game, score) => stats
                .games
                .get(game.name())
                .is_some_and(|gs| gs.high_score >= *score),

            AchievementCondition::LevelReached(game, level) => stats
                .games
                .get(game.name())
                .and_then(|gs| gs.best_level)
                .is_some_and(|l| l >= *level),

            AchievementCondition::GameWon(game) => stats
                .games
                .get(game.name())
                .is_some_and(|gs| gs.times_won > 0),

            AchievementCondition::CounterReached {
                game,
                counter,
                value,
            } => stats
                .games
                .get(game.name())
                .and_then(|gs| gs.counters.get(*counter))
                .is_some_and(|v| *v >= *value),

            AchievementCondition::AllGamesWon => {
                // Check if all winnable games have been won
                let winnable_games = [
                    GameType::Breakout,
                    GameType::Rogue,
                    GameType::Trek,
                    GameType::Brainiac,
                    GameType::Storyweaver,
                    GameType::DopeWars,
                    GameType::Minesweeper,
                    GameType::Artillery,
                ];
                winnable_games.iter().all(|game| {
                    stats
                        .games
                        .get(game.name())
                        .is_some_and(|gs| gs.times_won > 0)
                })
            }

            AchievementCondition::AllGamesPlayed => GameType::all().iter().all(|game| {
                stats
                    .games
                    .get(game.name())
                    .is_some_and(|gs| gs.times_played > 0)
            }),

            AchievementCondition::Custom(_key) => {
                // Custom conditions are checked by games themselves
                false
            }
        }
    }

    /// Unlock an achievement and queue it for display
    fn unlock(&mut self, achievement: &'static Achievement) {
        self.state.unlocked.insert(achievement.id.to_string());
        self.pending_toasts.push(achievement);
        self.dirty = true;
    }

    /// Manually unlock by custom key (called by games)
    pub fn unlock_custom(&mut self, key: &str) {
        for achievement in ACHIEVEMENTS {
            if let AchievementCondition::Custom(custom_key) = achievement.condition {
                if custom_key == key && !self.state.unlocked.contains(achievement.id) {
                    self.unlock(achievement);
                }
            }
        }
    }

    /// Update toast display (call from tick)
    pub fn tick(&mut self) {
        // Update current toast timer
        if let Some(toast) = &mut self.current_toast {
            if toast.ticks_remaining > 0 {
                toast.ticks_remaining -= 1;
            } else {
                // Mark as seen and clear
                self.state.seen.insert(toast.achievement.id.to_string());
                self.current_toast = None;
            }
        }

        // Show next pending toast if no current one
        if self.current_toast.is_none() && !self.pending_toasts.is_empty() {
            let achievement = self.pending_toasts.remove(0);
            self.current_toast = Some(AchievementToast {
                achievement,
                ticks_remaining: Self::TOAST_DURATION,
            });
        }
    }

    /// Dismiss current toast immediately
    pub fn dismiss_toast(&mut self) {
        if let Some(toast) = &self.current_toast {
            self.state.seen.insert(toast.achievement.id.to_string());
        }
        self.current_toast = None;
    }

    /// Check if there's a toast to display
    pub fn has_toast(&self) -> bool {
        self.current_toast.is_some()
    }

    /// Get count of unlocked achievements
    pub fn unlocked_count(&self) -> usize {
        self.state.unlocked.len()
    }

    /// Get total achievement count
    pub fn total_count(&self) -> usize {
        ACHIEVEMENTS.len()
    }

    /// Check if an achievement is unlocked
    pub fn is_unlocked(&self, id: &str) -> bool {
        self.state.unlocked.contains(id)
    }

    /// Get all achievements organized by game
    pub fn get_achievements_by_game(&self) -> Vec<(Option<GameType>, Vec<&'static Achievement>)> {
        let mut result: Vec<(Option<GameType>, Vec<&'static Achievement>)> = Vec::new();

        // Global achievements first
        let global: Vec<_> = ACHIEVEMENTS.iter().filter(|a| a.game.is_none()).collect();
        if !global.is_empty() {
            result.push((None, global));
        }

        // Per-game achievements
        for game in GameType::all() {
            let game_achievements: Vec<_> = ACHIEVEMENTS
                .iter()
                .filter(|a| a.game == Some(*game))
                .collect();
            if !game_achievements.is_empty() {
                result.push((Some(*game), game_achievements));
            }
        }

        result
    }

    /// Get progress for a specific achievement (for display)
    pub fn get_progress(&self, achievement: &Achievement, stats: &PlayerStats) -> Option<String> {
        if self.is_unlocked(achievement.id) {
            return Some("UNLOCKED".to_string());
        }

        match &achievement.condition {
            AchievementCondition::GamesPlayed(target) => {
                Some(format!("{} / {}", stats.total_games_played, target))
            }
            AchievementCondition::GamesWon(target) => {
                Some(format!("{} / {}", stats.total_games_won, target))
            }
            AchievementCondition::TotalPlaytime(target) => {
                let hours = stats.total_playtime_secs / 3600;
                let target_hours = target / 3600;
                Some(format!("{}h / {}h", hours, target_hours))
            }
            AchievementCondition::ScoreReached(game, target)
            | AchievementCondition::HighScoreReached(game, target) => {
                let current = stats
                    .games
                    .get(game.name())
                    .map(|gs| gs.high_score)
                    .unwrap_or(0);
                Some(format!("{} / {}", current, target))
            }
            AchievementCondition::LevelReached(game, target) => {
                let current = stats
                    .games
                    .get(game.name())
                    .and_then(|gs| gs.best_level)
                    .unwrap_or(0);
                Some(format!("{} / {}", current, target))
            }
            AchievementCondition::CounterReached {
                game,
                counter,
                value,
            } => {
                let current = stats
                    .games
                    .get(game.name())
                    .and_then(|gs| gs.counters.get(*counter))
                    .copied()
                    .unwrap_or(0);
                Some(format!("{} / {}", current, value))
            }
            AchievementCondition::AllGamesWon => {
                let winnable = [
                    GameType::Breakout,
                    GameType::Rogue,
                    GameType::Trek,
                    GameType::Brainiac,
                    GameType::Storyweaver,
                    GameType::DopeWars,
                    GameType::Minesweeper,
                    GameType::Artillery,
                ];
                let won = winnable
                    .iter()
                    .filter(|g| stats.games.get(g.name()).is_some_and(|gs| gs.times_won > 0))
                    .count();
                Some(format!("{} / {}", won, winnable.len()))
            }
            AchievementCondition::AllGamesPlayed => {
                let all_games = GameType::all();
                let played = all_games
                    .iter()
                    .filter(|g| {
                        stats
                            .games
                            .get(g.name())
                            .is_some_and(|gs| gs.times_played > 0)
                    })
                    .count();
                Some(format!("{} / {}", played, all_games.len()))
            }
            AchievementCondition::GameWon(_) | AchievementCondition::Custom(_) => None,
        }
    }

    /// Check if state has been modified
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark state as saved
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }
}

impl Default for AchievementManager {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_games_played_achievement() {
        let mut manager = AchievementManager::new();
        let mut stats = PlayerStats::default();

        // Not unlocked initially
        assert!(!manager.is_unlocked("first_steps"));

        // Play one game
        stats.total_games_played = 1;
        manager.check_all(&stats, None);

        assert!(manager.is_unlocked("first_steps"));
        assert_eq!(manager.pending_toasts.len(), 1);
    }

    #[test]
    fn test_score_achievement() {
        let mut manager = AchievementManager::new();
        let stats = PlayerStats::default();

        // Check with current score
        manager.check_all(&stats, Some((GameType::Snake, 150)));

        assert!(manager.is_unlocked("snake_score_100"));
    }

    #[test]
    fn test_achievement_progress() {
        let manager = AchievementManager::new();
        let mut stats = PlayerStats::default();
        stats.total_games_played = 0;

        let achievement = ACHIEVEMENTS.iter().find(|a| a.id == "first_steps").unwrap();
        let progress = manager.get_progress(achievement, &stats);

        assert_eq!(progress, Some("0 / 1".to_string()));
    }

    #[test]
    fn test_toast_lifecycle() {
        let mut manager = AchievementManager::new();
        let mut stats = PlayerStats::default();
        stats.total_games_played = 1;

        // Unlock achievement
        manager.check_all(&stats, None);
        assert!(!manager.pending_toasts.is_empty());
        assert!(manager.current_toast.is_none());

        // First tick shows toast
        manager.tick();
        assert!(manager.current_toast.is_some());
        assert!(manager.pending_toasts.is_empty());

        // Dismiss toast
        manager.dismiss_toast();
        assert!(manager.current_toast.is_none());
        assert!(manager.state.seen.contains("first_steps"));
    }
}
