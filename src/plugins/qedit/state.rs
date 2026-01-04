//! Q-EDIT state types

use std::path::PathBuf;

/// Q-EDIT editor modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorMode {
    /// Command mode - menu displayed at top
    #[default]
    Command,
    /// Insert mode - new characters inserted
    Insert,
    /// Overwrite mode - characters replaced
    Overwrite,
}

impl EditorMode {
    pub fn name(&self) -> &str {
        match self {
            EditorMode::Command => "Command",
            EditorMode::Insert => "Insert",
            EditorMode::Overwrite => "Overwrite",
        }
    }

    /// Toggle between insert and overwrite
    pub fn toggle_insert(&self) -> Self {
        match self {
            EditorMode::Insert => EditorMode::Overwrite,
            EditorMode::Overwrite => EditorMode::Insert,
            EditorMode::Command => EditorMode::Insert,
        }
    }
}

/// Display mode for the editor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DisplayMode {
    /// Normal ASCII text mode
    #[default]
    Ascii,
    /// Hexadecimal mode
    Hex,
}

impl DisplayMode {
    pub fn name(&self) -> &str {
        match self {
            DisplayMode::Ascii => "ASCII",
            DisplayMode::Hex => "HEX",
        }
    }
}

/// Q-EDIT menu items
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QEditMenuItem {
    Again,
    Buffer,
    Copy,
    Del,
    Edit,
    Find,
    Hex,
    Jump,
    Print,
    Quit,
    Replace,
    Set,
    Tag,
}

impl QEditMenuItem {
    pub fn all() -> Vec<Self> {
        vec![
            Self::Again,
            Self::Buffer,
            Self::Copy,
            Self::Del,
            Self::Edit,
            Self::Find,
            Self::Hex,
            Self::Jump,
            Self::Print,
            Self::Quit,
            Self::Replace,
            Self::Set,
            Self::Tag,
        ]
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Again => "Again",
            Self::Buffer => "Buffer",
            Self::Copy => "Copy",
            Self::Del => "Del",
            Self::Edit => "Edit",
            Self::Find => "Find",
            Self::Hex => "Hex",
            Self::Jump => "Jump",
            Self::Print => "Print",
            Self::Quit => "Quit",
            Self::Replace => "Replace",
            Self::Set => "Set",
            Self::Tag => "Tag",
        }
    }

    pub fn key(&self) -> char {
        match self {
            Self::Again => 'A',
            Self::Buffer => 'B',
            Self::Copy => 'C',
            Self::Del => 'D',
            Self::Edit => 'E',
            Self::Find => 'F',
            Self::Hex => 'H',
            Self::Jump => 'J',
            Self::Print => 'P',
            Self::Quit => 'Q',
            Self::Replace => 'R',
            Self::Set => 'S',
            Self::Tag => 'T',
        }
    }
}

/// Main Q-EDIT state
#[derive(Debug, Clone)]
pub struct QEditState {
    /// File path being edited (None for new file)
    pub file_path: Option<PathBuf>,
    /// File content as lines
    pub lines: Vec<String>,
    /// Current cursor line (0-indexed)
    pub cursor_line: usize,
    /// Current cursor column (0-indexed)
    pub cursor_col: usize,
    /// Scroll offset (first visible line)
    pub scroll_offset: usize,
    /// Horizontal scroll offset
    pub h_scroll_offset: usize,
    /// Current editor mode
    pub mode: EditorMode,
    /// Current display mode
    pub display_mode: DisplayMode,
    /// Selected menu item (in command mode)
    pub menu_index: usize,
    /// Whether file has been modified
    pub modified: bool,
    /// Auto-indent enabled
    pub auto_indent: bool,
    /// Tab size (2-9)
    pub tab_size: u8,
    /// Buffer for copy/paste operations
    pub buffer: Vec<String>,
    /// Last search pattern
    pub last_search: String,
    /// Markers (A, B, C, D)
    pub markers: [Option<(usize, usize)>; 4],
}

impl QEditState {
    /// Create a new empty editor state
    pub fn new() -> Self {
        Self {
            file_path: None,
            lines: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
            scroll_offset: 0,
            h_scroll_offset: 0,
            mode: EditorMode::Command,
            display_mode: DisplayMode::Ascii,
            menu_index: 0,
            modified: false,
            auto_indent: true,
            tab_size: 4,
            buffer: Vec::new(),
            last_search: String::new(),
            markers: [None; 4],
        }
    }

    /// Load a file into the editor
    pub fn load_file(path: PathBuf) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(&path)?;
        let lines: Vec<String> = content.lines().map(String::from).collect();
        let lines = if lines.is_empty() {
            vec![String::new()]
        } else {
            lines
        };

