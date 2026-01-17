# Q-PAINT Plan: MS Paint / Mario Paint Inspired Pixel Art Editor

## Summary

Create a new plugin crate `qdos-plugin-qpaint` - a pixel art editor with drawing tools, color palette, and image export. Inspired by MS Paint and Mario Paint.

## Key Features

1. **Canvas** - Pixel grid with zoom (1x-8x), pan/scroll
2. **Drawing Tools** - Pencil, brush, line, rectangle, ellipse, fill, eraser
3. **Color Palette** - 16-color DOS palette + custom colors
4. **Selection** - Rectangle select, move, copy/paste
5. **Undo/Redo** - History stack
6. **File I/O** - Load/save PNG, BMP
7. **ASCII Export** - Export as ANSI art (bonus!)

## Dependencies

```toml
[dependencies]
qdos-plugin-api = { path = "../qdos-plugin-api" }
inventory = "0.3"
ratatui = "0.29"
crossterm = "0.28"
image = "0.25"           # Already in workspace - PNG/BMP support
dirs = "6.0"
```

## Crate Structure

```
crates/qdos-plugin-qpaint/
├── Cargo.toml
└── src/
    ├── lib.rs          # Plugin struct, trait impl, key handlers
    ├── state.rs        # QPaintState, Canvas, Tool types
    ├── modal.rs        # UI rendering (canvas, palette, tools)
    ├── canvas.rs       # Canvas operations (draw, fill, shapes)
    └── file_io.rs      # PNG/BMP load/save
```

## State Design (state.rs)

```rust
pub enum QPaintView {
    Editor,         // Main canvas view
    Palette,        // Color selection
    FileMenu,       // Load/Save dialog
    Help,
}

pub enum Tool {
    Pencil,
    Brush,
    Line,
    Rectangle,
    Ellipse,
    Fill,
    Eraser,
    Select,
    ColorPicker,
}

pub struct Canvas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,      // RGB data (width * height * 3)
}

pub struct QPaintState {
    pub view: QPaintView,
    pub canvas: Canvas,

    // Tool state
    pub tool: Tool,
    pub brush_size: u8,
    pub fg_color: (u8, u8, u8),    // Foreground (left click)
    pub bg_color: (u8, u8, u8),    // Background (right click)

    // Cursor/viewport
    pub cursor_x: u32,
    pub cursor_y: u32,
    pub zoom: u8,                   // 1-8
    pub scroll_x: u32,
    pub scroll_y: u32,

    // Selection
    pub selection: Option<(u32, u32, u32, u32)>,  // x, y, w, h
    pub clipboard: Option<Vec<u8>>,

    // Drawing state
    pub drawing: bool,
    pub line_start: Option<(u32, u32)>,

    // History
    pub undo_stack: Vec<Vec<u8>>,
    pub redo_stack: Vec<Vec<u8>>,

    // Palette
    pub palette: Vec<(u8, u8, u8)>,
    pub palette_index: usize,

    // File
    pub file_path: Option<PathBuf>,
    pub modified: bool,
}
```

## DOS 16-Color Palette

```rust
const DOS_PALETTE: [(u8, u8, u8); 16] = [
    (0, 0, 0),       // 0: Black
    (0, 0, 170),     // 1: Blue
    (0, 170, 0),     // 2: Green
    (0, 170, 170),   // 3: Cyan
    (170, 0, 0),     // 4: Red
    (170, 0, 170),   // 5: Magenta
    (170, 85, 0),    // 6: Brown
    (170, 170, 170), // 7: Light Gray
    (85, 85, 85),    // 8: Dark Gray
    (85, 85, 255),   // 9: Light Blue
    (85, 255, 85),   // A: Light Green
    (85, 255, 255),  // B: Light Cyan
    (255, 85, 85),   // C: Light Red
    (255, 85, 255),  // D: Light Magenta
    (255, 255, 85),  // E: Yellow
    (255, 255, 255), // F: White
];
```

## Views

### Main Editor View
```
╔═════════════════════ Q-PAINT: untitled.png ═══════════════════════╗
║ [P]encil [B]rush [L]ine [R]ect [E]llipse [F]ill [X]Eraser [S]elect║
╠═══════════════════════════════════════════════════════════════════╣
║     0    1    2    3    4    5    6    7    8    9   10   11      ║
║  0  .... .... #### #### #### #### #### #### .... ....            ║
║  1  .... #### #### #### #### #### #### #### #### ....            ║
║  2  #### #### [@@] @@## ##@@ @@## ##@@ #### #### ####            ║
║  3  #### #### ##@@ @@## ##@@ @@## ##@@ #### #### ####            ║
║  4  #### #### #### #### #### #### #### #### #### ####            ║
║  5  #### #### #### #### #### #### #### #### #### ####            ║
║  6  #### #### #### @@## ##@@ #### #### #### #### ####            ║
║  7  .... #### #### #### @@@@ @@@@ #### #### #### ....            ║
║  8  .... .... #### #### #### #### #### #### .... ....            ║
╠═══════════════════════════════════════════════════════════════════╣
║ FG:[##] BG:[..] Tool:Pencil  Size:1  Zoom:4x  32x32  (16,8)       ║
╚═══════════════════════════════════════════════════════════════════╝
 1-9:Color  Tab:Palette  Z:Zoom  Arrows:Move  Space:Draw  Esc:Exit
```

