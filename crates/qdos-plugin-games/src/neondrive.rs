//! NEON DRIVE - Cyberpunk Racing
//!
//! Outrun-style racing through a neon-lit dystopian city.
//! Dodge traffic, use nitro, survive as long as possible.

use super::platform::{GameEngine, GameEvent, KeyHandleResult};
use crossterm::event::{KeyCode, KeyEvent};
use rand::Rng as _;

// =============================================================================
// CONSTANTS
// =============================================================================

const NUM_LANES: usize = 5;
const MIN_SPEED: f32 = 100.0;
const MAX_SPEED: f32 = 300.0;
const NITRO_MAX_SPEED: f32 = 350.0;
const MAX_NITRO: f32 = 3.0;
const NITRO_DURATION: u32 = 50; // ticks (~5 seconds)
const NITRO_RECHARGE_RATE: f32 = 0.003; // per tick
const ACCEL_RATE: f32 = 5.0;
const BRAKE_RATE: f32 = 10.0;
const COAST_RATE: f32 = 2.0;
const LANE_CHANGE_SPEED: f32 = 0.15;
const COLLISION_DISTANCE: f32 = 5.0;
const SPAWN_DISTANCE: f32 = 100.0;
const BASE_SPAWN_INTERVAL: u32 = 30;

// =============================================================================
// ENUMS
// =============================================================================

/// Current view state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NeondriveView {
    #[default]
    Menu,
    Playing,
    GameOver,
}

/// Types of obstacles on the road
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObstacleKind {
    Car,     // Normal traffic - 2 wide
    Van,     // Slow, wide - 3 wide
    Barrier, // Stationary debris
}

impl ObstacleKind {
    /// Get the relative speed of this obstacle (negative = moving same direction)
    pub fn relative_speed(&self) -> f32 {
        match self {
            ObstacleKind::Car => -0.3,    // Moving same direction, slower
            ObstacleKind::Van => -0.15,   // Moving same direction, very slow
            ObstacleKind::Barrier => 0.0, // Stationary
        }
    }

    /// Get the width of this obstacle in lanes
    pub fn width(&self) -> usize {
        match self {
            ObstacleKind::Car => 1,
            ObstacleKind::Van => 1,
            ObstacleKind::Barrier => 1,
        }
    }
}

// =============================================================================
// OBSTACLE
// =============================================================================

/// An obstacle on the road
#[derive(Debug, Clone)]
pub struct Obstacle {
    pub lane: usize,
    pub distance: f32, // 0.0 = at player, 100.0 = horizon
    pub kind: ObstacleKind,
}

impl Obstacle {
    pub fn new(lane: usize, kind: ObstacleKind) -> Self {
        Self {
            lane,
            distance: SPAWN_DISTANCE,
            kind,
        }
    }
}

// =============================================================================
// GAME STATE
// =============================================================================

/// Main game state
pub struct NeondriveState {
    // View state
    pub view: NeondriveView,

    // Player state
    pub lane: usize,
    pub lane_offset: f32,   // -1.0 to 1.0 for smooth transitions
    pub target_lane: usize, // Lane we're moving toward
    pub speed: f32,         // Current speed in kph
    pub nitro: f32,         // Nitro charges (0.0 to 3.0)
    pub nitro_active: bool,
    pub nitro_timer: u32, // Ticks remaining on current nitro

    // Game state
    pub distance: f32, // Total distance traveled
    pub score: u32,
    pub heat: u32, // 0-5 heat level (visual)
    pub game_over: bool,

    // Obstacles
    pub obstacles: Vec<Obstacle>,
    pub spawn_timer: u32,

    // Animation
    pub tick_count: u32,
    pub road_offset: f32, // For scrolling road markings

    // Input state (for held keys)
    pub accelerating: bool,
    pub braking: bool,
    pub steering_left: bool,
    pub steering_right: bool,

    // Events
    pending_events: Vec<GameEvent>,
}

impl Default for NeondriveState {
    fn default() -> Self {
        Self::new()
    }
}

