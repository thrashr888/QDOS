use crate::errors;
use crate::event::EventHandler;
use crate::file_ops::{
    apply_attributes, find_files_recursive, get_directory_contents, get_system_info, FileEntry,
    SystemInfo,
};
use crate::ui;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use std::fs;
use std::path::PathBuf;

/// Navigation menu items
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavItem {
    Directory,
    Tag,
    View,
    Copy,
    Move,
    Find,
    Erase,
    Rename,
    Space,
    Attribute,
    Print,
}

impl NavItem {
    pub const ALL: [NavItem; 11] = [
        NavItem::Directory,
        NavItem::Tag,
        NavItem::View,
        NavItem::Copy,
        NavItem::Move,
        NavItem::Find,
        NavItem::Erase,
        NavItem::Rename,
        NavItem::Space,
        NavItem::Attribute,
        NavItem::Print,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            NavItem::Directory => "Directory",
            NavItem::Tag => "Tag",
            NavItem::View => "View",
            NavItem::Copy => "Copy",
            NavItem::Move => "Move",
            NavItem::Find => "Find",
            NavItem::Erase => "Erase",
            NavItem::Rename => "Rename",
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
            NavItem::Copy => "Copy one or several files to another disk or directory",
            NavItem::Move => "Move one or several files from this directory to another directory",
            NavItem::Find => "Search all directories on the disk to find specified file(s)",
            NavItem::Erase => "Erase one or several files from this directory",
            NavItem::Rename => "Rename one or several files in this directory",
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

/// File viewer display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Normal,
    Hex,
    Image,
    Markdown,
}

/// File viewer filter mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewFilter {
    #[default]
    Off,
    Ascii,
    WordStar,
}

impl ViewFilter {
    pub fn next(&self) -> ViewFilter {
        match self {
            ViewFilter::Off => ViewFilter::Ascii,
            ViewFilter::Ascii => ViewFilter::WordStar,
            ViewFilter::WordStar => ViewFilter::Off,
        }
    }
}

/// File viewer state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileViewerState {
    /// File name being viewed
    pub file_name: String,
    /// Full file path (for loading images)
    pub file_path: PathBuf,
    /// File contents as bytes
    pub content: Vec<u8>,
    /// Current display mode
    pub mode: ViewMode,
    /// Current filter mode
    pub filter: ViewFilter,
    /// Current scroll offset (line number for Normal, byte offset for Hex)
    pub scroll_offset: usize,
    /// Whether cursor is on hex side (true) or ascii side (false) in hex mode
    pub hex_side: bool,
}

impl FileViewerState {
    pub fn new(file_name: String, file_path: PathBuf, content: Vec<u8>) -> Self {
        let mode = Self::detect_mode(&file_name);
        Self {
            file_name,
            file_path,
            content,
            mode,
            filter: ViewFilter::Off,
            scroll_offset: 0,
            hex_side: true,
        }
    }

    /// Calculate max scroll offset based on mode and visible height
    pub fn max_scroll(&self, visible_height: usize) -> usize {
        match self.mode {
            ViewMode::Normal | ViewMode::Markdown => {
                // Count lines in content
                let line_count = self.content.split(|&b| b == b'\n').count();
                line_count.saturating_sub(visible_height)
            }
            ViewMode::Hex => {
                // 16 bytes per line
                let bytes_per_line = 16;
                let total_lines = (self.content.len() + bytes_per_line - 1) / bytes_per_line;
                total_lines.saturating_sub(visible_height)
            }
            ViewMode::Image => {
                // No scrolling for images (could add panning later)
                0
            }
        }
    }

    /// Detect the best view mode based on file extension
    pub fn detect_mode(file_name: &str) -> ViewMode {
        let lower = file_name.to_lowercase();
        if lower.ends_with(".md") || lower.ends_with(".markdown") {
            ViewMode::Markdown
        } else if lower.ends_with(".png")
            || lower.ends_with(".jpg")
            || lower.ends_with(".jpeg")
            || lower.ends_with(".gif")
            || lower.ends_with(".bmp")
            || lower.ends_with(".webp")
            || lower.ends_with(".ico")
        {
            ViewMode::Image
        } else {
            ViewMode::Normal
        }
    }

    /// Scroll by delta, clamping to valid range
    #[allow(dead_code)]
    pub fn scroll(&mut self, delta: isize, visible_height: usize) {
        let max = self.max_scroll(visible_height);
        if delta < 0 {
            self.scroll_offset = self.scroll_offset.saturating_sub((-delta) as usize);
        } else {
            self.scroll_offset = (self.scroll_offset + delta as usize).min(max);
        }
    }
}

/// Shell command state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellCommandState {
    /// Command input buffer
    pub input: String,
    /// Output lines from executed command
    pub output: Vec<String>,
    /// Whether a command is currently running
    pub running: bool,
    /// Exit code of last command (None if not yet executed)
    pub exit_code: Option<i32>,
    /// Scroll offset for output
    pub scroll_offset: usize,
    /// Command history
    pub history: Vec<String>,
    /// Current position in history
    pub history_index: Option<usize>,
}

impl Default for ShellCommandState {
    fn default() -> Self {
        Self {
            input: String::new(),
            output: Vec::new(),
            running: false,
            exit_code: None,
            scroll_offset: 0,
            history: Vec::new(),
            history_index: None,
        }
    }
}

/// Directory tree node for Directory Map
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirTreeNode {
    pub name: String,
    pub path: PathBuf,
    pub expanded: bool,
    pub children: Vec<DirTreeNode>,
    pub depth: usize,
}

impl DirTreeNode {
    pub fn new(name: String, path: PathBuf, depth: usize) -> Self {
        Self {
            name,
            path,
            expanded: false,
            children: Vec::new(),
            depth,
        }
    }

    /// Load immediate children (one level deep)
    pub fn load_children(&mut self) {
        if !self.children.is_empty() {
            return; // Already loaded
        }
        if let Ok(entries) = fs::read_dir(&self.path) {
            let mut dirs: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
                .map(|e| {
                    DirTreeNode::new(
                        e.file_name().to_string_lossy().to_string(),
                        e.path(),
                        self.depth + 1,
                    )
                })
                .collect();
            dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            self.children = dirs;
        }
    }
}

/// Directory Map state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryMapState {
    pub root: DirTreeNode,
    pub selected_index: usize,
    pub flat_list: Vec<(PathBuf, usize, bool, bool)>, // (path, depth, expanded, has_children)
    pub input_mode: Option<String>,                   // For make directory input
    pub input_buffer: String,
    pub confirm_delete: Option<PathBuf>, // For delete confirmation
}

impl DirectoryMapState {
    pub fn new(start_path: &PathBuf) -> Self {
        // Find root (or use home/current)
        let root_path = if let Some(root) = start_path.ancestors().last() {
            root.to_path_buf()
        } else {
            start_path.clone()
        };

        let mut root = DirTreeNode::new(root_path.to_string_lossy().to_string(), root_path, 0);
        root.expanded = true;
        root.load_children();

        // Expand path to current directory
        let mut state = Self {
            root,
            selected_index: 0,
            flat_list: Vec::new(),
            input_mode: None,
            input_buffer: String::new(),
            confirm_delete: None,
        };
        state.expand_to_path(start_path);
        state.rebuild_flat_list();

        // Select the start path
        if let Some(idx) = state
            .flat_list
            .iter()
            .position(|(p, _, _, _)| p == start_path)
        {
            state.selected_index = idx;
        }

        state
    }

    /// Expand all directories from root to the given path
    fn expand_to_path(&mut self, target: &PathBuf) {
        let ancestors: Vec<_> = target.ancestors().collect();
        for ancestor in ancestors.into_iter().rev() {
            self.expand_path_in_tree(&mut self.root.clone(), &ancestor.to_path_buf());
        }
    }

    fn expand_path_in_tree(&mut self, _node: &DirTreeNode, target: &PathBuf) {
        // Recursive expand - simplified version
        fn expand_recursive(node: &mut DirTreeNode, target: &PathBuf) {
            if target.starts_with(&node.path) {
                node.expanded = true;
                node.load_children();
                for child in &mut node.children {
                    expand_recursive(child, target);
                }
            }
        }
        expand_recursive(&mut self.root, target);
    }

    /// Rebuild flat list from tree for display
    pub fn rebuild_flat_list(&mut self) {
        self.flat_list.clear();
        fn flatten(node: &DirTreeNode, list: &mut Vec<(PathBuf, usize, bool, bool)>) {
            let has_children = !node.children.is_empty() || {
                // Check if directory has subdirs
                fs::read_dir(&node.path)
                    .map(|entries| {
                        entries.filter_map(|e| e.ok()).any(|e| {
                            e.path().is_dir() && !e.file_name().to_string_lossy().starts_with('.')
                        })
                    })
                    .unwrap_or(false)
            };
            list.push((node.path.clone(), node.depth, node.expanded, has_children));
            if node.expanded {
                for child in &node.children {
                    flatten(child, list);
                }
            }
        }
        flatten(&self.root, &mut self.flat_list);
    }

