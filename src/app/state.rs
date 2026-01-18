//! Q-DOS II state types and enums
//!
//! This module contains all the state structs and enums used throughout the application.

use std::path::PathBuf;

// Import Git types needed by this module (Modal uses GitState).
// The types are defined in qdos-plugin-git2.
use qdos_plugin_git2::GitState;

// Import Beads types from qdos-plugin-beads2 for Modal enum
pub use qdos_plugin_beads2::{BeadsState, BeadsView};

/// Navigation menu items
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavItem {
    Directory,
    Tag,
    View,
    Open,
    Copy,
    Move,
    MkDir,
    Find,
    Erase,
    Rename,
    Git,
    Beads,
    Jj,
    Space,
    Attribute,
    Print,
}

impl NavItem {
    pub const ALL: [NavItem; 16] = [
        NavItem::Directory,
        NavItem::Tag,
        NavItem::View,
        NavItem::Open,
        NavItem::Copy,
        NavItem::Move,
        NavItem::MkDir,
        NavItem::Find,
        NavItem::Erase,
        NavItem::Rename,
        NavItem::Git,
        NavItem::Beads,
        NavItem::Jj,
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
            NavItem::MkDir => "MkDir",
            NavItem::Find => "Find",
            NavItem::Erase => "Erase",
            NavItem::Rename => "Rename",
            NavItem::Git => "Git",
            NavItem::Beads => "Beads",
            NavItem::Jj => "Jj",
            NavItem::Space => "Space",
            NavItem::Attribute => "Attribute",
            NavItem::Print => "Print",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            NavItem::Directory => "Change current directory, remove directory, see directory tree",
            NavItem::Tag => {
                "Tag groups of files, or clear all tags -- SPACE BAR tags highlighted file"
            }
            NavItem::View => {
                "View the contents of any file on the screen (in \"ASCII\" or \"HEX\")"
            }
            NavItem::Open => "Open file in its default application (macOS/Linux)",
            NavItem::Copy => "Copy one or several files to another disk or directory",
            NavItem::Move => "Move one or several files from this directory to another directory",
            NavItem::MkDir => "Create a new directory in the current location",
            NavItem::Find => "Search all directories on the disk to find specified file(s)",
            NavItem::Erase => "Erase one or several files from this directory",
            NavItem::Rename => "Rename one or several files in this directory",
            NavItem::Git => "Git integration: status, log, diff, commit, push, pull",
            NavItem::Beads => "Beads issue tracker: list, create, manage issues",
            NavItem::Jj => "Jujutsu VCS: status, log, diff, describe, bookmarks",
            NavItem::Space => "Show the total, used, and free space on any disk",
            NavItem::Attribute => "Change/view file attributes",
            NavItem::Print => "Print one or several files on the printer",
        }
    }

    /// Returns the index of the key character to highlight in the nav bar
    /// Most items highlight the first character (index 0), but MkDir highlights 'k' (index 1)
    pub fn key_index(&self) -> usize {
        match self {
            NavItem::MkDir => 1, // Highlight the 'k' in "MkDir"
            _ => 0,              // All others highlight first character
        }
    }
}

// Re-export SortMode from qdos-plugin-api
pub use qdos_plugin_api::SortMode;

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

// Re-export ColorTheme and ThemeColors from the plugin API crate for unified types
// (ColorThemeState is only used internally by the theme plugin)
pub use qdos_plugin_api::{ColorTheme, ThemeColors};

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
    MkDirInput(String),
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
