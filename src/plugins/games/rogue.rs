//! Rogue game implementation
//!
//! A classic ASCII dungeon crawler roguelike.

use super::platform::{GameEngine, GameEvent, KeyHandleResult};
use crossterm::event::{KeyCode, KeyEvent};
use rand::Rng;

/// Board dimensions
pub const BOARD_WIDTH: usize = 60;
pub const BOARD_HEIGHT: usize = 20;

/// Room dimensions
const MIN_ROOM_SIZE: usize = 4;
const MAX_ROOM_SIZE: usize = 10;
const MAX_ROOMS: usize = 8;

/// Tile types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    Floor,
    Wall,
    Door,
    Corridor,
    StairsDown,
    StairsUp,
    Trap,
    HiddenTrap,
}

impl Tile {
    pub fn is_passable(&self) -> bool {
        matches!(
            self,
            Tile::Floor
                | Tile::Door
                | Tile::Corridor
                | Tile::StairsDown
                | Tile::StairsUp
                | Tile::Trap
                | Tile::HiddenTrap
        )
    }

    pub fn char(&self) -> char {
        match self {
            Tile::Floor => '.',
            Tile::Wall => '#',
            Tile::Door => '+',
            Tile::Corridor => '#',
            Tile::StairsDown => '%',
            Tile::StairsUp => '<',
            Tile::Trap => '^',
            Tile::HiddenTrap => '.', // Looks like floor until revealed
        }
    }
}

/// Entity types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    Gold,
    Food,
    Potion,
    Scroll,
    Weapon,
    Armor,
}

impl EntityType {
    pub fn char(&self) -> char {
        match self {
            EntityType::Gold => '*',   // Piles of gold
            EntityType::Food => ':',   // Food rations
            EntityType::Potion => '!', // Magic potions
            EntityType::Scroll => '?', // Magic scrolls
            EntityType::Weapon => ')', // Weapons
            EntityType::Armor => ']',  // Armor pieces
        }
    }
}

/// Entity on the map
#[derive(Debug, Clone)]
pub struct Entity {
    pub x: usize,
    pub y: usize,
    pub entity_type: EntityType,
}

/// Monster types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonsterType {
    Bat,      // B - weak, fast
    Goblin,   // G - basic enemy
    Orc,      // O - tougher
    Troll,    // T - strong
    Dragon,   // D - boss
    Skeleton, // S - undead
    Rat,      // R - very weak
}

impl MonsterType {
    pub fn char(&self) -> char {
        match self {
            MonsterType::Bat => 'B',
            MonsterType::Goblin => 'G',
            MonsterType::Orc => 'O',
            MonsterType::Troll => 'T',
            MonsterType::Dragon => 'D',
            MonsterType::Skeleton => 'S',
            MonsterType::Rat => 'R',
        }
    }

    pub fn health(&self) -> i32 {
        match self {
            MonsterType::Rat => 2,
            MonsterType::Bat => 3,
            MonsterType::Goblin => 5,
            MonsterType::Skeleton => 6,
            MonsterType::Orc => 8,
            MonsterType::Troll => 12,
            MonsterType::Dragon => 20,
        }
    }

    pub fn damage(&self) -> i32 {
        match self {
            MonsterType::Rat => 1,
            MonsterType::Bat => 2,
            MonsterType::Goblin => 3,
            MonsterType::Skeleton => 4,
            MonsterType::Orc => 5,
            MonsterType::Troll => 7,
            MonsterType::Dragon => 10,
        }
    }

    pub fn xp(&self) -> u32 {
        match self {
            MonsterType::Rat => 5,
            MonsterType::Bat => 10,
            MonsterType::Goblin => 15,
            MonsterType::Skeleton => 20,
            MonsterType::Orc => 30,
            MonsterType::Troll => 50,
            MonsterType::Dragon => 100,
        }
    }

