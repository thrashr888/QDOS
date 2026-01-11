//! Star Trek (1971) game implementation
//!
//! A classic tactical space combat game where you command the USS Enterprise
//! through an 8x8 galaxy, hunting Klingons while managing energy and time.

use rand::Rng;

/// Galaxy dimensions (8x8 quadrants)
pub const GALAXY_SIZE: usize = 8;
/// Sector dimensions (8x8 within each quadrant)
pub const SECTOR_SIZE: usize = 8;

/// Entity types in a sector
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectorEntity {
    Empty,
    Enterprise,
    Klingon,
    Starbase,
    Star,
}

impl SectorEntity {
    pub fn char(&self) -> &'static str {
        match self {
            SectorEntity::Empty => " . ",
            SectorEntity::Enterprise => "<E>",
            SectorEntity::Klingon => "+K+",
            SectorEntity::Starbase => ">S<",
            SectorEntity::Star => " * ",
        }
    }
}

/// Klingon ship data
#[derive(Debug, Clone)]
pub struct Klingon {
    pub sector_x: usize,
    pub sector_y: usize,
    pub energy: i32,
}

/// Quadrant data
#[derive(Debug, Clone, Copy, Default)]
pub struct Quadrant {
    pub klingons: u8,
    pub starbases: u8,
    pub stars: u8,
    pub scanned: bool,
}

impl Quadrant {
    /// Get the encoded value for long-range sensors (KBS format)
    pub fn sensor_code(&self) -> String {
        format!("{}{}{}", self.klingons, self.starbases, self.stars)
    }
}

/// Command mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CommandMode {
    #[default]
    Main,
    Navigation,
    Phasers,
    Torpedoes,
    Shields,
    Computer,
}

/// Ship systems that can be damaged
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShipSystem {
    WarpEngines,
    ShortRangeSensors,
    LongRangeSensors,
    Phasers,
    PhotonTubes,
    DamageControl,
    ShieldControl,
    Computer,
}

impl ShipSystem {
    pub fn name(&self) -> &'static str {
        match self {
            ShipSystem::WarpEngines => "Warp Engines",
            ShipSystem::ShortRangeSensors => "S.R. Sensors",
            ShipSystem::LongRangeSensors => "L.R. Sensors",
            ShipSystem::Phasers => "Phasers",
            ShipSystem::PhotonTubes => "Photon Tubes",
            ShipSystem::DamageControl => "Damage Control",
            ShipSystem::ShieldControl => "Shield Control",
            ShipSystem::Computer => "Computer",
        }
    }

    pub fn all() -> &'static [ShipSystem] {
        &[
            ShipSystem::WarpEngines,
            ShipSystem::ShortRangeSensors,
            ShipSystem::LongRangeSensors,
            ShipSystem::Phasers,
            ShipSystem::PhotonTubes,
            ShipSystem::DamageControl,
            ShipSystem::ShieldControl,
            ShipSystem::Computer,
        ]
    }
}

/// Star Trek game state
pub struct TrekState {
    // Galaxy state
    pub galaxy: [[Quadrant; GALAXY_SIZE]; GALAXY_SIZE],
    pub sector: [[SectorEntity; SECTOR_SIZE]; SECTOR_SIZE],

    // Enterprise position
    pub quadrant_x: usize,
    pub quadrant_y: usize,
    pub sector_x: usize,
    pub sector_y: usize,

    // Ship status
    pub energy: i32,
    pub max_energy: i32,
    pub shields: i32,
    pub torpedoes: i32,
    pub max_torpedoes: i32,

    // System damage (negative = damaged, positive = repair time bonus)
    pub damage: [i32; 8],

    // Mission status
    pub klingons_remaining: i32,
    pub starbases_remaining: i32,
    pub stardate: f32,
    pub stardate_end: f32,
    pub docked: bool,

    // UI state
    pub mode: CommandMode,
    pub message: String,
    pub input_buffer: String,
    pub game_over: bool,
    pub game_won: bool,

