//! Asteroid game implementation
//!
//! Classic arcade asteroid shooter - destroy asteroids while avoiding collisions.

use super::platform::{GameEngine, GameEvent, KeyHandleResult};
use crossterm::event::{KeyCode, KeyEvent};
use rand::Rng;

pub const BOARD_WIDTH: usize = 60;
pub const BOARD_HEIGHT: usize = 20;

/// Ship state
#[derive(Debug, Clone)]
pub struct Ship {
    pub x: f32,
    pub y: f32,
    pub dx: f32,
    pub dy: f32,
    pub angle: f32, // 0 = up, 90 = right, etc.
}

impl Ship {
    pub fn new() -> Self {
        Self {
            x: (BOARD_WIDTH / 2) as f32,
            y: (BOARD_HEIGHT / 2) as f32,
            dx: 0.0,
            dy: 0.0,
            angle: 0.0,
        }
    }

    pub fn thrust(&mut self) {
        let rad = self.angle.to_radians();
        self.dx += rad.sin() * 0.15;
        self.dy -= rad.cos() * 0.15;
        // Limit max speed
        let speed = (self.dx * self.dx + self.dy * self.dy).sqrt();
        if speed > 1.0 {
            self.dx /= speed;
            self.dy /= speed;
        }
    }

    pub fn rotate_left(&mut self) {
        self.angle = (self.angle - 15.0).rem_euclid(360.0);
    }

    pub fn rotate_right(&mut self) {
        self.angle = (self.angle + 15.0).rem_euclid(360.0);
    }

    pub fn update(&mut self) {
        self.x += self.dx;
        self.y += self.dy;

        // Wrap around screen
        if self.x < 0.0 {
            self.x += BOARD_WIDTH as f32;
        }
        if self.x >= BOARD_WIDTH as f32 {
            self.x -= BOARD_WIDTH as f32;
        }
        if self.y < 0.0 {
            self.y += BOARD_HEIGHT as f32;
        }
        if self.y >= BOARD_HEIGHT as f32 {
            self.y -= BOARD_HEIGHT as f32;
        }

        // Friction
        self.dx *= 0.99;
        self.dy *= 0.99;
    }

    pub fn pos(&self) -> (i32, i32) {
        (self.x.round() as i32, self.y.round() as i32)
    }

    pub fn direction_char(&self) -> char {
        let a = self.angle.rem_euclid(360.0);
        if !(22.5..337.5).contains(&a) {
            '^'
        } else if a < 67.5 {
            '/'
        } else if a < 112.5 {
            '>'
        } else if a < 157.5 {
            '\\'
        } else if a < 202.5 {
            'v'
        } else if a < 247.5 {
            '/'
        } else if a < 292.5 {
            '<'
        } else {
            '\\'
        }
    }
}

impl Default for Ship {
    fn default() -> Self {
        Self::new()
    }
}

/// Bullet state
#[derive(Debug, Clone)]
pub struct Bullet {
    pub x: f32,
    pub y: f32,
    pub dx: f32,
    pub dy: f32,
    pub lifetime: u32,
}

/// Asteroid size
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsteroidSize {
    Large,
    Medium,
    Small,
}

impl AsteroidSize {
    pub fn radius(&self) -> f32 {
        match self {
            AsteroidSize::Large => 2.5,
            AsteroidSize::Medium => 1.5,
            AsteroidSize::Small => 0.8,
        }
    }

    pub fn points(&self) -> u32 {
        match self {
            AsteroidSize::Large => 20,
            AsteroidSize::Medium => 50,
            AsteroidSize::Small => 100,
        }
    }

    pub fn char(&self) -> char {
        match self {
            AsteroidSize::Large => 'O',
            AsteroidSize::Medium => 'o',
            AsteroidSize::Small => '*',
        }
    }
}

/// Asteroid state
#[derive(Debug, Clone)]
pub struct Asteroid {
    pub x: f32,
    pub y: f32,
    pub dx: f32,
    pub dy: f32,
    pub size: AsteroidSize,
}

