//! MICROPOLIS - ASCII City Builder
//!
//! SimCity meets Monopoly in a horizontal-scrolling city builder.
//! Buy properties, collect rent, survive disasters, build your empire!

use super::platform::{GameEngine, GameEvent, KeyHandleResult};
use crossterm::event::{KeyCode, KeyEvent};
use rand::Rng;

// =============================================================================
// CONSTANTS
// =============================================================================

const CITY_WIDTH: usize = 50;
const VIEWPORT_WIDTH: usize = 40;
const STARTING_CASH: i64 = 50_000;

// Property economics
const EMPTY_PRICE: i64 = 1_000;
const HOUSE_PRICE: i64 = 10_000;
const SHOP_PRICE: i64 = 25_000;
const FACTORY_PRICE: i64 = 50_000;
const PARK_PRICE: i64 = 5_000;

const HOUSE_RENT: i64 = 100;
const SHOP_RENT: i64 = 300;
const FACTORY_RENT: i64 = 800;

const HOUSE_MAINT: i64 = 5;
const SHOP_MAINT: i64 = 20;
const FACTORY_MAINT: i64 = 100;
const PARK_MAINT: i64 = 10;

const NPC_MARKUP: f64 = 1.5;
const FIRE_CHANCE: f64 = 0.05;
const FIRE_SPREAD_CHANCE: f64 = 0.30;
const FIRE_DAMAGE: u8 = 10;
const REPAIR_COST_RATIO: f64 = 0.25;
const MIN_CONDITION: u8 = 50;

// =============================================================================
// ENUMS
// =============================================================================

/// Property types available in the game
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PropertyType {
    #[default]
    Empty,
    House,
    Shop,
    Factory,
    Park,
}

impl PropertyType {
    pub fn symbol(&self) -> char {
        match self {
            PropertyType::Empty => '.',
            PropertyType::House => '⌂',
            PropertyType::Shop => '$',
            PropertyType::Factory => '*',
            PropertyType::Park => '♣',
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            PropertyType::Empty => "Empty Lot",
            PropertyType::House => "House",
            PropertyType::Shop => "Shop",
            PropertyType::Factory => "Factory",
            PropertyType::Park => "Park",
        }
    }

    pub fn base_price(&self) -> i64 {
        match self {
            PropertyType::Empty => EMPTY_PRICE,
            PropertyType::House => HOUSE_PRICE,
            PropertyType::Shop => SHOP_PRICE,
            PropertyType::Factory => FACTORY_PRICE,
            PropertyType::Park => PARK_PRICE,
        }
    }

    pub fn daily_rent(&self) -> i64 {
        match self {
            PropertyType::Empty => 0,
            PropertyType::House => HOUSE_RENT,
            PropertyType::Shop => SHOP_RENT,
            PropertyType::Factory => FACTORY_RENT,
            PropertyType::Park => 0,
        }
    }

    pub fn daily_maintenance(&self) -> i64 {
        match self {
            PropertyType::Empty => 0,
            PropertyType::House => HOUSE_MAINT,
            PropertyType::Shop => SHOP_MAINT,
            PropertyType::Factory => FACTORY_MAINT,
            PropertyType::Park => PARK_MAINT,
        }
    }

    pub fn fire_risk_multiplier(&self) -> f64 {
        match self {
            PropertyType::Empty => 0.0,
            PropertyType::House => 1.0,
            PropertyType::Shop => 1.5,
            PropertyType::Factory => 3.0,
            PropertyType::Park => 0.0, // Parks don't catch fire
        }
    }

    pub fn blocks_fire(&self) -> bool {
        matches!(self, PropertyType::Park | PropertyType::Empty)
    }
}

/// Property ownership
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Owner {
    #[default]
    None,
    Player,
    Npc,
}

/// Current view state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MicropolisView {
    #[default]
    Menu,
    City,
    Buy,
    Status,
    Disaster,
    GameOver,
}

// =============================================================================
// PROPERTY
// =============================================================================

/// A single property in the city
#[derive(Debug, Clone)]
pub struct Property {
    pub property_type: PropertyType,
    pub owner: Owner,
    pub condition: u8,
    pub on_fire: bool,
}

