# R-DOS Game Platform Specification

A unified game engine and platform for the R-DOS Games plugin, providing consistent
interfaces, player statistics, and achievements across all games.

## Design Principles

1. **Game Independence** - Each game owns its logic; platform provides services
2. **Event-Driven** - Games emit events; platform reacts (stats, achievements)
3. **Backward Compatible** - Existing games work during migration
4. **Minimal Overhead** - Trait methods have sensible defaults
5. **Single Player Focus** - No online features, local persistence only

## Current Games

| Game | Type | Win Condition | Persistence | Complexity |
|------|------|---------------|-------------|------------|
| Tetris | Arcade | None (endless) | Leaderboard | Simple |
| Snake | Arcade | None (endless) | Leaderboard | Simple |
| Breakout | Arcade | Clear all bricks | Leaderboard | Simple |
| Rogue | Roguelike | Reach floor 10 | Leaderboard | Medium |
| Trek | Strategy | Destroy Klingons | Leaderboard | Medium |
| Clicker | Idle/RPG | None (endless) | Full save | Complex |
| Brainiac | Trivia | Complete quiz | Leaderboard | Medium (AI) |
| Storyweaver | Adventure | Finish story | Leaderboard | Medium (AI) |
| Dope Wars | Trading | Max net worth in 30 days | Leaderboard | Medium |

---

## Splash Screens & Music

### Splash Screens

Each game displays a sixel splash screen before gameplay begins. Splash screens are:
- Embedded at compile time via `include_bytes!()`
- Located in `assets/splash/[game].png`
- Displayed using the ratatui-image crate with protocol auto-detection (Kitty/Sixel/iTerm2)
- Falls back to ASCII art if terminal doesn't support graphics

**Splash Screen Flow:**
1. User selects game from menu
2. `GamesView::Splash` is displayed with sixel image
3. Random menu melody plays as splash music
4. Any key starts the game (transitions to `GamesView::Playing`)
5. Q/Esc exits back to menu

**Files:**
- `src/plugins/games/modal/splash.rs` - Splash screen rendering
- `assets/splash/*.png` - Splash screen images (19 games)

### Background Music

The games platform uses procedural chiptune music via the `ChiptuneMusic` system:

**Menu Melodies (5 variants, randomly selected):**
- `GameMenu` - Classic upbeat 8-bit arpeggio
- `GameMenu2` - Mellow, slower ambient feel
- `GameMenu3` - Energetic, faster rhythmic pattern
- `GameMenu4` - Mysterious minor key theme
- `GameMenu5` - Triumphant fanfare style

**Music Triggers:**
- Menu opens → `ChiptuneMelody::random_menu()`
- Splash screen → `ChiptuneMelody::random_menu()`
- Game starts → `ChiptuneMelody::random_menu()`
- Return to menu → `ChiptuneMelody::random_menu()`

**Files:**
- `src/sound.rs` - `ChiptuneMusic` and `ChiptuneMelody` implementation

---

## 1. Game Engine Trait

### Core Trait Definition

```rust
pub trait GameEngine {
    // === Required Methods ===

    /// Process one game tick (~10Hz for most games)
    fn tick(&mut self);

    /// Handle keyboard input, return result
    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult;

    /// Current score for leaderboard/stats
    fn get_score(&self) -> u32;

    /// Is the game over (loss or win)?
    fn is_game_over(&self) -> bool;

    // === Optional Methods (with defaults) ===

    /// Did player win? (false for endless games)
    fn is_game_won(&self) -> bool { false }

    /// Current level/floor/chapter (None for level-less games)
    fn get_level(&self) -> Option<u32> { None }

    /// Collect pending events for platform processing
    fn drain_events(&mut self) -> Vec<GameEvent> { Vec::new() }

    /// Game-specific stats for achievement conditions
    fn get_stat(&self, key: &str) -> Option<u64> { None }
}
```

### Key Handle Result

```rust
pub enum KeyHandleResult {
    Handled,           // Input processed, continue
    NotHandled,        // Input not recognized
    GameOver,          // Game ended from this input
    RequestPause,      // Game wants to pause
    RequestQuit,       // Game wants to quit to menu
}
```

### Implementation Per Game

Each game implements the trait in its own module:

