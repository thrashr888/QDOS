//! Q-DOS II state types and enums
//!
//! This module contains all the state structs and enums used throughout the application.

use std::path::PathBuf;

// Import Git types needed by this module (Modal uses GitState).
// The types are defined in plugins/git/state.rs.
use crate::plugins::git::GitState;

// Import Beads types from plugins/beads/state.rs for Modal enum
pub use crate::plugins::beads::{
    BeadsActivityEntry, BeadsComment, BeadsIssue, BeadsState, BeadsSubIssue, BeadsView,
};

/// Navigation menu items
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavItem {
    Directory,
    Tag,
    View,
    Open,
    Copy,
    Move,
    Find,
    Erase,
    Rename,
    Git,
    Beads,
    Space,
    Attribute,
    Print,
}

impl NavItem {
    pub const ALL: [NavItem; 14] = [
        NavItem::Directory,
        NavItem::Tag,
        NavItem::View,
        NavItem::Open,
        NavItem::Copy,
        NavItem::Move,
        NavItem::Find,
        NavItem::Erase,
        NavItem::Rename,
        NavItem::Git,
        NavItem::Beads,
        NavItem::Space,
        NavItem::Attribute,
        NavItem::Print,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            NavItem::Directory => "Directory",
            NavItem::Tag => "Tag",
            NavItem::View => "View",
            NavItem::Open => "Open",
            NavItem::Copy => "Copy",
            NavItem::Move => "Move",
            NavItem::Find => "Find",
            NavItem::Erase => "Erase",
            NavItem::Rename => "Rename",
            NavItem::Git => "Git",
            NavItem::Beads => "Beads",
            NavItem::Space => "Space",
            NavItem::Attribute => "Attribute",
            NavItem::Print => "Print",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            NavItem::Directory => {
                "Change current directory, make or remove directory, see directory tree"
            }
            NavItem::Tag => {
                "Tag groups of files, or clear all tags -- SPACE BAR tags highlighted file"
            }
            NavItem::View => {
                "View the contents of any file on the screen (in \"ASCII\" or \"HEX\")"
            }
            NavItem::Open => "Open file in its default application (macOS/Linux)",
            NavItem::Copy => "Copy one or several files to another disk or directory",
            NavItem::Move => "Move one or several files from this directory to another directory",
            NavItem::Find => "Search all directories on the disk to find specified file(s)",
            NavItem::Erase => "Erase one or several files from this directory",
            NavItem::Rename => "Rename one or several files in this directory",
            NavItem::Git => "Git integration: status, log, diff, commit, push, pull",
            NavItem::Beads => "Beads issue tracker: list, create, manage issues",
            NavItem::Space => "Show the total, used, and free space on any disk",
            NavItem::Attribute => "Change/view file attributes",
            NavItem::Print => "Print one or several files on the printer",
        }
    }
}

/// Sort modes for file listing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortMode {
    #[default]
    NameAsc,
    NameDesc,
    ExtAsc,
    ExtDesc,
    SizeAsc,
    SizeDesc,
    DateAsc,
    DateDesc,
    None,
}

impl SortMode {
    pub fn next(&self) -> SortMode {
        match self {
            SortMode::NameAsc => SortMode::NameDesc,
            SortMode::NameDesc => SortMode::ExtAsc,
            SortMode::ExtAsc => SortMode::ExtDesc,
            SortMode::ExtDesc => SortMode::SizeAsc,
            SortMode::SizeAsc => SortMode::SizeDesc,
            SortMode::SizeDesc => SortMode::DateAsc,
            SortMode::DateAsc => SortMode::DateDesc,
            SortMode::DateDesc => SortMode::None,
            SortMode::None => SortMode::NameAsc,
        }
    }

    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            SortMode::NameAsc => "Name ↑",
            SortMode::NameDesc => "Name ↓",
            SortMode::ExtAsc => "Ext ↑",
            SortMode::ExtDesc => "Ext ↓",
            SortMode::SizeAsc => "Size ↑",
            SortMode::SizeDesc => "Size ↓",
            SortMode::DateAsc => "Date ↑",
            SortMode::DateDesc => "Date ↓",
            SortMode::None => "None",
        }
    }
}