### Palette View
```
╔═════════════════════ Q-PAINT: Palette ════════════════════════════╗
║ Standard DOS Palette:                                             ║
║                                                                   ║
║  [0] ##  [1] ##  [2] ##  [3] ##  [4] ##  [5] ##  [6] ##  [7] ##  ║
║  [8] ##  [9] ##  [A] ##  [B] ##  [C] ##  [D] ##  [E] ##  [F] ##  ║
║                                                                   ║
║ Current:  FG: [##] R:255 G:255 B: 85  (Yellow)                   ║
║           BG: [..] R:  0 G:  0 B:  0  (Black)                    ║
║                                                                   ║
║ Custom Color: R:[255] G:[255] B:[ 85]                            ║
╠═══════════════════════════════════════════════════════════════════╣
║ Selected: E - Yellow                                              ║
╚═══════════════════════════════════════════════════════════════════╝
 0-F:Select  Enter:Set FG  Shift+Enter:Set BG  Esc:Back
```

## Key Bindings

### Editor View
| Key | Action |
|-----|--------|
| Arrow keys | Move cursor |
| Space | Draw with current tool |
| Shift+Arrow | Draw while moving (pencil) |
| P | Pencil tool |
| B | Brush tool |
| L | Line tool |
| R | Rectangle tool |
| E | Ellipse tool |
| F | Fill tool |
| X | Eraser |
| S | Select tool |
| I | Color picker (eyedropper) |
| 1-9, 0 | Quick palette colors |
| Tab | Open palette |
| Z/Shift+Z | Zoom in/out |
| Ctrl+Z | Undo |
| Ctrl+Y | Redo |
| Ctrl+S | Save |
| Ctrl+O | Open |
| Ctrl+N | New canvas |
| Ctrl+C | Copy selection |
| Ctrl+V | Paste |
| Delete | Clear selection |
| Esc | Exit |

### Drawing Mechanics

**Pencil**: Single pixel at cursor, draw on Space/Shift+Arrow
**Brush**: Square of brush_size pixels
**Line**: First Space sets start, second Space draws line
**Rectangle**: First Space sets corner, second Space draws rect
**Ellipse**: First Space sets center, second Space draws ellipse
**Fill**: Flood fill from cursor position
**Eraser**: Paint with background color
**Select**: Drag to create selection rectangle
**ColorPicker**: Pick color at cursor position

## Implementation Phases

### Phase 1: Core Structure
1. Create crate skeleton with Cargo.toml
2. Implement state types (Canvas, Tool, QPaintState)
3. Implement Plugin trait boilerplate
4. Add to workspace Cargo.toml

### Phase 2: Canvas & Rendering
1. Canvas struct with pixel buffer operations
2. ASCII rendering of canvas (zoomed grid)
3. Cursor display and movement
4. Status bar with tool/color info

### Phase 3: Drawing Tools
1. Pencil (single pixel)
2. Brush (multi-pixel)
3. Line (Bresenham's algorithm)
4. Rectangle (outline or filled)
5. Ellipse (midpoint algorithm)
6. Fill (flood fill)
7. Eraser

### Phase 4: Color Palette
1. DOS 16-color palette
2. Palette view rendering
3. FG/BG color selection
4. Quick color keys (1-9, 0)

### Phase 5: Selection & Clipboard
1. Selection rectangle
2. Copy/paste operations
3. Move selection
4. Clear selection

### Phase 6: File I/O & History
1. PNG load/save via `image` crate
2. BMP load/save
3. Undo/redo stack
4. New canvas dialog

### Phase 7: Polish
1. Help screen
2. File menu
3. Quality checks

## File Modifications

| File | Changes |
|------|---------|
| `Cargo.toml` (workspace) | Add `qdos-plugin-qpaint` to members |
| `crates/qdos-plugin-qpaint/Cargo.toml` | **NEW** - Plugin dependencies |
| `crates/qdos-plugin-qpaint/src/lib.rs` | **NEW** - Plugin implementation |
| `crates/qdos-plugin-qpaint/src/state.rs` | **NEW** - State types |
| `crates/qdos-plugin-qpaint/src/modal.rs` | **NEW** - UI rendering |
| `crates/qdos-plugin-qpaint/src/canvas.rs` | **NEW** - Drawing operations |
| `crates/qdos-plugin-qpaint/src/file_io.rs` | **NEW** - Image load/save |

## Canvas ASCII Rendering

Each pixel represented by characters based on zoom level:
- **Zoom 1x**: Single char per pixel (`. # @ +` based on brightness)
- **Zoom 2x**: 2x2 char block per pixel
- **Zoom 4x**: 4x4 char block per pixel (default)
- **Zoom 8x**: 8x8 char block (for detail work)

Pixel representation using half-block characters for color:
```
'.' = empty (bg color)
'#' = filled (pixel color approximated)
'@' = cursor position
```

For color approximation, map RGB to nearest DOS palette color for display.

## Verification

1. `cargo build -p qdos-plugin-qpaint` - Plugin compiles
2. `cargo run` → Open Q-PAINT from Apps menu
3. Test drawing: Use pencil to draw pixels
4. Test shapes: Draw line, rectangle, ellipse
5. Test fill: Flood fill an area
6. Test palette: Change colors
7. Test save: Ctrl+S to save PNG
8. Test load: Open existing image
9. Test undo: Draw, Ctrl+Z to undo
10. Quality checks: `cargo fmt -- --check && cargo clippy -- -D warnings && cargo test`