impl NeondriveState {
    pub fn new() -> Self {
        Self {
            view: NeondriveView::Menu,
            lane: 2, // Start in center lane
            lane_offset: 0.0,
            target_lane: 2,
            speed: MIN_SPEED,
            nitro: MAX_NITRO, // Start with full nitro
            nitro_active: false,
            nitro_timer: 0,
            distance: 0.0,
            score: 0,
            heat: 0,
            game_over: false,
            obstacles: Vec::new(),
            spawn_timer: BASE_SPAWN_INTERVAL,
            tick_count: 0,
            road_offset: 0.0,
            accelerating: false,
            braking: false,
            steering_left: false,
            steering_right: false,
            pending_events: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.view = NeondriveView::Playing;
        self.lane = 2;
        self.lane_offset = 0.0;
        self.target_lane = 2;
        self.speed = MIN_SPEED;
        self.nitro = MAX_NITRO;
        self.nitro_active = false;
        self.nitro_timer = 0;
        self.distance = 0.0;
        self.score = 0;
        self.heat = 0;
        self.game_over = false;
        self.obstacles.clear();
        self.spawn_timer = BASE_SPAWN_INTERVAL;
        self.tick_count = 0;
        self.road_offset = 0.0;
        self.accelerating = false;
        self.braking = false;
        self.steering_left = false;
        self.steering_right = false;
    }

    // =========================================================================
    // GAME LOGIC
    // =========================================================================

    fn update_speed(&mut self) {
        let max = if self.nitro_active {
            NITRO_MAX_SPEED
        } else {
            MAX_SPEED
        };

        if self.accelerating {
            self.speed = (self.speed + ACCEL_RATE).min(max);
        } else if self.braking {
            self.speed = (self.speed - BRAKE_RATE).max(MIN_SPEED);
        } else {
            // Coast - gradually slow down
            self.speed = (self.speed - COAST_RATE).max(MIN_SPEED);
        }
    }

    fn update_nitro(&mut self) {
        // Handle active nitro
        if self.nitro_active {
            if self.nitro_timer > 0 {
                self.nitro_timer -= 1;
            } else {
                self.nitro_active = false;
            }
        }

        // Recharge nitro when not active
        if !self.nitro_active && self.nitro < MAX_NITRO {
            self.nitro = (self.nitro + NITRO_RECHARGE_RATE).min(MAX_NITRO);
        }
    }

    fn activate_nitro(&mut self) {
        if self.nitro >= 1.0 && !self.nitro_active {
            self.nitro -= 1.0;
            self.nitro_active = true;
            self.nitro_timer = NITRO_DURATION;
        }
    }

    fn update_lane(&mut self) {
        // Smooth transition toward target lane
        if self.lane != self.target_lane {
            if self.target_lane > self.lane {
                self.lane_offset += LANE_CHANGE_SPEED;
                if self.lane_offset >= 1.0 {
                    self.lane += 1;
                    self.lane_offset -= 1.0;
                    if self.lane == self.target_lane {
                        self.lane_offset = 0.0;
                    }
                }
            } else {
                self.lane_offset -= LANE_CHANGE_SPEED;
                if self.lane_offset <= -1.0 {
                    self.lane -= 1;
                    self.lane_offset += 1.0;
                    if self.lane == self.target_lane {
                        self.lane_offset = 0.0;
                    }
                }
            }
        } else {
            // Settle back to center of lane
            if self.lane_offset > 0.01 {
                self.lane_offset -= LANE_CHANGE_SPEED;
                if self.lane_offset < 0.0 {
                    self.lane_offset = 0.0;
                }
            } else if self.lane_offset < -0.01 {
                self.lane_offset += LANE_CHANGE_SPEED;
                if self.lane_offset > 0.0 {
                    self.lane_offset = 0.0;
                }
            }
        }
    }

    fn steer_left(&mut self) {
        if self.target_lane > 0 {
            self.target_lane -= 1;
        }
    }

    fn steer_right(&mut self) {
        if self.target_lane < NUM_LANES - 1 {
            self.target_lane += 1;
        }
    }

    fn update_obstacles(&mut self) {
        // Move obstacles based on speed differential
        let speed_factor = self.speed / 100.0;

        for obstacle in &mut self.obstacles {
            // Obstacle moves toward player (decreasing distance)
            // Speed factor makes obstacles approach faster at higher speeds
            let relative_speed = 1.0 + obstacle.kind.relative_speed();
            obstacle.distance -= speed_factor * relative_speed;
        }

        // Remove obstacles that have passed the player
        self.obstacles.retain(|o| o.distance > -10.0);
    }

    fn spawn_obstacles(&mut self) {
        self.spawn_timer = self.spawn_timer.saturating_sub(1);

        if self.spawn_timer == 0 {
            // Reset timer - spawns get more frequent as distance increases
            let difficulty = (self.distance / 1000.0).min(2.0);
            let interval = (BASE_SPAWN_INTERVAL as f32 / (1.0 + difficulty * 0.5)) as u32;
            self.spawn_timer = interval.max(10);

            // Random lane (weighted away from player)
            let mut rng = rand::thread_rng();
            let lane = loop {
                let l = rng.gen_range(0..NUM_LANES);
                // 70% chance to accept, 100% if not adjacent to player
                let diff = (l as i32 - self.lane as i32).unsigned_abs() as usize;
                if diff > 1 || rng.gen_ratio(7, 10) {
                    break l;
                }
            };

            // Random obstacle type
            let roll: u32 = rng.gen_range(0..100);
            let kind = if roll < 70 {
                ObstacleKind::Car
            } else if roll < 90 {
                ObstacleKind::Van
            } else {
                ObstacleKind::Barrier
            };

            self.obstacles.push(Obstacle::new(lane, kind));
        }
    }

    fn check_collisions(&mut self) {
        let player_lane = self.lane as f32 + self.lane_offset;

        for obstacle in &self.obstacles {
            // Check if obstacle is at player's position
            if obstacle.distance < COLLISION_DISTANCE && obstacle.distance > -2.0 {
                // Check lane overlap
                let obstacle_lane = obstacle.lane as f32;
                let width = obstacle.kind.width() as f32;

                // Player occupies roughly 0.8 of a lane
                let player_left = player_lane - 0.4;
                let player_right = player_lane + 0.4;
                let obs_left = obstacle_lane - width / 2.0;
                let obs_right = obstacle_lane + width / 2.0;

                if player_right > obs_left && player_left < obs_right {
                    self.game_over = true;
                    self.view = NeondriveView::GameOver;
                    return;
                }
            }
        }
    }

    fn update_score(&mut self) {
        // Score based on distance and speed
        let speed_multiplier = self.speed / 100.0;
        self.distance += speed_multiplier;
        self.score = (self.distance * speed_multiplier) as u32;

        // Update heat based on speed (visual only for MVP)
        self.heat = match self.speed as u32 {
            0..=149 => 0,
            150..=199 => 1,
            200..=249 => 2,
            250..=299 => 3,
            300..=349 => 4,
            _ => 5,
        };
    }

    fn update_animation(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);

        // Scroll road markings based on speed
        self.road_offset += self.speed / 50.0;
        if self.road_offset >= 10.0 {
            self.road_offset -= 10.0;
        }
    }
}

