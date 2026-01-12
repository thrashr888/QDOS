//! Clicker - A roguelike-themed incremental/idle game
//!
//! Based on "The Rogue Clicker" concept.
//! Fight monsters, gain gold and XP, level up, buy upgrades.

use rand::Rng;
use serde::{Deserialize, Serialize};

/// Enemy types with their display characters
const ENEMIES: &[(&str, char, &str)] = &[
    ("Rat", 'r', "A small, quick vermin."),
    ("Bat", 'b', "A flying creature of the night."),
    ("Kobold", 'k', "A small, cunning reptilian."),
    ("Goblin", 'g', "Small, green, and sneaky."),
    ("Orc", 'o', "A brutish green humanoid."),
    ("Zombie", 'Z', "The walking dead."),
    ("Skeleton", 'S', "Animated bones."),
    ("Ghost", 'G', "A restless spirit."),
    ("Vampire", 'V', "An undead blood-drinker."),
    ("Werewolf", 'W', "A cursed shapeshifter."),
    ("Troll", 'T', "Regenerating brute."),
    ("Ogre", 'O', "Massive and hungry."),
    ("Demon", 'D', "A fiend from the abyss."),
    ("Wraith", 'w', "A shadow of death."),
    ("Lich", 'L', "An undead sorcerer."),
    ("Hydra", 'H', "Many-headed serpent."),
    ("Dragon", 'd', "Ancient winged terror."),
    ("Balrog", 'B', "Flame and shadow."),
    ("Jabberwock", 'J', "The stuff of nightmares."),
];
const SCENERY: &[(&str, char, &str)] = &[
    ("Floor", '_', "The floor of the dungeon."),
    ("Wall", '|', "The walls of the dungeon."),
    ("Water", '~', "The water of the dungeon."),
    ("Grass", '"', "The grass of the dungeon."),
    ("Moss", '\'', "The moss of the dungeon."),
    ("Rubble", ',', "The rubble of the dungeon."),
    ("Bones", ')', "Scattered bones."),
    ("Torch", 'i', "A flickering wall torch."),
    ("Blood", ';', "Dried bloodstains."),
    ("Trap", '^', "A hidden trap."),
    ("Gold", '*', "A glint of treasure."),
];

/// Scenery elements for the dungeon floor
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Scenery {
    pub char: char,
    pub color_idx: u8, // 0=grey, 1=yellow, 2=cyan, 3=red, 4=green
}

impl Scenery {
    fn random() -> Self {
        let mut rng = rand::thread_rng();
        let roll = rng.gen_range(0..100);
        match roll {
            0..=59 => Self {
                char: SCENERY[0].1,
                color_idx: 0,
            }, // Floor (grey)
            60..=69 => Self {
                char: SCENERY[1].1,
                color_idx: 0,
            }, // Wall (grey)
            70..=74 => Self {
                char: SCENERY[2].1,
                color_idx: 2,
            }, // Water (cyan)
            75..=79 => Self {
                char: SCENERY[3].1,
                color_idx: 4,
            }, // Grass (green)
            80..=84 => Self {
                char: SCENERY[4].1,
                color_idx: 4,
            }, // Moss (green)
            85..=89 => Self {
                char: SCENERY[5].1,
                color_idx: 0,
            }, // Rubble (grey)
            90..=92 => Self {
                char: SCENERY[6].1,
                color_idx: 1,
            }, // Bones (yellow)
            93..=95 => Self {
                char: SCENERY[7].1,
                color_idx: 1,
            }, // Torch (yellow)
            96..=97 => Self {
                char: SCENERY[8].1,
                color_idx: 3,
            }, // Blood (red)
            98 => Self {
                char: SCENERY[9].1,
                color_idx: 3,
            }, // Trap (red)
            _ => Self {
                char: SCENERY[10].1,
                color_idx: 1,
            }, // Treasure hint (yellow)
        }
    }
}

// =============================================================================
// BIOMES - Visual variety per floor range
// =============================================================================

/// Dungeon biomes change every 20 floors
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Biome {
    #[default]
    Mines, // Floors 1-20: Basic dungeon
    Swamp, // Floors 21-40: Murky wetlands
    Crypt, // Floors 41-60: Undead themed
    Halls, // Floors 61-80: Grand demon halls
    Abyss, // Floors 81-100: The final depths
}

impl Biome {
    pub fn from_floor(floor: i32) -> Self {
        match floor {
            1..=20 => Biome::Mines,
            21..=40 => Biome::Swamp,
            41..=60 => Biome::Crypt,
            61..=80 => Biome::Halls,
            _ => Biome::Abyss,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Biome::Mines => "The Mines",
            Biome::Swamp => "The Swamp",
            Biome::Crypt => "The Crypt",
            Biome::Halls => "Demon Halls",
            Biome::Abyss => "The Abyss",
        }
    }

    pub fn floor_char(&self) -> char {
        match self {
            Biome::Mines => '.',
            Biome::Swamp => ',',
            Biome::Crypt => '_',
            Biome::Halls => '+',
            Biome::Abyss => '~',
        }
    }

    pub fn wall_char(&self) -> char {
        match self {
            Biome::Mines => '#',
            Biome::Swamp => 'T',
            Biome::Crypt => '▓',
            Biome::Halls => '║',
            Biome::Abyss => '░',
        }
    }

    /// Color index for biome: 0=grey, 1=yellow, 2=cyan, 3=red, 4=green, 5=blue
    pub fn color_idx(&self) -> u8 {
        match self {
            Biome::Mines => 0, // Grey
            Biome::Swamp => 4, // Green
            Biome::Crypt => 0, // Grey
            Biome::Halls => 3, // Red
            Biome::Abyss => 5, // Blue
        }
    }
}

// =============================================================================
// ASCENSION CLASSES - Soul-purchased starting bonuses
// =============================================================================

/// Ascension classes change playstyle
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum AscensionClass {
    #[default]
    Peasant, // Default, no bonuses
    Rogue,         // +15% crit, +10% gold
    Warrior,       // +5 STR, +5 ARM
    Wizard,        // Scrolls 2x potency, -20% HP
    Tourist,       // +50% gold, -30% damage
    Archaeologist, // +25% item drops, starts with artifact
}

impl AscensionClass {
    pub fn all() -> &'static [AscensionClass] {
        &[
            AscensionClass::Peasant,
            AscensionClass::Rogue,
            AscensionClass::Warrior,
            AscensionClass::Wizard,
            AscensionClass::Tourist,
            AscensionClass::Archaeologist,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            AscensionClass::Peasant => "Peasant",
            AscensionClass::Rogue => "Rogue",
            AscensionClass::Warrior => "Warrior",
            AscensionClass::Wizard => "Wizard",
            AscensionClass::Tourist => "Tourist",
            AscensionClass::Archaeologist => "Archaeologist",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            AscensionClass::Peasant => "No bonuses. The humble beginning.",
            AscensionClass::Rogue => "+15% crit, +10% gold find",
            AscensionClass::Warrior => "+5 STR, +5 ARM at start",
            AscensionClass::Wizard => "Scrolls 2x potent, -20% HP",
            AscensionClass::Tourist => "+50% gold, -30% damage",
            AscensionClass::Archaeologist => "+25% drops, start with artifact",
        }
    }

    pub fn unlock_cost(&self) -> i64 {
        match self {
            AscensionClass::Peasant => 0,
            AscensionClass::Rogue => 50,
            AscensionClass::Warrior => 100,
            AscensionClass::Wizard => 200,
            AscensionClass::Tourist => 150,
            AscensionClass::Archaeologist => 500,
        }
    }
}

// =============================================================================
// ALCHEMY MASTERY - Knowledge progression for potions
// =============================================================================

/// Alchemy mastery levels and bonuses
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AlchemyTier {
    Novice,      // Lv 0-9: No bonus
    Apprentice,  // Lv 10-24: +25% potion duration
    Journeyman,  // Lv 25-49: +50% duration, auto-ID potions
    Expert,      // Lv 50-74: +75% duration, refine bad potions
    Master,      // Lv 75-99: +100% duration, potions never fail
    Grandmaster, // Lv 100+: 2x effects, instant auto-quaff
}

impl AlchemyTier {
    pub fn from_level(level: i32) -> Self {
        match level {
            0..=9 => AlchemyTier::Novice,
            10..=24 => AlchemyTier::Apprentice,
            25..=49 => AlchemyTier::Journeyman,
            50..=74 => AlchemyTier::Expert,
            75..=99 => AlchemyTier::Master,
            _ => AlchemyTier::Grandmaster,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            AlchemyTier::Novice => "Novice",
            AlchemyTier::Apprentice => "Apprentice",
            AlchemyTier::Journeyman => "Journeyman",
            AlchemyTier::Expert => "Expert",
            AlchemyTier::Master => "Master",
            AlchemyTier::Grandmaster => "Grandmaster",
        }
    }

    /// Duration multiplier (percentage bonus)
    pub fn duration_bonus(&self) -> i32 {
        match self {
            AlchemyTier::Novice => 0,
            AlchemyTier::Apprentice => 25,
            AlchemyTier::Journeyman => 50,
            AlchemyTier::Expert => 75,
            AlchemyTier::Master => 100,
            AlchemyTier::Grandmaster => 100, // Same but 2x effect
        }
    }

    /// Effect multiplier for Grandmaster
    pub fn effect_multiplier(&self) -> i32 {
        match self {
            AlchemyTier::Grandmaster => 2,
            _ => 1,
        }
    }
}

// =============================================================================
// TRANSMUTATION - Convert items to Arcane Dust
// =============================================================================

/// Transmutation filter levels
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum TransmuteFilter {
    #[default]
    Off, // No auto-transmute
    Common,   // Auto-transmute common items
    Uncommon, // Auto-transmute uncommon and below
    Rare,     // Auto-transmute rare and below
    All,      // Transmute everything immediately
}

impl TransmuteFilter {
    pub fn all() -> &'static [TransmuteFilter] {
        &[
            TransmuteFilter::Off,
            TransmuteFilter::Common,
            TransmuteFilter::Uncommon,
            TransmuteFilter::Rare,
            TransmuteFilter::All,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            TransmuteFilter::Off => "Off",
            TransmuteFilter::Common => "Common",
            TransmuteFilter::Uncommon => "Uncommon",
            TransmuteFilter::Rare => "Rare",
            TransmuteFilter::All => "All",
        }
    }
}

/// The Heirloom - a weapon that persists across runs
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Heirloom {
    pub name: String,
    pub enchant_level: i32, // +1, +2, etc.
    pub str_bonus: i32,
    pub crit_bonus: i32,
    pub life_steal_bonus: i32,
}

impl Heirloom {
    pub fn new() -> Self {
        Self {
            name: "Ancestral Blade".to_string(),
            enchant_level: 0,
            str_bonus: 1,
            crit_bonus: 0,
            life_steal_bonus: 0,
        }
    }

    /// Cost to upgrade to next level
    pub fn upgrade_cost(&self) -> i64 {
        // Exponential scaling: 10 * 2^level
        10 * (2_i64.pow(self.enchant_level as u32))
    }

    /// Apply enchantment (costs dust)
    pub fn enchant(&mut self) {
        self.enchant_level += 1;
        // Every level: +1 STR
        self.str_bonus += 1;
        // Every 3 levels: +1% crit
        if self.enchant_level % 3 == 0 {
            self.crit_bonus += 1;
        }
        // Every 5 levels: +1% life steal
        if self.enchant_level % 5 == 0 {
            self.life_steal_bonus += 1;
        }
        // Update name
        self.name = format!("Ancestral Blade +{}", self.enchant_level);
    }
}

// =============================================================================
// YENDOR SHARDS - End-game meta progression
// =============================================================================

/// Types of Yendor shards for the matrix
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ShardType {
    Gold,     // +10% gold
    Power,    // +5% damage
    Vitality, // +10 max HP
    Speed,    // +5% attack speed
    Fortune,  // +5% item drops
    Vampiric, // +2% life steal
}

impl ShardType {
    pub fn all() -> &'static [ShardType] {
        &[
            ShardType::Gold,
            ShardType::Power,
            ShardType::Vitality,
            ShardType::Speed,
            ShardType::Fortune,
            ShardType::Vampiric,
        ]
    }

    pub fn char(&self) -> char {
        match self {
            ShardType::Gold => '$',
            ShardType::Power => '!',
            ShardType::Vitality => '♥',
            ShardType::Speed => '>',
            ShardType::Fortune => '?',
            ShardType::Vampiric => '%',
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ShardType::Gold => "Greed",
            ShardType::Power => "Power",
            ShardType::Vitality => "Vitality",
            ShardType::Speed => "Haste",
            ShardType::Fortune => "Fortune",
            ShardType::Vampiric => "Vampirism",
        }
    }

    /// Get a random shard
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        let all = Self::all();
        all[rng.gen_range(0..all.len())]
    }
}

/// Yendor shard synergies when adjacent
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShardSynergy {
    Avarice,   // Gold + Gold: Enemies drop gold on hit
    Bloodlust, // Power + Vampiric: Heal on kill
    Fortress,  // Vitality + Vitality: +5 armor
    Fury,      // Speed + Power: Crits deal 3x
    Treasure,  // Fortune + Gold: Double item drops
}

impl ShardSynergy {
    pub fn name(&self) -> &'static str {
        match self {
            ShardSynergy::Avarice => "Avarice",
            ShardSynergy::Bloodlust => "Bloodlust",
            ShardSynergy::Fortress => "Fortress",
            ShardSynergy::Fury => "Fury",
            ShardSynergy::Treasure => "Treasure Hunter",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ShardSynergy::Avarice => "Enemies drop gold on every hit",
            ShardSynergy::Bloodlust => "Heal 5 HP on each kill",
            ShardSynergy::Fortress => "+5 armor from shard power",
            ShardSynergy::Fury => "Critical hits deal 3x damage",
            ShardSynergy::Treasure => "Double item drop chance",
        }
    }
}