    /// Toggle expand/collapse at index
    pub fn toggle_expand(&mut self, index: usize) {
        if index >= self.flat_list.len() {
            return;
        }
        let (path, _, expanded, _) = &self.flat_list[index];
        let path = path.clone();
        let expanded = *expanded;

        fn toggle_in_tree(node: &mut DirTreeNode, target: &PathBuf, expand: bool) -> bool {
            if node.path == *target {
                node.expanded = expand;
                if expand {
                    node.load_children();
                }
                return true;
            }
            for child in &mut node.children {
                if toggle_in_tree(child, target, expand) {
                    return true;
                }
            }
            false
        }

        toggle_in_tree(&mut self.root, &path, !expanded);
        self.rebuild_flat_list();
    }

    /// Get currently selected path
    pub fn selected_path(&self) -> Option<PathBuf> {
        self.flat_list
            .get(self.selected_index)
            .map(|(p, _, _, _)| p.clone())
    }
}

/// Find file search state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindState {
    /// Search pattern (e.g., "*.txt", "foo*")
    pub pattern: String,
    /// Whether to pause on each match
    pub pause_on_match: bool,
    /// Current phase: InputPattern, AskPause, Searching, ShowResult, NoResults
    pub phase: FindPhase,
    /// Found matches: (path, display_string)
    pub matches: Vec<(PathBuf, String)>,
    /// Current match index (when pausing)
    pub current_match: usize,
    /// Scroll offset for results list
    pub scroll_offset: usize,
    /// Last search pattern (for Ctrl+R recall)
    pub last_pattern: String,
    /// Is search complete?
    pub search_complete: bool,
}

/// Find command phases
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindPhase {
    InputPattern,
    AskPause,
    Searching,
    ShowResult,
    ShowAllResults,
    NoResults,
}

impl FindState {
    pub fn new(last_pattern: String) -> Self {
        Self {
            pattern: String::new(),
            pause_on_match: true,
            phase: FindPhase::InputPattern,
            matches: Vec::new(),
            current_match: 0,
            scroll_offset: 0,
            last_pattern,
            search_complete: false,
        }
    }
}

/// Batch rename state for renaming multiple files
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchRenameState {
    /// List of files to rename (original path, current new name)
    pub files: Vec<(PathBuf, String)>,
    /// Current file index being renamed
    pub current_index: usize,
    /// Input buffer for new name
    pub input: String,
    /// Number of files successfully renamed
    pub renamed_count: usize,
    /// Last error message (if any)
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

    /// Get current file being renamed
    pub fn current_file(&self) -> Option<&(PathBuf, String)> {
        self.files.get(self.current_index)
    }

    /// Move to next file
    pub fn next(&mut self) {
        if self.current_index + 1 < self.files.len() {
            self.current_index += 1;
            if let Some((_, name)) = self.files.get(self.current_index) {
                self.input = name.clone();
            }
        }
    }

    /// Check if all files have been processed
    pub fn is_complete(&self) -> bool {
        self.current_index >= self.files.len()
    }
}

/// File attribute value for modification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttrValue {
    On,
    Off,
    NoChange, // Only available for tagged files
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

/// Attribute state for viewing/editing file attributes
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeState {
    /// File path being modified
    pub path: PathBuf,
    /// Original filename
    pub name: String,
    /// Whether this is for tagged files (enables N/C option)
    pub for_tagged: bool,
    /// Current attribute values: [HID, SYS, R/O, ARC]
    /// Note: On Unix, only R/O (read-only) is actually modifiable
    pub attrs: [AttrValue; 4],
    /// Original attribute values (to show what changed)
    pub original: [bool; 4],
    /// Currently selected attribute index (0-3)
    pub selected: usize,
    /// Is this in display-only mode?
    pub display_only: bool,
}

impl AttributeState {
    pub fn new(path: PathBuf, for_tagged: bool) -> Self {
        use std::os::unix::fs::PermissionsExt;

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        // Read current attributes
        let metadata = std::fs::metadata(&path);
        let (hidden, system, readonly, archive) = if let Ok(meta) = metadata {
            let mode = meta.permissions().mode();
            let readonly = (mode & 0o222) == 0; // No write permission = read-only
            let hidden = name.starts_with('.'); // Unix hidden convention
                                                // System and Archive don't have Unix equivalents
            (hidden, false, readonly, false)
        } else {
            (false, false, false, false)
        };

        let original = [hidden, system, readonly, archive];
        let attrs = if for_tagged {
            // For tagged files, start with N/C (no change)
            [AttrValue::NoChange; 4]
        } else {
            // For single file, show current values
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

    /// Get attribute name by index
    pub fn attr_name(index: usize) -> &'static str {
        match index {
            0 => "HID",
            1 => "SYS",
            2 => "R/O",
            3 => "ARC",
            _ => "???",
        }
    }

    /// Toggle current attribute
    pub fn toggle_current(&mut self) {
        if self.display_only {
            return;
        }
        self.attrs[self.selected] = self.attrs[self.selected].toggle(self.for_tagged);
    }

    /// Move to next attribute
    pub fn next_attr(&mut self) {
        if self.selected < 3 {
            self.selected += 1;
        }
    }

    /// Move to previous attribute
    pub fn prev_attr(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }
}

/// Search specification state for filtering files
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchSpecState {
    /// File name pattern (e.g., "*.EXE", "*.COM")
    pub pattern: String,
    /// Phase: 0 = editing pattern, 1 = editing attributes
    pub phase: u8,
    /// Attribute filters: [NORM, DIR, HID, SYS, R/O, ARC]
    /// true = include files with this attribute
    pub attrs: [bool; 6],
    /// Currently selected attribute (0-5) in phase 1
    pub selected_attr: usize,
}

impl SearchSpecState {
    pub fn new(current_spec: &str) -> Self {
        Self {
            pattern: current_spec.to_string(),
            phase: 0,
            // Default: NORM=true (normal files), DIR=true (directories), others=false
            attrs: [true, true, false, false, false, false],
            selected_attr: 0,
        }
    }

    /// Get attribute name by index
    pub fn attr_name(index: usize) -> &'static str {
        match index {
            0 => "NORM",
            1 => "DIR ",
            2 => "HID ",
            3 => "SYS ",
            4 => "R/O ",
            5 => "ARC ",
            _ => "????",
        }
    }

    /// Toggle the currently selected attribute
    pub fn toggle_current(&mut self) {
        self.attrs[self.selected_attr] = !self.attrs[self.selected_attr];
    }
}

/// Help topic entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpTopic {
    pub key: char,
    pub title: String,
    pub content: String,
}

/// Help system state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpState {
    /// All help topics
    pub topics: Vec<HelpTopic>,
    /// Current topic index (0 = index page)
    pub current_topic: usize,
    /// Scroll offset within current topic
    pub scroll_offset: usize,
}

impl HelpState {
    pub fn new() -> Self {
        let topics = Self::load_topics();
        Self {
            topics,
            current_topic: 0,
            scroll_offset: 0,
        }
    }

