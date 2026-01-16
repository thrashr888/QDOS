//! Q-DOCS State Management
//!
//! Document state for the word processor.

use std::path::PathBuf;

// =============================================================================
// DOCUMENT MODE
// =============================================================================

/// Q-DOCS editor modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DocsMode {
    /// Normal editing mode
    #[default]
    Edit,
    /// Preview mode (rendered markdown)
    Preview,
    /// Menu bar active
    Menu,
    /// Find dialog
    Find,
    /// Replace dialog
    Replace,
    /// Save As dialog
    SaveAs,
    /// Help overlay
    Help,
}

/// Input mode within editing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    /// Normal cursor navigation
    #[default]
    Normal,
    /// Insert mode - typing inserts characters
    Insert,
    /// Overwrite mode - typing replaces characters
    Overwrite,
}

impl InputMode {
    pub fn name(&self) -> &str {
        match self {
            InputMode::Normal => "NORMAL",
            InputMode::Insert => "INSERT",
            InputMode::Overwrite => "OVERWRITE",
        }
    }

    pub fn toggle(&self) -> Self {
        match self {
            InputMode::Normal => InputMode::Insert,
            InputMode::Insert => InputMode::Overwrite,
            InputMode::Overwrite => InputMode::Insert,
        }
    }
}

// =============================================================================
// MENU SYSTEM
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MenuCategory {
    #[default]
    File,
    Edit,
    View,
    Insert,
    Format,
    Tools,
    Help,
}

impl MenuCategory {
    pub fn all() -> &'static [MenuCategory] {
        &[
            MenuCategory::File,
            MenuCategory::Edit,
            MenuCategory::View,
            MenuCategory::Insert,
            MenuCategory::Format,
            MenuCategory::Tools,
            MenuCategory::Help,
        ]
    }

    pub fn name(&self) -> &str {
        match self {
            MenuCategory::File => "File",
            MenuCategory::Edit => "Edit",
            MenuCategory::View => "View",
            MenuCategory::Insert => "Insert",
            MenuCategory::Format => "Format",
            MenuCategory::Tools => "Tools",
            MenuCategory::Help => "Help",
        }
    }

    pub fn key(&self) -> char {
        match self {
            MenuCategory::File => 'F',
            MenuCategory::Edit => 'E',
            MenuCategory::View => 'V',
            MenuCategory::Insert => 'I',
            MenuCategory::Format => 'O',
            MenuCategory::Tools => 'T',
            MenuCategory::Help => 'H',
        }
    }

    pub fn items(&self) -> &'static [MenuItem] {
        match self {
            MenuCategory::File => &[
                MenuItem::New,
                MenuItem::Open,
                MenuItem::Save,
                MenuItem::SaveAs,
                MenuItem::Export,
                MenuItem::Quit,
            ],
            MenuCategory::Edit => &[
                MenuItem::Undo,
                MenuItem::Redo,
                MenuItem::Cut,
                MenuItem::Copy,
                MenuItem::Paste,
                MenuItem::Find,
                MenuItem::Replace,
            ],
            MenuCategory::View => &[
                MenuItem::Preview,
                MenuItem::Split,
                MenuItem::WordWrap,
                MenuItem::LineNumbers,
            ],
            MenuCategory::Insert => &[
                MenuItem::Heading,
                MenuItem::Bold,
                MenuItem::Italic,
                MenuItem::Link,
                MenuItem::Image,
                MenuItem::Code,
                MenuItem::List,
                MenuItem::Table,
            ],
            MenuCategory::Format => &[
                MenuItem::Paragraph,
                MenuItem::Quote,
                MenuItem::HorizontalRule,
            ],
            MenuCategory::Tools => &[
                MenuItem::SpellCheck,
                MenuItem::WordCount,
                MenuItem::Statistics,
            ],
            MenuCategory::Help => &[MenuItem::HelpTopic, MenuItem::About],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    // File
    New,
    Open,
    Save,
    SaveAs,
    Export,
    Quit,
    // Edit
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Find,
    Replace,
    // View
    Preview,
    Split,
    WordWrap,
    LineNumbers,
    // Insert
    Heading,
    Bold,
    Italic,
    Link,
    Image,
    Code,
    List,
    Table,
    // Format
    Paragraph,
    Quote,
    HorizontalRule,
    // Tools
    SpellCheck,
    WordCount,
    Statistics,
    // Help
    HelpTopic,
    About,
}