    pub fn for_level(level: u32) -> Self {
        let mut rng = rand::thread_rng();
        let roll: u32 = rng.gen_range(0..100);

        match level {
            1 => {
                if roll < 50 {
                    MonsterType::Rat
                } else {
                    MonsterType::Bat
                }
            }
            2..=3 => {
                if roll < 40 {
                    MonsterType::Bat
                } else if roll < 80 {
                    MonsterType::Goblin
                } else {
                    MonsterType::Skeleton
                }
            }
            4..=6 => {
                if roll < 30 {
                    MonsterType::Goblin
                } else if roll < 60 {
                    MonsterType::Skeleton
                } else if roll < 90 {
                    MonsterType::Orc
                } else {
                    MonsterType::Troll
                }
            }
            _ => {
                if roll < 20 {
                    MonsterType::Orc
                } else if roll < 50 {
                    MonsterType::Troll
                } else if roll < 90 {
                    MonsterType::Skeleton
                } else {
                    MonsterType::Dragon
                }
            }
        }
    }
}

/// Monster on the map
#[derive(Debug, Clone)]
pub struct Monster {
    pub x: usize,
    pub y: usize,
    pub monster_type: MonsterType,
    pub health: i32,
}

impl Monster {
    pub fn new(x: usize, y: usize, monster_type: MonsterType) -> Self {
        Self {
            x,
            y,
            monster_type,
            health: monster_type.health(),
        }
    }
}

/// Room in the dungeon
#[derive(Debug, Clone)]
struct Room {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

impl Room {
    fn center(&self) -> (usize, usize) {
        (self.x + self.width / 2, self.y + self.height / 2)
    }

    fn intersects(&self, other: &Room) -> bool {
        self.x <= other.x + other.width
            && self.x + self.width >= other.x
            && self.y <= other.y + other.height
            && self.y + self.height >= other.y
    }
}

/// Rogue game state
pub struct RogueState {
    pub board: [[Tile; BOARD_WIDTH]; BOARD_HEIGHT],
    pub player_x: usize,
    pub player_y: usize,
    pub health: i32,
    pub max_health: i32,
    pub gold: u32,
    pub level: u32,
    pub dungeon_level: u32,
    pub xp: u32,
    pub xp_to_next: u32,
    pub attack: i32,
    pub defense: i32,
    pub strength: i32,
    pub hunger: i32, // 0-1000, lower = hungrier
    pub max_hunger: i32,
    pub monsters: Vec<Monster>,
    pub entities: Vec<Entity>,
    pub messages: Vec<String>, // Message log (shows last N messages)
    pub max_messages: usize,   // Max messages to keep
    pub game_over: bool,
    pub game_won: bool,
    pub explored: [[bool; BOARD_WIDTH]; BOARD_HEIGHT],
    pub visible: [[bool; BOARD_WIDTH]; BOARD_HEIGHT], // Currently visible tiles (shadowcasting)
    tick_counter: u32,
    pending_events: Vec<GameEvent>,
}

impl Default for RogueState {
    fn default() -> Self {
        Self::new()
    }
}

impl RogueState {
    pub fn new() -> Self {
        let mut state = Self {
            board: [[Tile::Wall; BOARD_WIDTH]; BOARD_HEIGHT],
            player_x: 1,
            player_y: 1,
            health: 12,
            max_health: 12,
            gold: 0,
            level: 1,
            dungeon_level: 1,
            xp: 0,
            xp_to_next: 100,
            attack: 4,
            defense: 5,
            strength: 16,
            hunger: 1000,
            max_hunger: 1000,
            monsters: Vec::new(),
            entities: Vec::new(),
            messages: vec!["Welcome to the Dungeons of Doom!".to_string()],
            max_messages: 5,
            game_over: false,
            game_won: false,
            explored: [[false; BOARD_WIDTH]; BOARD_HEIGHT],
            visible: [[false; BOARD_WIDTH]; BOARD_HEIGHT],
            tick_counter: 0,
            pending_events: Vec::new(),
        };
        state.generate_dungeon();
        state.pending_events.push(GameEvent::GameStarted);
        state
    }

    /// Add a message to the log
    pub fn add_message(&mut self, msg: impl Into<String>) {
        self.messages.push(msg.into());
        while self.messages.len() > self.max_messages {
            self.messages.remove(0);
        }
    }

    /// Get the most recent message (for compatibility)
    pub fn last_message(&self) -> Option<&String> {
        self.messages.last()
    }

    pub fn reset(&mut self) {
        *self = Self::new();
        self.pending_events.push(GameEvent::GameStarted);
    }

