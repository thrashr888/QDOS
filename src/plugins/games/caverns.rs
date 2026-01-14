//! CAVERNS - Classic Text Adventure Game
//!
//! A Colossal Cave Adventure-style game with room exploration,
//! inventory management, puzzles, and treasure collection.

use crate::plugins::games::platform::{GameEngine, GameEvent, KeyHandleResult};
use crossterm::event::{KeyCode, KeyEvent};

// =============================================================================
// ENUMS
// =============================================================================

/// Direction for movement
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    North,
    South,
    East,
    West,
    Up,
    Down,
}

/// Item identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemId {
    // Tools
    BrassLamp,
    RustyKey,
    GoldenKey,
    OilCan,
    MagicSword,
    Rope,
    FoodRations,
    // Treasures
    GoldNugget,
    DiamondNecklace,
    PlatinumBar,
    RubyCrown,
    EmeraldGoblet,
    SilverChalice,
    JadeFigurine,
    PearlNecklace,
}

/// Creature identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureId {
    Bats,
    Snake,
    Troll,
    Dragon,
    Ghost,
}

/// Puzzle identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PuzzleId {
    RustyDoor,
    DarkPassage,
    TrollBridge,
    LockedChest,
    DragonLair,
}

/// Current UI view
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CavernsView {
    #[default]
    Playing,
    Inventory,
    Examining,
    ItemSelect,
    Encounter,
    GameOver,
    Victory,
}

/// Encounter action choices
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncounterAction {
    Attack,
    UseItem(usize),
    Flee,
}

// =============================================================================
// STATIC DATA STRUCTURES
// =============================================================================

/// Room exit connections
#[derive(Debug, Clone, Copy)]
pub struct Exits {
    pub north: Option<usize>,
    pub south: Option<usize>,
    pub east: Option<usize>,
    pub west: Option<usize>,
    pub up: Option<usize>,
    pub down: Option<usize>,
}

impl Exits {
    pub const fn none() -> Self {
        Self {
            north: None,
            south: None,
            east: None,
            west: None,
            up: None,
            down: None,
        }
    }
}

/// Room definition
#[derive(Debug, Clone)]
pub struct Room {
    pub id: usize,
    pub name: &'static str,
    pub description: &'static str,
    pub exits: Exits,
    pub initial_items: &'static [ItemId],
    pub creature: Option<CreatureId>,
    pub puzzle: Option<PuzzleId>,
    pub is_dark: bool,
}

/// Item definition
#[derive(Debug, Clone)]
pub struct ItemDef {
    pub id: ItemId,
    pub name: &'static str,
    pub description: &'static str,
    pub is_treasure: bool,
    pub points: u32,
    pub can_use: bool,
}

/// Creature definition
#[derive(Debug, Clone)]
pub struct CreatureDef {
    pub id: CreatureId,
    pub name: &'static str,
    pub description: &'static str,
    pub is_hostile: bool,
    pub defeat_with: Option<ItemId>,
    pub flee_with: Option<ItemId>,
    pub blocks_passage: bool,
    pub hint: Option<&'static str>,
}

/// Puzzle definition
#[derive(Debug, Clone)]
pub struct PuzzleDef {
    pub id: PuzzleId,
    pub name: &'static str,
    pub unsolved_msg: &'static str,
    pub solved_msg: &'static str,
    pub required_item: Option<ItemId>,
    pub requires_lamp_lit: bool,
    pub consumes_item: bool,
    pub points: u32,
}

// =============================================================================
// ITEM DEFINITIONS
// =============================================================================

