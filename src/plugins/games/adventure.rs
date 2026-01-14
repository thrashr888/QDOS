//! ADVENTURE - Dragon Quest
//!
//! Room-based exploration inspired by Atari's Adventure (1980).
//! Explore castles, avoid dragons, collect items, return the chalice!

use super::platform::{GameEngine, GameEvent, KeyHandleResult};
use crossterm::event::{KeyCode, KeyEvent};
use rand::Rng;

// =============================================================================
// CONSTANTS
// =============================================================================

const ROOM_WIDTH: usize = 38;
const ROOM_HEIGHT: usize = 14;
const DRAGON_MOVE_CHANCE: f64 = 0.3;
const DRAGON_CHASE_SPEED: i32 = 1;

// =============================================================================
// ENUMS
// =============================================================================

/// Current view state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AdventureView {
    #[default]
    Menu,
    Playing,
    Victory,
    GameOver,
}

/// Room types in the game
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RoomType {
    #[default]
    Outside,
    GoldCastle,
    GoldMaze1,
    GoldMaze2,
    GoldMaze3,
    GreenCastle,
    GreenMaze1,
    GreenMaze2,
    YorkleRoom,
    ChaliceRoom,
    BridgeRoom,
    SecretRoom,
}

impl RoomType {
    pub fn name(&self) -> &'static str {
        match self {
            RoomType::Outside => "Outside",
            RoomType::GoldCastle => "Gold Castle",
            RoomType::GoldMaze1 => "Gold Maze",
            RoomType::GoldMaze2 => "Gold Maze",
            RoomType::GoldMaze3 => "Gold Maze",
            RoomType::GreenCastle => "Green Castle",
            RoomType::GreenMaze1 => "Green Maze",
            RoomType::GreenMaze2 => "Green Maze",
            RoomType::YorkleRoom => "Yorgle's Lair",
            RoomType::ChaliceRoom => "Chalice Chamber",
            RoomType::BridgeRoom => "Bridge Room",
            RoomType::SecretRoom => "Secret Room",
        }
    }

    pub fn all() -> Vec<RoomType> {
        vec![
            RoomType::Outside,
            RoomType::GoldCastle,
            RoomType::GoldMaze1,
            RoomType::GoldMaze2,
            RoomType::GoldMaze3,
            RoomType::GreenCastle,
            RoomType::GreenMaze1,
            RoomType::GreenMaze2,
            RoomType::YorkleRoom,
            RoomType::ChaliceRoom,
            RoomType::BridgeRoom,
            RoomType::SecretRoom,
        ]
    }
}

/// Item types the player can carry
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemType {
    Sword,
    GoldKey,
    Bridge,
    Chalice,
}

impl ItemType {
    pub fn char(&self) -> char {
        match self {
            ItemType::Sword => '†',
            ItemType::GoldKey => '⚷',
            ItemType::Bridge => '═',
            ItemType::Chalice => '☗',
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ItemType::Sword => "Sword",
            ItemType::GoldKey => "Gold Key",
            ItemType::Bridge => "Bridge",
            ItemType::Chalice => "Chalice",
        }
    }
}

/// Dragon types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragonType {
    Yorgle,  // Yellow - slow, guards gold key
    Grundle, // Green - medium, guards bridge
}

impl DragonType {
    pub fn char(&self) -> char {
        match self {
            DragonType::Yorgle => 'Ʃ',
            DragonType::Grundle => 'Ʃ',
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            DragonType::Yorgle => "Yorgle",
            DragonType::Grundle => "Grundle",
        }
    }

    pub fn speed(&self) -> i32 {
        match self {
            DragonType::Yorgle => 1,
            DragonType::Grundle => 2,
        }
    }
}

// =============================================================================
// ITEM
// =============================================================================

/// An item in the world
#[derive(Debug, Clone)]
pub struct Item {
    pub item_type: ItemType,
    pub room: RoomType,
    pub x: usize,
    pub y: usize,
}

impl Item {
    pub fn new(item_type: ItemType, room: RoomType, x: usize, y: usize) -> Self {
        Self {
            item_type,
            room,
            x,
            y,
        }
    }
}

// =============================================================================
// DRAGON
// =============================================================================

/// A dragon in the world
#[derive(Debug, Clone)]
pub struct Dragon {
    pub dragon_type: DragonType,
    pub room: RoomType,
    pub x: usize,
    pub y: usize,
    pub alive: bool,
    pub has_eaten_player: bool,
}