```rust
// In tetris.rs
impl GameEngine for TetrisState {
    fn tick(&mut self) { /* existing tick logic */ }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Left => { self.move_left(); KeyHandleResult::Handled }
            KeyCode::Right => { self.move_right(); KeyHandleResult::Handled }
            KeyCode::Up => { self.rotate(); KeyHandleResult::Handled }
            KeyCode::Down => { self.soft_drop(); KeyHandleResult::Handled }
            KeyCode::Char(' ') => { self.hard_drop(); KeyHandleResult::Handled }
            KeyCode::Char('p') | KeyCode::Char('P') => KeyHandleResult::RequestPause,
            KeyCode::Esc => KeyHandleResult::RequestQuit,
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn get_score(&self) -> u32 { self.score }
    fn is_game_over(&self) -> bool { self.game_over }
    fn get_level(&self) -> Option<u32> { Some(self.level) }

    fn get_stat(&self, key: &str) -> Option<u64> {
        match key {
            "lines_cleared" => Some(self.lines_cleared as u64),
            "tetrises" => Some(self.tetrises as u64),
            _ => None,
        }
    }
}
```

---

## 2. Game Events

Events emitted by games for platform processing.

```rust
pub enum GameEvent {
    // Lifecycle
    GameStarted,
    GameEnded { won: bool },

    // Progress
    ScoreChanged { old: u32, new: u32 },
    LevelReached(u32),

    // Game-specific milestones
    LinesCleared(u32),          // Tetris
    FoodEaten,                   // Snake
    BrickDestroyed,              // Breakout
    FloorReached(u32),           // Rogue
    EnemyDefeated(String),       // Rogue, Trek, Clicker
    KlingonDestroyed,            // Trek
    QuestionAnswered { correct: bool }, // Brainiac
    ChapterCompleted,            // Storyweaver

    // Custom (for extensibility)
    Custom { key: String, value: u64 },
}
```

### Event Flow

```
Game.tick() / handle_key()
    |
    v
Game emits events to internal queue
    |
    v
Platform calls game.drain_events()
    |
    v
Platform processes events:
    - Update PlayerStats
    - Check AchievementConditions
    - Trigger notifications
```

---

## 3. Stats Tracking

### Data Structures

```rust
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlayerStats {
    // Global lifetime stats
    pub total_playtime_secs: u64,
    pub total_games_played: u32,
    pub total_games_won: u32,
    pub first_played: Option<DateTime<Utc>>,

    // Per-game stats
    pub games: HashMap<String, GameStats>,  // Key: GameType as string
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GameStats {
    pub times_played: u32,
    pub times_won: u32,
    pub total_playtime_secs: u64,
    pub high_score: u32,
    pub best_level: Option<u32>,
    pub last_played: Option<DateTime<Utc>>,

    // Game-specific counters (flexible)
    pub counters: HashMap<String, u64>,
}
```

### Tracked Counters Per Game

| Game | Counters |
|------|----------|
| Tetris | `lines_cleared`, `tetrises`, `max_level` |
| Snake | `food_eaten`, `max_length` |
| Breakout | `bricks_destroyed`, `perfect_games` |
| Rogue | `floors_reached`, `enemies_killed`, `gold_collected` |
| Trek | `klingons_destroyed`, `starbases_visited` |
| Clicker | `monsters_killed`, `total_gold`, `prestiges` |
| Brainiac | `questions_answered`, `correct_answers`, `perfect_rounds` |
| Storyweaver | `stories_completed`, `chapters_read`, `choices_made` |

### Stats UI

```
╔══════════════════════════════════════════════════════════════════════════════╗
║  PLAYER STATISTICS                                                           ║
╠══════════════════════════════════════════════════════════════════════════════╣
║                                                                              ║
║  LIFETIME                              RECENT ACTIVITY                       ║
║  ────────                              ───────────────                       ║
║  Total Play Time:    12h 34m           Last Played: Snake (2 hours ago)      ║
║  Games Played:       247               Session Time: 45m                     ║
║  Games Won:          38                                                      ║
║  Playing Since:      Jan 10, 2026                                            ║
║                                                                              ║
║  PER-GAME STATS                                                              ║
║  ──────────────                                                              ║
║  Game          Played   Won    High Score   Best Level   Time               ║
║  ──────────────────────────────────────────────────────────────────         ║
║  Tetris           42     -      125,400        15       2h 15m               ║
║  Snake            38     -        1,250         -       1h 42m               ║
║  Breakout         25    18       48,000         -         58m                ║
║  Rogue            31     3        2,847        10       3h 21m               ║
║  Trek             15     8       12,500         -       1h 05m               ║
║  Clicker          12     -    1,234,567        47       2h 48m               ║
║  Brainiac         52    21        9,850         -         34m                ║
║  Storyweaver      32    27        4,200         8         21m                ║
║                                                                              ║
╚══════════════════════════════════════════════════════════════════════════════╝
                              [S]ort  [Esc]Close
```