impl Default for Property {
    fn default() -> Self {
        Self {
            property_type: PropertyType::Empty,
            owner: Owner::None,
            condition: 100,
            on_fire: false,
        }
    }
}

impl Property {
    pub fn new(property_type: PropertyType, owner: Owner) -> Self {
        Self {
            property_type,
            owner,
            condition: 100,
            on_fire: false,
        }
    }

    /// Calculate current value based on type and condition
    pub fn value(&self) -> i64 {
        let base = self.property_type.base_price();
        (base as f64 * (self.condition as f64 / 100.0)) as i64
    }

    /// Calculate rent based on condition (no rent if on fire)
    pub fn rent(&self) -> i64 {
        if self.on_fire {
            return 0;
        }
        let base = self.property_type.daily_rent();
        (base as f64 * (self.condition as f64 / 100.0)) as i64
    }

    /// Cost to buy this property
    pub fn buy_cost(&self) -> i64 {
        let base = self.property_type.base_price();
        match self.owner {
            Owner::None => base,
            Owner::Npc => (base as f64 * NPC_MARKUP) as i64,
            Owner::Player => 0, // Already owned
        }
    }

    /// Cost to repair this property
    pub fn repair_cost(&self) -> i64 {
        if self.condition >= 100 {
            return 0;
        }
        let base = self.property_type.base_price();
        let damage_ratio = (100 - self.condition) as f64 / 100.0;
        (base as f64 * REPAIR_COST_RATIO * damage_ratio) as i64
    }
}

// =============================================================================
// GAME STATE
// =============================================================================

/// NYC street names
const NYC_STREETS: &[&str] = &[
    "Wall St",
    "Broadway",
    "5th Ave",
    "Park Ave",
    "Madison",
    "Lexington",
    "3rd Ave",
    "2nd Ave",
    "1st Ave",
    "York Ave",
    "FDR Dr",
    "42nd St",
    "34th St",
    "23rd St",
    "14th St",
    "Houston",
    "Canal St",
    "Chambers",
    "Fulton",
    "Liberty",
];

/// Main game state
pub struct MicropolisState {
    // View state
    pub view: MicropolisView,

    // City data
    pub city_name: &'static str,
    pub properties: Vec<Property>,

    // Player state
    pub cash: i64,
    pub day: u32,

    // Viewport
    pub camera_x: usize,
    pub cursor_x: usize,

    // Buy dialog
    pub buy_selection: usize,

    // Game state
    pub message: Option<String>,
    pub game_over: bool,

    // Animation
    pub tick_count: u32,

    // Events
    pending_events: Vec<GameEvent>,
}

impl Default for MicropolisState {
    fn default() -> Self {
        Self::new()
    }
}

impl MicropolisState {
    pub fn new() -> Self {
        Self {
            view: MicropolisView::Menu,
            city_name: "New York City",
            properties: Vec::new(),
            cash: STARTING_CASH,
            day: 1,
            camera_x: 0,
            cursor_x: 0,
            buy_selection: 0,
            message: None,
            game_over: false,
            tick_count: 0,
            pending_events: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.view = MicropolisView::City;
        self.cash = STARTING_CASH;
        self.day = 1;
        self.camera_x = 0;
        self.cursor_x = CITY_WIDTH / 2; // Start in middle
        self.buy_selection = 0;
        self.message = None;
        self.game_over = false;
        self.tick_count = 0;
        self.generate_city();
    }

    /// Generate initial city with mix of properties
    fn generate_city(&mut self) {
        let mut rng = rand::thread_rng();
        self.properties.clear();

        for _ in 0..CITY_WIDTH {
            let roll: u32 = rng.gen_range(0..100);
            let (property_type, owner) = if roll < 30 {
                // 30% empty lots
                (PropertyType::Empty, Owner::None)
            } else if roll < 50 {
                // 20% NPC houses
                (PropertyType::House, Owner::Npc)
            } else if roll < 65 {
                // 15% NPC shops
                (PropertyType::Shop, Owner::Npc)
            } else if roll < 75 {
                // 10% NPC factories
                (PropertyType::Factory, Owner::Npc)
            } else if roll < 85 {
                // 10% parks (city-owned, can't buy)
                (PropertyType::Park, Owner::Npc)
            } else {
                // 15% available empty lots
                (PropertyType::Empty, Owner::None)
            };

            self.properties.push(Property::new(property_type, owner));
        }
    }

    /// Get street name for a position
    pub fn street_name(&self, x: usize) -> &'static str {
        let idx = (x / 5) % NYC_STREETS.len();
        NYC_STREETS[idx]
    }

