//! Rogue game implementation
//!
//! A classic ASCII dungeon crawler roguelike.

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
    pub message: Option<String>,
    pub game_over: bool,
    pub game_won: bool,
    pub explored: [[bool; BOARD_WIDTH]; BOARD_HEIGHT],
    tick_counter: u32,
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
            message: Some("Welcome to the Dungeons of Doom!".to_string()),
            game_over: false,
            game_won: false,
            explored: [[false; BOARD_WIDTH]; BOARD_HEIGHT],
            tick_counter: 0,
        };
        state.generate_dungeon();
        state
    }

    pub fn reset(&mut self) {
        *self = Self::new();
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

    fn update_visibility(&mut self) {
        // Simple visibility: reveal tiles in a radius around player
        let radius: i32 = 5;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let nx = (self.player_x as i32 + dx) as usize;
                let ny = (self.player_y as i32 + dy) as usize;
                if nx < BOARD_WIDTH && ny < BOARD_HEIGHT {
                    // Check line of sight (simple version)
                    if dx * dx + dy * dy <= radius * radius {
                        self.explored[ny][nx] = true;
                    }
                }
            }
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
            self.message = None;

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
            self.message = Some(format!(
                "Killed {}! +{} XP",
                monster.monster_type.char(),
                xp
            ));
            self.xp += xp;
            self.monsters.remove(idx);
            self.check_level_up();
        } else {
            // Monster attacks back
            let monster_dmg = (monster.monster_type.damage() - self.defense).max(1);
            self.health -= monster_dmg;
            self.message = Some(format!(
                "Hit {}! Took {} damage",
                monster.monster_type.char(),
                monster_dmg
            ));

            if self.health <= 0 {
                self.game_over = true;
                self.message = Some("You died!".to_string());
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
            self.message = Some(format!("Level up! Now level {}", self.level));
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
                        self.message = Some(format!("Found {} gold!", amount));
                    }
                    EntityType::Food => {
                        // Food restores hunger
                        self.hunger = self.max_hunger;
                        self.message = Some("You feel full.".to_string());
                    }
                    EntityType::Potion => {
                        // Random potion effect
                        let effect = rng.gen_range(0..3);
                        match effect {
                            0 => {
                                let heal = rng.gen_range(5..15);
                                self.health = (self.health + heal).min(self.max_health);
                                self.message = Some(format!("Healing potion! +{} HP", heal));
                            }
                            1 => {
                                self.strength += 1;
                                self.message = Some("Potion of strength! +1 STR".to_string());
                            }
                            _ => {
                                let damage = rng.gen_range(1..5);
                                self.health -= damage;
                                self.message = Some(format!("Poison! -{} HP", damage));
                            }
                        }
                    }
                    EntityType::Scroll => {
                        // Random scroll effect
                        let effect = rng.gen_range(0..4);
                        match effect {
                            0 => {
                                // Scroll of identify - reveal all items
                                self.message = Some("Scroll of light! Area revealed.".to_string());
                                self.reveal_area();
                            }
                            1 => {
                                // Scroll of teleport
                                self.message = Some("Scroll of teleport!".to_string());
                                self.teleport_random();
                            }
                            2 => {
                                // Scroll of monster confusion
                                self.message = Some("Scroll of scare monster!".to_string());
                            }
                            _ => {
                                // Scroll of magic mapping
                                for y in 0..BOARD_HEIGHT {
                                    for x in 0..BOARD_WIDTH {
                                        self.explored[y][x] = true;
                                    }
                                }
                                self.message = Some("Scroll of magic mapping!".to_string());
                            }
                        }
                    }
                    EntityType::Weapon => {
                        let bonus = rng.gen_range(1..=3);
                        self.attack += bonus;
                        self.message = Some(format!("Found weapon! +{} ATK", bonus));
                    }
                    EntityType::Armor => {
                        let bonus = rng.gen_range(1..=2);
                        self.defense += bonus;
                        self.message = Some(format!("Found armor! +{} DEF", bonus));
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
            self.message = Some("You escaped the dungeon! You win!".to_string());
        } else {
            self.message = Some(format!("Descending to level {}...", self.dungeon_level));
            self.generate_dungeon();
        }
    }

    pub fn tick(&mut self) {
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
                if self.message.is_none() {
                    self.message = Some("You are starving!".to_string());
                }
                if self.health <= 0 {
                    self.game_over = true;
                    self.message = Some("You starved to death!".to_string());
                }
            } else if self.hunger == 300 {
                self.message = Some("You are getting hungry.".to_string());
            } else if self.hunger == 100 {
                self.message = Some("You are weak from hunger!".to_string());
            }
        }
    }

    fn move_monsters(&mut self) {
        let mut rng = rand::thread_rng();

        for i in 0..self.monsters.len() {
            let monster = &self.monsters[i];

            // Check if player is adjacent
            let dx = self.player_x as i32 - monster.x as i32;
            let dy = self.player_y as i32 - monster.y as i32;

            if dx.abs() <= 1 && dy.abs() <= 1 && (dx != 0 || dy != 0) {
                // Attack player
                let damage = (monster.monster_type.damage() - self.defense).max(1);
                self.health -= damage;
                self.message = Some(format!(
                    "{} attacks! -{} HP",
                    monster.monster_type.char(),
                    damage
                ));

                if self.health <= 0 {
                    self.game_over = true;
                    self.message = Some("You died!".to_string());
                }
            } else if dx.abs() + dy.abs() < 10 {
                // Move toward player
                let move_x = if dx > 0 {
                    1
                } else if dx < 0 {
                    -1
                } else {
                    0
                };
                let move_y = if dy > 0 {
                    1
                } else if dy < 0 {
                    -1
                } else {
                    0
                };

                let new_x = (monster.x as i32 + move_x) as usize;
                let new_y = (monster.y as i32 + move_y) as usize;

                // Check if target is passable and not occupied
                if new_x < BOARD_WIDTH
                    && new_y < BOARD_HEIGHT
                    && self.board[new_y][new_x].is_passable()
                    && !self.monsters.iter().any(|m| m.x == new_x && m.y == new_y)
                    && !(new_x == self.player_x && new_y == self.player_y)
                {
                    self.monsters[i].x = new_x;
                    self.monsters[i].y = new_y;
                }
            } else {
                // Random movement
                let dirs = [(0, 1), (0, -1), (1, 0), (-1, 0)];
                let (ddx, ddy) = dirs[rng.gen_range(0..4)];
                let new_x = (monster.x as i32 + ddx) as usize;
                let new_y = (monster.y as i32 + ddy) as usize;

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

    /// Check if a position is visible (within player's sight range)
    pub fn is_visible(&self, x: usize, y: usize) -> bool {
        let dx = (self.player_x as i32 - x as i32).abs();
        let dy = (self.player_y as i32 - y as i32).abs();
        dx * dx + dy * dy <= 25 // radius of 5
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
            self.message = Some("You found a trap!".to_string());
        } else {
            self.message = Some("You search but find nothing.".to_string());
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
                    self.message = Some("A teleport trap!".to_string());
                    self.teleport_random();
                }
                1 => {
                    // Dart trap
                    let damage = rng.gen_range(1..4);
                    self.health -= damage;
                    self.message = Some(format!("A dart trap! -{} HP", damage));
                }
                2 => {
                    // Bear trap - lose a turn
                    self.message = Some("A bear trap! You're stuck!".to_string());
                }
                _ => {
                    // Pit trap
                    let damage = rng.gen_range(2..6);
                    self.health -= damage;
                    self.message = Some(format!("You fell into a pit! -{} HP", damage));
                }
            }

            if self.health <= 0 {
                self.game_over = true;
                self.message = Some("You died!".to_string());
            }
        }
    }
}
