//! JUNGLE RUN - Pitfall Adventure
//!
//! Side-scrolling platformer inspired by Pitfall!
//! Jump over pits, avoid crocodiles and logs, collect treasures.

use super::platform::{GameEngine, GameEvent, KeyHandleResult};
use crossterm::event::{KeyCode, KeyEvent};
use rand::Rng;

// =============================================================================
// CONSTANTS
// =============================================================================

const NUM_SCREENS: usize = 32;
const SCREEN_WIDTH: usize = 40;
const GROUND_Y: f32 = 10.0;
const GRAVITY: f32 = 0.8;
const JUMP_VELOCITY: f32 = -6.0;
const MOVE_SPEED: f32 = 0.5;
const STARTING_LIVES: u8 = 3;
const STARTING_TIME: u32 = 3000; // 5 minutes at 10Hz
const PIT_DEATH_Y: f32 = 15.0;

// Treasure values
const GOLD_BAR_POINTS: u32 = 200;
const DIAMOND_POINTS: u32 = 500;

// Hazard animation
const CROC_CYCLE: u32 = 40; // Ticks for one croc cycle
const LOG_SPEED: f32 = 0.3;

// =============================================================================
// ENUMS
// =============================================================================

/// Current view state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JungleRunView {
    #[default]
    Menu,
    Playing,
    Paused,
    GameOver,
}

/// Player animation state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayerState {
    #[default]
    Idle,
    Running,
    Jumping,
    Falling,
    Dead,
}

/// Types of hazards
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HazardType {
    Pit,        // Gap in ground
    Crocodile,  // In water, cycles open/closed
    RollingLog, // Moves horizontally
}

/// Types of treasures
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreasureType {
    GoldBar,
    Diamond,
}

impl TreasureType {
    pub fn points(&self) -> u32 {
        match self {
            TreasureType::GoldBar => GOLD_BAR_POINTS,
            TreasureType::Diamond => DIAMOND_POINTS,
        }
    }

    pub fn char(&self) -> char {
        match self {
            TreasureType::GoldBar => '◊',
            TreasureType::Diamond => '♦',
        }
    }
}

// =============================================================================
// HAZARD
// =============================================================================

/// A hazard on a screen
#[derive(Debug, Clone)]
pub struct Hazard {
    pub x: f32,
    pub width: f32,
    pub hazard_type: HazardType,
    pub log_offset: f32, // For rolling logs
}

impl Hazard {
    pub fn pit(x: f32, width: f32) -> Self {
        Self {
            x,
            width,
            hazard_type: HazardType::Pit,
            log_offset: 0.0,
        }
    }

    pub fn crocodile(x: f32) -> Self {
        Self {
            x,
            width: 4.0,
            hazard_type: HazardType::Crocodile,
            log_offset: 0.0,
        }
    }

    pub fn rolling_log(x: f32) -> Self {
        Self {
            x,
            width: 3.0,
            hazard_type: HazardType::RollingLog,
            log_offset: 0.0,
        }
    }
}

// =============================================================================
// TREASURE
// =============================================================================

/// A treasure on a screen
#[derive(Debug, Clone)]
pub struct Treasure {
    pub x: f32,
    pub y: f32,
    pub treasure_type: TreasureType,
    pub collected: bool,
}

impl Treasure {
    pub fn new(x: f32, y: f32, treasure_type: TreasureType) -> Self {
        Self {
            x,
            y,
            treasure_type,
            collected: false,
        }
    }
}

// =============================================================================
// SCREEN
// =============================================================================

/// A single screen in the game
#[derive(Debug, Clone, Default)]
pub struct Screen {
    pub hazards: Vec<Hazard>,
    pub treasures: Vec<Treasure>,
}

impl Screen {
    pub fn new() -> Self {
        Self::default()
    }

