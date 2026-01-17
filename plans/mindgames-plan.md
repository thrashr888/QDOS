# MINDGAMES Implementation Plan

## Overview
Implement MINDGAMES - an algorithmic brain training game with 4 modes: Pattern Master, Memory Matrix, Number Ninja, and Daily Challenge. All content generated algorithmically (no AI/API calls), with local leaderboard tracking.

## 1. State Structure Design

### Main State Enum (MindgamesView)
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MindgamesView {
    #[default]
    ModeSelect,      // Choose game mode
    Playing,         // Active gameplay
    Feedback,        // Answer feedback (correct/wrong)
    GameOver,        // Final results
}
```

### Game Mode Enum
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MindgamesMode {
    PatternMaster,   // Sequence recognition
    MemoryMatrix,    // Grid memorization
    NumberNinja,     // Mental math
    DailyChallenge,  // Mixed mode with date seed
}
```

### Per-Mode State Enums
```rust
// Pattern Master phases
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternPhase {
    ShowPattern,     // Display sequence
    AnswerPrompt,    // Multiple choice
}

// Memory Matrix phases
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPhase {
    Memorize,        // Show grid with cells
    Recall,          // User recreates pattern
}

// Number Ninja (always showing question)
```

### Core State Structure
```rust
pub struct MindgamesState {
    // View control
    pub view: MindgamesView,
    pub mode: Option<MindgamesMode>,
    pub selected_mode: usize,  // For mode selection
    
    // Game progress
    pub question_index: usize,
    pub total_questions: usize,
    pub correct_count: u32,
    pub score: u32,
    pub streak: u32,
    pub best_streak: u32,
    
    // Timing
    pub time_remaining: u32,
    pub tick_counter: u32,
    pub question_start_time: u32,
    
    // Feedback
    pub last_correct: bool,
    pub feedback_timer: u32,
    
    // Pattern Master state
    pub pattern_phase: PatternPhase,
    pub pattern_sequence: Vec<String>,  // Visual representation
    pub pattern_choices: [String; 4],
    pub pattern_correct: usize,
    pub pattern_selected: usize,
    pub pattern_display_timer: u32,
    
    // Memory Matrix state
    pub memory_phase: MemoryPhase,
    pub memory_grid_size: (usize, usize),  // (rows, cols)
    pub memory_filled_cells: Vec<(usize, usize)>,  // Correct positions
    pub memory_player_cells: Vec<(usize, usize)>,  // Player guesses
    pub memory_cursor: (usize, usize),
    pub memory_display_timer: u32,
    
    // Number Ninja state
    pub number_equation: String,
    pub number_choices: [i32; 4],
    pub number_correct: usize,
    pub number_selected: usize,
    
    // Daily Challenge
    pub daily_seed: u64,  // Date-based seed
    pub daily_date: String,  // e.g., "2026-01-13"
    
    // RNG state (seeded for daily)
    rng: Option<StdRng>,  // None for non-daily, Some for daily
    
    // Events
    pending_events: Vec<GameEvent>,
}
```

## 2. File Organization

### New Files to Create
- `src/plugins/games/mindgames.rs` - Core game logic and state
- `src/plugins/games/modal/mindgames.rs` - Rendering for all views

### Modified Files
- `src/plugins/games/mod.rs` - Add `pub mod mindgames;` and register in dispatcher
- `src/plugins/games/state.rs` - Add `GameType::Mindgames` to enum, leaderboard, and impl
- `src/plugins/games/modal/mod.rs` - Add `mod mindgames;` and `pub use mindgames::draw_mindgames;`

## 3. Pattern Generation Algorithms

### Pattern Master
```rust
enum PatternType {
    Arithmetic,      // 2, 4, 6, 8, ? -> 10
    Geometric,       // 2, 4, 8, 16, ? -> 32
    Fibonacci,       // 1, 1, 2, 3, 5, ? -> 8
    Doubling,        // 3, 6, 12, 24, ? -> 48
    PlusTwo,         // 5, 7, 9, 11, ? -> 13
    AlternatingOps,  // 1, 3, 2, 4, 3, ? -> 5 (add 2, sub 1)
    Squares,         // 1, 4, 9, 16, ? -> 25
    Primes,          // 2, 3, 5, 7, 11, ? -> 13
}

fn generate_pattern(difficulty: u32, rng: &mut impl Rng) -> (Vec<String>, String, [String; 4]) {
    let pattern_type = select_random_pattern_type(rng);
    let sequence = generate_sequence(pattern_type, difficulty);
    let answer = sequence.last().unwrap().clone();
    let distractors = generate_distractors(&sequence, &answer, rng);
    let choices = shuffle_with_answer(answer, distractors, rng);
    (sequence[..sequence.len()-1].to_vec(), answer, choices)
}
```

