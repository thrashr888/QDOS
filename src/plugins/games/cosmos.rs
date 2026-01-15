//! COSMOS - Claude's Space Odyssey
//!
//! A procedurally-generated space exploration game about curiosity, discovery,
//! and the wonder of the unknown. Explore star systems, discover planets,
//! make first contact with alien civilizations, and gather knowledge.

use super::platform::{GameEngine, GameEvent, KeyHandleResult};
use crossterm::event::{KeyCode, KeyEvent};
use rand::prelude::*;
use rand::SeedableRng;

// =============================================================================
// CONSTANTS
// =============================================================================

const GALAXY_SIZE: usize = 12; // Number of star systems
const MAX_PLANETS_PER_SYSTEM: usize = 5;
const STARTING_FUEL: u32 = 100;
const STARTING_HULL: u32 = 100;
const WARP_FUEL_COST: u32 = 15;
const SCAN_FUEL_COST: u32 = 2;

// =============================================================================
// STAR TYPES
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarType {
    GType,   // Yellow - like our Sun
    MType,   // Red dwarf - most common
    KType,   // Orange
    FType,   // White-yellow
    Binary,  // Double star system
    Neutron, // Dead star - rare, dangerous
}

impl StarType {
    pub fn symbol(&self) -> &'static str {
        match self {
            StarType::GType => "*",
            StarType::MType => ".",
            StarType::KType => "o",
            StarType::FType => "+",
            StarType::Binary => "8",
            StarType::Neutron => "@",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            StarType::GType => "G-type (Yellow)",
            StarType::MType => "M-type (Red Dwarf)",
            StarType::KType => "K-type (Orange)",
            StarType::FType => "F-type (White)",
            StarType::Binary => "Binary System",
            StarType::Neutron => "Neutron Star",
        }
    }

    pub fn habitability_bonus(&self) -> i32 {
        match self {
            StarType::GType => 3,
            StarType::KType => 2,
            StarType::FType => 1,
            StarType::MType => 0,
            StarType::Binary => -1,
            StarType::Neutron => -3,
        }
    }
}

// =============================================================================
// PLANET TYPES
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetType {
    Terrestrial, // Rocky, potentially habitable
    GasGiant,    // Fuel source
    Ice,         // Frozen world
    Desert,      // Dry, hot
    Ocean,       // Water world
    Volcanic,    // Active volcanism
    Barren,      // Dead rock
}

impl PlanetType {
    pub fn symbol(&self) -> &'static str {
        match self {
            PlanetType::Terrestrial => "O",
            PlanetType::GasGiant => "0",
            PlanetType::Ice => "o",
            PlanetType::Desert => "~",
            PlanetType::Ocean => "w",
            PlanetType::Volcanic => "^",
            PlanetType::Barren => ".",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            PlanetType::Terrestrial => "Terrestrial",
            PlanetType::GasGiant => "Gas Giant",
            PlanetType::Ice => "Ice World",
            PlanetType::Desert => "Desert",
            PlanetType::Ocean => "Ocean World",
            PlanetType::Volcanic => "Volcanic",
            PlanetType::Barren => "Barren",
        }
    }

    pub fn can_land(&self) -> bool {
        !matches!(self, PlanetType::GasGiant)
    }

    pub fn fuel_available(&self) -> u32 {
        match self {
            PlanetType::GasGiant => 30,
            _ => 0,
        }
    }

    pub fn life_chance(&self) -> u32 {
        match self {
            PlanetType::Terrestrial => 60,
            PlanetType::Ocean => 50,
            PlanetType::Ice => 20,
            PlanetType::Desert => 10,
            PlanetType::Volcanic => 5,
            PlanetType::GasGiant => 3,
            PlanetType::Barren => 0,
        }
    }
}

// =============================================================================
// ALIEN SPECIES
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlienSpecies {
    Harmonics, // Musical communication
    Geometers, // Mathematical patterns
    Empaths,   // Emotional colors
}