// =============================================================================
// MONSTER ZOO - DPS check events
// =============================================================================

/// Monster Zoo event state
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ZooEvent {
    pub active: bool,
    pub monsters_remaining: i32,
    pub time_remaining: u32, // Ticks
    pub reward_pending: bool,
}

impl ZooEvent {
    pub fn start() -> Self {
        Self {
            active: true,
            monsters_remaining: 20, // Clear 20 monsters
            time_remaining: 200,    // 10 seconds at 20 ticks/sec
            reward_pending: false,
        }
    }

    pub fn tick(&mut self) {
        if self.active && self.time_remaining > 0 {
            self.time_remaining -= 1;
            if self.time_remaining == 0 && self.monsters_remaining > 0 {
                // Failed the zoo
                self.active = false;
            }
        }
    }

    pub fn monster_killed(&mut self) {
        if self.active {
            self.monsters_remaining -= 1;
            if self.monsters_remaining <= 0 {
                // Completed!
                self.active = false;
                self.reward_pending = true;
            }
        }
    }
}

// =============================================================================
// PARTICLE EFFECTS - Visual juice on crits
// =============================================================================

/// Particle effect for visual feedback
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Particle {
    pub x: i32,
    pub y: i32,
    pub char: char,
    pub color_idx: u8,
    pub life: u8, // Frames remaining
}

impl Particle {
    pub fn spawn_crit(x: i32, y: i32) -> Vec<Self> {
        let mut rng = rand::thread_rng();
        let chars = ['*', '^', '\'', '!', '+'];
        let mut particles = Vec::new();

        for _ in 0..5 {
            particles.push(Self {
                x: x + rng.gen_range(-2..=2),
                y: y + rng.gen_range(-1..=1),
                char: chars[rng.gen_range(0..chars.len())],
                color_idx: if rng.gen_bool(0.5) { 1 } else { 3 }, // Yellow or red
                life: rng.gen_range(2..5),
            });
        }
        particles
    }

    pub fn tick(&mut self) -> bool {
        if self.life > 0 {
            self.life -= 1;
            true // Still alive
        } else {
            false // Dead
        }
    }
}

/// Elite monster affixes (Diablo-style)
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum MonsterAffix {
    Fast,     // Attacks more frequently
    Deadly,   // Higher damage
    Tough,    // More HP
    Rich,     // Drops more gold
    Vampiric, // Heals when hitting player
}

impl MonsterAffix {
    fn name(&self) -> &'static str {
        match self {
            MonsterAffix::Fast => "Fast",
            MonsterAffix::Deadly => "Deadly",
            MonsterAffix::Tough => "Tough",
            MonsterAffix::Rich => "Rich",
            MonsterAffix::Vampiric => "Vampiric",
        }
    }
}

/// A monster in the dungeon
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Monster {
    pub name: String,
    pub char: char,
    pub description: String,
    pub hp: i32,
    pub max_hp: i32,
    pub damage: i32,
    pub gold: i32,
    pub xp: i32,
    pub is_boss: bool,
    pub is_floor_boss: bool,        // Super boss at floor 10, 20, 30...
    pub affixes: Vec<MonsterAffix>, // Elite monster modifiers
}

impl Monster {
    fn spawn(level: i32, force_boss: bool) -> Self {
        let mut rng = rand::thread_rng();
        // Higher level = access to tougher monsters
        let max_idx = ((level as usize) + 2).min(ENEMIES.len());
        let idx = if force_boss {
            // Bosses are from the higher tier
            (max_idx - 1).max(0)
        } else {
            rng.gen_range(0..max_idx)
        };
        let (name, ch, description) = ENEMIES[idx];

        // EXPONENTIAL scaling: monsters get much tougher on higher floors
        // This prevents "unkillable" builds - eventually monsters will overwhelm
        let tier = idx as i32 + 1;
        let floor_mult = 1.15_f64.powi(level); // 15% harder per floor (compounds!)

        // Base stats scale with tier, then multiply by exponential floor factor
        let base_hp = (tier * 8) + (level * 3) + rng.gen_range(0..10);
        let base_damage = (tier) + 2 + (level / 2);
        let mut hp = (base_hp as f64 * floor_mult) as i32;
        let mut damage = ((base_damage as f64 * floor_mult * 0.7) as i32).max(1);
        let mut gold = tier * 3 + rng.gen_range(1..=tier * 3);
        let mut xp = tier * 3 + level * 2;

        // Elite chance: 10% per floor, max 50%
        let elite_chance = (level * 10).min(50);
        let mut affixes = Vec::new();
        if !force_boss && rng.gen_range(0..100) < elite_chance {
            // Add 1-2 random affixes
            let affix_count = if rng.gen_bool(0.3) { 2 } else { 1 };
            let all_affixes = [
                MonsterAffix::Fast,
                MonsterAffix::Deadly,
                MonsterAffix::Tough,
                MonsterAffix::Rich,
                MonsterAffix::Vampiric,
            ];
            for _ in 0..affix_count {
                let affix = all_affixes[rng.gen_range(0..all_affixes.len())];
                if !affixes.contains(&affix) {
                    affixes.push(affix);
                }
            }

            // Apply affix modifiers
            for affix in &affixes {
                match affix {
                    MonsterAffix::Tough => hp = (hp as f64 * 1.5) as i32,
                    MonsterAffix::Deadly => damage = (damage as f64 * 1.5) as i32,
                    MonsterAffix::Rich => gold *= 3,
                    _ => {}
                }
            }
            xp = (xp as f64 * (1.0 + 0.5 * affixes.len() as f64)) as i32;
        }

        // Boss multipliers
        if force_boss {
            hp = (hp as f64 * 2.5) as i32;
            damage = (damage as f64 * 1.8) as i32;
            gold *= 6;
            xp *= 4;
        }

        // Build name with affixes
        let mut final_name = name.to_string();
        if !affixes.is_empty() {
            let affix_str: String = affixes
                .iter()
                .map(|a| a.name())
                .collect::<Vec<_>>()
                .join(" ");
            final_name = format!("{} {}", affix_str, name);
        }
        if force_boss {
            final_name = format!("Boss {}", final_name);
        }

        Self {
            name: final_name,
            char: if force_boss {
                ch.to_ascii_uppercase()
            } else {
                ch
            },
            description: description.to_string(),
            hp,
            max_hp: hp,
            damage,
            gold,
            xp,
            is_boss: force_boss,
            is_floor_boss: false,
            affixes,
        }
    }

    /// Spawn a floor boss (every 10 floors)
    fn spawn_floor_boss(floor: i32) -> Self {
        // Floor bosses use the highest tier monster
        let tier = ((floor / 10) as usize).min(ENEMIES.len() - 1);
        let (base_name, ch, description) = ENEMIES[tier.max(ENEMIES.len() / 2)];

        // Massive scaling for floor bosses
        let floor_mult = 1.2_f64.powi(floor); // 20% per floor!
        let base_hp = 200 + floor * 50;
        let base_damage = 20 + floor * 5;

        let hp = (base_hp as f64 * floor_mult) as i32;
        let damage = (base_damage as f64 * floor_mult * 0.6) as i32;
        let gold = 100 * (floor / 10 + 1);
        let xp = 50 * (floor / 10 + 1);

        // Floor bosses have special titles
        let floor_tier = floor / 10;
        let title = match floor_tier {
            1 => "Guardian",
            2 => "Champion",
            3 => "Warlord",
            4 => "Overlord",
            5 => "Tyrant",
            6 => "Ancient",
            7 => "Elder",
            8 => "Prime",
            9 => "Archon",
            _ => "Eternal",
        };

        Self {
            name: format!("{} {} (Floor {})", title, base_name, floor),
            char: ch.to_ascii_uppercase(),
            description: format!("Floor {} Boss - {}", floor, description),
            hp,
            max_hp: hp,
            damage,
            gold,
            xp,
            is_boss: true,
            is_floor_boss: true,
            affixes: vec![MonsterAffix::Tough, MonsterAffix::Deadly], // All floor bosses are tough and deadly
        }
    }
}

/// Equipment that can drop from monsters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Equipment {
    pub name: String,
    pub bonus: i32,
    pub is_weapon: bool, // true = weapon (+STR), false = armor (+ARM)
}

// ============================================================================
// INVENTORY SYSTEM
// ============================================================================

/// Potion types with their effects
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum PotionType {
    Healing,       // Restore 50% max HP
    Strength,      // +10 STR for 30 ticks
    Speed,         // 2x auto-attack for 30 ticks
    GiantStrength, // +25 STR for 20 ticks
    Poison,        // Deal 50% of monster's max HP as damage
}

impl PotionType {
    pub fn name(&self) -> &'static str {
        match self {
            PotionType::Healing => "Healing",
            PotionType::Strength => "Strength",
            PotionType::Speed => "Speed",
            PotionType::GiantStrength => "Giant Str",
            PotionType::Poison => "Poison",
        }
    }

    pub fn char(&self) -> char {
        '!'
    }

    pub fn description(&self) -> &'static str {
        match self {
            PotionType::Healing => "Restore 50% HP",
            PotionType::Strength => "+10 STR (30 ticks)",
            PotionType::Speed => "2x attack speed (30 ticks)",
            PotionType::GiantStrength => "+25 STR (20 ticks)",
            PotionType::Poison => "Poison enemy for 50% HP",
        }
    }
}

/// Scroll types with powerful one-time effects
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ScrollType {
    Destruction, // Deal 100 damage to current enemy
    Enchant,     // +3 to weapon or armor permanently
    GoldRush,    // 3x gold from next 5 kills
    Teleport,    // Skip 5 monsters, gain dungeon level
    MagicMap,    // Reveal next boss type
}

impl ScrollType {
    pub fn name(&self) -> &'static str {
        match self {
            ScrollType::Destruction => "Destruct",
            ScrollType::Enchant => "Enchant",
            ScrollType::GoldRush => "Gold Rush",
            ScrollType::Teleport => "Teleport",
            ScrollType::MagicMap => "Magic Map",
        }
    }

    pub fn char(&self) -> char {
        '?'
    }

    pub fn description(&self) -> &'static str {
        match self {
            ScrollType::Destruction => "Deal 100 damage",
            ScrollType::Enchant => "+3 to weapon/armor",
            ScrollType::GoldRush => "3x gold (5 kills)",
            ScrollType::Teleport => "Skip 5 monsters",
            ScrollType::MagicMap => "Reveal boss type",
        }
    }
}

/// Ring types with passive effects when equipped
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum RingType {
    Protection,   // +5 ARM
    Strength,     // +5 STR
    Regeneration, // +1 HP per 10 ticks
    Wealth,       // +50% gold drops
    Vampirism,    // +10% life steal
}

impl RingType {
    pub fn name(&self) -> &'static str {
        match self {
            RingType::Protection => "Protection",
            RingType::Strength => "Strength",
            RingType::Regeneration => "Regen",
            RingType::Wealth => "Wealth",
            RingType::Vampirism => "Vampirism",
        }
    }

    pub fn char(&self) -> char {
        '='
    }

    pub fn description(&self) -> &'static str {
        match self {
            RingType::Protection => "+5 ARM while worn",
            RingType::Strength => "+5 STR while worn",
            RingType::Regeneration => "+1 HP per 10 ticks",
            RingType::Wealth => "+50% gold drops",
            RingType::Vampirism => "+10% life steal",
        }
    }
}

/// Wand types with limited charges
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum WandType {
    Fire,      // 30 damage
    Lightning, // 50 damage
    Ice,       // Halve enemy damage for 10 ticks
    Polymorph, // Transform enemy to random weaker type
}

impl WandType {
    pub fn name(&self) -> &'static str {
        match self {
            WandType::Fire => "Fire",
            WandType::Lightning => "Lightning",
            WandType::Ice => "Ice",
            WandType::Polymorph => "Polymorph",
        }
    }

    pub fn char(&self) -> char {
        '/'
    }

    pub fn description(&self) -> &'static str {
        match self {
            WandType::Fire => "30 fire damage",
            WandType::Lightning => "50 lightning damage",
            WandType::Ice => "Halve enemy dmg (10 ticks)",
            WandType::Polymorph => "Transform enemy",
        }
    }
}

/// An item in the inventory
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Item {
    Potion(PotionType),
    Scroll(ScrollType),
    Ring(RingType),
    Wand(WandType, u8), // type + remaining charges
}

impl Item {
    pub fn name(&self) -> String {
        match self {
            Item::Potion(p) => format!("{} Potion", p.name()),
            Item::Scroll(s) => format!("Scroll of {}", s.name()),
            Item::Ring(r) => format!("Ring of {}", r.name()),
            Item::Wand(w, charges) => format!("Wand of {} [{}]", w.name(), charges),
        }
    }

    pub fn char(&self) -> char {
        match self {
            Item::Potion(p) => p.char(),
            Item::Scroll(s) => s.char(),
            Item::Ring(r) => r.char(),
            Item::Wand(w, _) => w.char(),
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Item::Potion(p) => p.description(),
            Item::Scroll(s) => s.description(),
            Item::Ring(r) => r.description(),
            Item::Wand(w, _) => w.description(),
        }
    }