    fn generate_dungeon(&mut self) {
        let mut rng = rand::thread_rng();
        let mut rooms: Vec<Room> = Vec::new();

        // Clear the board
        for y in 0..BOARD_HEIGHT {
            for x in 0..BOARD_WIDTH {
                self.board[y][x] = Tile::Wall;
                self.explored[y][x] = false;
            }
        }

        // Generate rooms
        for _ in 0..MAX_ROOMS {
            let w = rng.gen_range(MIN_ROOM_SIZE..=MAX_ROOM_SIZE);
            let h = rng.gen_range(MIN_ROOM_SIZE..=MAX_ROOM_SIZE);
            let x = rng.gen_range(1..BOARD_WIDTH - w - 1);
            let y = rng.gen_range(1..BOARD_HEIGHT - h - 1);

            let new_room = Room {
                x,
                y,
                width: w,
                height: h,
            };

            // Check for overlaps
            let mut overlaps = false;
            for room in &rooms {
                if new_room.intersects(room) {
                    overlaps = true;
                    break;
                }
            }

            if !overlaps {
                // Carve out the room
                for ry in new_room.y..new_room.y + new_room.height {
                    for rx in new_room.x..new_room.x + new_room.width {
                        self.board[ry][rx] = Tile::Floor;
                    }
                }

                // Connect to previous room
                if !rooms.is_empty() {
                    let (new_x, new_y) = new_room.center();
                    let (prev_x, prev_y) = rooms.last().unwrap().center();

                    // Randomly choose horizontal or vertical first
                    if rng.gen_bool(0.5) {
                        self.carve_h_corridor(prev_x, new_x, prev_y);
                        self.carve_v_corridor(prev_y, new_y, new_x);
                    } else {
                        self.carve_v_corridor(prev_y, new_y, prev_x);
                        self.carve_h_corridor(prev_x, new_x, new_y);
                    }
                }

                rooms.push(new_room);
            }
        }

        // Place player in first room
        if !rooms.is_empty() {
            let (px, py) = rooms[0].center();
            self.player_x = px;
            self.player_y = py;

            // Place stairs down in last room
            let (sx, sy) = rooms.last().unwrap().center();
            self.board[sy][sx] = Tile::StairsDown;

            // Spawn monsters
            self.monsters.clear();
            let monster_count = 2 + self.dungeon_level as usize;
            for room in rooms.iter().skip(1) {
                if self.monsters.len() >= monster_count {
                    break;
                }
                let mx = rng.gen_range(room.x + 1..room.x + room.width - 1);
                let my = rng.gen_range(room.y + 1..room.y + room.height - 1);
                if self.board[my][mx].is_passable() {
                    let monster_type = MonsterType::for_level(self.dungeon_level);
                    self.monsters.push(Monster::new(mx, my, monster_type));
                }
            }

            // Spawn items
            self.entities.clear();
            for room in rooms.iter().skip(1) {
                // Gold in every room
                let gx = rng.gen_range(room.x + 1..room.x + room.width - 1);
                let gy = rng.gen_range(room.y + 1..room.y + room.height - 1);
                self.entities.push(Entity {
                    x: gx,
                    y: gy,
                    entity_type: EntityType::Gold,
                });

                // Random item chance
                if rng.gen_bool(0.4) {
                    let ix = rng.gen_range(room.x + 1..room.x + room.width - 1);
                    let iy = rng.gen_range(room.y + 1..room.y + room.height - 1);
                    let item_type = match rng.gen_range(0..5) {
                        0 => EntityType::Food,
                        1 => EntityType::Potion,
                        2 => EntityType::Scroll,
                        3 => EntityType::Weapon,
                        _ => EntityType::Armor,
                    };
                    self.entities.push(Entity {
                        x: ix,
                        y: iy,
                        entity_type: item_type,
                    });
                }

                // Hidden trap chance (increases with dungeon level)
                let trap_chance = 0.1 + (self.dungeon_level as f64 * 0.05);
                if rng.gen_bool(trap_chance.min(0.5)) {
                    let tx = rng.gen_range(room.x + 1..room.x + room.width - 1);
                    let ty = rng.gen_range(room.y + 1..room.y + room.height - 1);
                    if self.board[ty][tx] == Tile::Floor {
                        self.board[ty][tx] = Tile::HiddenTrap;
                    }
                }
            }
        }

        // Update explored tiles
        self.update_visibility();
    }

