//! Q-DOS II state types and enums
//!
//! This module contains all the state structs and enums used throughout the application.

use crate::file_ops::SystemInfo;
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
    Git,
    Beads,
    Space,
    Attribute,
    Print,
}

impl NavItem {
    pub const ALL: [NavItem; 13] = [
        NavItem::Directory,
        NavItem::Tag,
        NavItem::View,
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

/// File viewer display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Normal,
    Hex,
    Image,
    Markdown,
    Blame,
    Diff,
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

/// Git file history entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileHistoryEntry {
    pub hash: String,
    pub date: String,
    pub message: String,
}

/// Git blame line entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlameLine {
    pub hash: String,
    pub author: String,
    pub time_ago: String,
    pub line_content: String,
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
    /// Git history for this file (oldest to newest, index 0 = oldest)
    pub git_history: Vec<FileHistoryEntry>,
    /// Current position in git history (None = current working copy)
    pub history_index: Option<usize>,
    /// Whether we're in a git repo
    pub is_git_repo: bool,
    /// Git blame data for blame view
    pub blame_lines: Vec<BlameLine>,
    /// Git diff lines for diff view
    pub diff_lines: Vec<String>,
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
            git_history: Vec::new(),
            history_index: None,
            is_git_repo: false,
            blame_lines: Vec::new(),
            diff_lines: Vec::new(),
        }
    }

    /// Check if we can go to an older version (there's older history)
    pub fn has_older_version(&self) -> bool {
        if self.git_history.is_empty() {
            return false;
        }
        match self.history_index {
            None => true,         // Currently at working copy, can go back
            Some(idx) => idx > 0, // Can go back if not at oldest
        }
    }

    /// Check if we can go to a newer version
    pub fn has_newer_version(&self) -> bool {
        if self.git_history.is_empty() {
            return false;
        }
        self.history_index.is_some() // If we're in history, we can go forward
    }

    /// Get current commit info (None if viewing working copy)
    pub fn current_commit(&self) -> Option<&FileHistoryEntry> {
        self.history_index.and_then(|idx| self.git_history.get(idx))
    }

    /// Set git history for this file
    pub fn set_git_history(&mut self, history: Vec<FileHistoryEntry>, is_git_repo: bool) {
        self.git_history = history;
        self.is_git_repo = is_git_repo;
    }

    /// Calculate max scroll offset based on mode and visible height
    pub fn max_scroll(&self, visible_height: usize) -> usize {
        match self.mode {
            ViewMode::Normal | ViewMode::Markdown => {
                let line_count = self.content.split(|&b| b == b'\n').count();
                line_count.saturating_sub(visible_height)
            }
            ViewMode::Hex => {
                let bytes_per_line = 16;
                let total_lines = self.content.len().div_ceil(bytes_per_line);
                total_lines.saturating_sub(visible_height)
            }
            ViewMode::Image => 0,
            ViewMode::Blame => self.blame_lines.len().saturating_sub(visible_height),
            ViewMode::Diff => self.diff_lines.len().saturating_sub(visible_height),
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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ShellCommandState {
    pub input: String,
    pub output: Vec<String>,
    pub running: bool,
    pub exit_code: Option<i32>,
    pub scroll_offset: usize,
    pub history: Vec<String>,
    pub history_index: Option<usize>,
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

    pub fn load_children(&mut self) {
        if !self.children.is_empty() {
            return;
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
    pub flat_list: Vec<(PathBuf, usize, bool, bool)>,
    pub input_mode: Option<String>,
    pub input_buffer: String,
    pub confirm_delete: Option<PathBuf>,
}

impl DirectoryMapState {
    pub fn new(start_path: &PathBuf) -> Self {
        let root_path = if let Some(root) = start_path.ancestors().last() {
            root.to_path_buf()
        } else {
            start_path.clone()
        };

        let mut root = DirTreeNode::new(root_path.to_string_lossy().to_string(), root_path, 0);
        root.expanded = true;
        root.load_children();

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

        if let Some(idx) = state
            .flat_list
            .iter()
            .position(|(p, _, _, _)| p == start_path)
        {
            state.selected_index = idx;
        }

        state
    }

    fn expand_to_path(&mut self, target: &PathBuf) {
        let ancestors: Vec<_> = target.ancestors().collect();
        for ancestor in ancestors.into_iter().rev() {
            self.expand_path_in_tree(&self.root.clone(), &ancestor.to_path_buf());
        }
    }

    fn expand_path_in_tree(&mut self, _node: &DirTreeNode, target: &PathBuf) {
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

    pub fn rebuild_flat_list(&mut self) {
        self.flat_list.clear();
        fn flatten(node: &DirTreeNode, list: &mut Vec<(PathBuf, usize, bool, bool)>) {
            let has_children = !node.children.is_empty() || {
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

    pub fn selected_path(&self) -> Option<PathBuf> {
        self.flat_list
            .get(self.selected_index)
            .map(|(p, _, _, _)| p.clone())
    }
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

/// Search specification state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchSpecState {
    pub pattern: String,
    pub phase: u8,
    pub attrs: [bool; 6],
    pub selected_attr: usize,
}

impl SearchSpecState {
    pub fn new(current_spec: &str) -> Self {
        Self {
            pattern: current_spec.to_string(),
            phase: 0,
            attrs: [true, true, false, false, false, false],
            selected_attr: 0,
        }
    }

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
    pub topics: Vec<HelpTopic>,
    pub current_topic: usize,
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
F9  - Edit: Open file in default text editor
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
            HelpTopic {
                key: 'G',
                title: "Git Integration".to_string(),
                content: r#"GIT INTEGRATION

Press G to open the Git menu (requires git repository).

MENU OPTIONS:
S - Status:  View modified/staged files
L - Log:     View recent commit history
D - Diff:    View unstaged changes
C - Commit:  Create a new commit
P - Push:    Push commits to remote
U - Pull:    Pull changes from remote

STATUS VIEW:
↑/↓ - Select file
A   - Stage/unstage file
R   - Refresh status

LOG VIEW:
↑/↓ - Scroll through commits
PgUp/PgDn - Scroll faster

DIFF VIEW:
↑/↓ - Scroll through diff
PgUp/PgDn - Scroll faster

COMMIT VIEW:
Type your commit message and press Enter to commit.
Press ESC to cancel.

PUSH/PULL:
Select from menu or press P/U to execute immediately."#
                    .to_string(),
            },
            HelpTopic {
                key: 'B',
                title: "Beads Integration".to_string(),
                content: r#"BEADS ISSUE TRACKER

Press B to open the Beads menu (requires .beads directory).

MENU OPTIONS:
L - List:    View open issues
R - Ready:   View issues ready to work (no blockers)
B - Blocked: View blocked issues
S - Stats:   View project statistics
C - Create:  Create a new issue

LIST/READY/BLOCKED VIEWS:
↑/↓   - Select issue
Enter - View issue details
R     - Refresh list
C     - Close selected issue
S     - Start work (set to in_progress)

CREATE VIEW:
Use arrow keys to navigate fields:
- Title: Type the issue title
- Type: task/bug/feature (←/→ to change)
- Priority: 0-4 (←/→ to change)
Press Enter to create the issue.

STATS VIEW:
Shows total, open, in-progress, closed, and blocked counts."#
                    .to_string(),
            },
        ]
    }
}

impl Default for HelpState {
    fn default() -> Self {
        Self::new()
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

    /// Get the RGB color values for this theme
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

/// QDSTART configuration fields
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QdstartField {
    SearchSpec,
    SortMethod,
    SortDirection,
    ShowHidden,
    ConfirmDelete,
    Editor,
    ColorTheme,
    MouseSupport,
    UppercaseNames,
}

impl QdstartField {
    pub const ALL: [QdstartField; 9] = [
        QdstartField::SearchSpec,
        QdstartField::SortMethod,
        QdstartField::SortDirection,
        QdstartField::ShowHidden,
        QdstartField::ConfirmDelete,
        QdstartField::Editor,
        QdstartField::ColorTheme,
        QdstartField::MouseSupport,
        QdstartField::UppercaseNames,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            QdstartField::SearchSpec => "Search Specification",
            QdstartField::SortMethod => "Sort Method",
            QdstartField::SortDirection => "Sort Direction",
            QdstartField::ShowHidden => "Show Hidden Files",
            QdstartField::ConfirmDelete => "Confirm Delete",
            QdstartField::Editor => "Default Editor",
            QdstartField::ColorTheme => "Color Theme",
            QdstartField::MouseSupport => "Mouse Support",
            QdstartField::UppercaseNames => "Uppercase Names",
        }
    }
}

/// State for QDSTART configuration modal
#[derive(Debug, Clone)]
pub struct QdstartState {
    /// Currently selected field
    pub selected: usize,
    /// Editing mode (for text input fields)
    pub editing: bool,
    /// Input buffer for text fields
    pub input_buffer: String,
    /// Search specification
    pub search_spec: String,
    /// Sort method (0=name, 1=ext, 2=size, 3=date, 4=none)
    pub sort_method: usize,
    /// Sort direction (true=asc, false=desc)
    pub sort_asc: bool,
    /// Show hidden files
    pub show_hidden: bool,
    /// Confirm before delete
    pub confirm_delete: bool,
    /// Editor command (None = use $EDITOR)
    pub editor: Option<String>,
    /// Color theme index
    pub theme_index: usize,
    /// Mouse support enabled
    pub mouse_support: bool,
    /// Show filenames in uppercase
    pub uppercase_names: bool,
}

impl QdstartState {
    pub fn new(
        search_spec: String,
        sort_method: usize,
        sort_asc: bool,
        show_hidden: bool,
        confirm_delete: bool,
        editor: Option<String>,
        theme_index: usize,
        mouse_support: bool,
        uppercase_names: bool,
    ) -> Self {
        Self {
            selected: 0,
            editing: false,
            input_buffer: String::new(),
            search_spec,
            sort_method,
            sort_asc,
            show_hidden,
            confirm_delete,
            editor,
            theme_index,
            mouse_support,
            uppercase_names,
        }
    }

    pub fn current_field(&self) -> QdstartField {
        QdstartField::ALL[self.selected]
    }

    pub fn sort_method_name(&self) -> &'static str {
        match self.sort_method {
            0 => "Name",
            1 => "Extension",
            2 => "Size",
            3 => "Date",
            _ => "None",
        }
    }

    pub fn cycle_sort_method(&mut self) {
        self.sort_method = (self.sort_method + 1) % 5;
    }

    pub fn toggle_sort_direction(&mut self) {
        self.sort_asc = !self.sort_asc;
    }

    pub fn cycle_theme(&mut self) {
        self.theme_index = (self.theme_index + 1) % ColorTheme::ALL.len();
    }

    pub fn theme(&self) -> ColorTheme {
        ColorTheme::ALL[self.theme_index]
    }
}

/// Git menu item options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GitMenuItem {
    #[default]
    Status,
    Log,
    Diff,
    Commit,
    Push,
    Pull,
    Branch,
    Stash,
    Tag,
    Config,
    Conflicts,
    Submodules,
}

impl GitMenuItem {
    pub const ALL: [GitMenuItem; 12] = [
        GitMenuItem::Status,
        GitMenuItem::Log,
        GitMenuItem::Diff,
        GitMenuItem::Commit,
        GitMenuItem::Push,
        GitMenuItem::Pull,
        GitMenuItem::Branch,
        GitMenuItem::Stash,
        GitMenuItem::Tag,
        GitMenuItem::Config,
        GitMenuItem::Conflicts,
        GitMenuItem::Submodules,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            GitMenuItem::Status => "Status",
            GitMenuItem::Log => "Log",
            GitMenuItem::Diff => "Diff",
            GitMenuItem::Commit => "Commit",
            GitMenuItem::Push => "Push",
            GitMenuItem::Pull => "Pull",
            GitMenuItem::Branch => "Branch",
            GitMenuItem::Stash => "Stash",
            GitMenuItem::Tag => "Tag",
            GitMenuItem::Config => "Config",
            GitMenuItem::Submodules => "Submodules",
            GitMenuItem::Conflicts => "Conflicts",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            GitMenuItem::Status => "Show working tree status",
            GitMenuItem::Log => "View commit history",
            GitMenuItem::Diff => "Show changes in working directory",
            GitMenuItem::Commit => "Commit staged changes",
            GitMenuItem::Push => "Push commits to remote",
            GitMenuItem::Pull => "Pull changes from remote",
            GitMenuItem::Branch => "List, switch, create, delete branches",
            GitMenuItem::Stash => "Stash and restore changes",
            GitMenuItem::Tag => "Manage git tags",
            GitMenuItem::Config => "View git configuration",
            GitMenuItem::Conflicts => "Resolve merge conflicts",
        }
    }
}

/// Git status file entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitFileStatus {
    pub path: String,
    pub status: char, // M, A, D, R, C, U, ?
    pub staged: bool,
}

/// Git branch entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitBranch {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
    pub last_commit: String,
}

/// Git stash entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitStashEntry {
    pub index: usize,
    pub message: String,
    pub branch: String,
}

/// Git tag entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitTag {
    pub name: String,
    pub commit: String,
    pub message: Option<String>,
}