    /// Generate a random item based on floor level
    pub fn random(floor: i32) -> Self {
        let mut rng = rand::thread_rng();

        // Higher floors = more rare items
        let roll = rng.gen_range(0..100);

        if roll < 40 {
            // 40% potion
            let potions = [
                PotionType::Healing,
                PotionType::Strength,
                PotionType::Speed,
                PotionType::GiantStrength,
                PotionType::Poison,
            ];
            let max_idx = (floor as usize / 2 + 2).min(potions.len());
            let idx = rng.gen_range(0..max_idx);
            Item::Potion(potions[idx])
        } else if roll < 65 {
            // 25% scroll
            let scrolls = [
                ScrollType::Destruction,
                ScrollType::Enchant,
                ScrollType::GoldRush,
                ScrollType::Teleport,
                ScrollType::MagicMap,
            ];
            let max_idx = (floor as usize / 3 + 2).min(scrolls.len());
            let idx = rng.gen_range(0..max_idx);
            Item::Scroll(scrolls[idx])
        } else if roll < 85 {
            // 20% ring
            let rings = [
                RingType::Protection,
                RingType::Strength,
                RingType::Regeneration,
                RingType::Wealth,
                RingType::Vampirism,
            ];
            let idx = rng.gen_range(0..rings.len());
            Item::Ring(rings[idx])
        } else {
            // 15% wand
            let wands = [
                WandType::Fire,
                WandType::Lightning,
                WandType::Ice,
                WandType::Polymorph,
            ];
            let idx = rng.gen_range(0..wands.len());
            let charges = rng.gen_range(3..=5);
            Item::Wand(wands[idx], charges)
        }
    }
}

/// Active buff with remaining duration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Buff {
    Strength(i32, u32), // bonus amount, ticks remaining
    Speed(u32),         // ticks remaining (2x attack speed)
    GoldRush(u8),       // kills remaining with 3x gold
    IceSlow(u32),       // ticks remaining (enemy half damage)
}

impl Buff {
    pub fn name(&self) -> &'static str {
        match self {
            Buff::Strength(_, _) => "STR+",
            Buff::Speed(_) => "SPEED",
            Buff::GoldRush(_) => "GOLD",
            Buff::IceSlow(_) => "ICE",
        }
    }
}

// ============================================================================
// SOUL / PRESTIGE SYSTEM
// ============================================================================

/// Soul upgrades - permanent bonuses across runs
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SoulUpgrade {
    StartingStr,        // +1 starting STR per level
    StartingArm,        // +1 starting ARM per level
    StartingGold,       // +50 starting gold per level
    AttackSpeed,        // +10% attack speed per level (permanent!)
    CritDamage,         // +50% crit multiplier (2x → 2.5x → 3x...)
    GoldMultiplier,     // +25% gold from all sources
    ItemDropRate,       // +3% item drop chance
    StartingHp,         // +10 starting HP per level
    FloorSkip,          // Start on floor 1 + level
    AutoEatThreshold,   // +10% auto-eat HP threshold (50% → 60% → 70% → 80%)
    StartWithAutoHit,   // Start runs with auto-attack unlocked
    StartWithAutoEat,   // Start runs with auto-eat unlocked
    StartWithAutoQuaff, // Start runs with auto-quaff unlocked
    StartWithAutoEquip, // Start runs with auto-equip unlocked
}

impl SoulUpgrade {
    pub fn all() -> &'static [SoulUpgrade] {
        &[
            SoulUpgrade::AttackSpeed,
            SoulUpgrade::AutoEatThreshold,
            SoulUpgrade::StartWithAutoHit,
            SoulUpgrade::StartWithAutoEat,
            SoulUpgrade::StartWithAutoQuaff,
            SoulUpgrade::StartWithAutoEquip,
            SoulUpgrade::StartingStr,
            SoulUpgrade::StartingArm,
            SoulUpgrade::StartingHp,
            SoulUpgrade::StartingGold,
            SoulUpgrade::CritDamage,
            SoulUpgrade::GoldMultiplier,
            SoulUpgrade::ItemDropRate,
            SoulUpgrade::FloorSkip,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            SoulUpgrade::StartingStr => "Soul Strength",
            SoulUpgrade::StartingArm => "Soul Armor",
            SoulUpgrade::StartingGold => "Soul Wealth",
            SoulUpgrade::AttackSpeed => "Soul Speed",
            SoulUpgrade::CritDamage => "Soul Fury",
            SoulUpgrade::GoldMultiplier => "Soul Greed",
            SoulUpgrade::ItemDropRate => "Soul Fortune",
            SoulUpgrade::StartingHp => "Soul Vitality",
            SoulUpgrade::FloorSkip => "Soul Warp",
            SoulUpgrade::AutoEatThreshold => "Soul Gluttony",
            SoulUpgrade::StartWithAutoHit => "Innate Fury",
            SoulUpgrade::StartWithAutoEat => "Innate Hunger",
            SoulUpgrade::StartWithAutoQuaff => "Innate Thirst",
            SoulUpgrade::StartWithAutoEquip => "Innate Style",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            SoulUpgrade::StartingStr => "+1 starting STR",
            SoulUpgrade::StartingArm => "+1 starting ARM",
            SoulUpgrade::StartingGold => "+50 starting gold",
            SoulUpgrade::AttackSpeed => "+10% attack speed (permanent!)",
            SoulUpgrade::CritDamage => "+50% crit damage (2x→2.5x→3x...)",
            SoulUpgrade::GoldMultiplier => "+25% gold from all sources",
            SoulUpgrade::ItemDropRate => "+3% item drop chance",
            SoulUpgrade::StartingHp => "+10 starting HP",
            SoulUpgrade::FloorSkip => "Start 1 floor deeper",
            SoulUpgrade::AutoEatThreshold => "+10% auto-eat threshold",
            SoulUpgrade::StartWithAutoHit => "Start with Auto-Hit unlocked",
            SoulUpgrade::StartWithAutoEat => "Start with Auto-Eat unlocked",
            SoulUpgrade::StartWithAutoQuaff => "Start with Auto-Quaff unlocked",
            SoulUpgrade::StartWithAutoEquip => "Start with Auto-Equip unlocked",
        }
    }

    pub fn base_cost(&self) -> i32 {
        match self {
            SoulUpgrade::StartingStr => 15,
            SoulUpgrade::StartingArm => 15,
            SoulUpgrade::StartingGold => 10,
            SoulUpgrade::AttackSpeed => 30,
            SoulUpgrade::CritDamage => 50,
            SoulUpgrade::GoldMultiplier => 40,
            SoulUpgrade::ItemDropRate => 25,
            SoulUpgrade::StartingHp => 20,
            SoulUpgrade::FloorSkip => 100,
            SoulUpgrade::AutoEatThreshold => 75,
            SoulUpgrade::StartWithAutoHit => 150, // One-time unlocks are expensive
            SoulUpgrade::StartWithAutoEat => 200,
            SoulUpgrade::StartWithAutoQuaff => 250,
            SoulUpgrade::StartWithAutoEquip => 300,
        }
    }

    pub fn max_level(&self) -> i32 {
        match self {
            SoulUpgrade::AttackSpeed => 10,     // Max +100% speed (2x)
            SoulUpgrade::CritDamage => 8,       // Max 6x crit damage
            SoulUpgrade::FloorSkip => 10,       // Max start on floor 11
            SoulUpgrade::AutoEatThreshold => 3, // Max 80% (50+10+10+10)
            SoulUpgrade::StartWithAutoHit => 1, // One-time purchase
            SoulUpgrade::StartWithAutoEat => 1,
            SoulUpgrade::StartWithAutoQuaff => 1,
            SoulUpgrade::StartWithAutoEquip => 1,
            _ => 99, // Effectively unlimited
        }
    }
}

/// Persistent soul data (survives across runs)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SoulData {
    pub total_souls: i64,
    pub souls_earned_this_run: i64,

    // Upgrade levels
    pub starting_str: i32,
    pub starting_arm: i32,
    pub starting_gold: i32,
    pub attack_speed: i32,    // Each level = +10% speed
    pub crit_damage: i32,     // Each level = +50% crit mult
    pub gold_multiplier: i32, // Each level = +25% gold
    pub item_drop_rate: i32,  // Each level = +3% drop
    pub starting_hp: i32,
    pub floor_skip: i32,
    pub auto_eat_threshold: i32, // Each level = +10% (0=50%, 1=60%, 2=70%, 3=80%)

    // Start-with unlocks (boolean, 0 or 1)
    pub start_with_auto_hit: i32,
    pub start_with_auto_eat: i32,
    pub start_with_auto_quaff: i32,
    pub start_with_auto_equip: i32,

    // === TRANSMUTATION SYSTEM ===
    pub dust: i64,                  // Arcane Dust currency
    pub heirloom: Option<Heirloom>, // Persistent weapon

    // === ALCHEMY MASTERY ===
    pub alchemy_xp: i64,
    pub alchemy_level: i32,

    // === ASCENSION CLASSES ===
    pub selected_class: AscensionClass,
    pub unlocked_classes: Vec<AscensionClass>,

    // === YENDOR SHARDS ===
    pub yendor_shards: i32,
    pub shard_grid: [[Option<ShardType>; 3]; 3],

    // Stats for fun
    pub total_runs: i32,
    pub total_deaths: i32,
    pub best_floor: i32,
    pub total_monsters_killed: i64,
    pub total_gold_earned: i64,
    pub total_zoo_cleared: i32,
}

impl SoulData {
    pub fn upgrade_level(&self, upgrade: SoulUpgrade) -> i32 {
        match upgrade {
            SoulUpgrade::StartingStr => self.starting_str,
            SoulUpgrade::StartingArm => self.starting_arm,
            SoulUpgrade::StartingGold => self.starting_gold,
            SoulUpgrade::AttackSpeed => self.attack_speed,
            SoulUpgrade::CritDamage => self.crit_damage,
            SoulUpgrade::GoldMultiplier => self.gold_multiplier,
            SoulUpgrade::ItemDropRate => self.item_drop_rate,
            SoulUpgrade::StartingHp => self.starting_hp,
            SoulUpgrade::FloorSkip => self.floor_skip,
            SoulUpgrade::AutoEatThreshold => self.auto_eat_threshold,
            SoulUpgrade::StartWithAutoHit => self.start_with_auto_hit,
            SoulUpgrade::StartWithAutoEat => self.start_with_auto_eat,
            SoulUpgrade::StartWithAutoQuaff => self.start_with_auto_quaff,
            SoulUpgrade::StartWithAutoEquip => self.start_with_auto_equip,
        }
    }

    pub fn upgrade_cost(&self, upgrade: SoulUpgrade) -> i64 {
        let level = self.upgrade_level(upgrade);
        let base = upgrade.base_cost() as i64;
        // Cost scales: base * 1.5^level
        (base as f64 * 1.5_f64.powi(level)) as i64
    }

    pub fn can_afford(&self, upgrade: SoulUpgrade) -> bool {
        let level = self.upgrade_level(upgrade);
        if level >= upgrade.max_level() {
            return false;
        }
        self.total_souls >= self.upgrade_cost(upgrade)
    }

    pub fn buy_upgrade(&mut self, upgrade: SoulUpgrade) -> bool {
        if !self.can_afford(upgrade) {
            return false;
        }
        let cost = self.upgrade_cost(upgrade);
        self.total_souls -= cost;

        match upgrade {
            SoulUpgrade::StartingStr => self.starting_str += 1,
            SoulUpgrade::StartingArm => self.starting_arm += 1,
            SoulUpgrade::StartingGold => self.starting_gold += 1,
            SoulUpgrade::AttackSpeed => self.attack_speed += 1,
            SoulUpgrade::CritDamage => self.crit_damage += 1,
            SoulUpgrade::GoldMultiplier => self.gold_multiplier += 1,
            SoulUpgrade::ItemDropRate => self.item_drop_rate += 1,
            SoulUpgrade::StartingHp => self.starting_hp += 1,
            SoulUpgrade::FloorSkip => self.floor_skip += 1,
            SoulUpgrade::AutoEatThreshold => self.auto_eat_threshold += 1,
            SoulUpgrade::StartWithAutoHit => self.start_with_auto_hit = 1,
            SoulUpgrade::StartWithAutoEat => self.start_with_auto_eat = 1,
            SoulUpgrade::StartWithAutoQuaff => self.start_with_auto_quaff = 1,
            SoulUpgrade::StartWithAutoEquip => self.start_with_auto_equip = 1,
        }
        true
    }

    /// Calculate souls earned from a run
    pub fn calculate_souls(&self, floor: i32, monsters: i32, gold: i64, bosses: i32) -> i64 {
        let floor_bonus = (floor as i64) * 10;
        let monster_bonus = monsters as i64;
        let gold_bonus = gold / 100;
        let boss_bonus = (bosses as i64) * 50;
        floor_bonus + monster_bonus + gold_bonus + boss_bonus
    }

    /// Attack speed multiplier from soul upgrades (1.0 = normal, 2.0 = 2x speed)
    pub fn attack_speed_multiplier(&self) -> f32 {
        1.0 + (self.attack_speed as f32 * 0.1)
    }

    /// Crit damage multiplier (base 2.0, each level adds 0.5)
    pub fn crit_damage_multiplier(&self) -> f32 {
        2.0 + (self.crit_damage as f32 * 0.5)
    }

    /// Gold multiplier from soul upgrades
    pub fn soul_gold_multiplier(&self) -> i32 {
        self.gold_multiplier * 25
    }

    /// Item drop bonus from soul upgrades
    pub fn soul_drop_bonus(&self) -> i32 {
        self.item_drop_rate * 3
    }
}

/// View state for clicker (playing, dead, soul shop)
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum ClickerView {
    #[default]
    Playing,
    Dead,     // Show death screen with souls earned
    SoulShop, // Browse and buy soul upgrades
}

/// Shop items for upgrades
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ShopItem {
    Strength,
    Armor,
    Food,
    AutoAttack,
    CritChance,
    GoldMultiplier,
    LifeSteal,
    AutoEat,
    AutoQuaff,
    AutoEquip,
}