impl MenuItem {
    pub fn name(&self) -> &str {
        match self {
            MenuItem::New => "New",
            MenuItem::Open => "Open",
            MenuItem::Save => "Save",
            MenuItem::SaveAs => "Save As...",
            MenuItem::Export => "Export...",
            MenuItem::Quit => "Quit",
            MenuItem::Undo => "Undo",
            MenuItem::Redo => "Redo",
            MenuItem::Cut => "Cut",
            MenuItem::Copy => "Copy",
            MenuItem::Paste => "Paste",
            MenuItem::Find => "Find...",
            MenuItem::Replace => "Replace...",
            MenuItem::Preview => "Preview Mode",
            MenuItem::Split => "Split View",
            MenuItem::WordWrap => "Word Wrap",
            MenuItem::LineNumbers => "Line Numbers",
            MenuItem::Heading => "Heading",
            MenuItem::Bold => "Bold",
            MenuItem::Italic => "Italic",
            MenuItem::Link => "Link",
            MenuItem::Image => "Image",
            MenuItem::Code => "Code Block",
            MenuItem::List => "List",
            MenuItem::Table => "Table",
            MenuItem::Paragraph => "Paragraph",
            MenuItem::Quote => "Blockquote",
            MenuItem::HorizontalRule => "Horizontal Rule",
            MenuItem::SpellCheck => "Spell Check",
            MenuItem::WordCount => "Word Count",
            MenuItem::Statistics => "Statistics",
            MenuItem::HelpTopic => "Help Topics",
            MenuItem::About => "About Q-DOCS",
        }
    }

    pub fn shortcut(&self) -> Option<&str> {
        match self {
            MenuItem::New => Some("Ctrl+N"),
            MenuItem::Open => Some("Ctrl+O"),
            MenuItem::Save => Some("Ctrl+S"),
            MenuItem::Quit => Some("Esc"),
            MenuItem::Undo => Some("Ctrl+Z"),
            MenuItem::Cut => Some("Ctrl+X"),
            MenuItem::Copy => Some("Ctrl+C"),
            MenuItem::Paste => Some("Ctrl+V"),
            MenuItem::Find => Some("Ctrl+F"),
            MenuItem::Replace => Some("Ctrl+H"),
            MenuItem::Preview => Some("F9"),
            MenuItem::Bold => Some("Ctrl+B"),
            MenuItem::Italic => Some("Ctrl+I"),
            _ => None,
        }
    }
}

// =============================================================================
// DOCUMENT STATE
// =============================================================================

/// Main Q-DOCS state
#[derive(Debug, Clone)]
pub struct DocsState {
    // File info
    pub file_path: Option<PathBuf>,
    pub modified: bool,

    // Content
    pub lines: Vec<String>,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub scroll_offset: usize,
    pub h_scroll_offset: usize,

    // Mode
    pub mode: DocsMode,
    pub input_mode: InputMode,

    // Menu
    pub menu_category: usize,
    pub menu_item: usize,

    // View settings
    pub show_line_numbers: bool,
    pub word_wrap: bool,
    pub preview_scroll: usize,

    // Selection
    pub selection_start: Option<(usize, usize)>,
    pub selection_end: Option<(usize, usize)>,

    // Clipboard
    pub clipboard: String,

    // Undo/Redo
    pub undo_stack: Vec<UndoOp>,
    pub redo_stack: Vec<UndoOp>,

    // Find/Replace
    pub find_query: String,
    pub replace_text: String,
    pub find_results: Vec<(usize, usize)>,
    pub find_index: usize,

    // Save As
    pub save_as_input: String,
    pub save_as_cursor: usize,

    // Status message
    pub status_message: Option<(String, u32)>,
}

#[derive(Debug, Clone)]
pub struct UndoOp {
    pub lines: Vec<String>,
    pub cursor_line: usize,
    pub cursor_col: usize,
}

impl Default for DocsState {
    fn default() -> Self {
        Self::new()
    }
}