pub const ITEMS: &[ItemDef] = &[
    // Tools
    ItemDef {
        id: ItemId::BrassLamp,
        name: "Brass Lamp",
        description: "An old brass oil lamp. It looks like it still has fuel.",
        is_treasure: false,
        points: 0,
        can_use: true,
    },
    ItemDef {
        id: ItemId::RustyKey,
        name: "Rusty Key",
        description: "A rusty iron key. It might open an old lock.",
        is_treasure: false,
        points: 0,
        can_use: false,
    },
    ItemDef {
        id: ItemId::GoldenKey,
        name: "Golden Key",
        description: "A gleaming golden key with intricate engravings.",
        is_treasure: false,
        points: 0,
        can_use: false,
    },
    ItemDef {
        id: ItemId::OilCan,
        name: "Oil Can",
        description: "A small can of machine oil. Good for rusty hinges.",
        is_treasure: false,
        points: 0,
        can_use: true,
    },
    ItemDef {
        id: ItemId::MagicSword,
        name: "Magic Sword",
        description: "An ancient sword that glows with a faint blue light.",
        is_treasure: false,
        points: 0,
        can_use: true,
    },
    ItemDef {
        id: ItemId::Rope,
        name: "Rope",
        description: "A sturdy hemp rope, about 50 feet long.",
        is_treasure: false,
        points: 0,
        can_use: true,
    },
    ItemDef {
        id: ItemId::FoodRations,
        name: "Food Rations",
        description: "Dried meat and hardtack. Not tasty, but sustaining.",
        is_treasure: false,
        points: 0,
        can_use: true,
    },
    // Treasures
    ItemDef {
        id: ItemId::GoldNugget,
        name: "Gold Nugget",
        description: "A heavy nugget of pure gold, glittering in the light.",
        is_treasure: true,
        points: 200,
        can_use: false,
    },
    ItemDef {
        id: ItemId::DiamondNecklace,
        name: "Diamond Necklace",
        description: "A delicate necklace set with brilliant diamonds.",
        is_treasure: true,
        points: 350,
        can_use: false,
    },
    ItemDef {
        id: ItemId::PlatinumBar,
        name: "Platinum Bar",
        description: "A bar of pure platinum, incredibly dense.",
        is_treasure: true,
        points: 300,
        can_use: false,
    },
    ItemDef {
        id: ItemId::RubyCrown,
        name: "Ruby Crown",
        description: "An ornate crown set with blood-red rubies.",
        is_treasure: true,
        points: 500,
        can_use: false,
    },
    ItemDef {
        id: ItemId::EmeraldGoblet,
        name: "Emerald Goblet",
        description: "A golden goblet encrusted with emeralds.",
        is_treasure: true,
        points: 400,
        can_use: false,
    },
    ItemDef {
        id: ItemId::SilverChalice,
        name: "Silver Chalice",
        description: "An ancient silver chalice with mystical runes.",
        is_treasure: true,
        points: 250,
        can_use: false,
    },
    ItemDef {
        id: ItemId::JadeFigurine,
        name: "Jade Figurine",
        description: "A carved jade figurine of a dragon.",
        is_treasure: true,
        points: 300,
        can_use: false,
    },
    ItemDef {
        id: ItemId::PearlNecklace,
        name: "Pearl Necklace",
        description: "A strand of perfect luminous pearls.",
        is_treasure: true,
        points: 200,
        can_use: false,
    },
];

// =============================================================================
// CREATURE DEFINITIONS
// =============================================================================

pub const CREATURES: &[CreatureDef] = &[
    CreatureDef {
        id: CreatureId::Bats,
        name: "Bats",
        description: "A swarm of bats circles overhead, their chittering echoing off the walls.",
        is_hostile: false,
        defeat_with: None,
        flee_with: None,
        blocks_passage: false,
        hint: None,
    },
    CreatureDef {
        id: CreatureId::Snake,
        name: "Giant Snake",
        description: "A massive snake coils menacingly, blocking your path. Its scales glisten in the dim light.",
        is_hostile: true,
        defeat_with: Some(ItemId::MagicSword),
        flee_with: Some(ItemId::BrassLamp),
        blocks_passage: true,
        hint: None,
    },
    CreatureDef {
        id: CreatureId::Troll,
        name: "Cave Troll",
        description: "A hulking troll guards the bridge, demanding payment to cross. \"TOLL!\" it grunts.",
        is_hostile: true,
        defeat_with: Some(ItemId::MagicSword),
        flee_with: Some(ItemId::FoodRations),
        blocks_passage: true,
        hint: None,
    },
    CreatureDef {
        id: CreatureId::Dragon,
        name: "Ancient Dragon",
        description: "An ancient dragon lies upon a mountain of treasure, smoke curling from its nostrils. It watches you with gleaming eyes.",
        is_hostile: true,
        defeat_with: Some(ItemId::MagicSword),
        flee_with: None,
        blocks_passage: true,
        hint: None,
    },
    CreatureDef {
        id: CreatureId::Ghost,
        name: "Spectral Guide",
        description: "A spectral figure drifts through the air, beckoning you closer.",
        is_hostile: false,
        defeat_with: None,
        flee_with: None,
        blocks_passage: false,
        hint: Some("The dragon fears only the blade that glows with ancient light..."),
    },
];

// =============================================================================
// PUZZLE DEFINITIONS
// =============================================================================