---

## 4. Achievements System

### Data Structures

```rust
pub struct Achievement {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub icon: char,              // ASCII icon for display
    pub game: Option<GameType>,  // None = cross-game achievement
    pub hidden: bool,            // Don't show until unlocked
    pub condition: AchievementCondition,
}

pub enum AchievementCondition {
    // Simple conditions
    GamesPlayed(u32),
    GamesWon(u32),
    TotalPlaytime(u64),  // seconds

    // Score-based
    ScoreReached(GameType, u32),
    HighScoreReached(GameType, u32),

    // Progress-based
    LevelReached(GameType, u32),
    GameWon(GameType),

    // Counter-based (uses GameStats.counters)
    CounterReached { game: GameType, counter: &'static str, value: u64 },

    // Compound conditions
    AllGamesWon,
    AllGamesPlayed,

    // Special
    Custom(&'static str),  // Checked via game.get_stat()
}
```

### Achievement Registry

```rust
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
];
```

### Achievements UI

```
╔══════════════════════════════════════════════════════════════════════════════╗
║  ACHIEVEMENTS                                                      12 / 32   ║
╠══════════════════════════════════════════════════════════════════════════════╣
║                                                                              ║
║  GLOBAL                                                                      ║
║  [*] First Steps         Play your first game                     UNLOCKED   ║
║  [+] Dedicated           Play for 1 hour total                    UNLOCKED   ║
║  [ ] Veteran             Play for 10 hours total                  4h / 10h   ║
║  [!] Winner              Win any game                             UNLOCKED   ║
║  [ ] Completionist       Win every game at least once             3 / 8      ║
║                                                                              ║
║  TETRIS                                                                      ║
║  [-] Line Clear          Clear your first line                    UNLOCKED   ║
║  [ ] Tetris Master       Clear 100 lines total                    67 / 100   ║
║  [>] Speed Demon         Reach level 10                           UNLOCKED   ║
║                                                                              ║
║  SNAKE                                                                       ║
║  [~] Hungry Snake        Score 100 points                         UNLOCKED   ║
║  [S] Snake Charmer       Score 500 points                         UNLOCKED   ║
║  [ ] ???                 ???                                      LOCKED     ║
║                                                                              ║
║  ... (scroll for more)                                                       ║
║                                                                              ║
╚══════════════════════════════════════════════════════════════════════════════╝
                            [^v]Scroll  [Esc]Close
```

### Achievement Toast

When an achievement unlocks, show a brief notification:

```
╔════════════════════════════════════╗
║  ACHIEVEMENT UNLOCKED!             ║
║  [S] Snake Charmer                 ║
║      Score 500 points in Snake     ║
╚════════════════════════════════════╝
```

Display for 3 seconds, or until dismissed with any key.

---

## 5. Persistence

### Platform Data Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GamePlatformData {
    pub version: u32,  // For migration
    pub stats: PlayerStats,
    pub achievements_unlocked: Vec<String>,
    pub achievements_seen: Vec<String>,  // Dismissed notifications
    pub leaderboards: Leaderboards,      // Existing leaderboard data

    // Game-specific saves
    pub clicker_save: Option<String>,    // Base64-encoded save
}
```

### Storage Location

```
~/Library/Application Support/rdos/config.toml  (macOS)
~/.config/rdos/config.toml                       (Linux)
%APPDATA%/rdos/config.toml                       (Windows)
```

### Save/Load Pattern

```rust
impl GamePlatform {
    pub fn save(&self) -> Result<(), String> {
        let data = GamePlatformData {
            version: 1,
            stats: self.stats.clone(),
            achievements_unlocked: self.achievements.unlocked.clone(),
            achievements_seen: self.achievements.seen.clone(),
            leaderboards: self.leaderboards.clone(),
            clicker_save: self.clicker_save.clone(),
        };

        // Merge into existing config.toml
        config::save_platform_data(&data)
    }