impl Dragon {
    pub fn new(dragon_type: DragonType, room: RoomType, x: usize, y: usize) -> Self {
        Self {
            dragon_type,
            room,
            x,
            y,
            alive: true,
            has_eaten_player: false,
        }
    }
}

// =============================================================================
// ROOM
// =============================================================================

/// Room exit connections
#[derive(Debug, Clone, Default)]
pub struct RoomExits {
    pub north: Option<RoomType>,
    pub south: Option<RoomType>,
    pub east: Option<RoomType>,
    pub west: Option<RoomType>,
}

/// A room in the game
#[derive(Debug, Clone)]
pub struct Room {
    pub room_type: RoomType,
    pub walls: [[bool; ROOM_WIDTH]; ROOM_HEIGHT],
    pub exits: RoomExits,
    pub requires_key: bool,
    pub has_gap: bool, // Requires bridge to cross
    pub gap_x: usize,
}

impl Room {
    pub fn new(room_type: RoomType) -> Self {
        let mut walls = [[false; ROOM_WIDTH]; ROOM_HEIGHT];

        // Border walls (top and bottom)
        for wall in walls[0].iter_mut().take(ROOM_WIDTH) {
            *wall = true;
        }
        for wall in walls[ROOM_HEIGHT - 1].iter_mut().take(ROOM_WIDTH) {
            *wall = true;
        }
        // Border walls (left and right)
        for row in walls.iter_mut().take(ROOM_HEIGHT) {
            row[0] = true;
            row[ROOM_WIDTH - 1] = true;
        }

        Self {
            room_type,
            walls,
            exits: RoomExits::default(),
            requires_key: false,
            has_gap: false,
            gap_x: 0,
        }
    }

    /// Add interior walls for maze rooms
    pub fn add_maze_walls(&mut self, pattern: usize) {
        match pattern % 4 {
            0 => {
                // Horizontal barrier with gap
                for x in 5..20 {
                    self.walls[6][x] = true;
                }
                for x in 25..35 {
                    self.walls[6][x] = true;
                }
            }
            1 => {
                // Vertical barrier
                for y in 2..12 {
                    self.walls[y][18] = true;
                }
            }
            2 => {
                // L-shaped wall
                for x in 10..25 {
                    self.walls[4][x] = true;
                }
                for y in 4..10 {
                    self.walls[y][10] = true;
                }
            }
            _ => {
                // Scattered blocks
                self.walls[4][15] = true;
                self.walls[4][16] = true;
                self.walls[5][15] = true;
                self.walls[5][16] = true;
                self.walls[8][25] = true;
                self.walls[8][26] = true;
                self.walls[9][25] = true;
                self.walls[9][26] = true;
            }
        }
    }

    /// Set up a gap that requires bridge
    pub fn add_gap(&mut self, x: usize) {
        self.has_gap = true;
        self.gap_x = x;
        // Gap spans several columns
        for y in 2..ROOM_HEIGHT - 2 {
            self.walls[y][x] = false;
            self.walls[y][x + 1] = false;
            self.walls[y][x + 2] = false;
        }
    }

    /// Check if position is walkable
    pub fn is_walkable(&self, x: usize, y: usize) -> bool {
        if x >= ROOM_WIDTH || y >= ROOM_HEIGHT {
            return false;
        }
        !self.walls[y][x]
    }

    /// Check if position is at a gap
    pub fn is_gap(&self, x: usize, _y: usize) -> bool {
        self.has_gap && x >= self.gap_x && x <= self.gap_x + 2
    }
}

// =============================================================================
// GAME STATE
// =============================================================================

/// Main game state
pub struct AdventureState {
    pub view: AdventureView,

    // Player state
    pub player_room: RoomType,
    pub player_x: usize,
    pub player_y: usize,
    pub held_item: Option<ItemType>,
    pub eaten_by: Option<DragonType>,

    // World state
    pub rooms: Vec<Room>,
    pub items: Vec<Item>,
    pub dragons: Vec<Dragon>,

    // Bridge placed position (if dropped on gap)
    pub bridge_placed: Option<(RoomType, usize)>,

    // Game stats
    pub score: u32,
    pub moves: u32,

    // Animation
    pub tick_count: u32,
    pub message: Option<String>,
    pub message_timer: u32,

    // State flags
    pub game_over: bool,
    pub game_won: bool,

    // Events
    pending_events: Vec<GameEvent>,
}