impl DocsState {
    pub fn new() -> Self {
        Self {
            file_path: None,
            modified: false,
            lines: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
            scroll_offset: 0,
            h_scroll_offset: 0,
            mode: DocsMode::Edit,
            input_mode: InputMode::Normal,
            menu_category: 0,
            menu_item: 0,
            show_line_numbers: true,
            word_wrap: true,
            preview_scroll: 0,
            selection_start: None,
            selection_end: None,
            clipboard: String::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            find_query: String::new(),
            replace_text: String::new(),
            find_results: Vec::new(),
            find_index: 0,
            save_as_input: String::new(),
            save_as_cursor: 0,
            status_message: None,
        }
    }

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
            ..Self::new()
        })
    }

    // =========================================================================
    // FILE OPERATIONS
    // =========================================================================

    pub fn save(&mut self) -> std::io::Result<()> {
        if let Some(ref path) = self.file_path {
            let content = self.lines.join("\n");
            std::fs::write(path, content)?;
            self.modified = false;
        }
        Ok(())
    }

    pub fn display_name(&self) -> String {
        self.file_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "[New Document]".to_string())
    }

    // =========================================================================
    // CURSOR OPERATIONS
    // =========================================================================

    pub fn current_line(&self) -> &str {
        self.lines
            .get(self.cursor_line)
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    pub fn move_up(&mut self) {
        if self.cursor_line > 0 {
            self.cursor_line -= 1;
            self.clamp_cursor_col();
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor_line + 1 < self.lines.len() {
            self.cursor_line += 1;
            self.clamp_cursor_col();
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_line > 0 {
            self.cursor_line -= 1;
            self.cursor_col = self.current_line().len();
        }
    }

    pub fn move_right(&mut self) {
        let line_len = self.current_line().len();
        if self.cursor_col < line_len {
            self.cursor_col += 1;
        } else if self.cursor_line + 1 < self.lines.len() {
            self.cursor_line += 1;
            self.cursor_col = 0;
        }
    }

    pub fn move_home(&mut self) {
        self.cursor_col = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor_col = self.current_line().len();
    }

    pub fn move_top(&mut self) {
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
    }

    pub fn move_bottom(&mut self) {
        self.cursor_line = self.lines.len().saturating_sub(1);
        self.cursor_col = 0;
    }

    fn clamp_cursor_col(&mut self) {
        let line_len = self.current_line().len();
        if self.cursor_col > line_len {
            self.cursor_col = line_len;
        }
    }

    pub fn ensure_visible(&mut self, visible_lines: usize) {
        if self.cursor_line < self.scroll_offset {
            self.scroll_offset = self.cursor_line;
        } else if self.cursor_line >= self.scroll_offset + visible_lines {
            self.scroll_offset = self.cursor_line - visible_lines + 1;
        }
    }

    // =========================================================================
    // TEXT EDITING
    // =========================================================================

    pub fn save_undo(&mut self) {
        self.undo_stack.push(UndoOp {
            lines: self.lines.clone(),
            cursor_line: self.cursor_line,
            cursor_col: self.cursor_col,
        });
        self.redo_stack.clear();
        // Limit undo stack size
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self) {
        if let Some(op) = self.undo_stack.pop() {
            self.redo_stack.push(UndoOp {
                lines: self.lines.clone(),
                cursor_line: self.cursor_line,
                cursor_col: self.cursor_col,
            });
            self.lines = op.lines;
            self.cursor_line = op.cursor_line;
            self.cursor_col = op.cursor_col;
            self.modified = true;
        }
    }

    pub fn redo(&mut self) {
        if let Some(op) = self.redo_stack.pop() {
            self.undo_stack.push(UndoOp {
                lines: self.lines.clone(),
                cursor_line: self.cursor_line,
                cursor_col: self.cursor_col,
            });
            self.lines = op.lines;
            self.cursor_line = op.cursor_line;
            self.cursor_col = op.cursor_col;
            self.modified = true;
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.save_undo();
        let line_len = self.lines[self.cursor_line].len();
        if self.cursor_col > line_len {
            self.cursor_col = line_len;
        }

        if self.input_mode == InputMode::Overwrite && self.cursor_col < line_len {
            self.lines[self.cursor_line].remove(self.cursor_col);
        }

        self.lines[self.cursor_line].insert(self.cursor_col, c);
        self.cursor_col += 1;
        self.modified = true;
    }

    pub fn insert_newline(&mut self) {
        self.save_undo();
        let rest = self.lines[self.cursor_line][self.cursor_col..].to_string();
        self.lines[self.cursor_line].truncate(self.cursor_col);

        // Auto-indent
        let indent: String = self.lines[self.cursor_line]
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect();

        self.cursor_line += 1;
        self.lines
            .insert(self.cursor_line, format!("{}{}", indent, rest));
        self.cursor_col = indent.len();
        self.modified = true;
    }

    pub fn backspace(&mut self) {
        self.save_undo();
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
            self.lines[self.cursor_line].remove(self.cursor_col);
            self.modified = true;
        } else if self.cursor_line > 0 {
            let current_line = self.lines.remove(self.cursor_line);
            self.cursor_line -= 1;
            self.cursor_col = self.lines[self.cursor_line].len();
            self.lines[self.cursor_line].push_str(&current_line);
            self.modified = true;
        }
    }

    pub fn delete_char(&mut self) {
        self.save_undo();
        let line_len = self.lines[self.cursor_line].len();
        if self.cursor_col < line_len {
            self.lines[self.cursor_line].remove(self.cursor_col);
            self.modified = true;
        } else if self.cursor_line + 1 < self.lines.len() {
            let next_line = self.lines.remove(self.cursor_line + 1);
            self.lines[self.cursor_line].push_str(&next_line);
            self.modified = true;
        }
    }

    // =========================================================================
    // MARKDOWN FORMATTING
    // =========================================================================

    pub fn insert_heading(&mut self, level: usize) {
        self.save_undo();
        let prefix = "#".repeat(level.min(6)) + " ";
        self.lines[self.cursor_line].insert_str(0, &prefix);
        self.cursor_col = prefix.len();
        self.modified = true;
    }

    pub fn toggle_bold(&mut self) {
        self.wrap_selection("**", "**");
    }

    pub fn toggle_italic(&mut self) {
        self.wrap_selection("*", "*");
    }

    pub fn insert_link(&mut self) {
        self.save_undo();
        let text = "[link text](url)";
        self.lines[self.cursor_line].insert_str(self.cursor_col, text);
        self.cursor_col += 1; // Position at start of "link text"
        self.modified = true;
    }

    pub fn insert_code_block(&mut self) {
        self.save_undo();
        let current_col = self.cursor_col;
        self.lines[self.cursor_line].truncate(current_col);
        self.lines.insert(self.cursor_line + 1, "```".to_string());
        self.lines.insert(self.cursor_line + 2, String::new());
        self.lines.insert(self.cursor_line + 3, "```".to_string());
        self.cursor_line += 2;
        self.cursor_col = 0;
        self.modified = true;
    }

    pub fn insert_horizontal_rule(&mut self) {
        self.save_undo();
        self.lines.insert(self.cursor_line + 1, "---".to_string());
        self.lines.insert(self.cursor_line + 2, String::new());
        self.cursor_line += 2;
        self.cursor_col = 0;
        self.modified = true;
    }

    fn wrap_selection(&mut self, prefix: &str, suffix: &str) {
        self.save_undo();
        // For now, just insert at cursor position
        self.lines[self.cursor_line].insert_str(self.cursor_col, prefix);
        self.cursor_col += prefix.len();
        self.lines[self.cursor_line].insert_str(self.cursor_col, suffix);
        self.modified = true;
    }

    // =========================================================================
    // STATISTICS
    // =========================================================================

    pub fn word_count(&self) -> usize {
        self.lines
            .iter()
            .map(|line| line.split_whitespace().count())
            .sum()
    }

    pub fn char_count(&self) -> usize {
        self.lines
            .iter()
            .map(|line| line.chars().count())
            .sum::<usize>()
            + self.lines.len().saturating_sub(1) // newlines
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn page_count(&self) -> usize {
        // Approximate: ~250 words per page
        self.word_count().div_ceil(250)
    }
}