impl Asteroid {
    pub fn new_random(avoid_x: f32, avoid_y: f32) -> Self {
        let mut rng = rand::thread_rng();
        let mut x;
        let mut y;

        // Spawn away from player
        loop {
            x = rng.gen_range(0.0..BOARD_WIDTH as f32);
            y = rng.gen_range(0.0..BOARD_HEIGHT as f32);
            let dist = ((x - avoid_x).powi(2) + (y - avoid_y).powi(2)).sqrt();
            if dist > 8.0 {
                break;
            }
        }

        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let speed = rng.gen_range(0.1..0.3);

        Self {
            x,
            y,
            dx: angle.cos() * speed,
            dy: angle.sin() * speed,
            size: AsteroidSize::Large,
        }
    }

    pub fn split(&self) -> Vec<Asteroid> {
        let mut rng = rand::thread_rng();
        let new_size = match self.size {
            AsteroidSize::Large => Some(AsteroidSize::Medium),
            AsteroidSize::Medium => Some(AsteroidSize::Small),
            AsteroidSize::Small => None,
        };

        if let Some(size) = new_size {
            (0..2)
                .map(|_| {
                    let angle = rng.gen_range(0.0..std::f32::consts::TAU);
                    let speed = rng.gen_range(0.2..0.4);
                    Asteroid {
                        x: self.x,
                        y: self.y,
                        dx: angle.cos() * speed,
                        dy: angle.sin() * speed,
                        size,
                    }
                })
                .collect()
        } else {
            vec![]
        }
    }

    pub fn update(&mut self) {
        self.x += self.dx;
        self.y += self.dy;

        // Wrap around
        if self.x < 0.0 {
            self.x += BOARD_WIDTH as f32;
        }
        if self.x >= BOARD_WIDTH as f32 {
            self.x -= BOARD_WIDTH as f32;
        }
        if self.y < 0.0 {
            self.y += BOARD_HEIGHT as f32;
        }
        if self.y >= BOARD_HEIGHT as f32 {
            self.y -= BOARD_HEIGHT as f32;
        }
    }

    pub fn pos(&self) -> (i32, i32) {
        (self.x.round() as i32, self.y.round() as i32)
    }
}

/// Asteroids game state
pub struct AsteroidState {
    pub ship: Ship,
    pub bullets: Vec<Bullet>,
    pub asteroids: Vec<Asteroid>,
    pub score: u32,
    pub lives: u32,
    pub level: u32,
    pub game_over: bool,
    pub invincible_frames: u32,
    pending_events: Vec<GameEvent>,
}

impl Default for AsteroidState {
    fn default() -> Self {
        Self::new()
    }
}

impl AsteroidState {
    pub fn new() -> Self {
        let mut state = Self {
            ship: Ship::new(),
            bullets: Vec::new(),
            asteroids: Vec::new(),
            score: 0,
            lives: 3,
            level: 1,
            game_over: false,
            invincible_frames: 0,
            pending_events: Vec::new(),
        };
        state.spawn_asteroids();
        state.pending_events.push(GameEvent::GameStarted);
        state
    }

    pub fn reset(&mut self) {
        self.ship = Ship::new();
        self.bullets.clear();
        self.asteroids.clear();
        self.score = 0;
        self.lives = 3;
        self.level = 1;
        self.game_over = false;
        self.invincible_frames = 60;
        self.spawn_asteroids();
        self.pending_events.clear();
        self.pending_events.push(GameEvent::GameStarted);
    }

    fn spawn_asteroids(&mut self) {
        let count = 2 + self.level as usize;
        for _ in 0..count {
            self.asteroids
                .push(Asteroid::new_random(self.ship.x, self.ship.y));
        }
    }

