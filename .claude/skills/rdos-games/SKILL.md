# R-DOS Games Platform Skill

Use this skill when working on the Games plugin: adding games, modifying game logic,
implementing platform features (stats, achievements), or fixing game bugs.

## Quick Reference

**Spec**: `spec/games.md` - Full platform specification
**Code**: `src/plugins/games/`

## Architecture Overview

```
src/plugins/games/
├── mod.rs              # GamesPlugin - delegates to platform
├── state.rs            # GamesState, GamesView, GameType enum
├── platform/           # Platform services (engine, stats, achievements)
│   ├── mod.rs          # GamePlatform coordinator
│   ├── engine.rs       # GameEngine trait
│   ├── events.rs       # GameEvent enum
│   ├── stats.rs        # PlayerStats, GameStats
│   └── achievements.rs # Achievement registry & manager
├── [game].rs           # Game state + impl GameEngine
└── modal/
    ├── mod.rs          # Modal dispatcher
    └── [game].rs       # Game renderer
```

## GameEngine Trait

All games MUST implement the `GameEngine` trait:

```rust
pub trait GameEngine {
    // Required
    fn tick(&mut self);
    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult;
    fn get_score(&self) -> u32;
    fn is_game_over(&self) -> bool;

    // Optional (have defaults)
    fn is_game_won(&self) -> bool { false }
    fn get_level(&self) -> Option<u32> { None }
    fn drain_events(&mut self) -> Vec<GameEvent> { Vec::new() }
    fn get_stat(&self, key: &str) -> Option<u64> { None }
}
```

## KeyHandleResult

Games return this from `handle_key()`:

```rust
pub enum KeyHandleResult {
    Handled,        // Input processed
    NotHandled,     // Input not recognized
    GameOver,       // Game ended from input
    RequestPause,   // Game wants to pause
    RequestQuit,    // Game wants to quit to menu
}
```

## GameEvent

Games emit events for platform processing:

```rust
pub enum GameEvent {
    GameStarted,
    GameEnded { won: bool },
    ScoreChanged { old: u32, new: u32 },
    LevelReached(u32),
    // Game-specific events...
    Custom { key: String, value: u64 },
}
```

## Adding a New Game

1. **Create game state** in `src/plugins/games/[game].rs`:
   ```rust
   pub struct MyGameState {
       pub score: u32,
       pub game_over: bool,
       // game-specific fields...
   }

   impl MyGameState {
       pub fn new() -> Self { ... }
       pub fn reset(&mut self) { ... }
   }

   impl GameEngine for MyGameState {
       fn tick(&mut self) { ... }
       fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult { ... }
       fn get_score(&self) -> u32 { self.score }
       fn is_game_over(&self) -> bool { self.game_over }
   }
   ```

2. **Add to GameType enum** in `state.rs`:
   ```rust
   pub enum GameType {
       // existing games...
       MyGame,
   }
   ```

3. **Add to GamesState** in `state.rs`:
   ```rust
   pub struct GamesState {
       // existing games...
       pub mygame: MyGameState,
   }
   ```

4. **Create renderer** in `modal/[game].rs`:
   ```rust
   pub fn draw_mygame(
       frame: &mut Frame,
       view: &FullScreenView,
       state: &MyGameState,
       colors: &ThemeColors,
   ) { ... }
   ```

5. **Register in modal/mod.rs** dispatch

6. **Add leaderboard** entry in `state.rs` Leaderboards

## UI Patterns

### Always use FullScreenView for game rendering:
```rust
pub fn draw_mygame(frame: &mut Frame, view: &FullScreenView, state: &MyGameState, colors: &ThemeColors) {
    // Score/status bar
    view.render_row(frame, 0, vec![
        Span::styled(format!("Score: {}", state.score), Style::default().fg(colors.green())),
    ]);

    // Game content (rows 2-19)
    // ...

    // Help footer
    view.render_help(frame, vec![("Esc", "quit"), ("P", "pause")]);
}
```

### Theme colors (never hardcode):
```rust
colors.fg()      // White - primary text
colors.bg()      // Background
colors.blue()    // Headers
colors.green()   // Score, success
colors.red()     // Errors, selection bg
colors.yellow()  // Highlights, titles
colors.grey()    // Disabled, hidden
colors.cyan()    // Accents
```

## Game State Patterns

### Standard fields every game should have:
```rust
pub struct MyGameState {
    pub score: u32,
    pub game_over: bool,
    pub game_won: bool,      // if applicable
    pub tick_count: u32,     // for timing
    // game-specific...
}
```

### Standard methods:
```rust
impl MyGameState {
    pub fn new() -> Self { ... }
    pub fn reset(&mut self) { ... }  // Reset for new game
    pub fn tick(&mut self) { ... }   // Called ~10Hz
}
```

## Complex Games (Multiple Views)

Games like Clicker, Brainiac, Storyweaver have internal view states:

```rust
pub enum MyGameView {
    Setup,      // Configuration
    Loading,    // AI generation
    Playing,    // Main gameplay
    Paused,     // Game paused
    GameOver,   // End screen
    Error,      // Error display
}

pub struct MyGameState {
    pub view: MyGameView,
    // ...
}
```

Handle view-specific keys in `handle_key()`:
```rust
fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
    match self.view {
        MyGameView::Setup => self.handle_setup_key(key),
        MyGameView::Playing => self.handle_playing_key(key),
        // ...
    }
}
```

## AI-Powered Games

For games using Claude API (Brainiac, Storyweaver):

### Deferred Generation Pattern
Never block the UI during API calls:

```rust
pub struct AIGameState {
    pub pending_generation: bool,
    // ...
}

pub fn start_game(&mut self) {
    self.view = GameView::Loading;
    self.pending_generation = true;  // Will process in tick()
}

fn tick(&mut self) {
    if self.view == GameView::Loading && self.pending_generation {
        self.pending_generation = false;
        self.generate_content();  // Now safe to block
    }
}
```

## Stats Integration

Games should emit events for stats tracking:

```rust
impl GameEngine for MyGameState {
    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.pending_events)
    }

    fn get_stat(&self, key: &str) -> Option<u64> {
        match key {
            "enemies_killed" => Some(self.enemies_killed as u64),
            "max_level" => Some(self.max_level as u64),
            _ => None,
        }
    }
}
```

## Achievement Hooks

Custom achievement conditions use `get_stat()`:

```rust
// In achievements.rs
AchievementCondition::Custom("mygame_special")

// In mygame.rs
fn get_stat(&self, key: &str) -> Option<u64> {
    match key {
        "mygame_special" => if self.special_condition { Some(1) } else { Some(0) },
        _ => None,
    }
}
```

## Testing Checklist

Before submitting game changes:
- [ ] `cargo fmt -- --check`
- [ ] `cargo clippy -- -D warnings`
- [ ] `cargo build`
- [ ] Manual test: start game, play, pause, quit
- [ ] Manual test: game over triggers correctly
- [ ] Manual test: score updates and displays
- [ ] Manual test: leaderboard records score