    fn carve_h_corridor(&mut self, x1: usize, x2: usize, y: usize) {
        let (start, end) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
        for x in start..=end {
            self.board[y][x] = Tile::Corridor;
        }
    }

    fn carve_v_corridor(&mut self, y1: usize, y2: usize, x: usize) {
        let (start, end) = if y1 < y2 { (y1, y2) } else { (y2, y1) };
        for y in start..=end {
            self.board[y][x] = Tile::Corridor;
        }
    }

    /// Update visibility using recursive shadowcasting algorithm
    /// This properly handles line-of-sight - you can't see through walls
    fn update_visibility(&mut self) {
        let radius = 8;

        // Clear current visibility
        for y in 0..BOARD_HEIGHT {
            for x in 0..BOARD_WIDTH {
                self.visible[y][x] = false;
            }
        }

        // Player position is always visible
        self.visible[self.player_y][self.player_x] = true;
        self.explored[self.player_y][self.player_x] = true;

        // Cast light in all 8 octants
        for octant in 0..8 {
            self.cast_light(
                self.player_x as i32,
                self.player_y as i32,
                radius,
                1,
                1.0,
                0.0,
                octant,
            );
        }
    }

    /// Recursive shadowcasting for a single octant
    /// Based on the algorithm from RogueBasin
    fn cast_light(
        &mut self,
        cx: i32,
        cy: i32,
        radius: i32,
        row: i32,
        mut start_slope: f64,
        end_slope: f64,
        octant: u8,
    ) {
        if start_slope < end_slope {
            return;
        }

        let mut blocked = false;
        let mut next_start_slope = start_slope;

        for j in row..=radius {
            if blocked {
                break;
            }

            for dx in (-j)..=0 {
                let dy = -j;

                // Calculate slopes
                let l_slope = (dx as f64 - 0.5) / (dy as f64 + 0.5);
                let r_slope = (dx as f64 + 0.5) / (dy as f64 - 0.5);

                if start_slope < r_slope {
                    continue;
                }
                if end_slope > l_slope {
                    break;
                }

                // Transform to actual coordinates based on octant
                let (ax, ay) = self.transform_octant(cx, cy, dx, dy, octant);

                // Check bounds
                if ax < 0 || ax >= BOARD_WIDTH as i32 || ay < 0 || ay >= BOARD_HEIGHT as i32 {
                    continue;
                }

                let ax = ax as usize;
                let ay = ay as usize;

                // Check if within radius (circular)
                let dist_sq = dx * dx + dy * dy;
                if dist_sq <= radius * radius {
                    self.visible[ay][ax] = true;
                    self.explored[ay][ax] = true;
                }

                // Check if this tile blocks light
                let tile = self.board[ay][ax];
                let blocks = matches!(tile, Tile::Wall);

                if blocked {
                    if blocks {
                        next_start_slope = r_slope;
                    } else {
                        blocked = false;
                        start_slope = next_start_slope;
                    }
                } else if blocks && j < radius {
                    blocked = true;
                    self.cast_light(cx, cy, radius, j + 1, start_slope, l_slope, octant);
                    next_start_slope = r_slope;
                }
            }
        }
    }

    /// Transform coordinates based on octant (for shadowcasting)
    fn transform_octant(&self, cx: i32, cy: i32, dx: i32, dy: i32, octant: u8) -> (i32, i32) {
        match octant {
            0 => (cx + dx, cy + dy),
            1 => (cx + dy, cy + dx),
            2 => (cx + dy, cy - dx),
            3 => (cx + dx, cy - dy),
            4 => (cx - dx, cy - dy),
            5 => (cx - dy, cy - dx),
            6 => (cx - dy, cy + dx),
            7 => (cx - dx, cy + dy),
            _ => (cx, cy),
        }
    }