        Ok(Self {
            file_path: Some(path),
            lines,
            cursor_line: 0,
            cursor_col: 0,
            scroll_offset: 0,
            h_scroll_offset: 0,
            mode: EditorMode::Command,
            display_mode: DisplayMode::Ascii,
            menu_index: 0,
            modified: false,
            auto_indent: true,
            tab_size: 4,
            buffer: Vec::new(),
            last_search: String::new(),
            markers: [None; 4],
        })
    }

    /// Save the file
    pub fn save(&mut self) -> std::io::Result<()> {
        if let Some(ref path) = self.file_path {
            let content = self.lines.join("\n");
            std::fs::write(path, content)?;
            self.modified = false;
        }
        Ok(())
    }

    /// Get current line text
    pub fn current_line(&self) -> &str {
        self.lines
            .get(self.cursor_line)
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    /// Get current line text mutably
    pub fn current_line_mut(&mut self) -> &mut String {
        if self.cursor_line >= self.lines.len() {
            self.lines.push(String::new());
        }
        &mut self.lines[self.cursor_line]
    }

    /// Insert a character at cursor position
    pub fn insert_char(&mut self, c: char) {
        // Get line length first to avoid borrow issues
        let line_len = self.lines[self.cursor_line].len();
        if self.cursor_col > line_len {
            self.cursor_col = line_len;
        }

        let cursor_col = self.cursor_col;
        if self.mode == EditorMode::Overwrite && cursor_col < line_len {
            self.lines[self.cursor_line].remove(cursor_col);
        }

        self.lines[self.cursor_line].insert(cursor_col, c);
        self.cursor_col += 1;
        self.modified = true;
    }

    /// Delete character at cursor position
    pub fn delete_char(&mut self) {
        let line_len = self.lines[self.cursor_line].len();
        let cursor_col = self.cursor_col;
        if cursor_col < line_len {
            self.lines[self.cursor_line].remove(cursor_col);
            self.modified = true;
        } else if self.cursor_line + 1 < self.lines.len() {
            // Join with next line
            let next_line = self.lines.remove(self.cursor_line + 1);
            self.lines[self.cursor_line].push_str(&next_line);
            self.modified = true;
        }
    }

    /// Delete character before cursor (backspace)
    pub fn backspace(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
            let cursor_col = self.cursor_col;
            self.lines[self.cursor_line].remove(cursor_col);
            self.modified = true;
        } else if self.cursor_line > 0 {
            // Join with previous line
            let current_line = self.lines.remove(self.cursor_line);
            self.cursor_line -= 1;
            self.cursor_col = self.lines[self.cursor_line].len();
            self.lines[self.cursor_line].push_str(&current_line);
            self.modified = true;
        }
    }

    /// Insert a new line at cursor position
    pub fn insert_newline(&mut self) {
        let cursor_col = self.cursor_col;
        let cursor_line = self.cursor_line;
        let rest = self.lines[cursor_line][cursor_col..].to_string();
        self.lines[cursor_line].truncate(cursor_col);

        // Handle auto-indent
        let indent = if self.auto_indent {
            let spaces: String = self.lines[cursor_line]
                .chars()
                .take_while(|c| c.is_whitespace())
                .collect();
            spaces
        } else {
            String::new()
        };

        self.cursor_line += 1;
        self.lines
            .insert(self.cursor_line, format!("{}{}", indent, rest));
        self.cursor_col = indent.len();
        self.modified = true;
    }

    /// Move cursor up
    pub fn move_up(&mut self) {
        if self.cursor_line > 0 {
            self.cursor_line -= 1;
            let line_len = self.current_line().len();
            if self.cursor_col > line_len {
                self.cursor_col = line_len;
            }
        }
    }

    /// Move cursor down
    pub fn move_down(&mut self) {
        if self.cursor_line + 1 < self.lines.len() {
            self.cursor_line += 1;
            let line_len = self.current_line().len();
            if self.cursor_col > line_len {
                self.cursor_col = line_len;
            }
        }
    }

    /// Move cursor left
    pub fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_line > 0 {
            self.cursor_line -= 1;
            self.cursor_col = self.current_line().len();
        }
    }

    /// Move cursor right
    pub fn move_right(&mut self) {
        let line_len = self.current_line().len();
        if self.cursor_col < line_len {
            self.cursor_col += 1;
        } else if self.cursor_line + 1 < self.lines.len() {
            self.cursor_line += 1;
            self.cursor_col = 0;
        }
    }

    /// Move cursor to start of line
    pub fn move_home(&mut self) {
        self.cursor_col = 0;
    }

    /// Move cursor to end of line
    pub fn move_end(&mut self) {
        self.cursor_col = self.current_line().len();
    }

    /// Move cursor to top of file
    pub fn move_top(&mut self) {
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
    }

    /// Move cursor to bottom of file
    pub fn move_bottom(&mut self) {
        self.cursor_line = self.lines.len().saturating_sub(1);
        self.cursor_col = 0;
    }

    /// Page up
    pub fn page_up(&mut self, visible_lines: usize) {
        self.cursor_line = self.cursor_line.saturating_sub(visible_lines);
        self.scroll_offset = self.scroll_offset.saturating_sub(visible_lines);
    }

    /// Page down
    pub fn page_down(&mut self, visible_lines: usize) {
        let max_line = self.lines.len().saturating_sub(1);
        self.cursor_line = (self.cursor_line + visible_lines).min(max_line);
        self.scroll_offset = (self.scroll_offset + visible_lines).min(max_line);
    }

    /// Ensure cursor is visible
    pub fn ensure_visible(&mut self, visible_lines: usize) {
        if self.cursor_line < self.scroll_offset {
            self.scroll_offset = self.cursor_line;
        } else if self.cursor_line >= self.scroll_offset + visible_lines {
            self.scroll_offset = self.cursor_line - visible_lines + 1;
        }
    }

    /// Get file name for display
    pub fn display_name(&self) -> String {
        self.file_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "[New File]".to_string())
    }

    /// Get total byte count (approximate)
    pub fn byte_count(&self) -> usize {
        self.lines
            .iter()
            .map(|l| l.len() + 1)
            .sum::<usize>()
            .saturating_sub(1)
    }
}

impl Default for QEditState {
    fn default() -> Self {
        Self::new()
    }
}