    // Klingons in current sector
    pub sector_klingons: Vec<Klingon>,

    // Navigation target
    pub nav_course: Option<f32>,
    pub nav_warp: Option<f32>,
}

impl Default for TrekState {
    fn default() -> Self {
        Self::new()
    }
}

impl TrekState {
    pub fn new() -> Self {
        let mut state = Self {
            galaxy: [[Quadrant::default(); GALAXY_SIZE]; GALAXY_SIZE],
            sector: [[SectorEntity::Empty; SECTOR_SIZE]; SECTOR_SIZE],
            quadrant_x: 0,
            quadrant_y: 0,
            sector_x: 0,
            sector_y: 0,
            energy: 3000,
            max_energy: 3000,
            shields: 0,
            torpedoes: 10,
            max_torpedoes: 10,
            damage: [0; 8],
            klingons_remaining: 0,
            starbases_remaining: 0,
            stardate: 2100.0,
            stardate_end: 2130.0,
            docked: false,
            mode: CommandMode::Main,
            message: String::new(),
            input_buffer: String::new(),
            game_over: false,
            game_won: false,
            sector_klingons: Vec::new(),
            nav_course: None,
            nav_warp: None,
        };
        state.initialize_galaxy();
        state
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    fn initialize_galaxy(&mut self) {
        let mut rng = rand::thread_rng();

        // Initialize each quadrant
        for qy in 0..GALAXY_SIZE {
            for qx in 0..GALAXY_SIZE {
                // Klingons: more likely in center of galaxy
                let k = if rng.gen_bool(0.2) {
                    rng.gen_range(1..=3)
                } else {
                    0
                };

                // Starbases: scattered
                let b = if rng.gen_bool(0.1) { 1 } else { 0 };

                // Stars: 1-5 per quadrant
                let s = rng.gen_range(1..=5);

                self.galaxy[qy][qx] = Quadrant {
                    klingons: k,
                    starbases: b,
                    stars: s,
                    scanned: false,
                };

                self.klingons_remaining += k as i32;
                self.starbases_remaining += b as i32;
            }
        }

        // Ensure at least 10 Klingons and 2 starbases
        while self.klingons_remaining < 10 {
            let qx = rng.gen_range(0..GALAXY_SIZE);
            let qy = rng.gen_range(0..GALAXY_SIZE);
            self.galaxy[qy][qx].klingons += 1;
            self.klingons_remaining += 1;
        }

        while self.starbases_remaining < 2 {
            let qx = rng.gen_range(0..GALAXY_SIZE);
            let qy = rng.gen_range(0..GALAXY_SIZE);
            if self.galaxy[qy][qx].starbases == 0 {
                self.galaxy[qy][qx].starbases = 1;
                self.starbases_remaining += 1;
            }
        }

        // Set time limit based on Klingon count
        self.stardate_end = self.stardate + self.klingons_remaining as f32 * 3.0;

        // Place Enterprise in random quadrant
        self.quadrant_x = rng.gen_range(0..GALAXY_SIZE);
        self.quadrant_y = rng.gen_range(0..GALAXY_SIZE);

        // Enter the quadrant
        self.enter_quadrant();

        self.message = format!(
            "MISSION: Destroy {} Klingons in {:.1} stardates. {} starbases for resupply.",
            self.klingons_remaining,
            self.stardate_end - self.stardate,
            self.starbases_remaining
        );
    }

    fn enter_quadrant(&mut self) {
        let mut rng = rand::thread_rng();

        // Clear sector
        for y in 0..SECTOR_SIZE {
            for x in 0..SECTOR_SIZE {
                self.sector[y][x] = SectorEntity::Empty;
            }
        }
        self.sector_klingons.clear();

        // Find empty position for Enterprise
        loop {
            self.sector_x = rng.gen_range(0..SECTOR_SIZE);
            self.sector_y = rng.gen_range(0..SECTOR_SIZE);
            if self.sector[self.sector_y][self.sector_x] == SectorEntity::Empty {
                self.sector[self.sector_y][self.sector_x] = SectorEntity::Enterprise;
                break;
            }
        }

        let quadrant = &self.galaxy[self.quadrant_y][self.quadrant_x];
        let num_klingons = quadrant.klingons;
        let num_starbases = quadrant.starbases;
        let num_stars = quadrant.stars;

        // Place Klingons
        for _ in 0..num_klingons {
            loop {
                let x = rng.gen_range(0..SECTOR_SIZE);
                let y = rng.gen_range(0..SECTOR_SIZE);
                if self.sector[y][x] == SectorEntity::Empty {
                    self.sector[y][x] = SectorEntity::Klingon;
                    self.sector_klingons.push(Klingon {
                        sector_x: x,
                        sector_y: y,
                        energy: rng.gen_range(200..500),
                    });
                    break;
                }
            }
        }

        // Place starbases
        for _ in 0..num_starbases {
            loop {
                let x = rng.gen_range(0..SECTOR_SIZE);
                let y = rng.gen_range(0..SECTOR_SIZE);
                if self.sector[y][x] == SectorEntity::Empty {
                    self.sector[y][x] = SectorEntity::Starbase;
                    break;
                }
            }
        }

        // Place stars
        for _ in 0..num_stars {
            loop {
                let x = rng.gen_range(0..SECTOR_SIZE);
                let y = rng.gen_range(0..SECTOR_SIZE);
                if self.sector[y][x] == SectorEntity::Empty {
                    self.sector[y][x] = SectorEntity::Star;
                    break;
                }
            }
        }

        // Mark quadrant as scanned
        self.galaxy[self.quadrant_y][self.quadrant_x].scanned = true;

        // Check if docked
        self.check_docked();
    }

    fn check_docked(&mut self) {
        self.docked = false;

        // Check adjacent sectors for starbase
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                let nx = self.sector_x as i32 + dx;
                let ny = self.sector_y as i32 + dy;
                if nx >= 0
                    && nx < SECTOR_SIZE as i32
                    && ny >= 0
                    && ny < SECTOR_SIZE as i32
                    && self.sector[ny as usize][nx as usize] == SectorEntity::Starbase
                {
                    self.docked = true;
                    self.energy = self.max_energy;
                    self.torpedoes = self.max_torpedoes;
                    self.shields = 0;
                    // Repair all systems
                    for d in &mut self.damage {
                        *d = 0;
                    }
                    return;
                }
            }
        }
    }

    pub fn handle_key(&mut self, key: char) {
        if self.game_over || self.game_won {
            return;
        }

        match self.mode {
            CommandMode::Main => self.handle_main_key(key),
            CommandMode::Navigation => self.handle_nav_key(key),
            CommandMode::Phasers => self.handle_phaser_key(key),
            CommandMode::Torpedoes => self.handle_torpedo_key(key),
            CommandMode::Shields => self.handle_shield_key(key),
            CommandMode::Computer => self.handle_computer_key(key),
        }
    }

    fn handle_main_key(&mut self, key: char) {
        match key {
            'n' | 'N' => {
                if self.damage[ShipSystem::WarpEngines as usize] < 0 {
                    self.message = "Warp engines are damaged!".to_string();
                } else {
                    self.mode = CommandMode::Navigation;
                    self.nav_course = None;
                    self.nav_warp = None;
                    self.input_buffer.clear();
                    self.message = "Course (1-9, 5=center)? ".to_string();
                }
            }
            's' | 'S' => {
                self.short_range_scan();
            }
            'l' | 'L' => {
                if self.damage[ShipSystem::LongRangeSensors as usize] < 0 {
                    self.message = "Long range sensors are damaged!".to_string();
                } else {
                    self.long_range_scan();
                }
            }
            'p' | 'P' => {
                if self.damage[ShipSystem::Phasers as usize] < 0 {
                    self.message = "Phasers are damaged!".to_string();
                } else if self.sector_klingons.is_empty() {
                    self.message = "No Klingons in this quadrant!".to_string();
                } else {
                    self.mode = CommandMode::Phasers;
                    self.input_buffer.clear();
                    self.message = format!("Phasers locked. Energy to fire (0-{})? ", self.energy);
                }
            }
            't' | 'T' => {
                if self.damage[ShipSystem::PhotonTubes as usize] < 0 {
                    self.message = "Photon tubes are damaged!".to_string();
                } else if self.torpedoes == 0 {
                    self.message = "No torpedoes remaining!".to_string();
                } else if self.sector_klingons.is_empty() {
                    self.message = "No Klingons in this quadrant!".to_string();
                } else {
                    self.mode = CommandMode::Torpedoes;
                    self.input_buffer.clear();
                    self.message = "Torpedo course (1-9)? ".to_string();
                }
            }
            'h' | 'H' => {
                if self.damage[ShipSystem::ShieldControl as usize] < 0 {
                    self.message = "Shield control is damaged!".to_string();
                } else {
                    self.mode = CommandMode::Shields;
                    self.input_buffer.clear();
                    self.message = format!(
                        "Energy for shields (0-{})? Current: {}",
                        self.energy + self.shields,
                        self.shields
                    );
                }
            }
            'c' | 'C' => {
                if self.damage[ShipSystem::Computer as usize] < 0 {
                    self.message = "Computer is damaged!".to_string();
                } else {
                    self.mode = CommandMode::Computer;
                    self.message = "Computer: (G)alaxy map, (S)tatus, (T)orpedo calc".to_string();
                }
            }
            'd' | 'D' => {
                self.show_damage_report();
            }
            _ => {
                self.message =
                    "(N)av (S)RS (L)RS (P)hasers (T)orpedoes s(H)ields (C)omputer (D)amage"
                        .to_string();
            }
        }
    }

    fn handle_nav_key(&mut self, key: char) {
        if key == '\x1b' {
            // Escape
            self.mode = CommandMode::Main;
            self.message = "Navigation cancelled.".to_string();
            return;
        }

        if self.nav_course.is_none() {
            // Getting course
            if let Some(d) = key.to_digit(10) {
                if (1..=9).contains(&d) && d != 5 {
                    self.nav_course = Some(d as f32);
                    self.input_buffer.clear();
                    self.message = format!("Course {} set. Warp factor (0.1-8.0)? ", d);
                } else {
                    self.message =
                        "Invalid course. Use 1-9 (except 5): 7 8 9 / 4 . 6 / 1 2 3".to_string();
                }
            }
        } else if self.nav_warp.is_none() {
            // Getting warp factor
            if key == '\n' || key == '\r' {
                if let Ok(warp) = self.input_buffer.parse::<f32>() {
                    if (0.1..=8.0).contains(&warp) {
                        let energy_needed = (warp * 10.0 * 8.0) as i32;
                        if energy_needed > self.energy {
                            self.message = format!(
                                "Insufficient energy. Need {}, have {}",
                                energy_needed, self.energy
                            );
                            self.input_buffer.clear();
                        } else {
                            self.nav_warp = Some(warp);
                            self.execute_navigation();
                        }
                    } else {
                        self.message = "Warp factor must be 0.1-8.0".to_string();
                        self.input_buffer.clear();
                    }
                } else {
                    self.message = "Invalid warp factor.".to_string();
                    self.input_buffer.clear();
                }
            } else if key.is_ascii_digit() || key == '.' {
                self.input_buffer.push(key);
            } else if key == '\x7f' || key == '\x08' {
                // Backspace
                self.input_buffer.pop();
            }
        }
    }

    fn execute_navigation(&mut self) {
        let course = self.nav_course.unwrap();
        let warp = self.nav_warp.unwrap();

        // Calculate direction from course (1-9 numpad style)
        let (dx, dy) = match course as u32 {
            1 => (-1, 1),
            2 => (0, 1),
            3 => (1, 1),
            4 => (-1, 0),
            6 => (1, 0),
            7 => (-1, -1),
            8 => (0, -1),
            9 => (1, -1),
            _ => (0, 0),
        };

        let distance = (warp * 8.0) as i32;
        let energy_used = distance * 10;
        self.energy -= energy_used;

        let mut new_sector_x = self.sector_x as i32;
        let mut new_sector_y = self.sector_y as i32;
        let mut new_quad_x = self.quadrant_x as i32;
        let mut new_quad_y = self.quadrant_y as i32;

        // Clear current position
        self.sector[self.sector_y][self.sector_x] = SectorEntity::Empty;

        // Move through sectors
        let mut blocked = false;
        for _ in 0..distance {
            new_sector_x += dx;
            new_sector_y += dy;

            // Check quadrant boundaries
            if new_sector_x < 0 {
                new_sector_x = SECTOR_SIZE as i32 - 1;
                new_quad_x -= 1;
            } else if new_sector_x >= SECTOR_SIZE as i32 {
                new_sector_x = 0;
                new_quad_x += 1;
            }
            if new_sector_y < 0 {
                new_sector_y = SECTOR_SIZE as i32 - 1;
                new_quad_y -= 1;
            } else if new_sector_y >= SECTOR_SIZE as i32 {
                new_sector_y = 0;
                new_quad_y += 1;
            }

            // Check galaxy boundaries
            if new_quad_x < 0 || new_quad_x >= GALAXY_SIZE as i32 {
                self.message = "Course blocked by galaxy edge!".to_string();
                blocked = true;
                break;
            }
            if new_quad_y < 0 || new_quad_y >= GALAXY_SIZE as i32 {
                self.message = "Course blocked by galaxy edge!".to_string();
                blocked = true;
                break;
            }

            // Check for collision in current quadrant
            if new_quad_x == self.quadrant_x as i32 && new_quad_y == self.quadrant_y as i32 {
                let entity = self.sector[new_sector_y as usize][new_sector_x as usize];
                if entity != SectorEntity::Empty {
                    self.message = format!("Course blocked by {:?}!", entity);
                    new_sector_x -= dx;
                    new_sector_y -= dy;
                    blocked = true;
                    break;
                }
            }
        }

        // Update position
        let changed_quadrant =
            new_quad_x != self.quadrant_x as i32 || new_quad_y != self.quadrant_y as i32;

        self.sector_x = new_sector_x.clamp(0, SECTOR_SIZE as i32 - 1) as usize;
        self.sector_y = new_sector_y.clamp(0, SECTOR_SIZE as i32 - 1) as usize;

        if changed_quadrant && !blocked {
            self.quadrant_x = new_quad_x as usize;
            self.quadrant_y = new_quad_y as usize;
            self.enter_quadrant();
            self.message = format!(
                "Entering quadrant {}-{}",
                self.quadrant_x + 1,
                self.quadrant_y + 1
            );
        } else {
            self.sector[self.sector_y][self.sector_x] = SectorEntity::Enterprise;
            self.check_docked();
            if !blocked {
                self.message = format!(
                    "Warp {} complete. Sector {}-{}",
                    warp,
                    self.sector_x + 1,
                    self.sector_y + 1
                );
            }
        }

        // Time passes
        self.stardate += if warp < 1.0 { 0.1 } else { 1.0 };

        // Klingons attack if present and not docked
        if !self.docked && !self.sector_klingons.is_empty() {
            self.klingon_attack();
        }

        // Repair systems
        self.repair_systems();

        // Check win/lose conditions
        self.check_game_end();

        self.mode = CommandMode::Main;
    }

    fn handle_phaser_key(&mut self, key: char) {
        if key == '\x1b' {
            self.mode = CommandMode::Main;
            self.message = "Phasers cancelled.".to_string();
            return;
        }

        if key == '\n' || key == '\r' {
            if let Ok(energy) = self.input_buffer.parse::<i32>() {
                if energy > 0 && energy <= self.energy {
                    self.fire_phasers(energy);
                } else if energy == 0 {
                    self.mode = CommandMode::Main;
                    self.message = "Phasers cancelled.".to_string();
                } else {
                    self.message = format!("Invalid energy. Available: {}", self.energy);
                    self.input_buffer.clear();
                }
            } else {
                self.message = "Invalid energy amount.".to_string();
                self.input_buffer.clear();
            }
        } else if key.is_ascii_digit() {
            self.input_buffer.push(key);
        } else if key == '\x7f' || key == '\x08' {
            self.input_buffer.pop();
        }
    }

    fn fire_phasers(&mut self, energy: i32) {
        self.energy -= energy;

        let mut rng = rand::thread_rng();
        let energy_per_klingon = energy / self.sector_klingons.len() as i32;
        let mut destroyed = 0;

        for klingon in &mut self.sector_klingons {
            // Damage decreases with distance
            let dx = (klingon.sector_x as i32 - self.sector_x as i32).abs();
            let dy = (klingon.sector_y as i32 - self.sector_y as i32).abs();
            let distance = ((dx * dx + dy * dy) as f32).sqrt();

            let damage = (energy_per_klingon as f32 / distance * rng.gen_range(0.7..1.3)) as i32;
            klingon.energy -= damage;

            if klingon.energy <= 0 {
                destroyed += 1;
            }
        }

        // Remove destroyed Klingons
        let mut i = 0;
        while i < self.sector_klingons.len() {
            if self.sector_klingons[i].energy <= 0 {
                let k = &self.sector_klingons[i];
                self.sector[k.sector_y][k.sector_x] = SectorEntity::Empty;
                self.sector_klingons.remove(i);
                self.klingons_remaining -= 1;
                self.galaxy[self.quadrant_y][self.quadrant_x].klingons -= 1;
            } else {
                i += 1;
            }
        }

        self.message = format!(
            "Phasers fired! {} Klingon(s) destroyed. {} remaining in sector.",
            destroyed,
            self.sector_klingons.len()
        );

        // Klingons return fire
        if !self.sector_klingons.is_empty() {
            self.klingon_attack();
        }

        self.check_game_end();
        self.mode = CommandMode::Main;
    }

    fn handle_torpedo_key(&mut self, key: char) {
        if key == '\x1b' {
            self.mode = CommandMode::Main;
            self.message = "Torpedoes cancelled.".to_string();
            return;
        }

        if let Some(d) = key.to_digit(10) {
            if (1..=9).contains(&d) && d != 5 {
                self.fire_torpedo(d as i32);
            } else {
                self.message = "Invalid course. Use 1-9 (except 5)".to_string();
            }
        }
    }

    fn fire_torpedo(&mut self, course: i32) {
        self.torpedoes -= 1;

        let (dx, dy) = match course {
            1 => (-1, 1),
            2 => (0, 1),
            3 => (1, 1),
            4 => (-1, 0),
            6 => (1, 0),
            7 => (-1, -1),
            8 => (0, -1),
            9 => (1, -1),
            _ => (0, 0),
        };

        let mut tx = self.sector_x as i32;
        let mut ty = self.sector_y as i32;
        let mut hit = false;

        // Track torpedo
        for _ in 0..SECTOR_SIZE {
            tx += dx;
            ty += dy;

            if tx < 0 || tx >= SECTOR_SIZE as i32 || ty < 0 || ty >= SECTOR_SIZE as i32 {
                self.message = "Torpedo missed - left quadrant.".to_string();
                break;
            }

            match self.sector[ty as usize][tx as usize] {
                SectorEntity::Klingon => {
                    // Find and destroy the Klingon
                    if let Some(idx) = self
                        .sector_klingons
                        .iter()
                        .position(|k| k.sector_x == tx as usize && k.sector_y == ty as usize)
                    {
                        self.sector_klingons.remove(idx);
                        self.klingons_remaining -= 1;
                        self.galaxy[self.quadrant_y][self.quadrant_x].klingons -= 1;
                    }
                    self.sector[ty as usize][tx as usize] = SectorEntity::Empty;
                    self.message = format!("*** KLINGON DESTROYED at {}-{} ***", tx + 1, ty + 1);
                    hit = true;
                    break;
                }
                SectorEntity::Star => {
                    self.message = format!("Torpedo hit star at {}-{}", tx + 1, ty + 1);
                    hit = true;
                    break;
                }
                SectorEntity::Starbase => {
                    self.sector[ty as usize][tx as usize] = SectorEntity::Empty;
                    self.starbases_remaining -= 1;
                    self.galaxy[self.quadrant_y][self.quadrant_x].starbases -= 1;
                    self.message = "*** STARBASE DESTROYED! ***".to_string();
                    hit = true;
                    break;
                }
                _ => {}
            }
        }

        if !hit {
            self.message = "Torpedo missed.".to_string();
        }

        // Klingons return fire
        if !self.sector_klingons.is_empty() {
            self.klingon_attack();
        }

        self.check_game_end();
        self.mode = CommandMode::Main;
    }

    fn handle_shield_key(&mut self, key: char) {
        if key == '\x1b' {
            self.mode = CommandMode::Main;
            self.message = "Shield change cancelled.".to_string();
            return;
        }

        if key == '\n' || key == '\r' {
            if let Ok(new_shields) = self.input_buffer.parse::<i32>() {
                let available = self.energy + self.shields;
                if new_shields >= 0 && new_shields <= available {
                    let delta = new_shields - self.shields;
                    self.energy -= delta;
                    self.shields = new_shields;
                    self.message =
                        format!("Shields set to {}. Energy: {}", self.shields, self.energy);
                } else {
                    self.message = format!("Invalid. Available energy: {}", available);
                    self.input_buffer.clear();
                    return;
                }
            } else {
                self.message = "Invalid shield value.".to_string();
                self.input_buffer.clear();
                return;
            }
            self.mode = CommandMode::Main;
        } else if key.is_ascii_digit() {
            self.input_buffer.push(key);
        } else if key == '\x7f' || key == '\x08' {
            self.input_buffer.pop();
        }
    }

    fn handle_computer_key(&mut self, key: char) {
        match key {
            'g' | 'G' => {
                self.message =
                    "Galaxy map shown. Scanned quadrants display KBS (Klingons/Bases/Stars)."
                        .to_string();
            }
            's' | 'S' => {
                self.message = format!(
                    "STATUS: Klingons:{} Stardates:{:.1} Bases:{} Energy:{} Shields:{} Torps:{}",
                    self.klingons_remaining,
                    self.stardate_end - self.stardate,
                    self.starbases_remaining,
                    self.energy,
                    self.shields,
                    self.torpedoes
                );
            }
            't' | 'T' => {
                if self.sector_klingons.is_empty() {
                    self.message = "No Klingons in sector.".to_string();
                } else {
                    let k = &self.sector_klingons[0];
                    let dx = k.sector_x as i32 - self.sector_x as i32;
                    let dy = k.sector_y as i32 - self.sector_y as i32;
                    // Convert to course
                    let course = if dx < 0 && dy < 0 {
                        7
                    } else if dx == 0 && dy < 0 {
                        8
                    } else if dx > 0 && dy < 0 {
                        9
                    } else if dx < 0 && dy == 0 {
                        4
                    } else if dx > 0 && dy == 0 {
                        6
                    } else if dx < 0 && dy > 0 {
                        1
                    } else if dx == 0 && dy > 0 {
                        2
                    } else {
                        3
                    };
                    self.message = format!(
                        "Klingon at {}-{}: Course {} recommended",
                        k.sector_x + 1,
                        k.sector_y + 1,
                        course
                    );
                }
            }
            _ => {
                self.mode = CommandMode::Main;
                self.message = "Computer offline.".to_string();
            }
        }
    }

    fn short_range_scan(&mut self) {
        if self.damage[ShipSystem::ShortRangeSensors as usize] < 0 {
            self.message = "Short range sensors damaged - display may be inaccurate.".to_string();
        } else {
            self.message = format!(
                "Quadrant {}-{}, Sector {}-{}. {} Klingon(s) detected.",
                self.quadrant_x + 1,
                self.quadrant_y + 1,
                self.sector_x + 1,
                self.sector_y + 1,
                self.sector_klingons.len()
            );
        }
    }

    fn long_range_scan(&mut self) {
        // Mark surrounding quadrants as scanned
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                let qx = self.quadrant_x as i32 + dx;
                let qy = self.quadrant_y as i32 + dy;
                if qx >= 0 && qx < GALAXY_SIZE as i32 && qy >= 0 && qy < GALAXY_SIZE as i32 {
                    self.galaxy[qy as usize][qx as usize].scanned = true;
                }
            }
        }
        self.message = "Long range scan complete. Adjacent quadrants scanned.".to_string();
    }

    fn show_damage_report(&mut self) {
        let mut damaged = Vec::new();
        for (i, system) in ShipSystem::all().iter().enumerate() {
            if self.damage[i] < 0 {
                damaged.push(format!("{}: {}", system.name(), self.damage[i]));
            }
        }
        if damaged.is_empty() {
            self.message = "All systems operational.".to_string();
        } else {
            self.message = format!("Damaged: {}", damaged.join(", "));
        }
    }

    fn klingon_attack(&mut self) {
        let mut rng = rand::thread_rng();
        let mut total_damage = 0;

        for klingon in &self.sector_klingons {
            let dx = (klingon.sector_x as i32 - self.sector_x as i32).abs();
            let dy = (klingon.sector_y as i32 - self.sector_y as i32).abs();
            let distance = ((dx * dx + dy * dy) as f32).sqrt();

            let hit = (klingon.energy as f32 / distance * rng.gen_range(0.3..0.7)) as i32;
            total_damage += hit;
        }

        if total_damage > 0 {
            // Shields absorb damage first
            if self.shields >= total_damage {
                self.shields -= total_damage;
                self.message = format!(
                    "{} Shields absorb {} damage. Shields at {}",
                    self.message, total_damage, self.shields
                );
            } else {
                let hull_damage = total_damage - self.shields;
                self.shields = 0;
                self.energy -= hull_damage;

                // Random system damage
                if rng.gen_bool(0.3) {
                    let system = rng.gen_range(0..8);
                    self.damage[system] -= rng.gen_range(1..4);
                    self.message = format!(
                        "{} {} damaged! Hull hit for {}",
                        self.message,
                        ShipSystem::all()[system].name(),
                        hull_damage
                    );
                } else {
                    self.message = format!("{} Hull hit for {} damage!", self.message, hull_damage);
                }
            }
        }

        if self.energy <= 0 {
            self.game_over = true;
            self.message = "*** ENTERPRISE DESTROYED ***".to_string();
        }
    }

    fn repair_systems(&mut self) {
        let mut rng = rand::thread_rng();

        for d in &mut self.damage {
            if *d < 0 {
                // Repair slightly each turn
                *d += if self.docked { 2 } else { 1 };
                if *d > 0 {
                    *d = 0;
                }
            }
        }

        // Random new damage while moving
        if !self.docked && rng.gen_bool(0.1) {
            let system = rng.gen_range(0..8);
            self.damage[system] -= rng.gen_range(1..3);
        }
    }

    fn check_game_end(&mut self) {
        if self.klingons_remaining == 0 {
            self.game_won = true;
            self.message = "*** CONGRATULATIONS! All Klingons destroyed! ***".to_string();
        } else if self.stardate >= self.stardate_end {
            self.game_over = true;
            self.message = "*** TIME EXPIRED! Mission failed! ***".to_string();
        } else if self.energy <= 0 {
            self.game_over = true;
            self.message = "*** ENTERPRISE DESTROYED! ***".to_string();
        } else if self.starbases_remaining == 0 && self.energy < 100 && self.shields == 0 {
            self.game_over = true;
            self.message = "*** Stranded in space with no starbases! ***".to_string();
        }
    }

    pub fn tick(&mut self) {
        // Game runs on player input, no continuous tick needed
    }
}
