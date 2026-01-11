# Star Trek (1971) - Tactical Space Combat

A classic tactical space combat game where you command the USS Enterprise through an 8x8 galaxy, hunting Klingons while managing energy and time.

## Objective

Destroy all Klingons in the galaxy before the time limit expires. The mission generates with:
- 10+ Klingons scattered across the galaxy
- 2+ Starbases for resupply
- Time limit = Klingon count * 3 stardates

## Controls

### Main Commands
| Key | Command | Description |
|-----|---------|-------------|
| `N` | Navigation | Set course and warp speed |
| `S` | Short Range Scan | View current sector in detail |
| `L` | Long Range Scan | Scan surrounding quadrants |
| `P` | Phasers | Fire energy weapons at Klingons |
| `T` | Torpedoes | Fire photon torpedoes |
| `H` | Shields | Transfer energy to shields |
| `C` | Computer | Access ship computer functions |
| `D` | Damage Report | View ship system status |

### Command Entry
- Number keys for courses/values
- `Enter` to confirm
- `Esc` to cancel

## Galaxy Layout

The galaxy is an 8x8 grid of **quadrants**. Each quadrant contains an 8x8 grid of **sectors**.

### Sector Entities
| Symbol | Name | Description |
|--------|------|-------------|
| `<E>` | Enterprise | Your ship |
| `+K+` | Klingon | Enemy warship |
| `>S<` | Starbase | Friendly base for resupply |
| ` * ` | Star | Obstacle, blocks movement |
| ` . ` | Empty | Empty space |

## Ship Systems

### Energy
- Start with 3000 energy
- Used for: Warp travel, phasers, shields
- Regenerates when docked at starbase

### Shields
- Absorb damage from Klingon attacks
- Transfer energy to shields with `H` command
- Shields at 0 means hull takes direct damage

### Photon Torpedoes
- Start with 10 torpedoes
- Instant kill on direct hit
- Must aim with course (1-9)
- Reload at starbases

### Ship Systems (can be damaged)
| System | Function |
|--------|----------|
| Warp Engines | Navigation and movement |
| S.R. Sensors | Short range scan display |
| L.R. Sensors | Long range scan capability |
| Phasers | Phaser weapon systems |
| Photon Tubes | Torpedo launching |
| Damage Control | Repair speed |
| Shield Control | Shield management |
| Computer | Ship computer functions |

Damaged systems show negative values in damage report. Systems repair slowly over time (faster when docked).

## Navigation

### Course Directions (Numpad Style)
```
7 8 9
4 . 6
1 2 3
```
Course 5 is invalid (no movement).

### Warp Speed
- Range: 0.1 to 8.0
- Higher warp = more energy, faster travel
- Warp 1.0 moves 8 sectors
- Crossing quadrant boundaries uses same energy

### Movement
- Each warp uses: warp * 10 * 8 energy
- Time passes: 0.1 stardates (warp < 1) or 1.0 stardates (warp >= 1)
- Collision with obstacles blocks movement

## Combat

### Phasers
1. Press `P` to activate phasers
2. Enter energy amount (0 to cancel, max = available energy)
3. Energy is divided among all Klingons in sector
4. Damage decreases with distance from target
5. Destroyed Klingons disappear

### Photon Torpedoes
1. Press `T` to fire torpedo
2. Enter course (1-9, except 5)
3. Torpedo travels in straight line
4. First target hit is destroyed
5. Can accidentally destroy starbases!

### Klingon Attacks
After your turn, Klingons attack if:
- You're in the same quadrant
- You're not docked at a starbase

Damage calculation:
- Based on Klingon energy / distance
- Shields absorb damage first
- Remaining damage hits hull (energy) and may damage systems

## Starbases

Dock at a starbase by moving adjacent to it:
- Full energy restoration
- Full torpedo reload
- All systems repaired
- Shields reset to 0 (absorbed into energy)
- Protected from Klingon attacks while docked

## Computer Functions

Access with `C` command:

| Key | Function | Description |
|-----|----------|-------------|
| `G` | Galaxy Map | View scanned quadrant data |
| `S` | Status | Full ship and mission status |
| `T` | Torpedo Calculator | Suggest course to nearest Klingon |

### Galaxy Map
Shows scanned quadrants in KBS format:
- **K** = Number of Klingons
- **B** = Number of Starbases
- **S** = Number of Stars

Example: `210` = 2 Klingons, 1 Starbase, 0 Stars

Unscanned quadrants show `???`.

## Game End Conditions

### Victory
> "*** CONGRATULATIONS! All Klingons destroyed! ***"

Destroy all Klingons before time expires.

### Defeat
Several conditions end the game:
- **Enterprise Destroyed** - Energy reaches 0
- **Time Expired** - Stardate exceeds mission deadline
- **Stranded** - No starbases remain and energy critically low

## Strategy Tips

1. **Use Long Range Sensors early** - Map the galaxy to plan your route
2. **Prioritize quadrants with starbases** - Clear nearby threats to protect resupply
3. **Don't waste torpedoes** - Use computer for course calculation
4. **Shields up before combat** - Transfer energy before entering Klingon space
5. **Dock often** - Free repairs and reload
6. **Watch the clock** - Time is your biggest enemy
7. **Phasers for multiple enemies** - Torpedoes for single targets
8. **Don't destroy starbases** - You need them for resupply!
9. **Low warp in quadrant** - Use warp < 1.0 to save time while maneuvering
10. **Full warp between quadrants** - Minimize travel time across the galaxy

## Scoring

Your final score considers:
- Klingons destroyed
- Time remaining
- Starbases preserved
- Energy efficiency

## Historical Note

This game is based on the original Star Trek game (1971-1972) by Mike Mayfield, one of the earliest computer games ever created. It was written in BASIC and distributed as a type-in program, becoming incredibly popular on mainframes, minicomputers, and early personal computers.

The game's mechanics - the 8x8 galaxy grid, energy management, warp navigation, and tactical combat - became a template for countless space strategy games that followed.