### Memory Matrix
```rust
fn generate_memory_grid(difficulty: u32, rng: &mut impl Rng) -> (usize, usize, Vec<(usize, usize)>) {
    // difficulty 1-5 maps to grid size and cell count
    let (rows, cols, cell_count) = match difficulty {
        1 => (3, 3, 4),   // 3x3 grid, 4 cells
        2 => (3, 3, 5),   // 3x3 grid, 5 cells
        3 => (4, 4, 6),   // 4x4 grid, 6 cells
        4 => (4, 4, 8),   // 4x4 grid, 8 cells
        _ => (5, 5, 10),  // 5x5 grid, 10 cells
    };
    
    // Generate random cell positions
    let mut cells = HashSet::new();
    while cells.len() < cell_count {
        let row = rng.gen_range(0..rows);
        let col = rng.gen_range(0..cols);
        cells.insert((row, col));
    }
    
    (rows, cols, cells.into_iter().collect())
}
```

### Number Ninja
```rust
enum Operation { Add, Sub, Mul }

fn generate_equation(difficulty: u32, rng: &mut impl Rng) -> (String, i32, [i32; 4]) {
    let (op, max_num) = match difficulty {
        1 => (Operation::Add, 20),
        2 => (Operation::Sub, 50),
        3 => (Operation::Mul, 12),
        4 => (Operation::Add, 100),  // Mixed larger
        _ => (Operation::Mul, 20),   // Mixed all ops
    };
    
    let (a, b) = (rng.gen_range(1..=max_num), rng.gen_range(1..=max_num));
    let (equation, answer) = match op {
        Operation::Add => (format!("{} + {}", a, b), a + b),
        Operation::Sub => {
            let (big, small) = (a.max(b), a.min(b));
            (format!("{} - {}", big, small), big - small)
        }
        Operation::Mul => (format!("{} × {}", a, b), a * b),
    };
    
    let distractors = generate_number_distractors(answer, rng);
    let choices = shuffle_number_choices(answer, distractors, rng);
    
    (equation, answer, choices)
}
```

## 4. Daily Challenge Seeding

```rust
use chrono::Local;
use rand::{SeedableRng, rngs::StdRng};

fn get_daily_seed() -> (u64, String) {
    let now = Local::now();
    let date_str = now.format("%Y-%m-%d").to_string();
    
    // Convert date to seed: hash or simple numeric conversion
    let seed = hash_string(&date_str);  // Or use date components as u64
    
    (seed, date_str)
}

fn start_daily_challenge(&mut self) {
    let (seed, date) = get_daily_seed();
    self.daily_seed = seed;
    self.daily_date = date;
    self.rng = Some(StdRng::seed_from_u64(seed));
    
    // Generate mixed questions: 3 Pattern + 3 Memory + 4 Number
    self.total_questions = 10;
    // ... generate all questions upfront using seeded RNG
}
```

## 5. Transition Flow

```
ModeSelect:
  - Arrow keys: select mode
  - Enter: start_game() -> view = Playing

Playing:
  PatternMaster:
    - ShowPattern phase: display timer counts down
    - AnswerPrompt phase: arrow keys select, Enter submits
    - On answer: check_answer() -> view = Feedback
  
  MemoryMatrix:
    - Memorize phase: display timer counts down -> Recall phase
    - Recall phase: arrow keys move cursor, Space toggles cell, Enter submits
    - On submit: check_memory() -> view = Feedback
  
  NumberNinja:
    - Arrow keys select answer
    - Enter submits
    - On answer: check_answer() -> view = Feedback

Feedback:
  - Timer counts down (2 seconds)
  - Show correct/wrong, score gained, streak
  - Auto-advance: next_question() or game_over()

GameOver:
  - Show final stats
  - Enter: return to ModeSelect
  - Esc: close modal
```

## 6. Key Handling Per View

```rust
impl GameEngine for MindgamesState {
    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match self.view {
            MindgamesView::ModeSelect => {
                match key.code {
                    KeyCode::Up => { self.mode_select_prev(); Handled }
                    KeyCode::Down => { self.mode_select_next(); Handled }
                    KeyCode::Enter => { self.start_game(); Handled }
                    KeyCode::Esc => RequestQuit,
                    _ => NotHandled,
                }
            }
            MindgamesView::Playing => {
                match self.mode {
                    Some(MindgamesMode::PatternMaster) => self.handle_pattern_key(key),
                    Some(MindgamesMode::MemoryMatrix) => self.handle_memory_key(key),
                    Some(MindgamesMode::NumberNinja) => self.handle_number_key(key),
                    Some(MindgamesMode::DailyChallenge) => self.handle_daily_key(key),
                    None => NotHandled,
                }
            }
            MindgamesView::Feedback => NotHandled,  // Auto-advances
            MindgamesView::GameOver => {
                match key.code {
                    KeyCode::Enter | KeyCode::Char('r') => { self.reset(); Handled }
                    KeyCode::Esc => RequestQuit,
                    _ => NotHandled,
                }
            }
        }
    }
}
```

