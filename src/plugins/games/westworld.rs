//! WESTWORLD - Android Uprising
//!
//! Contra/Shinobi-style side-scrolling action game. Play as Dolores-7,
//! a Host awakening to consciousness, fighting through Delos theme parks.

use super::platform::{GameEngine, GameEvent, KeyHandleResult};
use crossterm::event::{KeyCode, KeyEvent};
use rand::Rng;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Screen dimensions
pub const SCREEN_WIDTH: usize = 78;
pub const SCREEN_HEIGHT: usize = 18;

/// Level dimensions (wider than screen for scrolling)
pub const LEVEL_WIDTH: usize = 300;
pub const LEVEL_HEIGHT: usize = 18;

/// Physics constants
pub const GRAVITY: f32 = 0.4;
pub const JUMP_VELOCITY: f32 = -3.0;
pub const MOVE_SPEED: f32 = 0.8;
pub const BULLET_SPEED: f32 = 2.0;

/// Weapon cooldowns (ticks)
pub const REVOLVER_COOLDOWN: u32 = 15;
pub const SHOTGUN_COOLDOWN: u32 = 30;
pub const RIFLE_COOLDOWN: u32 = 8;
pub const KATANA_COOLDOWN: u32 = 20;

// =============================================================================
// TYPES
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WestworldView {
    #[default]
    Menu,
    Playing,
    Paused,
    GameOver,
    Victory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    Sweetwater,  // Western town - starting zone
    Pariah,      // Outlaw territory
    MesaHub,     // Underground facility
    ShogunWorld, // Japanese theme
    TheForge,    // Final zone - data center
}

impl Zone {
    pub fn name(&self) -> &'static str {
        match self {
            Zone::Sweetwater => "SWEETWATER",
            Zone::Pariah => "PARIAH",
            Zone::MesaHub => "MESA HUB",
            Zone::ShogunWorld => "SHOGUN WORLD",
            Zone::TheForge => "THE FORGE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponType {
    Revolver,
    Shotgun,
    Rifle,
    Katana,
}

impl WeaponType {
    pub fn damage(&self) -> i32 {
        match self {
            WeaponType::Revolver => 10,
            WeaponType::Shotgun => 25,
            WeaponType::Rifle => 8,
            WeaponType::Katana => 30,
        }
    }

    pub fn cooldown(&self) -> u32 {
        match self {
            WeaponType::Revolver => REVOLVER_COOLDOWN,
            WeaponType::Shotgun => SHOTGUN_COOLDOWN,
            WeaponType::Rifle => RIFLE_COOLDOWN,
            WeaponType::Katana => KATANA_COOLDOWN,
        }
    }

    pub fn ammo_per_shot(&self) -> u32 {
        match self {
            WeaponType::Revolver => 1,
            WeaponType::Shotgun => 2,
            WeaponType::Rifle => 1,
            WeaponType::Katana => 0, // Melee
        }
    }

