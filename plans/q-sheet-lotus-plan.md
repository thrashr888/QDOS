# Q-SHEET Enhancement Plan: Lotus 1-2-3 Style Features

Enhance Q-SHEET with Lotus 1-2-3 style UI and Save As dialog.

**Issues:** QDOS-imd8 (Shared Infrastructure), QDOS-flo2 (Q-SHEET)

---

## Overview

Add three major features to Q-SHEET:
1. **Lotus 1-2-3 Style Menu Bar** - Two-row menu activated by `/` key
2. **Save As Dialog** - Modal for saving files with path input (shared component)
3. **XLSX File Format Support** - Read/write Excel files

---

## Phase 1: Shared Office Infrastructure (QDOS-imd8)

### Add to `src/plugins/office/shared/`

**New file: `menu.rs`** - Reusable Lotus-style menu bar component
```rust
pub trait MenuBar {
    type Category;
    type Item;
    fn categories() -> &'static [Self::Category];
    fn items(category: &Self::Category) -> &'static [Self::Item];
}

pub struct MenuState {
    pub active: bool,
    pub category_index: usize,
    pub item_index: usize,
}

pub fn draw_menu_bar<M: MenuBar>(...);
pub fn handle_menu_key<M: MenuBar>(...);
```

**New file: `saveas.rs`** - Reusable Save As dialog
```rust
pub struct SaveAsState {
    pub input: String,
    pub cursor: usize,
    pub base_dir: PathBuf,
}

pub fn draw_save_as_modal(...);
pub fn handle_save_as_key(...) -> SaveAsResult;
pub fn tab_complete(input: &str, cwd: &Path) -> Option<String>;
```

**New file: `formats.rs`** - Shared format detection
```rust
pub enum FileFormat { Csv, Xlsx, Txt, Rtf }
pub fn detect_format(path: &Path) -> Option<FileFormat>;
```

### Files to Create/Modify
| File | Purpose |
|------|---------|
| `src/plugins/office/shared/menu.rs` | Reusable menu bar |
| `src/plugins/office/shared/saveas.rs` | Reusable Save As dialog |
| `src/plugins/office/shared/formats.rs` | Format detection |
| `src/plugins/office/shared/mod.rs` | Export new modules |

---

## Phase 2: Q-SHEET Menu System

### Menu Bar Layout (Lotus 1-2-3 Style)
```
 Q-SHEET: budget.csv                                              [Modified]
═══════════════════════════════════════════════════════════════════════════════
 Worksheet  Range  Copy  Move  File  Print  Graph  Data  System  Quit    <- Row 0
 New  Open  Save  SaveAs  Quit                                           <- Row 1 (submenu)
───────────────────────────────────────────────────────────────────────────────
     │    A         │    B         │    C         │ ...
```

