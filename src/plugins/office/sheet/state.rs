//! Q-SHEET State
//!
//! State management for the spreadsheet editor.

use super::formula;
use crate::plugins::office::shared::OfficeDocument;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// =============================================================================
// CONSTANTS
// =============================================================================

pub const MAX_COLS: usize = 26; // A-Z
pub const DEFAULT_COL_WIDTH: usize = 12;
pub const ROW_NUM_WIDTH: usize = 5; // "  1 |"

// =============================================================================
// CELL VALUE
// =============================================================================

/// Value stored in a cell
#[derive(Debug, Clone, PartialEq, Default)]
pub enum CellValue {
    #[default]
    Empty,
    Text(String),
    Number(f64),
    Formula {
        formula: String,
        cached: f64,
    },
    Error(String),
}

impl CellValue {
    /// Get display string for the cell
    pub fn display(&self) -> String {
        match self {
            CellValue::Empty => String::new(),
            CellValue::Text(s) => s.clone(),
            CellValue::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 1e10 {
                    format!("{}", *n as i64)
                } else {
                    format!("{:.2}", n)
                }
            }
            CellValue::Formula { cached, .. } => {
                if cached.fract() == 0.0 && cached.abs() < 1e10 {
                    format!("{}", *cached as i64)
                } else {
                    format!("{:.2}", cached)
                }
            }
            CellValue::Error(e) => format!("#ERR:{}", e),
        }
    }

    /// Get the formula text if this is a formula cell
    pub fn formula_text(&self) -> Option<&str> {
        match self {
            CellValue::Formula { formula, .. } => Some(formula),
            _ => None,
        }
    }

    /// Get numeric value if available
    pub fn as_number(&self) -> Option<f64> {
        match self {
            CellValue::Number(n) => Some(*n),
            CellValue::Formula { cached, .. } => Some(*cached),
            CellValue::Text(s) => s.parse().ok(),
            _ => None,
        }
    }

    /// Check if cell is empty
    pub fn is_empty(&self) -> bool {
        matches!(self, CellValue::Empty)
    }
}

// =============================================================================
// SHEET MODE
// =============================================================================

/// Editor mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SheetMode {
    #[default]
    Navigate,
    Edit,
    Menu,   // Lotus 1-2-3 style menu active
    SaveAs, // Save As dialog active
}

// =============================================================================
// LOTUS 1-2-3 STYLE MENU
// =============================================================================

/// Menu categories (Lotus 1-2-3 R4 style top row)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MenuCategory {
    #[default]
    Worksheet,
    Range,
    Copy,
    Move,
    File,
    Print,
    Graph,
    Data,
    Tools, // Lotus 1-2-3 R4 uses "Tools" instead of older "System"
    Quit,
}

impl MenuCategory {
    /// Get all categories in display order
    pub const fn all() -> &'static [MenuCategory] {
        &[
            MenuCategory::Worksheet,
            MenuCategory::Range,
            MenuCategory::Copy,
            MenuCategory::Move,
            MenuCategory::File,
            MenuCategory::Print,
            MenuCategory::Graph,
            MenuCategory::Data,
            MenuCategory::Tools,
            MenuCategory::Quit,
        ]
    }

    /// Get display name
    pub const fn name(&self) -> &'static str {
        match self {
            MenuCategory::Worksheet => "Worksheet",
            MenuCategory::Range => "Range",
            MenuCategory::Copy => "Copy",
            MenuCategory::Move => "Move",
            MenuCategory::File => "File",
            MenuCategory::Print => "Print",
            MenuCategory::Graph => "Graph",
            MenuCategory::Data => "Data",
            MenuCategory::Tools => "Tools",
            MenuCategory::Quit => "Quit",
        }
    }

    /// Get shortcut key
    pub const fn key(&self) -> char {
        match self {
            MenuCategory::Worksheet => 'W',
            MenuCategory::Range => 'R',
            MenuCategory::Copy => 'C',
            MenuCategory::Move => 'M',
            MenuCategory::File => 'F',
            MenuCategory::Print => 'P',
            MenuCategory::Graph => 'G',
            MenuCategory::Data => 'D',
            MenuCategory::Tools => 'T',
            MenuCategory::Quit => 'Q',
        }
    }

    /// Get from index
    pub fn from_index(index: usize) -> Self {
        Self::all().get(index).copied().unwrap_or_default()
    }
}

