//! Command Palette state types

use qdos_plugin_api::NavItem;
use std::path::PathBuf;

/// Category for palette results
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PaletteCategory {
    /// Calculator result (highest priority)
    Calculator,
    /// Menu commands (Directory, Copy, Move, etc.)
    Commands,
    /// Plugins/Apps
    Apps,
    /// File search results
    Files,
}

impl PaletteCategory {
    pub fn label(&self) -> &'static str {
        match self {
            PaletteCategory::Calculator => "Calc",
            PaletteCategory::Commands => "Command",
            PaletteCategory::Apps => "App",
            PaletteCategory::Files => "File",
        }
    }
}

/// Action to take when a result is selected
#[derive(Debug, Clone)]
pub enum PaletteAction {
    /// Open a plugin by ID
    OpenPlugin(String),
    /// Execute a menu command
    ExecuteCommand(NavItem),
    /// Navigate to a file (select it in the file list)
    NavigateFile(PathBuf),
    /// Copy result to clipboard (calculator)
    CopyToClipboard(String),
}

/// A single result in the palette
#[derive(Debug, Clone)]
pub struct PaletteResult {
    pub category: PaletteCategory,
    pub label: String,
    pub description: String,
    pub score: u32,
    pub action: PaletteAction,
}

impl PaletteResult {
    pub fn calculator(value: f64) -> Self {
        let formatted = if value.fract() == 0.0 && value.abs() < 1e15 {
            format!("{}", value as i64)
        } else {
            format!("{:.6}", value)
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        };
        Self {
            category: PaletteCategory::Calculator,
            label: format!("= {}", formatted),
            description: "Press Enter to copy".to_string(),
            score: 10000, // Always highest priority
            action: PaletteAction::CopyToClipboard(formatted),
        }
    }

    pub fn command(item: NavItem, score: u32) -> Self {
        Self {
            category: PaletteCategory::Commands,
            label: item.as_str().to_string(),
            description: item.description().to_string(),
            score,
            action: PaletteAction::ExecuteCommand(item),
        }
    }

    pub fn app(id: String, name: String, description: String, score: u32) -> Self {
        Self {
            category: PaletteCategory::Apps,
            label: name,
            description,
            score,
            action: PaletteAction::OpenPlugin(id),
        }
    }

    pub fn file(path: PathBuf, score: u32) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        let parent = path
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        Self {
            category: PaletteCategory::Files,
            label: name,
            description: parent,
            score,
            action: PaletteAction::NavigateFile(path),
        }
    }
}

/// Main palette state
#[derive(Debug)]
pub struct PaletteState {
    /// Current input text
    pub input: String,
    /// Cursor position in input
    pub cursor: usize,
    /// Current filtered results
    pub results: Vec<PaletteResult>,
    /// Selected result index
    pub selected: usize,
    /// Scroll offset for long result lists
    pub scroll_offset: usize,
    /// Maximum visible results
    pub max_visible: usize,
}

impl Default for PaletteState {
    fn default() -> Self {
        Self::new()
    }
}

impl PaletteState {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            results: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            max_visible: 8,
        }
    }

    /// Reset state for new invocation
    pub fn reset(&mut self) {
        self.input.clear();
        self.cursor = 0;
        self.results.clear();
        self.selected = 0;
        self.scroll_offset = 0;
    }

    /// Insert a character at cursor
    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor, c);
        self.cursor += 1;
    }

    /// Delete character before cursor
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.input.remove(self.cursor);
        }
    }

    /// Delete character at cursor
    pub fn delete(&mut self) {
        if self.cursor < self.input.len() {
            self.input.remove(self.cursor);
        }
    }

    /// Move cursor left
    pub fn cursor_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// Move cursor right
    pub fn cursor_right(&mut self) {
        if self.cursor < self.input.len() {
            self.cursor += 1;
        }
    }

    /// Move cursor to start
    pub fn cursor_home(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to end
    pub fn cursor_end(&mut self) {
        self.cursor = self.input.len();
    }

    /// Select next result
    pub fn select_next(&mut self) {
        if !self.results.is_empty() {
            self.selected = (self.selected + 1) % self.results.len();
            self.adjust_scroll();
        }
    }

    /// Select previous result
    pub fn select_prev(&mut self) {
        if !self.results.is_empty() {
            if self.selected == 0 {
                self.selected = self.results.len() - 1;
            } else {
                self.selected -= 1;
            }
            self.adjust_scroll();
        }
    }

    /// Adjust scroll to keep selection visible
    fn adjust_scroll(&mut self) {
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + self.max_visible {
            self.scroll_offset = self.selected - self.max_visible + 1;
        }
    }

    /// Get currently selected result
    pub fn selected_result(&self) -> Option<&PaletteResult> {
        self.results.get(self.selected)
    }

    /// Update results (sorts by score descending, then by category)
    pub fn set_results(&mut self, mut results: Vec<PaletteResult>) {
        results.sort_by(|a, b| {
            // Higher score first
            b.score
                .cmp(&a.score)
                // Then by category (Calculator < Commands < Apps < Files)
                .then_with(|| a.category.cmp(&b.category))
        });
        self.results = results;
        self.selected = 0;
        self.scroll_offset = 0;
    }

    /// Get visible results (accounting for scroll)
    pub fn visible_results(&self) -> impl Iterator<Item = (usize, &PaletteResult)> {
        self.results
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(self.max_visible)
    }
}