    pub fn move_player(&mut self, dx: i32, dy: i32) {
        if self.game_over || self.game_won {
            return;
        }

        let new_x = (self.player_x as i32 + dx) as usize;
        let new_y = (self.player_y as i32 + dy) as usize;

        if new_x >= BOARD_WIDTH || new_y >= BOARD_HEIGHT {
            return;
        }

        // Check for monster collision
        if let Some(idx) = self
            .monsters
            .iter()
            .position(|m| m.x == new_x && m.y == new_y)
        {
            self.attack_monster(idx);
            return;
        }

        if self.board[new_y][new_x].is_passable() {
            self.player_x = new_x;
            self.player_y = new_y;

            // Check for traps
            self.check_trap();

            // Check for stairs
            if self.board[new_y][new_x] == Tile::StairsDown {
                self.descend();
            }

            // Check for items
            self.pickup_items();

            // Update visibility
            self.update_visibility();
        }
    }

    fn attack_monster(&mut self, idx: usize) {
        let damage = (self.attack - self.monsters[idx].monster_type.damage() / 2).max(1);
        self.monsters[idx].health -= damage;

        let monster = &self.monsters[idx];
        if monster.health <= 0 {
            let xp = monster.monster_type.xp();
            let monster_name = format!("{:?}", monster.monster_type);
            self.add_message(format!(
                "Killed {}! +{} XP",
                monster.monster_type.char(),
                xp
            ));
            let old_score = self.gold;
            self.xp += xp;
            self.monsters.remove(idx);

            // Emit events
            self.pending_events.push(GameEvent::EnemyDefeated {
                enemy_type: monster_name,
            });
            self.pending_events.push(GameEvent::ScoreChanged {
                old: old_score,
                new: self.gold,
            });

            self.check_level_up();
        } else {
            // Monster attacks back
            let monster_dmg = (monster.monster_type.damage() - self.defense).max(1);
            self.health -= monster_dmg;
            self.add_message(format!(
                "Hit {}! Took {} damage",
                monster.monster_type.char(),
                monster_dmg
            ));

            if self.health <= 0 {
                self.game_over = true;
                self.add_message("You died!");
                self.pending_events
                    .push(GameEvent::GameEnded { won: false });
            }
        }
    }

    fn check_level_up(&mut self) {
        if self.xp >= self.xp_to_next {
            self.level += 1;
            self.xp -= self.xp_to_next;
            self.xp_to_next = self.xp_to_next * 3 / 2;
            self.max_health += 5;
            self.health = self.max_health;
            self.attack += 2;
            self.defense += 1;
            self.add_message(format!("Level up! Now level {}", self.level));
            self.pending_events
                .push(GameEvent::LevelReached(self.level));
        }
    }

    fn pickup_items(&mut self) {
        let mut rng = rand::thread_rng();
        let mut i = 0;
        while i < self.entities.len() {
            let entity = &self.entities[i];
            if entity.x == self.player_x && entity.y == self.player_y {
                match entity.entity_type {
                    EntityType::Gold => {
                        let amount = 10 + self.dungeon_level * 5;
                        self.gold += amount;
                        self.add_message(format!("Found {} gold!", amount));
                    }
                    EntityType::Food => {
                        // Food restores hunger
                        self.hunger = self.max_hunger;
                        self.add_message("You feel full.");
                    }
                    EntityType::Potion => {
                        // Random potion effect
                        let effect = rng.gen_range(0..3);
                        match effect {
                            0 => {
                                let heal = rng.gen_range(5..15);
                                self.health = (self.health + heal).min(self.max_health);
                                self.add_message(format!("Healing potion! +{} HP", heal));
                            }
                            1 => {
                                self.strength += 1;
                                self.add_message("Potion of strength! +1 STR");
                            }
                            _ => {
                                let damage = rng.gen_range(1..5);
                                self.health -= damage;
                                self.add_message(format!("Poison! -{} HP", damage));
                            }
                        }
                    }
                    EntityType::Scroll => {
                        // Random scroll effect
                        let effect = rng.gen_range(0..4);
                        match effect {
                            0 => {
                                // Scroll of identify - reveal all items
                                self.add_message("Scroll of light! Area revealed.");
                                self.reveal_area();
                            }
                            1 => {
                                // Scroll of teleport
                                self.add_message("Scroll of teleport!");
                                self.teleport_random();
                            }
                            2 => {
                                // Scroll of monster confusion
                                self.add_message("Scroll of scare monster!");
                            }
                            _ => {
                                // Scroll of magic mapping
                                for y in 0..BOARD_HEIGHT {
                                    for x in 0..BOARD_WIDTH {
                                        self.explored[y][x] = true;
                                    }
                                }
                                self.add_message("Scroll of magic mapping!");
                            }
                        }
                    }
                    EntityType::Weapon => {
                        let bonus = rng.gen_range(1..=3);
                        self.attack += bonus;
                        self.add_message(format!("Found weapon! +{} ATK", bonus));
                    }
                    EntityType::Armor => {
                        let bonus = rng.gen_range(1..=2);
                        self.defense += bonus;
                        self.add_message(format!("Found armor! +{} DEF", bonus));
                    }
                }
                self.entities.remove(i);
            } else {
                i += 1;
            }
        }
    }