/// Find command phases
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindPhase {
    SelectMode,
    InputPattern,
    AskPause,
    Searching,
    ShowResult,
    ShowAllResults,
    NoResults,
}

/// Search mode for Find command
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchMode {
    /// Search by filename with glob patterns (original behavior)
    #[default]
    ByName,
    /// Search by file content using ripgrep
    ByContent,
}

impl SearchMode {
    /// Get display name for this mode
    #[allow(dead_code)]
    pub fn name(&self) -> &str {
        match self {
            SearchMode::ByName => "Filename",
            SearchMode::ByContent => "Content (rg)",
        }
    }

    /// Toggle to the other mode
    pub fn toggle(&self) -> Self {
        match self {
            SearchMode::ByName => SearchMode::ByContent,
            SearchMode::ByContent => SearchMode::ByName,
        }
    }
}

/// Find file search state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindState {
    pub pattern: String,
    pub pause_on_match: bool,
    pub phase: FindPhase,
    pub matches: Vec<(PathBuf, String)>,
    pub current_match: usize,
    pub scroll_offset: usize,
    pub last_pattern: String,
    pub search_complete: bool,
    pub search_mode: SearchMode,
    pub search_tool: crate::config::SearchTool,
    pub search_tool_available: bool,
}

impl FindState {
    pub fn new(last_pattern: String, search_tool: crate::config::SearchTool) -> Self {
        let resolved = search_tool.resolve();
        let available = resolved.is_available();
        Self {
            pattern: String::new(),
            pause_on_match: true,
            phase: FindPhase::SelectMode,
            matches: Vec::new(),
            current_match: 0,
            scroll_offset: 0,
            last_pattern,
            search_complete: false,
            search_mode: SearchMode::ByName,
            search_tool,
            search_tool_available: available,
        }
    }
}

/// Batch rename state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchRenameState {
    pub files: Vec<(PathBuf, String)>,
    pub current_index: usize,
    pub input: String,
    pub renamed_count: usize,
    pub last_error: Option<String>,
}

impl BatchRenameState {
    pub fn new(files: Vec<PathBuf>) -> Self {
        let files: Vec<(PathBuf, String)> = files
            .into_iter()
            .map(|p| {
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                (p, name)
            })
            .collect();

        let input = files
            .first()
            .map(|(_, name)| name.clone())
            .unwrap_or_default();

        Self {
            files,
            current_index: 0,
            input,
            renamed_count: 0,
            last_error: None,
        }
    }

    pub fn current_file(&self) -> Option<&(PathBuf, String)> {
        self.files.get(self.current_index)
    }

    pub fn next(&mut self) {
        if self.current_index + 1 < self.files.len() {
            self.current_index += 1;
            if let Some((_, name)) = self.files.get(self.current_index) {
                self.input = name.clone();
            }
        }
    }

    pub fn is_complete(&self) -> bool {
        self.current_index >= self.files.len()
    }
}

/// File attribute value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttrValue {
    On,
    Off,
    NoChange,
}

impl AttrValue {
    pub fn toggle(&self, for_tagged: bool) -> AttrValue {
        match self {
            AttrValue::On => AttrValue::Off,
            AttrValue::Off => {
                if for_tagged {
                    AttrValue::NoChange
                } else {
                    AttrValue::On
                }
            }
            AttrValue::NoChange => AttrValue::On,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            AttrValue::On => "ON ",
            AttrValue::Off => "OFF",
            AttrValue::NoChange => "N/C",
        }
    }
}

/// Attribute state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeState {
    pub path: PathBuf,
    pub name: String,
    pub for_tagged: bool,
    pub attrs: [AttrValue; 4],
    pub original: [bool; 4],
    pub selected: usize,
    pub display_only: bool,
}