pub const PUZZLES: &[PuzzleDef] = &[
    PuzzleDef {
        id: PuzzleId::RustyDoor,
        name: "Rusty Door",
        unsolved_msg: "A heavy iron door blocks your way, its hinges rusted shut.",
        solved_msg: "The oiled hinges swing smoothly. The door opens!",
        required_item: Some(ItemId::OilCan),
        requires_lamp_lit: false,
        consumes_item: true,
        points: 100,
    },
    PuzzleDef {
        id: PuzzleId::DarkPassage,
        name: "Dark Passage",
        unsolved_msg: "The passage ahead is pitch black. You cannot proceed without light.",
        solved_msg: "Your lamp illuminates a narrow passage ahead.",
        required_item: None,
        requires_lamp_lit: true,
        consumes_item: false,
        points: 50,
    },
    PuzzleDef {
        id: PuzzleId::TrollBridge,
        name: "Troll Bridge",
        unsolved_msg: "The troll blocks the bridge, demanding payment.",
        solved_msg: "The troll steps aside, allowing you to pass.",
        required_item: Some(ItemId::FoodRations),
        requires_lamp_lit: false,
        consumes_item: true,
        points: 75,
    },
    PuzzleDef {
        id: PuzzleId::LockedChest,
        name: "Locked Chest",
        unsolved_msg: "An ornate chest sits here, firmly locked with a golden lock.",
        solved_msg: "The chest clicks open, revealing treasures within!",
        required_item: Some(ItemId::GoldenKey),
        requires_lamp_lit: false,
        consumes_item: true,
        points: 150,
    },
    PuzzleDef {
        id: PuzzleId::DragonLair,
        name: "Dragon's Lair",
        unsolved_msg: "The dragon blocks your path to the treasure. You'll need a powerful weapon.",
        solved_msg: "With a mighty swing, you vanquish the dragon!",
        required_item: Some(ItemId::MagicSword),
        requires_lamp_lit: false,
        consumes_item: false,
        points: 200,
    },
];

// =============================================================================
// ROOM DEFINITIONS (20 ROOMS)
// =============================================================================