### State Changes (`state.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SheetMode {
    #[default]
    Navigate,
    Edit,
    Menu,      // NEW: Lotus menu active
    SaveAs,    // NEW: Save As dialog active
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuCategory {
    Worksheet, Range, Copy, Move, File, Print, Graph, Data, System, Quit
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileMenuItem {
    New, Open, Save, SaveAs, Quit
}
```

### Key Handling
- `/` key opens menu (classic Lotus command)
- Arrow keys navigate categories (Left/Right) and items (Up/Down)
- Letter keys quick-select (W=Worksheet, F=File, etc.)
- Enter executes, Esc closes menu

### Initial Menu Implementation
Only File menu functional initially:
- **New** - Clear sheet, prompt if modified
- **Open** - Not implemented yet (show message)
- **Save** - Save to current path or open Save As
- **SaveAs** - Open Save As dialog
- **Quit** - Close spreadsheet

---

## Phase 3: Save As Dialog

### Behavior
- **Ctrl+S** with no file path → opens Save As
- **File > SaveAs** from menu → opens Save As
- Shows current directory, accepts filename input
- Tab completion for paths
- Validates and adds extension if missing

### Dialog Layout (using ModalFrame)
```
╔══════════════════════ SAVE AS ══════════════════════╗
║                                                      ║
║  Directory: /Users/name/Documents                    ║
║                                                      ║
║  Filename: budget█                                   ║
║                                                      ║
║  (Use .csv or .xlsx extension)                       ║
║                                                      ║
╠══════════════════════════════════════════════════════╣
║  Tab complete   Enter save   Esc cancel              ║
╚══════════════════════════════════════════════════════╝
```

---

## Phase 4: XLSX File Format Support

### Dependencies (add to `Cargo.toml`)
```toml
rust_xlsxwriter = "0.80"  # Writing xlsx
calamine = "0.26"         # Reading xlsx
```

### File Structure Change
```
src/plugins/office/sheet/
├── mod.rs
├── state.rs
├── modal.rs
├── formula.rs
└── formats/           # NEW directory
    ├── mod.rs         # Format detection, dispatch
    ├── csv.rs         # Moved from csv.rs
    └── xlsx.rs        # NEW: Excel support
```

### Format Detection
```rust
pub fn detect_format(path: &Path) -> Option<FileFormat> {
    match path.extension()?.to_str()?.to_lowercase().as_str() {
        "csv" | "tsv" => Some(FileFormat::Csv),
        "xlsx" | "xls" => Some(FileFormat::Xlsx),
        _ => None,
    }
}
```

---

## Implementation Order

### Step 1: Shared Infrastructure
1. Create `src/plugins/office/shared/saveas.rs` with SaveAsState and helpers
2. Create `src/plugins/office/shared/formats.rs` with format detection
3. Update `src/plugins/office/shared/mod.rs` to export new modules

### Step 2: Menu State
4. Add `MenuCategory`, `FileMenuItem` enums to `state.rs`
5. Add `Menu` and `SaveAs` variants to `SheetMode`
6. Add menu state fields to `SheetState`

### Step 3: Menu Rendering
7. Add `draw_menu_bar()` function to `modal.rs`
8. Update `draw_sheet_modal()` to show menu bar in Menu mode
9. Adjust grid rendering to account for menu rows

### Step 4: Menu Key Handling
10. Add `/` key to enter Menu mode in `handle_navigate_key()`
11. Create `handle_menu_key()` function in `mod.rs`
12. Implement `execute_menu_action()` for File menu items

### Step 5: Save As Dialog
13. Implement `handle_save_as_key()` in `mod.rs`
14. Add `draw_save_as_modal()` to `modal.rs`
15. Wire Ctrl+S to open Save As when no file path

### Step 6: XLSX Support
16. Add dependencies to `Cargo.toml`
17. Create `formats/` directory structure
18. Move `csv.rs` to `formats/csv.rs`
19. Create `formats/xlsx.rs` with load/save
20. Create `formats/mod.rs` with dispatch
21. Update imports throughout

### Step 7: Testing
22. Run `cargo fmt -- --check`
23. Run `cargo clippy -- -D warnings`
24. Run `cargo test`
25. Manual testing (see verification below)

---

## Files to Create

| File | Lines | Purpose |
|------|-------|---------|
| `shared/saveas.rs` | ~100 | Reusable Save As dialog |
| `shared/formats.rs` | ~30 | Format detection |
| `sheet/formats/mod.rs` | ~40 | Format dispatch |
| `sheet/formats/xlsx.rs` | ~120 | Excel read/write |

## Files to Modify

| File | Changes |
|------|---------|
| `shared/mod.rs` | Export new modules |
| `sheet/state.rs` | Add menu enums, mode variants, state fields |
| `sheet/mod.rs` | Add menu and save-as key handlers |
| `sheet/modal.rs` | Add menu bar and save-as rendering |
| `sheet/csv.rs` | Move to formats/csv.rs |
| `Cargo.toml` | Add rust_xlsxwriter, calamine |

---

## Verification

```bash
# Quality gates
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test

# Manual testing checklist:
# 1. F12 > S opens Q-SHEET directly
# 2. F12 > O opens Office Suite menu, select Q-SHEET
# 3. Press `/` - menu bar appears with categories highlighted
# 4. Arrow Left/Right - navigate between categories
# 5. Press `F` - File category selected, submenu shown
# 6. Arrow Down - navigate File submenu items
# 7. Press `A` or navigate to SaveAs, press Enter
# 8. Save As dialog appears with cursor
# 9. Type "test.csv", press Enter - file saved
# 10. Ctrl+S - saves without dialog (has path now)
# 11. New file, Ctrl+S - Save As dialog appears
# 12. Type "test.xlsx", press Enter - Excel file saved
# 13. Open test.xlsx in Excel - verify data correct
# 14. Esc from menu/dialog returns to Navigate mode
# 15. Esc from Navigate closes Q-SHEET
```

---

## Future Enhancements (Not This Phase)

- Multi-cell selection (Shift+arrows)
- Copy/paste (Ctrl+C/V) with selection
- Remaining menu categories (Range, Copy, Move, etc.)
- Column/row insert/delete
- Freeze panes
- Number formatting