impl AttributeState {
    pub fn new(path: PathBuf, for_tagged: bool) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        let metadata = std::fs::metadata(&path);
        let (hidden, system, readonly, archive) = if let Ok(meta) = metadata {
            #[cfg(unix)]
            let readonly = {
                use std::os::unix::fs::PermissionsExt;
                let mode = meta.permissions().mode();
                (mode & 0o222) == 0
            };
            #[cfg(not(unix))]
            let readonly = meta.permissions().readonly();
            let hidden = name.starts_with('.');
            (hidden, false, readonly, false)
        } else {
            (false, false, false, false)
        };

        let original = [hidden, system, readonly, archive];
        let attrs = if for_tagged {
            [AttrValue::NoChange; 4]
        } else {
            [
                if hidden {
                    AttrValue::On
                } else {
                    AttrValue::Off
                },
                if system {
                    AttrValue::On
                } else {
                    AttrValue::Off
                },
                if readonly {
                    AttrValue::On
                } else {
                    AttrValue::Off
                },
                if archive {
                    AttrValue::On
                } else {
                    AttrValue::Off
                },
            ]
        };

        Self {
            path,
            name,
            for_tagged,
            attrs,
            original,
            selected: 0,
            display_only: false,
        }
    }

    pub fn attr_name(index: usize) -> &'static str {
        match index {
            0 => "HID",
            1 => "SYS",
            2 => "R/O",
            3 => "ARC",
            _ => "???",
        }
    }

    pub fn toggle_current(&mut self) {
        if self.display_only {
            return;
        }
        self.attrs[self.selected] = self.attrs[self.selected].toggle(self.for_tagged);
    }

    pub fn next_attr(&mut self) {
        if self.selected < 3 {
            self.selected += 1;
        }
    }

    pub fn prev_attr(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }
}

/// Type of file operation for progress tracking
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressOperation {
    Copy,
    Move,
    Erase,
}

/// Progress state for file operations
#[derive(Debug, Clone)]
pub struct ProgressState {
    pub operation: ProgressOperation,
    pub files: Vec<PathBuf>,
    pub current_index: usize,
    pub destination: Option<PathBuf>,
    pub completed: usize,
    pub failed: usize,
    pub last_error: Option<String>,
}

impl ProgressState {
    pub fn new(
        operation: ProgressOperation,
        files: Vec<PathBuf>,
        destination: Option<PathBuf>,
    ) -> Self {
        Self {
            operation,
            files,
            current_index: 0,
            destination,
            completed: 0,
            failed: 0,
            last_error: None,
        }
    }

    pub fn is_done(&self) -> bool {
        self.current_index >= self.files.len()
    }

    pub fn current_file(&self) -> Option<&PathBuf> {
        self.files.get(self.current_index)
    }

    pub fn operation_name(&self) -> &'static str {
        match self.operation {
            ProgressOperation::Copy => "Copying",
            ProgressOperation::Move => "Moving",
            ProgressOperation::Erase => "Erasing",
        }
    }
}

/// Color theme options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorTheme {
    #[default]
    Default,
    Monochrome,
    Blue,
    Green,
    Amber,
}