pub const ROOMS: &[Room] = &[
    // 0: Cave Entrance (Starting room, treasure deposit point)
    Room {
        id: 0,
        name: "Cave Entrance",
        description: "You stand at the mouth of a vast cavern. Sunlight streams in behind you, illuminating ancient stone walls covered in moss. A cold draft flows from the darkness ahead. This is where adventurers deposit their treasures.",
        exits: Exits { north: Some(1), south: None, east: None, west: None, up: None, down: None },
        initial_items: &[ItemId::BrassLamp],
        creature: None,
        puzzle: None,
        is_dark: false,
    },
    // 1: Main Hall
    Room {
        id: 1,
        name: "Main Hall",
        description: "A massive natural cavern stretches before you. Stalactites hang from the ceiling like stone daggers. Passages lead in multiple directions.",
        exits: Exits { north: Some(2), south: Some(0), east: Some(5), west: Some(3), up: None, down: Some(10) },
        initial_items: &[],
        creature: Some(CreatureId::Bats),
        puzzle: None,
        is_dark: true,
    },
    // 2: Stalactite Gallery
    Room {
        id: 2,
        name: "Stalactite Gallery",
        description: "Thousands of delicate stalactites cover the ceiling, dripping mineral-rich water into shallow pools below. The sound of droplets echoes peacefully.",
        exits: Exits { north: Some(4), south: Some(1), east: None, west: None, up: None, down: None },
        initial_items: &[ItemId::GoldNugget],
        creature: None,
        puzzle: None,
        is_dark: true,
    },
    // 3: Dusty Passage
    Room {
        id: 3,
        name: "Dusty Passage",
        description: "A narrow corridor thick with ancient dust. Footprints from long-ago explorers can still be seen on the floor.",
        exits: Exits { north: None, south: None, east: Some(1), west: Some(6), up: None, down: None },
        initial_items: &[ItemId::FoodRations],
        creature: None,
        puzzle: None,
        is_dark: true,
    },
    // 4: Crystal Chamber
    Room {
        id: 4,
        name: "Crystal Chamber",
        description: "The walls sparkle with countless crystals of every color. Light from your lamp creates a dazzling rainbow display that takes your breath away.",
        exits: Exits { north: None, south: Some(2), east: Some(7), west: None, up: None, down: None },
        initial_items: &[ItemId::DiamondNecklace],
        creature: None,
        puzzle: None,
        is_dark: true,
    },
    // 5: Fork in the Road
    Room {
        id: 5,
        name: "Fork in the Road",
        description: "The passage splits here. To the north, you hear the sound of rushing water. To the east, a foul smell wafts from the darkness.",
        exits: Exits { north: Some(8), south: None, east: Some(9), west: Some(1), up: None, down: None },
        initial_items: &[],
        creature: None,
        puzzle: None,
        is_dark: true,
    },
    // 6: Rusty Door Room
    Room {
        id: 6,
        name: "Iron Door",
        description: "A massive iron door is set into the western wall, covered in rust and age. Beyond it, you sense something valuable.",
        exits: Exits { north: None, south: None, east: Some(3), west: Some(11), up: None, down: None },
        initial_items: &[ItemId::OilCan],
        creature: None,
        puzzle: Some(PuzzleId::RustyDoor),
        is_dark: true,
    },
    // 7: Echo Chamber
    Room {
        id: 7,
        name: "Echo Chamber",
        description: "Every sound here echoes endlessly, creating a disorienting symphony. The acoustics are remarkable.",
        exits: Exits { north: None, south: None, east: None, west: Some(4), up: Some(12), down: None },
        initial_items: &[ItemId::Rope],
        creature: None,
        puzzle: None,
        is_dark: true,
    },
    // 8: Underground Lake
    Room {
        id: 8,
        name: "Underground Lake",
        description: "A vast underground lake stretches before you, its waters impossibly clear. Blind fish dart beneath the surface. The far shore is lost in darkness.",
        exits: Exits { north: None, south: Some(5), east: None, west: None, up: None, down: None },
        initial_items: &[ItemId::PearlNecklace],
        creature: None,
        puzzle: None,
        is_dark: true,
    },
    // 9: Snake Pit
    Room {
        id: 9,
        name: "Snake Pit",
        description: "A circular chamber with bones scattered across the floor. Something large has made this its lair.",
        exits: Exits { north: None, south: None, east: Some(13), west: Some(5), up: None, down: None },
        initial_items: &[ItemId::RustyKey],
        creature: Some(CreatureId::Snake),
        puzzle: None,
        is_dark: true,
    },
    // 10: Deep Cavern
    Room {
        id: 10,
        name: "Deep Cavern",
        description: "The ceiling here is lost in darkness above. Strange minerals in the walls glow faintly with their own light.",
        exits: Exits { north: Some(14), south: None, east: None, west: None, up: Some(1), down: Some(15) },
        initial_items: &[ItemId::PlatinumBar],
        creature: None,
        puzzle: None,
        is_dark: true,
    },
    // 11: Hidden Treasury
    Room {
        id: 11,
        name: "Hidden Treasury",
        description: "Beyond the rusty door lies a small chamber filled with ancient chests and coin-scattered floors.",
        exits: Exits { north: None, south: None, east: Some(6), west: None, up: None, down: None },
        initial_items: &[ItemId::SilverChalice, ItemId::GoldenKey],
        creature: None,
        puzzle: None,
        is_dark: true,
    },
    // 12: High Ledge
    Room {
        id: 12,
        name: "High Ledge",
        description: "A narrow ledge high above the Echo Chamber. The view is vertiginous. Ancient carvings cover the walls.",
        exits: Exits { north: None, south: None, east: Some(16), west: None, up: None, down: Some(7) },
        initial_items: &[ItemId::JadeFigurine],
        creature: Some(CreatureId::Ghost),
        puzzle: None,
        is_dark: true,
    },
    // 13: Dark Passage
    Room {
        id: 13,
        name: "Dark Passage",
        description: "The darkness here is absolute, oppressive. Even your lamp struggles against it.",
        exits: Exits { north: None, south: None, east: Some(17), west: Some(9), up: None, down: None },
        initial_items: &[],
        creature: None,
        puzzle: Some(PuzzleId::DarkPassage),
        is_dark: true,
    },
    // 14: Troll Bridge
    Room {
        id: 14,
        name: "Stone Bridge",
        description: "A narrow stone bridge spans a seemingly bottomless chasm. Something large lurks here.",
        exits: Exits { north: Some(18), south: Some(10), east: None, west: None, up: None, down: None },
        initial_items: &[],
        creature: Some(CreatureId::Troll),
        puzzle: Some(PuzzleId::TrollBridge),
        is_dark: true,
    },
    // 15: Mushroom Forest
    Room {
        id: 15,
        name: "Mushroom Forest",
        description: "Giant luminescent mushrooms grow here, casting an eerie blue glow. The air is thick with spores.",
        exits: Exits { north: None, south: None, east: None, west: None, up: Some(10), down: Some(19) },
        initial_items: &[],
        creature: None,
        puzzle: None,
        is_dark: false, // Mushrooms provide light
    },
    // 16: Ancient Ruins
    Room {
        id: 16,
        name: "Ancient Ruins",
        description: "Crumbling stone pillars and weathered statues hint at a civilization that once thrived here.",
        exits: Exits { north: None, south: None, east: None, west: Some(12), up: None, down: Some(17) },
        initial_items: &[ItemId::MagicSword],
        creature: None,
        puzzle: None,
        is_dark: true,
    },
    // 17: Treasure Vault
    Room {
        id: 17,
        name: "Treasure Vault",
        description: "An ancient vault with a massive locked chest in the center. The lock gleams with gold.",
        exits: Exits { north: None, south: None, east: None, west: Some(13), up: Some(16), down: None },
        initial_items: &[ItemId::EmeraldGoblet],
        creature: None,
        puzzle: Some(PuzzleId::LockedChest),
        is_dark: true,
    },
    // 18: Dragon's Lair
    Room {
        id: 18,
        name: "Dragon's Lair",
        description: "Heat radiates from this massive chamber. Piles of gold and bones litter the floor. The dragon's hoard lies beyond.",
        exits: Exits { north: None, south: Some(14), east: None, west: None, up: None, down: None },
        initial_items: &[ItemId::RubyCrown],
        creature: Some(CreatureId::Dragon),
        puzzle: Some(PuzzleId::DragonLair),
        is_dark: false, // Dragon's fire provides light
    },
    // 19: Cave Exit
    Room {
        id: 19,
        name: "Cave Exit",
        description: "A shaft of daylight pierces the gloom! You've found another exit from the caverns. The surface awaits above.",
        exits: Exits { north: None, south: None, east: None, west: None, up: Some(15), down: None },
        initial_items: &[],
        creature: None,
        puzzle: None,
        is_dark: false,
    },
];

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

