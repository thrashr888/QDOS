# Q-DESIGN: Print Designer Plan

## Summary

Create a new plugin crate `qdos-plugin-qdesign` - a print/layout designer for creating cards, banners, flyers, and other printable designs. Inspired by Print Shop, PrintMaster, and Canva.

## Key Features

1. **Canvas Layout** - Fixed-size pages with rulers/guides
2. **Text Frames** - Styled text boxes with fonts and alignment
3. **Image Frames** - Place and resize images (sixel preview)
4. **Clip Art Library** - Built-in simple ASCII/sixel art collection
5. **Templates** - Pre-made layouts (cards, banners, flyers)
6. **Export** - PNG, ASCII art output

## Dependencies

```toml
[dependencies]
qdos-plugin-api = { path = "../qdos-plugin-api" }
inventory = "0.3"
ratatui = "0.29"
crossterm = "0.28"
image = "0.25"           # Image handling
ratatui-image = "4.0"    # Sixel rendering
dirs = "6.0"
```

## Crate Structure

```
crates/qdos-plugin-qdesign/
├── Cargo.toml
└── src/
    ├── lib.rs          # Plugin struct, trait impl, key handlers
    ├── state.rs        # QDesignState, Frame types, Tool types
    ├── modal.rs        # UI rendering (canvas, toolbox, properties)
    ├── canvas.rs       # Canvas operations (layout, frames)
    ├── templates.rs    # Built-in templates
    ├── clipart.rs      # Clip art library
    └── export.rs       # PNG/ASCII export
```

## State Design (state.rs)

```rust
pub enum QDesignView {
    Canvas,         // Main design view
    TemplateSelect, // Choose template
    ClipArt,        // Clip art browser
    TextEdit,       // Editing text frame
    Properties,     // Frame properties
    Export,         // Export dialog
    Help,
}

pub enum Tool {
    Select,         // Select/move frames
    TextFrame,      // Create text frame
    ImageFrame,     // Create image frame
    ClipArtFrame,   // Insert clip art
}

pub enum FrameContent {
    Text { text: String, font_size: u8, alignment: Alignment },
    Image { path: PathBuf, scale: f32 },
    ClipArt { id: String },
}

pub struct Frame {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub content: FrameContent,
    pub border: bool,
    pub rotation: u8,  // 0, 90, 180, 270
}

pub struct Page {
    pub width: u16,   // In characters
    pub height: u16,
    pub frames: Vec<Frame>,
}

pub struct QDesignState {
    pub view: QDesignView,
    pub tool: Tool,

    // Document
    pub pages: Vec<Page>,
    pub current_page: usize,
    pub title: String,
    pub file_path: Option<PathBuf>,
    pub modified: bool,

    // Selection
    pub selected_frame: Option<usize>,
    pub cursor_x: u16,
    pub cursor_y: u16,

    // Clipboard
    pub clipboard: Option<Frame>,

    // History
    pub undo_stack: Vec<Vec<Page>>,
    pub redo_stack: Vec<Vec<Page>>,

    // Templates
    pub templates: Vec<Template>,
    pub template_cursor: usize,

    // Clip art
    pub clipart_cursor: usize,
}
```

## Views

### Main Canvas View
```
╔══════════════════════════════════════ Q-DESIGN ═══════════════════╗
║ Select  TextFrame  ImageFrame  ClipArt    │ untitled.qds *        ║
╠═══════════════════════════════════════════╧══════════════════════╣
║    1    2    3    4    5    6    7    8    9   10   11   12      ║
║  1 ┌─────────────────────────────────────────────────────────┐   ║
║  2 │                                                         │   ║
║  3 │   ┌─────────────────────────────────┐                   │   ║
║  4 │   │      HAPPY BIRTHDAY!            │                   │   ║
║  5 │   │                                 │                   │   ║
║  6 │   └─────────────────────────────────┘                   │   ║
║  7 │                                                         │   ║
║  8 │        [SELECTED IMAGE FRAME]                           │   ║
║  9 │        ┌───────────────────┐                            │   ║
║ 10 │        │                   │                            │   ║
║ 11 │        │    (sixel img)    │                            │   ║
║ 12 │        │                   │                            │   ║
║ 13 │        └───────────────────┘                            │   ║
║ 14 │                                                         │   ║
║ 15 └─────────────────────────────────────────────────────────┘   ║
╠══════════════════════════════════════════════════════════════════╣
║ Frame: Image (cake.png) 20x10 at (15,8)   Page 1/1   Tool:Select ║
╚══════════════════════════════════════════════════════════════════╝
 Arrows:Move  T:Text  I:Image  C:ClipArt  Del:Delete  Ctrl+S:Save
```