    /// Generate a screen with random hazards and treasures
    pub fn generate(screen_num: usize, rng: &mut impl Rng) -> Self {
        let mut screen = Screen::new();

        // Seed randomization based on screen number for consistency
        let complexity = (screen_num as f32 / NUM_SCREENS as f32).min(0.8);

        // Add 1-3 hazards based on complexity
        let num_hazards = 1 + (complexity * 2.5) as usize;

        for i in 0..num_hazards {
            let base_x = 8.0 + (i as f32 * 10.0) + rng.gen_range(0.0..5.0);

            match rng.gen_range(0..3) {
                0 => {
                    // Pit
                    let width = 3.0 + rng.gen_range(0.0..3.0);
                    screen.hazards.push(Hazard::pit(base_x, width));
                }
                1 => {
                    // Crocodile
                    screen.hazards.push(Hazard::crocodile(base_x));
                }
                _ => {
                    // Rolling log
                    screen.hazards.push(Hazard::rolling_log(base_x));
                }
            }
        }

        // Add 1-2 treasures
        let num_treasures = 1 + rng.gen_range(0..2);
        for _ in 0..num_treasures {
            let x = rng.gen_range(5.0..35.0);
            let y = GROUND_Y - 3.0 - rng.gen_range(0.0..2.0);
            let treasure_type = if rng.gen_bool(0.3) {
                TreasureType::Diamond
            } else {
                TreasureType::GoldBar
            };
            screen.treasures.push(Treasure::new(x, y, treasure_type));
        }

        screen
    }

    /// Check if position is over a pit
    pub fn is_over_pit(&self, x: f32) -> bool {
        for hazard in &self.hazards {
            if hazard.hazard_type == HazardType::Pit
                && x >= hazard.x
                && x <= hazard.x + hazard.width
            {
                return true;
            }
        }
        false
    }
}

// =============================================================================
// GAME STATE
// =============================================================================

/// Main game state
pub struct JungleRunState {
    // View state
    pub view: JungleRunView,

    // Player state
    pub player_x: f32,
    pub player_y: f32,
    pub velocity_y: f32,
    pub player_state: PlayerState,
    pub facing_right: bool,

    // World state
    pub current_screen: usize,
    pub screens: Vec<Screen>,

    // Game stats
    pub score: u32,
    pub lives: u8,
    pub time_remaining: u32,
    pub treasures_collected: u32,

    // Animation
    pub tick_count: u32,
    pub run_frame: u8,

    // State flags
    pub game_over: bool,
    pub game_won: bool,

    // Messages
    pub message: Option<String>,
    pub message_timer: u32,

    // Events
    pending_events: Vec<GameEvent>,
}

impl Default for JungleRunState {
    fn default() -> Self {
        Self::new()
    }
}

impl JungleRunState {
    pub fn new() -> Self {
        Self {
            view: JungleRunView::Menu,
            player_x: 5.0,
            player_y: GROUND_Y,
            velocity_y: 0.0,
            player_state: PlayerState::Idle,
            facing_right: true,
            current_screen: 0,
            screens: Vec::new(),
            score: 0,
            lives: STARTING_LIVES,
            time_remaining: STARTING_TIME,
            treasures_collected: 0,
            tick_count: 0,
            run_frame: 0,
            game_over: false,
            game_won: false,
            message: None,
            message_timer: 0,
            pending_events: Vec::new(),
        }
    }

    /// Start a new game
    pub fn start_game(&mut self) {
        self.view = JungleRunView::Playing;
        self.player_x = 5.0;
        self.player_y = GROUND_Y;
        self.velocity_y = 0.0;
        self.player_state = PlayerState::Idle;
        self.facing_right = true;
        self.current_screen = 0;
        self.score = 0;
        self.lives = STARTING_LIVES;
        self.time_remaining = STARTING_TIME;
        self.treasures_collected = 0;
        self.tick_count = 0;
        self.run_frame = 0;
        self.game_over = false;
        self.game_won = false;
        self.message = None;

        // Generate all screens
        let mut rng = rand::thread_rng();
        self.screens.clear();
        for i in 0..NUM_SCREENS {
            self.screens.push(Screen::generate(i, &mut rng));
        }
    }

    /// Move player left
    fn move_left(&mut self) {
        self.facing_right = false;
        if self.player_state != PlayerState::Dead {
            self.player_x -= MOVE_SPEED;
            if self.player_x < 0.0 {
                // Go to previous screen
                if self.current_screen > 0 {
                    self.current_screen -= 1;
                    self.player_x = SCREEN_WIDTH as f32 - 1.0;
                } else {
                    self.player_x = 0.0;
                }
            }
            if self.player_state == PlayerState::Idle {
                self.player_state = PlayerState::Running;
            }
        }
    }