/// File menu items (Lotus 1-2-3 style submenu)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileMenuItem {
    #[default]
    Retrieve, // Open
    Save,
    Combine,
    Xtract,
    Erase,
    List,
    Import,
    Directory,
}

impl FileMenuItem {
    /// Get all file menu items in display order
    pub const fn all() -> &'static [FileMenuItem] {
        &[
            FileMenuItem::Retrieve,
            FileMenuItem::Save,
            FileMenuItem::Combine,
            FileMenuItem::Xtract,
            FileMenuItem::Erase,
            FileMenuItem::List,
            FileMenuItem::Import,
            FileMenuItem::Directory,
        ]
    }

    /// Get display name
    pub const fn name(&self) -> &'static str {
        match self {
            FileMenuItem::Retrieve => "Retrieve",
            FileMenuItem::Save => "Save",
            FileMenuItem::Combine => "Combine",
            FileMenuItem::Xtract => "Xtract",
            FileMenuItem::Erase => "Erase",
            FileMenuItem::List => "List",
            FileMenuItem::Import => "Import",
            FileMenuItem::Directory => "Directory",
        }
    }

    /// Get shortcut key
    pub const fn key(&self) -> char {
        match self {
            FileMenuItem::Retrieve => 'R',
            FileMenuItem::Save => 'S',
            FileMenuItem::Combine => 'C',
            FileMenuItem::Xtract => 'X',
            FileMenuItem::Erase => 'E',
            FileMenuItem::List => 'L',
            FileMenuItem::Import => 'I',
            FileMenuItem::Directory => 'D',
        }
    }

    /// Get from index
    pub fn from_index(index: usize) -> Self {
        Self::all().get(index).copied().unwrap_or_default()
    }
}

// =============================================================================
// SHEET STATE
// =============================================================================

/// Main spreadsheet state
pub struct SheetState {
    // Document info
    pub file_path: Option<PathBuf>,
    pub modified: bool,

    // Cell data - sparse storage (col, row) -> value
    pub cells: HashMap<(usize, usize), CellValue>,
    pub row_count: usize,

    // Cursor position
    pub cursor_col: usize,
    pub cursor_row: usize,

    // Viewport
    pub scroll_col: usize,
    pub scroll_row: usize,

    // Editing
    pub mode: SheetMode,
    pub edit_buffer: String,
    pub edit_cursor: usize,

    // Column widths
    pub col_widths: [usize; MAX_COLS],

    // Animation
    pub tick_count: u32,

    // Menu state (Lotus 1-2-3 style)
    pub menu_category: usize,
    pub menu_item: usize,

    // Save As dialog state
    pub save_as_input: String,
    pub save_as_cursor: usize,

    // Status message (message, ticks remaining)
    pub status_message: Option<(String, u32)>,
}

impl Default for SheetState {
    fn default() -> Self {
        Self::new()
    }
}

impl SheetState {
    pub fn new() -> Self {
        Self {
            file_path: None,
            modified: false,
            cells: HashMap::new(),
            row_count: 100,
            cursor_col: 0,
            cursor_row: 0,
            scroll_col: 0,
            scroll_row: 0,
            mode: SheetMode::Navigate,
            edit_buffer: String::new(),
            edit_cursor: 0,
            col_widths: [DEFAULT_COL_WIDTH; MAX_COLS],
            tick_count: 0,
            menu_category: 0,
            menu_item: 0,
            save_as_input: String::new(),
            save_as_cursor: 0,
            status_message: None,
        }
    }

    // =========================================================================
    // MENU NAVIGATION
    // =========================================================================

    /// Enter menu mode
    pub fn enter_menu_mode(&mut self) {
        self.mode = SheetMode::Menu;
        self.menu_category = 4; // Default to File (most common)
        self.menu_item = 0;
    }

    /// Exit menu mode
    pub fn exit_menu_mode(&mut self) {
        self.mode = SheetMode::Navigate;
    }