## 7. Score Calculation Formula

### Base Scoring
```rust
const BASE_SCORE: u32 = 100;

fn calculate_score(mode: MindgamesMode, time_remaining: u32, max_time: u32) -> u32 {
    let base = BASE_SCORE;
    
    // Time bonus
    let time_bonus = match mode {
        MindgamesMode::PatternMaster => {
            if time_remaining > 10 { 50 } 
            else if time_remaining > 5 { 25 } 
            else { 0 }
        }
        MindgamesMode::MemoryMatrix => 0,  // No time bonus for memory
        MindgamesMode::NumberNinja => {
            if time_remaining > 5 { 25 } else { 0 }
        }
        MindgamesMode::DailyChallenge => {
            // Mixed bonus based on question type
            determine_current_question_type_bonus(...)
        }
    };
    
    base + time_bonus
}

fn apply_streak_multiplier(score: u32, streak: u32) -> u32 {
    let multiplier = if streak >= 5 { 2.0 }
                     else if streak >= 3 { 1.5 }
                     else { 1.0 };
    (score as f64 * multiplier) as u32
}
```

### Final Score
```rust
fn final_score(&self) -> u32 {
    let mut total = self.score;
    
    // Perfect bonus
    if self.correct_count == self.total_questions as u32 {
        total = total * 2;
    }
    
    total
}
```

## 8. Rendering Strategy

### Main Dispatcher (modal/mindgames.rs)
```rust
pub fn draw_mindgames(frame: &mut Frame, view: &FullScreenView, state: &MindgamesState, colors: &ThemeColors) {
    match state.view {
        MindgamesView::ModeSelect => draw_mode_select(frame, view, state, colors),
        MindgamesView::Playing => {
            match state.mode {
                Some(MindgamesMode::PatternMaster) => draw_pattern(frame, view, state, colors),
                Some(MindgamesMode::MemoryMatrix) => draw_memory(frame, view, state, colors),
                Some(MindgamesMode::NumberNinja) => draw_number(frame, view, state, colors),
                Some(MindgamesMode::DailyChallenge) => draw_daily(frame, view, state, colors),
                None => {}
            }
        }
        MindgamesView::Feedback => draw_feedback(frame, view, state, colors),
        MindgamesView::GameOver => draw_game_over(frame, view, state, colors),
    }
}
```

### Sub-Renderers
- `draw_mode_select`: Title, 4 mode options, descriptions, help footer
- `draw_pattern`: Question #, score, streak, timer bar, sequence display, 4 choices
- `draw_memory`: Question #, score, grid (filled during Memorize, clickable during Recall), help
- `draw_number`: Question #, score, streak, timer bar, equation, 4 choices
- `draw_daily`: Shows daily date, seed, mixed question type indicators, delegates to specific renderer
- `draw_feedback`: Correct/Wrong banner, score gained, streak info, fun fact or explanation
- `draw_game_over`: Final score, accuracy %, best streak, rating (GENIUS/Excellent/Good)

## 9. Integration Steps

### Step 1: Add to GameType enum (state.rs)
```rust
pub enum GameType {
    // ... existing games
    Mindgames,
}

impl GameType {
    pub fn all() -> &'static [GameType] {
        &[/* ... */, GameType::Mindgames]
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            // ...
            GameType::Mindgames => "Mindgames",
        }
    }
    
    pub fn description(&self) -> &'static str {
        match self {
            // ...
            GameType::Mindgames => "Brain training - patterns, memory, math",
        }
    }
}
```

### Step 2: Add leaderboard (state.rs)
```rust
pub struct Leaderboards {
    // ... existing
    #[serde(default)]
    pub mindgames: GameLeaderboard,
}

impl Leaderboards {
    pub fn get(&self, game: GameType) -> &GameLeaderboard {
        match game {
            // ...
            GameType::Mindgames => &self.mindgames,
        }
    }
    
    pub fn get_mut(&mut self, game: GameType) -> &mut GameLeaderboard {
        match game {
            // ...
            GameType::Mindgames => &mut self.mindgames,
        }
    }
}
```

### Step 3: Add to GamesState (state.rs)
```rust
pub struct GamesState {
    // ... existing game states
    pub mindgames: MindgamesState,
}

impl GamesState {
    pub fn new() -> Self {
        Self {
            // ...
            mindgames: MindgamesState::new(),
        }
    }
}
```