    /// Get the currently selected property
    pub fn selected_property(&self) -> Option<&Property> {
        self.properties.get(self.cursor_x)
    }

    /// Get the currently selected property mutably
    pub fn selected_property_mut(&mut self) -> Option<&mut Property> {
        self.properties.get_mut(self.cursor_x)
    }

    // =========================================================================
    // NAVIGATION
    // =========================================================================

    fn move_cursor_left(&mut self) {
        if self.cursor_x > 0 {
            self.cursor_x -= 1;
            self.update_camera();
        }
    }

    fn move_cursor_right(&mut self) {
        if self.cursor_x < CITY_WIDTH - 1 {
            self.cursor_x += 1;
            self.update_camera();
        }
    }

    fn update_camera(&mut self) {
        // Keep cursor near center of viewport
        let margin = VIEWPORT_WIDTH / 4;

        if self.cursor_x < self.camera_x + margin {
            self.camera_x = self.cursor_x.saturating_sub(margin);
        } else if self.cursor_x > self.camera_x + VIEWPORT_WIDTH - margin {
            self.camera_x = (self.cursor_x + margin).saturating_sub(VIEWPORT_WIDTH);
        }

        // Clamp camera to valid range
        self.camera_x = self.camera_x.min(CITY_WIDTH.saturating_sub(VIEWPORT_WIDTH));
    }

    // =========================================================================
    // ECONOMY
    // =========================================================================

    /// Count player-owned properties
    pub fn player_property_count(&self) -> usize {
        self.properties
            .iter()
            .filter(|p| p.owner == Owner::Player)
            .count()
    }

    /// Calculate total daily income
    pub fn daily_income(&self) -> i64 {
        self.properties
            .iter()
            .filter(|p| p.owner == Owner::Player)
            .map(|p| p.rent())
            .sum()
    }

    /// Calculate total daily expenses
    pub fn daily_expenses(&self) -> i64 {
        self.properties
            .iter()
            .filter(|p| p.owner == Owner::Player)
            .map(|p| p.property_type.daily_maintenance())
            .sum()
    }

    /// Calculate net worth (score)
    pub fn net_worth(&self) -> i64 {
        let property_value: i64 = self
            .properties
            .iter()
            .filter(|p| p.owner == Owner::Player)
            .map(|p| p.value())
            .sum();
        self.cash + property_value
    }

    /// Calculate fire risk level
    pub fn fire_risk(&self) -> &'static str {
        let factory_count = self
            .properties
            .iter()
            .filter(|p| p.owner == Owner::Player && p.property_type == PropertyType::Factory)
            .count();
        let park_count = self
            .properties
            .iter()
            .filter(|p| p.owner == Owner::Player && p.property_type == PropertyType::Park)
            .count();

