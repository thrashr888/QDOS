//! Dope Wars game implementation
//!
//! A classic drug trading game - buy low, sell high, pay off your debt,
//! and maximize your net worth over 30 days.

use super::platform::{GameEngine, GameEvent, KeyHandleResult};
use crossterm::event::{KeyCode, KeyEvent};
use rand::Rng;
use serde::{Deserialize, Serialize};

const MAX_DAYS: u32 = 30;
const STARTING_CASH: i64 = 2000;
const STARTING_DEBT: i64 = 5000;
const COAT_CAPACITY: u32 = 100;
const INTEREST_RATE: f64 = 0.10; // 10% per day

/// Product types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum Product {
    Acid,
    Cocaine,
    Hashish,
    Heroin,
    MDA,
    Opium,
}

impl Product {
    pub fn all() -> &'static [Product] {
        &[
            Product::Acid,
            Product::Cocaine,
            Product::Hashish,
            Product::Heroin,
            Product::MDA,
            Product::Opium,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Product::Acid => "Acid",
            Product::Cocaine => "Cocaine",
            Product::Hashish => "Hashish",
            Product::Heroin => "Heroin",
            Product::MDA => "MDA",
            Product::Opium => "Opium",
        }
    }

    /// Base price range for the product
    pub fn base_price_range(&self) -> (i64, i64) {
        match self {
            Product::Acid => (1000, 4500),
            Product::Cocaine => (15000, 30000),
            Product::Hashish => (480, 1280),
            Product::Heroin => (5500, 13000),
            Product::MDA => (1500, 4400),
            Product::Opium => (540, 1250),
        }
    }
}

/// Locations in the game
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Location {
    Bronx,
    Brooklyn,
    Manhattan,
    Queens,
    StatenIsland,
    CentralPark,
}

impl Location {
    pub fn all() -> &'static [Location] {
        &[
            Location::Bronx,
            Location::Brooklyn,
            Location::Manhattan,
            Location::Queens,
            Location::StatenIsland,
            Location::CentralPark,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Location::Bronx => "Bronx",
            Location::Brooklyn => "Brooklyn",
            Location::Manhattan => "Manhattan",
            Location::Queens => "Queens",
            Location::StatenIsland => "Staten Island",
            Location::CentralPark => "Central Park",
        }
    }
}

/// Current market prices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Market {
    pub prices: Vec<(Product, Option<i64>)>, // None = not available
}

impl Market {
    pub fn new(_location: Location) -> Self {
        let mut rng = rand::thread_rng();
        let mut prices = Vec::new();

        for &product in Product::all() {
            let (min, max) = product.base_price_range();

            // Random chance product is not available (30%)
            let available = rng.gen_bool(0.7);

            let price = if available {
                // Random price in range, with occasional spikes/crashes
                let base = rng.gen_range(min..=max);
                let spike = rng.gen_range(0..100);

                let price = if spike < 5 {
                    // 5% chance of crash (very cheap)
                    base / 4
                } else if spike < 10 {
                    // 5% chance of spike (very expensive)
                    base * 4
                } else {
                    base
                };
                Some(price)
            } else {
                None
            };

            prices.push((product, price));
        }

        Self { prices }
    }

    pub fn get_price(&self, product: Product) -> Option<i64> {
        self.prices
            .iter()
            .find(|(p, _)| *p == product)
            .and_then(|(_, price)| *price)
    }
}

/// Player inventory
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Inventory {
    pub items: Vec<(Product, u32)>,
}

impl Inventory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_quantity(&self, product: Product) -> u32 {
        self.items
            .iter()
            .find(|(p, _)| *p == product)
            .map(|(_, q)| *q)
            .unwrap_or(0)
    }

    pub fn add(&mut self, product: Product, quantity: u32) {
        if let Some(item) = self.items.iter_mut().find(|(p, _)| *p == product) {
            item.1 += quantity;
        } else {
            self.items.push((product, quantity));
        }
    }

    pub fn remove(&mut self, product: Product, quantity: u32) -> bool {
        if let Some(item) = self.items.iter_mut().find(|(p, _)| *p == product) {
            if item.1 >= quantity {
                item.1 -= quantity;
                if item.1 == 0 {
                    self.items.retain(|(p, _)| *p != product);
                }
                return true;
            }
        }
        false
    }

    pub fn total_items(&self) -> u32 {
        self.items.iter().map(|(_, q)| *q).sum()
    }
}

