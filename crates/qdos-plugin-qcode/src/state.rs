//! Q-CODE State Management
//!
//! State for the code editor/IDE plugin.

use std::path::PathBuf;

// =============================================================================
// VIEWS
// =============================================================================

/// Main view mode for Q-CODE
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QCodeView {
    /// File tree browser on the left
    #[default]
    FileTree,
    /// Code editor view
    Editor,
    /// Terminal output pane
    Terminal,
    /// Help overlay
    Help,
}

// =============================================================================
// FILE TREE
// =============================================================================

/// Entry in the file tree
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub depth: usize,
    pub expanded: bool,
}

impl FileEntry {
    pub fn new(name: String, path: PathBuf, is_dir: bool, depth: usize) -> Self {
        Self {
            name,
            path,
            is_dir,
            depth,
            expanded: false,
        }
    }
}

// =============================================================================
// EDITOR BUFFER
// =============================================================================

/// A single editor buffer representing an open file
#[derive(Debug, Clone)]
pub struct EditorBuffer {
    /// Lines of text in the buffer
    pub lines: Vec<String>,
    /// Current cursor row (0-indexed)
    pub cursor_row: usize,
    /// Current cursor column (0-indexed)
    pub cursor_col: usize,
    /// File path (None for unsaved buffers)
    pub file_path: Option<PathBuf>,
    /// Whether the buffer has been modified
    pub modified: bool,
    /// Vertical scroll offset
    pub scroll_offset: usize,
    /// Horizontal scroll offset
    pub h_scroll_offset: usize,
    /// Goal column for vertical navigation
    pub goal_col: Option<usize>,
}