        if factory_count == 0 {
            "NONE"
        } else if park_count >= factory_count {
            "LOW"
        } else if factory_count <= 2 {
            "MEDIUM"
        } else {
            "HIGH"
        }
    }

    // =========================================================================
    // PURCHASE
    // =========================================================================

    fn try_buy_property(&mut self) {
        // Extract needed info first to avoid borrow conflicts
        let (owner, cost, property_type, is_empty) = {
            if let Some(prop) = self.properties.get(self.cursor_x) {
                (
                    prop.owner,
                    prop.buy_cost(),
                    prop.property_type,
                    prop.property_type == PropertyType::Empty,
                )
            } else {
                return;
            }
        };

        if owner == Owner::Player {
            self.message = Some("You already own this property!".to_string());
            return;
        }

        if cost > self.cash {
            self.message = Some(format!("Not enough cash! Need ${}", cost));
            return;
        }

        // If empty lot, show build menu
        if is_empty {
            self.buy_selection = 0;
            self.view = MicropolisView::Buy;
            return;
        }

        // Buy existing property
        self.cash -= cost;
        if let Some(prop) = self.properties.get_mut(self.cursor_x) {
            prop.owner = Owner::Player;
        }
        self.message = Some(format!("Purchased {} for ${}!", property_type.name(), cost));
    }

    fn build_on_lot(&mut self, building_type: PropertyType) {
        let cost = building_type.base_price();
        if cost > self.cash {
            self.message = Some(format!("Not enough cash! Need ${}", cost));
            return;
        }

        self.cash -= cost;
        if let Some(prop) = self.properties.get_mut(self.cursor_x) {
            prop.property_type = building_type;
            prop.owner = Owner::Player;
            prop.condition = 100;
        }
        self.message = Some(format!("Built {} for ${}!", building_type.name(), cost));
        self.view = MicropolisView::City;
    }

    fn try_repair_property(&mut self) {
        if let Some(prop) = self.properties.get(self.cursor_x) {
            if prop.owner != Owner::Player {
                self.message = Some("You don't own this property!".to_string());
                return;
            }

            if prop.condition >= 100 && !prop.on_fire {
                self.message = Some("Property is in perfect condition!".to_string());
                return;
            }

            let cost = prop.repair_cost();
            if prop.on_fire {
                // Extinguish fire first
                if cost > self.cash {
                    self.message = Some(format!("Not enough cash to repair! Need ${}", cost));
                    return;
                }
            }

            if cost > self.cash {
                self.message = Some(format!("Not enough cash! Need ${}", cost));
                return;
            }

            self.cash -= cost;
            if let Some(prop) = self.properties.get_mut(self.cursor_x) {
                prop.on_fire = false;
                prop.condition = 100;
            }
            self.message = Some(format!("Repaired for ${}!", cost));
        }
    }

    // =========================================================================
    // DAY PROGRESSION
    // =========================================================================

    fn advance_day(&mut self) {
        self.day += 1;

        // Collect rent
        let income = self.daily_income();
        self.cash += income;

        // Pay maintenance
        let expenses = self.daily_expenses();
        self.cash -= expenses;

        // Degrade properties
        for prop in &mut self.properties {
            if prop.owner == Owner::Player && prop.condition > MIN_CONDITION {
                prop.condition = prop.condition.saturating_sub(1);
            }
        }

        // Process fires
        self.process_fires();

        // Check for new fire
        self.check_for_fire();

        // Check bankruptcy
        if self.cash < 0 {
            self.game_over = true;
            self.view = MicropolisView::GameOver;
            self.message = Some("BANKRUPT! Game Over!".to_string());
            return;
        }

        // Update message
        let net = income - expenses;
        if net >= 0 {
            self.message = Some(format!(
                "Day {}: +${} income, -${} expenses",
                self.day, income, expenses
            ));
        } else {
            self.message = Some(format!("Day {}: -${} net loss!", self.day, net.abs()));
        }
    }

    // =========================================================================
    // FIRE SYSTEM
    // =========================================================================

    fn check_for_fire(&mut self) {
        let mut rng = rand::thread_rng();

        // Check each property for fire start
        for i in 0..self.properties.len() {
            let prop = &self.properties[i];
            if prop.on_fire || prop.property_type.blocks_fire() {
                continue;
            }

            let risk = FIRE_CHANCE * prop.property_type.fire_risk_multiplier();
            if rng.gen_bool(risk) {
                self.properties[i].on_fire = true;
                self.message = Some(format!(
                    "FIRE! A {} at {} is burning!",
                    self.properties[i].property_type.name(),
                    self.street_name(i)
                ));
                self.view = MicropolisView::Disaster;
                return;
            }
        }
    }

    fn process_fires(&mut self) {
        let mut rng = rand::thread_rng();
        let mut new_fires = Vec::new();

        for i in 0..self.properties.len() {
            let prop = &mut self.properties[i];
            if !prop.on_fire {
                continue;
            }

            // Damage burning property
            prop.condition = prop.condition.saturating_sub(FIRE_DAMAGE);

            // If destroyed, convert to empty lot
            if prop.condition == 0 {
                prop.property_type = PropertyType::Empty;
                prop.owner = Owner::None;
                prop.on_fire = false;
                prop.condition = 100;
                continue;
            }

            // Check spread to neighbors
            if i > 0
                && !self.properties[i - 1].property_type.blocks_fire()
                && !self.properties[i - 1].on_fire
                && rng.gen_bool(FIRE_SPREAD_CHANCE)
            {
                new_fires.push(i - 1);
            }
            if i < self.properties.len() - 1
                && !self.properties[i + 1].property_type.blocks_fire()
                && !self.properties[i + 1].on_fire
                && rng.gen_bool(FIRE_SPREAD_CHANCE)
            {
                new_fires.push(i + 1);
            }
        }

        // Apply new fires
        for idx in new_fires {
            self.properties[idx].on_fire = true;
        }
    }

    /// Count active fires
    pub fn active_fire_count(&self) -> usize {
        self.properties.iter().filter(|p| p.on_fire).count()
    }
}