    fn descend(&mut self) {
        self.dungeon_level += 1;
        if self.dungeon_level > 10 {
            self.game_won = true;
            self.add_message("You escaped the dungeon! You win!");
            self.pending_events.push(GameEvent::GameEnded { won: true });
        } else {
            self.add_message(format!("Descending to level {}...", self.dungeon_level));
            self.pending_events
                .push(GameEvent::FloorReached(self.dungeon_level));
            self.generate_dungeon();
        }
    }

    fn tick_internal(&mut self) {
        if self.game_over || self.game_won {
            return;
        }

        self.tick_counter += 1;

        // Move monsters every 3 ticks
        if self.tick_counter.is_multiple_of(3) {
            self.move_monsters();
        }

        // Decrease hunger every 10 ticks
        if self.tick_counter.is_multiple_of(10) {
            self.hunger = (self.hunger - 1).max(0);
            if self.hunger == 0 {
                // Starving - take damage
                self.health -= 1;
                self.add_message("You are starving!");
                if self.health <= 0 {
                    self.game_over = true;
                    self.add_message("You starved to death!");
                    self.pending_events
                        .push(GameEvent::GameEnded { won: false });
                }
            } else if self.hunger == 300 {
                self.add_message("You are getting hungry.");
            } else if self.hunger == 100 {
                self.add_message("You are weak from hunger!");
            }
        }
    }

    fn move_monsters(&mut self) {
        let mut rng = rand::thread_rng();

        for i in 0..self.monsters.len() {
            let monster = &self.monsters[i];
            let mx = monster.x;
            let my = monster.y;

            // Check if player is adjacent
            let dx = self.player_x as i32 - mx as i32;
            let dy = self.player_y as i32 - my as i32;

            if dx.abs() <= 1 && dy.abs() <= 1 && (dx != 0 || dy != 0) {
                // Attack player
                let damage = (monster.monster_type.damage() - self.defense).max(1);
                self.health -= damage;
                self.add_message(format!(
                    "{} attacks! -{} HP",
                    monster.monster_type.char(),
                    damage
                ));

                if self.health <= 0 {
                    self.game_over = true;
                    self.add_message("You died!");
                }
            } else if dx.abs() + dy.abs() < 15 {
                // Use A* pathfinding to find path to player
                if let Some((next_x, next_y)) = self.find_path_astar(mx, my) {
                    // Check if target is not occupied by another monster
                    let occupied = self.monsters.iter().any(|m| m.x == next_x && m.y == next_y);
                    let is_player = next_x == self.player_x && next_y == self.player_y;
                    if !occupied && !is_player {
                        self.monsters[i].x = next_x;
                        self.monsters[i].y = next_y;
                    }
                }
            } else {
                // Random movement when far from player
                let dirs = [(0, 1), (0, -1), (1, 0), (-1, 0)];
                let (ddx, ddy) = dirs[rng.gen_range(0..4)];
                let new_x = (mx as i32 + ddx) as usize;
                let new_y = (my as i32 + ddy) as usize;

                if new_x < BOARD_WIDTH
                    && new_y < BOARD_HEIGHT
                    && self.board[new_y][new_x].is_passable()
                    && !self.monsters.iter().any(|m| m.x == new_x && m.y == new_y)
                    && !(new_x == self.player_x && new_y == self.player_y)
                {
                    self.monsters[i].x = new_x;
                    self.monsters[i].y = new_y;
                }
            }
        }
    }