/// Git remote entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitRemote {
    pub name: String,
    pub url: String,
}

/// Git conflict section (ours vs theirs)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictSection {
    pub start_line: usize,
    pub ours: Vec<String>,
    pub theirs: Vec<String>,
    pub resolved: Option<ConflictResolution>,
}

/// How a conflict was resolved
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    Ours,
    Theirs,
    Both,
}

/// Git conflict file with sections
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictFile {
    pub path: String,
    pub sections: Vec<ConflictSection>,
    pub selected_section: usize,
}

/// Remote action type (push or pull)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RemoteAction {
    #[default]
    Push,
    Pull,
}

/// Git state for the git modal
#[derive(Debug, Clone)]
pub struct GitState {
    /// Current view in git modal
    pub view: GitView,
    /// Selected menu item
    pub menu_selected: usize,
    /// Files with git status
    pub files: Vec<GitFileStatus>,
    /// Selected file in status view
    pub selected_file: usize,
    /// Scroll offset for status view
    pub scroll_offset: usize,
    /// Commit log entries
    pub log_entries: Vec<GitLogEntry>,
    /// Selected log entry in log view
    pub selected_log: usize,
    /// Diff content
    pub diff_content: Vec<String>,
    /// Commit message input
    pub commit_message: String,
    /// Whether in commit input mode
    pub commit_input_mode: bool,
    /// Error message if any
    pub error: Option<String>,
    /// Whether we're in a git repository
    pub is_repo: bool,
    /// Previous view to return to from diff
    pub prev_view: Option<GitView>,
    /// Branch list
    pub branches: Vec<GitBranch>,
    /// Selected branch in branch view
    pub selected_branch: usize,
    /// Input mode for branch creation
    pub branch_input_mode: bool,
    /// New branch name input
    pub branch_name_input: String,
    /// Stash list
    pub stashes: Vec<GitStashEntry>,
    /// Selected stash in stash view
    pub selected_stash: usize,
    /// Input mode for stash message
    pub stash_input_mode: bool,
    /// Stash message input
    pub stash_message_input: String,
    /// Tag list
    pub tags: Vec<GitTag>,
    /// Selected tag in tag view
    pub selected_tag: usize,
    /// Input mode for tag creation
    pub tag_input_mode: bool,
    /// New tag name input
    pub tag_name_input: String,
    /// Remote list
    pub remotes: Vec<GitRemote>,
    /// Selected remote in remote view
    pub selected_remote: usize,
    /// Current remote action (push or pull)
    pub remote_action: RemoteAction,
    /// Config entries
    pub config_entries: Vec<GitConfigEntry>,
    /// Selected config entry
    pub selected_config: usize,
    /// Conflict files
    pub conflict_files: Vec<ConflictFile>,
    /// Selected conflict file
    pub selected_conflict_file: usize,
    /// Submodules list
    pub submodules: Vec<GitSubmodule>,
    /// Selected submodule
    pub selected_submodule: usize,
}