    /// Move player right
    fn move_right(&mut self) {
        self.facing_right = true;
        if self.player_state != PlayerState::Dead {
            self.player_x += MOVE_SPEED;
            if self.player_x >= SCREEN_WIDTH as f32 {
                // Go to next screen
                if self.current_screen < NUM_SCREENS - 1 {
                    self.current_screen += 1;
                    self.player_x = 1.0;
                    self.score += 100; // Screen completion bonus
                    self.show_message("+100 Screen Clear!");
                } else {
                    // Won the game!
                    self.game_won = true;
                    self.game_over = true;
                    let time_bonus = self.time_remaining / 10;
                    self.score += time_bonus;
                    self.view = JungleRunView::GameOver;
                }
            }
            if self.player_state == PlayerState::Idle {
                self.player_state = PlayerState::Running;
            }
        }
    }

    /// Initiate a jump
    fn jump(&mut self) {
        if self.player_state == PlayerState::Idle || self.player_state == PlayerState::Running {
            self.velocity_y = JUMP_VELOCITY;
            self.player_state = PlayerState::Jumping;
        }
    }

    /// Show a temporary message
    fn show_message(&mut self, msg: &str) {
        self.message = Some(msg.to_string());
        self.message_timer = 20;
    }

    /// Kill the player
    fn die(&mut self) {
        if self.player_state == PlayerState::Dead {
            return;
        }

        self.player_state = PlayerState::Dead;
        self.lives = self.lives.saturating_sub(1);

        if self.lives == 0 {
            self.game_over = true;
            self.view = JungleRunView::GameOver;
        } else {
            // Respawn after a delay (handled in tick)
            self.message = Some(format!("Lives: {}", self.lives));
            self.message_timer = 30;
        }
    }

    /// Respawn the player
    fn respawn(&mut self) {
        self.player_x = 5.0;
        self.player_y = GROUND_Y;
        self.velocity_y = 0.0;
        self.player_state = PlayerState::Idle;
    }

    /// Update physics and game state
    fn update_physics(&mut self) {
        if self.player_state == PlayerState::Dead {
            return;
        }

        let screen = &self.screens[self.current_screen];
        let over_pit = screen.is_over_pit(self.player_x);

        // Apply gravity
        if self.player_y < GROUND_Y || over_pit {
            self.velocity_y += GRAVITY;
            self.player_y += self.velocity_y;

            if self.velocity_y > 0.0 {
                self.player_state = PlayerState::Falling;
            }

            // Fell into pit
            if self.player_y > PIT_DEATH_Y {
                self.die();
                return;
            }
        } else {
            // On ground
            if self.player_state == PlayerState::Falling
                || self.player_state == PlayerState::Jumping
            {
                self.player_y = GROUND_Y;
                self.velocity_y = 0.0;
                self.player_state = PlayerState::Idle;
            }
        }

        // Landing on ground (not over pit)
        if !over_pit && self.player_y >= GROUND_Y && self.velocity_y >= 0.0 {
            self.player_y = GROUND_Y;
            self.velocity_y = 0.0;
            if self.player_state == PlayerState::Falling {
                self.player_state = PlayerState::Idle;
            }
        }
    }

    /// Check collisions with hazards
    fn check_hazards(&mut self) {
        if self.player_state == PlayerState::Dead {
            return;
        }

        let tick = self.tick_count;
        let screen = &self.screens[self.current_screen];

        for hazard in &screen.hazards {
            match hazard.hazard_type {
                HazardType::Pit => {
                    // Handled in physics
                }
                HazardType::Crocodile => {
                    // Crocodile cycles open/closed
                    let cycle_pos = (tick % CROC_CYCLE) as f32 / CROC_CYCLE as f32;
                    let is_open = cycle_pos > 0.5;

                    // Check if player is in the water area and croc is open
                    if is_open
                        && self.player_y >= GROUND_Y - 1.0
                        && self.player_x >= hazard.x
                        && self.player_x <= hazard.x + hazard.width
                    {
                        self.die();
                        return;
                    }
                }
                HazardType::RollingLog => {
                    // Log moves back and forth
                    let log_x =
                        hazard.x + ((tick as f32 * LOG_SPEED) % 10.0) - 5.0 + hazard.log_offset;

                    // Check collision
                    if self.player_y >= GROUND_Y - 2.0
                        && self.player_x >= log_x - 1.0
                        && self.player_x <= log_x + hazard.width + 1.0
                    {
                        self.die();
                        return;
                    }
                }
            }
        }
    }