    /// A* pathfinding - returns the next step toward the player
    fn find_path_astar(&self, start_x: usize, start_y: usize) -> Option<(usize, usize)> {
        use std::cmp::Ordering;
        use std::collections::{BinaryHeap, HashMap, HashSet};

        #[derive(Copy, Clone, Eq, PartialEq)]
        struct Node {
            x: usize,
            y: usize,
            f: i32, // f = g + h
        }

        impl Ord for Node {
            fn cmp(&self, other: &Self) -> Ordering {
                // Reverse order for min-heap behavior
                other.f.cmp(&self.f)
            }
        }

        impl PartialOrd for Node {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        let goal_x = self.player_x;
        let goal_y = self.player_y;

        // Heuristic: Manhattan distance
        let heuristic = |x: usize, y: usize| -> i32 {
            (x as i32 - goal_x as i32).abs() + (y as i32 - goal_y as i32).abs()
        };

        let mut open_set = BinaryHeap::new();
        let mut came_from: HashMap<(usize, usize), (usize, usize)> = HashMap::new();
        let mut g_score: HashMap<(usize, usize), i32> = HashMap::new();
        let mut closed_set: HashSet<(usize, usize)> = HashSet::new();

        g_score.insert((start_x, start_y), 0);
        open_set.push(Node {
            x: start_x,
            y: start_y,
            f: heuristic(start_x, start_y),
        });

        // Limit iterations to prevent infinite loops
        let mut iterations = 0;
        let max_iterations = 200;

        while let Some(current) = open_set.pop() {
            iterations += 1;
            if iterations > max_iterations {
                break;
            }

            let (cx, cy) = (current.x, current.y);

            // Check if we're adjacent to the goal (one step away)
            let dx = (cx as i32 - goal_x as i32).abs();
            let dy = (cy as i32 - goal_y as i32).abs();
            if dx <= 1 && dy <= 1 && (dx + dy > 0) {
                // Reconstruct path and return first step
                return self.reconstruct_first_step(&came_from, (cx, cy), (start_x, start_y));
            }

            if closed_set.contains(&(cx, cy)) {
                continue;
            }
            closed_set.insert((cx, cy));

            // 4-directional neighbors (8-directional for diagonal movement)
            let neighbors = [
                (cx.wrapping_sub(1), cy),
                (cx + 1, cy),
                (cx, cy.wrapping_sub(1)),
                (cx, cy + 1),
            ];

            for (nx, ny) in neighbors {
                if nx >= BOARD_WIDTH || ny >= BOARD_HEIGHT {
                    continue;
                }
                if closed_set.contains(&(nx, ny)) {
                    continue;
                }
                if !self.board[ny][nx].is_passable() {
                    continue;
                }

                let tentative_g = g_score
                    .get(&(cx, cy))
                    .unwrap_or(&i32::MAX)
                    .saturating_add(1);

                if tentative_g < *g_score.get(&(nx, ny)).unwrap_or(&i32::MAX) {
                    came_from.insert((nx, ny), (cx, cy));
                    g_score.insert((nx, ny), tentative_g);
                    let f = tentative_g + heuristic(nx, ny);
                    open_set.push(Node { x: nx, y: ny, f });
                }
            }
        }

        None
    }

    /// Reconstruct the first step in the path from start to goal
    fn reconstruct_first_step(
        &self,
        came_from: &std::collections::HashMap<(usize, usize), (usize, usize)>,
        goal: (usize, usize),
        start: (usize, usize),
    ) -> Option<(usize, usize)> {
        let mut current = goal;
        let mut prev = goal;

        // Walk backwards until we find the step right after start
        while current != start {
            if let Some(&parent) = came_from.get(&current) {
                prev = current;
                current = parent;
            } else {
                return None;
            }
        }

        Some(prev)
    }

    /// Check if a position is currently visible (calculated by shadowcasting)
    pub fn is_visible(&self, x: usize, y: usize) -> bool {
        if x < BOARD_WIDTH && y < BOARD_HEIGHT {
            self.visible[y][x]
        } else {
            false
        }
    }

    /// Reveal a larger area around the player (scroll of light effect)
    fn reveal_area(&mut self) {
        let radius: i32 = 10;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let nx = self.player_x as i32 + dx;
                let ny = self.player_y as i32 + dy;
                if nx >= 0
                    && nx < BOARD_WIDTH as i32
                    && ny >= 0
                    && ny < BOARD_HEIGHT as i32
                    && dx * dx + dy * dy <= radius * radius
                {
                    self.explored[ny as usize][nx as usize] = true;
                }
            }
        }
    }