impl ShopItem {
    pub fn all() -> &'static [ShopItem] {
        &[
            ShopItem::Food,
            ShopItem::Strength,
            ShopItem::Armor,
            ShopItem::CritChance,
            ShopItem::GoldMultiplier,
            ShopItem::LifeSteal,
            ShopItem::AutoAttack,
            ShopItem::AutoEat,
            ShopItem::AutoQuaff,
            ShopItem::AutoEquip,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            ShopItem::Strength => "+1 STR",
            ShopItem::Armor => "+1 ARM",
            ShopItem::Food => "+10 Food",
            ShopItem::AutoAttack => "Auto-Hit",
            ShopItem::CritChance => "+5% Crit",
            ShopItem::GoldMultiplier => "+25% Gold",
            ShopItem::LifeSteal => "+5% Lifesteal",
            ShopItem::AutoEat => "Auto-Eat",
            ShopItem::AutoQuaff => "Auto-Quaff",
            ShopItem::AutoEquip => "Auto-Equip",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ShopItem::Strength => "Increase attack damage",
            ShopItem::Armor => "Reduce damage taken",
            ShopItem::Food => "Restore HP with 'e'",
            ShopItem::AutoAttack => "Attack automatically",
            ShopItem::CritChance => "Chance for 2x damage",
            ShopItem::GoldMultiplier => "More gold per kill",
            ShopItem::LifeSteal => "Heal from damage dealt",
            ShopItem::AutoEat => "Eat food automatically",
            ShopItem::AutoQuaff => "Auto-use potions smartly",
            ShopItem::AutoEquip => "Auto-equip better gear",
        }
    }

    pub fn base_cost(&self) -> i32 {
        match self {
            ShopItem::Strength => 25,
            ShopItem::Armor => 25,
            ShopItem::Food => 5,
            ShopItem::AutoAttack => 150,
            ShopItem::CritChance => 40,
            ShopItem::GoldMultiplier => 60,
            ShopItem::LifeSteal => 50,
            ShopItem::AutoEat => 200,
            ShopItem::AutoQuaff => 250,
            ShopItem::AutoEquip => 300,
        }
    }
}

/// Maximum inventory size
pub const INVENTORY_SIZE: usize = 8;

/// Equipment slot types
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum EquipSlot {
    Weapon,
    Armor,
    Helm,
    Amulet,
    Cloak,
    Gloves,
    Boots,
    Shield,
}

impl EquipSlot {
    pub fn name(&self) -> &'static str {
        match self {
            EquipSlot::Weapon => "Weapon",
            EquipSlot::Armor => "Armor",
            EquipSlot::Helm => "Helm",
            EquipSlot::Amulet => "Amulet",
            EquipSlot::Cloak => "Cloak",
            EquipSlot::Gloves => "Gloves",
            EquipSlot::Boots => "Boots",
            EquipSlot::Shield => "Shield",
        }
    }
}

/// Extended equipment with multiple stat bonuses
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Gear {
    pub name: String,
    pub slot: EquipSlot,
    pub str_bonus: i32,   // Attack damage
    pub arm_bonus: i32,   // Defense
    pub hp_bonus: i32,    // Max HP
    pub crit_bonus: i32,  // Crit chance %
    pub speed_bonus: i32, // Attack speed %
}

impl Gear {
    pub fn random(slot: EquipSlot, floor: i32) -> Self {
        let mut rng = rand::thread_rng();
        let tier = (floor / 3) + 1;

        let (prefixes, base_name) = match slot {
            EquipSlot::Weapon => (&["Rusty", "Sharp", "Fine", "Deadly", "Vorpal"][..], "mace"),
            EquipSlot::Armor => (&["Worn", "Studded", "Ring", "Chain", "Plate"][..], "mail"),
            EquipSlot::Helm => (
                &["Leather", "Iron", "Steel", "Mithril", "Crystal"][..],
                "helm",
            ),
            EquipSlot::Amulet => (
                &["Copper", "Silver", "Gold", "Jade", "Diamond"][..],
                "amulet",
            ),
            EquipSlot::Cloak => (
                &["Tattered", "Cloth", "Silk", "Shadow", "Elven"][..],
                "cloak",
            ),
            EquipSlot::Gloves => (
                &["Cloth", "Leather", "Chain", "Plate", "Dragon"][..],
                "gloves",
            ),
            EquipSlot::Boots => (
                &["Sandals", "Leather", "Iron", "Speed", "Elven"][..],
                "boots",
            ),
            EquipSlot::Shield => (
                &["Buckler", "Round", "Kite", "Tower", "Dragon"][..],
                "shield",
            ),
        };

        let prefix_idx = ((tier as usize) - 1).min(prefixes.len() - 1);
        let prefix = prefixes[prefix_idx];

        // Generate stats based on slot specialty
        let (str_bonus, arm_bonus, hp_bonus, crit_bonus, speed_bonus) = match slot {
            EquipSlot::Weapon => (
                tier + rng.gen_range(0..=tier),
                0,
                0,
                rng.gen_range(0..=2),
                0,
            ),
            EquipSlot::Armor => (0, tier + rng.gen_range(0..=tier), tier * 2, 0, 0),
            EquipSlot::Helm => (0, tier / 2, tier * 3, 0, 0),
            EquipSlot::Amulet => (tier / 2, tier / 2, tier, tier, tier),
            EquipSlot::Cloak => (0, tier / 2, 0, 0, tier + rng.gen_range(0..=tier)),
            EquipSlot::Gloves => (tier / 2, 0, 0, tier + rng.gen_range(0..=tier), 0),
            EquipSlot::Boots => (0, 0, 0, 0, tier * 2 + rng.gen_range(0..=tier)),
            EquipSlot::Shield => (0, tier + rng.gen_range(0..=tier * 2), tier, 0, -tier / 2),
        };

        let name = format!("{} {} (+{})", prefix, base_name, tier);

        Self {
            name,
            slot,
            str_bonus,
            arm_bonus,
            hp_bonus,
            crit_bonus,
            speed_bonus,
        }
    }

    /// Get total "power" for comparison
    pub fn power(&self) -> i32 {
        self.str_bonus + self.arm_bonus + self.hp_bonus / 3 + self.crit_bonus + self.speed_bonus
    }
}

/// Main clicker game state
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClickerState {
    // View state
    pub view: ClickerView,
    pub shop_selected: usize,
    pub inv_selected: usize,
    pub soul_shop_selected: usize,

    // Soul/Prestige system (persists across runs)
    pub souls: SoulData,

    // Player stats
    pub hp: i32,
    pub max_hp: i32,
    pub strength: i32,
    pub armor: i32,
    pub food: i32,
    pub level: i32,
    pub xp: i32,
    pub gold: i64,

    // Upgrades
    pub auto_attack: bool,
    pub auto_attack_timer: u32,
    pub auto_eat: bool,
    pub auto_eat_timer: u32,
    pub auto_quaff: bool, // Auto-use healing potions
    pub auto_quaff_timer: u32,
    pub auto_equip: bool,        // Auto-equip better gear
    pub auto_eat_threshold: i32, // HP% threshold for auto-eat (0=off, 20-80)
    pub crit_chance: i32,
    pub gold_multiplier: i32,
    pub life_steal: i32,

    // Equipment slots (old system kept for compatibility, new Gear system)
    pub weapon: Option<Equipment>,
    pub armor_equip: Option<Equipment>,
    // New equipment slots
    pub helm: Option<Gear>,
    pub amulet: Option<Gear>,
    pub cloak: Option<Gear>,
    pub gloves: Option<Gear>,
    pub boots: Option<Gear>,
    pub shield: Option<Gear>,

    // Inventory system
    pub inventory: Vec<Item>,
    pub ring_slots: [Option<RingType>; 2],
    pub buffs: Vec<Buff>,

    // Combat lanes (multi-monster combat)
    pub combat_lanes: usize,            // 1-10, increases with prestige
    pub monsters: Vec<Option<Monster>>, // One per lane

    // Dungeon state
    pub dungeon_level: i32,
    pub dungeon_floor: i32,
    pub floor_kills: i32,
    pub stairs_available: bool,
    pub monsters_killed: i32,
    pub bosses_killed: i32,
    pub total_gold_earned: i64,
    pub current_monster: Option<Monster>, // Kept for compatibility
    pub monster_queue: Vec<Monster>,
    pub kills_until_boss: i32,

    // Display
    pub floor: Vec<Scenery>,
    pub floors: Vec<Vec<Scenery>>, // Multiple floors for lanes
    pub message: Option<String>,
    pub tick: u32,
    pub last_crit: bool,
    pub last_drop: Option<String>,

    // Game state
    pub game_over: bool,

    // === NEW SYSTEMS ===
    // Biome (visual variety)
    pub biome: Biome,

    // Monster Zoo event
    pub zoo_event: ZooEvent,
    pub zoo_kill_counter: i32, // Triggers zoo every 100 kills

    // Particle effects
    pub particles: Vec<Particle>,

    // Transmutation filter (auto-transmute setting)
    pub transmute_filter: TransmuteFilter,
}

impl Default for ClickerState {
    fn default() -> Self {
        Self::new()
    }
}

impl ClickerState {
    pub fn new() -> Self {
        Self::new_with_souls(SoulData::default())
    }

    /// Create a new game with existing soul data (for prestige)
    pub fn new_with_souls(souls: SoulData) -> Self {
        let mut floor = Vec::with_capacity(60);
        for _ in 0..60 {
            floor.push(Scenery::random());
        }

        // Apply soul bonuses to starting stats
        let starting_hp = 25 + souls.starting_hp * 10;
        let starting_str = 5 + souls.starting_str;
        let starting_arm = 2 + souls.starting_arm;
        let starting_gold = (souls.starting_gold * 50) as i64;
        let starting_floor = 1 + souls.floor_skip;

        // Combat lanes based on prestige (1 + total_runs / 10, max 10)
        let combat_lanes = ((souls.total_runs / 10) + 1).min(10) as usize;

        // Create floors for each lane
        let mut floors = Vec::with_capacity(combat_lanes);
        for _ in 0..combat_lanes {
            let mut lane_floor = Vec::with_capacity(60);
            for _ in 0..60 {
                lane_floor.push(Scenery::random());
            }
            floors.push(lane_floor);
        }

        // Create monsters for each lane
        let monsters = vec![None; combat_lanes];

        let mut state = Self {
            // View state
            view: ClickerView::Playing,
            shop_selected: 0,
            inv_selected: 0,
            soul_shop_selected: 0,
            // Soul system
            souls,
            // Player stats (with soul bonuses)
            hp: starting_hp,
            max_hp: starting_hp,
            strength: starting_str,
            armor: starting_arm,
            food: 15,
            level: 1,
            xp: 0,
            gold: starting_gold,
            auto_attack: false,
            auto_attack_timer: 0,
            auto_eat: false,
            auto_eat_timer: 0,
            auto_quaff: false,
            auto_quaff_timer: 0,
            auto_equip: false,
            auto_eat_threshold: 0, // 0 = off, set to 50 when auto_eat purchased
            crit_chance: 5,
            gold_multiplier: 0,
            life_steal: 0,
            // Equipment
            weapon: None,
            armor_equip: None,
            helm: None,
            amulet: None,
            cloak: None,
            gloves: None,
            boots: None,
            shield: None,
            // Inventory system
            inventory: Vec::new(),
            ring_slots: [None, None],
            buffs: Vec::new(),
            // Combat lanes
            combat_lanes,
            monsters,
            // Dungeon state
            dungeon_level: starting_floor,
            dungeon_floor: starting_floor,
            floor_kills: 0,
            stairs_available: false,
            monsters_killed: 0,
            bosses_killed: 0,
            total_gold_earned: 0,
            current_monster: None,
            monster_queue: Vec::new(),
            kills_until_boss: 10,
            floor,
            floors,
            message: Some("Welcome to the dungeon! Press SPACE to attack!".to_string()),
            tick: 0,
            last_crit: false,
            last_drop: None,
            game_over: false,
            // New systems
            biome: Biome::from_floor(starting_floor),
            zoo_event: ZooEvent::default(),
            zoo_kill_counter: 0,
            particles: Vec::new(),
            transmute_filter: TransmuteFilter::Off,
        };

        // Apply ascension class bonuses
        state.apply_class_bonuses();

        // Apply start-with soul upgrades
        if state.souls.start_with_auto_hit > 0 {
            state.auto_attack = true;
        }
        if state.souls.start_with_auto_eat > 0 {
            state.auto_eat = true;
            // Set threshold based on soul upgrade level
            state.auto_eat_threshold = 50 + state.souls.auto_eat_threshold * 10;
        }
        if state.souls.start_with_auto_quaff > 0 {
            state.auto_quaff = true;
        }
        if state.souls.start_with_auto_equip > 0 {
            state.auto_equip = true;
        }

        state.spawn_all_monsters(false);
        state
    }

    pub fn reset(&mut self) {
        // Preserve soul data across resets
        let souls = self.souls.clone();
        *self = Self::new_with_souls(souls);
    }

    /// Reset with prestige - start fresh with current soul data
    /// Note: Souls are added in show_death_screen(), so we don't add them here
    pub fn prestige(&mut self) {
        // Track best floor and lifetime stats
        if self.dungeon_floor > self.souls.best_floor {
            self.souls.best_floor = self.dungeon_floor;
        }
        self.souls.total_monsters_killed += self.monsters_killed as i64;
        self.souls.total_gold_earned += self.total_gold_earned;

        // Clear souls earned this run (they've been claimed)
        self.souls.souls_earned_this_run = 0;

        // Reset with updated soul data
        let souls = self.souls.clone();
        *self = Self::new_with_souls(souls);
    }