impl ColorTheme {
    pub const ALL: [ColorTheme; 5] = [
        ColorTheme::Default,
        ColorTheme::Monochrome,
        ColorTheme::Blue,
        ColorTheme::Green,
        ColorTheme::Amber,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            ColorTheme::Default => "Default",
            ColorTheme::Monochrome => "Monochrome",
            ColorTheme::Blue => "Blue",
            ColorTheme::Green => "Green",
            ColorTheme::Amber => "Amber",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ColorTheme::Default => "DOS-style blue/white/yellow",
            ColorTheme::Monochrome => "Black and white only",
            ColorTheme::Blue => "Classic blue theme",
            ColorTheme::Green => "Matrix-style green",
            ColorTheme::Amber => "Vintage amber monitor",
        }
    }

    /// Get the RGB color values for this theme, optionally adjusted for terminal luma
    /// If luma > 0.6, the terminal is considered "light" and colors are adjusted
    pub fn colors_for_luma(&self, luma: Option<f32>) -> ThemeColors {
        let base_colors = self.colors();
        match luma {
            Some(l) if l > 0.6 => base_colors.for_light_terminal(),
            _ => base_colors,
        }
    }

    /// Get the RGB color values for this theme (assumes dark terminal)
    pub fn colors(&self) -> ThemeColors {
        match self {
            ColorTheme::Default => ThemeColors {
                background: (0, 0, 0),
                foreground: (255, 255, 255),
                blue: (102, 183, 179),
                green: (103, 204, 77),
                red: (157, 31, 20),
                yellow: (232, 218, 89),
                grey: (128, 128, 128),
                cyan: (0, 170, 170),
                magenta: (170, 0, 170),
            },
            ColorTheme::Monochrome => ThemeColors {
                background: (0, 0, 0),
                foreground: (170, 170, 170),
                blue: (170, 170, 170),
                green: (170, 170, 170),
                red: (85, 85, 85),
                yellow: (255, 255, 255),
                grey: (128, 128, 128),
                cyan: (170, 170, 170),
                magenta: (170, 170, 170),
            },
            ColorTheme::Blue => ThemeColors {
                background: (0, 0, 173),     // Original QDOS blue background
                foreground: (255, 255, 255), // White (same as default)
                blue: (149, 249, 253),       // Bright cyan for borders/menus
                green: (103, 204, 77),       // Same as default
                red: (157, 31, 20),          // Same as default
                yellow: (232, 218, 89),      // Same as default
                grey: (128, 128, 128),       // Same as default
                cyan: (149, 249, 253),       // Bright cyan
                magenta: (170, 0, 170),      // Same as default
            },
            ColorTheme::Green => ThemeColors {
                background: (0, 0, 0),
                foreground: (0, 255, 0),
                blue: (0, 180, 0),
                green: (0, 255, 0),
                red: (0, 100, 0),
                yellow: (180, 255, 0),
                grey: (0, 128, 0),
                cyan: (0, 200, 100),
                magenta: (100, 200, 0),
            },
            ColorTheme::Amber => ThemeColors {
                background: (0, 0, 0),
                foreground: (255, 176, 0),
                blue: (255, 128, 0),
                green: (255, 200, 0),
                red: (128, 64, 0),
                yellow: (255, 255, 0),
                grey: (180, 100, 0),
                cyan: (255, 200, 50),
                magenta: (200, 100, 0),
            },
        }
    }
}

/// RGB color values for a theme
#[derive(Debug, Clone, Copy)]
pub struct ThemeColors {
    pub background: (u8, u8, u8),
    pub foreground: (u8, u8, u8),
    pub blue: (u8, u8, u8),
    pub green: (u8, u8, u8),
    pub red: (u8, u8, u8),
    pub yellow: (u8, u8, u8),
    pub grey: (u8, u8, u8),
    pub cyan: (u8, u8, u8),
    pub magenta: (u8, u8, u8),
}

impl ThemeColors {
    /// Get background color (Reset for terminal default)
    pub fn bg(&self) -> ratatui::style::Color {
        if self.background == (0, 0, 0) {
            ratatui::style::Color::Reset
        } else {
            let (r, g, b) = self.background;
            ratatui::style::Color::Rgb(r, g, b)
        }
    }

    /// Get foreground color
    pub fn fg(&self) -> ratatui::style::Color {
        let (r, g, b) = self.foreground;
        ratatui::style::Color::Rgb(r, g, b)
    }

    /// Get blue color
    pub fn blue(&self) -> ratatui::style::Color {
        let (r, g, b) = self.blue;
        ratatui::style::Color::Rgb(r, g, b)
    }

    /// Get green color
    pub fn green(&self) -> ratatui::style::Color {
        let (r, g, b) = self.green;
        ratatui::style::Color::Rgb(r, g, b)
    }

    /// Get red color
    pub fn red(&self) -> ratatui::style::Color {
        let (r, g, b) = self.red;
        ratatui::style::Color::Rgb(r, g, b)
    }

    /// Get yellow color
    pub fn yellow(&self) -> ratatui::style::Color {
        let (r, g, b) = self.yellow;
        ratatui::style::Color::Rgb(r, g, b)
    }