    /// Move menu category left
    pub fn menu_left(&mut self) {
        if self.menu_category > 0 {
            self.menu_category -= 1;
            self.menu_item = 0;
        }
    }

    /// Move menu category right
    pub fn menu_right(&mut self) {
        if self.menu_category < MenuCategory::all().len() - 1 {
            self.menu_category += 1;
            self.menu_item = 0;
        }
    }

    /// Move menu item up
    pub fn menu_up(&mut self) {
        if self.menu_item > 0 {
            self.menu_item -= 1;
        }
    }

    /// Move menu item down
    pub fn menu_down(&mut self) {
        // Only File menu has items for now
        if MenuCategory::from_index(self.menu_category) == MenuCategory::File
            && self.menu_item < FileMenuItem::all().len() - 1
        {
            self.menu_item += 1;
        }
    }

    /// Select menu category by key
    pub fn menu_select_category(&mut self, key: char) -> bool {
        let upper = key.to_ascii_uppercase();
        for (i, cat) in MenuCategory::all().iter().enumerate() {
            if cat.key() == upper {
                self.menu_category = i;
                self.menu_item = 0;
                return true;
            }
        }
        false
    }

    /// Select menu item by key (within File menu)
    pub fn menu_select_item(&mut self, key: char) -> bool {
        if MenuCategory::from_index(self.menu_category) == MenuCategory::File {
            let upper = key.to_ascii_uppercase();
            for (i, item) in FileMenuItem::all().iter().enumerate() {
                if item.key() == upper {
                    self.menu_item = i;
                    return true;
                }
            }
        }
        false
    }

    /// Get current menu category
    pub fn current_menu_category(&self) -> MenuCategory {
        MenuCategory::from_index(self.menu_category)
    }

    /// Get current file menu item
    pub fn current_file_menu_item(&self) -> FileMenuItem {
        FileMenuItem::from_index(self.menu_item)
    }

    // =========================================================================
    // SAVE AS DIALOG
    // =========================================================================

    /// Enter Save As mode
    pub fn enter_save_as_mode(&mut self) {
        self.mode = SheetMode::SaveAs;
        self.save_as_input.clear();
        self.save_as_cursor = 0;
    }

    /// Exit Save As mode
    pub fn exit_save_as_mode(&mut self) {
        self.mode = SheetMode::Navigate;
        self.save_as_input.clear();
        self.save_as_cursor = 0;
    }

    /// Insert character in Save As input
    pub fn save_as_insert(&mut self, c: char) {
        self.save_as_input.insert(self.save_as_cursor, c);
        self.save_as_cursor += 1;
    }

    /// Backspace in Save As input
    pub fn save_as_backspace(&mut self) {
        if self.save_as_cursor > 0 {
            self.save_as_cursor -= 1;
            self.save_as_input.remove(self.save_as_cursor);
        }
    }

    /// Delete in Save As input
    pub fn save_as_delete(&mut self) {
        if self.save_as_cursor < self.save_as_input.len() {
            self.save_as_input.remove(self.save_as_cursor);
        }
    }

    /// Move Save As cursor left
    pub fn save_as_cursor_left(&mut self) {
        if self.save_as_cursor > 0 {
            self.save_as_cursor -= 1;
        }
    }

    /// Move Save As cursor right
    pub fn save_as_cursor_right(&mut self) {
        if self.save_as_cursor < self.save_as_input.len() {
            self.save_as_cursor += 1;
        }
    }

    /// Get full path for Save As
    pub fn save_as_full_path(&self, base_dir: &Path) -> PathBuf {
        let filename = if self.save_as_input.is_empty() {
            "untitled.csv".to_string()
        } else if !self.save_as_input.contains('.') {
            format!("{}.csv", self.save_as_input)
        } else {
            self.save_as_input.clone()
        };
        base_dir.join(filename)
    }

    // =========================================================================
    // CELL ACCESS
    // =========================================================================

    /// Get cell value at position
    pub fn get_cell(&self, col: usize, row: usize) -> &CellValue {
        self.cells.get(&(col, row)).unwrap_or(&CellValue::Empty)
    }