    /// Total strength including all equipment, rings, heirloom, and shards
    pub fn total_strength(&self) -> i32 {
        let base = self.strength + self.weapon.as_ref().map_or(0, |w| w.bonus);
        let gear_bonus: i32 = [
            self.helm.as_ref().map_or(0, |g| g.str_bonus),
            self.amulet.as_ref().map_or(0, |g| g.str_bonus),
            self.cloak.as_ref().map_or(0, |g| g.str_bonus),
            self.gloves.as_ref().map_or(0, |g| g.str_bonus),
            self.boots.as_ref().map_or(0, |g| g.str_bonus),
            self.shield.as_ref().map_or(0, |g| g.str_bonus),
        ]
        .iter()
        .sum();
        let ring_bonus: i32 = self
            .ring_slots
            .iter()
            .filter_map(|r| match r {
                Some(RingType::Strength) => Some(5),
                _ => None,
            })
            .sum();
        let buff_bonus: i32 = self
            .buffs
            .iter()
            .filter_map(|b| match b {
                Buff::Strength(amt, _) => Some(*amt),
                _ => None,
            })
            .sum();
        let heirloom_bonus = self.heirloom_str_bonus();
        let (_, shard_damage, _, _, _, _) = self.shard_bonuses();
        base + gear_bonus + ring_bonus + buff_bonus + heirloom_bonus + shard_damage
    }

    /// Total armor including all equipment, rings, and shard synergies
    pub fn total_armor(&self) -> i32 {
        let base = self.armor + self.armor_equip.as_ref().map_or(0, |a| a.bonus);
        let gear_bonus: i32 = [
            self.helm.as_ref().map_or(0, |g| g.arm_bonus),
            self.amulet.as_ref().map_or(0, |g| g.arm_bonus),
            self.cloak.as_ref().map_or(0, |g| g.arm_bonus),
            self.gloves.as_ref().map_or(0, |g| g.arm_bonus),
            self.boots.as_ref().map_or(0, |g| g.arm_bonus),
            self.shield.as_ref().map_or(0, |g| g.arm_bonus),
        ]
        .iter()
        .sum();
        let ring_bonus: i32 = self
            .ring_slots
            .iter()
            .filter_map(|r| match r {
                Some(RingType::Protection) => Some(5),
                _ => None,
            })
            .sum();
        let shard_bonus = self.shard_armor_bonus();
        base + gear_bonus + ring_bonus + shard_bonus
    }

    /// Total crit chance including equipment and heirloom
    pub fn total_crit_chance(&self) -> i32 {
        let base = self.crit_chance;
        let gear_bonus: i32 = [
            self.helm.as_ref().map_or(0, |g| g.crit_bonus),
            self.amulet.as_ref().map_or(0, |g| g.crit_bonus),
            self.cloak.as_ref().map_or(0, |g| g.crit_bonus),
            self.gloves.as_ref().map_or(0, |g| g.crit_bonus),
            self.boots.as_ref().map_or(0, |g| g.crit_bonus),
            self.shield.as_ref().map_or(0, |g| g.crit_bonus),
        ]
        .iter()
        .sum();
        let heirloom_bonus = self.heirloom_crit_bonus();
        base + gear_bonus + heirloom_bonus
    }

    /// Total speed bonus from equipment (percentage)
    pub fn gear_speed_bonus(&self) -> i32 {
        [
            self.helm.as_ref().map_or(0, |g| g.speed_bonus),
            self.amulet.as_ref().map_or(0, |g| g.speed_bonus),
            self.cloak.as_ref().map_or(0, |g| g.speed_bonus),
            self.gloves.as_ref().map_or(0, |g| g.speed_bonus),
            self.boots.as_ref().map_or(0, |g| g.speed_bonus),
            self.shield.as_ref().map_or(0, |g| g.speed_bonus),
        ]
        .iter()
        .sum()
    }

    /// Total HP bonus from equipment
    pub fn gear_hp_bonus(&self) -> i32 {
        [
            self.helm.as_ref().map_or(0, |g| g.hp_bonus),
            self.amulet.as_ref().map_or(0, |g| g.hp_bonus),
            self.cloak.as_ref().map_or(0, |g| g.hp_bonus),
            self.gloves.as_ref().map_or(0, |g| g.hp_bonus),
            self.boots.as_ref().map_or(0, |g| g.hp_bonus),
            self.shield.as_ref().map_or(0, |g| g.hp_bonus),
        ]
        .iter()
        .sum()
    }

    /// Total gold multiplier including rings and shards
    pub fn total_gold_multiplier(&self) -> i32 {
        let base = self.gold_multiplier;
        let ring_bonus: i32 = self
            .ring_slots
            .iter()
            .filter_map(|r| match r {
                Some(RingType::Wealth) => Some(50),
                _ => None,
            })
            .sum();
        let buff_bonus: i32 = self
            .buffs
            .iter()
            .filter_map(|b| {
                match b {
                    Buff::GoldRush(_) => Some(200), // 3x = +200%
                    _ => None,
                }
            })
            .sum();
        let (shard_gold, _, _, _, _, _) = self.shard_bonuses();
        base + ring_bonus + buff_bonus + shard_gold
    }

    /// Total life steal including rings, heirloom, and shards
    pub fn total_life_steal(&self) -> i32 {
        let base = self.life_steal;
        let ring_bonus: i32 = self
            .ring_slots
            .iter()
            .filter_map(|r| match r {
                Some(RingType::Vampirism) => Some(10),
                _ => None,
            })
            .sum();
        let heirloom_bonus = self.heirloom_life_steal_bonus();
        let (_, _, _, _, _, shard_lifesteal) = self.shard_bonuses();
        base + ring_bonus + heirloom_bonus + shard_lifesteal
    }

    /// Check if speed buff is active (2x attack speed)
    pub fn has_speed_buff(&self) -> bool {
        self.buffs.iter().any(|b| matches!(b, Buff::Speed(_)))
    }

    /// Check if ice slow is active (enemy half damage)
    pub fn has_ice_slow(&self) -> bool {
        self.buffs.iter().any(|b| matches!(b, Buff::IceSlow(_)))
    }

    /// XP required for next level
    pub fn xp_for_level(&self) -> i32 {
        10 * self.level * self.level
    }

    /// Check for level up
    fn check_level_up(&mut self) {
        while self.xp >= self.xp_for_level() {
            self.xp -= self.xp_for_level();
            self.level += 1;
            self.max_hp += 5;
            self.hp = self.max_hp; // Full heal on level up
            self.strength += 1;
            self.message = Some(format!(
                "*** LEVEL UP! You are now level {}! ***",
                self.level
            ));
        }
    }

    /// Spawn a new monster (single lane - legacy)
    fn spawn_monster(&mut self, force_boss: bool) {
        self.current_monster = Some(Monster::spawn(self.dungeon_level, force_boss));
    }

    /// Spawn monsters in all combat lanes
    fn spawn_all_monsters(&mut self, force_boss: bool) {
        // Spawn in primary lane (legacy system)
        self.spawn_monster(force_boss);

        // Spawn in additional lanes
        for i in 0..self.combat_lanes {
            if self.monsters.get(i).is_some() && self.monsters[i].is_none() {
                self.monsters[i] = Some(Monster::spawn(self.dungeon_level, false));
            }
        }
    }

    /// Try to drop equipment from a killed monster
    fn try_drop_equipment(&mut self, monster: &Monster) {
        let mut rng = rand::thread_rng();

        // Base drop chance: 10%, bosses: 50%
        let drop_chance = if monster.is_boss { 50 } else { 10 };

        if rng.gen_range(0..100) < drop_chance {
            let is_weapon = rng.gen_bool(0.5);
            let tier = (self.dungeon_floor / 2) + 1;
            let bonus = tier + rng.gen_range(0..=tier);

            let prefixes = if is_weapon {
                &["Rusty", "Sharp", "Fine", "Deadly", "Vorpal"][..]
            } else {
                &["Worn", "Studded", "Ring", "Chain", "Plate"][..]
            };
            let prefix = prefixes[(tier as usize - 1).min(prefixes.len() - 1)];
            let base_name = if is_weapon { "mace" } else { "mail" };
            let name = format!("{} {} (+{})", prefix, base_name, bonus);

            let equip = Equipment {
                name: name.clone(),
                bonus,
                is_weapon,
            };

            // Auto-equip if better (and auto_equip enabled)
            if is_weapon {
                let current_bonus = self.weapon.as_ref().map_or(0, |w| w.bonus);
                if self.auto_equip && bonus > current_bonus {
                    self.weapon = Some(equip);
                    self.last_drop = Some(format!("Wielding {}!", name));
                } else {
                    let sell_value = bonus * 10;
                    self.gold += sell_value as i64;
                    self.last_drop = Some(format!("Sold {} for {} gold", name, sell_value));
                }
            } else {
                let current_bonus = self.armor_equip.as_ref().map_or(0, |a| a.bonus);
                if self.auto_equip && bonus > current_bonus {
                    self.armor_equip = Some(equip);
                    self.last_drop = Some(format!("Wearing {}!", name));
                } else {
                    let sell_value = bonus * 10;
                    self.gold += sell_value as i64;
                    self.last_drop = Some(format!("Sold {} for {} gold", name, sell_value));
                }
            }
        }
    }

    /// Try to drop gear (new equipment slots) from a killed monster
    fn try_drop_gear(&mut self, monster: &Monster) {
        let mut rng = rand::thread_rng();

        // Gear drop chance: 5% normal, 40% boss
        let drop_chance = if monster.is_boss { 40 } else { 5 };

        if rng.gen_range(0..100) < drop_chance {
            // Pick a random slot (excluding weapon/armor which use old system)
            let slots = [
                EquipSlot::Helm,
                EquipSlot::Amulet,
                EquipSlot::Cloak,
                EquipSlot::Gloves,
                EquipSlot::Boots,
                EquipSlot::Shield,
            ];
            let slot = slots[rng.gen_range(0..slots.len())];
            let gear = Gear::random(slot, self.dungeon_floor);

            // Get current gear in that slot
            let current = match slot {
                EquipSlot::Helm => &self.helm,
                EquipSlot::Amulet => &self.amulet,
                EquipSlot::Cloak => &self.cloak,
                EquipSlot::Gloves => &self.gloves,
                EquipSlot::Boots => &self.boots,
                EquipSlot::Shield => &self.shield,
                _ => return,
            };

            let current_power = current.as_ref().map_or(0, |g| g.power());
            let new_power = gear.power();

            if self.auto_equip && new_power > current_power {
                // Equip new gear
                let gear_name = gear.name.clone();
                match slot {
                    EquipSlot::Helm => self.helm = Some(gear),
                    EquipSlot::Amulet => self.amulet = Some(gear),
                    EquipSlot::Cloak => self.cloak = Some(gear),
                    EquipSlot::Gloves => self.gloves = Some(gear),
                    EquipSlot::Boots => self.boots = Some(gear),
                    EquipSlot::Shield => self.shield = Some(gear),
                    _ => {}
                }
                self.last_drop = Some(format!("Equipped {}!", gear_name));
            } else {
                // Sell for gold
                let sell_value = (new_power * 5 + 10) as i64;
                self.gold += sell_value;
                self.last_drop = Some(format!("Sold {} for {}g", gear.name, sell_value));
            }
        }
    }

    /// Try to drop an item from a killed monster
    fn try_drop_item(&mut self, monster: &Monster) {
        let mut rng = rand::thread_rng();

        // Item drop chance: 8% normal, 60% boss + soul bonus
        let soul_bonus = self.souls.soul_drop_bonus();
        let drop_chance = if monster.is_boss {
            60 + soul_bonus
        } else {
            8 + soul_bonus
        };

        if rng.gen_range(0..100) < drop_chance {
            let item = Item::random(self.dungeon_floor);
            self.add_item(item);
        }
    }

    /// Add an item to inventory (sells if full)
    pub fn add_item(&mut self, item: Item) {
        if self.inventory.len() < INVENTORY_SIZE {
            let name = item.name();
            self.inventory.push(item);
            self.last_drop = Some(format!("Found {}!", name));
        } else {
            // Inventory full - sell for gold
            let sell_value = 15 + self.dungeon_floor * 5;
            self.gold += sell_value as i64;
            self.last_drop = Some(format!(
                "Pack full! Sold {} for {}g",
                item.name(),
                sell_value
            ));
        }
    }