    fn load_topics() -> Vec<HelpTopic> {
        // Embedded help content based on spec/help.txt
        vec![
            HelpTopic {
                key: 'I',
                title: "Introduction to Q-DOS II".to_string(),
                content: r#"Q-DOS II lets you easily manage DOS directories and files. You can
create directories with a few keystrokes and see them displayed on a
"directory map." You can find "lost" files located anywhere on the
disk and you can also edit files in any directory.

Q-DOS II lets you mark files and move, copy, rename, print, or erase
them without ever typing file names. You can load and execute
programs or any DOS commands.

HOW TO SELECT A COMMAND

As you enter Q-DOS II, you will see the Main Screen with the main
commands listed on the top line. One of them will be "highlighted."

You may select a command by highlighting it with the arrow keys
and pressing RETURN, or by pressing the first letter of the command.

HOW TO TAG FILES

COPY, ERASE, RENAME, PRINT, ATTRIBUTE, and MOVE can operate on
several files at once. You identify multiple files by tagging them.
Press SPACE BAR to tag/untag the highlighted file.

THE ESC KEY

The Escape (ESC) key returns you to the Main Screen. When pressed
in the middle of a command, it will cancel the command."#
                    .to_string(),
            },
            HelpTopic {
                key: 'A',
                title: "Attribute Command".to_string(),
                content: r#"The ATTRIBUTE command allows you to display and/or change file
attributes. File attributes include: HID (Hidden), SYS (System),
R/O (Read-Only), and ARC (Archive).

On Unix/macOS, only the R/O (Read-Only) attribute can be modified.
This controls whether the file has write permissions.

TO USE:
1. Highlight a file or tag multiple files
2. Select ATTRIBUTE from the menu
3. Use arrow keys to select an attribute
4. Press SPACE to toggle ON/OFF/N/C
5. Press ENTER to apply changes

Note: You cannot change DIR, NORM, or VOL attributes."#
                    .to_string(),
            },
            HelpTopic {
                key: 'C',
                title: "Copy Command".to_string(),
                content: r#"The COPY command copies files from the current directory to another.

TO USE:
1. Tag files to copy (or use highlighted file)
2. Select COPY from the menu
3. Enter destination path (Tab for completion)
4. Press ENTER to copy

The original files remain in their current location.
Use Tab for path auto-completion."#
                    .to_string(),
            },
            HelpTopic {
                key: 'D',
                title: "Directory Command".to_string(),
                content: r#"The DIRECTORY command lets you manage directories.

DIRECTORY MAP (D key):
Opens a tree view of all directories. You can:
- Navigate with arrow keys
- Expand/collapse with Enter or Right/Left arrows
- Create new directories with M
- Delete empty directories with D (requires confirmation)

CHANGE DIRECTORY (F5):
Enter a path to change to a different directory.

PREVIOUS DIRECTORY (F4):
Return to the previously visited directory."#
                    .to_string(),
            },
            HelpTopic {
                key: 'E',
                title: "Erase Command".to_string(),
                content: r#"The ERASE command deletes files from the current directory.

TO USE:
1. Tag files to erase (or use highlighted file)
2. Select ERASE from the menu
3. Confirm with Y or cancel with N

WARNING: Erased files cannot be recovered!

To delete directories, use the Directory Map (D key)."#
                    .to_string(),
            },
            HelpTopic {
                key: 'F',
                title: "Find Command".to_string(),
                content: r#"The FIND command searches for files matching a pattern.

TO USE:
1. Select FIND from the menu
2. Enter a search pattern (e.g., *.txt, config.*)
3. Choose whether to pause on each match

WILDCARDS:
* - matches any characters
? - matches a single character

When a match is found:
- J: Jump to the file's directory
- V: View the file contents
- C: Continue searching"#
                    .to_string(),
            },
            HelpTopic {
                key: 'M',
                title: "Move Command".to_string(),
                content: r#"The MOVE command moves files from the current directory to another.

TO USE:
1. Tag files to move (or use highlighted file)
2. Select MOVE from the menu
3. Enter destination path (Tab for completion)
4. Press ENTER to move

Unlike COPY, the original files are removed after moving."#
                    .to_string(),
            },
            HelpTopic {
                key: 'R',
                title: "Rename Command".to_string(),
                content: r#"The RENAME command changes the name of files.

SINGLE FILE:
1. Highlight the file
2. Select RENAME from the menu
3. Edit the filename
4. Press ENTER to rename

BATCH RENAME (tagged files):
1. Tag multiple files
2. Select RENAME from the menu
3. Edit each filename, press ENTER to rename
4. Press TAB to skip a file
5. Press ESC when done"#
                    .to_string(),
            },
            HelpTopic {
                key: 'S',
                title: "Space Command".to_string(),
                content: r#"The SPACE command displays disk space information.

Shows:
- Total disk space
- Used space (and percentage)
- Available space

Press any key to close the display."#
                    .to_string(),
            },
            HelpTopic {
                key: 'T',
                title: "Tag Command".to_string(),
                content: r#"The TAG command marks files for batch operations.

TO TAG FILES:
- Press SPACE BAR on highlighted file
- Or select TAG from menu for options

Tagged files show a marker (▶) next to their name.

TAG OPTIONS:
- Tag All: Tag all files in directory
- Untag All: Remove all tags
- Invert: Toggle all tags

Tags are used by: COPY, MOVE, ERASE, RENAME, ATTRIBUTE"#
                    .to_string(),
            },
            HelpTopic {
                key: 'V',
                title: "View Command".to_string(),
                content: r#"The VIEW command displays file contents.

MODES:
- ASCII (A): Text view with line numbers
- HEX (H): Hexadecimal byte view
- Raw (R): Plain text without formatting

NAVIGATION:
- Arrow keys, PgUp/PgDn: Scroll
- Home/End: Jump to start/end
- G: Go to specific line
- /: Search for text

FILTERS:
- 1: Show all lines
- 2: Non-empty lines only
- 3: Code lines (no comments)"#
                    .to_string(),
            },
            HelpTopic {
                key: '1',
                title: "Function Keys".to_string(),
                content: r#"FUNCTION KEY REFERENCE

F1  - Help: Display this help system
F2  - Status: Show system status and memory info
F3  - Change Drive: Switch to a different drive/mount
F4  - Previous Directory: Go back to last directory
F5  - Change Directory: Enter path to navigate
F6  - DOS Command: Run a shell command
F7  - Search Spec: Set file filter pattern
F8  - Sort: Cycle through sort modes
F9  - Edit: Open file in editor (not implemented)
F10 - Quit: Exit Q-DOS II"#
                    .to_string(),
            },
            HelpTopic {
                key: '2',
                title: "Navigation Keys".to_string(),
                content: r#"NAVIGATION REFERENCE

FILE LIST:
↑/↓     - Move selection up/down
PgUp/Dn - Move one page
Home    - Jump to first file
End     - Jump to last file
Enter   - Enter directory / Execute action

MENU:
←/→     - Move between menu items
Enter   - Select menu item
Letter  - Jump to command by first letter

GENERAL:
ESC     - Cancel / Close modal
SPACE   - Tag/untag file"#
                    .to_string(),
            },
        ]
    }
}

pub enum Modal {
    None,
    Help(HelpState),
    Status(SystemInfo),
    Quit,
    SearchSpec(SearchSpecState),
    Space,
    Error(String),
    Success(String),
    PathInput(String),
    CopyTo(String),
    MoveTo(String),
    EraseConfirm,
    RenameInput(String),
    ShellCommand(ShellCommandState),
    FileViewer(FileViewerState),
    DirectoryMap(DirectoryMapState),
    Find(FindState),
    BatchRename(BatchRenameState),
    Attribute(AttributeState),
}

/// Application state
pub struct App {
    /// Current directory path
    pub current_path: PathBuf,
    /// Files in current directory
    pub files: Vec<FileEntry>,
    /// Currently selected file index
    pub selected_index: usize,
    /// Tagged files (by full path)
    pub tagged_files: Vec<PathBuf>,
    /// Current sort mode
    pub sort_mode: SortMode,
    /// Selected navigation menu item
    pub nav_index: usize,
    /// Current active modal
    pub modal: Modal,
    /// Scroll offset for file list
    pub scroll_offset: usize,
    /// Should the app quit
    pub should_quit: bool,
    /// Search/filter specification
    pub search_spec: String,
    /// Navigation history
    pub history: Vec<PathBuf>,
    /// Last find pattern (for Ctrl+R recall)
    pub last_find_pattern: String,
}

impl App {
    pub fn new(start_path: &str) -> Result<Self> {
        let current_path = PathBuf::from(start_path).canonicalize()?;
        let files = get_directory_contents(&current_path, SortMode::NameAsc)?;

        Ok(Self {
            current_path,
            files,
            selected_index: 0,
            tagged_files: Vec::new(),
            sort_mode: SortMode::NameAsc,
            nav_index: 0,
            modal: Modal::None,
            scroll_offset: 0,
            should_quit: false,
            search_spec: "*.*".to_string(),
            history: Vec::new(),
            last_find_pattern: String::new(),
        })
    }

    /// Main application loop
    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        let event_handler = EventHandler::new(100);