// =============================================================================
// GAME ENGINE TRAIT
// =============================================================================

impl GameEngine for MicropolisState {
    fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
        // No automatic progression - player controls time with N key
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        // Clear message on any key
        if self.message.is_some() && self.view != MicropolisView::Disaster {
            self.message = None;
        }

        match self.view {
            MicropolisView::Menu => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.reset();
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => KeyHandleResult::RequestQuit,
                _ => KeyHandleResult::Handled,
            },

            MicropolisView::City => match key.code {
                KeyCode::Esc => KeyHandleResult::RequestQuit,
                KeyCode::Char('p') | KeyCode::Char('P') => KeyHandleResult::RequestPause,

                // Navigation
                KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') => {
                    self.move_cursor_left();
                    KeyHandleResult::Handled
                }
                KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('D') => {
                    self.move_cursor_right();
                    KeyHandleResult::Handled
                }

                // Actions
                KeyCode::Char('b') | KeyCode::Char('B') => {
                    self.try_buy_property();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Char(' ') => {
                    self.advance_day();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.view = MicropolisView::Status;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    self.try_repair_property();
                    KeyHandleResult::Handled
                }

                _ => KeyHandleResult::Handled,
            },

            MicropolisView::Buy => match key.code {
                KeyCode::Esc => {
                    self.view = MicropolisView::City;
                    KeyHandleResult::Handled
                }
                KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                    if self.buy_selection > 0 {
                        self.buy_selection -= 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => {
                    if self.buy_selection < 3 {
                        self.buy_selection += 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Char('1') => {
                    self.build_on_lot(PropertyType::House);
                    KeyHandleResult::Handled
                }
                KeyCode::Char('2') => {
                    self.build_on_lot(PropertyType::Shop);
                    KeyHandleResult::Handled
                }
                KeyCode::Char('3') => {
                    self.build_on_lot(PropertyType::Factory);
                    KeyHandleResult::Handled
                }
                KeyCode::Char('4') => {
                    self.build_on_lot(PropertyType::Park);
                    KeyHandleResult::Handled
                }
                KeyCode::Enter => {
                    let building = match self.buy_selection {
                        0 => PropertyType::House,
                        1 => PropertyType::Shop,
                        2 => PropertyType::Factory,
                        _ => PropertyType::Park,
                    };
                    self.build_on_lot(building);
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },

            MicropolisView::Status => match key.code {
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ') => {
                    self.view = MicropolisView::City;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },

            MicropolisView::Disaster => match key.code {
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ') => {
                    self.message = None;
                    self.view = MicropolisView::City;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },

            MicropolisView::GameOver => match key.code {
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
        self.net_worth().max(0) as u32
    }

    fn is_game_over(&self) -> bool {
        self.game_over
    }

    fn is_game_won(&self) -> bool {
        false // Endless game
    }

    fn get_level(&self) -> Option<u32> {
        Some(self.day)
    }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.pending_events)
    }
}
