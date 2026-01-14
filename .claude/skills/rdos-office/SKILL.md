# R-DOS Office Suite Skill

Use this skill when implementing Office applications: Q-SHEET, Q-DECK, Q-WEB, Q-DOCS,
Q-CODE, Q-PAINT, Q-MAIL, Q-FORM, Q-DESIGN, or Q-MIDI.

## Quick Reference

**Spec**: `spec/OFFICE.md` - Full suite specification
**Code**: `src/plugins/office/`

## Suite Overview

| App | Type | Priority | Key Feature |
|-----|------|----------|-------------|
| Q-SHEET | Spreadsheet | 1 | VisiCalc/Lotus 1-2-3 style CSV editor |
| Q-DECK | Presentation | 2 | ANSI/ASCII slideshow editor |
| Q-WEB | Browser | 3 | Lynx-style reader mode |
| Q-DOCS | Word Processor | 4 | WordPerfect-style MD/DOC editor |
| Q-CODE | IDE | 5 | VIM-style editor with LSP |
| Q-PAINT | Graphics | 6 | DeluxePaint-style pixel art |
| Q-MAIL | Email | 7 | Pine/Mutt-style client |
| Q-FORM | Forms | 8 | Form designer |
| Q-DESIGN | Publishing | 9 | PrintMaster Gold-style cards |
| Q-MIDI | Music | 10 | Tracker-style sequencer |

## Architecture

```
src/plugins/office/
├── mod.rs              # Office plugin registration
├── shared/
│   ├── mod.rs
│   ├── document.rs     # OfficeDocument trait
│   ├── text.rs         # TextBuffer, TextEditor
│   ├── clipboard.rs    # Clipboard operations
│   ├── help.rs         # Help system
│   └── ai.rs           # AI integration
├── formats/            # File format parsers
│   ├── csv.rs
│   ├── markdown.rs
│   ├── html.rs
│   └── ...
└── [app]/              # Individual apps
```

## OfficeDocument Trait

All document types MUST implement:

```rust
pub trait OfficeDocument {
    /// File extensions this document type supports
    fn extensions() -> &'static [&'static str];

    /// Create new empty document
    fn new() -> Self;

    /// Load from file
    fn load(path: &Path) -> Result<Self, OfficeError>;

    /// Save to file
    fn save(&self, path: &Path) -> Result<(), OfficeError>;

    /// Export to different format
    fn export(&self, path: &Path, format: ExportFormat) -> Result<(), OfficeError>;

    /// Check if document has unsaved changes
    fn is_modified(&self) -> bool;

    /// Get document metadata
    fn metadata(&self) -> DocumentMetadata;
}
```

## TextEditor Trait

For text-based apps (Q-DOCS, Q-CODE):

```rust
pub trait TextEditor {
    fn buffer(&self) -> &TextBuffer;
    fn buffer_mut(&mut self) -> &mut TextBuffer;

    // Navigation
    fn move_cursor(&mut self, motion: Motion);
    fn goto_line(&mut self, line: usize);

    // Editing
    fn insert(&mut self, text: &str);
    fn delete(&mut self, range: Range);
    fn undo(&mut self);
    fn redo(&mut self);

    // Selection
    fn copy(&mut self);
    fn cut(&mut self);
    fn paste(&mut self);

    // Search
    fn find(&mut self, query: &str, opts: FindOptions) -> Vec<Match>;
    fn replace(&mut self, from: &str, to: &str, opts: ReplaceOptions);
}
```

## Universal Keybindings

All office apps share these bindings:

| Key | Action |
|-----|--------|
| F1 | Help |
| F2 | Save |
| F3 | Open |
| F10 | Menu |
| Ctrl+Q | Quit |
| Ctrl+S | Save |
| Ctrl+Z | Undo |
| Ctrl+Y | Redo |
| Ctrl+C | Copy |
| Ctrl+V | Paste |
| Ctrl+X | Cut |
| Ctrl+F | Find |