    /// Set cell value at position
    pub fn set_cell(&mut self, col: usize, row: usize, value: CellValue) {
        if value.is_empty() {
            self.cells.remove(&(col, row));
        } else {
            self.cells.insert((col, row), value);
        }
        self.modified = true;
        // Expand row count if needed
        if row >= self.row_count {
            self.row_count = row + 1;
        }
    }

    /// Get cell display string
    pub fn get_cell_display(&self, col: usize, row: usize) -> String {
        self.get_cell(col, row).display()
    }

    // =========================================================================
    // NAVIGATION
    // =========================================================================

    pub fn move_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.ensure_visible();
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor_row < self.row_count.saturating_sub(1) {
            self.cursor_row += 1;
            self.ensure_visible();
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
            self.ensure_visible();
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor_col < MAX_COLS - 1 {
            self.cursor_col += 1;
            self.ensure_visible();
        }
    }

    pub fn move_home(&mut self) {
        self.cursor_col = 0;
        self.ensure_visible();
    }

    pub fn move_end(&mut self) {
        // Find last non-empty column in current row
        let mut last_col = 0;
        for col in 0..MAX_COLS {
            if !self.get_cell(col, self.cursor_row).is_empty() {
                last_col = col;
            }
        }
        self.cursor_col = last_col;
        self.ensure_visible();
    }

    pub fn page_up(&mut self, visible_rows: usize) {
        self.cursor_row = self.cursor_row.saturating_sub(visible_rows);
        self.ensure_visible();
    }

    pub fn page_down(&mut self, visible_rows: usize) {
        self.cursor_row = (self.cursor_row + visible_rows).min(self.row_count.saturating_sub(1));
        self.ensure_visible();
    }

    /// Ensure cursor is visible in viewport
    pub fn ensure_visible(&mut self) {
        // Vertical scrolling - assume ~15 visible rows
        let visible_rows = 15;
        if self.cursor_row < self.scroll_row {
            self.scroll_row = self.cursor_row;
        } else if self.cursor_row >= self.scroll_row + visible_rows {
            self.scroll_row = self.cursor_row - visible_rows + 1;
        }

        // Horizontal scrolling - assume ~5 visible columns
        let visible_cols = 5;
        if self.cursor_col < self.scroll_col {
            self.scroll_col = self.cursor_col;
        } else if self.cursor_col >= self.scroll_col + visible_cols {
            self.scroll_col = self.cursor_col - visible_cols + 1;
        }
    }

    // =========================================================================
    // EDITING
    // =========================================================================