// =============================================================================
// GAME ENGINE TRAIT
// =============================================================================

impl GameEngine for NeondriveState {
    fn tick(&mut self) {
        // Always increment for menu animation
        self.tick_count = self.tick_count.wrapping_add(1);

        if self.view != NeondriveView::Playing || self.game_over {
            return;
        }

        // Update game state
        self.update_speed();
        self.update_nitro();
        self.update_lane();
        self.update_obstacles();
        self.spawn_obstacles();
        self.check_collisions();
        self.update_score();
        self.update_animation();
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match self.view {
            NeondriveView::Menu => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.reset();
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => KeyHandleResult::RequestQuit,
                _ => KeyHandleResult::Handled,
            },

            NeondriveView::Playing => match key.code {
                KeyCode::Esc => KeyHandleResult::RequestQuit,
                KeyCode::Char('p') | KeyCode::Char('P') => KeyHandleResult::RequestPause,

                // Steering
                KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') => {
                    self.steer_left();
                    KeyHandleResult::Handled
                }
                KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('D') => {
                    self.steer_right();
                    KeyHandleResult::Handled
                }

                // Speed control
                KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                    self.accelerating = true;
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.braking = true;
                    KeyHandleResult::Handled
                }

                // Nitro
                KeyCode::Char(' ') => {
                    self.activate_nitro();
                    KeyHandleResult::Handled
                }

                _ => KeyHandleResult::Handled,
            },

            NeondriveView::GameOver => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.reset();
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

    fn is_game_won(&self) -> bool {
        false // Endless game
    }

    fn get_level(&self) -> Option<u32> {
        Some((self.distance / 1000.0) as u32 + 1)
    }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.pending_events)
    }
}