impl AlienSpecies {
    pub fn name(&self) -> &'static str {
        match self {
            AlienSpecies::Harmonics => "The Harmonics",
            AlienSpecies::Geometers => "The Geometers",
            AlienSpecies::Empaths => "The Empaths",
        }
    }

    pub fn greeting(&self) -> &'static str {
        match self {
            AlienSpecies::Harmonics => "do-re-mi-fa-so-la-ti",
            AlienSpecies::Geometers => "/\\ [] () /\\/\\ [][]",
            AlienSpecies::Empaths => "<3 <> <3 <> <3",
        }
    }

    pub fn translated_greeting(&self) -> &'static str {
        match self {
            AlienSpecies::Harmonics => "GREETING-harmony FRIEND-chord",
            AlienSpecies::Geometers => "WELCOME = /\\  TRADE = []",
            AlienSpecies::Empaths => "Curiosity-Peace-Wonder",
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            AlienSpecies::Harmonics => "M",
            AlienSpecies::Geometers => "/\\",
            AlienSpecies::Empaths => "<>",
        }
    }

    pub fn knowledge_gift(&self) -> u32 {
        match self {
            AlienSpecies::Harmonics => 25,
            AlienSpecies::Geometers => 35,
            AlienSpecies::Empaths => 20,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiplomaticStatus {
    Unknown,
    FirstContact,
    Cautious,
    Friendly,
    Allied,
}

impl DiplomaticStatus {
    pub fn name(&self) -> &'static str {
        match self {
            DiplomaticStatus::Unknown => "Unknown",
            DiplomaticStatus::FirstContact => "First Contact",
            DiplomaticStatus::Cautious => "Cautious",
            DiplomaticStatus::Friendly => "Friendly",
            DiplomaticStatus::Allied => "Allied",
        }
    }
}

// =============================================================================
// GAME STRUCTURES
// =============================================================================

#[derive(Debug, Clone)]
pub struct Planet {
    pub name: String,
    pub planet_type: PlanetType,
    pub has_life: bool,
    pub has_ruins: bool,
    pub scanned: bool,
    pub landed: bool,
    pub alien_species: Option<AlienSpecies>,
}

#[derive(Debug, Clone)]
pub struct StarSystem {
    pub name: String,
    pub star_type: StarType,
    pub x: i32,
    pub y: i32,
    pub planets: Vec<Planet>,
    pub visited: bool,
    pub fully_explored: bool,
}

#[derive(Debug, Clone)]
pub struct AlienContact {
    pub species: AlienSpecies,
    pub status: DiplomaticStatus,
    pub times_met: u32,
}

// =============================================================================
// GAME STATE
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CosmosView {
    #[default]
    Menu,
    GalaxyMap,
    StarSystem,
    PlanetSurface,
    FirstContact,
    Ship,
    Knowledge,
}

pub struct CosmosState {
    pub view: CosmosView,
    pub galaxy_seed: u64,
    pub systems: Vec<StarSystem>,
    pub current_system: usize,
    pub selected_system: usize,
    pub selected_planet: usize,
    pub current_planet: Option<usize>,

    // Ship stats
    pub fuel: u32,
    pub hull: u32,
    pub data_collected: u32, // Knowledge points

    // Alien relations
    pub contacts: Vec<AlienContact>,
    pub active_contact: Option<AlienSpecies>,
    pub contact_phase: u32,

    // Discovery tracking
    pub stars_explored: u32,
    pub planets_scanned: u32,
    pub planets_landed: u32,
    pub species_contacted: u32,
    pub ruins_discovered: u32,

    // UI state
    pub message: Option<String>,
    pub message_timer: u32,
    pub tick_count: u32,
    pub game_over: bool,

    events: Vec<GameEvent>,
}

impl Default for CosmosState {
    fn default() -> Self {
        Self::new()
    }
}

impl CosmosState {
    pub fn new() -> Self {
        Self {
            view: CosmosView::Menu,
            galaxy_seed: 0,
            systems: Vec::new(),
            current_system: 0,
            selected_system: 0,
            selected_planet: 0,
            current_planet: None,
            fuel: STARTING_FUEL,
            hull: STARTING_HULL,
            data_collected: 0,
            contacts: Vec::new(),
            active_contact: None,
            contact_phase: 0,
            stars_explored: 0,
            planets_scanned: 0,
            planets_landed: 0,
            species_contacted: 0,
            ruins_discovered: 0,
            message: None,
            message_timer: 0,
            tick_count: 0,
            game_over: false,
            events: Vec::new(),
        }
    }

    pub fn start_game(&mut self) {
        let mut rng = rand::thread_rng();
        self.galaxy_seed = rng.gen();
        self.generate_galaxy();
        self.view = CosmosView::GalaxyMap;
        self.fuel = STARTING_FUEL;
        self.hull = STARTING_HULL;
        self.data_collected = 0;
        self.contacts.clear();
        self.stars_explored = 0;
        self.planets_scanned = 0;
        self.planets_landed = 0;
        self.species_contacted = 0;
        self.ruins_discovered = 0;
        self.game_over = false;
        self.current_system = 0;
        self.selected_system = 0;
        self.systems[0].visited = true;
        self.stars_explored = 1;
    }

    fn generate_galaxy(&mut self) {
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.galaxy_seed);
        self.systems.clear();

        let star_names = [
            "Sol", "Proxima", "Vega", "Altair", "Sirius", "Rigel", "Betel", "Polaris", "Deneb",
            "Arcturus", "Capella", "Aldeb",
        ];

        for (i, star_name) in star_names.iter().enumerate().take(GALAXY_SIZE) {
            let star_type = match rng.gen_range(0..100) {
                0..=40 => StarType::MType,
                41..=60 => StarType::KType,
                61..=80 => StarType::GType,
                81..=90 => StarType::FType,
                91..=97 => StarType::Binary,
                _ => StarType::Neutron,
            };

            let num_planets = rng.gen_range(1..=MAX_PLANETS_PER_SYSTEM);
            let mut planets = Vec::new();

            for p in 0..num_planets {
                let planet_type = match rng.gen_range(0..100) {
                    0..=25 => PlanetType::Terrestrial,
                    26..=40 => PlanetType::GasGiant,
                    41..=55 => PlanetType::Barren,
                    56..=65 => PlanetType::Ice,
                    66..=75 => PlanetType::Desert,
                    76..=88 => PlanetType::Ocean,
                    _ => PlanetType::Volcanic,
                };

                let life_roll = rng.gen_range(0..100);
                let has_life =
                    life_roll < planet_type.life_chance() + star_type.habitability_bonus() as u32;

                let has_ruins = has_life && rng.gen_range(0..100) < 20;

                let alien_species = if has_life && rng.gen_range(0..100) < 30 {
                    Some(match rng.gen_range(0..3) {
                        0 => AlienSpecies::Harmonics,
                        1 => AlienSpecies::Geometers,
                        _ => AlienSpecies::Empaths,
                    })
                } else {
                    None
                };

                planets.push(Planet {
                    name: format!("{}-{}", star_name, (b'a' + p as u8) as char),
                    planet_type,
                    has_life,
                    has_ruins,
                    scanned: false,
                    landed: false,
                    alien_species,
                });
            }

            // Place systems in a rough grid with some randomness
            let grid_x = (i % 4) as i32;
            let grid_y = (i / 4) as i32;
            let x = grid_x * 18 + rng.gen_range(-3..=3);
            let y = grid_y * 6 + rng.gen_range(-1..=1);

            self.systems.push(StarSystem {
                name: star_name.to_string(),
                star_type,
                x,
                y,
                planets,
                visited: false,
                fully_explored: false,
            });
        }
    }

    pub fn warp_to_system(&mut self, target: usize) {
        if target >= self.systems.len() || target == self.current_system {
            return;
        }

        if self.fuel < WARP_FUEL_COST {
            self.show_message("Insufficient fuel for warp!");
            return;
        }

        self.fuel -= WARP_FUEL_COST;
        self.current_system = target;
        self.selected_planet = 0;
        self.current_planet = None;

        if !self.systems[target].visited {
            self.systems[target].visited = true;
            self.stars_explored += 1;
            self.data_collected += 10;
            self.show_message(&format!(
                "Arrived at {} system! (+10 data)",
                self.systems[target].name
            ));
        } else {
            self.show_message(&format!("Returned to {} system", self.systems[target].name));
        }

        self.view = CosmosView::StarSystem;
    }

    pub fn scan_planet(&mut self) {
        let system = &mut self.systems[self.current_system];
        if self.selected_planet >= system.planets.len() {
            return;
        }

        if self.fuel < SCAN_FUEL_COST {
            self.show_message("Insufficient fuel for scan!");
            return;
        }

        let planet = &mut system.planets[self.selected_planet];
        if planet.scanned {
            self.show_message("Planet already scanned");
            return;
        }

        self.fuel -= SCAN_FUEL_COST;
        planet.scanned = true;
        self.planets_scanned += 1;
        self.data_collected += 5;

        let mut msg = format!("{} scanned! (+5 data)", planet.name);
        if planet.has_life {
            msg.push_str(" LIFE DETECTED!");
            self.data_collected += 10;
        }
        if planet.has_ruins {
            msg.push_str(" RUINS!");
        }
        if planet.alien_species.is_some() {
            msg.push_str(" SIGNALS!");
        }
        self.show_message(&msg);
    }

    pub fn land_on_planet(&mut self) {
        let system = &self.systems[self.current_system];
        if self.selected_planet >= system.planets.len() {
            return;
        }

        let planet = &system.planets[self.selected_planet];
        if !planet.planet_type.can_land() {
            self.show_message("Cannot land on gas giant!");
            return;
        }

        // Need to scan first
        if !planet.scanned {
            self.show_message("Scan planet before landing!");
            return;
        }

        self.current_planet = Some(self.selected_planet);
        self.view = CosmosView::PlanetSurface;

        // Mark as landed and check for first landing bonus
        let planet = &self.systems[self.current_system].planets[self.selected_planet];
        let already_landed = planet.landed;
        let planet_name = planet.name.clone();
        let has_ruins = planet.has_ruins;

        if !already_landed {
            self.systems[self.current_system].planets[self.selected_planet].landed = true;
            self.planets_landed += 1;
            self.data_collected += 15;
            self.show_message(&format!("Landed on {}! (+15 data)", planet_name));

            if has_ruins {
                self.ruins_discovered += 1;
                self.data_collected += 50;
                self.show_message("Ancient ruins discovered! (+50 data)");
            }
        }
    }

    pub fn refuel_from_gas_giant(&mut self) {
        let system = &self.systems[self.current_system];
        if self.selected_planet >= system.planets.len() {
            return;
        }

        let planet = &system.planets[self.selected_planet];
        if planet.planet_type != PlanetType::GasGiant {
            self.show_message("Can only refuel from gas giants!");
            return;
        }

        let fuel_gained = planet.planet_type.fuel_available().min(100 - self.fuel);
        if fuel_gained == 0 {
            self.show_message("Fuel tanks already full!");
            return;
        }

        self.fuel += fuel_gained;
        self.show_message(&format!("Collected {} fuel from atmosphere!", fuel_gained));
    }

    pub fn initiate_contact(&mut self) {
        if let Some(planet_idx) = self.current_planet {
            let planet = &self.systems[self.current_system].planets[planet_idx];
            if let Some(species) = planet.alien_species {
                self.active_contact = Some(species);
                self.contact_phase = 0;
                self.view = CosmosView::FirstContact;

                // Emit sound event for alien contact
                let species_id = match species {
                    AlienSpecies::Harmonics => "harmonics",
                    AlienSpecies::Geometers => "geometers",
                    AlienSpecies::Empaths => "empaths",
                };
                self.events.push(GameEvent::AlienContact {
                    species: species_id.to_string(),
                });

                // Check if first contact with this species
                let already_met = self.contacts.iter().any(|c| c.species == species);
                if !already_met {
                    self.contacts.push(AlienContact {
                        species,
                        status: DiplomaticStatus::FirstContact,
                        times_met: 1,
                    });
                    self.species_contacted += 1;
                    self.data_collected += species.knowledge_gift();
                    self.show_message(&format!(
                        "First contact with {}! (+{} data)",
                        species.name(),
                        species.knowledge_gift()
                    ));
                } else {
                    // Update existing contact
                    if let Some(contact) = self.contacts.iter_mut().find(|c| c.species == species) {
                        contact.times_met += 1;
                        if contact.times_met >= 3
                            && contact.status == DiplomaticStatus::FirstContact
                        {
                            contact.status = DiplomaticStatus::Cautious;
                        }
                        if contact.times_met >= 5 && contact.status == DiplomaticStatus::Cautious {
                            contact.status = DiplomaticStatus::Friendly;
                        }
                        if contact.times_met >= 10 && contact.status == DiplomaticStatus::Friendly {
                            contact.status = DiplomaticStatus::Allied;
                            self.data_collected += 100;
                            self.show_message(&format!("Alliance formed with {}!", species.name()));
                        }
                    }
                }
            }
        }
    }

    pub fn check_system_explored(&mut self) {
        let system = &self.systems[self.current_system];
        if system.planets.iter().all(|p| p.scanned) && !system.fully_explored {
            self.systems[self.current_system].fully_explored = true;
            self.data_collected += 25;
            self.show_message("System fully explored! (+25 data)");
        }
    }

    pub fn get_diplomatic_status(&self, species: AlienSpecies) -> DiplomaticStatus {
        self.contacts
            .iter()
            .find(|c| c.species == species)
            .map(|c| c.status)
            .unwrap_or(DiplomaticStatus::Unknown)
    }

    pub fn show_message(&mut self, msg: &str) {
        self.message = Some(msg.to_string());
        self.message_timer = 90;
    }

    pub fn current_system(&self) -> &StarSystem {
        &self.systems[self.current_system]
    }

    pub fn exploration_percentage(&self) -> u32 {
        if self.systems.is_empty() {
            return 0;
        }
        let total_planets: usize = self.systems.iter().map(|s| s.planets.len()).sum();
        if total_planets == 0 {
            return 0;
        }
        (self.planets_scanned as usize * 100 / total_planets) as u32
    }
}