/// Git view type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GitView {
    #[default]
    Menu,
    Status,
    Log,
    Diff,
    Commit,
    Branch,
    Stash,
    Tag,
    Remote,
    Config,
    Conflicts,
    Submodules,
}

/// Git config entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitConfigEntry {
    pub key: String,
    pub value: String,
    pub scope: String, // "local", "global", or "system"
}

/// Git log entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitLogEntry {
    pub hash: String,
    pub author: String,
    pub date: String,
    pub message: String,
}

/// Git submodule entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitSubmodule {
    pub name: String,
    pub path: String,
    pub url: String,
    pub status: SubmoduleStatus,
    pub commit: String,
}

/// Submodule status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SubmoduleStatus {
    #[default]
    Uninitialized,
    Initialized,
    OutOfDate,
    Modified,
    Conflict,
}

impl Default for GitState {
    fn default() -> Self {
        Self {
            view: GitView::Menu,
            menu_selected: 0,
            files: Vec::new(),
            selected_file: 0,
            scroll_offset: 0,
            log_entries: Vec::new(),
            selected_log: 0,
            diff_content: Vec::new(),
            commit_message: String::new(),
            commit_input_mode: false,
            error: None,
            is_repo: false,
            prev_view: None,
            branches: Vec::new(),
            selected_branch: 0,
            branch_input_mode: false,
            branch_name_input: String::new(),
            stashes: Vec::new(),
            selected_stash: 0,
            stash_input_mode: false,
            stash_message_input: String::new(),
            tags: Vec::new(),
            selected_tag: 0,
            tag_input_mode: false,
            tag_name_input: String::new(),
            remotes: Vec::new(),
            selected_remote: 0,
            remote_action: RemoteAction::Push,
            config_entries: Vec::new(),
            selected_config: 0,
            conflict_files: Vec::new(),
            selected_conflict_file: 0,
            submodules: Vec::new(),
            selected_submodule: 0,
        }
    }
}

