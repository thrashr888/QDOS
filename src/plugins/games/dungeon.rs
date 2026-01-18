//! DUNGEON - ASCII Maze Crawler
//!
//! A roguelike maze game with procedurally-generated labyrinths,
//! enemies that pursue you, friendly NPCs, and loot collection.

use super::platform::{GameEngine, GameEvent, KeyHandleResult};
use crossterm::event::{KeyCode, KeyEvent};
use rand::prelude::*;
use std::collections::{BinaryHeap, HashMap, HashSet};

// =============================================================================
// CONSTANTS
// =============================================================================

/// Board dimensions (fits in 80x25 terminal with UI)
pub const BOARD_WIDTH: usize = 60;
pub const BOARD_HEIGHT: usize = 18;

/// Torch radius for visibility
pub const TORCH_RADIUS: i32 = 5;

/// Starting player stats
const STARTING_HP: i32 = 20;
const STARTING_ATTACK: i32 = 3;

// =============================================================================
// TILE SYSTEM
// =============================================================================

/// Tile types in the maze
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    Wall,
    Floor,
    Exit,       // Stairs down to next floor
    Door,       // Locked door (requires key)
    DoorOpen,   // Unlocked door
    Trap,       // Visible trap
    TrapHidden, // Hidden trap (looks like floor)
    Water,      // Slows movement
}

impl Tile {
    pub fn is_passable(&self) -> bool {
        matches!(
            self,
            Tile::Floor | Tile::Exit | Tile::DoorOpen | Tile::Trap | Tile::TrapHidden | Tile::Water
        )
    }

    pub fn char(&self) -> char {
        match self {
            Tile::Wall => '█',
            Tile::Floor => '·',
            Tile::Exit => '▼',
            Tile::Door => '╬',
            Tile::DoorOpen => '░',
            Tile::Trap => '^',
            Tile::TrapHidden => '·', // Looks like floor
            Tile::Water => '~',
        }
    }
}

// =============================================================================
// ENTITY SYSTEM
// =============================================================================

/// Item types the player can collect
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemType {
    Gold,   // Currency
    Food,   // Restores HP
    Key,    // Opens doors
    Potion, // Various effects
    Weapon, // Damage boost
}

impl ItemType {
    pub fn char(&self) -> char {
        match self {
            ItemType::Gold => '$',
            ItemType::Food => '%',
            ItemType::Key => '&',
            ItemType::Potion => '!',
            ItemType::Weapon => '+',
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ItemType::Gold => "Gold",
            ItemType::Food => "Food",
            ItemType::Key => "Key",
            ItemType::Potion => "Potion",
            ItemType::Weapon => "Weapon",
        }
    }
}

/// An item on the ground
#[derive(Debug, Clone)]
pub struct Item {
    pub x: usize,
    pub y: usize,
    pub item_type: ItemType,
    pub value: u32, // Amount for gold, HP for food, etc.
}

/// Enemy types (pursue player)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyType {
    Snake,    // Weak, fast movement
    Goblin,   // Basic enemy
    Ghost,    // Can move through walls
    Skeleton, // Undead
    Troll,    // Strong, slow
    Boss,     // Floor boss (every 5 floors)
}

impl EnemyType {
    pub fn char(&self) -> char {
        match self {
            EnemyType::Snake => '§',
            EnemyType::Goblin => 'G',
            EnemyType::Ghost => 'Ω',
            EnemyType::Skeleton => 'S',
            EnemyType::Troll => 'T',
            EnemyType::Boss => 'Ð',
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            EnemyType::Snake => "Snake",
            EnemyType::Goblin => "Goblin",
            EnemyType::Ghost => "Ghost",
            EnemyType::Skeleton => "Skeleton",
            EnemyType::Troll => "Troll",
            EnemyType::Boss => "Dungeon Lord",
        }
    }

    pub fn max_hp(&self) -> i32 {
        match self {
            EnemyType::Snake => 5,
            EnemyType::Goblin => 8,
            EnemyType::Ghost => 6,
            EnemyType::Skeleton => 10,
            EnemyType::Troll => 15,
            EnemyType::Boss => 50,
        }
    }