/// Game view states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DopeWarsView {
    Market,
    Travel,
    Status,
    Event,
}

/// Random events
#[derive(Debug, Clone)]
pub enum RandomEvent {
    None,
    CopsRaid { escaped: bool, damage: u32 },
    FindStash { product: Product, quantity: u32 },
    Mugged { amount: i64, damage: u32 },
    LoanShark { paid_off: i64 },
    GunShop,
    OfficerOffer { bribe: i64 },
}

/// Dope Wars game state
pub struct DopeWarsState {
    pub view: DopeWarsView,
    pub day: u32,
    pub cash: i64,
    pub debt: i64,
    pub health: u32,
    pub guns: u32,
    pub location: Location,
    pub inventory: Inventory,
    pub market: Market,
    pub selected_product: usize,
    pub selected_location: usize,
    pub quantity_buffer: String,
    pub message: Option<String>,
    pub event: RandomEvent,
    pub game_over: bool,
    pub tick_count: u32,
    pending_events: Vec<GameEvent>,
}

impl Default for DopeWarsState {
    fn default() -> Self {
        Self::new()
    }
}

impl DopeWarsState {
    pub fn new() -> Self {
        let location = Location::Bronx;
        let market = Market::new(location);

        Self {
            view: DopeWarsView::Market,
            day: 1,
            cash: STARTING_CASH,
            debt: STARTING_DEBT,
            health: 100,
            guns: 0,
            location,
            inventory: Inventory::new(),
            market,
            selected_product: 0,
            selected_location: 0,
            quantity_buffer: String::new(),
            message: Some(
                "Welcome to Dope Wars! Buy low, sell high, pay off your debt.".to_string(),
            ),
            event: RandomEvent::None,
            game_over: false,
            tick_count: 0,
            pending_events: vec![GameEvent::GameStarted],
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Calculate final score (net worth)
    pub fn score(&self) -> u32 {
        let inventory_value: i64 = self
            .inventory
            .items
            .iter()
            .map(|(product, quantity)| {
                let (min, max) = product.base_price_range();
                let avg_price = (min + max) / 2;
                avg_price * (*quantity as i64)
            })
            .sum();

        let net_worth = self.cash + inventory_value - self.debt;
        net_worth.max(0) as u32
    }

    /// Buy product from market
    pub fn buy(&mut self, product: Product, quantity: u32) {
        if let Some(price) = self.market.get_price(product) {
            let cost = price * (quantity as i64);
            let space_available = COAT_CAPACITY - self.inventory.total_items();

            if quantity > space_available {
                self.message = Some(format!(
                    "Not enough space! Only {} slots available.",
                    space_available
                ));
                return;
            }

            if cost > self.cash {
                self.message = Some("Not enough cash!".to_string());
                return;
            }

            self.cash -= cost;
            self.inventory.add(product, quantity);
            self.message = Some(format!(
                "Bought {} {} for ${}",
                quantity,
                product.name(),
                cost
            ));
            self.quantity_buffer.clear();
        }
    }

    /// Sell product to market
    pub fn sell(&mut self, product: Product, quantity: u32) {
        if let Some(price) = self.market.get_price(product) {
            let have = self.inventory.get_quantity(product);
            let quantity = quantity.min(have);

            if quantity == 0 {
                self.message = Some("You don't have any of that!".to_string());
                return;
            }

            if self.inventory.remove(product, quantity) {
                let revenue = price * (quantity as i64);
                self.cash += revenue;
                self.message = Some(format!(
                    "Sold {} {} for ${}",
                    quantity,
                    product.name(),
                    revenue
                ));
                self.quantity_buffer.clear();
            }
        }
    }

    /// Travel to a new location
    pub fn travel(&mut self) {
        let locations = Location::all();
        let new_location = locations[self.selected_location];

        if new_location == self.location {
            self.message = Some("You're already here!".to_string());
            return;
        }

        self.location = new_location;
        self.day += 1;

        // Apply interest on debt
        if self.debt > 0 {
            let interest = (self.debt as f64 * INTEREST_RATE) as i64;
            self.debt += interest;
        }

        // Generate new market
        self.market = Market::new(self.location);

        // Random events
        self.check_random_events();

        // Check if game is over
        if self.day > MAX_DAYS {
            self.game_over = true;
        } else if self.view != DopeWarsView::Event {
            // Only set arrival message if no event occurred
            self.message = Some(format!(
                "Day {}: Arrived at {}",
                self.day,
                self.location.name()
            ));
        }

        // Only return to market if no event is showing
        if self.view != DopeWarsView::Event {
            self.view = DopeWarsView::Market;
        }
    }

    fn check_random_events(&mut self) {
        let mut rng = rand::thread_rng();
        let roll = rng.gen_range(0..100);

        // 70% chance of some event happening (was 50%, increasing to make events more visible)
        if roll < 70 {
            let event_roll = rng.gen_range(0..100);

            if event_roll < 20 && self.inventory.total_items() > 0 {
                // 20% of events = Cops raid
                let has_guns = self.guns > 0;
                let escaped = has_guns && rng.gen_bool(0.5 + (self.guns as f64 * 0.05));
                let damage = if !escaped {
                    rng.gen_range(10..=30)
                } else {
                    rng.gen_range(0..=10)
                };

                self.health = self.health.saturating_sub(damage);

                if !escaped {
                    let lost_amount = rng.gen_range(0.3..=0.7);
                    let mut items_to_lose = Vec::new();
                    for (product, qty) in &self.inventory.items {
                        let lose_qty = ((*qty as f64) * lost_amount) as u32;
                        if lose_qty > 0 {
                            items_to_lose.push((*product, lose_qty));
                        }
                    }
                    for (product, qty) in items_to_lose {
                        self.inventory.remove(product, qty);
                    }
                    self.message = Some(format!(
                        "Officer Hardass raided you! Lost some stash and took {} damage!",
                        damage
                    ));
                    self.event = RandomEvent::CopsRaid {
                        escaped: false,
                        damage,
                    };
                } else {
                    self.message = Some(format!("Officer Hardass tried to raid you but you fought back with your guns! -{} HP", damage));
                    self.event = RandomEvent::CopsRaid {
                        escaped: true,
                        damage,
                    };
                }

                self.view = DopeWarsView::Event;

                if self.health == 0 {
                    self.game_over = true;
                    self.message = Some("You died from your injuries!".to_string());
                }
            } else if event_roll < 35 {
                // 15% of events = Find stash
                let products = Product::all();
                let product = products[rng.gen_range(0..products.len())];
                let quantity = rng.gen_range(10..=30);

                if self.inventory.total_items() + quantity <= COAT_CAPACITY {
                    self.inventory.add(product, quantity);
                    self.message = Some(format!(
                        "You found {} {} on a dead dude in the subway!",
                        quantity,
                        product.name()
                    ));
                    self.event = RandomEvent::FindStash { product, quantity };
                } else {
                    self.message = Some(
                        "You found drugs on a dead guy but your trenchcoat is full!".to_string(),
                    );
                    self.event = RandomEvent::None;
                }
                self.view = DopeWarsView::Event;
            } else if event_roll < 50 && self.cash > 500 {
                // 15% of events = Mugged
                let stolen = rng.gen_range(200..=800).min(self.cash);
                let damage = if self.guns > 0 {
                    rng.gen_range(0..=5)
                } else {
                    rng.gen_range(10..=25)
                };

                self.cash -= stolen;
                self.health = self.health.saturating_sub(damage);

                if self.guns > 0 {
                    self.message = Some(format!(
                        "Thugs tried to mug you! You fought back but lost ${} and {} HP",
                        stolen, damage
                    ));
                } else {
                    self.message = Some(format!(
                        "Thugs mugged you! Lost ${} and took {} damage",
                        stolen, damage
                    ));
                }

                self.event = RandomEvent::Mugged {
                    amount: stolen,
                    damage,
                };
                self.view = DopeWarsView::Event;

                if self.health == 0 {
                    self.game_over = true;
                    self.message = Some("You were beaten to death!".to_string());
                }
            } else if event_roll < 65 {
                // 15% of events = Gun shop
                self.message =
                    Some("You meet a trenchcoat dealer offering guns! Press G to buy.".to_string());
                self.event = RandomEvent::GunShop;
                self.view = DopeWarsView::Event;
            } else if event_roll < 80 && self.cash > 1000 && self.inventory.total_items() > 0 {
                // 15% of events = Officer offers bribe
                let bribe = rng.gen_range(1000..=3000).min(self.cash);
                self.message = Some(format!(
                    "Officer Hardass: I need ${} or I'm taking you in!",
                    bribe
                ));
                self.event = RandomEvent::OfficerOffer { bribe };
                self.view = DopeWarsView::Event;
            } else if event_roll < 95 && self.debt > 1000 {
                // 15% of events = Loan shark deal
                let payment = rng.gen_range(800..=1500).min(self.cash);
                if payment > 0 {
                    self.cash -= payment;
                    self.debt -= payment * 2;
                    self.debt = self.debt.max(0);
                    self.message = Some(format!(
                        "The loan shark liked your face. Paid ${}, debt reduced by ${}",
                        payment,
                        payment * 2
                    ));
                    self.event = RandomEvent::LoanShark { paid_off: payment };
                    self.view = DopeWarsView::Event;
                }
            } else {
                // 5% of events = Free health
                let heal = rng.gen_range(20..=40);
                self.health = (self.health + heal).min(100);
                self.message = Some(format!(
                    "A nice old lady bandages your wounds. +{} HP",
                    heal
                ));
                self.event = RandomEvent::None;
                self.view = DopeWarsView::Event;
            }
        }
    }

    pub fn buy_guns(&mut self) {
        if matches!(self.event, RandomEvent::GunShop) {
            let gun_price = 400;
            let max_guns = self.cash / gun_price;
            let guns_to_buy = max_guns.min(10 - self.guns as i64) as u32;

            if guns_to_buy > 0 {
                let cost = guns_to_buy as i64 * gun_price;
                self.cash -= cost;
                self.guns += guns_to_buy;
                self.message = Some(format!("Bought {} guns for ${}", guns_to_buy, cost));
            } else if self.guns >= 10 {
                self.message = Some("You can't carry more than 10 guns!".to_string());
            } else {
                self.message = Some("You can't afford any guns! ($400 each)".to_string());
            }
            self.event = RandomEvent::None;
            self.view = DopeWarsView::Market;
        }
    }

    pub fn pay_bribe(&mut self) {
        if let RandomEvent::OfficerOffer { bribe } = self.event {
            if self.cash >= bribe {
                self.cash -= bribe;
                self.message = Some(format!(
                    "Paid ${} bribe to Officer Hardass. He let you go.",
                    bribe
                ));
            } else {
                // Can't afford, lose everything
                self.cash = 0;
                self.inventory = Inventory::new();
                self.health = self.health.saturating_sub(20);
                self.message = Some(
                    "You couldn't pay! Officer Hardass took everything and beat you up!"
                        .to_string(),
                );

                if self.health == 0 {
                    self.game_over = true;
                }
            }
            self.event = RandomEvent::None;
            self.view = DopeWarsView::Market;
        }
    }

    pub fn refuse_bribe(&mut self) {
        if let RandomEvent::OfficerOffer { .. } = self.event {
            if self.guns >= 3 && rand::thread_rng().gen_bool(0.6) {
                // Fight back successfully
                self.health = self.health.saturating_sub(15);
                self.message = Some("You fought Officer Hardass and escaped! -15 HP".to_string());

                if self.health == 0 {
                    self.game_over = true;
                    self.message = Some("Officer Hardass killed you in the shootout!".to_string());
                }
            } else {
                // Lose the fight
                self.inventory = Inventory::new();
                self.health = self.health.saturating_sub(30);
                self.message = Some("You tried to fight but Officer Hardass kicked your ass! Lost all drugs and -30 HP".to_string());

                if self.health == 0 {
                    self.game_over = true;
                    self.message = Some("Officer Hardass beat you to death!".to_string());
                }
            }
            self.event = RandomEvent::None;
            self.view = DopeWarsView::Market;
        }
    }

    /// Pay off debt
    pub fn pay_debt(&mut self, amount: i64) {
        let amount = amount.min(self.cash).min(self.debt);
        if amount > 0 {
            self.cash -= amount;
            self.debt -= amount;
            self.message = Some(format!(
                "Paid ${} toward debt. Remaining: ${}",
                amount, self.debt
            ));
        }
    }

    /// UI navigation methods
    pub fn product_up(&mut self) {
        if self.selected_product > 0 {
            self.selected_product -= 1;
        }
    }

    pub fn product_down(&mut self) {
        if self.selected_product < Product::all().len() - 1 {
            self.selected_product += 1;
        }
    }

    pub fn location_up(&mut self) {
        if self.selected_location > 0 {
            self.selected_location -= 1;
        }
    }

    pub fn location_down(&mut self) {
        if self.selected_location < Location::all().len() - 1 {
            self.selected_location += 1;
        }
    }

    pub fn add_quantity_digit(&mut self, digit: char) {
        if self.quantity_buffer.len() < 3 {
            self.quantity_buffer.push(digit);
        }
    }

    pub fn backspace_quantity(&mut self) {
        self.quantity_buffer.pop();
    }

    pub fn get_quantity(&self) -> u32 {
        self.quantity_buffer.parse().unwrap_or(1)
    }
}

impl GameEngine for DopeWarsState {
    fn tick(&mut self) {
        self.tick_count += 1;
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match self.view {
            DopeWarsView::Market => match key.code {
                KeyCode::Esc => KeyHandleResult::RequestQuit,
                KeyCode::Char('p') | KeyCode::Char('P') => KeyHandleResult::RequestPause,
                KeyCode::Up | KeyCode::Char('k') => {
                    self.product_up();
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.product_down();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('b') | KeyCode::Char('B') => {
                    let products = Product::all();
                    let product = products[self.selected_product];
                    let quantity = self.get_quantity();
                    self.buy(product, quantity);
                    KeyHandleResult::Handled
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    let products = Product::all();
                    let product = products[self.selected_product];
                    let quantity = self.get_quantity();
                    self.sell(product, quantity);
                    KeyHandleResult::Handled
                }
                KeyCode::Char(c @ '0'..='9') => {
                    self.add_quantity_digit(c);
                    KeyHandleResult::Handled
                }
                KeyCode::Backspace => {
                    self.backspace_quantity();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('t') | KeyCode::Char('T') | KeyCode::Tab => {
                    self.view = DopeWarsView::Travel;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('i') | KeyCode::Char('I') => {
                    self.view = DopeWarsView::Status;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    // Pay off debt - pay maximum possible
                    let payment = self.cash.min(self.debt);
                    self.pay_debt(payment);
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::NotHandled,
            },
            DopeWarsView::Travel => match key.code {
                KeyCode::Esc => {
                    self.view = DopeWarsView::Market;
                    KeyHandleResult::Handled
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.location_up();
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.location_down();
                    KeyHandleResult::Handled
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.travel();
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::NotHandled,
            },
            DopeWarsView::Status | DopeWarsView::Event => match key.code {
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ') => {
                    self.view = DopeWarsView::Market;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::NotHandled,
            },
        }
    }

    fn get_score(&self) -> u32 {
        DopeWarsState::score(self)
    }

    fn is_game_over(&self) -> bool {
        self.game_over
    }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.pending_events)
    }

    fn get_level(&self) -> Option<u32> {
        Some(self.day)
    }
}