    /// Get grey color
    pub fn grey(&self) -> ratatui::style::Color {
        let (r, g, b) = self.grey;
        ratatui::style::Color::Rgb(r, g, b)
    }

    /// Get cyan color
    pub fn cyan(&self) -> ratatui::style::Color {
        let (r, g, b) = self.cyan;
        ratatui::style::Color::Rgb(r, g, b)
    }

    /// Get magenta color
    pub fn magenta(&self) -> ratatui::style::Color {
        let (r, g, b) = self.magenta;
        ratatui::style::Color::Rgb(r, g, b)
    }

    /// Adapt colors for a light terminal background
    /// Darkens colors that would be hard to read on light backgrounds
    pub fn for_light_terminal(&self) -> ThemeColors {
        ThemeColors {
            // Use light background, dark foreground
            background: (240, 240, 240),
            foreground: (30, 30, 30),
            // Darken accent colors for visibility on light backgrounds
            blue: Self::darken(self.blue, 0.4),
            green: Self::darken(self.green, 0.5),
            red: self.red, // Red is typically already visible
            yellow: Self::darken(self.yellow, 0.3),
            grey: (100, 100, 100),
            cyan: Self::darken(self.cyan, 0.4),
            magenta: Self::darken(self.magenta, 0.3),
        }
    }

    /// Darken an RGB color by a factor (0.0 = no change, 1.0 = black)
    fn darken(color: (u8, u8, u8), factor: f32) -> (u8, u8, u8) {
        let f = 1.0 - factor.clamp(0.0, 1.0);
        (
            (color.0 as f32 * f) as u8,
            (color.1 as f32 * f) as u8,
            (color.2 as f32 * f) as u8,
        )
    }
}

/// State for color theme selection modal
#[derive(Debug, Clone)]
pub struct ColorThemeState {
    /// Currently selected theme in the list
    pub selected: usize,
    /// The theme that was active when the modal opened
    pub original_theme: ColorTheme,
}

impl ColorThemeState {
    pub fn new(current_theme: ColorTheme) -> Self {
        let selected = ColorTheme::ALL
            .iter()
            .position(|&t| t == current_theme)
            .unwrap_or(0);
        Self {
            selected,
            original_theme: current_theme,
        }
    }

    pub fn selected_theme(&self) -> ColorTheme {
        ColorTheme::ALL[self.selected]
    }
}

/// Clipboard item for copying
#[derive(Debug, Clone)]
pub struct ClipboardItem {
    pub label: String,
    pub value: String,
}

/// State for clipboard selection modal
#[derive(Debug, Clone)]
pub struct ClipboardState {
    /// Available items to copy
    pub items: Vec<ClipboardItem>,
    /// Currently selected item
    pub selected: usize,
}

impl ClipboardState {
    pub fn new(items: Vec<ClipboardItem>) -> Self {
        Self { items, selected: 0 }
    }

    pub fn selected_item(&self) -> Option<&ClipboardItem> {
        self.items.get(self.selected)
    }
}

// Git types (GitMenuItem, GitState, GitView, etc.) are now in plugins/git/state.rs
// Beads types (BeadsMenuItem, BeadsState, BeadsView, etc.) are now in plugins/beads/state.rs
// Both are re-exported above via `pub use crate::plugins::*`

/// Modal dialog types
// Note: Some variants are legacy (plugin migration in progress)
#[allow(clippy::large_enum_variant, dead_code)]
pub enum Modal {
    None,
    Quit,
    Error(String),
    Success(String),
    PathInput(String),
    CopyTo(String),
    MoveTo(String),
    EraseConfirm,
    RenameInput(String),
    Find(FindState),
    BatchRename(BatchRenameState),
    Attribute(AttributeState),
    Progress(ProgressState),
    Git(GitState),
    Beads(BeadsState),
    /// Clipboard menu for copying context items
    Clipboard(ClipboardState),
    /// Plugin-provided modal - plugin ID stored, plugin manages its own state
    Plugin(String),
}