    /// Enter edit mode for current cell
    pub fn enter_edit_mode(&mut self) {
        self.mode = SheetMode::Edit;
        // Load current cell content into edit buffer
        let cell = self.get_cell(self.cursor_col, self.cursor_row);
        self.edit_buffer = match cell {
            CellValue::Formula { formula, .. } => formula.clone(),
            CellValue::Text(s) => s.clone(),
            CellValue::Number(n) => {
                if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            CellValue::Error(e) => e.clone(),
            CellValue::Empty => String::new(),
        };
        self.edit_cursor = self.edit_buffer.len();
    }

    /// Start typing a new value (clears cell first)
    pub fn start_typing(&mut self, c: char) {
        self.mode = SheetMode::Edit;
        self.edit_buffer = c.to_string();
        self.edit_cursor = 1;
    }

    /// Insert character at edit cursor
    pub fn edit_insert(&mut self, c: char) {
        self.edit_buffer.insert(self.edit_cursor, c);
        self.edit_cursor += 1;
    }

    /// Delete character before edit cursor
    pub fn edit_backspace(&mut self) {
        if self.edit_cursor > 0 {
            self.edit_cursor -= 1;
            self.edit_buffer.remove(self.edit_cursor);
        }
    }

    /// Delete character at edit cursor
    pub fn edit_delete(&mut self) {
        if self.edit_cursor < self.edit_buffer.len() {
            self.edit_buffer.remove(self.edit_cursor);
        }
    }

    /// Move edit cursor left
    pub fn edit_cursor_left(&mut self) {
        if self.edit_cursor > 0 {
            self.edit_cursor -= 1;
        }
    }

    /// Move edit cursor right
    pub fn edit_cursor_right(&mut self) {
        if self.edit_cursor < self.edit_buffer.len() {
            self.edit_cursor += 1;
        }
    }

    /// Move edit cursor to start
    pub fn edit_cursor_home(&mut self) {
        self.edit_cursor = 0;
    }

    /// Move edit cursor to end
    pub fn edit_cursor_end(&mut self) {
        self.edit_cursor = self.edit_buffer.len();
    }

    /// Confirm edit and save to cell
    pub fn confirm_edit(&mut self) {
        let value = self.parse_input(&self.edit_buffer.clone());
        self.set_cell(self.cursor_col, self.cursor_row, value);
        self.mode = SheetMode::Navigate;
        self.edit_buffer.clear();
        self.edit_cursor = 0;

        // Recalculate all formulas
        self.recalculate();
    }

    /// Cancel edit and discard changes
    pub fn cancel_edit(&mut self) {
        self.mode = SheetMode::Navigate;
        self.edit_buffer.clear();
        self.edit_cursor = 0;
    }

    /// Parse input string to cell value
    fn parse_input(&self, input: &str) -> CellValue {
        let trimmed = input.trim();

        if trimmed.is_empty() {
            return CellValue::Empty;
        }

        // Check for formula
        if trimmed.starts_with('=') {
            let result = formula::evaluate(trimmed, &self.cells);
            match result {
                Ok(n) => CellValue::Formula {
                    formula: trimmed.to_string(),
                    cached: n,
                },
                Err(e) => CellValue::Error(e),
            }
        }
        // Try parsing as number
        else if let Ok(n) = trimmed.parse::<f64>() {
            CellValue::Number(n)
        }
        // Otherwise it's text
        else {
            CellValue::Text(trimmed.to_string())
        }
    }

    /// Recalculate all formula cells
    pub fn recalculate(&mut self) {
        // Collect formula cells
        let formula_cells: Vec<(usize, usize, String)> = self
            .cells
            .iter()
            .filter_map(|((col, row), value)| {
                if let CellValue::Formula { formula, .. } = value {
                    Some((*col, *row, formula.clone()))
                } else {
                    None
                }
            })
            .collect();

        // Recalculate each formula
        for (col, row, formula_str) in formula_cells {
            let result = formula::evaluate(&formula_str, &self.cells);
            match result {
                Ok(n) => {
                    self.cells.insert(
                        (col, row),
                        CellValue::Formula {
                            formula: formula_str,
                            cached: n,
                        },
                    );
                }
                Err(e) => {
                    self.cells.insert((col, row), CellValue::Error(e));
                }
            }
        }
    }

    // =========================================================================
    // CELL ADDRESS HELPERS
    // =========================================================================

    /// Convert column index to letter (0 -> A, 25 -> Z)
    pub fn col_to_letter(col: usize) -> char {
        (b'A' + col as u8) as char
    }

    /// Convert letter to column index (A -> 0, Z -> 25)
    pub fn letter_to_col(letter: char) -> Option<usize> {
        let upper = letter.to_ascii_uppercase();
        if upper.is_ascii_uppercase() {
            Some((upper as u8 - b'A') as usize)
        } else {
            None
        }
    }

    /// Get cell address string (e.g., "A1", "B5")
    pub fn cell_address(&self) -> String {
        format!(
            "{}{}",
            Self::col_to_letter(self.cursor_col),
            self.cursor_row + 1
        )
    }
}

// =============================================================================
// OFFICE DOCUMENT TRAIT
// =============================================================================

impl OfficeDocument for SheetState {
    fn extensions() -> &'static [&'static str] {
        &["csv", "tsv", "xlsx"]
    }

    fn new_document() -> Self {
        Self::new()
    }

    fn load(path: &Path) -> Result<Self, String> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "xlsx" => super::xlsx::load_xlsx(path),
            _ => super::csv::load_csv(path),
        }
    }

    fn save(&self, path: &Path) -> Result<(), String> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "xlsx" => super::xlsx::save_xlsx(self, path),
            _ => super::csv::save_csv(self, path),
        }
    }

    fn is_modified(&self) -> bool {
        self.modified
    }

    fn display_name(&self) -> String {
        self.file_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Untitled".to_string())
    }
}
