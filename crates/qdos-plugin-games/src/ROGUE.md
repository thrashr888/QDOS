# Rogue - ASCII Dungeon Crawler

A classic ASCII dungeon crawler roguelike. Explore procedurally generated dungeons, fight monsters, collect items, and try to escape from dungeon level 10.

## Objective

Descend through 10 dungeon levels and escape to win. Each level is procedurally generated with rooms, corridors, monsters, and treasure.

## Controls

### Movement
- `Arrow keys` - Move in cardinal directions
- `H/J/K/L` - Vi-style movement (left/down/up/right)
- `Y/U/B/N` - Diagonal movement (up-left/up-right/down-left/down-right)

### Actions
- `S` - Search for hidden traps and doors
- `>` - Descend stairs (when standing on `%`)
- `Esc` - Quit to menu
- `P` - Pause game

## Map Symbols

### Terrain
| Symbol | Name | Description |
|--------|------|-------------|
| `.` | Floor | Walkable dungeon floor |
| `#` | Wall/Corridor | Solid walls or corridor passages |
| `+` | Door | Doorway between rooms |
| `%` | Stairs Down | Descend to next level |
| `<` | Stairs Up | Return to previous level |
| `^` | Trap | Visible trap (dangerous!) |

### Entities
| Symbol | Name | Description |
|--------|------|-------------|
| `@` | You | The player character |
| `*` | Gold | Piles of treasure |
| `:` | Food | Food rations to stave off hunger |
| `!` | Potion | Magic potions with various effects |
| `?` | Scroll | Magic scrolls with powerful effects |
| `)` | Weapon | Weapons to increase attack |
| `]` | Armor | Armor to increase defense |

### Monsters
| Symbol | Name | HP | Damage | XP | Notes |
|--------|------|-----|--------|-----|-------|
| `R` | Rat | 2 | 1 | 5 | Very weak, common on level 1 |
| `B` | Bat | 3 | 2 | 10 | Weak, flies around |
| `G` | Goblin | 5 | 3 | 15 | Basic enemy |
| `S` | Skeleton | 6 | 4 | 20 | Undead, appears mid-game |
| `O` | Orc | 8 | 5 | 30 | Tough fighter |
| `T` | Troll | 12 | 7 | 50 | Strong, appears later |
| `D` | Dragon | 20 | 10 | 100 | Boss-level monster |

Monster types depend on dungeon level:
- **Level 1**: Mostly Rats and Bats
- **Levels 2-3**: Bats, Goblins, Skeletons
- **Levels 4-6**: Goblins, Skeletons, Orcs, occasional Trolls
- **Level 7+**: Orcs, Trolls, Skeletons, Dragons (rare)

## Game Mechanics

### Combat
Walk into a monster to attack. Combat is automatic:
- **Damage dealt** = Your Attack - Monster Damage/2 (minimum 1)
- **Damage taken** = Monster Damage - Your Defense (minimum 1)

When you kill a monster, you gain XP. If the monster kills you, the game ends.

### Hunger System
You have a hunger meter (0-1000):
- Hunger decreases by 1 every 10 game ticks
- **>500**: Normal (no message)
- **301-500**: Hungry
- **101-300**: Weak
- **0-100**: Fainting
- **0**: Starving - you take 1 damage per tick

Eat food (`:`) to restore hunger to full.

### Level Up
Gain XP from killing monsters. XP required for next level = current level * 50.

On level up:
- +5 Max HP
- Full HP heal
- +2 Attack
- +1 Defense

### Visibility (Shadowcasting)
You can only see what's in your line of sight (radius 8). The game uses recursive shadowcasting for realistic vision - you cannot see through walls. Previously explored areas appear dimmed.

### Searching
Press `S` to search adjacent squares for hidden traps. There's a 50% chance per hidden trap to reveal it.

### Traps
Stepping on a trap (visible or hidden) triggers a random effect:
- **Teleport trap** - Teleport to a random location
- **Dart trap** - Take 1-3 damage
- **Bear trap** - Lose a turn (stuck)
- **Pit trap** - Take 2-5 damage

Hidden traps look like floor (`.`) until revealed.

## Items

### Gold (`*`)
Piles of gold worth 10 + (dungeon_level * 5). Automatically picked up when walked over.

### Food (`:`)
Restores hunger to maximum (1000). Essential for survival on longer runs.

### Potions (`!`)
Random effect when picked up:
- **Healing Potion** (33%) - Restore 5-14 HP
- **Strength Potion** (33%) - +1 Strength permanently
- **Poison** (33%) - Take 1-4 damage

### Scrolls (`?`)
Random effect when picked up:
- **Scroll of Light** (25%) - Reveal larger area around you
- **Scroll of Teleport** (25%) - Teleport to random safe location
- **Scroll of Scare Monster** (25%) - Confuses nearby monsters
- **Scroll of Magic Mapping** (25%) - Reveal entire dungeon level

### Weapons (`)`)
Found weapons add +1 to +3 to your Attack stat permanently.

### Armor (`]`)
Found armor adds +1 to +2 to your Defense stat permanently.

## Dungeon Generation

Each level is generated with:
- Up to 8 rooms of varying sizes (4-10 tiles)
- Corridors connecting rooms
- Monsters (2 + dungeon_level per floor)
- Gold in every room (except first room)
- 40% chance of an item per room
- Traps increase with dungeon depth (10% + 5% per level, max 50%)

The player starts in the first room. Stairs down (`%`) are placed in the last room.

## Strategy Tips

1. **Explore carefully** - Use `S` to search for traps before moving into new areas
2. **Watch your hunger** - Keep food in reserve, don't let it reach 0
3. **Pick your fights** - Some monsters are too strong early on
4. **Corridor combat** - Fight in corridors to prevent being surrounded
5. **Level up before descending** - Clear the floor for XP before going deeper
6. **Save scrolls of teleport** - Emergency escape from bad situations
7. **Armor is valuable** - Defense reduces all incoming damage
8. **Don't be greedy** - Sometimes retreat is the best option

## Scoring

Your final score is based on:
- Gold collected
- Monsters killed
- Dungeon levels cleared
- Bonus for escaping (level 10+)

## Win Condition

Reach dungeon level 10 and find the stairs down. Descending from level 10 completes the game:

> "You escaped the dungeon! You win!"