// =============================================================================
// GAME ENGINE IMPLEMENTATION
// =============================================================================

impl GameEngine for CosmosState {
    fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);

        if self.message_timer > 0 {
            self.message_timer -= 1;
            if self.message_timer == 0 {
                self.message = None;
            }
        }

        // Animate first contact sequence
        if self.view == CosmosView::FirstContact && self.tick_count.is_multiple_of(10) {
            self.contact_phase = (self.contact_phase + 1) % 4;
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match self.view {
            CosmosView::Menu => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.start_game();
                    KeyHandleResult::Handled
                }
                KeyCode::Esc | KeyCode::Char('q') => KeyHandleResult::RequestQuit,
                _ => KeyHandleResult::NotHandled,
            },

            CosmosView::GalaxyMap => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.selected_system >= 4 {
                        self.selected_system -= 4;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.selected_system + 4 < self.systems.len() {
                        self.selected_system += 4;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    if self.selected_system > 0 {
                        self.selected_system -= 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    if self.selected_system + 1 < self.systems.len() {
                        self.selected_system += 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Enter | KeyCode::Char('w') => {
                    let target = self.selected_system;
                    self.warp_to_system(target);
                    KeyHandleResult::Handled
                }
                KeyCode::Char('s') => {
                    self.view = CosmosView::Ship;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('i') => {
                    self.view = CosmosView::Knowledge;
                    KeyHandleResult::Handled
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.game_over = true;
                    KeyHandleResult::GameOver
                }
                _ => KeyHandleResult::NotHandled,
            },

            CosmosView::StarSystem => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.selected_planet > 0 {
                        self.selected_planet -= 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let max = self.systems[self.current_system].planets.len();
                    if self.selected_planet + 1 < max {
                        self.selected_planet += 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Char('s') => {
                    self.scan_planet();
                    self.check_system_explored();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('l') | KeyCode::Enter => {
                    self.land_on_planet();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('r') => {
                    self.refuel_from_gas_giant();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('m') => {
                    self.view = CosmosView::GalaxyMap;
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => {
                    self.view = CosmosView::GalaxyMap;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::NotHandled,
            },

            CosmosView::PlanetSurface => match key.code {
                KeyCode::Char('c') => {
                    self.initiate_contact();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('o') | KeyCode::Esc => {
                    self.current_planet = None;
                    self.view = CosmosView::StarSystem;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::NotHandled,
            },

            CosmosView::FirstContact => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.active_contact = None;
                    self.view = CosmosView::PlanetSurface;
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => {
                    self.active_contact = None;
                    self.view = CosmosView::PlanetSurface;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::NotHandled,
            },

            CosmosView::Ship | CosmosView::Knowledge => match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.view = CosmosView::GalaxyMap;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::NotHandled,
            },
        }
    }

    fn get_score(&self) -> u32 {
        self.data_collected
    }

    fn is_game_over(&self) -> bool {
        self.game_over
    }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }
}