## UI Components

### Extended components for office apps:

```rust
// Document tabs (for multi-document editing)
pub struct TabBar {
    tabs: Vec<Tab>,
    active: usize,
}

// Toolbar with tool icons
pub struct Toolbar {
    tools: Vec<ToolButton>,
    selected: usize,
}

// Status bar with document info
pub struct DocumentStatusBar {
    filename: String,
    modified: bool,
    position: Position,
    mode: String,
    encoding: String,
}

// Grid for spreadsheet
pub struct Grid {
    columns: Vec<Column>,
    rows: Vec<Row>,
    frozen_rows: usize,
    frozen_cols: usize,
    selection: CellRange,
}

// Canvas for graphics apps
pub struct Canvas {
    width: usize,
    height: usize,
    pixels: Vec<Vec<Color>>,
    zoom: f32,
}

// Timeline for MIDI/presentations
pub struct Timeline {
    tracks: Vec<Track>,
    position: usize,
    zoom: f32,
    playing: bool,
}
```

## AI Integration

Office apps can use AI for assistance:

```rust
pub enum AIAssistant {
    Writing { mode: WritingMode },      // Q-DOCS, Q-MAIL
    Formula { context: Vec<CellRef> },  // Q-SHEET
    Code { language: String },          // Q-CODE
    Creative { style: String },         // Q-DESIGN, Q-PAINT
    Presentation { topic: String },     // Q-DECK
}

pub trait AIEnabled {
    fn ai_available(&self) -> bool;
    fn ai_suggest(&self) -> Vec<AISuggestion>;
    fn ai_apply(&mut self, suggestion: &AISuggestion);
    fn ai_generate(&mut self, prompt: &str) -> Result<String, AIError>;
}
```

## Creating an Office App

### 1. Create app module structure:
```
src/plugins/office/myapp/
├── mod.rs        # App state and plugin impl
├── state.rs      # App-specific state
├── ops.rs        # Operations/commands
└── modal.rs      # Rendering
```

### 2. Implement OfficeDocument:
```rust
pub struct MyDocument {
    path: Option<PathBuf>,
    modified: bool,
    content: Vec<String>,
}

impl OfficeDocument for MyDocument {
    fn extensions() -> &'static [&'static str] { &["myext"] }
    fn new() -> Self { ... }
    fn load(path: &Path) -> Result<Self, OfficeError> { ... }
    fn save(&self, path: &Path) -> Result<(), OfficeError> { ... }
    // ...
}
```

### 3. Create app state:
```rust
pub struct MyAppState {
    pub view: MyAppView,
    pub document: MyDocument,
    // app-specific state...
}
```

### 4. Register as plugin following standard plugin patterns

## Q-DOS Aesthetic Guidelines

All office apps MUST:
- Use DOS-era UI with modern features
- Use component library (FullScreenView, etc.)
- Provide F1 context-sensitive help
- Handle errors with Q-DOS style messages
- Show [Modified] indicator when unsaved
- Display file info in status bar
- Support keyboard-first navigation

## Screen Layout Pattern

```
 Q-APP: filename.ext                                              [Modified]
═══════════════════════════════════════════════════════════════════════════════
 [Menu Bar or Toolbar]
───────────────────────────────────────────────────────────────────────────────

                           [ Main Content Area ]

═══════════════════════════════════════════════════════════════════════════════
 Status Bar: position, mode, encoding, etc.
───────────────────────────────────────────────────────────────────────────────
 F1 Help  F2 Save  F3 Open  F5 Action  F10 Menu  ESC Quit
```

## Quality Standards

Before submitting office app changes:
- [ ] `cargo fmt -- --check`
- [ ] `cargo clippy -- -D warnings`
- [ ] `cargo build`
- [ ] Implements OfficeDocument trait
- [ ] Universal keybindings work
- [ ] F1 help is available
- [ ] Modified indicator shows correctly
- [ ] File operations work (new, open, save)