### Template Selection
```
╔═════════════════════ Q-DESIGN: Templates ═════════════════════════╗
║                                                                   ║
║   [>] Birthday Card     4x6 greeting card                         ║
║   [ ] Business Card     3.5x2 standard card                       ║
║   [ ] Flyer             8.5x11 full page                          ║
║   [ ] Banner            Large horizontal banner                   ║
║   [ ] Poster            11x17 poster                              ║
║   [ ] Invitation        5x7 formal invitation                     ║
║   [ ] Blank             Start from scratch                        ║
║                                                                   ║
╚═══════════════════════════════════════════════════════════════════╝
 ↑↓:Select  Enter:Choose  Esc:Cancel
```

## Key Bindings

| Key | Action |
|-----|--------|
| Arrow keys | Move cursor/selection |
| Tab | Cycle through tools |
| S | Select tool |
| T | Text frame tool |
| I | Image frame tool |
| C | Open clip art browser |
| Enter | Create frame / Edit selected |
| Delete | Delete selected frame |
| Ctrl+C | Copy frame |
| Ctrl+V | Paste frame |
| Ctrl+Z | Undo |
| Ctrl+Y | Redo |
| Ctrl+S | Save |
| Ctrl+E | Export |
| Ctrl+N | New from template |
| PgUp/PgDn | Switch pages |
| Esc | Deselect / Exit |

## Implementation Phases

### Phase 1: Core Structure
1. Create crate skeleton with Cargo.toml
2. Implement state types (Page, Frame, QDesignState)
3. Implement Plugin trait boilerplate
4. Add to workspace Cargo.toml

### Phase 2: Canvas Rendering
1. Draw page outline with rulers
2. Render frames (text boxes, placeholders)
3. Selection highlighting
4. Cursor movement

### Phase 3: Text Frames
1. Create text frame tool
2. Text editing mode
3. Basic styling (size, alignment)

### Phase 4: Templates
1. Template data structures
2. Built-in templates (card, flyer, banner)
3. Template selection UI

### Phase 5: Clip Art & Images
1. Clip art library (simple ASCII art collection)
2. Clip art browser
3. Image frame with sixel preview

### Phase 6: Export
1. ASCII art export (for terminal printing)
2. PNG export using image crate

### Phase 7: Polish
1. Undo/redo
2. Copy/paste
3. Help screen
4. Integration with Office Suite

## File Modifications

| File | Changes |
|------|---------|
| `Cargo.toml` (workspace) | Add `qdos-plugin-qdesign` to members |
| `crates/qdos-plugin-qdesign/*` | **NEW** - All plugin files |
| `src/plugins/mod.rs` | Add import for QDesignPlugin |
| `src/app/mod.rs` | Register QDesignPlugin |
| `src/plugins/office/mod.rs` | Add to Office Suite menu |

## Verification

1. `cargo build -p qdos-plugin-qdesign` - Plugin compiles
2. `cargo run` -> Open Q-DESIGN from Office Suite
3. Test templates: Choose template, verify page layout
4. Test text frames: Create, edit, move text
5. Test clip art: Insert clip art from browser
6. Test export: Export to ASCII art
7. Quality checks: `cargo fmt -- --check && cargo clippy -- -D warnings`