    /// Teleport player to a random passable location
    fn teleport_random(&mut self) {
        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            let x = rng.gen_range(1..BOARD_WIDTH - 1);
            let y = rng.gen_range(1..BOARD_HEIGHT - 1);
            if self.board[y][x].is_passable() && !self.monsters.iter().any(|m| m.x == x && m.y == y)
            {
                self.player_x = x;
                self.player_y = y;
                self.update_visibility();
                return;
            }
        }
    }

    /// Search for hidden traps and doors in adjacent squares
    pub fn search(&mut self) {
        let mut rng = rand::thread_rng();
        let mut found = false;

        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                let nx = self.player_x as i32 + dx;
                let ny = self.player_y as i32 + dy;
                if nx >= 0 && nx < BOARD_WIDTH as i32 && ny >= 0 && ny < BOARD_HEIGHT as i32 {
                    let tile = &mut self.board[ny as usize][nx as usize];
                    if *tile == Tile::HiddenTrap && rng.gen_bool(0.5) {
                        *tile = Tile::Trap;
                        found = true;
                    }
                }
            }
        }

        if found {
            self.add_message("You found a trap!");
        } else {
            self.add_message("You search but find nothing.");
        }
    }

    /// Get hunger status string
    pub fn hunger_status(&self) -> &'static str {
        match self.hunger {
            0..=100 => "Fainting",
            101..=300 => "Weak",
            301..=500 => "Hungry",
            _ => "",
        }
    }

    /// Check if player stepped on a trap
    fn check_trap(&mut self) {
        let tile = self.board[self.player_y][self.player_x];
        if tile == Tile::Trap || tile == Tile::HiddenTrap {
            // Reveal hidden trap
            if tile == Tile::HiddenTrap {
                self.board[self.player_y][self.player_x] = Tile::Trap;
            }

            let mut rng = rand::thread_rng();
            let trap_type = rng.gen_range(0..4);
            match trap_type {
                0 => {
                    // Teleport trap
                    self.add_message("A teleport trap!");
                    self.teleport_random();
                }
                1 => {
                    // Dart trap
                    let damage = rng.gen_range(1..4);
                    self.health -= damage;
                    self.add_message(format!("A dart trap! -{} HP", damage));
                }
                2 => {
                    // Bear trap - lose a turn
                    self.add_message("A bear trap! You're stuck!");
                }
                _ => {
                    // Pit trap
                    let damage = rng.gen_range(2..6);
                    self.health -= damage;
                    self.add_message(format!("You fell into a pit! -{} HP", damage));
                }
            }

            if self.health <= 0 {
                self.game_over = true;
                self.add_message("You died!");
                self.pending_events
                    .push(GameEvent::GameEnded { won: false });
            }
        }
    }
}

// === GameEngine Implementation ===

impl GameEngine for RogueState {
    fn tick(&mut self) {
        self.tick_internal();
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_player(0, -1);
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_player(0, 1);
                KeyHandleResult::Handled
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.move_player(-1, 0);
                KeyHandleResult::Handled
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.move_player(1, 0);
                KeyHandleResult::Handled
            }
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
            KeyCode::Char('s') => {
                self.search();
                KeyHandleResult::Handled
            }
            KeyCode::Char('.') | KeyCode::Char(' ') => {
                // Rest/wait
                KeyHandleResult::Handled
            }
            KeyCode::Char('p') | KeyCode::Char('P') => KeyHandleResult::RequestPause,
            KeyCode::Esc => KeyHandleResult::RequestQuit,
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn get_score(&self) -> u32 {
        self.gold
    }

    fn is_game_over(&self) -> bool {
        self.game_over
    }

    fn is_game_won(&self) -> bool {
        self.game_won
    }

    fn get_level(&self) -> Option<u32> {
        Some(self.dungeon_level)
    }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.pending_events)
    }

    fn get_stat(&self, key: &str) -> Option<u64> {
        match key {
            "dungeon_level" => Some(self.dungeon_level as u64),
            "player_level" => Some(self.level as u64),
            "gold" => Some(self.gold as u64),
            "health" => Some(self.health as u64),
            _ => None,
        }
    }
}
