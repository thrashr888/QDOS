# DOS Game Examples

These classic DOS games can be run using the R-DOS Emulator plugin (`x` key or F12 > Emulator).

## Requirements

Install DOSBox-X:
```bash
brew install dosbox-x
```

## Games

### Rogue (1983)
**File:** `rogue/ROGUE.EXE`

The original roguelike dungeon crawler that inspired our built-in Rogue game!
Navigate the Dungeons of Doom to retrieve the Amulet of Yendor.

**Controls:**
- `h/j/k/l` - Move left/down/up/right
- `y/u/b/n` - Diagonal movement
- `s` - Search for traps
- `>` - Go down stairs
- `?` - Help

**Source:** [Internet Archive - Rogue 1983](https://archive.org/details/msdos_Rogue_1983)
**License:** Freeware/Public Domain

### Arcade Volleyball (1987)
**File:** `arcade_volleyball/AV.EXE`

A simple two-player volleyball game. Keep the ball from hitting your side!

**Controls:**
- Player 1: `Q/A` (up/down), `Z` (jump)
- Player 2: Arrow keys, `M` (jump)

**Source:** [Internet Archive - Arcade Volleyball 1987](https://archive.org/details/msdos_Arcade_Volleyball_1987)
**License:** Public Domain (released by developer Rhett Anderson)

## Running Games

1. Select the `.EXE` file in R-DOS
2. Press `x` to open the Emulator
3. Press `Enter` to run in DOSBox-X
4. Press `Alt+Enter` for fullscreen in DOSBox-X
5. Press `Ctrl+F9` to exit DOSBox-X

## Configuration

The `dosbox-x.conf` file in this folder configures DOSBox-X to use
texture rendering instead of OpenGL, which fixes display issues on macOS.
The emulator plugin automatically loads this config when present.