impl Default for EditorBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorBuffer {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            file_path: None,
            modified: false,
            scroll_offset: 0,
            h_scroll_offset: 0,
            goal_col: None,
        }
    }

    pub fn from_file(path: PathBuf) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(&path)?;
        let lines: Vec<String> = content.lines().map(String::from).collect();
        let lines = if lines.is_empty() {
            vec![String::new()]
        } else {
            lines
        };

        Ok(Self {
            lines,
            cursor_row: 0,
            cursor_col: 0,
            file_path: Some(path),
            modified: false,
            scroll_offset: 0,
            h_scroll_offset: 0,
            goal_col: None,
        })
    }

    pub fn display_name(&self) -> String {
        self.file_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "[Untitled]".to_string())
    }

    pub fn extension(&self) -> Option<String> {
        self.file_path
            .as_ref()
            .and_then(|p| p.extension())
            .map(|e| e.to_string_lossy().to_lowercase())
    }

    pub fn current_line(&self) -> &str {
        self.lines
            .get(self.cursor_row)
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    pub fn save(&mut self) -> std::io::Result<()> {
        if let Some(ref path) = self.file_path {
            let content = self.lines.join("\n");
            std::fs::write(path, content)?;
            self.modified = false;
        }
        Ok(())
    }

    // =========================================================================
    // CURSOR MOVEMENT
    // =========================================================================

    pub fn move_up(&mut self) {
        if self.cursor_row > 0 {
            let goal = self.goal_col.unwrap_or(self.cursor_col);
            self.goal_col = Some(goal);
            self.cursor_row -= 1;
            self.cursor_col = goal.min(self.current_line().len());
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor_row + 1 < self.lines.len() {
            let goal = self.goal_col.unwrap_or(self.cursor_col);
            self.goal_col = Some(goal);
            self.cursor_row += 1;
            self.cursor_col = goal.min(self.current_line().len());
        }
    }

    pub fn move_left(&mut self) {
        self.goal_col = None;
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.current_line().len();
        }
    }

    pub fn move_right(&mut self) {
        self.goal_col = None;
        let line_len = self.current_line().len();
        if self.cursor_col < line_len {
            self.cursor_col += 1;
        } else if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
    }

    pub fn move_home(&mut self) {
        self.goal_col = None;
        self.cursor_col = 0;
    }

    pub fn move_end(&mut self) {
        self.goal_col = None;
        self.cursor_col = self.current_line().len();
    }

    pub fn move_page_up(&mut self, page_size: usize) {
        self.goal_col = None;
        if self.cursor_row >= page_size {
            self.cursor_row -= page_size;
        } else {
            self.cursor_row = 0;
        }
        self.cursor_col = self.cursor_col.min(self.current_line().len());
    }

    pub fn move_page_down(&mut self, page_size: usize) {
        self.goal_col = None;
        self.cursor_row = (self.cursor_row + page_size).min(self.lines.len().saturating_sub(1));
        self.cursor_col = self.cursor_col.min(self.current_line().len());
    }

    // =========================================================================
    // TEXT EDITING
    // =========================================================================

    pub fn insert_char(&mut self, c: char) {
        self.goal_col = None;
        let line_len = self.lines[self.cursor_row].len();
        if self.cursor_col > line_len {
            self.cursor_col = line_len;
        }
        self.lines[self.cursor_row].insert(self.cursor_col, c);
        self.cursor_col += 1;
        self.modified = true;
    }

    pub fn insert_newline(&mut self) {
        self.goal_col = None;
        let rest = self.lines[self.cursor_row][self.cursor_col..].to_string();
        self.lines[self.cursor_row].truncate(self.cursor_col);

        // Auto-indent: copy leading whitespace
        let indent: String = self.lines[self.cursor_row]
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect();

        self.cursor_row += 1;
        self.lines
            .insert(self.cursor_row, format!("{}{}", indent, rest));
        self.cursor_col = indent.len();
        self.modified = true;
    }

    pub fn backspace(&mut self) {
        self.goal_col = None;
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
            self.lines[self.cursor_row].remove(self.cursor_col);
            self.modified = true;
        } else if self.cursor_row > 0 {
            let current_line = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].push_str(&current_line);
            self.modified = true;
        }
    }

    pub fn delete_char(&mut self) {
        self.goal_col = None;
        let line_len = self.lines[self.cursor_row].len();
        if self.cursor_col < line_len {
            self.lines[self.cursor_row].remove(self.cursor_col);
            self.modified = true;
        } else if self.cursor_row + 1 < self.lines.len() {
            let next_line = self.lines.remove(self.cursor_row + 1);
            self.lines[self.cursor_row].push_str(&next_line);
            self.modified = true;
        }
    }

    pub fn insert_tab(&mut self) {
        self.goal_col = None;
        // Insert 4 spaces for a tab
        for _ in 0..4 {
            self.insert_char(' ');
        }
        // Only mark modified once (already done by insert_char)
    }

    // =========================================================================
    // SCROLL MANAGEMENT
    // =========================================================================

    pub fn ensure_visible(&mut self, visible_lines: usize) {
        if self.cursor_row < self.scroll_offset {
            self.scroll_offset = self.cursor_row;
        } else if self.cursor_row >= self.scroll_offset + visible_lines {
            self.scroll_offset = self.cursor_row - visible_lines + 1;
        }
    }
}

// =============================================================================
// MAIN STATE
// =============================================================================

/// Main Q-CODE state
#[derive(Debug)]
pub struct QCodeState {
    /// Current view
    pub view: QCodeView,

    /// File tree entries
    pub file_tree: Vec<FileEntry>,
    /// File tree cursor position
    pub file_tree_cursor: usize,
    /// File tree scroll offset
    pub file_tree_scroll: usize,

    /// Open editor buffers
    pub buffers: Vec<EditorBuffer>,
    /// Index of currently active buffer
    pub current_buffer: usize,

    /// Terminal output lines
    pub terminal_output: Vec<String>,
    /// Terminal scroll offset
    pub terminal_scroll: usize,

    /// Current working directory
    pub cwd: PathBuf,

    /// Status message and display ticks
    pub status_message: Option<(String, u32)>,

    /// Width of file tree panel (in columns)
    pub tree_width: u16,
}

impl Default for QCodeState {
    fn default() -> Self {
        Self::new()
    }
}

impl QCodeState {
    pub fn new() -> Self {
        Self {
            view: QCodeView::FileTree,
            file_tree: Vec::new(),
            file_tree_cursor: 0,
            file_tree_scroll: 0,
            buffers: vec![EditorBuffer::new()],
            current_buffer: 0,
            terminal_output: Vec::new(),
            terminal_scroll: 0,
            cwd: PathBuf::new(),
            status_message: None,
            tree_width: 25,
        }
    }

    pub fn set_cwd(&mut self, cwd: PathBuf) {
        self.cwd = cwd;
        self.refresh_file_tree();
    }