impl GitState {
    pub fn new(is_repo: bool) -> Self {
        Self {
            is_repo,
            ..Default::default()
        }
    }
}

/// Beads menu item options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BeadsMenuItem {
    #[default]
    List,
    Ready,
    Blocked,
    Stats,
    Create,
    Graph,
    Kanban,
    Sync,
    Human,
    Init,
    Doctor,
}

impl BeadsMenuItem {
    /// Items shown when beads is initialized
    pub const INITIALIZED: [BeadsMenuItem; 10] = [
        BeadsMenuItem::List,
        BeadsMenuItem::Ready,
        BeadsMenuItem::Blocked,
        BeadsMenuItem::Stats,
        BeadsMenuItem::Create,
        BeadsMenuItem::Graph,
        BeadsMenuItem::Kanban,
        BeadsMenuItem::Sync,
        BeadsMenuItem::Human,
        BeadsMenuItem::Doctor,
    ];

    /// Items shown when beads is NOT initialized
    pub const NOT_INITIALIZED: [BeadsMenuItem; 1] = [BeadsMenuItem::Init];

    /// Get menu items based on initialization state
    pub fn items(is_initialized: bool) -> &'static [BeadsMenuItem] {
        if is_initialized {
            &Self::INITIALIZED
        } else {
            &Self::NOT_INITIALIZED
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            BeadsMenuItem::List => "List",
            BeadsMenuItem::Ready => "Ready",
            BeadsMenuItem::Blocked => "Blocked",
            BeadsMenuItem::Stats => "Stats",
            BeadsMenuItem::Create => "Create",
            BeadsMenuItem::Graph => "Graph",
            BeadsMenuItem::Kanban => "Kanban",
            BeadsMenuItem::Sync => "Sync",
            BeadsMenuItem::Human => "Human",
            BeadsMenuItem::Init => "Init",
            BeadsMenuItem::Doctor => "Doctor",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            BeadsMenuItem::List => "List all open issues",
            BeadsMenuItem::Ready => "Show issues ready to work on",
            BeadsMenuItem::Blocked => "Show blocked issues",
            BeadsMenuItem::Stats => "Project statistics",
            BeadsMenuItem::Create => "Create a new issue",
            BeadsMenuItem::Graph => "View dependency graph",
            BeadsMenuItem::Kanban => "Kanban board view",
            BeadsMenuItem::Sync => "Sync with git remote",
            BeadsMenuItem::Human => "Show common commands help",
            BeadsMenuItem::Init => "Initialize beads in this project",
            BeadsMenuItem::Doctor => "Check beads installation health",
        }
    }
}