pub fn get_item_def(id: ItemId) -> &'static ItemDef {
    ITEMS.iter().find(|i| i.id == id).expect("Item not found")
}

pub fn get_creature_def(id: CreatureId) -> &'static CreatureDef {
    CREATURES
        .iter()
        .find(|c| c.id == id)
        .expect("Creature not found")
}

pub fn get_puzzle_def(id: PuzzleId) -> &'static PuzzleDef {
    PUZZLES
        .iter()
        .find(|p| p.id == id)
        .expect("Puzzle not found")
}

pub fn get_room(id: usize) -> &'static Room {
    &ROOMS[id]
}

// =============================================================================
// GAME STATE
// =============================================================================

pub struct CavernsState {
    // Player state
    pub current_room: usize,
    pub previous_room: usize,
    pub inventory: Vec<ItemId>,
    pub max_inventory: usize,
    pub lamp_fuel: u32,
    pub lamp_lit: bool,

    // World state (mutable copies)
    pub room_items: Vec<Vec<ItemId>>,
    pub solved_puzzles: Vec<PuzzleId>,
    pub defeated_creatures: Vec<CreatureId>,
    pub visited_rooms: Vec<bool>,

    // Score tracking
    pub treasures_deposited: Vec<ItemId>,
    pub rooms_discovered: u32,
    pub puzzles_solved: u32,

    // UI state
    pub view: CavernsView,
    pub messages: Vec<String>,
    pub selected_item: usize,
    pub encounter_creature: Option<CreatureId>,
    pub examine_target: Option<ExamineTarget>,

    // Game state
    pub turns: u32,
    pub game_over: bool,
    pub game_won: bool,

    // Events
    pending_events: Vec<GameEvent>,
}

#[derive(Debug, Clone)]
pub enum ExamineTarget {
    Room,
    Item(ItemId),
}

impl Default for CavernsState {
    fn default() -> Self {
        Self::new()
    }
}

impl CavernsState {
    pub fn new() -> Self {
        // Initialize room items from static definitions
        let room_items: Vec<Vec<ItemId>> = ROOMS.iter().map(|r| r.initial_items.to_vec()).collect();

        let visited_rooms = vec![false; ROOMS.len()];

        let mut state = Self {
            current_room: 0,
            previous_room: 0,
            inventory: Vec::new(),
            max_inventory: 6,
            lamp_fuel: 100,
            lamp_lit: false,

            room_items,
            solved_puzzles: Vec::new(),
            defeated_creatures: Vec::new(),
            visited_rooms,

            treasures_deposited: Vec::new(),
            rooms_discovered: 0,
            puzzles_solved: 0,

            view: CavernsView::Playing,
            messages: Vec::new(),
            selected_item: 0,
            encounter_creature: None,
            examine_target: None,

            turns: 0,
            game_over: false,
            game_won: false,

            pending_events: Vec::new(),
        };

        // Mark entrance as visited
        state.visited_rooms[0] = true;
        state.rooms_discovered = 1;
        state.add_message("Welcome to CAVERNS! Find all 8 treasures and return them here.");

        state
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    // -------------------------------------------------------------------------
    // MESSAGE HANDLING
    // -------------------------------------------------------------------------

    pub fn add_message(&mut self, msg: &str) {
        self.messages.push(msg.to_string());
        // Keep only last 5 messages
        if self.messages.len() > 5 {
            self.messages.remove(0);
        }
    }

    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }

    // -------------------------------------------------------------------------
    // MOVEMENT
    // -------------------------------------------------------------------------

    pub fn try_move(&mut self, direction: Direction) -> bool {
        let room = get_room(self.current_room);

        // Check if room is dark and lamp is not lit
        if room.is_dark && !self.lamp_lit {
            self.add_message("It's too dark to see! Light your lamp first. (Press Enter)");
            return false;
        }

        // Get the exit for this direction
        let next_room = match direction {
            Direction::North => room.exits.north,
            Direction::South => room.exits.south,
            Direction::East => room.exits.east,
            Direction::West => room.exits.west,
            Direction::Up => room.exits.up,
            Direction::Down => room.exits.down,
        };

        match next_room {
            Some(room_id) => {
                // Check for blocking puzzles
                let next = get_room(room_id);
                if let Some(puzzle_id) = next.puzzle {
                    if !self.solved_puzzles.contains(&puzzle_id) {
                        let puzzle = get_puzzle_def(puzzle_id);
                        // Check if we can auto-solve
                        if self.can_solve_puzzle(puzzle_id) {
                            self.solve_puzzle(puzzle_id);
                        } else {
                            self.add_message(puzzle.unsolved_msg);
                            return false;
                        }
                    }
                }

                // Check for blocking creatures in next room
                if let Some(creature_id) = next.creature {
                    if !self.defeated_creatures.contains(&creature_id) {
                        let creature = get_creature_def(creature_id);
                        if creature.blocks_passage {
                            // Will trigger encounter when entering
                        }
                    }
                }

                self.previous_room = self.current_room;
                self.current_room = room_id;
                self.on_enter_room();
                true
            }
            None => {
                self.add_message("You can't go that way.");
                false
            }
        }
    }