impl Default for AdventureState {
    fn default() -> Self {
        Self::new()
    }
}

impl AdventureState {
    pub fn new() -> Self {
        Self {
            view: AdventureView::Menu,
            player_room: RoomType::Outside,
            player_x: ROOM_WIDTH / 2,
            player_y: ROOM_HEIGHT / 2,
            held_item: None,
            eaten_by: None,
            rooms: Vec::new(),
            items: Vec::new(),
            dragons: Vec::new(),
            bridge_placed: None,
            score: 0,
            moves: 0,
            tick_count: 0,
            message: None,
            message_timer: 0,
            game_over: false,
            game_won: false,
            pending_events: Vec::new(),
        }
    }

    /// Start a new game
    pub fn start_game(&mut self) {
        self.view = AdventureView::Playing;
        self.player_room = RoomType::Outside;
        self.player_x = ROOM_WIDTH / 2;
        self.player_y = ROOM_HEIGHT / 2;
        self.held_item = None;
        self.eaten_by = None;
        self.bridge_placed = None;
        self.score = 0;
        self.moves = 0;
        self.tick_count = 0;
        self.message = None;
        self.game_over = false;
        self.game_won = false;

        self.setup_world();
    }

    /// Set up rooms, items, and dragons
    fn setup_world(&mut self) {
        self.rooms.clear();
        self.items.clear();
        self.dragons.clear();

        // Create rooms
        for room_type in RoomType::all() {
            let mut room = Room::new(room_type);

            match room_type {
                RoomType::Outside => {
                    room.exits.south = Some(RoomType::GoldCastle);
                    room.exits.east = Some(RoomType::GreenCastle);
                }
                RoomType::GoldCastle => {
                    room.exits.north = Some(RoomType::Outside);
                    room.exits.south = Some(RoomType::GoldMaze1);
                    room.requires_key = true;
                }
                RoomType::GoldMaze1 => {
                    room.exits.north = Some(RoomType::GoldCastle);
                    room.exits.south = Some(RoomType::GoldMaze2);
                    room.add_maze_walls(0);
                }
                RoomType::GoldMaze2 => {
                    room.exits.north = Some(RoomType::GoldMaze1);
                    room.exits.south = Some(RoomType::GoldMaze3);
                    room.add_maze_walls(1);
                }
                RoomType::GoldMaze3 => {
                    room.exits.north = Some(RoomType::GoldMaze2);
                    room.exits.east = Some(RoomType::SecretRoom);
                    room.add_maze_walls(2);
                }
                RoomType::GreenCastle => {
                    room.exits.west = Some(RoomType::Outside);
                    room.exits.south = Some(RoomType::GreenMaze1);
                }
                RoomType::GreenMaze1 => {
                    room.exits.north = Some(RoomType::GreenCastle);
                    room.exits.south = Some(RoomType::GreenMaze2);
                    room.add_maze_walls(3);
                }
                RoomType::GreenMaze2 => {
                    room.exits.north = Some(RoomType::GreenMaze1);
                    room.exits.east = Some(RoomType::YorkleRoom);
                    room.add_maze_walls(0);
                }
                RoomType::YorkleRoom => {
                    room.exits.west = Some(RoomType::GreenMaze2);
                    room.exits.south = Some(RoomType::BridgeRoom);
                }
                RoomType::BridgeRoom => {
                    room.exits.north = Some(RoomType::YorkleRoom);
                    room.exits.east = Some(RoomType::ChaliceRoom);
                    room.add_gap(20); // Gap that requires bridge
                }
                RoomType::ChaliceRoom => {
                    room.exits.west = Some(RoomType::BridgeRoom);
                }
                RoomType::SecretRoom => {
                    room.exits.west = Some(RoomType::GoldMaze3);
                }
            }

            self.rooms.push(room);
        }

        // Place items
        self.items
            .push(Item::new(ItemType::Sword, RoomType::Outside, 10, 8));
        self.items
            .push(Item::new(ItemType::GoldKey, RoomType::YorkleRoom, 30, 10));
        self.items
            .push(Item::new(ItemType::Bridge, RoomType::GreenMaze2, 15, 6));
        self.items
            .push(Item::new(ItemType::Chalice, RoomType::ChaliceRoom, 20, 7));

        // Place dragons
        self.dragons
            .push(Dragon::new(DragonType::Yorgle, RoomType::YorkleRoom, 25, 7));
        self.dragons.push(Dragon::new(
            DragonType::Grundle,
            RoomType::GreenMaze1,
            20,
            8,
        ));
    }