    pub fn refresh_file_tree(&mut self) {
        self.file_tree.clear();
        self.file_tree_cursor = 0;

        if let Ok(entries) = std::fs::read_dir(&self.cwd) {
            let mut dirs = Vec::new();
            let mut files = Vec::new();

            for entry in entries.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                // Skip hidden files
                if name.starts_with('.') {
                    continue;
                }

                let is_dir = path.is_dir();
                let file_entry = FileEntry::new(name, path, is_dir, 0);

                if is_dir {
                    dirs.push(file_entry);
                } else {
                    files.push(file_entry);
                }
            }

            // Sort alphabetically
            dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

            // Directories first, then files
            self.file_tree.extend(dirs);
            self.file_tree.extend(files);
        }
    }

    // =========================================================================
    // FILE TREE NAVIGATION
    // =========================================================================

    pub fn file_tree_up(&mut self) {
        if self.file_tree_cursor > 0 {
            self.file_tree_cursor -= 1;
        }
    }

    pub fn file_tree_down(&mut self) {
        if self.file_tree_cursor + 1 < self.file_tree.len() {
            self.file_tree_cursor += 1;
        }
    }

    pub fn selected_file(&self) -> Option<&FileEntry> {
        self.file_tree.get(self.file_tree_cursor)
    }

    pub fn open_selected_file(&mut self) -> Result<(), String> {
        let entry = match self.file_tree.get(self.file_tree_cursor) {
            Some(e) => e.clone(),
            None => return Err("No file selected".to_string()),
        };

        if entry.is_dir {
            // Navigate into directory
            self.cwd = entry.path;
            self.refresh_file_tree();
            self.status_message = Some(("Entered directory".to_string(), 30));
        } else {
            // Open file in editor
            match EditorBuffer::from_file(entry.path.clone()) {
                Ok(buffer) => {
                    // Check if file is already open
                    for (i, buf) in self.buffers.iter().enumerate() {
                        if buf.file_path == buffer.file_path {
                            self.current_buffer = i;
                            self.view = QCodeView::Editor;
                            return Ok(());
                        }
                    }

                    // Add new buffer
                    self.buffers.push(buffer);
                    self.current_buffer = self.buffers.len() - 1;
                    self.view = QCodeView::Editor;
                    self.status_message = Some((format!("Opened {}", entry.name), 30));
                }
                Err(e) => {
                    return Err(format!("Failed to open file: {}", e));
                }
            }
        }
        Ok(())
    }

    pub fn go_up_directory(&mut self) {
        if let Some(parent) = self.cwd.parent() {
            self.cwd = parent.to_path_buf();
            self.refresh_file_tree();
        }
    }

    // =========================================================================
    // BUFFER MANAGEMENT
    // =========================================================================

    pub fn current_buffer(&self) -> Option<&EditorBuffer> {
        self.buffers.get(self.current_buffer)
    }

    pub fn current_buffer_mut(&mut self) -> Option<&mut EditorBuffer> {
        self.buffers.get_mut(self.current_buffer)
    }

    pub fn next_buffer(&mut self) {
        if !self.buffers.is_empty() {
            self.current_buffer = (self.current_buffer + 1) % self.buffers.len();
        }
    }

    pub fn prev_buffer(&mut self) {
        if !self.buffers.is_empty() {
            if self.current_buffer == 0 {
                self.current_buffer = self.buffers.len() - 1;
            } else {
                self.current_buffer -= 1;
            }
        }
    }

    pub fn close_current_buffer(&mut self) {
        if self.buffers.len() > 1 {
            self.buffers.remove(self.current_buffer);
            if self.current_buffer >= self.buffers.len() {
                self.current_buffer = self.buffers.len() - 1;
            }
        } else {
            // Keep at least one buffer
            self.buffers[0] = EditorBuffer::new();
        }
    }

    pub fn save_current_buffer(&mut self) -> Result<(), String> {
        if let Some(buffer) = self.buffers.get_mut(self.current_buffer) {
            if buffer.file_path.is_some() {
                buffer.save().map_err(|e| e.to_string())?;
                self.status_message = Some(("Saved".to_string(), 30));
            } else {
                return Err("No file path set".to_string());
            }
        }
        Ok(())
    }
}