    fn on_enter_room(&mut self) {
        let room = get_room(self.current_room);

        // Mark as visited
        if !self.visited_rooms[self.current_room] {
            self.visited_rooms[self.current_room] = true;
            self.rooms_discovered += 1;
            self.pending_events.push(GameEvent::Custom {
                key: "room_discovered".to_string(),
                value: self.rooms_discovered as u64,
            });
        }

        // Check for creature encounters
        if let Some(creature_id) = room.creature {
            if !self.defeated_creatures.contains(&creature_id) {
                let creature = get_creature_def(creature_id);
                if creature.is_hostile {
                    self.encounter_creature = Some(creature_id);
                    self.view = CavernsView::Encounter;
                } else {
                    // Non-hostile creatures give hints
                    if let Some(hint) = creature.hint {
                        self.add_message(&format!("The {} speaks: \"{}\"", creature.name, hint));
                    }
                    self.defeated_creatures.push(creature_id);
                }
            }
        }

        // Consume lamp fuel if lit
        if self.lamp_lit {
            self.lamp_fuel = self.lamp_fuel.saturating_sub(1);
            if self.lamp_fuel == 0 {
                self.lamp_lit = false;
                self.add_message("Your lamp sputters and goes out!");
            } else if self.lamp_fuel == 20 {
                self.add_message("Your lamp is running low on fuel...");
            }
        }

        self.turns += 1;
    }

    // -------------------------------------------------------------------------
    // INVENTORY
    // -------------------------------------------------------------------------

    pub fn take_item(&mut self, item: ItemId) -> bool {
        if self.inventory.len() >= self.max_inventory {
            self.add_message("Your hands are full! Drop something first.");
            return false;
        }

        let room_items = &mut self.room_items[self.current_room];
        if let Some(pos) = room_items.iter().position(|i| *i == item) {
            room_items.remove(pos);
            self.inventory.push(item);
            let item_def = get_item_def(item);
            self.add_message(&format!("Taken: {}", item_def.name));
            true
        } else {
            false
        }
    }

    pub fn drop_item(&mut self, item: ItemId) {
        if let Some(pos) = self.inventory.iter().position(|i| *i == item) {
            self.inventory.remove(pos);

            // If at entrance and it's a treasure, deposit it
            if self.current_room == 0 && get_item_def(item).is_treasure {
                self.treasures_deposited.push(item);
                let item_def = get_item_def(item);
                self.add_message(&format!(
                    "Deposited {} in the treasure pile! +{} points",
                    item_def.name, item_def.points
                ));
                self.check_victory();
            } else {
                self.room_items[self.current_room].push(item);
                self.add_message(&format!("Dropped: {}", get_item_def(item).name));
            }
        }
    }

    pub fn use_item(&mut self, item: ItemId) {
        match item {
            ItemId::BrassLamp => {
                if self.lamp_fuel > 0 {
                    self.lamp_lit = !self.lamp_lit;
                    if self.lamp_lit {
                        self.add_message("The lamp flickers to life, casting a warm glow.");
                    } else {
                        self.add_message("You extinguish the lamp.");
                    }
                } else {
                    self.add_message("The lamp is out of fuel.");
                }
            }
            ItemId::FoodRations => {
                self.add_message("You eat some rations. Feeling better!");
                self.remove_from_inventory(item);
            }
            _ => {
                self.add_message("You can't use that here.");
            }
        }
    }

    fn remove_from_inventory(&mut self, item: ItemId) {
        if let Some(pos) = self.inventory.iter().position(|i| *i == item) {
            self.inventory.remove(pos);
        }
    }

    pub fn get_room_items(&self) -> &[ItemId] {
        &self.room_items[self.current_room]
    }

    // -------------------------------------------------------------------------
    // PUZZLES
    // -------------------------------------------------------------------------

    fn can_solve_puzzle(&self, puzzle_id: PuzzleId) -> bool {
        let puzzle = get_puzzle_def(puzzle_id);

        if puzzle.requires_lamp_lit {
            return self.lamp_lit;
        }

        if let Some(required) = puzzle.required_item {
            return self.inventory.contains(&required);
        }

        true
    }