    pub fn damage(&self) -> i32 {
        match self {
            EnemyType::Snake => 2,
            EnemyType::Goblin => 3,
            EnemyType::Ghost => 4,
            EnemyType::Skeleton => 3,
            EnemyType::Troll => 6,
            EnemyType::Boss => 10,
        }
    }

    pub fn xp(&self) -> u32 {
        match self {
            EnemyType::Snake => 10,
            EnemyType::Goblin => 20,
            EnemyType::Ghost => 25,
            EnemyType::Skeleton => 20,
            EnemyType::Troll => 40,
            EnemyType::Boss => 200,
        }
    }

    /// How often enemy moves (every N ticks)
    pub fn speed(&self) -> u32 {
        match self {
            EnemyType::Snake => 2, // Fast
            EnemyType::Goblin => 3,
            EnemyType::Ghost => 3,
            EnemyType::Skeleton => 4,
            EnemyType::Troll => 5, // Slow
            EnemyType::Boss => 4,
        }
    }

    /// Can this enemy move through walls?
    pub fn can_phase(&self) -> bool {
        matches!(self, EnemyType::Ghost)
    }

    /// Pick enemy type based on floor
    pub fn for_floor(floor: u32) -> Self {
        let mut rng = rand::thread_rng();
        let roll: u32 = rng.gen_range(0..100);

        match floor {
            1 => EnemyType::Snake,
            2..=3 => {
                if roll < 50 {
                    EnemyType::Snake
                } else {
                    EnemyType::Goblin
                }
            }
            4..=5 => {
                if roll < 30 {
                    EnemyType::Goblin
                } else if roll < 60 {
                    EnemyType::Skeleton
                } else {
                    EnemyType::Ghost
                }
            }
            6..=8 => {
                if roll < 30 {
                    EnemyType::Skeleton
                } else if roll < 60 {
                    EnemyType::Ghost
                } else {
                    EnemyType::Troll
                }
            }
            _ => {
                if roll < 25 {
                    EnemyType::Ghost
                } else if roll < 50 {
                    EnemyType::Skeleton
                } else {
                    EnemyType::Troll
                }
            }
        }
    }
}

/// An enemy in the dungeon
#[derive(Debug, Clone)]
pub struct Enemy {
    pub x: usize,
    pub y: usize,
    pub enemy_type: EnemyType,
    pub hp: i32,
    pub move_counter: u32, // Tracks when to move
}

impl Enemy {
    pub fn new(x: usize, y: usize, enemy_type: EnemyType) -> Self {
        Self {
            x,
            y,
            enemy_type,
            hp: enemy_type.max_hp(),
            move_counter: 0,
        }
    }
}

/// Friendly NPC types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FriendlyType {
    Sheep,    // Runs away, drops wool (gold)
    Merchant, // Sells items
    Fairy,    // Heals player when nearby
}

impl FriendlyType {
    pub fn char(&self) -> char {
        match self {
            FriendlyType::Sheep => '@',
            FriendlyType::Merchant => '$',
            FriendlyType::Fairy => '*',
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            FriendlyType::Sheep => "Sheep",
            FriendlyType::Merchant => "Merchant",
            FriendlyType::Fairy => "Fairy",
        }
    }
}

/// A friendly NPC
#[derive(Debug, Clone)]
pub struct Friendly {
    pub x: usize,
    pub y: usize,
    pub friendly_type: FriendlyType,
    pub talked: bool,
}

// =============================================================================
// A* PATHFINDING
// =============================================================================

#[derive(Clone, Eq, PartialEq)]
struct PathNode {
    x: usize,
    y: usize,
    cost: u32,
    heuristic: u32,
}

impl Ord for PathNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (other.cost + other.heuristic).cmp(&(self.cost + self.heuristic))
    }
}

