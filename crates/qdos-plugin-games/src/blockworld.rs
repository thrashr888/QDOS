//! BLOCKWORLD - Terraria-style 2D Mining Adventure
//!
//! A 2D side-scrolling mining, building, and survival game.
//! Mine blocks, craft tools, build structures, survive the night!

use super::platform::{GameEngine, GameEvent, KeyHandleResult};
use crossterm::event::{KeyCode, KeyEvent};
use rand::Rng;

// =============================================================================
// CONSTANTS
// =============================================================================

/// World dimensions
pub const WORLD_WIDTH: usize = 200;
pub const WORLD_HEIGHT: usize = 80;

/// Visible area
pub const VIEW_WIDTH: usize = 76;
pub const VIEW_HEIGHT: usize = 18;

/// Sky height (blocks above ground)
pub const SKY_HEIGHT: usize = 20;

/// Day/night cycle
pub const DAY_LENGTH: u32 = 1200; // ticks per day (20 minutes at 1 tick/sec)
pub const DAWN_START: u32 = 0;
pub const DAY_START: u32 = 150;
pub const DUSK_START: u32 = 900;
pub const NIGHT_START: u32 = 1050;

/// Physics
pub const GRAVITY: f32 = 0.5;
pub const JUMP_VELOCITY: f32 = -2.5;
pub const MOVE_SPEED: f32 = 0.5;

/// Mining speeds (ticks to break)
pub const MINE_SPEED_HAND: u32 = 30;
pub const MINE_SPEED_WOOD: u32 = 20;
pub const MINE_SPEED_STONE: u32 = 12;
pub const MINE_SPEED_IRON: u32 = 8;
pub const MINE_SPEED_DIAMOND: u32 = 4;

// =============================================================================
// BLOCK TYPES
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    Air,
    Dirt,
    Grass,
    Stone,
    Bedrock,
    Wood,
    Leaves,
    Coal,
    Iron,
    Gold,
    Diamond,
    Water,
    Sand,
    Planks,
    Cobblestone,
    Torch,
    Workbench,
    Furnace,
    Chest,
}

impl BlockType {
    pub fn char(&self) -> char {
        match self {
            BlockType::Air => ' ',
            BlockType::Dirt => '#',
            BlockType::Grass => '"',
            BlockType::Stone => '%',
            BlockType::Bedrock => '@',
            BlockType::Wood => '|',
            BlockType::Leaves => '*',
            BlockType::Coal => 'C',
            BlockType::Iron => 'I',
            BlockType::Gold => 'G',
            BlockType::Diamond => 'D',
            BlockType::Water => '~',
            BlockType::Sand => '.',
            BlockType::Planks => '=',
            BlockType::Cobblestone => '+',
            BlockType::Torch => 'i',
            BlockType::Workbench => 'W',
            BlockType::Furnace => 'F',
            BlockType::Chest => 'B',
        }
    }

    pub fn is_solid(&self) -> bool {
        !matches!(self, BlockType::Air | BlockType::Water | BlockType::Torch)
    }

    pub fn hardness(&self) -> u32 {
        match self {
            BlockType::Air => 0,
            BlockType::Leaves => 5,
            BlockType::Torch => 1,
            BlockType::Dirt | BlockType::Grass | BlockType::Sand => 10,
            BlockType::Wood | BlockType::Planks => 15,
            BlockType::Cobblestone | BlockType::Stone => 20,
            BlockType::Coal => 25,
            BlockType::Iron | BlockType::Workbench | BlockType::Furnace | BlockType::Chest => 30,
            BlockType::Gold => 35,
            BlockType::Diamond => 40,
            BlockType::Water => 0,
            BlockType::Bedrock => u32::MAX,
        }
    }

    pub fn required_tool(&self) -> Option<ToolType> {
        match self {
            BlockType::Stone
            | BlockType::Cobblestone
            | BlockType::Coal
            | BlockType::Iron
            | BlockType::Gold
            | BlockType::Diamond
            | BlockType::Furnace => Some(ToolType::Pickaxe),
            BlockType::Wood | BlockType::Planks | BlockType::Workbench | BlockType::Chest => {
                Some(ToolType::Axe)
            }
            BlockType::Dirt | BlockType::Grass | BlockType::Sand => Some(ToolType::Shovel),
            _ => None,
        }
    }

    pub fn drops(&self) -> Option<(ItemType, u32)> {
        match self {
            BlockType::Dirt | BlockType::Grass => Some((ItemType::Block(BlockType::Dirt), 1)),
            BlockType::Stone => Some((ItemType::Block(BlockType::Cobblestone), 1)),
            BlockType::Wood => Some((ItemType::Block(BlockType::Wood), 1)),
            BlockType::Leaves => {
                // 10% chance to drop sapling
                None
            }
            BlockType::Coal => Some((ItemType::Coal, 1)),
            BlockType::Iron => Some((ItemType::IronOre, 1)),
            BlockType::Gold => Some((ItemType::GoldOre, 1)),
            BlockType::Diamond => Some((ItemType::Diamond, 1)),
            BlockType::Sand => Some((ItemType::Block(BlockType::Sand), 1)),
            BlockType::Planks => Some((ItemType::Block(BlockType::Planks), 1)),
            BlockType::Cobblestone => Some((ItemType::Block(BlockType::Cobblestone), 1)),
            BlockType::Torch => Some((ItemType::Torch, 1)),
            BlockType::Workbench => Some((ItemType::Block(BlockType::Workbench), 1)),
            BlockType::Furnace => Some((ItemType::Block(BlockType::Furnace), 1)),
            BlockType::Chest => Some((ItemType::Block(BlockType::Chest), 1)),
            _ => None,
        }
    }