    /// Get current room
    pub fn current_room(&self) -> Option<&Room> {
        self.rooms.iter().find(|r| r.room_type == self.player_room)
    }

    /// Get items in current room
    pub fn items_in_room(&self) -> Vec<&Item> {
        self.items
            .iter()
            .filter(|i| i.room == self.player_room)
            .collect()
    }

    /// Get dragons in current room
    pub fn dragons_in_room(&self) -> Vec<&Dragon> {
        self.dragons
            .iter()
            .filter(|d| d.room == self.player_room && d.alive)
            .collect()
    }

    /// Show a temporary message
    fn show_message(&mut self, msg: &str) {
        self.message = Some(msg.to_string());
        self.message_timer = 20;
    }

    /// Try to move player
    fn try_move(&mut self, dx: i32, dy: i32) {
        if self.eaten_by.is_some() {
            // Inside dragon - movement is different
            return;
        }

        let new_x = (self.player_x as i32 + dx).max(0) as usize;
        let new_y = (self.player_y as i32 + dy).max(0) as usize;

        // Check room boundaries and exits
        if let Some(room) = self.current_room().cloned() {
            // Check for room exit
            if new_y == 0 {
                if let Some(north) = room.exits.north {
                    if !room.requires_key || self.held_item == Some(ItemType::GoldKey) {
                        self.player_room = north;
                        self.player_y = ROOM_HEIGHT - 2;
                        self.moves += 1;
                        return;
                    } else {
                        self.show_message("Locked! Need Gold Key");
                        return;
                    }
                }
            }
            if new_y >= ROOM_HEIGHT - 1 {
                if let Some(south) = room.exits.south {
                    self.player_room = south;
                    self.player_y = 1;
                    self.moves += 1;
                    return;
                }
            }
            if new_x == 0 {
                if let Some(west) = room.exits.west {
                    self.player_room = west;
                    self.player_x = ROOM_WIDTH - 2;
                    self.moves += 1;
                    return;
                }
            }
            if new_x >= ROOM_WIDTH - 1 {
                if let Some(east) = room.exits.east {
                    self.player_room = east;
                    self.player_x = 1;
                    self.moves += 1;
                    return;
                }
            }

            // Check gap
            if room.is_gap(new_x, new_y) {
                // Check if bridge is placed here
                if let Some((bridge_room, bridge_x)) = self.bridge_placed {
                    if bridge_room == self.player_room && new_x >= bridge_x && new_x <= bridge_x + 2
                    {
                        // Bridge is here, can cross
                    } else {
                        self.show_message("Gap! Need bridge");
                        return;
                    }
                } else {
                    self.show_message("Gap! Need bridge");
                    return;
                }
            }

            // Check wall collision
            if room.is_walkable(new_x, new_y) {
                self.player_x = new_x;
                self.player_y = new_y;
                self.moves += 1;
            }
        }
    }

    /// Pick up or drop item
    fn interact(&mut self) {
        // If inside dragon and holding sword, escape!
        if let Some(dragon_type) = self.eaten_by {
            if self.held_item == Some(ItemType::Sword) {
                // Kill dragon from inside!
                for dragon in &mut self.dragons {
                    if dragon.dragon_type == dragon_type && dragon.has_eaten_player {
                        dragon.alive = false;
                        dragon.has_eaten_player = false;
                        self.eaten_by = None;
                        self.score += 500;
                        self.show_message(&format!("{} slain from within!", dragon_type.name()));
                        return;
                    }
                }
            }
            return;
        }

        // Drop current item
        if let Some(item_type) = self.held_item {
            // If dropping bridge on a gap, place it
            if let Some(room) = self.current_room() {
                if item_type == ItemType::Bridge && room.is_gap(self.player_x, self.player_y) {
                    self.bridge_placed = Some((self.player_room, self.player_x));
                    self.held_item = None;
                    self.show_message("Bridge placed!");
                    return;
                }
            }

            // Otherwise just drop it
            self.items.push(Item::new(
                item_type,
                self.player_room,
                self.player_x,
                self.player_y,
            ));
            self.held_item = None;
            self.show_message(&format!("Dropped {}", item_type.name()));
            return;
        }

        // Try to pick up item at current location
        let mut picked_up = None;
        for (i, item) in self.items.iter().enumerate() {
            if item.room == self.player_room {
                let dx = (item.x as i32 - self.player_x as i32).abs();
                let dy = (item.y as i32 - self.player_y as i32).abs();
                if dx <= 1 && dy <= 1 {
                    picked_up = Some((i, item.item_type));
                    break;
                }
            }
        }

        if let Some((idx, item_type)) = picked_up {
            self.items.remove(idx);
            self.held_item = Some(item_type);
            self.show_message(&format!("Got {}!", item_type.name()));
        }
    }