/// Beads issue entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeadsIssue {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub issue_type: String,
    pub blocked_by: Vec<String>,
    pub dependents: Vec<BeadsSubIssue>,
    pub comments: Vec<BeadsComment>,
}

/// Sub-issue info for epic children
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeadsSubIssue {
    pub id: String,
    pub title: String,
    pub status: String,
    pub issue_type: String,
}

/// Comment on an issue
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeadsComment {
    pub author: String,
    pub text: String,
    pub created_at: String,
}

/// Beads activity/history entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeadsActivityEntry {
    pub timestamp: String,
    pub event_type: String,
    pub symbol: String,
    pub message: String,
    pub old_status: Option<String>,
    pub new_status: Option<String>,
    pub actor: Option<String>,
}

/// Beads view type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BeadsView {
    #[default]
    Menu,
    List,
    Ready,
    Blocked,
    Stats,
    Create,
    Detail,
    Comments,
    Dependencies,
    Kanban,
    History,
    FileIssues,
    Human,
    Doctor,
}

/// Beads state for the beads modal
#[derive(Debug, Clone)]
pub struct BeadsState {
    /// Current view in beads modal
    pub view: BeadsView,
    /// Selected menu item
    pub menu_selected: usize,
    /// All issues
    pub issues: Vec<BeadsIssue>,
    /// Selected issue in list view
    pub selected_issue: usize,
    /// Detailed issue (loaded when viewing detail)
    pub detail_issue: Option<BeadsIssue>,
    /// Selected subtask in detail view (for epics)
    pub selected_subtask: usize,
    /// Scroll offset for list view
    pub scroll_offset: usize,
    /// Stats data
    pub stats: BeadsStats,
    /// Create form state
    pub create_title: String,
    pub create_type: usize,
    pub create_priority: usize,
    pub create_field: usize,
    /// Whether we're in a beads-enabled project
    pub is_beads_project: bool,
    /// Error message if any
    pub error: Option<String>,
    /// Success message if any
    pub success_message: Option<String>,
    /// Output lines for Human/Doctor views
    pub output_lines: Vec<String>,
    /// Search query for filtering issues
    pub search_query: String,
    /// Whether search input is active
    pub search_active: bool,
    /// Comment text input
    pub comment_input: String,
    /// Whether comment input is active
    pub comment_input_active: bool,
    /// Selected comment in comments view
    pub selected_comment: usize,
    /// Current kanban column (0=Open, 1=In Progress, 2=Closed)
    pub kanban_column: usize,
    /// Selected row within current kanban column
    pub kanban_row: usize,
    /// Activity/history entries for timeline view
    pub activity_entries: Vec<BeadsActivityEntry>,
    /// Selected activity entry in history view
    pub selected_activity: usize,
    /// Current file path being queried for related issues
    pub file_query_path: String,
    /// Issues related to the currently queried file
    pub file_related_issues: Vec<BeadsIssue>,
    /// Selected issue in file-issues view
    pub file_issue_selected: usize,
}

