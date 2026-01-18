//! Dino Jump game implementation
//!
//! Endless runner - jump over obstacles Chrome dino style.

use super::platform::{GameEngine, GameEvent, KeyHandleResult};
use crossterm::event::{KeyCode, KeyEvent};
use rand::Rng;

pub const GROUND_Y: usize = 16;
pub const BOARD_WIDTH: usize = 60;
pub const BOARD_HEIGHT: usize = 18;

/// Obstacle type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObstacleType {
    Cactus,
    CactusSmall,
    CactusTall,
    Bird,
}

impl ObstacleType {
    pub fn width(&self) -> usize {
        match self {
            ObstacleType::Cactus => 2,
            ObstacleType::CactusSmall => 1,
            ObstacleType::CactusTall => 2,
            ObstacleType::Bird => 3,
        }
    }

    pub fn height(&self) -> usize {
        match self {
            ObstacleType::Cactus => 2,
            ObstacleType::CactusSmall => 1,
            ObstacleType::CactusTall => 3,
            ObstacleType::Bird => 1,
        }
    }

    pub fn y_offset(&self) -> i32 {
        match self {
            ObstacleType::Bird => -3, // Birds fly above ground
            _ => 0,
        }
    }

    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        match rng.gen_range(0..10) {
            0..=3 => ObstacleType::Cactus,
            4..=5 => ObstacleType::CactusSmall,
            6..=7 => ObstacleType::CactusTall,
            _ => ObstacleType::Bird,
        }
    }

    pub fn chars(&self) -> Vec<&'static str> {
        match self {
            ObstacleType::Cactus => vec!["##", "##"],
            ObstacleType::CactusSmall => vec!["#"],
            ObstacleType::CactusTall => vec!["||", "||", "||"],
            ObstacleType::Bird => vec!["<=>"],
        }
    }
}

/// Obstacle
#[derive(Debug, Clone)]
pub struct Obstacle {
    pub x: f32,
    pub obstacle_type: ObstacleType,
}

/// Dino state
#[derive(Debug, Clone)]
pub struct Dino {
    pub y: f32,
    pub velocity: f32,
    pub is_jumping: bool,
    pub is_ducking: bool,
}

impl Dino {
    pub fn new() -> Self {
        Self {
            y: GROUND_Y as f32,
            velocity: 0.0,
            is_jumping: false,
            is_ducking: false,
        }
    }

    pub fn jump(&mut self) {
        if !self.is_jumping {
            self.is_jumping = true;
            self.velocity = -1.8;
        }
    }

    pub fn duck(&mut self, ducking: bool) {
        self.is_ducking = ducking && !self.is_jumping;
    }

    pub fn update(&mut self) {
        if self.is_jumping {
            self.velocity += 0.12; // Gravity
            self.y += self.velocity;

            if self.y >= GROUND_Y as f32 {
                self.y = GROUND_Y as f32;
                self.is_jumping = false;
                self.velocity = 0.0;
            }
        }
    }

    pub fn hitbox(&self) -> (i32, i32, i32, i32) {
        // (x1, y1, x2, y2)
        let height = if self.is_ducking { 1 } else { 2 };
        (5, self.y as i32 - height + 1, 7, self.y as i32)
    }
}

impl Default for Dino {
    fn default() -> Self {
        Self::new()
    }
}

/// Dino Jump game state
pub struct DinoJumpState {
    pub dino: Dino,
    pub obstacles: Vec<Obstacle>,
    pub score: u32,
    pub high_score: u32,
    pub speed: f32,
    pub distance: f32,
    pub game_over: bool,
    pub frame: u32,
    pending_events: Vec<GameEvent>,
}

impl Default for DinoJumpState {
    fn default() -> Self {
        Self::new()
    }
}

impl DinoJumpState {
    pub fn new() -> Self {
        let mut state = Self {
            dino: Dino::new(),
            obstacles: Vec::new(),
            score: 0,
            high_score: 0,
            speed: 0.8,
            distance: 0.0,
            game_over: false,
            frame: 0,
            pending_events: Vec::new(),
        };
        state.pending_events.push(GameEvent::GameStarted);
        state
    }

    pub fn reset(&mut self) {
        if self.score > self.high_score {
            self.high_score = self.score;
        }
        self.dino = Dino::new();
        self.obstacles.clear();
        self.score = 0;
        self.speed = 0.8;
        self.distance = 0.0;
        self.game_over = false;
        self.frame = 0;
        self.pending_events.clear();
        self.pending_events.push(GameEvent::GameStarted);
    }

    fn spawn_obstacle(&mut self) {
        let mut rng = rand::thread_rng();
        let min_gap = 40.0 + (self.speed * 20.0);

        // Check distance from last obstacle
        let last_x = self.obstacles.last().map(|o| o.x).unwrap_or(-100.0);
        if (BOARD_WIDTH as f32) - last_x < min_gap {
            return;
        }

        if rng.gen_bool(0.03) {
            self.obstacles.push(Obstacle {
                x: BOARD_WIDTH as f32 + 5.0,
                obstacle_type: ObstacleType::random(),
            });
        }
    }

    fn check_collision(&self) -> bool {
        let (dx1, dy1, dx2, dy2) = self.dino.hitbox();

        for obstacle in &self.obstacles {
            let ox1 = obstacle.x as i32;
            let ox2 = ox1 + obstacle.obstacle_type.width() as i32;
            let oy_base = GROUND_Y as i32 + obstacle.obstacle_type.y_offset();
            let oy1 = oy_base - obstacle.obstacle_type.height() as i32 + 1;
            let oy2 = oy_base;

            // AABB collision
            if dx1 < ox2 && dx2 > ox1 && dy1 < oy2 && dy2 > oy1 {
                return true;
            }
        }

        false
    }

    pub fn ground_chars(&self) -> String {
        let offset = (self.distance as usize) % 4;
        let pattern = "-_-_";
        let mut ground = String::new();
        for i in 0..BOARD_WIDTH {
            ground.push(pattern.chars().nth((i + offset) % 4).unwrap_or('-'));
        }
        ground
    }
}

impl GameEngine for DinoJumpState {
    fn tick(&mut self) {
        if self.game_over {
            return;
        }

        self.frame += 1;

        // Update dino
        self.dino.update();

        // Move obstacles
        for obstacle in &mut self.obstacles {
            obstacle.x -= self.speed;
        }

        // Remove off-screen obstacles
        self.obstacles.retain(|o| o.x > -10.0);

        // Spawn new obstacles
        self.spawn_obstacle();

        // Update distance and score
        self.distance += self.speed;
        let new_score = (self.distance / 10.0) as u32;
        if new_score != self.score {
            let old_score = self.score;
            self.score = new_score;
            self.pending_events.push(GameEvent::ScoreChanged {
                old: old_score,
                new: self.score,
            });
        }

        // Increase speed over time
        if self.frame.is_multiple_of(500) && self.speed < 2.0 {
            self.speed += 0.1;
        }

        // Check collision
        if self.check_collision() {
            self.game_over = true;
            self.pending_events
                .push(GameEvent::GameEnded { won: false });
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        if self.game_over {
            if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                self.reset();
                return KeyHandleResult::Handled;
            }
            return KeyHandleResult::NotHandled;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char(' ') | KeyCode::Char('w') | KeyCode::Char('k') => {
                self.dino.jump();
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('j') => {
                self.dino.duck(true);
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.pending_events)
    }

    fn get_score(&self) -> u32 {
        self.score
    }

    fn is_game_over(&self) -> bool {
        self.game_over
    }
}