    pub fn light_level(&self) -> u8 {
        match self {
            BlockType::Torch => 10,
            _ => 0,
        }
    }
}

// =============================================================================
// ITEMS AND TOOLS
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolType {
    Pickaxe,
    Axe,
    Shovel,
    Sword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolMaterial {
    Wood,
    Stone,
    Iron,
    Gold,
    Diamond,
}

impl ToolMaterial {
    pub fn mining_speed(&self) -> u32 {
        match self {
            ToolMaterial::Wood => MINE_SPEED_WOOD,
            ToolMaterial::Stone => MINE_SPEED_STONE,
            ToolMaterial::Iron => MINE_SPEED_IRON,
            ToolMaterial::Gold => MINE_SPEED_IRON, // Gold is fast but weak
            ToolMaterial::Diamond => MINE_SPEED_DIAMOND,
        }
    }

    pub fn damage(&self) -> i32 {
        match self {
            ToolMaterial::Wood => 4,
            ToolMaterial::Stone => 6,
            ToolMaterial::Iron => 8,
            ToolMaterial::Gold => 5,
            ToolMaterial::Diamond => 12,
        }
    }

    pub fn durability(&self) -> u32 {
        match self {
            ToolMaterial::Wood => 60,
            ToolMaterial::Stone => 132,
            ToolMaterial::Iron => 251,
            ToolMaterial::Gold => 33,
            ToolMaterial::Diamond => 1562,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemType {
    Block(BlockType),
    Tool(ToolType, ToolMaterial),
    Coal,
    IronOre,
    IronBar,
    GoldOre,
    GoldBar,
    Diamond,
    Torch,
    Stick,
    RawMeat,
    CookedMeat,
}

impl ItemType {
    pub fn char(&self) -> char {
        match self {
            ItemType::Block(b) => b.char(),
            ItemType::Tool(ToolType::Pickaxe, _) => 'T',
            ItemType::Tool(ToolType::Axe, _) => 'A',
            ItemType::Tool(ToolType::Shovel, _) => 'S',
            ItemType::Tool(ToolType::Sword, _) => '/',
            ItemType::Coal => 'c',
            ItemType::IronOre => 'o',
            ItemType::IronBar => 'i',
            ItemType::GoldOre => 'g',
            ItemType::GoldBar => '$',
            ItemType::Diamond => 'd',
            ItemType::Torch => 'i',
            ItemType::Stick => '!',
            ItemType::RawMeat => 'm',
            ItemType::CookedMeat => 'M',
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ItemType::Block(BlockType::Dirt) => "Dirt",
            ItemType::Block(BlockType::Stone) => "Stone",
            ItemType::Block(BlockType::Wood) => "Wood",
            ItemType::Block(BlockType::Planks) => "Planks",
            ItemType::Block(BlockType::Cobblestone) => "Cobblestone",
            ItemType::Block(BlockType::Sand) => "Sand",
            ItemType::Block(BlockType::Workbench) => "Workbench",
            ItemType::Block(BlockType::Furnace) => "Furnace",
            ItemType::Block(BlockType::Chest) => "Chest",
            ItemType::Block(_) => "Block",
            ItemType::Tool(ToolType::Pickaxe, m) => match m {
                ToolMaterial::Wood => "Wood Pickaxe",
                ToolMaterial::Stone => "Stone Pickaxe",
                ToolMaterial::Iron => "Iron Pickaxe",
                ToolMaterial::Gold => "Gold Pickaxe",
                ToolMaterial::Diamond => "Diamond Pickaxe",
            },
            ItemType::Tool(ToolType::Axe, m) => match m {
                ToolMaterial::Wood => "Wood Axe",
                ToolMaterial::Stone => "Stone Axe",
                ToolMaterial::Iron => "Iron Axe",
                ToolMaterial::Gold => "Gold Axe",
                ToolMaterial::Diamond => "Diamond Axe",
            },
            ItemType::Tool(ToolType::Shovel, m) => match m {
                ToolMaterial::Wood => "Wood Shovel",
                ToolMaterial::Stone => "Stone Shovel",
                ToolMaterial::Iron => "Iron Shovel",
                ToolMaterial::Gold => "Gold Shovel",
                ToolMaterial::Diamond => "Diamond Shovel",
            },
            ItemType::Tool(ToolType::Sword, m) => match m {
                ToolMaterial::Wood => "Wood Sword",
                ToolMaterial::Stone => "Stone Sword",
                ToolMaterial::Iron => "Iron Sword",
                ToolMaterial::Gold => "Gold Sword",
                ToolMaterial::Diamond => "Diamond Sword",
            },
            ItemType::Coal => "Coal",
            ItemType::IronOre => "Iron Ore",
            ItemType::IronBar => "Iron Bar",
            ItemType::GoldOre => "Gold Ore",
            ItemType::GoldBar => "Gold Bar",
            ItemType::Diamond => "Diamond",
            ItemType::Torch => "Torch",
            ItemType::Stick => "Stick",
            ItemType::RawMeat => "Raw Meat",
            ItemType::CookedMeat => "Cooked Meat",
        }
    }

    pub fn max_stack(&self) -> u32 {
        match self {
            ItemType::Tool(_, _) => 1,
            _ => 99,
        }
    }
}

/// An inventory slot
#[derive(Debug, Clone)]
pub struct InventorySlot {
    pub item: ItemType,
    pub count: u32,
    pub durability: Option<u32>,
}

// =============================================================================
// CREATURES
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureType {
    // Passive
    Pig,
    Cow,
    Chicken,
    // Hostile (night)
    Zombie,
    Skeleton,
    Slime,
    Spider,
}

impl CreatureType {
    pub fn char(&self) -> char {
        match self {
            CreatureType::Pig => 'p',
            CreatureType::Cow => 'c',
            CreatureType::Chicken => 'k',
            CreatureType::Zombie => 'Z',
            CreatureType::Skeleton => 'S',
            CreatureType::Slime => 'o',
            CreatureType::Spider => 'x',
        }
    }

    pub fn is_hostile(&self) -> bool {
        matches!(
            self,
            CreatureType::Zombie
                | CreatureType::Skeleton
                | CreatureType::Slime
                | CreatureType::Spider
        )
    }

    pub fn max_hp(&self) -> i32 {
        match self {
            CreatureType::Pig => 10,
            CreatureType::Cow => 15,
            CreatureType::Chicken => 5,
            CreatureType::Zombie => 20,
            CreatureType::Skeleton => 15,
            CreatureType::Slime => 10,
            CreatureType::Spider => 12,
        }
    }

    pub fn damage(&self) -> i32 {
        match self {
            CreatureType::Pig | CreatureType::Cow | CreatureType::Chicken => 0,
            CreatureType::Zombie => 4,
            CreatureType::Skeleton => 3,
            CreatureType::Slime => 2,
            CreatureType::Spider => 3,
        }
    }

    pub fn drops(&self) -> Option<(ItemType, u32)> {
        match self {
            CreatureType::Pig | CreatureType::Cow => Some((ItemType::RawMeat, 2)),
            CreatureType::Chicken => Some((ItemType::RawMeat, 1)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Creature {
    pub creature_type: CreatureType,
    pub x: f32,
    pub y: f32,
    pub vel_x: f32,
    pub vel_y: f32,
    pub hp: i32,
    pub direction: i8, // -1 left, 1 right
    pub attack_cooldown: u32,
}

// =============================================================================
// GAME STATE
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlockworldView {
    #[default]
    Menu,
    Playing,
    Inventory,
    Crafting,
    Paused,
    GameOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeOfDay {
    Dawn,
    Day,
    Dusk,
    Night,
}

pub struct BlockworldState {
    // View state
    pub view: BlockworldView,

    // World
    pub blocks: Vec<Vec<BlockType>>,
    pub creatures: Vec<Creature>,

    // Player
    pub player_x: f32,
    pub player_y: f32,
    pub player_vel_x: f32,
    pub player_vel_y: f32,
    pub player_hp: i32,
    pub player_max_hp: i32,
    pub player_hunger: i32,
    pub player_max_hunger: i32,
    pub player_direction: i8,
    pub on_ground: bool,

    // Camera
    pub camera_x: usize,
    pub camera_y: usize,

    // Mining
    pub mining_target: Option<(usize, usize)>,
    pub mining_progress: u32,

    // Cursor (for placing/mining)
    pub cursor_x: i32,
    pub cursor_y: i32,

    // Inventory (9 hotbar + 27 main = 36 slots)
    pub inventory: Vec<Option<InventorySlot>>,
    pub selected_slot: usize,
    pub inventory_cursor: usize,

    // Time
    pub time_of_day: u32,
    pub day_count: u32,

    // Game state
    pub tick_count: u32,
    pub score: u32,
    pub blocks_mined: u32,
    pub blocks_placed: u32,
    pub creatures_killed: u32,
    pub game_over: bool,
    pub spawn_x: f32,
    pub spawn_y: f32,

    // Events
    pub pending_events: Vec<GameEvent>,
    pub message: Option<String>,
    pub message_timer: u32,
}

impl Default for BlockworldState {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockworldState {
    pub fn new() -> Self {
        let mut state = Self {
            view: BlockworldView::Menu,
            blocks: vec![vec![BlockType::Air; WORLD_HEIGHT]; WORLD_WIDTH],
            creatures: Vec::new(),
            player_x: (WORLD_WIDTH / 2) as f32,
            player_y: SKY_HEIGHT as f32 - 3.0,
            player_vel_x: 0.0,
            player_vel_y: 0.0,
            player_hp: 20,
            player_max_hp: 20,
            player_hunger: 20,
            player_max_hunger: 20,
            player_direction: 1,
            on_ground: false,
            camera_x: 0,
            camera_y: 0,
            mining_target: None,
            mining_progress: 0,
            cursor_x: 0,
            cursor_y: 0,
            inventory: vec![None; 36],
            selected_slot: 0,
            inventory_cursor: 0,
            time_of_day: DAY_START,
            day_count: 1,
            tick_count: 0,
            score: 0,
            blocks_mined: 0,
            blocks_placed: 0,
            creatures_killed: 0,
            game_over: false,
            spawn_x: (WORLD_WIDTH / 2) as f32,
            spawn_y: SKY_HEIGHT as f32 - 3.0,
            pending_events: Vec::new(),
            message: None,
            message_timer: 0,
        };
        state.generate_world();
        // Give starter items
        state.add_item(ItemType::Tool(ToolType::Pickaxe, ToolMaterial::Wood));
        state.add_item(ItemType::Tool(ToolType::Axe, ToolMaterial::Wood));
        state.add_item(ItemType::Torch);
        state.add_item(ItemType::Torch);
        state.add_item(ItemType::Torch);
        state
    }

    pub fn start_game(&mut self) {
        *self = Self::new();
        self.view = BlockworldView::Playing;
        self.pending_events.push(GameEvent::GameStarted);
    }

    fn generate_world(&mut self) {
        let mut rng = rand::thread_rng();

        // Generate terrain using simple heightmap
        let mut heights = vec![0usize; WORLD_WIDTH];
        let base_height = SKY_HEIGHT;

        // Generate smooth terrain
        let mut height = base_height as f32;
        for h in heights.iter_mut() {
            // Random walk for terrain
            height += rng.gen_range(-0.5..0.5);
            height = height.clamp((base_height - 10) as f32, (base_height + 10) as f32);
            *h = height as usize;
        }

        // Fill blocks
        for (x, &surface) in heights.iter().enumerate() {
            for y in 0..WORLD_HEIGHT {
                self.blocks[x][y] = if y < surface.saturating_sub(5) {
                    // Sky
                    BlockType::Air
                } else if y == surface.saturating_sub(5) {
                    // Grass layer
                    BlockType::Grass
                } else if y < surface + 5 {
                    // Dirt layer
                    BlockType::Dirt
                } else if y >= WORLD_HEIGHT - 3 {
                    // Bedrock
                    BlockType::Bedrock
                } else {
                    // Stone with ores
                    let depth = y - surface;
                    let ore_roll: f32 = rng.gen::<f32>();

                    if ore_roll < 0.02 && depth > 30 {
                        BlockType::Diamond
                    } else if ore_roll < 0.05 && depth > 20 {
                        BlockType::Gold
                    } else if ore_roll < 0.10 && depth > 10 {
                        BlockType::Iron
                    } else if ore_roll < 0.15 {
                        BlockType::Coal
                    } else {
                        BlockType::Stone
                    }
                };
            }
        }

        // Generate caves
        self.generate_caves(&mut rng);

        // Generate trees
        self.generate_trees(&mut rng, &heights);

        // Set spawn point
        let spawn_x = WORLD_WIDTH / 2;
        let spawn_y = heights[spawn_x].saturating_sub(6);
        self.player_x = spawn_x as f32;
        self.player_y = spawn_y as f32;
        self.spawn_x = spawn_x as f32;
        self.spawn_y = spawn_y as f32;

        // Center camera on player
        self.update_camera();
    }

    fn generate_caves(&mut self, rng: &mut impl Rng) {
        // Simple cave generation - random tunnels
        for _ in 0..20 {
            let mut x = rng.gen_range(10..WORLD_WIDTH - 10);
            let mut y = rng.gen_range(SKY_HEIGHT + 10..WORLD_HEIGHT - 10);

            for _ in 0..50 {
                // Carve out area
                for dx in -2i32..=2 {
                    for dy in -2i32..=2 {
                        let nx = (x as i32 + dx) as usize;
                        let ny = (y as i32 + dy) as usize;
                        if nx < WORLD_WIDTH
                            && ny < WORLD_HEIGHT
                            && ny > SKY_HEIGHT
                            && self.blocks[nx][ny] != BlockType::Bedrock
                        {
                            self.blocks[nx][ny] = BlockType::Air;
                        }
                    }
                }

                // Random walk
                match rng.gen_range(0..4) {
                    0 => x = x.saturating_sub(1).max(1),
                    1 => x = (x + 1).min(WORLD_WIDTH - 2),
                    2 => y = y.saturating_sub(1).max(SKY_HEIGHT + 1),
                    _ => y = (y + 1).min(WORLD_HEIGHT - 4),
                }
            }
        }
    }

    fn generate_trees(&mut self, rng: &mut impl Rng, heights: &[usize]) {
        let mut last_tree_x = 0;

        for (x, &ground_height) in heights.iter().enumerate().skip(5).take(WORLD_WIDTH - 10) {
            if x - last_tree_x < 8 {
                continue;
            }

            if rng.gen::<f32>() < 0.15 {
                let ground_y = ground_height.saturating_sub(5);
                let tree_height = rng.gen_range(4..8);

                // Trunk
                for ty in 0..tree_height {
                    let y = ground_y.saturating_sub(ty + 1);
                    if y > 0 && self.blocks[x][y] == BlockType::Air {
                        self.blocks[x][y] = BlockType::Wood;
                    }
                }

                // Leaves (simple dome)
                let top_y = ground_y.saturating_sub(tree_height);
                for dx in -2i32..=2 {
                    for dy in -2i32..=1 {
                        let lx = (x as i32 + dx) as usize;
                        let ly = (top_y as i32 + dy) as usize;
                        if lx < WORLD_WIDTH
                            && ly < WORLD_HEIGHT
                            && self.blocks[lx][ly] == BlockType::Air
                        {
                            self.blocks[lx][ly] = BlockType::Leaves;
                        }
                    }
                }

                last_tree_x = x;
            }
        }
    }

    fn update_camera(&mut self) {
        // Center camera on player
        let target_x = (self.player_x as usize).saturating_sub(VIEW_WIDTH / 2);
        let target_y = (self.player_y as usize).saturating_sub(VIEW_HEIGHT / 2);

        self.camera_x = target_x.min(WORLD_WIDTH.saturating_sub(VIEW_WIDTH));
        self.camera_y = target_y.min(WORLD_HEIGHT.saturating_sub(VIEW_HEIGHT));
    }

    pub fn get_time_of_day(&self) -> TimeOfDay {
        if self.time_of_day < DAY_START {
            TimeOfDay::Dawn
        } else if self.time_of_day < DUSK_START {
            TimeOfDay::Day
        } else if self.time_of_day < NIGHT_START {
            TimeOfDay::Dusk
        } else {
            TimeOfDay::Night
        }
    }

    pub fn is_night(&self) -> bool {
        matches!(self.get_time_of_day(), TimeOfDay::Night)
    }

    fn add_item(&mut self, item: ItemType) -> bool {
        // Try to stack with existing
        if item.max_stack() > 1 {
            for slot in &mut self.inventory {
                if let Some(s) = slot {
                    if s.item == item && s.count < item.max_stack() {
                        s.count += 1;
                        return true;
                    }
                }
            }
        }

        // Find empty slot
        for slot in &mut self.inventory {
            if slot.is_none() {
                *slot = Some(InventorySlot {
                    item,
                    count: 1,
                    durability: if let ItemType::Tool(_, m) = item {
                        Some(m.durability())
                    } else {
                        None
                    },
                });
                return true;
            }
        }

        false // Inventory full
    }

    fn get_selected_item(&self) -> Option<&InventorySlot> {
        self.inventory[self.selected_slot].as_ref()
    }

    fn use_selected_item(&mut self) {
        if let Some(slot) = &mut self.inventory[self.selected_slot] {
            if let Some(dur) = &mut slot.durability {
                *dur = dur.saturating_sub(1);
                if *dur == 0 {
                    self.inventory[self.selected_slot] = None;
                    self.show_message("Tool broke!");
                }
            } else if slot.count > 1 {
                slot.count -= 1;
            } else {
                self.inventory[self.selected_slot] = None;
            }
        }
    }

    fn show_message(&mut self, msg: &str) {
        self.message = Some(msg.to_string());
        self.message_timer = 60;
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

    fn get_block(&self, x: i32, y: i32) -> BlockType {
        if x < 0 || y < 0 || x >= WORLD_WIDTH as i32 || y >= WORLD_HEIGHT as i32 {
            BlockType::Bedrock
        } else {
            self.blocks[x as usize][y as usize]
        }
    }

    fn set_block(&mut self, x: usize, y: usize, block: BlockType) {
        if x < WORLD_WIDTH && y < WORLD_HEIGHT {
            self.blocks[x][y] = block;
        }
    }

    fn mine_block(&mut self) {
        let target_x = (self.player_x as i32 + self.cursor_x) as usize;
        let target_y = (self.player_y as i32 + self.cursor_y) as usize;

        if target_x >= WORLD_WIDTH || target_y >= WORLD_HEIGHT {
            return;
        }

        let block = self.blocks[target_x][target_y];
        if block == BlockType::Air || block == BlockType::Bedrock {
            return;
        }

        // Check if we have the right tool
        let mining_speed = if let Some(InventorySlot {
            item: ItemType::Tool(tool_type, material),
            ..
        }) = self.get_selected_item()
        {
            if block.required_tool() == Some(*tool_type) {
                material.mining_speed()
            } else {
                MINE_SPEED_HAND * 2 // Wrong tool is slower
            }
        } else {
            MINE_SPEED_HAND
        };

        // Check if same target
        if self.mining_target != Some((target_x, target_y)) {
            self.mining_target = Some((target_x, target_y));
            self.mining_progress = 0;
        }

        self.mining_progress += 1;

        let required = (block.hardness() * mining_speed) / 10;
        if self.mining_progress >= required {
            // Block mined!
            if let Some((drop_item, count)) = block.drops() {
                for _ in 0..count {
                    self.add_item(drop_item);
                }
            }

            self.set_block(target_x, target_y, BlockType::Air);
            self.blocks_mined += 1;
            self.score += 1;

            // Use tool durability
            if let Some(InventorySlot {
                item: ItemType::Tool(_, _),
                ..
            }) = self.get_selected_item()
            {
                self.use_selected_item();
            }

            self.mining_target = None;
            self.mining_progress = 0;

            // Block mined event
        }
    }

    fn place_block(&mut self) {
        let target_x = (self.player_x as i32 + self.cursor_x) as usize;
        let target_y = (self.player_y as i32 + self.cursor_y) as usize;

        if target_x >= WORLD_WIDTH || target_y >= WORLD_HEIGHT {
            return;
        }

        // Can't place where player is
        let px = self.player_x as usize;
        let py = self.player_y as usize;
        if target_x == px && (target_y == py || target_y == py + 1) {
            return;
        }

        if self.blocks[target_x][target_y] != BlockType::Air {
            return;
        }

        // Check if selected item is a placeable block
        if let Some(slot) = &self.inventory[self.selected_slot] {
            if let ItemType::Block(block_type) = slot.item {
                self.set_block(target_x, target_y, block_type);
                self.use_selected_item();
                self.blocks_placed += 1;

                // Block placed event
            } else if slot.item == ItemType::Torch {
                self.set_block(target_x, target_y, BlockType::Torch);
                self.use_selected_item();
            }
        }
    }

    fn update_physics(&mut self) {
        // Apply gravity
        self.player_vel_y += GRAVITY;
        self.player_vel_y = self.player_vel_y.min(5.0); // Terminal velocity

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
            .get_block(test_x as i32, self.player_y as i32)
            .is_solid()
            && !self
                .get_block(test_x as i32, (self.player_y + 1.0) as i32)
                .is_solid()
        {
            self.player_x = new_x.clamp(0.5, (WORLD_WIDTH - 1) as f32 - 0.5);
        }
        self.player_vel_x *= 0.8; // Friction

        // Vertical collision
        self.on_ground = false;
        if self.player_vel_y > 0.0 {
            // Falling
            if self
                .get_block(self.player_x as i32, (new_y + 1.0) as i32)
                .is_solid()
            {
                self.player_y = (new_y as i32) as f32;
                self.player_vel_y = 0.0;
                self.on_ground = true;
            } else {
                self.player_y = new_y.min((WORLD_HEIGHT - 2) as f32);
            }
        } else {
            // Rising
            if self
                .get_block(self.player_x as i32, new_y as i32)
                .is_solid()
            {
                self.player_vel_y = 0.0;
            } else {
                self.player_y = new_y.max(0.0);
            }
        }

        // Update camera
        self.update_camera();
    }

    fn spawn_creatures(&mut self) {
        let mut rng = rand::thread_rng();

        // Only spawn at night for hostile, or during day for passive
        let is_night = self.is_night();

        // Limit total creatures
        if self.creatures.len() >= 10 {
            return;
        }

        // Random spawn near player
        if rng.gen::<f32>() < 0.02 {
            let spawn_dist = rng.gen_range(20..40) as f32;
            let spawn_dir = if rng.gen::<bool>() { 1.0 } else { -1.0 };
            let spawn_x = self.player_x + spawn_dist * spawn_dir;

            if spawn_x > 0.0 && spawn_x < WORLD_WIDTH as f32 {
                // Find ground level
                let x = spawn_x as usize;
                let mut spawn_y = None;
                for y in 0..WORLD_HEIGHT - 1 {
                    if self.blocks[x][y] == BlockType::Air && self.blocks[x][y + 1].is_solid() {
                        spawn_y = Some(y as f32);
                        break;
                    }
                }

                if let Some(y) = spawn_y {
                    let creature_type = if is_night {
                        match rng.gen_range(0..4) {
                            0 => CreatureType::Zombie,
                            1 => CreatureType::Skeleton,
                            2 => CreatureType::Slime,
                            _ => CreatureType::Spider,
                        }
                    } else {
                        match rng.gen_range(0..3) {
                            0 => CreatureType::Pig,
                            1 => CreatureType::Cow,
                            _ => CreatureType::Chicken,
                        }
                    };

                    self.creatures.push(Creature {
                        creature_type,
                        x: spawn_x,
                        y,
                        vel_x: 0.0,
                        vel_y: 0.0,
                        hp: creature_type.max_hp(),
                        direction: if spawn_dir > 0.0 { -1 } else { 1 },
                        attack_cooldown: 0,
                    });
                }
            }
        }
    }

    fn update_creatures(&mut self) {
        let player_x = self.player_x;
        let player_y = self.player_y;
        let mut player_damage = 0i32;

        // Helper to check block solidity without borrowing self
        let check_solid = |blocks: &Vec<Vec<BlockType>>, x: i32, y: i32| -> bool {
            if x < 0 || y < 0 || x >= WORLD_WIDTH as i32 || y >= WORLD_HEIGHT as i32 {
                true // Out of bounds = solid
            } else {
                blocks[x as usize][y as usize].is_solid()
            }
        };

        let mut rng = rand::thread_rng();

        for creature in &mut self.creatures {
            // Apply gravity
            creature.vel_y += GRAVITY;
            creature.vel_y = creature.vel_y.min(5.0);

            // AI movement
            if creature.creature_type.is_hostile() {
                // Chase player
                let dx = player_x - creature.x;
                if dx.abs() > 1.0 {
                    creature.vel_x = if dx > 0.0 { 0.3 } else { -0.3 };
                    creature.direction = if dx > 0.0 { 1 } else { -1 };
                }

                // Jump if blocked
                let next_x = creature.x + creature.vel_x;
                if check_solid(&self.blocks, next_x as i32, creature.y as i32)
                    && creature.vel_y >= 0.0
                {
                    creature.vel_y = JUMP_VELOCITY;
                }
            } else {
                // Random wander
                if rng.gen::<f32>() < 0.02 {
                    creature.direction = if rng.gen::<bool>() { 1 } else { -1 };
                }
                creature.vel_x = creature.direction as f32 * 0.1;
            }

            // Apply velocity with collision
            let new_x = creature.x + creature.vel_x;
            let new_y = creature.y + creature.vel_y;

            // Horizontal
            if !check_solid(&self.blocks, new_x as i32, creature.y as i32) {
                creature.x = new_x.clamp(0.5, (WORLD_WIDTH - 1) as f32 - 0.5);
            } else {
                creature.direction *= -1;
            }

            // Vertical
            if creature.vel_y > 0.0 {
                if check_solid(&self.blocks, creature.x as i32, (new_y + 1.0) as i32) {
                    creature.y = new_y.floor();
                    creature.vel_y = 0.0;
                } else {
                    creature.y = new_y.min((WORLD_HEIGHT - 2) as f32);
                }
            } else if !check_solid(&self.blocks, creature.x as i32, new_y as i32) {
                creature.y = new_y.max(0.0);
            } else {
                creature.vel_y = 0.0;
            }

            // Attack player if close
            if creature.attack_cooldown > 0 {
                creature.attack_cooldown -= 1;
            }

            if creature.creature_type.is_hostile() {
                let dist =
                    ((player_x - creature.x).powi(2) + (player_y - creature.y).powi(2)).sqrt();
                if dist < 1.5 && creature.attack_cooldown == 0 {
                    player_damage += creature.creature_type.damage();
                    creature.attack_cooldown = 30;
                }
            }
        }

        // Apply player damage after loop
        if player_damage > 0 {
            self.player_hp -= player_damage;
            if self.player_hp <= 0 {
                self.die();
            }
        }

        // Remove dead creatures and despawn far ones
        self.creatures
            .retain(|c| c.hp > 0 && (c.x - player_x).abs() < 60.0);
    }

    fn attack(&mut self) {
        // Attack in cursor direction
        let attack_x = self.player_x + self.cursor_x as f32;
        let attack_y = self.player_y + self.cursor_y as f32;

        let damage = if let Some(InventorySlot {
            item: ItemType::Tool(ToolType::Sword, material),
            ..
        }) = self.get_selected_item()
        {
            material.damage()
        } else {
            2 // Fist damage
        };

        let mut hit = false;
        let mut kills = 0u32;
        let mut score_gain = 0u32;
        let mut items_to_add: Vec<(ItemType, u32)> = Vec::new();

        for creature in &mut self.creatures {
            let dist = ((attack_x - creature.x).powi(2) + (attack_y - creature.y).powi(2)).sqrt();
            if dist < 2.0 {
                creature.hp -= damage;
                hit = true;

                if creature.hp <= 0 {
                    kills += 1;
                    score_gain += 10;

                    // Collect items to drop
                    if let Some((item, count)) = creature.creature_type.drops() {
                        items_to_add.push((item, count));
                    }
                }
            }
        }

        // Apply kills and score after loop
        self.creatures_killed += kills;
        self.score += score_gain;

        // Add collected items
        for (item, count) in items_to_add {
            for _ in 0..count {
                self.add_item(item);
            }
        }

        if hit {
            // Use sword durability
            if let Some(InventorySlot {
                item: ItemType::Tool(ToolType::Sword, _),
                ..
            }) = self.get_selected_item()
            {
                self.use_selected_item();
            }
        }
    }

    fn die(&mut self) {
        self.player_hp = self.player_max_hp;
        self.player_x = self.spawn_x;
        self.player_y = self.spawn_y;
        self.player_vel_x = 0.0;
        self.player_vel_y = 0.0;

        // Drop some items
        self.show_message("You died! Respawning...");

        self.pending_events
            .push(GameEvent::GameEnded { won: false });
    }

    fn update_time(&mut self) {
        self.time_of_day += 1;
        if self.time_of_day >= DAY_LENGTH {
            self.time_of_day = 0;
            self.day_count += 1;
        }

        // Hunger decreases over time
        if self.tick_count.is_multiple_of(200) && self.player_hunger > 0 {
            self.player_hunger -= 1;
        }

        // Starving damages player
        if self.player_hunger == 0 && self.tick_count.is_multiple_of(60) {
            self.player_hp -= 1;
            if self.player_hp <= 0 {
                self.die();
            }
        }

        // Regenerate health if well-fed
        if self.player_hunger > 15
            && self.player_hp < self.player_max_hp
            && self.tick_count.is_multiple_of(100)
        {
            self.player_hp += 1;
        }
    }

    fn eat(&mut self) {
        if let Some(slot) = &self.inventory[self.selected_slot] {
            match slot.item {
                ItemType::RawMeat => {
                    self.player_hunger = (self.player_hunger + 3).min(self.player_max_hunger);
                    self.use_selected_item();
                    self.show_message("Ate raw meat (+3 hunger)");
                }
                ItemType::CookedMeat => {
                    self.player_hunger = (self.player_hunger + 8).min(self.player_max_hunger);
                    self.use_selected_item();
                    self.show_message("Ate cooked meat (+8 hunger)");
                }
                _ => {}
            }
        }
    }
}

// =============================================================================
// GAME ENGINE IMPLEMENTATION
// =============================================================================

impl GameEngine for BlockworldState {
    fn tick(&mut self) {
        // Always increment for menu animation
        self.tick_count = self.tick_count.wrapping_add(1);

        if self.view != BlockworldView::Playing {
            return;
        }

        // Update physics
        self.update_physics();

        // Update time
        self.update_time();

        // Update message timer
        if self.message_timer > 0 {
            self.message_timer -= 1;
            if self.message_timer == 0 {
                self.message = None;
            }
        }

        // Spawn and update creatures
        self.spawn_creatures();
        self.update_creatures();
    }

    fn handle_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match self.view {
            BlockworldView::Menu => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.start_game();
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => KeyHandleResult::RequestQuit,
                _ => KeyHandleResult::Handled,
            },
            BlockworldView::Playing => match key.code {
                KeyCode::Esc => {
                    self.view = BlockworldView::Paused;
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
                KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => {
                    // Crouch or down - maybe fall through platforms later
                    KeyHandleResult::Handled
                }
                // Mining cursor
                KeyCode::Char('h') | KeyCode::Char('H') => {
                    self.cursor_x = -1;
                    self.cursor_y = 0;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('j') | KeyCode::Char('J') => {
                    self.cursor_x = 0;
                    self.cursor_y = 1;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('k') | KeyCode::Char('K') => {
                    self.cursor_x = 0;
                    self.cursor_y = -1;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('l') | KeyCode::Char('L') => {
                    self.cursor_x = 1;
                    self.cursor_y = 0;
                    KeyHandleResult::Handled
                }
                // Mining/placing
                KeyCode::Char('z') | KeyCode::Char('Z') => {
                    self.mine_block();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('x') | KeyCode::Char('X') => {
                    self.place_block();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    self.attack();
                    KeyHandleResult::Handled
                }
                // Eat
                KeyCode::Char('e') | KeyCode::Char('E') => {
                    self.eat();
                    KeyHandleResult::Handled
                }
                // Inventory
                KeyCode::Char('i') | KeyCode::Char('I') | KeyCode::Tab => {
                    self.view = BlockworldView::Inventory;
                    KeyHandleResult::Handled
                }
                // Hotbar selection
                KeyCode::Char('1') => {
                    self.selected_slot = 0;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('2') => {
                    self.selected_slot = 1;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('3') => {
                    self.selected_slot = 2;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('4') => {
                    self.selected_slot = 3;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('5') => {
                    self.selected_slot = 4;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('6') => {
                    self.selected_slot = 5;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('7') => {
                    self.selected_slot = 6;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('8') => {
                    self.selected_slot = 7;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('9') => {
                    self.selected_slot = 8;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            BlockworldView::Inventory => match key.code {
                KeyCode::Esc | KeyCode::Char('i') | KeyCode::Char('I') | KeyCode::Tab => {
                    self.view = BlockworldView::Playing;
                    KeyHandleResult::Handled
                }
                KeyCode::Left => {
                    if self.inventory_cursor > 0 {
                        self.inventory_cursor -= 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Right => {
                    if self.inventory_cursor < 35 {
                        self.inventory_cursor += 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Up => {
                    if self.inventory_cursor >= 9 {
                        self.inventory_cursor -= 9;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Down => {
                    if self.inventory_cursor + 9 < 36 {
                        self.inventory_cursor += 9;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    // Swap with selected hotbar slot if in hotbar, or move to hotbar
                    if self.inventory_cursor < 9 {
                        self.selected_slot = self.inventory_cursor;
                    } else {
                        // Swap current slot with inventory cursor
                        self.inventory
                            .swap(self.selected_slot, self.inventory_cursor);
                    }
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            BlockworldView::Paused => match key.code {
                KeyCode::Esc | KeyCode::Char('p') | KeyCode::Char('P') => {
                    self.view = BlockworldView::Playing;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('q') | KeyCode::Char('Q') => KeyHandleResult::RequestQuit,
                _ => KeyHandleResult::Handled,
            },
            BlockworldView::Crafting => match key.code {
                KeyCode::Esc => {
                    self.view = BlockworldView::Playing;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            BlockworldView::GameOver => match key.code {
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