    pub fn shoot(&mut self) {
        if self.bullets.len() >= 5 || self.game_over {
            return;
        }

        let rad = self.ship.angle.to_radians();
        let speed = 1.2;

        self.bullets.push(Bullet {
            x: self.ship.x,
            y: self.ship.y,
            dx: rad.sin() * speed + self.ship.dx,
            dy: -rad.cos() * speed + self.ship.dy,
            lifetime: 50,
        });
    }

    fn check_collisions(&mut self) {
        // Bullet-asteroid collisions
        let mut new_asteroids = vec![];
        let mut bullets_to_remove = vec![];
        let mut asteroids_to_remove = vec![];

        for (bi, bullet) in self.bullets.iter().enumerate() {
            for (ai, asteroid) in self.asteroids.iter().enumerate() {
                let dist =
                    ((bullet.x - asteroid.x).powi(2) + (bullet.y - asteroid.y).powi(2)).sqrt();
                if dist < asteroid.size.radius() {
                    bullets_to_remove.push(bi);
                    asteroids_to_remove.push(ai);
                    let old_score = self.score;
                    self.score += asteroid.size.points();
                    new_asteroids.extend(asteroid.split());
                    self.pending_events.push(GameEvent::ScoreChanged {
                        old: old_score,
                        new: self.score,
                    });
                    break;
                }
            }
        }

        // Remove hit bullets and asteroids
        bullets_to_remove.sort_unstable();
        bullets_to_remove.reverse();
        for bi in bullets_to_remove {
            if bi < self.bullets.len() {
                self.bullets.remove(bi);
            }
        }

        asteroids_to_remove.sort_unstable();
        asteroids_to_remove.dedup();
        asteroids_to_remove.reverse();
        for ai in asteroids_to_remove {
            if ai < self.asteroids.len() {
                self.asteroids.remove(ai);
            }
        }

        self.asteroids.extend(new_asteroids);

        // Ship-asteroid collision
        if self.invincible_frames == 0 {
            for asteroid in &self.asteroids {
                let dist = ((self.ship.x - asteroid.x).powi(2)
                    + (self.ship.y - asteroid.y).powi(2))
                .sqrt();
                if dist < asteroid.size.radius() + 0.5 {
                    self.lives = self.lives.saturating_sub(1);
                    if self.lives == 0 {
                        self.game_over = true;
                        self.pending_events
                            .push(GameEvent::GameEnded { won: false });
                    } else {
                        self.ship = Ship::new();
                        self.invincible_frames = 90;
                    }
                    break;
                }
            }
        }
    }
}

impl GameEngine for AsteroidState {
    fn tick(&mut self) {
        if self.game_over {
            return;
        }

        // Update ship
        self.ship.update();

        // Update invincibility
        if self.invincible_frames > 0 {
            self.invincible_frames -= 1;
        }

        // Update bullets
        for bullet in &mut self.bullets {
            bullet.x += bullet.dx;
            bullet.y += bullet.dy;

            // Wrap around
            if bullet.x < 0.0 {
                bullet.x += BOARD_WIDTH as f32;
            }
            if bullet.x >= BOARD_WIDTH as f32 {
                bullet.x -= BOARD_WIDTH as f32;
            }
            if bullet.y < 0.0 {
                bullet.y += BOARD_HEIGHT as f32;
            }
            if bullet.y >= BOARD_HEIGHT as f32 {
                bullet.y -= BOARD_HEIGHT as f32;
            }

            bullet.lifetime = bullet.lifetime.saturating_sub(1);
        }

        // Remove dead bullets
        self.bullets.retain(|b| b.lifetime > 0);

        // Update asteroids
        for asteroid in &mut self.asteroids {
            asteroid.update();
        }

        // Check collisions
        self.check_collisions();

        // Level complete?
        if self.asteroids.is_empty() {
            self.level += 1;
            self.spawn_asteroids();
            self.invincible_frames = 60;
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
            KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('h') => {
                self.ship.rotate_left();
                KeyHandleResult::Handled
            }
            KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('l') => {
                self.ship.rotate_right();
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('k') => {
                self.ship.thrust();
                KeyHandleResult::Handled
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                self.shoot();
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