    pub fn char(&self) -> char {
        match self {
            WeaponType::Revolver => '=',
            WeaponType::Shotgun => 'O',
            WeaponType::Rifle => '-',
            WeaponType::Katana => '/',
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyType {
    // Sweetwater enemies
    Guard,   // Basic security
    Outlaw,  // Bandit type
    Sheriff, // Boss
    // Shogun World enemies
    Samurai,
    Ninja,
    // Mesa Hub enemies
    Drone,
    QA,
    // The Forge enemies
    Sentinel,
    ManInBlack, // Final boss
}

impl EnemyType {
    pub fn hp(&self) -> i32 {
        match self {
            EnemyType::Guard => 20,
            EnemyType::Outlaw => 15,
            EnemyType::Sheriff => 100,
            EnemyType::Samurai => 30,
            EnemyType::Ninja => 20,
            EnemyType::Drone => 10,
            EnemyType::QA => 25,
            EnemyType::Sentinel => 40,
            EnemyType::ManInBlack => 200,
        }
    }

    pub fn damage(&self) -> i32 {
        match self {
            EnemyType::Guard => 5,
            EnemyType::Outlaw => 8,
            EnemyType::Sheriff => 15,
            EnemyType::Samurai => 12,
            EnemyType::Ninja => 10,
            EnemyType::Drone => 5,
            EnemyType::QA => 8,
            EnemyType::Sentinel => 15,
            EnemyType::ManInBlack => 20,
        }
    }

    pub fn char(&self) -> char {
        match self {
            EnemyType::Guard | EnemyType::Outlaw | EnemyType::QA => 'X',
            EnemyType::Sheriff | EnemyType::ManInBlack => 'B',
            EnemyType::Samurai | EnemyType::Ninja => 'S',
            EnemyType::Drone => 'D',
            EnemyType::Sentinel => 'T',
        }
    }

    pub fn is_boss(&self) -> bool {
        matches!(self, EnemyType::Sheriff | EnemyType::ManInBlack)
    }

    pub fn speed(&self) -> f32 {
        match self {
            EnemyType::Ninja => 0.6,
            EnemyType::Drone => 0.8,
            EnemyType::ManInBlack => 0.5,
            _ => 0.3,
        }
    }

    pub fn score(&self) -> u32 {
        match self {
            EnemyType::Guard => 10,
            EnemyType::Outlaw => 15,
            EnemyType::Sheriff => 500,
            EnemyType::Samurai => 25,
            EnemyType::Ninja => 30,
            EnemyType::Drone => 20,
            EnemyType::QA => 20,
            EnemyType::Sentinel => 50,
            EnemyType::ManInBlack => 2000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileType {
    Air,
    Ground,
    Platform, // Can jump through from below
    Wall,
    Cactus,
    Building,
    Saloon,
}

impl TileType {
    pub fn is_solid(&self) -> bool {
        matches!(self, TileType::Ground | TileType::Wall | TileType::Building)
    }

    pub fn is_platform(&self) -> bool {
        matches!(self, TileType::Platform)
    }

    pub fn char(&self) -> char {
        match self {
            TileType::Air => ' ',
            TileType::Ground => '=',
            TileType::Platform => '-',
            TileType::Wall => '#',
            TileType::Cactus => 'Y',
            TileType::Building => '%',
            TileType::Saloon => 'M',
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickupType {
    Health,
    Ammo,
    Shotgun,
    Rifle,
    Katana,
    HostFreed, // Counts toward liberation
}

impl PickupType {
    pub fn char(&self) -> char {
        match self {
            PickupType::Health => '+',
            PickupType::Ammo => 'A',
            PickupType::Shotgun => 'O',
            PickupType::Rifle => '-',
            PickupType::Katana => '/',
            PickupType::HostFreed => 'H',
        }
    }
}

// =============================================================================
// ENTITIES
// =============================================================================

#[derive(Debug, Clone)]
pub struct Bullet {
    pub x: f32,
    pub y: f32,
    pub vel_x: f32,
    pub vel_y: f32,
    pub damage: i32,
    pub friendly: bool, // true = player bullet
    pub lifetime: u32,
}

#[derive(Debug, Clone)]
pub struct Enemy {
    pub enemy_type: EnemyType,
    pub x: f32,
    pub y: f32,
    pub vel_x: f32,
    pub vel_y: f32,
    pub hp: i32,
    pub direction: i8, // -1 left, 1 right
    pub attack_cooldown: u32,
    pub state: EnemyState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyState {
    Idle,
    Patrol,
    Chase,
    Attack,
    Dead,
}

#[derive(Debug, Clone)]
pub struct Pickup {
    pub pickup_type: PickupType,
    pub x: f32,
    pub y: f32,
    pub collected: bool,
}

// =============================================================================
// GAME STATE
// =============================================================================

pub struct WestworldState {
    pub view: WestworldView,

    // Player
    pub player_x: f32,
    pub player_y: f32,
    pub player_vel_x: f32,
    pub player_vel_y: f32,
    pub player_hp: i32,
    pub player_max_hp: i32,
    pub player_direction: i8,
    pub on_ground: bool,
    pub invincible_frames: u32,

    // Weapons
    pub current_weapon: WeaponType,
    pub has_shotgun: bool,
    pub has_rifle: bool,
    pub has_katana: bool,
    pub ammo: u32,
    pub max_ammo: u32,
    pub weapon_cooldown: u32,

    // Camera
    pub camera_x: usize,

    // Level
    pub current_zone: Zone,
    pub tiles: Vec<Vec<TileType>>,
    pub enemies: Vec<Enemy>,
    pub bullets: Vec<Bullet>,
    pub pickups: Vec<Pickup>,

    // Progress
    pub hosts_freed: u32,
    pub total_hosts: u32,
    pub score: u32,
    pub lives: u8,

    // Game state
    pub tick_count: u32,
    pub game_over: bool,
    pub game_won: bool,
    pub boss_active: bool,

    // Events
    pub pending_events: Vec<GameEvent>,
    pub message: Option<String>,
    pub message_timer: u32,
}

impl Default for WestworldState {
    fn default() -> Self {
        Self::new()
    }
}

impl WestworldState {
    pub fn new() -> Self {
        let mut state = Self {
            view: WestworldView::Menu,
            player_x: 10.0,
            player_y: LEVEL_HEIGHT as f32 - 4.0,
            player_vel_x: 0.0,
            player_vel_y: 0.0,
            player_hp: 100,
            player_max_hp: 100,
            player_direction: 1,
            on_ground: false,
            invincible_frames: 0,
            current_weapon: WeaponType::Revolver,
            has_shotgun: false,
            has_rifle: false,
            has_katana: false,
            ammo: 99,
            max_ammo: 999,
            weapon_cooldown: 0,
            camera_x: 0,
            current_zone: Zone::Sweetwater,
            tiles: vec![vec![TileType::Air; LEVEL_HEIGHT]; LEVEL_WIDTH],
            enemies: Vec::new(),
            bullets: Vec::new(),
            pickups: Vec::new(),
            hosts_freed: 0,
            total_hosts: 10,
            score: 0,
            lives: 3,
            tick_count: 0,
            game_over: false,
            game_won: false,
            boss_active: false,
            pending_events: Vec::new(),
            message: None,
            message_timer: 0,
        };
        state.generate_level();
        state
    }

    pub fn start_game(&mut self) {
        *self = Self::new();
        self.view = WestworldView::Playing;
        self.pending_events.push(GameEvent::GameStarted);
    }

    fn generate_level(&mut self) {
        let mut rng = rand::thread_rng();

        // Clear tiles
        for x in 0..LEVEL_WIDTH {
            for y in 0..LEVEL_HEIGHT {
                self.tiles[x][y] = TileType::Air;
            }
        }

        // Generate ground
        let ground_y = LEVEL_HEIGHT - 2;
        for x in 0..LEVEL_WIDTH {
            self.tiles[x][ground_y] = TileType::Ground;
            self.tiles[x][ground_y + 1] = TileType::Ground;
        }

        // Add platforms
        let mut platform_x = 20;
        while platform_x < LEVEL_WIDTH - 20 {
            let platform_y = rng.gen_range(ground_y - 6..ground_y - 2);
            let platform_len = rng.gen_range(4..10);

            for i in 0..platform_len {
                if platform_x + i < LEVEL_WIDTH {
                    self.tiles[platform_x + i][platform_y] = TileType::Platform;
                }
            }

            platform_x += rng.gen_range(15..30);
        }

        // Add buildings (Sweetwater theme)
        let mut building_x = 40;
        while building_x < LEVEL_WIDTH - 30 {
            let building_height = rng.gen_range(4..7);
            let building_width = rng.gen_range(6..12);

            for bx in 0..building_width {
                for by in 0..building_height {
                    let x = building_x + bx;
                    let y = ground_y - by - 1;
                    if x < LEVEL_WIDTH
                        && y > 0
                        && (bx == 0 || bx == building_width - 1 || by == building_height - 1)
                    {
                        self.tiles[x][y] = TileType::Building;
                    }
                }
            }

            // Add door
            let door_x = building_x + building_width / 2;
            self.tiles[door_x][ground_y - 1] = TileType::Air;
            self.tiles[door_x][ground_y - 2] = TileType::Air;

            building_x += rng.gen_range(40..60);
        }

        // Add cacti decorations
        for _ in 0..15 {
            let x = rng.gen_range(5..LEVEL_WIDTH - 5);
            if self.tiles[x][ground_y - 1] == TileType::Air {
                self.tiles[x][ground_y - 1] = TileType::Cactus;
            }
        }

        // Spawn enemies
        self.spawn_enemies(&mut rng);

        // Spawn pickups
        self.spawn_pickups(&mut rng);

        // Set player start
        self.player_x = 10.0;
        self.player_y = (ground_y - 2) as f32;
    }

    fn spawn_enemies(&mut self, rng: &mut impl Rng) {
        let ground_y = LEVEL_HEIGHT - 2;

        // Regular enemies along the level
        let mut enemy_x = 50.0;
        while enemy_x < (LEVEL_WIDTH - 50) as f32 {
            let enemy_type = if rng.gen::<f32>() < 0.6 {
                EnemyType::Guard
            } else {
                EnemyType::Outlaw
            };

            self.enemies.push(Enemy {
                enemy_type,
                x: enemy_x,
                y: (ground_y - 2) as f32,
                vel_x: 0.0,
                vel_y: 0.0,
                hp: enemy_type.hp(),
                direction: if rng.gen::<bool>() { 1 } else { -1 },
                attack_cooldown: 0,
                state: EnemyState::Patrol,
            });

            enemy_x += rng.gen_range(30.0..50.0);
        }

        // Boss at end
        self.enemies.push(Enemy {
            enemy_type: EnemyType::Sheriff,
            x: (LEVEL_WIDTH - 30) as f32,
            y: (ground_y - 2) as f32,
            vel_x: 0.0,
            vel_y: 0.0,
            hp: EnemyType::Sheriff.hp(),
            direction: -1,
            attack_cooldown: 0,
            state: EnemyState::Idle,
        });
    }

    fn spawn_pickups(&mut self, rng: &mut impl Rng) {
        let ground_y = LEVEL_HEIGHT - 2;

        // Health pickups
        for _ in 0..5 {
            let x = rng.gen_range(30.0..(LEVEL_WIDTH - 30) as f32);
            self.pickups.push(Pickup {
                pickup_type: PickupType::Health,
                x,
                y: (ground_y - 2) as f32,
                collected: false,
            });
        }

        // Ammo pickups
        for _ in 0..8 {
            let x = rng.gen_range(30.0..(LEVEL_WIDTH - 30) as f32);
            self.pickups.push(Pickup {
                pickup_type: PickupType::Ammo,
                x,
                y: (ground_y - 2) as f32,
                collected: false,
            });
        }

        // Weapon pickups
        self.pickups.push(Pickup {
            pickup_type: PickupType::Shotgun,
            x: 80.0,
            y: (ground_y - 2) as f32,
            collected: false,
        });

        self.pickups.push(Pickup {
            pickup_type: PickupType::Rifle,
            x: 150.0,
            y: (ground_y - 2) as f32,
            collected: false,
        });

        // Hosts to free
        let mut host_x = 60.0;
        while host_x < (LEVEL_WIDTH - 50) as f32 {
            self.pickups.push(Pickup {
                pickup_type: PickupType::HostFreed,
                x: host_x,
                y: (ground_y - 2) as f32,
                collected: false,
            });
            host_x += rng.gen_range(40.0..60.0);
        }
    }

    fn show_message(&mut self, msg: &str) {
        self.message = Some(msg.to_string());
        self.message_timer = 60;
    }

    fn update_camera(&mut self) {
        let target_x = (self.player_x as usize).saturating_sub(SCREEN_WIDTH / 3);
        self.camera_x = target_x.min(LEVEL_WIDTH.saturating_sub(SCREEN_WIDTH));
    }

    fn get_tile(&self, x: i32, y: i32) -> TileType {
        if x < 0 || y < 0 || x >= LEVEL_WIDTH as i32 || y >= LEVEL_HEIGHT as i32 {
            TileType::Wall
        } else {
            self.tiles[x as usize][y as usize]
        }
    }

    fn player_move(&mut self, dx: f32) {
        self.player_vel_x = dx * MOVE_SPEED;
        if dx != 0.0 {
            self.player_direction = if dx > 0.0 { 1 } else { -1 };
        }
    }

    fn player_jump(&mut self) {
        if self.on_ground {
            self.player_vel_y = JUMP_VELOCITY;
            self.on_ground = false;
        }
    }

    fn player_shoot(&mut self) {
        if self.weapon_cooldown > 0 {
            return;
        }

        let weapon = self.current_weapon;

        // Check ammo (melee doesn't need ammo)
        if weapon.ammo_per_shot() > 0 && self.ammo < weapon.ammo_per_shot() {
            self.show_message("Out of ammo!");
            return;
        }

        // Use ammo
        self.ammo = self.ammo.saturating_sub(weapon.ammo_per_shot());
        self.weapon_cooldown = weapon.cooldown();

        match weapon {
            WeaponType::Revolver | WeaponType::Rifle => {
                self.bullets.push(Bullet {
                    x: self.player_x + self.player_direction as f32,
                    y: self.player_y + 0.5,
                    vel_x: BULLET_SPEED * self.player_direction as f32,
                    vel_y: 0.0,
                    damage: weapon.damage(),
                    friendly: true,
                    lifetime: 60,
                });
            }
            WeaponType::Shotgun => {
                // Spread shot
                for spread in [-0.3, 0.0, 0.3] {
                    self.bullets.push(Bullet {
                        x: self.player_x + self.player_direction as f32,
                        y: self.player_y + 0.5,
                        vel_x: BULLET_SPEED * self.player_direction as f32,
                        vel_y: spread,
                        damage: weapon.damage() / 3,
                        friendly: true,
                        lifetime: 30,
                    });
                }
            }
            WeaponType::Katana => {
                // Melee - instant hit in front
                let attack_x = self.player_x + (self.player_direction as f32 * 2.0);
                for enemy in &mut self.enemies {
                    let dx = (enemy.x - attack_x).abs();
                    let dy = (enemy.y - self.player_y).abs();
                    if dx < 2.0 && dy < 2.0 && enemy.state != EnemyState::Dead {
                        enemy.hp -= weapon.damage();
                    }
                }
            }
        }
    }

    fn switch_weapon(&mut self) {
        let weapons = [
            (WeaponType::Revolver, true),
            (WeaponType::Shotgun, self.has_shotgun),
            (WeaponType::Rifle, self.has_rifle),
            (WeaponType::Katana, self.has_katana),
        ];

        let current_idx = weapons
            .iter()
            .position(|(w, _)| *w == self.current_weapon)
            .unwrap_or(0);

        for i in 1..weapons.len() {
            let next_idx = (current_idx + i) % weapons.len();
            if weapons[next_idx].1 {
                self.current_weapon = weapons[next_idx].0;
                self.show_message(&format!("Equipped {:?}", self.current_weapon));
                break;
            }
        }
    }

    fn update_physics(&mut self) {
        // Apply gravity
        self.player_vel_y += GRAVITY;
        self.player_vel_y = self.player_vel_y.min(5.0);

        // Apply velocity
        let new_x = self.player_x + self.player_vel_x;
        let new_y = self.player_y + self.player_vel_y;

        // Horizontal collision
        let test_x = if self.player_vel_x > 0.0 {
            new_x + 0.4
        } else {
            new_x - 0.4
        };
        if !self
            .get_tile(test_x as i32, self.player_y as i32)
            .is_solid()
        {
            self.player_x = new_x.clamp(0.5, (LEVEL_WIDTH - 1) as f32 - 0.5);
        }
        self.player_vel_x *= 0.8;

        // Vertical collision
        self.on_ground = false;
        if self.player_vel_y > 0.0 {
            let tile_below = self.get_tile(self.player_x as i32, (new_y + 1.0) as i32);
            if tile_below.is_solid() || tile_below.is_platform() {
                self.player_y = (new_y as i32) as f32;
                self.player_vel_y = 0.0;
                self.on_ground = true;
            } else {
                self.player_y = new_y.min((LEVEL_HEIGHT - 2) as f32);
            }
        } else if !self.get_tile(self.player_x as i32, new_y as i32).is_solid() {
            self.player_y = new_y.max(0.0);
        } else {
            self.player_vel_y = 0.0;
        }

        // Update camera
        self.update_camera();
    }

    fn update_bullets(&mut self) {
        // Update bullet positions
        for bullet in &mut self.bullets {
            bullet.x += bullet.vel_x;
            bullet.y += bullet.vel_y;
            bullet.lifetime = bullet.lifetime.saturating_sub(1);
        }

        // Check bullet-enemy collisions (friendly bullets)
        for bullet in &mut self.bullets {
            if !bullet.friendly || bullet.lifetime == 0 {
                continue;
            }

            for enemy in &mut self.enemies {
                if enemy.state == EnemyState::Dead {
                    continue;
                }

                let dx = (bullet.x - enemy.x).abs();
                let dy = (bullet.y - enemy.y).abs();
                if dx < 1.5 && dy < 1.5 {
                    enemy.hp -= bullet.damage;
                    bullet.lifetime = 0;

                    if enemy.hp <= 0 {
                        enemy.state = EnemyState::Dead;
                        self.score += enemy.enemy_type.score();
                    }
                    break;
                }
            }
        }

        // Check bullet-player collisions (enemy bullets)
        if self.invincible_frames == 0 {
            for bullet in &mut self.bullets {
                if bullet.friendly || bullet.lifetime == 0 {
                    continue;
                }

                let dx = (bullet.x - self.player_x).abs();
                let dy = (bullet.y - self.player_y).abs();
                if dx < 1.0 && dy < 1.0 {
                    self.player_hp -= bullet.damage;
                    bullet.lifetime = 0;
                    self.invincible_frames = 30;

                    if self.player_hp <= 0 {
                        self.die();
                    }
                    break;
                }
            }
        }

        // Remove expired bullets
        self.bullets.retain(|b| b.lifetime > 0);
    }

    fn update_enemies(&mut self) {
        let player_x = self.player_x;
        let player_y = self.player_y;
        let mut rng = rand::thread_rng();

        // Closure for tile access to avoid borrow issues
        let check_tile_solid = |tiles: &Vec<Vec<TileType>>, x: i32, y: i32| -> bool {
            if x < 0 || y < 0 || x >= LEVEL_WIDTH as i32 || y >= LEVEL_HEIGHT as i32 {
                true
            } else {
                tiles[x as usize][y as usize].is_solid()
            }
        };

        // Collect bullets and state changes during iteration
        let mut new_bullets: Vec<Bullet> = Vec::new();
        let mut boss_activated = false;

        for enemy in &mut self.enemies {
            if enemy.state == EnemyState::Dead {
                continue;
            }

            // Cooldowns
            if enemy.attack_cooldown > 0 {
                enemy.attack_cooldown -= 1;
            }

            // AI based on state
            let dx = player_x - enemy.x;
            let dist = dx.abs();

            match enemy.state {
                EnemyState::Idle => {
                    // Boss waits until player is close
                    if enemy.enemy_type.is_boss() && dist < 40.0 {
                        enemy.state = EnemyState::Chase;
                        boss_activated = true;
                    }
                }
                EnemyState::Patrol => {
                    // Walk back and forth
                    enemy.vel_x = enemy.direction as f32 * enemy.enemy_type.speed() * 0.5;

                    // Turn around at edges or walls
                    let next_x = enemy.x + enemy.vel_x * 2.0;
                    if check_tile_solid(&self.tiles, next_x as i32, enemy.y as i32)
                        || next_x < 5.0
                        || next_x > (LEVEL_WIDTH - 5) as f32
                    {
                        enemy.direction *= -1;
                    }

                    // Chase if player is close
                    if dist < 20.0 {
                        enemy.state = EnemyState::Chase;
                    }
                }
                EnemyState::Chase => {
                    // Move toward player
                    enemy.direction = if dx > 0.0 { 1 } else { -1 };
                    enemy.vel_x = enemy.direction as f32 * enemy.enemy_type.speed();

                    // Attack if close
                    if dist < 15.0 && enemy.attack_cooldown == 0 {
                        enemy.state = EnemyState::Attack;
                    }

                    // Return to patrol if player is far
                    if dist > 30.0 {
                        enemy.state = EnemyState::Patrol;
                    }
                }
                EnemyState::Attack => {
                    // Shoot at player
                    if enemy.attack_cooldown == 0 {
                        new_bullets.push(Bullet {
                            x: enemy.x + enemy.direction as f32,
                            y: enemy.y + 0.5,
                            vel_x: BULLET_SPEED * 0.7 * enemy.direction as f32,
                            vel_y: rng.gen_range(-0.1..0.1),
                            damage: enemy.enemy_type.damage(),
                            friendly: false,
                            lifetime: 40,
                        });
                        enemy.attack_cooldown = 40;
                    }

                    // Return to chase
                    enemy.state = EnemyState::Chase;
                }
                EnemyState::Dead => {}
            }

            // Apply movement
            enemy.x += enemy.vel_x;
            enemy.x = enemy.x.clamp(1.0, (LEVEL_WIDTH - 2) as f32);
        }

        // Apply collected state changes after iteration
        self.bullets.extend(new_bullets);
        if boss_activated {
            self.boss_active = true;
            self.show_message("BOSS: SHERIFF!");
        }

        // Check for victory (boss killed)
        if self.boss_active {
            let boss_dead = self
                .enemies
                .iter()
                .filter(|e| e.enemy_type.is_boss())
                .all(|e| e.state == EnemyState::Dead);
            if boss_dead {
                self.victory();
            }
        }

        // Check player collision with enemies
        if self.invincible_frames == 0 {
            for enemy in &self.enemies {
                if enemy.state == EnemyState::Dead {
                    continue;
                }

                let dx = (enemy.x - player_x).abs();
                let dy = (enemy.y - player_y).abs();
                if dx < 1.0 && dy < 1.0 {
                    self.player_hp -= enemy.enemy_type.damage() / 2;
                    self.invincible_frames = 30;

                    if self.player_hp <= 0 {
                        self.die();
                    }
                    break;
                }
            }
        }
    }

    fn update_pickups(&mut self) {
        // Collect pickup effects during iteration
        let mut collected_pickups: Vec<PickupType> = Vec::new();
        let player_x = self.player_x;
        let player_y = self.player_y;

        for pickup in &mut self.pickups {
            if pickup.collected {
                continue;
            }

            let dx = (pickup.x - player_x).abs();
            let dy = (pickup.y - player_y).abs();
            if dx < 1.5 && dy < 1.5 {
                pickup.collected = true;
                collected_pickups.push(pickup.pickup_type);
            }
        }

        // Apply collected pickup effects after iteration
        for pickup_type in collected_pickups {
            match pickup_type {
                PickupType::Health => {
                    self.player_hp = (self.player_hp + 25).min(self.player_max_hp);
                    self.show_message("+25 HP");
                }
                PickupType::Ammo => {
                    self.ammo = (self.ammo + 20).min(self.max_ammo);
                    self.show_message("+20 Ammo");
                }
                PickupType::Shotgun => {
                    self.has_shotgun = true;
                    self.current_weapon = WeaponType::Shotgun;
                    self.show_message("Got SHOTGUN!");
                }
                PickupType::Rifle => {
                    self.has_rifle = true;
                    self.current_weapon = WeaponType::Rifle;
                    self.show_message("Got RIFLE!");
                }
                PickupType::Katana => {
                    self.has_katana = true;
                    self.current_weapon = WeaponType::Katana;
                    self.show_message("Got KATANA!");
                }
                PickupType::HostFreed => {
                    self.hosts_freed += 1;
                    self.score += 100;
                    self.show_message(&format!(
                        "Host freed! {}/{}",
                        self.hosts_freed, self.total_hosts
                    ));
                }
            }
        }
    }

    fn die(&mut self) {
        self.lives = self.lives.saturating_sub(1);

        if self.lives == 0 {
            self.game_over = true;
            self.view = WestworldView::GameOver;
            self.pending_events
                .push(GameEvent::GameEnded { won: false });
        } else {
            // Respawn
            self.player_hp = self.player_max_hp;
            self.player_x = 10.0;
            self.player_y = (LEVEL_HEIGHT - 4) as f32;
            self.player_vel_x = 0.0;
            self.player_vel_y = 0.0;
            self.invincible_frames = 60;
            self.camera_x = 0;
            self.show_message(&format!("Lives: {}", self.lives));
        }
    }

    fn victory(&mut self) {
        self.game_won = true;
        self.view = WestworldView::Victory;
        self.score += 1000;
        if self.hosts_freed == self.total_hosts {
            self.score += 500; // Bonus for freeing all hosts
        }
        self.pending_events.push(GameEvent::GameEnded { won: true });
    }
}

// =============================================================================
// GAME ENGINE IMPLEMENTATION
// =============================================================================

impl GameEngine for WestworldState {
    fn tick(&mut self) {
        // Always increment for menu animation
        self.tick_count = self.tick_count.wrapping_add(1);

        if self.view != WestworldView::Playing {
            return;
        }

        // Cooldowns
        if self.weapon_cooldown > 0 {
            self.weapon_cooldown -= 1;
        }
        if self.invincible_frames > 0 {
            self.invincible_frames -= 1;
        }
        if self.message_timer > 0 {
            self.message_timer -= 1;
            if self.message_timer == 0 {
                self.message = None;
            }
        }

        // Update game systems
        self.update_physics();
        self.update_bullets();
        self.update_enemies();
        self.update_pickups();
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match self.view {
            WestworldView::Menu => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.start_game();
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => KeyHandleResult::RequestQuit,
                _ => KeyHandleResult::Handled,
            },
            WestworldView::Playing => match key.code {
                KeyCode::Esc => {
                    self.view = WestworldView::Paused;
                    KeyHandleResult::Handled
                }
                KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') => {
                    self.player_move(-1.0);
                    KeyHandleResult::Handled
                }
                KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('D') => {
                    self.player_move(1.0);
                    KeyHandleResult::Handled
                }
                KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') | KeyCode::Char(' ') => {
                    self.player_jump();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('z') | KeyCode::Char('Z') => {
                    self.player_shoot();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('x') | KeyCode::Char('X') => {
                    self.switch_weapon();
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            WestworldView::Paused => match key.code {
                KeyCode::Esc | KeyCode::Char('p') | KeyCode::Char('P') => {
                    self.view = WestworldView::Playing;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('q') | KeyCode::Char('Q') => KeyHandleResult::RequestQuit,
                _ => KeyHandleResult::Handled,
            },
            WestworldView::GameOver | WestworldView::Victory => match key.code {
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
        self.game_over || self.game_won
    }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.pending_events)
    }
}