    /// Use an item from inventory by slot (0-7)
    pub fn use_item(&mut self, slot: usize) -> bool {
        if slot >= self.inventory.len() {
            return false;
        }

        let item = self.inventory[slot].clone();

        match item {
            Item::Potion(potion) => {
                match potion {
                    PotionType::Healing => {
                        let heal = self.max_hp / 2;
                        self.hp = (self.hp + heal).min(self.max_hp);
                        self.message = Some(format!(
                            "You quaff the potion. You feel better. (+{} Hp)",
                            heal
                        ));
                    }
                    PotionType::Strength => {
                        self.buffs.push(Buff::Strength(10, 600)); // 30 seconds
                        self.message = Some("You quaff the potion. You feel stronger!".to_string());
                    }
                    PotionType::Speed => {
                        self.buffs.push(Buff::Speed(600));
                        self.message = Some(
                            "You quaff the potion. You feel yourself moving faster!".to_string(),
                        );
                    }
                    PotionType::GiantStrength => {
                        self.buffs.push(Buff::Strength(25, 400)); // 20 seconds
                        self.message =
                            Some("You quaff the potion. You feel like a giant!".to_string());
                    }
                    PotionType::Poison => {
                        if let Some(ref mut monster) = self.current_monster {
                            let poison_dmg = monster.max_hp / 2;
                            monster.hp -= poison_dmg;
                            self.message = Some(format!(
                                "You throw the potion. The {} writhes in agony!",
                                monster.name
                            ));
                        } else {
                            self.message = Some("There is nothing to throw at.".to_string());
                            return false;
                        }
                    }
                }
                self.inventory.remove(slot);
                // Gain alchemy XP for using potion
                let xp_value = match potion {
                    PotionType::Healing => 5,
                    PotionType::Strength => 8,
                    PotionType::Speed => 8,
                    PotionType::GiantStrength => 12,
                    PotionType::Poison => 10,
                };
                self.gain_alchemy_xp(xp_value);
                true
            }
            Item::Scroll(scroll) => {
                match scroll {
                    ScrollType::Destruction => {
                        if let Some(ref mut monster) = self.current_monster {
                            monster.hp -= 100;
                            self.message = Some(format!(
                                "You read the scroll. The {} is hit by lightning!",
                                monster.name
                            ));
                        } else {
                            self.message = Some("There is nothing to destroy.".to_string());
                            return false;
                        }
                    }
                    ScrollType::Enchant => {
                        if let Some(ref mut w) = self.weapon {
                            w.bonus += 3;
                            let base = w.name.split(" (+").next().unwrap_or(&w.name).to_string();
                            w.name = format!("{} (+{})", base, w.bonus);
                            self.message =
                                Some(format!("Your {} glows blue for a moment.", w.name));
                        } else if let Some(ref mut a) = self.armor_equip {
                            a.bonus += 3;
                            let base = a.name.split(" (+").next().unwrap_or(&a.name).to_string();
                            a.name = format!("{} (+{})", base, a.bonus);
                            self.message =
                                Some(format!("Your {} glows blue for a moment.", a.name));
                        } else {
                            self.weapon = Some(Equipment {
                                name: "enchanted mace (+3)".to_string(),
                                bonus: 3,
                                is_weapon: true,
                            });
                            self.message = Some("A mace appears in your hands!".to_string());
                        }
                    }
                    ScrollType::GoldRush => {
                        self.buffs.push(Buff::GoldRush(5));
                        self.message = Some("You read the scroll. You feel greedy!".to_string());
                    }
                    ScrollType::Teleport => {
                        self.dungeon_floor += 1;
                        self.floor_kills = 0;
                        self.stairs_available = false;
                        self.monsters_killed += 5;
                        self.current_monster = None;
                        self.spawn_monster(false);
                        self.message = Some(format!(
                            "You read the scroll. You find yourself on level {}!",
                            self.dungeon_floor
                        ));
                    }
                    ScrollType::MagicMap => {
                        let boss_tier = (self.dungeon_floor as usize + 2).min(ENEMIES.len() - 1);
                        let (name, _, _) = ENEMIES[boss_tier];
                        self.message = Some(format!(
                            "You read the scroll. You sense a {} below...",
                            name
                        ));
                    }
                }
                self.inventory.remove(slot);
                true
            }
            Item::Ring(ring) => {
                self.equip_ring(ring);
                self.inventory.remove(slot);
                true
            }
            Item::Wand(wand, charges) => {
                if charges == 0 {
                    self.message = Some("The wand has no charges left.".to_string());
                    return false;
                }

                match wand {
                    WandType::Fire => {
                        if let Some(ref mut monster) = self.current_monster {
                            monster.hp -= 30;
                            self.message =
                                Some(format!("A bolt of fire hits the {}!", monster.name));
                        } else {
                            self.message = Some("The bolt hits the wall.".to_string());
                            return false;
                        }
                    }
                    WandType::Lightning => {
                        if let Some(ref mut monster) = self.current_monster {
                            monster.hp -= 50;
                            self.message =
                                Some(format!("A bolt of lightning hits the {}!", monster.name));
                        } else {
                            self.message = Some("The bolt hits the wall.".to_string());
                            return false;
                        }
                    }
                    WandType::Ice => {
                        self.buffs.push(Buff::IceSlow(200)); // 10 seconds
                        self.message = Some("The air grows cold around you.".to_string());
                    }
                    WandType::Polymorph => {
                        if self.current_monster.is_some() {
                            let weak_tier =
                                (self.dungeon_floor as usize / 2).min(ENEMIES.len() - 1);
                            let (name, ch, desc) = ENEMIES[weak_tier];
                            if let Some(ref mut monster) = self.current_monster {
                                let new_hp = monster.max_hp / 2;
                                monster.name = name.to_string();
                                monster.char = ch;
                                monster.description = desc.to_string();
                                monster.hp = new_hp;
                                monster.max_hp = new_hp;
                                monster.damage /= 2;
                            }
                            self.message = Some(format!("The monster turns into a {}!", name));
                        } else {
                            self.message = Some("There is nothing to transform.".to_string());
                            return false;
                        }
                    }
                }

                // Reduce charges
                if let Some(Item::Wand(_, ref mut c)) = self.inventory.get_mut(slot) {
                    *c -= 1;
                    if *c == 0 {
                        self.inventory.remove(slot);
                    }
                }
                true
            }
        }
    }

    /// Equip a ring (to first empty slot, or replace slot 0)
    pub fn equip_ring(&mut self, ring: RingType) {
        if self.ring_slots[0].is_none() {
            self.ring_slots[0] = Some(ring);
            self.message = Some(format!(
                "You are now wearing a ring of {}.",
                ring.name().to_lowercase()
            ));
        } else if self.ring_slots[1].is_none() {
            self.ring_slots[1] = Some(ring);
            self.message = Some(format!(
                "You are now wearing a ring of {}.",
                ring.name().to_lowercase()
            ));
        } else {
            let old = self.ring_slots[0].take();
            self.ring_slots[0] = Some(ring);
            if let Some(old_ring) = old {
                if self.inventory.len() < INVENTORY_SIZE {
                    self.inventory.push(Item::Ring(old_ring));
                    self.message = Some(format!(
                        "You put on the ring of {}. Old ring in pack.",
                        ring.name().to_lowercase()
                    ));
                } else {
                    self.gold += 25_i64;
                    self.message = Some(format!(
                        "You put on the ring of {}. Sold old ring.",
                        ring.name().to_lowercase()
                    ));
                }
            }
        }
    }

    /// Unequip a ring by slot (0 or 1)
    pub fn unequip_ring(&mut self, slot: usize) {
        if slot > 1 {
            return;
        }
        if let Some(ring) = self.ring_slots[slot].take() {
            if self.inventory.len() < INVENTORY_SIZE {
                self.inventory.push(Item::Ring(ring));
                self.message = Some(format!(
                    "You take off the ring of {}.",
                    ring.name().to_lowercase()
                ));
            } else {
                self.gold += 25_i64;
                self.message = Some(format!(
                    "Pack full. Sold ring of {} for 25g.",
                    ring.name().to_lowercase()
                ));
            }
        }
    }

    /// Descend stairs to next level
    pub fn descend_stairs(&mut self) {
        if !self.stairs_available {
            self.message = Some("You see no way down.".to_string());
            return;
        }

        let old_biome = self.biome;
        self.dungeon_floor += 1;
        self.floor_kills = 0;
        self.stairs_available = false;
        self.current_monster = None;

        // Update biome based on floor
        self.update_biome();
        let new_biome = self.biome;

        // Check for floor boss on milestone floors (10, 20, 30...)
        if self.dungeon_floor % 10 == 0 {
            self.current_monster = Some(Monster::spawn_floor_boss(self.dungeon_floor));
            if old_biome != new_biome {
                self.message = Some(format!(
                    "⚔ FLOOR {} BOSS in {}! ⚔ A powerful enemy blocks your path!",
                    self.dungeon_floor,
                    new_biome.name()
                ));
            } else {
                self.message = Some(format!(
                    "⚔ FLOOR {} BOSS! ⚔ A powerful enemy blocks your path!",
                    self.dungeon_floor
                ));
            }
        } else {
            self.spawn_monster(false);
            if old_biome != new_biome {
                self.message = Some(format!(
                    "You enter {}! Floor {}.",
                    new_biome.name(),
                    self.dungeon_floor
                ));
            } else {
                self.message = Some(format!("You descend to floor {}.", self.dungeon_floor));
            }
        }
    }

    /// Process buff timers (call from tick)
    fn process_buffs(&mut self) {
        self.buffs.retain_mut(|buff| match buff {
            Buff::Strength(_, ref mut ticks) => {
                *ticks = ticks.saturating_sub(1);
                *ticks > 0
            }
            Buff::Speed(ref mut ticks) => {
                *ticks = ticks.saturating_sub(1);
                *ticks > 0
            }
            Buff::GoldRush(kills) => *kills > 0,
            Buff::IceSlow(ref mut ticks) => {
                *ticks = ticks.saturating_sub(1);
                *ticks > 0
            }
        });

        // Ring of regeneration
        if self.tick.is_multiple_of(10) {
            let regen_count = self
                .ring_slots
                .iter()
                .filter(|r| matches!(r, Some(RingType::Regeneration)))
                .count();
            if regen_count > 0 {
                self.hp = (self.hp + regen_count as i32).min(self.max_hp);
            }
        }
    }

    /// Inventory navigation
    pub fn inv_next(&mut self) {
        if !self.inventory.is_empty() {
            self.inv_selected = (self.inv_selected + 1) % self.inventory.len();
        }
    }

    pub fn inv_prev(&mut self) {
        if !self.inventory.is_empty() {
            self.inv_selected =
                (self.inv_selected + self.inventory.len() - 1) % self.inventory.len();
        }
    }

    /// Use currently selected inventory item (quaff/read/zap/put on)
    pub fn use_selected(&mut self) {
        if !self.inventory.is_empty() {
            self.use_item(self.inv_selected);
            if self.inv_selected >= self.inventory.len() && !self.inventory.is_empty() {
                self.inv_selected = self.inventory.len() - 1;
            }
        }
    }

    /// Get cost of a shop item (scales with purchases)
    pub fn item_cost(&self, item: ShopItem) -> i32 {
        let base = item.base_cost();
        match item {
            ShopItem::Strength => base + (self.strength - 5) * 10,
            ShopItem::Armor => base + (self.armor - 2) * 10,
            ShopItem::Food => base,
            ShopItem::AutoAttack => {
                if self.auto_attack {
                    0
                } else {
                    base
                }
            }
            ShopItem::AutoEat => {
                if self.auto_eat {
                    0
                } else {
                    base
                }
            }
            ShopItem::AutoQuaff => {
                if self.auto_quaff {
                    0
                } else {
                    base
                }
            }
            ShopItem::AutoEquip => {
                if self.auto_equip {
                    0
                } else {
                    base
                }
            }
            ShopItem::CritChance => base + ((self.crit_chance - 5) / 5) * 20,
            ShopItem::GoldMultiplier => base + (self.gold_multiplier / 25) * 30,
            ShopItem::LifeSteal => base + (self.life_steal / 5) * 25,
        }
    }

    /// Can afford an item?
    pub fn can_afford(&self, item: ShopItem) -> bool {
        match item {
            ShopItem::AutoAttack if self.auto_attack => false,
            ShopItem::AutoEat if self.auto_eat => false,
            ShopItem::AutoQuaff if self.auto_quaff => false,
            ShopItem::AutoEquip if self.auto_equip => false,
            ShopItem::CritChance if self.crit_chance >= 50 => false, // Max 50%
            ShopItem::GoldMultiplier if self.gold_multiplier >= 200 => false, // Max 200%
            ShopItem::LifeSteal if self.life_steal >= 25 => false,   // Max 25%
            _ => self.gold >= self.item_cost(item) as i64,
        }
    }

    /// Is item maxed out?
    pub fn is_maxed(&self, item: ShopItem) -> bool {
        match item {
            ShopItem::AutoAttack => self.auto_attack,
            ShopItem::AutoEat => self.auto_eat,
            ShopItem::AutoQuaff => self.auto_quaff,
            ShopItem::AutoEquip => self.auto_equip,
            ShopItem::CritChance => self.crit_chance >= 50,
            ShopItem::GoldMultiplier => self.gold_multiplier >= 200,
            ShopItem::LifeSteal => self.life_steal >= 25,
            _ => false,
        }
    }

    /// Buy a shop item
    pub fn buy_item(&mut self, item: ShopItem) {
        if !self.can_afford(item) {
            return;
        }

        let cost = self.item_cost(item);
        self.gold -= cost as i64;

        match item {
            ShopItem::Strength => {
                self.strength += 1;
                self.message = Some(format!("STR increased to {}!", self.strength));
            }
            ShopItem::Armor => {
                self.armor += 1;
                self.message = Some(format!("ARM increased to {}!", self.armor));
            }
            ShopItem::Food => {
                self.food += 10;
                self.message = Some("Bought 10 food!".to_string());
            }
            ShopItem::AutoAttack => {
                self.auto_attack = true;
                self.message = Some("Auto-attack enabled!".to_string());
            }
            ShopItem::AutoEat => {
                self.auto_eat = true;
                // Set threshold based on soul upgrade level (50% base + 10% per level)
                self.auto_eat_threshold = 50 + self.souls.auto_eat_threshold * 10;
                self.message = Some(format!(
                    "Auto-eat enabled at {}% HP!",
                    self.auto_eat_threshold
                ));
            }
            ShopItem::AutoQuaff => {
                self.auto_quaff = true;
                self.message = Some("Auto-quaff enabled!".to_string());
            }
            ShopItem::AutoEquip => {
                self.auto_equip = true;
                self.message = Some("Auto-equip enabled!".to_string());
            }
            ShopItem::CritChance => {
                self.crit_chance += 5;
                self.message = Some(format!("Crit chance: {}%!", self.crit_chance));
            }
            ShopItem::GoldMultiplier => {
                self.gold_multiplier += 25;
                self.message = Some(format!("Gold bonus: +{}%!", self.gold_multiplier));
            }
            ShopItem::LifeSteal => {
                self.life_steal += 5;
                self.message = Some(format!("Life steal: {}%!", self.life_steal));
            }
        }
    }