    /// Check treasure collection
    fn check_treasures(&mut self) {
        // Find treasures to collect without borrowing self mutably
        let player_x = self.player_x;
        let player_y = self.player_y;
        let screen_idx = self.current_screen;

        let mut collected_treasures: Vec<(usize, u32, char)> = Vec::new();

        if let Some(screen) = self.screens.get(screen_idx) {
            for (i, treasure) in screen.treasures.iter().enumerate() {
                if treasure.collected {
                    continue;
                }

                // Check if player is near treasure
                let dx = (player_x - treasure.x).abs();
                let dy = (player_y - treasure.y).abs();

                if dx < 2.0 && dy < 2.0 {
                    let points = treasure.treasure_type.points();
                    let ch = treasure.treasure_type.char();
                    collected_treasures.push((i, points, ch));
                }
            }
        }

        // Now process collected treasures
        for (idx, points, ch) in collected_treasures {
            if let Some(screen) = self.screens.get_mut(screen_idx) {
                if let Some(treasure) = screen.treasures.get_mut(idx) {
                    treasure.collected = true;
                }
            }
            self.score += points;
            self.treasures_collected += 1;
            self.show_message(&format!("+{} {}", points, ch));
        }
    }

    /// Get current screen
    pub fn current_screen_data(&self) -> Option<&Screen> {
        self.screens.get(self.current_screen)
    }

    /// Check if crocodile is currently open (dangerous)
    pub fn is_croc_open(&self) -> bool {
        let cycle_pos = (self.tick_count % CROC_CYCLE) as f32 / CROC_CYCLE as f32;
        cycle_pos > 0.5
    }

    /// Get log position offset for animation
    pub fn log_offset(&self) -> f32 {
        ((self.tick_count as f32 * LOG_SPEED) % 10.0) - 5.0
    }

    /// Format time remaining as MM:SS
    pub fn time_string(&self) -> String {
        let seconds = self.time_remaining / 10;
        let mins = seconds / 60;
        let secs = seconds % 60;
        format!("{}:{:02}", mins, secs)
    }
}

// =============================================================================
// GAME ENGINE IMPLEMENTATION
// =============================================================================

impl GameEngine for JungleRunState {
    fn tick(&mut self) {
        if self.view != JungleRunView::Playing {
            return;
        }

        self.tick_count += 1;

        // Update message timer
        if self.message_timer > 0 {
            self.message_timer -= 1;
            if self.message_timer == 0 {
                self.message = None;
            }
        }

        // Dead player respawn
        if self.player_state == PlayerState::Dead && self.lives > 0 && self.message_timer == 0 {
            self.respawn();
        }

        // Timer countdown
        if self.time_remaining > 0 {
            self.time_remaining -= 1;
            if self.time_remaining == 0 {
                self.game_over = true;
                self.view = JungleRunView::GameOver;
                return;
            }
        }

        // Update animation frame
        if self.tick_count.is_multiple_of(5) {
            self.run_frame = (self.run_frame + 1) % 4;
        }

        // Update physics
        self.update_physics();

        // Check hazards
        self.check_hazards();

        // Check treasures
        self.check_treasures();

        // Auto-stop running if not moving
        if self.player_state == PlayerState::Running {
            self.player_state = PlayerState::Idle;
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match self.view {
            JungleRunView::Menu => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.start_game();
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => KeyHandleResult::RequestQuit,
                _ => KeyHandleResult::Handled,
            },
            JungleRunView::Playing => match key.code {
                KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') => {
                    self.move_left();
                    KeyHandleResult::Handled
                }
                KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('D') => {
                    self.move_right();
                    KeyHandleResult::Handled
                }
                KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') | KeyCode::Char(' ') => {
                    self.jump();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('p') | KeyCode::Char('P') => {
                    self.view = JungleRunView::Paused;
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => {
                    self.view = JungleRunView::Menu;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            JungleRunView::Paused => match key.code {
                KeyCode::Char('p') | KeyCode::Char('P') | KeyCode::Enter | KeyCode::Char(' ') => {
                    self.view = JungleRunView::Playing;
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => {
                    self.view = JungleRunView::Menu;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            JungleRunView::GameOver => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.start_game();
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => KeyHandleResult::RequestQuit,
                _ => KeyHandleResult::Handled,
            },
        }
    }

    fn get_score(&self) -> u32 {
        self.score
    }

    fn is_game_over(&self) -> bool {
        self.game_over
    }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.pending_events)
    }
}
