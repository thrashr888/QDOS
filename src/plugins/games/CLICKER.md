# Clicker - Roguelike Idle Game

A roguelike-themed incremental/idle game where you fight monsters, gain gold and XP, and buy upgrades.

## Controls

### Combat

- `Space` or `H` - Attack monster (manual hit)
- `E` - Eat food (restore HP)
- `Q/R/Z` - Use potion from inventory
- `1-8` - Use item from inventory slot

### Navigation

- `S` or `Tab` - Open shop
- `B` or `Enter` - Open inventory/buy
- `>` - Descend to next floor (when stairs available)
- `Esc` - Return to menu (or close shop)
- `W` - Save game manually

### Shop Navigation

- `Up/Down` or `K/J` - Select item
- `Left/Right` - Switch between shop tabs
- `Enter` - Buy selected item

## Progression

### Floors and Biomes

The dungeon changes visually every 20 floors:

- **Floors 1-20**: The Mines (basic dungeon)
- **Floors 21-40**: The Swamp (murky wetlands)
- **Floors 41-60**: The Crypt (undead themed)
- **Floors 61-80**: Demon Halls (grand halls)
- **Floors 81+**: The Abyss (final depths)

### Combat Lanes

As you complete more runs, you unlock additional combat lanes:

- **1 lane** (default) - Fight one monster at a time
- **Extra lanes** - Unlock 1 additional lane per 10 total runs (max 10 lanes)

Multiple lanes means more monsters to fight simultaneously, more gold, and more XP.

### Monster Difficulty

Monsters scale exponentially with floor depth (15% harder per floor), so you will eventually hit a wall. This is intended - prestige to get stronger!

### Elite Monsters

Starting from floor 1, monsters have a chance to spawn as elites with special affixes:

- **Fast** - Attacks more frequently
- **Deadly** - Higher damage
- **Tough** - More HP
- **Rich** - Drops more gold
- **Vampiric** - Heals when hitting player

Elites give bonus XP.

### Floor Bosses

Every 10 floors features a floor boss with a special title:

- Floor 10: Guardian
- Floor 20: Champion
- Floor 30: Warlord
- Floor 40: Overlord
- etc.

## Shop Items (Gold)

### Consumables

- **+10 Food** (5g) - Restore HP with 'E'

### Stats

- **+1 STR** (25g) - Increase attack damage
- **+1 ARM** (25g) - Reduce damage taken
- **+5% Crit** (40g) - Chance for 2x damage
- **+25% Gold** (60g) - More gold per kill
- **+5% Lifesteal** (50g) - Heal from damage dealt

### Automation

- **Auto-Hit** (150g) - Attack automatically
- **Auto-Eat** (200g) - Eat food when HP is low (default 50%)
- **Auto-Quaff** (250g) - Auto-use potions smartly
- **Auto-Equip** (300g) - Auto-equip better gear

## Souls & Prestige System

When you die, you earn **Souls** based on:

- Floors explored (10 souls per floor)
- Monsters killed (1 soul per monster)
- Gold earned (1 soul per 100 gold)
- Bosses killed (50 souls per boss)

### Soul Shop Upgrades

Soul upgrades persist across all runs:

| Upgrade       | Cost | Effect                                       |
| ------------- | ---- | -------------------------------------------- |
| Soul Speed    | 30+  | +10% attack speed (max 10 levels = 2x speed) |
| Soul Gluttony | 75+  | +10% auto-eat threshold (max 3 levels = 80%) |
| Innate Fury   | 150  | Start with Auto-Hit unlocked                 |
| Innate Hunger | 200  | Start with Auto-Eat unlocked                 |
| Innate Thirst | 250  | Start with Auto-Quaff unlocked               |
| Innate Style  | 300  | Start with Auto-Equip unlocked               |
| Soul Strength | 15+  | +1 starting STR                              |
| Soul Armor    | 15+  | +1 starting ARM                              |
| Soul Vitality | 20+  | +10 starting HP                              |
| Soul Wealth   | 10+  | +50 starting gold                            |
| Soul Fury     | 50+  | +50% crit dmg (2x -> 2.5x -> 3x...)          |
| Soul Greed    | 40+  | +25% gold from all sources                   |
| Soul Fortune  | 25+  | +3% item drop chance                         |
| Soul Warp     | 100+ | Start 1 floor deeper (max 10 = floor 11)     |