/// Beads project statistics
#[derive(Debug, Clone, Default)]
pub struct BeadsStats {
    pub total: usize,
    pub open: usize,
    pub in_progress: usize,
    pub closed: usize,
    pub blocked: usize,
}

impl Default for BeadsState {
    fn default() -> Self {
        Self {
            view: BeadsView::Menu,
            menu_selected: 0,
            issues: Vec::new(),
            selected_issue: 0,
            detail_issue: None,
            selected_subtask: 0,
            scroll_offset: 0,
            stats: BeadsStats::default(),
            create_title: String::new(),
            create_type: 0,
            create_priority: 2,
            create_field: 0,
            is_beads_project: false,
            error: None,
            success_message: None,
            output_lines: Vec::new(),
            search_query: String::new(),
            search_active: false,
            comment_input: String::new(),
            comment_input_active: false,
            selected_comment: 0,
            kanban_column: 0,
            kanban_row: 0,
            activity_entries: Vec::new(),
            selected_activity: 0,
            file_query_path: String::new(),
            file_related_issues: Vec::new(),
            file_issue_selected: 0,
        }
    }
}

impl BeadsState {
    pub fn new(is_beads_project: bool) -> Self {
        Self {
            is_beads_project,
            ..Default::default()
        }
    }
}

/// Modal dialog types
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
    Progress(ProgressState),
    ColorTheme(ColorThemeState),
    Qdstart(QdstartState),
    Git(GitState),
    Beads(BeadsState),
}