    pub fn load() -> Self {
        config::load_platform_data()
            .map(|data| Self::from_data(data))
            .unwrap_or_default()
    }
}
```

---

## 6. File Structure

```
src/plugins/games/
├── mod.rs                  # GamesPlugin, delegates to platform
├── state.rs                # GamesState, GamesView, GameType
│
├── platform/
│   ├── mod.rs              # GamePlatform coordinator
│   ├── engine.rs           # GameEngine trait definition
│   ├── events.rs           # GameEvent enum
│   ├── stats.rs            # PlayerStats, GameStats, StatsTracker
│   ├── achievements.rs     # Achievement, ACHIEVEMENTS registry, AchievementManager
│   └── persistence.rs      # Save/load platform data
│
├── tetris.rs               # TetrisState + impl GameEngine
├── snake.rs                # SnakeState + impl GameEngine
├── breakout.rs             # BreakoutState + impl GameEngine
├── rogue.rs                # RogueState + impl GameEngine
├── trek.rs                 # TrekState + impl GameEngine
├── clicker.rs              # ClickerState + impl GameEngine
├── brainiac.rs             # BrainiacState + impl GameEngine
├── storyweaver.rs          # StoryweaverState + impl GameEngine
│
└── modal/
    ├── mod.rs              # Modal dispatcher
    ├── menu.rs             # Game selection menu
    ├── splash.rs           # Sixel splash screen rendering
    ├── stats.rs            # Stats UI screen
    ├── achievements.rs     # Achievements UI screen
    ├── tetris.rs           # Tetris renderer
    ├── snake.rs            # Snake renderer
    ├── breakout.rs         # Breakout renderer
    ├── rogue.rs            # Rogue renderer
    ├── trek.rs             # Trek renderer
    ├── clicker.rs          # Clicker renderer
    ├── brainiac.rs         # Brainiac renderer
    └── storyweaver.rs      # Storyweaver renderer

assets/splash/               # Splash screen images (embedded at compile time)
├── adventure.png
├── biolab.png
├── blackjack.png
├── blockworld.png
├── breakout.png
├── caverns.png
├── cosmos.png
├── dopewars.png
├── dungeon.png
├── gumshoe.png
├── micropolis.png
├── minesweeper.png
├── poker.png
├── rogue.png
├── slots.png
├── snake.png
├── tetris.png
├── trek.png
└── westworld.png
```

---

## 7. Migration Plan

### Phase 1: Foundation
1. Create `platform/` directory with trait and event definitions
2. Add `GameEngine` trait with defaults for backward compat
3. Implement trait for one game (Tetris) as proof of concept
4. Verify existing functionality still works

### Phase 2: Full Trait Implementation
1. Implement `GameEngine` for remaining 7 games
2. Move key handling from mod.rs to each game's `handle_key()`
3. Add `get_score()`, `is_game_over()`, `is_game_won()` where missing
4. Refactor tick dispatcher to use trait

### Phase 3: Stats System
1. Create `PlayerStats` and `GameStats` structures
2. Add stats persistence to config
3. Hook game start/end to update stats
4. Implement session playtime tracking
5. Create Stats UI modal

### Phase 4: Events & Achievements
1. Implement `GameEvent` enum and event queuing in games
2. Create `AchievementManager` with condition checking
3. Add achievement registry (30+ achievements)
4. Create Achievements UI modal
5. Implement achievement toast notifications

### Phase 5: Polish
1. Add achievement progress indicators
2. Add session summary on game end
3. Clean up any remaining code duplication
4. Documentation and testing

---

## 8. Testing Strategy

### Unit Tests
- Achievement condition evaluation
- Stats calculation and aggregation
- Event processing

### Integration Tests
- Game lifecycle (start -> play -> end)
- Achievement unlocking flow
- Stats persistence across sessions

### Manual Testing
- Play each game, verify stats update
- Trigger each achievement type
- Verify toast notifications appear
- Test save/load between sessions

---

## 9. Game Specifications

### Dope Wars

**Type**: Trading/Economic simulation
**Win Condition**: Maximize net worth in 30 days
**Complexity**: Medium

#### Overview

A classic drug trading game where the player travels between locations buying and selling products to maximize profit while paying off debt. The game ends after 30 days, and the final score is calculated as net worth (cash + inventory value - debt).

#### Game Mechanics

**Starting Conditions:**
- Cash: $2,000
- Debt: $5,500 (to loan shark)
- Location: Bronx
- Coat Capacity: 100 units
- Interest Rate: 10% per day on debt

**Locations:**
- Bronx
- Brooklyn
- Manhattan
- Queens
- Staten Island
- Central Park

**Products:**
- Acid ($1,000-$4,500)
- Cocaine ($15,000-$30,000)
- Hashish ($480-$1,280)
- Heroin ($5,500-$13,000)
- MDA ($1,500-$4,400)
- Opium ($540-$1,250)

**Market Dynamics:**
- Prices vary randomly by location and day
- 30% chance a product isn't available
- 5% chance of price crash (×0.25)
- 5% chance of price spike (×4)

**Random Events** (per travel):
- 10% chance of cops (50% chance lose all inventory)
- 5% chance find stash (5-20 units of random product)
- 5% chance mugged ($100-$500 stolen)
- 5% chance loan shark deal (pay $X, reduce debt by $2X)

#### Views

**Market View:**
- Product list with prices and inventory
- Quantity input (type digits)
- Buy (B), Sell (S), Travel (T), Pay Debt (D), Info (I)

**Travel View:**
- Location selection
- Travel advances 1 day and applies debt interest

**Status View:**
- Days remaining
- Financial summary (cash, debt, inventory)
- Estimated net worth

#### Controls

```
Market:
  ↑↓/jk       - Select product
  B           - Buy selected product
  S           - Sell selected product
  0-9         - Enter quantity
  Backspace   - Delete digit
  T/Tab       - Travel menu
  D           - Pay debt
  I           - Status screen
  P           - Pause
  Esc         - Quit