Costs increase by 1.5x per level purchased.

## Inventory System

You have 8 inventory slots for items found while dungeon-crawling.

### Item Types

**Potions** (40% drop chance):

- Healing - Restore 50% HP
- Strength - +10 STR for 30 ticks
- Speed - 2x attack speed for 30 ticks
- Giant Strength - +25 STR for 20 ticks
- Poison - Deal 50% of monster's max HP as damage

**Scrolls** (25% drop chance):

- Destruction - Deal 100 damage
- Enchant - +3 to weapon or armor permanently
- Gold Rush - 3x gold from next 5 kills
- Teleport - Skip 5 monsters
- Magic Map - Reveal boss type

**Rings** (20% drop chance) - Passive when equipped:

- Protection - +5 ARM while worn
- Strength - +5 STR while worn
- Regeneration - +1 HP per 10 ticks
- Wealth - +50% gold drops
- Vampirism - +10% life steal

**Wands** (15% drop chance) - Limited charges:

- Fire [3-5 charges] - 30 fire damage
- Lightning [3-5 charges] - 50 lightning damage
- Ice [3-5 charges] - Halve enemy damage for 10 ticks
- Polymorph [3-5 charges] - Transform enemy to weaker type

### Equipment Slots

- Weapon (STR focus)
- Armor (ARM + HP)
- Helm (ARM + HP)
- Amulet (balanced stats)
- Cloak (Speed focus)
- Gloves (Crit focus)
- Boots (Speed focus)
- Shield (ARM focus, -Speed)
- 2 Ring slots

## Advanced Systems

### Monster Zoo Events

Every 100 kills after floor 5, a **Monster Zoo** event triggers:

- Kill 20 monsters in 10 seconds (200 ticks)
- Success: Bonus rewards
- Failure: No penalty, just miss the reward

### Arcane Dust & Transmutation

Convert items to **Arcane Dust** currency:

- Use the transmutation filter to auto-convert items
- Filter levels: Off, Common, Uncommon, Rare, All
- Dust is used to enchant your **Heirloom**

### The Heirloom

A persistent weapon that survives across runs:

- Starts as "Ancestral Blade" with +1 STR
- Enchant with Arcane Dust to upgrade
- Each enchant: +1 STR, every 3 levels +1% crit, every 5 levels +1% life steal
- Cost doubles each level (10, 20, 40, 80...)

### Ascension Classes

Unlock with souls for different playstyles:

- **Peasant** (free) - No bonuses
- **Rogue** (50 souls) - +15% crit, +10% gold
- **Warrior** (100 souls) - +5 STR, +5 ARM at start
- **Wizard** (200 souls) - Scrolls 2x potent, -20% HP
- **Tourist** (150 souls) - +50% gold, -30% damage
- **Archaeologist** (500 souls) - +25% drops, start with artifact

### Alchemy Mastery

Gain alchemy XP by using potions. Tiers:

- Novice (0-9) - No bonus
- Apprentice (10-24) - +25% potion duration
- Journeyman (25-49) - +50% duration, auto-ID potions
- Expert (50-74) - +75% duration, refine bad potions
- Master (75-99) - +100% duration, potions never fail
- Grandmaster (100+) - 2x effects, instant auto-quaff

### Yendor Shards

End-game meta progression:

- Collect shards from deep dungeon runs
- Place in 3x3 grid for bonuses
- Adjacent matching shards create synergies:
  - **Avarice** (Gold+Gold) - Enemies drop gold on hit
  - **Bloodlust** (Power+Vampiric) - Heal 5 HP on kill
  - **Fortress** (Vitality+Vitality) - +5 armor
  - **Fury** (Speed+Power) - Crits deal 3x
  - **Treasure** (Fortune+Gold) - Double item drops

## Tips for New Players

1. **Focus on Auto-Hit first** - Your primary gold upgrade
2. **Buy food regularly** - Staying alive = more progress
3. **Don't ignore armor** - Reduces damage significantly
4. **Save souls for Soul Speed** - Permanent attack speed helps every run
5. **Use the stairs** - Go deeper for more rewards, but beware scaling
6. **Prestige often early** - Early souls compound quickly
7. **Watch for Zoo events** - Great for bonus rewards
8. **Equip rings** - Free passive bonuses