impl PartialOrd for PathNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// A* pathfinding from (sx, sy) to (gx, gy)
fn find_path(
    board: &[[Tile; BOARD_WIDTH]; BOARD_HEIGHT],
    sx: usize,
    sy: usize,
    gx: usize,
    gy: usize,
    can_phase: bool,
) -> Option<(usize, usize)> {
    if sx == gx && sy == gy {
        return None;
    }

    let mut open = BinaryHeap::new();
    let mut came_from: HashMap<(usize, usize), (usize, usize)> = HashMap::new();
    let mut g_score: HashMap<(usize, usize), u32> = HashMap::new();
    let mut closed: HashSet<(usize, usize)> = HashSet::new();

    let heuristic = |x: usize, y: usize| -> u32 {
        ((x as i32 - gx as i32).abs() + (y as i32 - gy as i32).abs()) as u32
    };

    g_score.insert((sx, sy), 0);
    open.push(PathNode {
        x: sx,
        y: sy,
        cost: 0,
        heuristic: heuristic(sx, sy),
    });

    let directions: [(i32, i32); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];

    while let Some(current) = open.pop() {
        if current.x == gx && current.y == gy {
            // Reconstruct path and return first step
            let mut pos = (gx, gy);
            while let Some(&prev) = came_from.get(&pos) {
                if prev == (sx, sy) {
                    return Some(pos);
                }
                pos = prev;
            }
            return Some((gx, gy));
        }

        if closed.contains(&(current.x, current.y)) {
            continue;
        }
        closed.insert((current.x, current.y));

        for (dx, dy) in &directions {
            let nx = current.x as i32 + dx;
            let ny = current.y as i32 + dy;

            if nx < 0 || ny < 0 || nx >= BOARD_WIDTH as i32 || ny >= BOARD_HEIGHT as i32 {
                continue;
            }

            let nx = nx as usize;
            let ny = ny as usize;

            // Check passability
            let passable = can_phase || board[ny][nx].is_passable();
            if !passable {
                continue;
            }

            let tentative_g = g_score.get(&(current.x, current.y)).unwrap_or(&u32::MAX) + 1;

            if tentative_g < *g_score.get(&(nx, ny)).unwrap_or(&u32::MAX) {
                came_from.insert((nx, ny), (current.x, current.y));
                g_score.insert((nx, ny), tentative_g);
                open.push(PathNode {
                    x: nx,
                    y: ny,
                    cost: tentative_g,
                    heuristic: heuristic(nx, ny),
                });
            }
        }
    }

    None
}

// =============================================================================
// GAME STATE
// =============================================================================

/// Main game state
pub struct DungeonState {
    // Map
    pub board: [[Tile; BOARD_WIDTH]; BOARD_HEIGHT],
    pub visible: [[bool; BOARD_WIDTH]; BOARD_HEIGHT],
    pub explored: [[bool; BOARD_WIDTH]; BOARD_HEIGHT],

    // Player
    pub player_x: usize,
    pub player_y: usize,
    pub hp: i32,
    pub max_hp: i32,
    pub attack: i32,
    pub gold: u32,
    pub keys: u32,
    pub xp: u32,
    pub level: u32,

    // Progression
    pub floor: u32,
    pub max_floor: u32, // Deepest reached

    // Entities
    pub enemies: Vec<Enemy>,
    pub friendlies: Vec<Friendly>,
    pub items: Vec<Item>,

    // UI
    pub messages: Vec<String>,
    pub max_messages: usize,

    // State
    pub game_over: bool,
    pub game_won: bool,
    pub tick_counter: u32,
    pending_events: Vec<GameEvent>,
}

impl Default for DungeonState {
    fn default() -> Self {
        Self::new()
    }
}