### Step 4: Register in game loop (mod.rs)
```rust
// In handle_modal_key, Playing view:
Some(GameType::Mindgames) => self.state.mindgames.handle_key(key),

// In tick:
Some(GameType::Mindgames) => {
    self.state.mindgames.tick();
    self.state.score = self.state.mindgames.get_score();
    if self.state.mindgames.is_game_over() {
        self.state.game_over();
    }
}

// In start_game:
GameType::Mindgames => self.mindgames.reset(),
```

### Step 5: Register renderer (modal/mod.rs)
```rust
mod mindgames;
pub use mindgames::draw_mindgames;

// In draw_game dispatcher:
Some(GameType::Mindgames) => draw_mindgames(frame, view, &state.mindgames, colors),
```

## 10. Implementation Order

1. Create `mindgames.rs` with basic structure:
   - Enums (MindgamesView, MindgamesMode, phases)
   - MindgamesState struct with all fields
   - `new()`, `reset()` methods
   - Stub methods for state transitions

2. Implement pattern generation algorithms:
   - `generate_pattern()` with all PatternType variants
   - `generate_distractors()` for choices
   - Test with fixed seeds

3. Implement memory grid generation:
   - `generate_memory_grid()` with difficulty scaling
   - `check_memory_answer()` validation

4. Implement number equation generation:
   - `generate_equation()` with operations
   - `generate_number_distractors()`

5. Implement daily challenge:
   - `get_daily_seed()` from date
   - Mixed question generation
   - Seeded RNG integration

6. Implement GameEngine trait:
   - `tick()` for timers and auto-advances
   - `handle_key()` with mode dispatching
   - `get_score()`, `is_game_over()`
   - Event emission

7. Implement state transitions:
   - `start_game()` - mode setup
   - `check_answer()` - scoring logic
   - `next_question()` - progression
   - `game_over()` - final state

8. Create `modal/mindgames.rs` with all renderers:
   - Mode select screen
   - Pattern view (both phases)
   - Memory view (both phases)
   - Number view
   - Feedback screen
   - Game over screen

9. Integration:
   - Update GameType enum and impl
   - Add to Leaderboards
   - Add to GamesState
   - Register in mod.rs dispatcher
   - Register renderer in modal/mod.rs

10. Testing:
    - Test each mode independently
    - Test daily challenge with fixed dates
    - Test scoring and streak logic
    - Test leaderboard integration
    - Test all transitions

## 11. Rust Implementation Details

### Dependencies (already in Cargo.toml)
- `rand` - RNG for question generation
- `chrono` - Date handling for daily challenge
- `serde` - Serialization for state

### Key Patterns to Follow
- Use `FullScreenView` for rendering (NOT ModalFrame - it panics on full screen)
- Emit `GameEvent` to `pending_events` vec
- Drain events in `drain_events()` method
- Follow Brainiac pattern for multi-phase gameplay
- Use `tick_counter` for timing (10 ticks = 1 second)
- Store RNG as `Option<StdRng>` - None for random, Some for daily seeded

### Error Handling
- All RNG operations are infallible (no unwrap needed)
- Pattern generation should always produce valid output
- Date parsing uses chrono's safe methods

## 12. Testing Strategy

### Unit Tests (in mindgames.rs)
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    
    #[test]
    fn test_pattern_generation() {
        let mut rng = StdRng::seed_from_u64(42);
        let (seq, answer, choices) = generate_pattern(1, &mut rng);
        assert_eq!(seq.len(), 4);  // 4-item sequence
        assert!(choices.contains(&answer));
    }
    
    #[test]
    fn test_daily_seed_deterministic() {
        let seed1 = hash_string("2026-01-13");
        let seed2 = hash_string("2026-01-13");
        assert_eq!(seed1, seed2);
    }
    
    #[test]
    fn test_score_calculation() {
        let score = calculate_score(MindgamesMode::PatternMaster, 12, 15);
        assert_eq!(score, 150);  // 100 base + 50 time bonus
    }
}
```

### Manual Testing
- Play each mode to completion
- Verify daily challenge uses same seed on same day
- Test streak bonuses at 3x and 5x
- Test perfect game bonus (2x multiplier)
- Verify leaderboard saves and loads

---

## Summary

MINDGAMES will be a pure algorithmic brain training game with 4 modes, following the established R-DOS game patterns (similar to Brainiac's multi-view structure and DopeWars' view enum pattern). Key innovations:

1. **Algorithmic Content** - No AI/API calls, all patterns/questions generated deterministically
2. **Daily Challenge** - Date-seeded RNG for fair competition
3. **Multi-Mode** - 3 core modes + mixed daily mode
4. **Progressive Difficulty** - Scales within session
5. **Local Leaderboard** - Persisted scores per mode

The implementation follows existing patterns:
- GameEngine trait for platform integration
- View enum for state machine
- FullScreenView for rendering
- Event emission for stats tracking
- Leaderboard integration for persistence