    /// Hit the current monster
    pub fn hit(&mut self) {
        if self.game_over {
            return;
        }

        self.last_crit = false;
        self.last_drop = None;

        let mut rng = rand::thread_rng();

        // Get values before borrowing monster
        let total_str = self.total_strength();
        let total_arm = self.total_armor();
        let crit_chance = self.total_crit_chance();
        let life_steal = self.total_life_steal();
        let gold_mult = self.total_gold_multiplier();
        let has_ice = self.has_ice_slow();
        let has_fury = self.has_fury_synergy();
        let crit_mult = if has_fury {
            3.0 // Fury synergy: 3x crit damage
        } else {
            self.crit_damage_multiplier()
        };

        if let Some(ref mut monster) = self.current_monster {
            // Calculate player damage
            let base_damage = total_str + rng.gen_range(0..=total_str / 2);

            // Check for critical hit (with soul crit damage bonus)
            let is_crit = rng.gen_range(0..100) < crit_chance;
            let player_damage = if is_crit {
                self.last_crit = true;
                (base_damage as f32 * crit_mult) as i32
            } else {
                base_damage
            };

            monster.hp -= player_damage;

            // Life steal
            if life_steal > 0 {
                let heal = (player_damage * life_steal / 100).max(1);
                self.hp = (self.hp + heal).min(self.max_hp);
            }

            // Monster attacks player (reduced by armor, halved if ice slow)
            let mut monster_damage = (monster.damage - total_arm / 2).max(1);
            if has_ice {
                monster_damage /= 2;
            }
            let actual_damage = monster_damage + rng.gen_range(0..=monster_damage / 2);
            self.hp -= actual_damage;

            let crit_text = if is_crit { " CRITICAL!" } else { "" };
            self.message = Some(format!(
                "Hit {} for {}!{} Took {} damage.",
                monster.name, player_damage, crit_text, actual_damage
            ));

            // Check if monster died
            if monster.hp <= 0 {
                // Calculate gold with multiplier and exponential floor scaling
                // Gold scales exponentially: base × 1.5^floor
                let floor_mult = 1.5_f64.powi(self.dungeon_floor);
                let soul_gold_mult = self.souls.soul_gold_multiplier();
                let base_gold = monster.gold;
                let bonus_gold = base_gold * (gold_mult + soul_gold_mult) / 100;
                let total_gold = ((base_gold + bonus_gold) as f64 * floor_mult) as i64;

                let monster_name = monster.name.clone();
                let monster_xp = monster.xp;
                let monster_clone = monster.clone();
                let was_boss = monster.is_boss;

                self.gold += total_gold;
                self.total_gold_earned += total_gold;
                self.xp += monster_xp;
                self.monsters_killed += 1;
                self.floor_kills += 1;
                self.kills_until_boss -= 1;
                if was_boss {
                    self.bosses_killed += 1;
                }

                // Track zoo kills
                self.zoo_monster_killed();

                // Decrement gold rush kills if active
                for buff in &mut self.buffs {
                    if let Buff::GoldRush(ref mut kills) = buff {
                        *kills = kills.saturating_sub(1);
                    }
                }

                // Clear monster first, then do other operations
                self.current_monster = None;

                // Try to drop equipment, gear, and items
                self.try_drop_equipment(&monster_clone);
                self.try_drop_gear(&monster_clone);
                self.try_drop_item(&monster_clone);

                if let Some(ref drop_msg) = self.last_drop {
                    self.message = Some(format!(
                        "Killed {}! +{} gold, +{} xp. {}",
                        monster_name, total_gold, monster_xp, drop_msg
                    ));
                } else {
                    self.message = Some(format!(
                        "Killed {}! +{} gold, +{} xp",
                        monster_name, total_gold, monster_xp
                    ));
                }

                // Check for stairs (every 5 kills on floor, or after boss)
                if !self.stairs_available && (self.floor_kills >= 5 || was_boss) {
                    self.stairs_available = true;
                    self.message = Some(format!(
                        "Killed {}! +{} gold. You see stairs going down (>).",
                        monster_name, total_gold
                    ));
                }

                // Check for boss
                if self.kills_until_boss <= 0 {
                    self.dungeon_level += 1;
                    self.kills_until_boss = 10;
                    self.message = Some(format!(
                        "*** BOSS APPROACHING! Dungeon level {} ***",
                        self.dungeon_level
                    ));
                    self.spawn_monster(true);
                } else {
                    self.spawn_monster(false);
                }

                self.check_level_up();

                // Small chance to spawn extra monster
                if rng.gen_range(0..100) < 10 {
                    self.monster_queue
                        .push(Monster::spawn(self.dungeon_floor, false));
                }
            }
        }

        // Spawn crit particles if we had a crit
        if self.last_crit {
            self.spawn_crit_particles(40, 10);
        }

        // Check for death
        if self.hp <= 0 {
            self.game_over = true;
            self.souls.total_deaths += 1;
            self.message = Some(format!(
                "You died! Floor {}, killed {} monsters, level {}.",
                self.dungeon_floor, self.monsters_killed, self.level
            ));
        }
    }

    /// Eat food to restore HP
    pub fn eat(&mut self) {
        if self.game_over || self.food <= 0 {
            return;
        }

        self.food -= 1;
        let heal = 5 + self.level * 2;
        self.hp = (self.hp + heal).min(self.max_hp);
        self.message = Some(format!("Ate food, restored {} HP!", heal));
    }

    /// Hit a monster in a specific lane (for multi-lane combat)
    pub fn hit_lane(&mut self, lane: usize) {
        if self.game_over || lane >= self.combat_lanes || lane >= self.monsters.len() {
            return;
        }

        let mut rng = rand::thread_rng();
        let total_str = self.total_strength();
        let total_arm = self.total_armor();
        let crit_chance = self.total_crit_chance();
        let life_steal = self.total_life_steal();
        let gold_mult = self.total_gold_multiplier();
        let has_ice = self.has_ice_slow();
        let crit_mult = self.crit_damage_multiplier();

        if let Some(ref mut monster) = self.monsters[lane] {
            // Calculate player damage
            let base_damage = total_str + rng.gen_range(0..=total_str / 2);
            let is_crit = rng.gen_range(0..100) < crit_chance;
            let player_damage = if is_crit {
                (base_damage as f32 * crit_mult) as i32
            } else {
                base_damage
            };

            monster.hp -= player_damage;

            // Life steal
            if life_steal > 0 {
                let heal = (player_damage * life_steal / 100).max(1);
                self.hp = (self.hp + heal).min(self.max_hp);
            }

            // Monster attacks back (reduced by armor, halved if ice slow)
            let mut monster_damage = (monster.damage - total_arm / 2).max(1);
            if has_ice {
                monster_damage /= 2;
            }
            let actual_damage = monster_damage + rng.gen_range(0..=monster_damage / 2);
            self.hp -= actual_damage;

            // Check if monster died
            if monster.hp <= 0 {
                // Calculate gold with multiplier and exponential floor scaling
                let floor_mult = 1.5_f64.powi(self.dungeon_floor);
                let soul_gold_mult = self.souls.soul_gold_multiplier();
                let base_gold = monster.gold;
                let bonus_gold = base_gold * (gold_mult + soul_gold_mult) / 100;
                let total_gold = ((base_gold + bonus_gold) as f64 * floor_mult) as i64;

                let monster_xp = monster.xp;
                let was_boss = monster.is_boss;

                self.gold += total_gold;
                self.total_gold_earned += total_gold;
                self.xp += monster_xp;
                self.monsters_killed += 1;
                if was_boss {
                    self.bosses_killed += 1;
                }

                // Decrement gold rush kills if active
                for buff in &mut self.buffs {
                    if let Buff::GoldRush(ref mut kills) = buff {
                        *kills = kills.saturating_sub(1);
                    }
                }

                // Clear monster (will respawn in tick)
                self.monsters[lane] = None;

                self.check_level_up();
            }
        }

        // Check for death
        if self.hp <= 0 {
            self.game_over = true;
            self.souls.total_deaths += 1;
            self.message = Some(format!(
                "You died! Floor {}, killed {} monsters, level {}.",
                self.dungeon_floor, self.monsters_killed, self.level
            ));
        }
    }

    /// Hit all monsters in all lanes (for multi-lane auto-attack)
    pub fn hit_all_lanes(&mut self) {
        // Hit primary monster first
        self.hit();

        // Hit additional lanes
        for lane in 0..self.combat_lanes {
            if !self.game_over {
                self.hit_lane(lane);
            }
        }
    }

    /// Game tick (for auto-attack and animations)
    pub fn tick(&mut self) {
        if self.game_over {
            return;
        }

        self.tick += 1;

        // Process buffs
        self.process_buffs();

        // Auto-attack with soul speed bonus
        // Base interval: 20 ticks
        // Speed multiplier reduces this (1.5x speed = 13 ticks, 2x = 10 ticks)
        if self.auto_attack {
            self.auto_attack_timer += 1;
            let speed_mult = self.attack_speed_multiplier();
            let attack_interval = (20.0 / speed_mult) as u32;
            if self.auto_attack_timer >= attack_interval.max(1) {
                self.auto_attack_timer = 0;
                // Hit all lanes when multi-lane combat is active
                if self.combat_lanes > 1 {
                    self.hit_all_lanes();
                } else {
                    self.hit();
                }
            }
        }

        // Auto-eat when HP is below threshold
        if self.auto_eat && self.auto_eat_threshold > 0 && self.food > 0 {
            let threshold_hp = self.max_hp * self.auto_eat_threshold / 100;
            if self.hp < threshold_hp {
                self.auto_eat_timer += 1;
                if self.auto_eat_timer >= 40 {
                    self.auto_eat_timer = 0;
                    self.eat();
                }
            }
        }

        // Auto-quaff potions intelligently
        if self.auto_quaff {
            self.auto_quaff_timer += 1;
            if self.auto_quaff_timer >= 20 {
                self.auto_quaff_timer = 0;

                // Priority 1: Healing when HP is low (below 30%)
                if self.hp < self.max_hp * 3 / 10 {
                    if let Some(idx) = self
                        .inventory
                        .iter()
                        .position(|item| matches!(item, Item::Potion(PotionType::Healing)))
                    {
                        self.use_item(idx);
                        return; // One potion per tick
                    }
                }

                // Priority 2: Speed buff when in combat and no active speed buff
                if self.current_monster.is_some() && !self.has_speed_buff() {
                    if let Some(idx) = self
                        .inventory
                        .iter()
                        .position(|item| matches!(item, Item::Potion(PotionType::Speed)))
                    {
                        self.use_item(idx);
                        return;
                    }
                }

                // Priority 3: Strength buff when in combat and no active strength buff
                let has_str_buff = self.buffs.iter().any(|b| matches!(b, Buff::Strength(_, _)));
                if self.current_monster.is_some() && !has_str_buff {
                    if let Some(idx) = self.inventory.iter().position(|item| {
                        matches!(
                            item,
                            Item::Potion(PotionType::Strength | PotionType::GiantStrength)
                        )
                    }) {
                        self.use_item(idx);
                        return;
                    }
                }

                // Priority 4: Poison on bosses
                if let Some(ref monster) = self.current_monster {
                    if monster.is_boss && monster.hp > monster.max_hp / 2 {
                        if let Some(idx) = self
                            .inventory
                            .iter()
                            .position(|item| matches!(item, Item::Potion(PotionType::Poison)))
                        {
                            self.use_item(idx);
                        }
                    }
                }
            }
        }

        // Process monster queue
        if self.current_monster.is_none() && !self.monster_queue.is_empty() {
            self.current_monster = Some(self.monster_queue.remove(0));
        }

        // Respawn monsters in empty lanes
        for i in 0..self.combat_lanes {
            if i < self.monsters.len() && self.monsters[i].is_none() {
                self.monsters[i] = Some(Monster::spawn(self.dungeon_level, false));
            }
        }

        // Update floor display (scrolling dungeon effect)
        if self.tick.is_multiple_of(3) {
            self.floor.rotate_left(1);
            let last = self.floor.len() - 1;
            self.floor[last] = Scenery::random();

            // Scroll additional lane floors
            for lane_floor in &mut self.floors {
                if !lane_floor.is_empty() {
                    lane_floor.rotate_left(1);
                    let last_idx = lane_floor.len() - 1;
                    lane_floor[last_idx] = Scenery::random();
                }
            }
        }

        // === NEW SYSTEMS ===

        // Monster Zoo event processing
        self.tick_zoo();
        self.check_zoo_trigger();

        // Particle effects
        self.tick_particles();
    }

    /// Get current score (gold + xp + level bonus)
    pub fn score(&self) -> u32 {
        let score = self.gold
            + (self.xp as i64)
            + (self.level as i64 * 100)
            + (self.monsters_killed as i64 * 10);
        score.min(u32::MAX as i64) as u32
    }

    /// Shop navigation
    pub fn shop_next(&mut self) {
        let count = ShopItem::all().len();
        self.shop_selected = (self.shop_selected + 1) % count;
    }

    pub fn shop_prev(&mut self) {
        let count = ShopItem::all().len();
        self.shop_selected = (self.shop_selected + count - 1) % count;
    }

    pub fn selected_shop_item(&self) -> ShopItem {
        ShopItem::all()[self.shop_selected]
    }

    pub fn buy_selected(&mut self) {
        let item = self.selected_shop_item();
        self.buy_item(item);
    }

    // ========================================================================
    // SOUL SHOP METHODS
    // ========================================================================

    /// Soul shop navigation
    pub fn soul_shop_next(&mut self) {
        let count = SoulUpgrade::all().len();
        self.soul_shop_selected = (self.soul_shop_selected + 1) % count;
    }

    pub fn soul_shop_prev(&mut self) {
        let count = SoulUpgrade::all().len();
        self.soul_shop_selected = (self.soul_shop_selected + count - 1) % count;
    }