impl DungeonState {
    pub fn new() -> Self {
        let mut state = Self {
            board: [[Tile::Wall; BOARD_WIDTH]; BOARD_HEIGHT],
            visible: [[false; BOARD_WIDTH]; BOARD_HEIGHT],
            explored: [[false; BOARD_WIDTH]; BOARD_HEIGHT],

            player_x: 1,
            player_y: 1,
            hp: STARTING_HP,
            max_hp: STARTING_HP,
            attack: STARTING_ATTACK,
            gold: 0,
            keys: 0,
            xp: 0,
            level: 1,

            floor: 1,
            max_floor: 1,

            enemies: Vec::new(),
            friendlies: Vec::new(),
            items: Vec::new(),

            messages: vec!["Welcome to the DUNGEON!".to_string()],
            max_messages: 4,

            game_over: false,
            game_won: false,
            tick_counter: 0,
            pending_events: Vec::new(),
        };

        state.generate_floor();
        state.update_visibility();
        state.pending_events.push(GameEvent::GameStarted);
        state
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Add a message to the log
    pub fn add_message(&mut self, msg: impl Into<String>) {
        self.messages.push(msg.into());
        while self.messages.len() > self.max_messages {
            self.messages.remove(0);
        }
    }

    /// Generate a new floor using recursive backtracker maze algorithm
    fn generate_floor(&mut self) {
        let mut rng = rand::thread_rng();

        // Clear everything
        for y in 0..BOARD_HEIGHT {
            for x in 0..BOARD_WIDTH {
                self.board[y][x] = Tile::Wall;
                self.visible[y][x] = false;
                self.explored[y][x] = false;
            }
        }
        self.enemies.clear();
        self.friendlies.clear();
        self.items.clear();

        // Recursive backtracker maze generation
        // Work with cells (odd coordinates only for corridors)
        let cell_w = (BOARD_WIDTH - 1) / 2;
        let cell_h = (BOARD_HEIGHT - 1) / 2;

        let mut visited = vec![vec![false; cell_w]; cell_h];
        let mut stack: Vec<(usize, usize)> = Vec::new();

        // Start from random cell
        let start_cx = rng.gen_range(0..cell_w);
        let start_cy = rng.gen_range(0..cell_h);
        visited[start_cy][start_cx] = true;
        stack.push((start_cx, start_cy));

        // Carve the maze
        while let Some(&(cx, cy)) = stack.last() {
            let mut neighbors = Vec::new();

            // Check all 4 directions
            if cx > 0 && !visited[cy][cx - 1] {
                neighbors.push((cx - 1, cy, -1i32, 0i32));
            }
            if cx + 1 < cell_w && !visited[cy][cx + 1] {
                neighbors.push((cx + 1, cy, 1, 0));
            }
            if cy > 0 && !visited[cy - 1][cx] {
                neighbors.push((cx, cy - 1, 0, -1));
            }
            if cy + 1 < cell_h && !visited[cy + 1][cx] {
                neighbors.push((cx, cy + 1, 0, 1));
            }

            if neighbors.is_empty() {
                stack.pop();
            } else {
                let (ncx, ncy, dx, dy) = neighbors[rng.gen_range(0..neighbors.len())];

                // Carve the wall between cells
                let bx = 1 + cx * 2;
                let by = 1 + cy * 2;
                let wall_x = (bx as i32 + dx) as usize;
                let wall_y = (by as i32 + dy) as usize;

                self.board[by][bx] = Tile::Floor;
                self.board[wall_y][wall_x] = Tile::Floor;
                self.board[1 + ncy * 2][1 + ncx * 2] = Tile::Floor;

                visited[ncy][ncx] = true;
                stack.push((ncx, ncy));
            }
        }

        // Place player at top-left area
        self.player_x = 1;
        self.player_y = 1;
        self.board[1][1] = Tile::Floor;

        // Place exit at bottom-right area
        let exit_x = 1 + (cell_w - 1) * 2;
        let exit_y = 1 + (cell_h - 1) * 2;
        self.board[exit_y][exit_x] = Tile::Exit;

        // Spawn enemies based on floor
        let num_enemies = 2 + self.floor.min(10);
        for _ in 0..num_enemies {
            if let Some((x, y)) = self.find_empty_spot(&mut rng) {
                // Don't spawn too close to player
                let dist = ((x as i32 - self.player_x as i32).abs()
                    + (y as i32 - self.player_y as i32).abs()) as u32;
                if dist > 5 {
                    let enemy_type = if self.floor.is_multiple_of(5) && self.enemies.is_empty() {
                        EnemyType::Boss
                    } else {
                        EnemyType::for_floor(self.floor)
                    };
                    self.enemies.push(Enemy::new(x, y, enemy_type));
                }
            }
        }

        // Spawn items
        let num_items = 3 + rng.gen_range(0..3);
        for _ in 0..num_items {
            if let Some((x, y)) = self.find_empty_spot(&mut rng) {
                let item_type = match rng.gen_range(0..100) {
                    0..=40 => ItemType::Gold,
                    41..=60 => ItemType::Food,
                    61..=80 => ItemType::Key,
                    81..=95 => ItemType::Potion,
                    _ => ItemType::Weapon,
                };
                let value = match item_type {
                    ItemType::Gold => rng.gen_range(5..20) * self.floor,
                    ItemType::Food => rng.gen_range(3..8),
                    ItemType::Key => 1,
                    ItemType::Potion => rng.gen_range(5..15),
                    ItemType::Weapon => 1 + self.floor / 3,
                };
                self.items.push(Item {
                    x,
                    y,
                    item_type,
                    value,
                });
            }
        }

        // Spawn friendlies (rarely)
        if rng.gen_bool(0.3) {
            if let Some((x, y)) = self.find_empty_spot(&mut rng) {
                let friendly_type = match rng.gen_range(0..3) {
                    0 => FriendlyType::Sheep,
                    1 => FriendlyType::Merchant,
                    _ => FriendlyType::Fairy,
                };
                self.friendlies.push(Friendly {
                    x,
                    y,
                    friendly_type,
                    talked: false,
                });
            }
        }

        // Add some traps on higher floors
        if self.floor >= 3 {
            let num_traps = (self.floor / 2).min(5);
            for _ in 0..num_traps {
                if let Some((x, y)) = self.find_empty_spot(&mut rng) {
                    self.board[y][x] = Tile::TrapHidden;
                }
            }
        }
    }

    /// Find an empty floor spot
    fn find_empty_spot(&self, rng: &mut impl Rng) -> Option<(usize, usize)> {
        for _ in 0..100 {
            let x = rng.gen_range(1..BOARD_WIDTH - 1);
            let y = rng.gen_range(1..BOARD_HEIGHT - 1);

            if self.board[y][x] == Tile::Floor
                && (x != self.player_x || y != self.player_y)
                && !self.enemies.iter().any(|e| e.x == x && e.y == y)
                && !self.items.iter().any(|i| i.x == x && i.y == y)
                && !self.friendlies.iter().any(|f| f.x == x && f.y == y)
            {
                return Some((x, y));
            }
        }
        None
    }

    /// Update visibility based on torch radius
    fn update_visibility(&mut self) {
        // Clear visibility
        for row in &mut self.visible {
            for cell in row.iter_mut() {
                *cell = false;
            }
        }

        // Simple circular visibility
        let px = self.player_x as i32;
        let py = self.player_y as i32;

        for dy in -TORCH_RADIUS..=TORCH_RADIUS {
            for dx in -TORCH_RADIUS..=TORCH_RADIUS {
                let dist_sq = dx * dx + dy * dy;
                if dist_sq <= TORCH_RADIUS * TORCH_RADIUS {
                    let x = px + dx;
                    let y = py + dy;

                    if x >= 0 && y >= 0 && x < BOARD_WIDTH as i32 && y < BOARD_HEIGHT as i32 {
                        // Simple line-of-sight check
                        if self.has_los(px, py, x, y) {
                            self.visible[y as usize][x as usize] = true;
                            self.explored[y as usize][x as usize] = true;
                        }
                    }
                }
            }
        }
    }

    /// Simple line-of-sight check using Bresenham's
    fn has_los(&self, x0: i32, y0: i32, x1: i32, y1: i32) -> bool {
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;

        let mut x = x0;
        let mut y = y0;

        loop {
            if x == x1 && y == y1 {
                return true;
            }

            // Check if wall blocks LOS (but allow seeing the wall itself)
            if (x != x0 || y != y0) && self.board[y as usize][x as usize] == Tile::Wall {
                return x == x1 && y == y1;
            }

            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Move player in direction
    fn move_player(&mut self, dx: i32, dy: i32) {
        let new_x = (self.player_x as i32 + dx) as usize;
        let new_y = (self.player_y as i32 + dy) as usize;

        if new_x >= BOARD_WIDTH || new_y >= BOARD_HEIGHT {
            return;
        }

        // Check for enemy collision (bump to attack)
        if let Some(idx) = self
            .enemies
            .iter()
            .position(|e| e.x == new_x && e.y == new_y)
        {
            self.attack_enemy(idx);
            return;
        }

        // Check for friendly collision (bump to interact)
        if let Some(idx) = self
            .friendlies
            .iter()
            .position(|f| f.x == new_x && f.y == new_y)
        {
            self.interact_friendly(idx);
            return;
        }

        // Check tile passability
        let tile = self.board[new_y][new_x];

        match tile {
            Tile::Door => {
                if self.keys > 0 {
                    self.keys -= 1;
                    self.board[new_y][new_x] = Tile::DoorOpen;
                    self.add_message("You unlock the door!");
                } else {
                    self.add_message("The door is locked. You need a key.");
                }
                return;
            }
            Tile::Exit => {
                self.descend_floor();
                return;
            }
            _ => {}
        }

        if !tile.is_passable() {
            return;
        }

        // Move player
        self.player_x = new_x;
        self.player_y = new_y;

        // Check for trap
        if self.board[new_y][new_x] == Tile::TrapHidden {
            self.board[new_y][new_x] = Tile::Trap;
            let damage = 2 + self.floor as i32 / 3;
            self.hp -= damage;
            self.add_message(format!("You triggered a trap! -{} HP", damage));
            if self.hp <= 0 {
                self.die();
            }
        } else if self.board[new_y][new_x] == Tile::Trap {
            let damage = 1 + self.floor as i32 / 4;
            self.hp -= damage;
            self.add_message(format!("The trap hurts! -{} HP", damage));
            if self.hp <= 0 {
                self.die();
            }
        }

        // Pick up items
        self.pickup_items();

        self.update_visibility();
    }

    /// Pick up items at player position
    fn pickup_items(&mut self) {
        let items_here: Vec<_> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, i)| i.x == self.player_x && i.y == self.player_y)
            .map(|(idx, i)| (idx, i.item_type, i.value))
            .collect();

        // Remove in reverse order to maintain indices
        for (idx, item_type, value) in items_here.into_iter().rev() {
            match item_type {
                ItemType::Gold => {
                    self.gold += value;
                    self.add_message(format!("Picked up {} gold!", value));
                }
                ItemType::Food => {
                    self.hp = (self.hp + value as i32).min(self.max_hp);
                    self.add_message(format!("Ate food! +{} HP", value));
                }
                ItemType::Key => {
                    self.keys += 1;
                    self.add_message("Found a key!");
                }
                ItemType::Potion => {
                    self.hp = (self.hp + value as i32).min(self.max_hp);
                    self.add_message(format!("Drank potion! +{} HP", value));
                }
                ItemType::Weapon => {
                    self.attack += value as i32;
                    self.add_message(format!("Found a weapon! +{} ATK", value));
                }
            }
            self.items.remove(idx);
        }
    }

    /// Attack an enemy
    fn attack_enemy(&mut self, idx: usize) {
        let damage = self.attack + rand::thread_rng().gen_range(0..3);
        let enemy = &mut self.enemies[idx];
        enemy.hp -= damage;

        let enemy_name = enemy.enemy_type.name();

        if enemy.hp <= 0 {
            let xp_gain = enemy.enemy_type.xp();
            self.xp += xp_gain;
            self.add_message(format!("Defeated {}! +{} XP", enemy_name, xp_gain));
            self.enemies.remove(idx);
            self.check_level_up();
        } else {
            self.add_message(format!("Hit {} for {} damage!", enemy_name, damage));

            // Enemy counterattacks
            let counter = self.enemies[idx].enemy_type.damage();
            self.hp -= counter;
            self.add_message(format!("{} hits back for {}!", enemy_name, counter));

            if self.hp <= 0 {
                self.die();
            }
        }
    }

    /// Interact with a friendly NPC
    fn interact_friendly(&mut self, idx: usize) {
        let friendly_type = self.friendlies[idx].friendly_type;
        let talked = self.friendlies[idx].talked;

        match friendly_type {
            FriendlyType::Sheep => {
                // Sheep runs away and drops gold
                if !talked {
                    let gold = rand::thread_rng().gen_range(5..15);
                    self.gold += gold;
                    self.add_message(format!("The sheep drops {} gold and flees!", gold));
                    self.friendlies[idx].talked = true;
                    // Move sheep away
                    if let Some((x, y)) = self.find_empty_spot(&mut rand::thread_rng()) {
                        self.friendlies[idx].x = x;
                        self.friendlies[idx].y = y;
                    }
                } else {
                    self.add_message("The sheep bleats nervously.");
                }
            }
            FriendlyType::Merchant => {
                if self.gold >= 20 {
                    self.gold -= 20;
                    self.hp = self.max_hp;
                    self.add_message("Merchant: 'Here, take this healing!' (-20g, full HP)");
                } else {
                    self.add_message("Merchant: 'Come back with 20 gold!'");
                }
            }
            FriendlyType::Fairy => {
                let heal = 5 + self.floor as i32;
                self.hp = (self.hp + heal).min(self.max_hp);
                self.add_message(format!("Fairy heals you! +{} HP", heal));
            }
        }
    }

    /// Check for level up
    fn check_level_up(&mut self) {
        let xp_needed = self.level * 100;
        if self.xp >= xp_needed {
            self.xp -= xp_needed;
            self.level += 1;
            self.max_hp += 5;
            self.hp = self.max_hp;
            self.attack += 1;
            self.add_message(format!("LEVEL UP! You are now level {}!", self.level));
        }
    }

    /// Descend to next floor
    fn descend_floor(&mut self) {
        self.floor += 1;
        if self.floor > self.max_floor {
            self.max_floor = self.floor;
        }

        if self.floor > 10 {
            // Win condition: clear 10 floors
            self.game_won = true;
            self.game_over = true;
            self.add_message("You escaped the dungeon! YOU WIN!");
            self.pending_events.push(GameEvent::GameEnded { won: true });
        } else {
            self.add_message(format!("Descending to floor {}...", self.floor));
            self.generate_floor();
            self.update_visibility();
        }
    }

    /// Player dies
    fn die(&mut self) {
        self.hp = 0;
        self.game_over = true;
        self.add_message("You have died...");
        self.pending_events
            .push(GameEvent::GameEnded { won: false });
    }

    /// Move all enemies (called each tick)
    fn move_enemies(&mut self) {
        // First pass: collect all intended moves and attacks
        let mut moves: Vec<(usize, usize, usize)> = Vec::new(); // (idx, new_x, new_y)
        let mut attacks: Vec<(EnemyType, i32)> = Vec::new(); // (enemy_type, damage)

        for i in 0..self.enemies.len() {
            let enemy = &self.enemies[i];
            let new_counter = enemy.move_counter + 1;

            if new_counter < enemy.enemy_type.speed() {
                continue;
            }

            let enemy_x = enemy.x;
            let enemy_y = enemy.y;
            let enemy_type = enemy.enemy_type;
            let can_phase = enemy_type.can_phase();

            // Use A* to find path to player
            if let Some((next_x, next_y)) = find_path(
                &self.board,
                enemy_x,
                enemy_y,
                self.player_x,
                self.player_y,
                can_phase,
            ) {
                if next_x == self.player_x && next_y == self.player_y {
                    attacks.push((enemy_type, enemy_type.damage()));
                } else {
                    moves.push((i, next_x, next_y));
                }
            }
        }

        // Second pass: update counters
        for enemy in &mut self.enemies {
            enemy.move_counter += 1;
            if enemy.move_counter >= enemy.enemy_type.speed() {
                enemy.move_counter = 0;
            }
        }

        // Third pass: apply moves (check for collisions)
        for (idx, new_x, new_y) in moves {
            let occupied = self
                .enemies
                .iter()
                .enumerate()
                .any(|(j, e)| j != idx && e.x == new_x && e.y == new_y);

            if !occupied {
                self.enemies[idx].x = new_x;
                self.enemies[idx].y = new_y;
            }
        }

        // Fourth pass: apply attacks
        for (enemy_type, damage) in attacks {
            self.hp -= damage;
            self.add_message(format!("{} attacks! -{} HP", enemy_type.name(), damage));
            if self.hp <= 0 {
                self.die();
                return;
            }
        }
    }

    /// Fairy passive healing (called each tick)
    fn fairy_healing(&mut self) {
        for friendly in &self.friendlies {
            if friendly.friendly_type == FriendlyType::Fairy {
                let dist = ((friendly.x as i32 - self.player_x as i32).abs()
                    + (friendly.y as i32 - self.player_y as i32).abs())
                    as u32;

                // Heal if within range
                if dist <= 3 && self.hp < self.max_hp && self.tick_counter.is_multiple_of(30) {
                    self.hp = (self.hp + 1).min(self.max_hp);
                    // Don't spam messages
                }
            }
        }
    }
}

// =============================================================================
// GAME ENGINE IMPLEMENTATION
// =============================================================================

impl GameEngine for DungeonState {
    fn tick(&mut self) {
        if self.game_over {
            return;
        }

        self.tick_counter += 1;

        // Move enemies every few ticks
        if self.tick_counter.is_multiple_of(3) {
            self.move_enemies();
        }

        // Fairy passive healing
        self.fairy_healing();
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        if self.game_over {
            return match key.code {
                KeyCode::Enter => {
                    self.reset();
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => KeyHandleResult::RequestQuit,
                _ => KeyHandleResult::Handled,
            };
        }

        match key.code {
            // Movement
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('w') => {
                self.move_player(0, -1);
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('s') => {
                self.move_player(0, 1);
                KeyHandleResult::Handled
            }
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('a') => {
                self.move_player(-1, 0);
                KeyHandleResult::Handled
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('d') => {
                self.move_player(1, 0);
                KeyHandleResult::Handled
            }
            // Diagonal movement
            KeyCode::Char('y') => {
                self.move_player(-1, -1);
                KeyHandleResult::Handled
            }
            KeyCode::Char('u') => {
                self.move_player(1, -1);
                KeyHandleResult::Handled
            }
            KeyCode::Char('b') => {
                self.move_player(-1, 1);
                KeyHandleResult::Handled
            }
            KeyCode::Char('n') => {
                self.move_player(1, 1);
                KeyHandleResult::Handled
            }
            // Wait (skip turn but enemies move)
            KeyCode::Char('.') | KeyCode::Char(' ') => KeyHandleResult::Handled,
            // Descend stairs
            KeyCode::Char('>') => {
                if self.board[self.player_y][self.player_x] == Tile::Exit {
                    self.descend_floor();
                } else {
                    self.add_message("No stairs here.");
                }
                KeyHandleResult::Handled
            }
            // Quit
            KeyCode::Esc => KeyHandleResult::RequestQuit,
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn get_score(&self) -> u32 {
        // Score = gold + xp + floor bonus
        self.gold + self.xp + (self.floor * 100)
    }

    fn get_level(&self) -> Option<u32> {
        Some(self.floor)
    }

    fn is_game_over(&self) -> bool {
        self.game_over
    }

    fn is_game_won(&self) -> bool {
        self.game_won
    }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.pending_events)
    }
}