        loop {
            // Draw UI
            terminal.draw(|frame| ui::draw(frame, self))?;

            // Handle events
            if let Some(event) = event_handler.next().await? {
                match event {
                    crate::event::Event::Key(key) => self.handle_key(key)?,
                    crate::event::Event::Tick => {}
                    crate::event::Event::Resize(_, _) => {}
                }
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Handle keyboard input
    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        // Handle modal-specific input first
        if !matches!(self.modal, Modal::None) {
            return self.handle_modal_key(key);
        }

        match key.code {
            // Quit
            KeyCode::F(10) | KeyCode::Char('q') => {
                self.modal = Modal::Quit;
            }
            // Help
            KeyCode::F(1) => {
                self.modal = Modal::Help(HelpState::new());
            }
            // Status
            KeyCode::F(2) => {
                let info = get_system_info()?;
                self.modal = Modal::Status(info);
            }
            // Change drive (not applicable on Unix, show error)
            KeyCode::F(3) => {
                self.modal =
                    Modal::Error("Drive selection not available on this platform".to_string());
            }
            // Previous directory
            KeyCode::F(4) => {
                self.go_to_parent()?;
            }
            // Change directory
            KeyCode::F(5) => {
                let path = self.current_path.to_string_lossy().to_string();
                self.modal = Modal::PathInput(path);
            }
            // Shell Command
            KeyCode::F(6) => {
                self.modal = Modal::ShellCommand(ShellCommandState::default());
            }
            // Search spec
            KeyCode::F(7) => {
                let state = SearchSpecState::new(&self.search_spec);
                self.modal = Modal::SearchSpec(state);
            }
            // Sort
            KeyCode::F(8) => {
                self.cycle_sort_mode()?;
            }
            // Edit (not implemented)
            KeyCode::F(9) => {
                self.modal = Modal::Error("Edit not implemented".to_string());
            }
            // Navigation
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_selection(-1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_selection(1);
            }
            KeyCode::PageUp => {
                self.move_selection(-20);
            }
            KeyCode::PageDown => {
                self.move_selection(20);
            }
            KeyCode::Home => {
                self.selected_index = 0;
                self.scroll_offset = 0;
            }
            KeyCode::End => {
                if !self.files.is_empty() {
                    self.selected_index = self.files.len() - 1;
                }
            }
            // Menu navigation
            KeyCode::Left | KeyCode::Char('h') => {
                if self.nav_index > 0 {
                    self.nav_index -= 1;
                } else {
                    self.nav_index = NavItem::ALL.len() - 1;
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.nav_index = (self.nav_index + 1) % NavItem::ALL.len();
            }
            // Tag file
            KeyCode::Char(' ') => {
                self.toggle_tag();
                self.move_selection(1);
            }
            // Enter directory or execute menu action
            KeyCode::Enter => {
                self.execute_action()?;
            }
            // Escape
            KeyCode::Esc => {
                // Do nothing in main view
            }
            // Ctrl+C opens quit confirmation (same as F10)
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.modal = Modal::Quit;
            }
            _ => {}
        }

        Ok(())
    }

    /// Handle keyboard input in modal dialogs
    fn handle_modal_key(&mut self, key: KeyEvent) -> Result<()> {
        match &mut self.modal {
            Modal::Quit => {
                match key.code {
                    // F10 again or Ctrl+C again quits immediately
                    KeyCode::F(10) => {
                        self.should_quit = true;
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.should_quit = true;
                    }
                    // RETURN for options (for now, just quit)
                    KeyCode::Enter => {
                        self.should_quit = true;
                    }
                    // ESC returns to Q-DOS II
                    KeyCode::Esc => {
                        self.modal = Modal::None;
                    }
                    // Y/N still work for convenience
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        self.should_quit = true;
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') => {
                        self.modal = Modal::None;
                    }
                    _ => {}
                }
            }
            Modal::PathInput(ref mut path) => match key.code {
                KeyCode::Enter => {
                    let new_path = PathBuf::from(path.clone());
                    self.modal = Modal::None;
                    if let Err(e) = self.navigate_to(&new_path) {
                        self.modal = Modal::Error(format!("Cannot navigate: {}", e));
                    }
                }
                KeyCode::Esc => {
                    self.modal = Modal::None;
                }
                KeyCode::Backspace => {
                    path.pop();
                }
                KeyCode::Tab => {
                    if let Some(completed) = Self::tab_complete(path) {
                        *path = completed;
                    }
                }
                KeyCode::Char(c) => {
                    path.push(c);
                }
                _ => {}
            },
            Modal::CopyTo(ref mut dest) => match key.code {
                KeyCode::Enter => {
                    let dest_path = PathBuf::from(dest.clone());
                    self.modal = Modal::None;
                    match self.copy_tagged_files(&dest_path) {
                        Ok(count) => {
                            self.modal = Modal::Success(format!("Copied {} file(s)", count));
                        }
                        Err(e) => {
                            self.modal = Modal::Error(format!("Copy failed: {}", e));
                        }
                    }
                }
                KeyCode::Esc => {
                    self.modal = Modal::None;
                }
                KeyCode::Backspace => {
                    dest.pop();
                }
                KeyCode::Tab => {
                    if let Some(completed) = Self::tab_complete(dest) {
                        *dest = completed;
                    }
                }
                KeyCode::Char(c) => {
                    dest.push(c);
                }
                _ => {}
            },
            Modal::MoveTo(ref mut dest) => {
                match key.code {
                    KeyCode::Enter => {
                        let dest_path = PathBuf::from(dest.clone());
                        self.modal = Modal::None;
                        match self.move_tagged_files(&dest_path) {
                            Ok(count) => {
                                self.modal = Modal::Success(format!("Moved {} file(s)", count));
                                // Refresh the file list
                                let _ = self.refresh_files();
                            }
                            Err(e) => {
                                self.modal = Modal::Error(format!("Move failed: {}", e));
                            }
                        }
                    }
                    KeyCode::Esc => {
                        self.modal = Modal::None;
                    }
                    KeyCode::Backspace => {
                        dest.pop();
                    }
                    KeyCode::Tab => {
                        if let Some(completed) = Self::tab_complete(dest) {
                            *dest = completed;
                        }
                    }
                    KeyCode::Char(c) => {
                        dest.push(c);
                    }
                    _ => {}
                }
            }
            Modal::EraseConfirm => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    self.modal = Modal::None;
                    match self.erase_tagged_files() {
                        Ok(count) => {
                            self.modal = Modal::Success(format!("Erased {} file(s)", count));
                            let _ = self.refresh_files();
                        }
                        Err(e) => {
                            self.modal = Modal::Error(format!("Erase failed: {}", e));
                        }
                    }
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.modal = Modal::None;
                }
                _ => {}
            },
            Modal::RenameInput(ref mut new_name) => match key.code {
                KeyCode::Enter => {
                    let name = new_name.clone();
                    self.modal = Modal::None;
                    match self.rename_selected_file(&name) {
                        Ok(()) => {
                            self.modal = Modal::Success("File renamed".to_string());
                            let _ = self.refresh_files();
                        }
                        Err(e) => {
                            self.modal = Modal::Error(format!("Rename failed: {}", e));
                        }
                    }
                }
                KeyCode::Esc => {
                    self.modal = Modal::None;
                }
                KeyCode::Backspace => {
                    new_name.pop();
                }
                KeyCode::Tab => {
                    if let Some(completed) = Self::tab_complete(new_name) {
                        *new_name = completed;
                    }
                }
                KeyCode::Char(c) => {
                    new_name.push(c);
                }
                _ => {}
            },
            Modal::FileViewer(ref mut state) => {
                // Calculate max scroll based on mode and content
                let max_scroll = state.max_scroll(20); // Assume ~20 lines visible, will be recalculated in UI

                match key.code {
                    KeyCode::Esc => {
                        self.modal = Modal::None;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        state.scroll_offset = state.scroll_offset.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        state.scroll_offset = (state.scroll_offset + 1).min(max_scroll);
                    }
                    KeyCode::PageUp => {
                        state.scroll_offset = state.scroll_offset.saturating_sub(20);
                    }
                    KeyCode::PageDown => {
                        state.scroll_offset = (state.scroll_offset + 20).min(max_scroll);
                    }
                    KeyCode::Home => {
                        state.scroll_offset = 0;
                    }
                    KeyCode::End => {
                        state.scroll_offset = max_scroll;
                    }
                    KeyCode::Char('h') | KeyCode::Char('H') => {
                        state.mode = ViewMode::Hex;
                        // Clamp scroll for new mode
                        state.scroll_offset = state.scroll_offset.min(state.max_scroll(20));
                    }
                    KeyCode::Char('n')
                    | KeyCode::Char('N')
                    | KeyCode::Char('a')
                    | KeyCode::Char('A') => {
                        state.mode = ViewMode::Normal;
                        // Clamp scroll for new mode
                        state.scroll_offset = state.scroll_offset.min(state.max_scroll(20));
                    }
                    KeyCode::Char('i') | KeyCode::Char('I') => {
                        state.mode = ViewMode::Image;
                        state.scroll_offset = 0;
                    }
                    KeyCode::Char('m') | KeyCode::Char('M') => {
                        state.mode = ViewMode::Markdown;
                        // Clamp scroll for new mode
                        state.scroll_offset = state.scroll_offset.min(state.max_scroll(20));
                    }
                    KeyCode::Char('f') | KeyCode::Char('F') => {
                        state.filter = state.filter.next();
                    }
                    KeyCode::F(4) => {
                        // Toggle hex/ascii side in hex mode
                        if state.mode == ViewMode::Hex {
                            state.hex_side = !state.hex_side;
                        }
                    }
                    _ => {}
                }
            }
            Modal::ShellCommand(ref mut state) => {
                match key.code {
                    KeyCode::Esc => {
                        self.modal = Modal::None;
                    }
                    KeyCode::Enter => {
                        if !state.input.is_empty() {
                            let cmd = state.input.clone();
                            let cwd = self.current_path.clone();
                            // Add to history
                            state.history.push(cmd.clone());
                            state.history_index = None;
                            // Execute command (use standalone function to avoid borrow issues)
                            let output = execute_shell_command_impl(&cmd, &cwd);
                            state.output = output.0;
                            state.exit_code = Some(output.1);
                            state.input.clear();
                            state.scroll_offset = 0;
                        }
                    }
                    KeyCode::Backspace => {
                        state.input.pop();
                    }
                    KeyCode::Up => {
                        // Navigate history up
                        if !state.history.is_empty() {
                            let new_idx = match state.history_index {
                                Some(idx) if idx > 0 => idx - 1,
                                Some(idx) => idx,
                                None => state.history.len() - 1,
                            };
                            state.history_index = Some(new_idx);
                            state.input = state.history[new_idx].clone();
                        }
                    }
                    KeyCode::Down => {
                        // Navigate history down
                        if let Some(idx) = state.history_index {
                            if idx + 1 < state.history.len() {
                                let new_idx = idx + 1;
                                state.history_index = Some(new_idx);
                                state.input = state.history[new_idx].clone();
                            } else {
                                state.history_index = None;
                                state.input.clear();
                            }
                        }
                    }
                    KeyCode::PageUp => {
                        state.scroll_offset = state.scroll_offset.saturating_sub(10);
                    }
                    KeyCode::PageDown => {
                        let max_scroll = state.output.len().saturating_sub(10);
                        state.scroll_offset = (state.scroll_offset + 10).min(max_scroll);
                    }
                    KeyCode::Tab => {
                        // Tab completion for commands/paths
                        if let Some(completed) = Self::tab_complete(&state.input) {
                            state.input = completed;
                        }
                    }
                    KeyCode::Char(c) => {
                        state.input.push(c);
                    }
                    _ => {}
                }
            }
            Modal::DirectoryMap(ref mut state) => {
                // Handle delete confirmation mode
                if let Some(ref path_to_delete) = state.confirm_delete.clone() {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            // Confirmed - delete the directory
                            match fs::remove_dir(&path_to_delete) {
                                Ok(()) => {
                                    // Refresh tree
                                    let parent_idx = if state.selected_index > 0 {
                                        state.selected_index - 1
                                    } else {
                                        0
                                    };
                                    state.confirm_delete = None;
                                    state.rebuild_flat_list();
                                    state.selected_index =
                                        parent_idx.min(state.flat_list.len().saturating_sub(1));
                                }
                                Err(e) => {
                                    state.confirm_delete = None;
                                    self.modal =
                                        Modal::Error(format!("Cannot remove directory: {}", e));
                                    return Ok(());
                                }
                            }
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            // Cancelled
                            state.confirm_delete = None;
                        }
                        _ => {}
                    }
                }
                // Handle input mode (for make directory)
                else if state.input_mode.is_some() {
                    match key.code {
                        KeyCode::Enter => {
                            let dir_name = state.input_buffer.clone();
                            if !dir_name.is_empty() {
                                if let Some(parent_path) = state.selected_path() {
                                    let new_dir = parent_path.join(&dir_name);
                                    match fs::create_dir(&new_dir) {
                                        Ok(()) => {
                                            // Refresh the tree
                                            state.input_mode = None;
                                            state.input_buffer.clear();
                                            // Reload children of selected node
                                            state.toggle_expand(state.selected_index);
                                            state.toggle_expand(state.selected_index);
                                        }
                                        Err(e) => {
                                            state.input_mode = None;
                                            state.input_buffer.clear();
                                            self.modal = Modal::Error(format!(
                                                "Failed to create directory: {}",
                                                e
                                            ));
                                            return Ok(());
                                        }
                                    }
                                }
                            }
                            state.input_mode = None;
                            state.input_buffer.clear();
                        }
                        KeyCode::Esc => {
                            state.input_mode = None;
                            state.input_buffer.clear();
                        }
                        KeyCode::Backspace => {
                            state.input_buffer.pop();
                        }
                        KeyCode::Char(c) => {
                            state.input_buffer.push(c);
                        }
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Esc => {
                            self.modal = Modal::None;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if state.selected_index > 0 {
                                state.selected_index -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if state.selected_index + 1 < state.flat_list.len() {
                                state.selected_index += 1;
                            }
                        }
                        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                            // Navigate to directory or expand
                            if let Some((_, _, expanded, has_children)) =
                                state.flat_list.get(state.selected_index)
                            {
                                if *has_children && !*expanded {
                                    state.toggle_expand(state.selected_index);
                                } else if let Some(path) = state.selected_path() {
                                    // Navigate to the selected directory
                                    let path = path.clone();
                                    self.modal = Modal::None;
                                    let _ = self.navigate_to(&path);
                                }
                            }
                        }
                        KeyCode::Left | KeyCode::Char('h') | KeyCode::Backspace => {
                            // Collapse if expanded, otherwise go to parent
                            if let Some((_, _, expanded, _)) =
                                state.flat_list.get(state.selected_index)
                            {
                                if *expanded {
                                    state.toggle_expand(state.selected_index);
                                } else if state.selected_index > 0 {
                                    // Find parent (look for item with depth - 1)
                                    let current_depth = state
                                        .flat_list
                                        .get(state.selected_index)
                                        .map(|(_, d, _, _)| *d)
                                        .unwrap_or(0);
                                    if current_depth > 0 {
                                        for i in (0..state.selected_index).rev() {
                                            if let Some((_, d, _, _)) = state.flat_list.get(i) {
                                                if *d < current_depth {
                                                    state.selected_index = i;
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Char('m') | KeyCode::Char('M') => {
                            // Make directory mode
                            state.input_mode = Some("New directory name".to_string());
                            state.input_buffer.clear();
                        }
                        KeyCode::Char('d') | KeyCode::Char('D') => {
                            // Request delete confirmation
                            if let Some(path) = state.selected_path() {
                                // Don't allow deleting root
                                if path != state.root.path {
                                    state.confirm_delete = Some(path);
                                }
                            }
                        }
                        KeyCode::Home => {
                            state.selected_index = 0;
                        }
                        KeyCode::End => {
                            state.selected_index = state.flat_list.len().saturating_sub(1);
                        }
                        _ => {}
                    }
                }
            }
            Modal::Find(ref mut state) => {
                match state.phase {
                    FindPhase::InputPattern => {
                        match key.code {
                            KeyCode::Enter => {
                                if state.pattern.is_empty() {
                                    // Use *.* if no pattern entered
                                    state.pattern = "*.*".to_string();
                                }
                                // Move to ask pause phase
                                state.phase = FindPhase::AskPause;
                            }
                            KeyCode::Esc => {
                                self.modal = Modal::None;
                            }
                            KeyCode::Backspace => {
                                state.pattern.pop();
                            }
                            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                // Recall last pattern
                                state.pattern = state.last_pattern.clone();
                            }
                            KeyCode::Char(c) => {
                                state.pattern.push(c);
                            }
                            _ => {}
                        }
                    }
                    FindPhase::AskPause => {
                        match key.code {
                            KeyCode::Char('y') | KeyCode::Char('Y') => {
                                state.pause_on_match = true;
                                // Start searching
                                state.phase = FindPhase::Searching;
                                let root = self.current_path.clone();
                                state.matches = find_files_recursive(&root, &state.pattern);
                                self.last_find_pattern = state.pattern.clone();
                                state.search_complete = true;

                                if state.matches.is_empty() {
                                    state.phase = FindPhase::NoResults;
                                } else {
                                    state.phase = FindPhase::ShowResult;
                                    state.current_match = 0;
                                }
                            }
                            KeyCode::Char('n') | KeyCode::Char('N') => {
                                state.pause_on_match = false;
                                // Start searching
                                state.phase = FindPhase::Searching;
                                let root = self.current_path.clone();
                                state.matches = find_files_recursive(&root, &state.pattern);
                                self.last_find_pattern = state.pattern.clone();
                                state.search_complete = true;

                                if state.matches.is_empty() {
                                    state.phase = FindPhase::NoResults;
                                } else {
                                    state.phase = FindPhase::ShowAllResults;
                                    state.scroll_offset = 0;
                                }
                            }
                            KeyCode::Esc => {
                                self.modal = Modal::None;
                            }
                            _ => {}
                        }
                    }
                    FindPhase::Searching => {
                        // Searching is synchronous for now, so this won't be hit
                        // In future could make async with progress display
                    }
                    FindPhase::ShowResult => {
                        match key.code {
                            KeyCode::Char('c') | KeyCode::Char('C') => {
                                // Continue to next match
                                if state.current_match + 1 < state.matches.len() {
                                    state.current_match += 1;
                                } else {
                                    // No more matches
                                    state.phase = FindPhase::NoResults;
                                }
                            }
                            KeyCode::Char('j') | KeyCode::Char('J') => {
                                // Jump to directory containing the file
                                if let Some((path, _)) = state.matches.get(state.current_match) {
                                    if let Some(parent) = path.parent() {
                                        let parent = parent.to_path_buf();
                                        let file_name = path
                                            .file_name()
                                            .map(|n| n.to_string_lossy().to_string());
                                        self.modal = Modal::None;
                                        if let Ok(()) = self.navigate_to(&parent) {
                                            // Try to select the file
                                            if let Some(name) = file_name {
                                                if let Some(idx) = self.files.iter().position(|f| {
                                                    let full_name = if f.extension.is_empty() {
                                                        f.name.clone()
                                                    } else {
                                                        format!(
                                                            "{}.{}",
                                                            f.name,
                                                            f.extension.to_lowercase()
                                                        )
                                                    };
                                                    full_name.eq_ignore_ascii_case(&name)
                                                }) {
                                                    self.selected_index = idx;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('v') | KeyCode::Char('V') => {
                                // View the file
                                if let Some((path, _)) = state.matches.get(state.current_match) {
                                    if let Ok(content) = std::fs::read(path) {
                                        let file_name = path
                                            .file_name()
                                            .map(|n| n.to_string_lossy().to_string())
                                            .unwrap_or_else(|| "file".to_string());
                                        let file_path = path.clone();
                                        self.modal = Modal::FileViewer(FileViewerState::new(
                                            file_name, file_path, content,
                                        ));
                                    }
                                }
                            }
                            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::ALT) => {
                                // Alt-E: Erase the found file (with confirmation in future)
                                // For now, skip this - requires confirmation workflow
                            }
                            KeyCode::Esc => {
                                self.modal = Modal::None;
                            }
                            _ => {}
                        }
                    }
                    FindPhase::ShowAllResults => {
                        let visible_height = 15; // Approximate visible lines

                        match key.code {
                            KeyCode::Up | KeyCode::Char('k') => {
                                // Move selection up
                                if state.current_match > 0 {
                                    state.current_match -= 1;
                                    // Adjust scroll to keep selection visible
                                    if state.current_match < state.scroll_offset {
                                        state.scroll_offset = state.current_match;
                                    }
                                }
                            }
                            KeyCode::Down => {
                                // Move selection down
                                if state.current_match + 1 < state.matches.len() {
                                    state.current_match += 1;
                                    // Adjust scroll to keep selection visible
                                    if state.current_match >= state.scroll_offset + visible_height {
                                        state.scroll_offset =
                                            state.current_match - visible_height + 1;
                                    }
                                }
                            }
                            KeyCode::PageUp => {
                                state.current_match =
                                    state.current_match.saturating_sub(visible_height);
                                state.scroll_offset =
                                    state.scroll_offset.saturating_sub(visible_height);
                            }
                            KeyCode::PageDown => {
                                let max = state.matches.len().saturating_sub(1);
                                state.current_match =
                                    (state.current_match + visible_height).min(max);
                                let max_scroll = state.matches.len().saturating_sub(visible_height);
                                state.scroll_offset =
                                    (state.scroll_offset + visible_height).min(max_scroll);
                            }
                            KeyCode::Home => {
                                state.current_match = 0;
                                state.scroll_offset = 0;
                            }
                            KeyCode::End => {
                                state.current_match = state.matches.len().saturating_sub(1);
                                let max_scroll = state.matches.len().saturating_sub(visible_height);
                                state.scroll_offset = max_scroll;
                            }
                            KeyCode::Char('j') | KeyCode::Char('J') => {
                                // Jump to directory containing the selected file
                                if let Some((path, _)) = state.matches.get(state.current_match) {
                                    if let Some(parent) = path.parent() {
                                        let parent = parent.to_path_buf();
                                        let file_name = path
                                            .file_name()
                                            .map(|n| n.to_string_lossy().to_string());
                                        self.modal = Modal::None;
                                        if let Ok(()) = self.navigate_to(&parent) {
                                            // Try to select the file
                                            if let Some(name) = file_name {
                                                if let Some(idx) = self.files.iter().position(|f| {
                                                    let full_name = if f.extension.is_empty() {
                                                        f.name.clone()
                                                    } else {
                                                        format!(
                                                            "{}.{}",
                                                            f.name,
                                                            f.extension.to_lowercase()
                                                        )
                                                    };
                                                    full_name.eq_ignore_ascii_case(&name)
                                                }) {
                                                    self.selected_index = idx;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('v') | KeyCode::Char('V') => {
                                // View the selected file
                                if let Some((path, _)) = state.matches.get(state.current_match) {
                                    if let Ok(content) = std::fs::read(path) {
                                        let file_name = path
                                            .file_name()
                                            .map(|n| n.to_string_lossy().to_string())
                                            .unwrap_or_else(|| "file".to_string());
                                        let file_path = path.clone();
                                        self.modal = Modal::FileViewer(FileViewerState::new(
                                            file_name, file_path, content,
                                        ));
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                // Jump to the selected file (same as J)
                                if let Some((path, _)) = state.matches.get(state.current_match) {
                                    if let Some(parent) = path.parent() {
                                        let parent = parent.to_path_buf();
                                        let file_name = path
                                            .file_name()
                                            .map(|n| n.to_string_lossy().to_string());
                                        self.modal = Modal::None;
                                        if let Ok(()) = self.navigate_to(&parent) {
                                            if let Some(name) = file_name {
                                                if let Some(idx) = self.files.iter().position(|f| {
                                                    let full_name = if f.extension.is_empty() {
                                                        f.name.clone()
                                                    } else {
                                                        format!(
                                                            "{}.{}",
                                                            f.name,
                                                            f.extension.to_lowercase()
                                                        )
                                                    };
                                                    full_name.eq_ignore_ascii_case(&name)
                                                }) {
                                                    self.selected_index = idx;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Esc => {
                                self.modal = Modal::None;
                            }
                            _ => {}
                        }
                    }
                    FindPhase::NoResults => {
                        // Any key closes
                        self.modal = Modal::None;
                    }
                }
            }
            Modal::BatchRename(ref mut state) => {
                match key.code {
                    KeyCode::Enter => {
                        // Rename the current file
                        if let Some((path, _)) = state.current_file().cloned() {
                            let new_name = state.input.clone();
                            if !new_name.is_empty()
                                && new_name
                                    != path
                                        .file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_default()
                            {
                                let new_path = path.parent().unwrap_or(&path).join(&new_name);
                                match fs::rename(&path, &new_path) {
                                    Ok(()) => {
                                        state.renamed_count += 1;
                                        state.last_error = None;
                                        // Remove from tagged files
                                        self.tagged_files.retain(|p| *p != path);
                                    }
                                    Err(e) => {
                                        state.last_error = Some(format!("Error: {}", e));
                                    }
                                }
                            }
                        }
                        // Move to next file
                        state.next();
                        if state.is_complete() {
                            // Done - refresh and show summary
                            let count = state.renamed_count;
                            self.modal = Modal::None;
                            let _ = self.refresh_files();
                            if count > 0 {
                                self.modal = Modal::Success(format!("Renamed {} file(s)", count));
                            }
                        }
                    }
                    KeyCode::Esc => {
                        // Exit batch rename
                        let count = state.renamed_count;
                        self.modal = Modal::None;
                        let _ = self.refresh_files();
                        if count > 0 {
                            self.modal = Modal::Success(format!("Renamed {} file(s)", count));
                        }
                    }
                    KeyCode::Backspace => {
                        state.input.pop();
                        state.last_error = None;
                    }
                    KeyCode::Char(c) => {
                        state.input.push(c);
                        state.last_error = None;
                    }
                    KeyCode::Tab => {
                        // Skip this file without renaming
                        state.next();
                        if state.is_complete() {
                            let count = state.renamed_count;
                            self.modal = Modal::None;
                            let _ = self.refresh_files();
                            if count > 0 {
                                self.modal = Modal::Success(format!("Renamed {} file(s)", count));
                            }
                        }
                    }
                    _ => {}
                }
            }
            Modal::Attribute(ref mut state) => {
                match key.code {
                    KeyCode::Left | KeyCode::Char('h') => {
                        state.prev_attr();
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        state.next_attr();
                    }
                    KeyCode::Char(' ') => {
                        // Toggle current attribute
                        state.toggle_current();
                    }
                    KeyCode::Enter => {
                        if state.display_only {
                            // Just close in display mode
                            self.modal = Modal::None;
                        } else {
                            // Apply attribute changes
                            let path = state.path.clone();
                            let attrs = state.attrs;
                            let for_tagged = state.for_tagged;

                            // Apply to all tagged files if for_tagged, otherwise just the one file
                            let files_to_update: Vec<PathBuf> = if for_tagged {
                                self.tagged_files.clone()
                            } else {
                                vec![path]
                            };

                            let mut success_count = 0;
                            let mut error_msg = None;

                            for file_path in files_to_update {
                                match apply_attributes(&file_path, &attrs) {
                                    Ok(()) => success_count += 1,
                                    Err(e) => {
                                        if error_msg.is_none() {
                                            error_msg = Some(format!("Error: {}", e));
                                        }
                                    }
                                }
                            }

                            self.modal = Modal::None;
                            let _ = self.refresh_files();

                            if let Some(err) = error_msg {
                                self.modal = Modal::Error(err);
                            } else if success_count > 0 {
                                self.modal = Modal::Success(format!(
                                    "Updated attributes for {} file(s)",
                                    success_count
                                ));
                            }
                        }
                    }
                    KeyCode::Esc => {
                        self.modal = Modal::None;
                    }
                    _ => {}
                }
            }
            Modal::SearchSpec(ref mut state) => {
                match state.phase {
                    0 => {
                        // Phase 0: Editing pattern
                        match key.code {
                            KeyCode::Enter => {
                                // Move to attribute selection phase
                                state.phase = 1;
                            }
                            KeyCode::Esc => {
                                self.modal = Modal::None;
                            }
                            KeyCode::Backspace => {
                                state.pattern.pop();
                            }
                            KeyCode::Char(c) => {
                                state.pattern.push(c);
                            }
                            _ => {}
                        }
                    }
                    1 => {
                        // Phase 1: Editing attributes
                        match key.code {
                            KeyCode::Left | KeyCode::Char('h') => {
                                if state.selected_attr > 0 {
                                    state.selected_attr -= 1;
                                }
                            }
                            KeyCode::Right | KeyCode::Char('l') => {
                                if state.selected_attr < 5 {
                                    state.selected_attr += 1;
                                }
                            }
                            KeyCode::Char(' ') => {
                                state.toggle_current();
                            }
                            KeyCode::Enter => {
                                // Apply the search specification
                                let pattern = state.pattern.clone();
                                self.search_spec = pattern;
                                self.modal = Modal::None;
                                let _ = self.refresh_files();
                            }
                            KeyCode::Esc => {
                                // Go back to pattern phase
                                state.phase = 0;
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            Modal::Help(ref mut state) => {
                match key.code {
                    KeyCode::Esc => {
                        if state.current_topic == 0 {
                            // On index page, close help
                            self.modal = Modal::None;
                        } else {
                            // On topic page, go back to index
                            state.current_topic = 0;
                            state.scroll_offset = 0;
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if state.scroll_offset > 0 {
                            state.scroll_offset -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        state.scroll_offset += 1;
                    }
                    KeyCode::PageUp => {
                        state.scroll_offset = state.scroll_offset.saturating_sub(10);
                    }
                    KeyCode::PageDown => {
                        state.scroll_offset += 10;
                    }
                    KeyCode::Home => {
                        state.scroll_offset = 0;
                    }
                    KeyCode::Char(c) => {
                        // Check if character matches a topic key
                        let c_upper = c.to_ascii_uppercase();
                        for (i, topic) in state.topics.iter().enumerate() {
                            if topic.key == c_upper {
                                state.current_topic = i + 1;
                                state.scroll_offset = 0;
                                break;
                            }
                        }
                    }
                    KeyCode::Enter => {
                        if state.current_topic == 0 {
                            // On index, go to first topic
                            state.current_topic = 1;
                            state.scroll_offset = 0;
                        }
                    }
                    _ => {}
                }
            }
            Modal::Status(_) | Modal::Space | Modal::Error(_) | Modal::Success(_) => {
                match key.code {
                    KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ') => {
                        self.modal = Modal::None;
                    }
                    _ => {}
                }
            }
            Modal::None => {}
        }

        Ok(())
    }

    /// Move file selection up or down
    fn move_selection(&mut self, delta: i32) {
        if self.files.is_empty() {
            return;
        }

        let new_index = if delta < 0 {
            self.selected_index.saturating_sub((-delta) as usize)
        } else {
            (self.selected_index + delta as usize).min(self.files.len() - 1)
        };

        self.selected_index = new_index;
    }

    /// Toggle tag on currently selected file
    fn toggle_tag(&mut self) {
        if self.files.is_empty() {
            return;
        }

        let file = &self.files[self.selected_index];
        let path = file.path.clone();

        if let Some(pos) = self.tagged_files.iter().position(|p| *p == path) {
            self.tagged_files.remove(pos);
        } else {
            self.tagged_files.push(path);
        }
    }

    /// Check if a file is tagged
    pub fn is_tagged(&self, path: &PathBuf) -> bool {
        self.tagged_files.contains(path)
    }

    /// Get total size of tagged files
    pub fn tagged_size(&self) -> u64 {
        self.tagged_files
            .iter()
            .filter_map(|p| self.files.iter().find(|f| &f.path == p).map(|f| f.size))
            .sum()
    }

    /// Go to parent directory
    fn go_to_parent(&mut self) -> Result<()> {
        if let Some(parent) = self.current_path.parent() {
            let parent = parent.to_path_buf();
            self.navigate_to(&parent)?;
        }
        Ok(())
    }

    /// Navigate to a new directory
    fn navigate_to(&mut self, path: &PathBuf) -> Result<()> {
        let canonical = path.canonicalize()?;

        if !canonical.is_dir() {
            anyhow::bail!("Not a directory");
        }

        // Save current path to history
        self.history.push(self.current_path.clone());

        // Update state
        self.current_path = canonical;
        self.files = get_directory_contents(&self.current_path, self.sort_mode)?;
        self.selected_index = 0;
        self.scroll_offset = 0;

        Ok(())
    }

    /// Cycle through sort modes
    fn cycle_sort_mode(&mut self) -> Result<()> {
        self.sort_mode = self.sort_mode.next();
        self.files = get_directory_contents(&self.current_path, self.sort_mode)?;
        Ok(())
    }

    /// Execute the current menu action or enter directory
    fn execute_action(&mut self) -> Result<()> {
        // If a directory is selected, enter it
        if !self.files.is_empty() {
            let file = &self.files[self.selected_index];
            if file.is_dir {
                let path = file.path.clone();
                return self.navigate_to(&path);
            }
        }

        // Otherwise, execute menu action
        let nav_item = NavItem::ALL[self.nav_index];
        match nav_item {
            NavItem::Directory => {
                // Open Directory Map tree view
                let state = DirectoryMapState::new(&self.current_path);
                self.modal = Modal::DirectoryMap(state);
            }
            NavItem::Tag => {
                self.toggle_tag();
            }
            NavItem::Space => {
                self.modal = Modal::Space;
            }
            NavItem::Copy => {
                if !self.tagged_files.is_empty() {
                    // Copy tagged files
                    let dest = self.current_path.to_string_lossy().to_string();
                    self.modal = Modal::CopyTo(dest);
                } else if self.files.is_empty() || self.files[self.selected_index].name == ".." {
                    self.modal = Modal::Error(errors::command::NO_FILES_FOR_COMMAND.to_string());
                } else {
                    // Copy the highlighted file - temporarily tag it
                    let file = &self.files[self.selected_index];
                    self.tagged_files.push(file.path.clone());
                    let dest = self.current_path.to_string_lossy().to_string();
                    self.modal = Modal::CopyTo(dest);
                }
            }
            NavItem::Move => {
                if !self.tagged_files.is_empty() {
                    // Move tagged files
                    let dest = self.current_path.to_string_lossy().to_string();
                    self.modal = Modal::MoveTo(dest);
                } else if self.files.is_empty() || self.files[self.selected_index].name == ".." {
                    self.modal = Modal::Error(errors::command::NO_FILES_FOR_COMMAND.to_string());
                } else {
                    // Move the highlighted file - temporarily tag it
                    let file = &self.files[self.selected_index];
                    self.tagged_files.push(file.path.clone());
                    let dest = self.current_path.to_string_lossy().to_string();
                    self.modal = Modal::MoveTo(dest);
                }
            }
            NavItem::Erase => {
                if !self.tagged_files.is_empty() {
                    // Erase tagged files
                    self.modal = Modal::EraseConfirm;
                } else if self.files.is_empty() || self.files[self.selected_index].name == ".." {
                    self.modal = Modal::Error(errors::command::NO_FILES_FOR_COMMAND.to_string());
                } else if self.files[self.selected_index].is_dir {
                    self.modal = Modal::Error(errors::file::CANNOT_ERASE_DIR.to_string());
                } else {
                    // Erase the highlighted file - temporarily tag it for the confirm dialog
                    let file = &self.files[self.selected_index];
                    self.tagged_files.push(file.path.clone());
                    self.modal = Modal::EraseConfirm;
                }
            }
            NavItem::Rename => {
                if !self.tagged_files.is_empty() {
                    // Batch rename for tagged files
                    let files = self.tagged_files.clone();
                    let state = BatchRenameState::new(files);
                    self.modal = Modal::BatchRename(state);
                } else if self.files.is_empty() || self.files[self.selected_index].name == ".." {
                    self.modal = Modal::Error(errors::command::NO_FILES_FOR_COMMAND.to_string());
                } else {
                    // Single file rename
                    let current_name = self.files[self.selected_index].name.clone();
                    let ext = &self.files[self.selected_index].extension;
                    let full_name = if ext.is_empty() {
                        current_name
                    } else {
                        format!("{}.{}", current_name, ext.to_lowercase())
                    };
                    self.modal = Modal::RenameInput(full_name);
                }
            }
            NavItem::View => {
                if self.files.is_empty() || self.files[self.selected_index].name == ".." {
                    self.modal = Modal::Error(errors::command::NO_FILES_FOR_COMMAND.to_string());
                } else if self.files[self.selected_index].is_dir {
                    self.modal = Modal::Error(errors::file::CANNOT_VIEW_DIR.to_string());
                } else {
                    let file = &self.files[self.selected_index];
                    match std::fs::read(&file.path) {
                        Ok(content) => {
                            let file_name = if file.extension.is_empty() {
                                file.name.clone()
                            } else {
                                format!("{}.{}", file.name, file.extension)
                            };
                            let file_path = file.path.clone();
                            self.modal = Modal::FileViewer(FileViewerState::new(
                                file_name, file_path, content,
                            ));
                        }
                        Err(_e) => {
                            self.modal =
                                Modal::Error(errors::file::CANNOT_OPEN_HIGHLIGHTED.to_string());
                        }
                    }
                }
            }
            NavItem::Find => {
                // Open Find dialog
                let state = FindState::new(self.last_find_pattern.clone());
                self.modal = Modal::Find(state);
            }
            NavItem::Attribute => {
                if !self.tagged_files.is_empty() {
                    // For tagged files, show attribute editor with N/C option
                    // We'll use the first tagged file as reference
                    let path = self.tagged_files[0].clone();
                    let state = AttributeState::new(path, true);
                    self.modal = Modal::Attribute(state);
                } else if self.files.is_empty() || self.files[self.selected_index].name == ".." {
                    // No file selected - show display mode for current file or error
                    self.modal =
                        Modal::Error("No file selected for attribute display.".to_string());
                } else {
                    // Single file - show attribute editor
                    let file = &self.files[self.selected_index];
                    let state = AttributeState::new(file.path.clone(), false);
                    self.modal = Modal::Attribute(state);
                }
            }
            NavItem::Print => {
                self.modal = Modal::Error("Print not yet implemented".to_string());
            }
        }

        Ok(())
    }

    /// Get count of directories in file list
    pub fn dir_count(&self) -> usize {
        self.files.iter().filter(|f| f.is_dir).count()
    }

    /// Get count of regular files in file list
    pub fn file_count(&self) -> usize {
        self.files.iter().filter(|f| !f.is_dir).count()
    }

    /// Get total size of all files
    pub fn total_size(&self) -> u64 {
        self.files
            .iter()
            .filter(|f| !f.is_dir)
            .map(|f| f.size)
            .sum()
    }

    /// Tab completion for paths
    fn tab_complete(partial: &str) -> Option<String> {
        let path = PathBuf::from(partial);

        // Determine the directory to search and the prefix to match
        let (search_dir, prefix) = if partial.ends_with('/') || partial.ends_with('\\') {
            (path.clone(), String::new())
        } else if let Some(parent) = path.parent() {
            let file_name = path.file_name()?.to_string_lossy().to_string();
            (parent.to_path_buf(), file_name)
        } else {
            (PathBuf::from("."), partial.to_string())
        };

        // Read directory and find matches
        let entries = fs::read_dir(&search_dir).ok()?;
        let mut matches: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                if name.to_lowercase().starts_with(&prefix.to_lowercase()) {
                    let full_path = search_dir.join(&name);
                    let mut result = full_path.to_string_lossy().to_string();
                    // Add trailing slash for directories
                    if full_path.is_dir() && !result.ends_with('/') {
                        result.push('/');
                    }
                    Some(result)
                } else {
                    None
                }
            })
            .collect();

        matches.sort();

        // Return first match, or find common prefix if multiple matches
        if matches.len() == 1 {
            Some(matches.remove(0))
        } else if matches.len() > 1 {
            // Find common prefix among all matches
            let first = &matches[0];
            let mut common_len = first.len();
            for m in &matches[1..] {
                common_len = first
                    .chars()
                    .zip(m.chars())
                    .take_while(|(a, b)| a == b)
                    .count()
                    .min(common_len);
            }
            if common_len > partial.len() {
                Some(first[..common_len].to_string())
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Refresh the current file list
    fn refresh_files(&mut self) -> Result<()> {
        self.files = get_directory_contents(&self.current_path, self.sort_mode)?;
        if self.selected_index >= self.files.len() && !self.files.is_empty() {
            self.selected_index = self.files.len() - 1;
        }
        Ok(())
    }

    /// Copy tagged files to destination directory
    fn copy_tagged_files(&mut self, dest: &PathBuf) -> Result<usize> {
        if self.tagged_files.is_empty() {
            anyhow::bail!("No files tagged");
        }

        let dest_dir = if dest.is_dir() {
            dest.clone()
        } else {
            dest.parent().unwrap_or(dest).to_path_buf()
        };

        if !dest_dir.exists() {
            fs::create_dir_all(&dest_dir)?;
        }

        let mut count = 0;
        for src_path in &self.tagged_files.clone() {
            if let Some(file_name) = src_path.file_name() {
                let dest_path = dest_dir.join(file_name);
                if src_path.is_dir() {
                    copy_dir_recursive(src_path, &dest_path)?;
                } else {
                    fs::copy(src_path, &dest_path)?;
                }
                count += 1;
            }
        }

        self.tagged_files.clear();
        Ok(count)
    }

    /// Move tagged files to destination directory
    fn move_tagged_files(&mut self, dest: &PathBuf) -> Result<usize> {
        if self.tagged_files.is_empty() {
            anyhow::bail!("No files tagged");
        }

        let dest_dir = if dest.is_dir() {
            dest.clone()
        } else {
            dest.parent().unwrap_or(dest).to_path_buf()
        };

        if !dest_dir.exists() {
            fs::create_dir_all(&dest_dir)?;
        }

        let mut count = 0;
        for src_path in &self.tagged_files.clone() {
            if let Some(file_name) = src_path.file_name() {
                let dest_path = dest_dir.join(file_name);
                fs::rename(src_path, &dest_path)?;
                count += 1;
            }
        }

        self.tagged_files.clear();
        Ok(count)
    }

    /// Erase (delete) tagged files
    fn erase_tagged_files(&mut self) -> Result<usize> {
        if self.tagged_files.is_empty() {
            anyhow::bail!("No files tagged");
        }

        let mut count = 0;
        for path in &self.tagged_files.clone() {
            if path.is_dir() {
                fs::remove_dir_all(path)?;
            } else {
                fs::remove_file(path)?;
            }
            count += 1;
        }

        self.tagged_files.clear();
        Ok(count)
    }

    /// Rename the selected file
    fn rename_selected_file(&mut self, new_name: &str) -> Result<()> {
        if self.files.is_empty() {
            anyhow::bail!("No file selected");
        }

        let file = &self.files[self.selected_index];
        if file.name == ".." {
            anyhow::bail!("Cannot rename parent directory");
        }

        let new_path = file
            .path
            .parent()
            .unwrap_or(&self.current_path)
            .join(new_name);
        fs::rename(&file.path, &new_path)?;

        Ok(())
    }

    /// Get files to operate on (tagged files or selected file)
    #[allow(dead_code)]
    pub fn get_operation_targets(&self) -> Vec<PathBuf> {
        if !self.tagged_files.is_empty() {
            self.tagged_files.clone()
        } else if !self.files.is_empty() && self.files[self.selected_index].name != ".." {
            vec![self.files[self.selected_index].path.clone()]
        } else {
            Vec::new()
        }
    }
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &PathBuf, dest: &PathBuf) -> Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
}

/// Execute a shell command and return (output lines, exit code)
fn execute_shell_command_impl(cmd: &str, cwd: &PathBuf) -> (Vec<String>, i32) {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

    let result = Command::new(&shell)
        .arg("-c")
        .arg(cmd)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match result {
        Ok(output) => {
            let mut lines = Vec::new();

            // Add stdout lines
            let stdout_reader = BufReader::new(&output.stdout[..]);
            for line in stdout_reader.lines() {
                if let Ok(l) = line {
                    lines.push(l);
                }
            }

            // Add stderr lines
            let stderr_reader = BufReader::new(&output.stderr[..]);
            for line in stderr_reader.lines() {
                if let Ok(l) = line {
                    lines.push(format!("stderr: {}", l));
                }
            }

            let exit_code = output.status.code().unwrap_or(-1);
            (lines, exit_code)
        }
        Err(e) => (vec![format!("Error executing command: {}", e)], -1),
    }
}