    pub fn selected_soul_upgrade(&self) -> SoulUpgrade {
        SoulUpgrade::all()[self.soul_shop_selected]
    }

    /// Buy the currently selected soul upgrade
    pub fn buy_soul_upgrade(&mut self) -> bool {
        let upgrade = self.selected_soul_upgrade();
        self.souls.buy_upgrade(upgrade)
    }

    /// Open the soul shop view
    pub fn open_soul_shop(&mut self) {
        self.view = ClickerView::SoulShop;
    }

    /// Start a new run after death (prestige)
    pub fn start_new_run(&mut self) {
        self.prestige();
        self.view = ClickerView::Playing;
    }

    /// Show death screen
    pub fn show_death_screen(&mut self) {
        // Calculate souls before showing
        let earned = self.souls.calculate_souls(
            self.dungeon_floor,
            self.monsters_killed,
            self.total_gold_earned,
            self.bosses_killed,
        );
        self.souls.souls_earned_this_run = earned;
        self.souls.total_souls += earned; // Add souls to total
        self.souls.total_runs += 1;
        self.view = ClickerView::Dead;
    }

    /// Get effective attack speed multiplier (from soul upgrades + speed buff)
    pub fn attack_speed_multiplier(&self) -> f32 {
        let soul_speed = self.souls.attack_speed_multiplier();
        let buff_speed = if self.has_speed_buff() { 2.0 } else { 1.0 };
        soul_speed * buff_speed
    }

    /// Get effective crit damage multiplier (from soul upgrades)
    pub fn crit_damage_multiplier(&self) -> f32 {
        self.souls.crit_damage_multiplier()
    }

    // ========================================================================
    // TRANSMUTATION SYSTEM
    // ========================================================================

    /// Get dust value of an item
    pub fn item_dust_value(item: &Item) -> i64 {
        match item {
            // Potions
            Item::Potion(PotionType::Healing) => 1,
            Item::Potion(PotionType::Strength) => 2,
            Item::Potion(PotionType::Speed) => 2,
            Item::Potion(PotionType::Poison) => 3,
            Item::Potion(PotionType::GiantStrength) => 5,
            // Scrolls
            Item::Scroll(ScrollType::Teleport) => 4,
            Item::Scroll(ScrollType::Destruction) => 3,
            Item::Scroll(ScrollType::Enchant) => 8,
            Item::Scroll(ScrollType::MagicMap) => 2,
            Item::Scroll(ScrollType::GoldRush) => 6,
            // Wands (value increases with charges)
            Item::Wand(WandType::Fire, charges) => 3 + (*charges as i64),
            Item::Wand(WandType::Ice, charges) => 3 + (*charges as i64),
            Item::Wand(WandType::Lightning, charges) => 4 + (*charges as i64),
            Item::Wand(WandType::Polymorph, charges) => 5 + (*charges as i64),
            // Rings are valuable
            Item::Ring(_) => 10,
        }
    }

    /// Transmute an inventory item at index to dust
    pub fn transmute_item(&mut self, idx: usize) -> Option<i64> {
        if idx >= self.inventory.len() {
            return None;
        }
        let item = self.inventory.remove(idx);
        let dust = Self::item_dust_value(&item);
        self.souls.dust += dust;
        Some(dust)
    }

    /// Upgrade the heirloom (requires dust)
    pub fn upgrade_heirloom(&mut self) -> bool {
        // Create heirloom if not exists
        if self.souls.heirloom.is_none() {
            self.souls.heirloom = Some(Heirloom::new());
            return true;
        }

        if let Some(ref mut heirloom) = self.souls.heirloom {
            let cost = heirloom.upgrade_cost();
            if self.souls.dust >= cost {
                self.souls.dust -= cost;
                heirloom.enchant();
                return true;
            }
        }
        false
    }

    /// Get heirloom strength bonus
    pub fn heirloom_str_bonus(&self) -> i32 {
        self.souls.heirloom.as_ref().map_or(0, |h| h.str_bonus)
    }

    /// Get heirloom crit bonus
    pub fn heirloom_crit_bonus(&self) -> i32 {
        self.souls.heirloom.as_ref().map_or(0, |h| h.crit_bonus)
    }

    /// Get heirloom life steal bonus
    pub fn heirloom_life_steal_bonus(&self) -> i32 {
        self.souls
            .heirloom
            .as_ref()
            .map_or(0, |h| h.life_steal_bonus)
    }

    // ========================================================================
    // ALCHEMY MASTERY SYSTEM
    // ========================================================================

    /// Gain alchemy XP when using a potion
    pub fn gain_alchemy_xp(&mut self, base_xp: i64) {
        self.souls.alchemy_xp += base_xp;
        // Level up every 100 XP
        let new_level = (self.souls.alchemy_xp / 100) as i32;
        if new_level > self.souls.alchemy_level {
            self.souls.alchemy_level = new_level;
            self.message = Some(format!("Alchemy Mastery increased to level {}!", new_level));
        }
    }

    /// Get current alchemy tier
    pub fn alchemy_tier(&self) -> AlchemyTier {
        AlchemyTier::from_level(self.souls.alchemy_level)
    }

    /// Get buff duration multiplier from alchemy mastery
    pub fn alchemy_duration_multiplier(&self) -> f32 {
        let tier = self.alchemy_tier();
        1.0 + (tier.duration_bonus() as f32 / 100.0)
    }

    /// Get effect multiplier from alchemy mastery (Grandmaster gets 2x)
    pub fn alchemy_effect_multiplier(&self) -> i32 {
        self.alchemy_tier().effect_multiplier()
    }

    // ========================================================================
    // ASCENSION CLASSES SYSTEM
    // ========================================================================

    /// Get the selected class bonuses description
    pub fn class_bonus_description(&self) -> &'static str {
        self.souls.selected_class.description()
    }

    /// Apply class-specific starting bonuses (called in new_with_souls)
    fn apply_class_bonuses(&mut self) {
        match self.souls.selected_class {
            AscensionClass::Peasant => {}
            AscensionClass::Rogue => {
                self.crit_chance += 15;
                self.gold_multiplier += 10;
            }
            AscensionClass::Warrior => {
                self.strength += 5;
                self.armor += 5;
            }
            AscensionClass::Wizard => {
                self.max_hp = (self.max_hp * 80) / 100;
                self.hp = self.max_hp;
            }
            AscensionClass::Tourist => {
                self.gold_multiplier += 50;
            }
            AscensionClass::Archaeologist => {
                // Handled in drop logic
            }
        }
    }

    /// Unlock an ascension class (costs souls)
    pub fn unlock_class(&mut self, class: AscensionClass) -> bool {
        if self.souls.unlocked_classes.contains(&class) {
            return false;
        }
        let cost = class.unlock_cost();
        if self.souls.total_souls >= cost {
            self.souls.total_souls -= cost;
            self.souls.unlocked_classes.push(class);
            return true;
        }
        false
    }

    /// Select an ascension class for next run
    pub fn select_class(&mut self, class: AscensionClass) -> bool {
        if class == AscensionClass::Peasant || self.souls.unlocked_classes.contains(&class) {
            self.souls.selected_class = class;
            return true;
        }
        false
    }

    // ========================================================================
    // BIOME SYSTEM
    // ========================================================================

    /// Update biome based on current floor
    pub fn update_biome(&mut self) {
        self.biome = Biome::from_floor(self.dungeon_floor);
    }

    /// Get biome-themed scenery
    pub fn biome_scenery(&self) -> Scenery {
        Scenery {
            char: self.biome.floor_char(),
            color_idx: self.biome.color_idx(),
        }
    }

    // ========================================================================
    // MONSTER ZOO SYSTEM
    // ========================================================================

    /// Check if zoo should trigger (every 100 kills)
    pub fn check_zoo_trigger(&mut self) {
        if !self.zoo_event.active && self.zoo_kill_counter >= 100 && self.dungeon_floor >= 5 {
            self.zoo_kill_counter = 0;
            self.zoo_event = ZooEvent::start();
            self.message =
                Some("*** MONSTER ZOO! Clear 20 monsters in 10 seconds! ***".to_string());
        }
    }

    /// Process zoo event tick
    pub fn tick_zoo(&mut self) {
        if self.zoo_event.active {
            self.zoo_event.tick();
            if !self.zoo_event.active && !self.zoo_event.reward_pending {
                self.message = Some("Monster Zoo failed! The monsters escaped.".to_string());
            }
        }

        // Claim reward if pending
        if self.zoo_event.reward_pending {
            self.zoo_event.reward_pending = false;
            self.souls.total_zoo_cleared += 1;
            // Bonus gold and XP
            let bonus_gold = (self.dungeon_floor as i64) * 100;
            let bonus_xp = self.dungeon_floor * 50;
            self.gold += bonus_gold;
            self.xp += bonus_xp;
            self.message = Some(format!(
                "Monster Zoo cleared! +{} gold, +{} XP!",
                bonus_gold, bonus_xp
            ));
        }
    }

    /// Called when a monster is killed during zoo
    pub fn zoo_monster_killed(&mut self) {
        if self.zoo_event.active {
            self.zoo_event.monster_killed();
        }
        self.zoo_kill_counter += 1;
    }

    // ========================================================================
    // PARTICLE EFFECTS SYSTEM
    // ========================================================================

    /// Spawn crit particles at a position
    pub fn spawn_crit_particles(&mut self, x: i32, y: i32) {
        let new_particles = Particle::spawn_crit(x, y);
        self.particles.extend(new_particles);
    }

    /// Tick all particles (remove dead ones)
    pub fn tick_particles(&mut self) {
        self.particles.retain_mut(|p| p.tick());
    }

    // ========================================================================
    // YENDOR SHARDS SYSTEM
    // ========================================================================

    /// Grant a Yendor shard (earned from floor 100+ bosses)
    pub fn grant_yendor_shard(&mut self) {
        self.souls.yendor_shards += 1;
    }

    /// Place a shard in the grid
    pub fn place_shard(&mut self, row: usize, col: usize, shard: ShardType) -> bool {
        if row >= 3 || col >= 3 {
            return false;
        }
        if self.souls.shard_grid[row][col].is_some() {
            return false;
        }
        if self.souls.yendor_shards <= 0 {
            return false;
        }
        self.souls.yendor_shards -= 1;
        self.souls.shard_grid[row][col] = Some(shard);
        true
    }

    /// Get active synergies from the shard grid
    pub fn active_synergies(&self) -> Vec<ShardSynergy> {
        let mut synergies = Vec::new();
        let grid = &self.souls.shard_grid;

        // Check horizontal and vertical adjacencies
        for row in 0..3 {
            for col in 0..3 {
                if let Some(shard) = grid[row][col] {
                    // Check right neighbor
                    if col + 1 < 3 {
                        if let Some(right) = grid[row][col + 1] {
                            if let Some(syn) = Self::check_synergy(shard, right) {
                                if !synergies.contains(&syn) {
                                    synergies.push(syn);
                                }
                            }
                        }
                    }
                    // Check bottom neighbor
                    if row + 1 < 3 {
                        if let Some(bottom) = grid[row + 1][col] {
                            if let Some(syn) = Self::check_synergy(shard, bottom) {
                                if !synergies.contains(&syn) {
                                    synergies.push(syn);
                                }
                            }
                        }
                    }
                }
            }
        }
        synergies
    }

    /// Check if two adjacent shards create a synergy
    fn check_synergy(a: ShardType, b: ShardType) -> Option<ShardSynergy> {
        match (a, b) {
            (ShardType::Gold, ShardType::Gold) => Some(ShardSynergy::Avarice),
            (ShardType::Power, ShardType::Vampiric) | (ShardType::Vampiric, ShardType::Power) => {
                Some(ShardSynergy::Bloodlust)
            }
            (ShardType::Vitality, ShardType::Vitality) => Some(ShardSynergy::Fortress),
            (ShardType::Speed, ShardType::Power) | (ShardType::Power, ShardType::Speed) => {
                Some(ShardSynergy::Fury)
            }
            (ShardType::Fortune, ShardType::Gold) | (ShardType::Gold, ShardType::Fortune) => {
                Some(ShardSynergy::Treasure)
            }
            _ => None,
        }
    }

    /// Calculate total shard bonuses
    pub fn shard_bonuses(&self) -> (i32, i32, i32, i32, i32, i32) {
        // (gold%, damage%, hp, speed%, drops%, lifesteal%)
        let mut gold = 0;
        let mut damage = 0;
        let mut hp = 0;
        let mut speed = 0;
        let mut drops = 0;
        let mut lifesteal = 0;

        for row in &self.souls.shard_grid {
            for shard in row.iter().flatten() {
                match shard {
                    ShardType::Gold => gold += 10,
                    ShardType::Power => damage += 5,
                    ShardType::Vitality => hp += 10,
                    ShardType::Speed => speed += 5,
                    ShardType::Fortune => drops += 5,
                    ShardType::Vampiric => lifesteal += 2,
                }
            }
        }

        // Apply synergy bonuses
        for syn in self.active_synergies() {
            match syn {
                ShardSynergy::Avarice => gold += 25,
                ShardSynergy::Bloodlust => lifesteal += 5,
                ShardSynergy::Fortress => {} // Armor handled separately
                ShardSynergy::Fury => {}     // Crit damage handled separately
                ShardSynergy::Treasure => drops += 25,
            }
        }

        (gold, damage, hp, speed, drops, lifesteal)
    }

    /// Get armor bonus from Fortress synergy
    pub fn shard_armor_bonus(&self) -> i32 {
        if self.active_synergies().contains(&ShardSynergy::Fortress) {
            5
        } else {
            0
        }
    }

    /// Check if Fury synergy is active (3x crit damage)
    pub fn has_fury_synergy(&self) -> bool {
        self.active_synergies().contains(&ShardSynergy::Fury)
    }
}