    fn solve_puzzle(&mut self, puzzle_id: PuzzleId) {
        let puzzle = get_puzzle_def(puzzle_id);
        self.solved_puzzles.push(puzzle_id);
        self.puzzles_solved += 1;
        self.add_message(puzzle.solved_msg);
        self.add_message(&format!("+{} points!", puzzle.points));

        if puzzle.consumes_item {
            if let Some(item) = puzzle.required_item {
                self.remove_from_inventory(item);
            }
        }

        // Also mark creature as defeated if it's a creature puzzle
        if puzzle_id == PuzzleId::TrollBridge {
            self.defeated_creatures.push(CreatureId::Troll);
        } else if puzzle_id == PuzzleId::DragonLair {
            self.defeated_creatures.push(CreatureId::Dragon);
        }

        self.pending_events.push(GameEvent::Custom {
            key: "puzzle_solved".to_string(),
            value: self.puzzles_solved as u64,
        });
    }

    // -------------------------------------------------------------------------
    // ENCOUNTERS
    // -------------------------------------------------------------------------

    pub fn handle_encounter(&mut self, action: EncounterAction) {
        let creature_id = match self.encounter_creature {
            Some(c) => c,
            None => return,
        };
        let creature = get_creature_def(creature_id);

        match action {
            EncounterAction::Attack => {
                if let Some(weapon) = creature.defeat_with {
                    if self.inventory.contains(&weapon) {
                        self.defeated_creatures.push(creature_id);
                        self.encounter_creature = None;
                        self.view = CavernsView::Playing;
                        self.add_message(&format!("You defeated the {}!", creature.name));
                    } else {
                        self.add_message("You have nothing to fight with!");
                    }
                } else {
                    self.add_message("This creature cannot be harmed.");
                }
            }
            EncounterAction::UseItem(idx) => {
                if idx < self.inventory.len() {
                    let item = self.inventory[idx];
                    if creature.flee_with == Some(item) {
                        self.defeated_creatures.push(creature_id);
                        self.encounter_creature = None;
                        self.view = CavernsView::Playing;
                        let item_def = get_item_def(item);
                        self.add_message(&format!(
                            "The {} flees from the {}!",
                            creature.name, item_def.name
                        ));
                        // Some items are consumed
                        if item == ItemId::FoodRations {
                            self.remove_from_inventory(item);
                        }
                    } else {
                        self.add_message("That doesn't seem to help here.");
                    }
                }
            }
            EncounterAction::Flee => {
                if creature.blocks_passage {
                    // Must retreat
                    self.current_room = self.previous_room;
                    self.add_message("You retreat from the danger.");
                }
                self.encounter_creature = None;
                self.view = CavernsView::Playing;
            }
        }
    }

    // -------------------------------------------------------------------------
    // SCORING & VICTORY
    // -------------------------------------------------------------------------

    pub fn calculate_score(&self) -> u32 {
        let mut score = 0;

        // Treasure points
        for treasure in &self.treasures_deposited {
            score += get_item_def(*treasure).points;
        }

        // Room discovery
        score += self.rooms_discovered * 10;

        // Puzzle completion
        for puzzle in &self.solved_puzzles {
            score += get_puzzle_def(*puzzle).points;
        }

        // Victory bonus
        if self.game_won {
            score += 500;
        }

        score
    }

    fn check_victory(&mut self) {
        // Win condition: deposited all 8 treasures
        if self.treasures_deposited.len() >= 8 {
            self.game_won = true;
            self.game_over = true;
            self.view = CavernsView::Victory;
            self.add_message("You've collected all the treasures! YOU WIN!");
            self.pending_events.push(GameEvent::GameEnded { won: true });
        }
    }

    // -------------------------------------------------------------------------
    // EXAMINE
    // -------------------------------------------------------------------------

    pub fn examine_room(&mut self) {
        self.examine_target = Some(ExamineTarget::Room);
        self.view = CavernsView::Examining;
    }

    pub fn examine_item(&mut self, item: ItemId) {
        self.examine_target = Some(ExamineTarget::Item(item));
        self.view = CavernsView::Examining;
    }

    // -------------------------------------------------------------------------
    // KEY HANDLING
    // -------------------------------------------------------------------------