    /// Update dragon AI
    fn update_dragons(&mut self) {
        let mut rng = rand::thread_rng();

        for dragon in &mut self.dragons {
            if !dragon.alive || dragon.has_eaten_player {
                continue;
            }

            // Only move if in same room as player (or adjacent)
            if dragon.room == self.player_room {
                // Chase player with probability
                if rng.gen_bool(DRAGON_MOVE_CHANCE) {
                    let dx = if self.player_x > dragon.x {
                        1
                    } else if self.player_x < dragon.x {
                        -1
                    } else {
                        0
                    };
                    let dy = if self.player_y > dragon.y {
                        1
                    } else if self.player_y < dragon.y {
                        -1
                    } else {
                        0
                    };

                    let speed = dragon.dragon_type.speed();
                    dragon.x = (dragon.x as i32 + dx * speed).max(1) as usize;
                    dragon.y = (dragon.y as i32 + dy * speed).max(1) as usize;
                    dragon.x = dragon.x.min(ROOM_WIDTH - 2);
                    dragon.y = dragon.y.min(ROOM_HEIGHT - 2);
                }
            }
        }
    }

    /// Check dragon collisions
    fn check_dragon_collision(&mut self) {
        if self.eaten_by.is_some() {
            return;
        }

        // Find which dragon (if any) collides with player
        let mut collision_result: Option<(usize, bool)> = None; // (dragon_index, has_sword)

        for (i, dragon) in self.dragons.iter().enumerate() {
            if !dragon.alive {
                continue;
            }

            if dragon.room == self.player_room {
                let dx = (dragon.x as i32 - self.player_x as i32).abs();
                let dy = (dragon.y as i32 - self.player_y as i32).abs();

                if dx <= 1 && dy <= 1 {
                    collision_result = Some((i, self.held_item == Some(ItemType::Sword)));
                    break;
                }
            }
        }

        // Process collision outside of borrow
        if let Some((dragon_idx, has_sword)) = collision_result {
            let dragon_type = self.dragons[dragon_idx].dragon_type;
            if has_sword {
                // Kill dragon!
                self.dragons[dragon_idx].alive = false;
                self.score += 500;
                self.show_message(&format!("{} slain!", dragon_type.name()));
            } else {
                // Eaten by dragon!
                self.dragons[dragon_idx].has_eaten_player = true;
                self.eaten_by = Some(dragon_type);
                self.show_message(&format!("Eaten by {}!", dragon_type.name()));
            }
        }
    }

    /// Check win condition
    fn check_win(&mut self) {
        if self.held_item == Some(ItemType::Chalice) && self.player_room == RoomType::GoldCastle {
            self.game_won = true;
            self.game_over = true;
            self.score += 1000 + (10000 / self.moves.max(1));
            self.view = AdventureView::Victory;
        }
    }
}

// =============================================================================
// GAME ENGINE IMPLEMENTATION
// =============================================================================

impl GameEngine for AdventureState {
    fn tick(&mut self) {
        if self.view != AdventureView::Playing {
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

        // Update dragons (every 5 ticks)
        if self.tick_count.is_multiple_of(5) {
            self.update_dragons();
            self.check_dragon_collision();
        }

        // Check win
        self.check_win();
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match self.view {
            AdventureView::Menu => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.start_game();
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => KeyHandleResult::RequestQuit,
                _ => KeyHandleResult::Handled,
            },
            AdventureView::Playing => match key.code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Char('w') => {
                    self.try_move(0, -1);
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Char('s') => {
                    self.try_move(0, 1);
                    KeyHandleResult::Handled
                }
                KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('a') => {
                    self.try_move(-1, 0);
                    KeyHandleResult::Handled
                }
                KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') | KeyCode::Char('d') => {
                    self.try_move(1, 0);
                    KeyHandleResult::Handled
                }
                KeyCode::Char(' ') | KeyCode::Enter => {
                    self.interact();
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => {
                    self.view = AdventureView::Menu;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            AdventureView::Victory | AdventureView::GameOver => match key.code {
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