Travel:
  ↑↓/jk       - Select location
  Enter/Space - Travel
  Esc         - Cancel

Status:
  Esc/Enter   - Return to market
```

#### Scoring

Final score = Cash + Inventory Value - Debt

Where inventory value is calculated using average base prices.

#### Implementation

**Files:**
- `src/plugins/games/dopewars.rs` - Game logic
- `src/plugins/games/modal/dopewars.rs` - UI rendering

**Key Types:**
```rust
pub enum Product { Acid, Cocaine, Hashish, Heroin, MDA, Opium }
pub enum Location { Bronx, Brooklyn, Manhattan, Queens, StatenIsland, CentralPark }
pub struct Market { prices: Vec<(Product, Option<i64>)> }
pub struct Inventory { items: Vec<(Product, u32)> }
pub enum DopeWarsView { Market, Travel, Status, Event }
```

**State:**
```rust
pub struct DopeWarsState {
    pub view: DopeWarsView,
    pub day: u32,
    pub cash: i64,
    pub debt: i64,
    pub location: Location,
    pub inventory: Inventory,
    pub market: Market,
    pub selected_product: usize,
    pub selected_location: usize,
    pub quantity_buffer: String,
    pub message: Option<String>,
    pub game_over: bool,
}
```

**Testing:**
- Buy/sell mechanics
- Debt interest calculation
- Random event probabilities
- Market price generation
- 30-day game length
- Score calculation
- Inventory capacity limits

#### Updated Mechanics (v2)

**Health & Combat:**
- Starting health: 100 HP
- Health can be damaged by cops, thugs, and combat
- Game over if health reaches 0
- Can be healed by random events (nice old lady)

**Guns:**
- Buy from gun shop encounters ($400 each)
- Max 10 guns
- Provides protection against cops and thugs
- With 3+ guns, 60% chance to win fights
- Reduces damage taken in muggings

**Enhanced Random Events (50% chance per travel):**
- **Officer Hardass Raid** (20% of events) - Loses 30-70% of inventory, takes damage. Guns help escape.
- **Find Stash** (15% of events) - 10-30 units of random product
- **Mugged** (15% of events) - Lose $200-800, take damage. Guns reduce damage.
- **Gun Shop** (15% of events) - Trenchcoat dealer offers guns at $400 each
- **Officer Hardass Bribe** (15% of events) - Pay $1000-3000 or fight/lose everything
- **Loan Shark Deal** (15% of events) - Pay $X, debt reduced by $2X
- **Free Healing** (5% of events) - Nice old lady heals 20-40 HP

**Event Interactions:**
- **Gun Shop**: Press G to buy max guns, N to decline
- **Officer Bribe**: Press P to pay, F to fight (risky unless 3+ guns)
