use super::platform::{GameEngine, KeyHandleResult};
use crossterm::event::{KeyCode, KeyEvent};
use rand::Rng;
use serde::{Deserialize, Serialize};

const FIELD_WIDTH: usize = 76;
const FIELD_HEIGHT: usize = 18;
const MAX_HEALTH: i32 = 100;
const MIN_ANGLE: i32 = 0;
const MAX_ANGLE: i32 = 90;
const MIN_POWER: i32 = 10;
const MAX_POWER: i32 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GamePhase {
    Aiming,
    Firing,
    GameOver,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tank {
    pub x: usize,
    pub y: usize, // Ground level
    pub health: i32,
    pub is_player: bool,
}

impl Tank {
    pub fn new(x: usize, y: usize, is_player: bool) -> Self {
        Self {
            x,
            y,
            health: MAX_HEALTH,
            is_player,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.health > 0
    }

    pub fn take_damage(&mut self, damage: i32) {
        self.health = (self.health - damage).max(0);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Projectile {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub active: bool,
}

impl Projectile {
    pub fn new(x: f64, y: f64, angle: i32, power: i32, wind: i32) -> Self {
        let angle_rad = (angle as f64).to_radians();
        let power_factor = power as f64 / 50.0;
        let wind_factor = wind as f64 / 100.0;

        Self {
            x,
            y,
            vx: angle_rad.cos() * power_factor + wind_factor,
            vy: -angle_rad.sin() * power_factor,
            active: true,
        }
    }

    pub fn update(&mut self) {
        const GRAVITY: f64 = 0.15;
        self.x += self.vx;
        self.y += self.vy;
        self.vy += GRAVITY;
    }

    pub fn is_out_of_bounds(&self) -> bool {
        self.x < 0.0 || self.x >= FIELD_WIDTH as f64 || self.y >= FIELD_HEIGHT as f64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtilleryState {
    pub terrain: Vec<Vec<bool>>, // true = solid, false = empty
    pub player_tank: Tank,
    pub enemy_tank: Tank,
    pub projectile: Option<Projectile>,
    pub phase: GamePhase,
    pub angle: i32,
    pub power: i32,
    pub wind: i32,
    pub current_player: bool, // true = player, false = enemy
    pub game_over: bool,
    pub winner: Option<bool>, // Some(true) = player won, Some(false) = enemy won
    pub message: String,
    tick_count: u32,
}

impl Default for ArtilleryState {
    fn default() -> Self {
        Self::new()
    }
}

impl ArtilleryState {
    pub fn new() -> Self {
        let terrain = Self::generate_terrain();

        // Find ground level at each position
        let player_x = 15;
        let enemy_x = FIELD_WIDTH - 15;
        let player_y = Self::find_ground_level(&terrain, player_x);
        let enemy_y = Self::find_ground_level(&terrain, enemy_x);

        Self {
            terrain,
            player_tank: Tank::new(player_x, player_y, true),
            enemy_tank: Tank::new(enemy_x, enemy_y, false),
            projectile: None,
            phase: GamePhase::Aiming,
            angle: 45,
            power: 50,
            wind: rand::thread_rng().gen_range(-15..=15),
            current_player: true,
            game_over: false,
            winner: None,
            message: "Adjust angle and power, then fire!".to_string(),
            tick_count: 0,
        }
    }

    #[allow(clippy::needless_range_loop)]
    fn generate_terrain() -> Vec<Vec<bool>> {
        let mut terrain = vec![vec![false; FIELD_WIDTH]; FIELD_HEIGHT];
        let mut rng = rand::thread_rng();

        // Generate random hills using sine waves
        let wave1_amp = rng.gen_range(3.0..6.0);
        let wave1_freq = rng.gen_range(0.05..0.15);
        let wave2_amp = rng.gen_range(2.0..4.0);
        let wave2_freq = rng.gen_range(0.1..0.2);

        let base_height = FIELD_HEIGHT - 5;

        for x in 0..FIELD_WIDTH {
            let height1 = (x as f64 * wave1_freq).sin() * wave1_amp;
            let height2 = (x as f64 * wave2_freq).sin() * wave2_amp;
            let ground_level = (base_height as f64 + height1 + height2) as usize;

            // Fill from ground level to bottom
            for y in ground_level..FIELD_HEIGHT {
                terrain[y][x] = true;
            }
        }

        terrain
    }

    #[allow(clippy::needless_range_loop)]
    fn find_ground_level(terrain: &[Vec<bool>], x: usize) -> usize {
        for y in 0..FIELD_HEIGHT {
            if terrain[y][x] {
                return y.saturating_sub(1);
            }
        }
        FIELD_HEIGHT - 1
    }

    pub fn adjust_angle(&mut self, delta: i32) {
        self.angle = (self.angle + delta).clamp(MIN_ANGLE, MAX_ANGLE);
    }

    pub fn adjust_power(&mut self, delta: i32) {
        self.power = (self.power + delta).clamp(MIN_POWER, MAX_POWER);
    }

    pub fn fire(&mut self) {
        if self.phase != GamePhase::Aiming {
            return;
        }

        let tank = if self.current_player {
            &self.player_tank
        } else {
            &self.enemy_tank
        };

        if !tank.is_alive() {
            return;
        }

        let start_x = tank.x as f64;
        let start_y = tank.y as f64;
        let angle = if self.current_player {
            self.angle
        } else {
            180 - self.angle // Enemy fires left
        };

        self.projectile = Some(Projectile::new(
            start_x, start_y, angle, self.power, self.wind,
        ));
        self.phase = GamePhase::Firing;
        self.message = "FIRING!".to_string();
    }

    fn update_projectile(&mut self) {
        if let Some(proj) = &mut self.projectile {
            proj.update();

            let px = proj.x as usize;
            let py = proj.y as usize;

            // Check collision with terrain or tanks
            if proj.is_out_of_bounds() {
                self.end_turn("Missed!");
            } else if px < FIELD_WIDTH && py < FIELD_HEIGHT && self.terrain[py][px] {
                self.create_explosion(px, py);
                self.end_turn("Hit!");
            } else {
                // Check tank collision
                if self.check_tank_hit(px, py) {
                    self.create_explosion(px, py);
                    self.end_turn("Direct hit!");
                }
            }
        }
    }

    fn check_tank_hit(&mut self, px: usize, py: usize) -> bool {
        // Check player tank
        if px >= self.player_tank.x.saturating_sub(2)
            && px <= self.player_tank.x + 2
            && py >= self.player_tank.y.saturating_sub(2)
            && py <= self.player_tank.y + 1
        {
            self.player_tank.take_damage(50);
            return true;
        }

        // Check enemy tank
        if px >= self.enemy_tank.x.saturating_sub(2)
            && px <= self.enemy_tank.x + 2
            && py >= self.enemy_tank.y.saturating_sub(2)
            && py <= self.enemy_tank.y + 1
        {
            self.enemy_tank.take_damage(50);
            return true;
        }

        false
    }

    fn create_explosion(&mut self, cx: usize, cy: usize) {
        const BLAST_RADIUS: usize = 4;

        for dy in 0..=BLAST_RADIUS {
            for dx in 0..=BLAST_RADIUS {
                let dist = ((dx * dx + dy * dy) as f64).sqrt();
                if dist <= BLAST_RADIUS as f64 {
                    // Destroy terrain in all 4 quadrants
                    for (sx, sy) in &[
                        (cx.saturating_add(dx), cy.saturating_add(dy)),
                        (cx.saturating_sub(dx), cy.saturating_add(dy)),
                        (cx.saturating_add(dx), cy.saturating_sub(dy)),
                        (cx.saturating_sub(dx), cy.saturating_sub(dy)),
                    ] {
                        if *sx < FIELD_WIDTH && *sy < FIELD_HEIGHT {
                            self.terrain[*sy][*sx] = false;
                        }
                    }
                }
            }
        }

        // Update tank positions if ground below them is destroyed
        self.update_tank_positions();
    }

    fn update_tank_positions(&mut self) {
        // Player tank
        while self.player_tank.y < FIELD_HEIGHT - 1
            && !self.terrain[self.player_tank.y + 1][self.player_tank.x]
        {
            self.player_tank.y += 1;
        }

        // Enemy tank
        while self.enemy_tank.y < FIELD_HEIGHT - 1
            && !self.terrain[self.enemy_tank.y + 1][self.enemy_tank.x]
        {
            self.enemy_tank.y += 1;
        }
    }

    fn end_turn(&mut self, msg: &str) {
        self.projectile = None;
        self.phase = GamePhase::Aiming;

        if !self.player_tank.is_alive() {
            self.game_over = true;
            self.winner = Some(false);
            self.phase = GamePhase::GameOver;
            self.message = "Enemy wins!".to_string();
        } else if !self.enemy_tank.is_alive() {
            self.game_over = true;
            self.winner = Some(true);
            self.phase = GamePhase::GameOver;
            self.message = "You win!".to_string();
        } else {
            self.current_player = !self.current_player;
            self.wind = rand::thread_rng().gen_range(-15..=15);

            if self.current_player {
                self.message = format!("{} Your turn!", msg);
            } else {
                self.message = format!("{} Enemy's turn...", msg);
                // Enemy will fire on next tick
            }
        }
    }

    fn ai_fire(&mut self) {
        // Simple AI: aim at player with some randomness
        let mut rng = rand::thread_rng();

        // Calculate rough angle to player
        let _dx = (self.player_tank.x as i32 - self.enemy_tank.x as i32).abs();
        let dy = self.enemy_tank.y as i32 - self.player_tank.y as i32;

        let base_angle = if dy > 0 {
            45 + (dy * 2).min(30)
        } else {
            45 - (dy.abs() * 2).min(20)
        };

        // Add randomness
        self.angle = (base_angle + rng.gen_range(-10..=10)).clamp(MIN_ANGLE, MAX_ANGLE);
        self.power = (50 + rng.gen_range(-20..=20)).clamp(MIN_POWER, MAX_POWER);

        self.fire();
    }

    pub fn final_score(&self) -> u32 {
        if self.winner == Some(true) {
            self.player_tank.health as u32 * 10
        } else {
            0
        }
    }
}

impl GameEngine for ArtilleryState {
    fn tick(&mut self) {
        if self.game_over {
            return;
        }

        match self.phase {
            GamePhase::Firing => {
                self.update_projectile();
            }
            GamePhase::Aiming => {
                if !self.current_player {
                    // AI turn
                    self.tick_count += 1;
                    if self.tick_count >= 15 {
                        // Wait a bit before AI fires
                        self.ai_fire();
                        self.tick_count = 0;
                    }
                }
            }
            GamePhase::GameOver => {}
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        if self.game_over {
            match key.code {
                KeyCode::Esc => return KeyHandleResult::RequestQuit,
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    *self = Self::new();
                    return KeyHandleResult::Handled;
                }
                _ => return KeyHandleResult::NotHandled,
            }
        }

        if !self.current_player {
            // Don't accept input during AI turn
            return KeyHandleResult::Handled;
        }

        match self.phase {
            GamePhase::Aiming => match key.code {
                KeyCode::Esc => KeyHandleResult::RequestQuit,
                KeyCode::Char('p') | KeyCode::Char('P') => KeyHandleResult::RequestPause,
                KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') => {
                    self.adjust_angle(-1);
                    KeyHandleResult::Handled
                }
                KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('D') => {
                    self.adjust_angle(1);
                    KeyHandleResult::Handled
                }
                KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                    self.adjust_power(1);
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.adjust_power(-1);
                    KeyHandleResult::Handled
                }
                KeyCode::Char(' ') | KeyCode::Enter => {
                    self.fire();
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::NotHandled,
            },
            GamePhase::Firing => KeyHandleResult::Handled,
            GamePhase::GameOver => KeyHandleResult::Handled,
        }
    }

    fn get_score(&self) -> u32 {
        self.final_score()
    }

    fn is_game_over(&self) -> bool {
        self.game_over
    }

    fn is_game_won(&self) -> bool {
        self.winner == Some(true)
    }
}