    fn handle_playing_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            // Movement
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Up => {
                self.try_move(Direction::North);
                KeyHandleResult::Handled
            }
            KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Down => {
                self.try_move(Direction::South);
                KeyHandleResult::Handled
            }
            KeyCode::Char('e') | KeyCode::Char('E') | KeyCode::Right => {
                self.try_move(Direction::East);
                KeyHandleResult::Handled
            }
            KeyCode::Char('w') | KeyCode::Char('W') | KeyCode::Left => {
                self.try_move(Direction::West);
                KeyHandleResult::Handled
            }
            KeyCode::Char('u') | KeyCode::Char('U') => {
                self.try_move(Direction::Up);
                KeyHandleResult::Handled
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                self.try_move(Direction::Down);
                KeyHandleResult::Handled
            }
            // Inventory
            KeyCode::Char('i') | KeyCode::Char('I') => {
                self.selected_item = 0;
                self.view = CavernsView::Inventory;
                KeyHandleResult::Handled
            }
            // Get item
            KeyCode::Char('g') | KeyCode::Char('G') => {
                let items = self.get_room_items().to_vec();
                if items.is_empty() {
                    self.add_message("There's nothing here to take.");
                } else if items.len() == 1 {
                    self.take_item(items[0]);
                } else {
                    self.selected_item = 0;
                    self.view = CavernsView::ItemSelect;
                }
                KeyHandleResult::Handled
            }
            // Look
            KeyCode::Char('l') | KeyCode::Char('L') => {
                self.clear_messages();
                let room = get_room(self.current_room);
                self.add_message(room.description);
                KeyHandleResult::Handled
            }
            // Examine
            KeyCode::Char('x') | KeyCode::Char('X') => {
                self.examine_room();
                KeyHandleResult::Handled
            }
            // Use lamp (Enter toggles lamp if you have it)
            KeyCode::Enter => {
                if self.inventory.contains(&ItemId::BrassLamp) {
                    self.use_item(ItemId::BrassLamp);
                } else {
                    self.add_message("Press I for inventory, G to get items.");
                }
                KeyHandleResult::Handled
            }
            // Quit
            KeyCode::Esc => KeyHandleResult::RequestQuit,
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_inventory_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_item > 0 {
                    self.selected_item -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_item < self.inventory.len().saturating_sub(1) {
                    self.selected_item += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                // Use selected item
                if let Some(&item) = self.inventory.get(self.selected_item) {
                    let item_def = get_item_def(item);
                    if item_def.can_use {
                        self.use_item(item);
                    } else {
                        self.add_message("You can't use that.");
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                // Drop selected item
                if let Some(&item) = self.inventory.get(self.selected_item) {
                    self.drop_item(item);
                    if self.selected_item > 0 {
                        self.selected_item -= 1;
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('x') | KeyCode::Char('X') => {
                // Examine selected item
                if let Some(&item) = self.inventory.get(self.selected_item) {
                    self.examine_item(item);
                }
                KeyHandleResult::Handled
            }
            KeyCode::Esc => {
                self.view = CavernsView::Playing;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_item_select_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        let items = self.get_room_items().to_vec();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_item > 0 {
                    self.selected_item -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_item < items.len().saturating_sub(1) {
                    self.selected_item += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                if let Some(&item) = items.get(self.selected_item) {
                    self.take_item(item);
                }
                self.view = CavernsView::Playing;
                KeyHandleResult::Handled
            }
            KeyCode::Esc => {
                self.view = CavernsView::Playing;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_encounter_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.handle_encounter(EncounterAction::Attack);
                KeyHandleResult::Handled
            }
            KeyCode::Char('f') | KeyCode::Char('F') | KeyCode::Esc => {
                self.handle_encounter(EncounterAction::Flee);
                KeyHandleResult::Handled
            }
            KeyCode::Char('1'..='9') => {
                let idx = (key.code.to_string().parse::<usize>().unwrap_or(1)) - 1;
                self.handle_encounter(EncounterAction::UseItem(idx));
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_examine_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.examine_target = None;
                self.view = CavernsView::Playing;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_game_over_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Enter => {
                self.reset();
                KeyHandleResult::Handled
            }
            KeyCode::Esc => KeyHandleResult::RequestQuit,
            _ => KeyHandleResult::Handled,
        }
    }
}

// =============================================================================
// GAME ENGINE IMPLEMENTATION
// =============================================================================

impl GameEngine for CavernsState {
    fn tick(&mut self) {
        // Turn-based game, no real-time updates needed
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match self.view {
            CavernsView::Playing => self.handle_playing_key(key),
            CavernsView::Inventory => self.handle_inventory_key(key),
            CavernsView::ItemSelect => self.handle_item_select_key(key),
            CavernsView::Encounter => self.handle_encounter_key(key),
            CavernsView::Examining => self.handle_examine_key(key),
            CavernsView::GameOver | CavernsView::Victory => self.handle_game_over_key(key),
        }
    }

    fn get_score(&self) -> u32 {
        self.calculate_score()
    }

    fn is_game_over(&self) -> bool {
        self.game_over
    }

    fn is_game_won(&self) -> bool {
        self.game_won
    }

    fn get_level(&self) -> Option<u32> {
        Some(self.rooms_discovered)
    }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.pending_events)
    }

    fn get_stat(&self, key: &str) -> Option<u64> {
        match key {
            "rooms_discovered" => Some(self.rooms_discovered as u64),
            "puzzles_solved" => Some(self.puzzles_solved as u64),
            "treasures" => Some(self.treasures_deposited.len() as u64),
            "turns" => Some(self.turns as u64),
            _ => None,
        }
    }
}
