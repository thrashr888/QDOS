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
    /// Export format selection
    Export,
}

/// Export format options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExportFormat {
    #[default]
    Html,
    Pdf,
    PlainText,
}

impl ExportFormat {
    pub fn all() -> &'static [ExportFormat] {
        &[
            ExportFormat::Html,
            ExportFormat::Pdf,
            ExportFormat::PlainText,
        ]
    }

    pub fn name(&self) -> &str {
        match self {
            ExportFormat::Html => "HTML (.html)",
            ExportFormat::Pdf => "PDF (.pdf)",
            ExportFormat::PlainText => "Plain Text (.txt)",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            ExportFormat::Html => "Native export",
            ExportFormat::Pdf => "Requires pandoc",
            ExportFormat::PlainText => "Strip formatting",
        }
    }

    pub fn extension(&self) -> &str {
        match self {
            ExportFormat::Html => "html",
            ExportFormat::Pdf => "pdf",
            ExportFormat::PlainText => "txt",
        }
    }
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

    // Selection (Phase 1)
    pub selection_start: Option<(usize, usize)>,
    pub selection_end: Option<(usize, usize)>,
    pub selecting: bool,

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

    // Goal column for vertical navigation (remembers column when passing through short lines)
    pub goal_col: Option<usize>,

    // Ruler and Margins (Phase 2)
    pub left_margin: usize,
    pub right_margin: usize,
    pub tab_stops: Vec<usize>,
    pub show_ruler: bool,

    // Page View (Phase 3)
    pub page_view_enabled: bool,
    pub lines_per_page: usize,

    // Export (Phase 4)
    pub export_format: ExportFormat,
    pub export_input: String,
    pub export_cursor: usize,
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
            selecting: false,
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
            goal_col: None,
            // Ruler and Margins (Phase 2)
            left_margin: 0,
            right_margin: 80,
            tab_stops: vec![
                4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44, 48, 52, 56, 60, 64, 68, 72, 76,
            ],
            show_ruler: true,
            // Page View (Phase 3)
            page_view_enabled: false,
            lines_per_page: 60,
            // Export (Phase 4)
            export_format: ExportFormat::Html,
            export_input: String::new(),
            export_cursor: 0,
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
            // Remember the goal column (use current col if none set)
            let goal = self.goal_col.unwrap_or(self.cursor_col);
            self.goal_col = Some(goal);

            self.cursor_line -= 1;
            // Try to reach goal column, clamp to line length
            self.cursor_col = goal.min(self.current_line().len());
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor_line + 1 < self.lines.len() {
            // Remember the goal column (use current col if none set)
            let goal = self.goal_col.unwrap_or(self.cursor_col);
            self.goal_col = Some(goal);

            self.cursor_line += 1;
            // Try to reach goal column, clamp to line length
            self.cursor_col = goal.min(self.current_line().len());
        }
    }

    pub fn move_left(&mut self) {
        self.goal_col = None; // Clear goal column on horizontal movement
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_line > 0 {
            self.cursor_line -= 1;
            self.cursor_col = self.current_line().len();
        }
    }

    pub fn move_right(&mut self) {
        self.goal_col = None; // Clear goal column on horizontal movement
        let line_len = self.current_line().len();
        if self.cursor_col < line_len {
            self.cursor_col += 1;
        } else if self.cursor_line + 1 < self.lines.len() {
            self.cursor_line += 1;
            self.cursor_col = 0;
        }
    }

    pub fn move_home(&mut self) {
        self.goal_col = None; // Clear goal column
        self.cursor_col = 0;
    }

    pub fn move_end(&mut self) {
        self.goal_col = None; // Clear goal column
        self.cursor_col = self.current_line().len();
    }

    pub fn move_top(&mut self) {
        self.goal_col = None; // Clear goal column
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
    }

    pub fn move_bottom(&mut self) {
        self.goal_col = None; // Clear goal column
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
        self.goal_col = None; // Clear goal column on editing
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

    // =========================================================================
    // SELECTION (Phase 1)
    // =========================================================================

    /// Check if there is an active selection
    pub fn has_selection(&self) -> bool {
        self.selection_start.is_some() && self.selection_end.is_some()
    }

    /// Get selection bounds in normalized order (start always before end)
    pub fn selection_bounds(&self) -> Option<((usize, usize), (usize, usize))> {
        match (self.selection_start, self.selection_end) {
            (Some(start), Some(end)) => {
                if start.0 < end.0 || (start.0 == end.0 && start.1 <= end.1) {
                    Some((start, end))
                } else {
                    Some((end, start))
                }
            }
            _ => None,
        }
    }

    /// Get selected text as a String
    pub fn selected_text(&self) -> Option<String> {
        let ((start_line, start_col), (end_line, end_col)) = self.selection_bounds()?;

        if start_line == end_line {
            // Single line selection
            let line = self.lines.get(start_line)?;
            Some(line[start_col..end_col.min(line.len())].to_string())
        } else {
            // Multi-line selection
            let mut result = String::new();
            for (idx, line) in self.lines.iter().enumerate() {
                if idx == start_line {
                    result.push_str(&line[start_col..]);
                    result.push('\n');
                } else if idx == end_line {
                    result.push_str(&line[..end_col.min(line.len())]);
                } else if idx > start_line && idx < end_line {
                    result.push_str(line);
                    result.push('\n');
                }
            }
            Some(result)
        }
    }

    /// Delete selected text
    pub fn delete_selection(&mut self) -> bool {
        if let Some(((start_line, start_col), (end_line, end_col))) = self.selection_bounds() {
            self.save_undo();

            if start_line == end_line {
                // Single line - remove substring
                let line = &mut self.lines[start_line];
                let end_col = end_col.min(line.len());
                line.drain(start_col..end_col);
            } else {
                // Multi-line - join start and end, remove middle
                let end_col = end_col.min(self.lines[end_line].len());
                let end_text = self.lines[end_line][end_col..].to_string();
                self.lines[start_line].truncate(start_col);
                self.lines[start_line].push_str(&end_text);

                // Remove lines between start and end (inclusive of end)
                for _ in start_line + 1..=end_line {
                    if start_line + 1 < self.lines.len() {
                        self.lines.remove(start_line + 1);
                    }
                }
            }

            self.cursor_line = start_line;
            self.cursor_col = start_col;
            self.clear_selection();
            self.modified = true;
            true
        } else {
            false
        }
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.selection_start = None;
        self.selection_end = None;
        self.selecting = false;
    }

    /// Start or extend selection from current cursor position
    pub fn extend_selection(&mut self) {
        if self.selection_start.is_none() {
            self.selection_start = Some((self.cursor_line, self.cursor_col));
        }
        self.selection_end = Some((self.cursor_line, self.cursor_col));
    }

    /// Select entire document
    pub fn select_all(&mut self) {
        self.selection_start = Some((0, 0));
        let last_line = self.lines.len().saturating_sub(1);
        let last_col = self.lines.last().map(|l| l.len()).unwrap_or(0);
        self.selection_end = Some((last_line, last_col));
    }

    // =========================================================================
    // RULER AND MARGINS (Phase 2)
    // =========================================================================

    /// Get the next tab stop position from current column
    pub fn next_tab_stop(&self, col: usize) -> usize {
        for &stop in &self.tab_stops {
            if stop > col {
                return stop;
            }
        }
        // Default: round up to next multiple of 4
        ((col / 4) + 1) * 4
    }

    /// Insert spaces to reach next tab stop
    pub fn insert_tab(&mut self) {
        let target = self.next_tab_stop(self.cursor_col);
        let spaces = target - self.cursor_col;
        self.save_undo();
        for _ in 0..spaces {
            // Insert without saving undo again
            let line_len = self.lines[self.cursor_line].len();
            if self.cursor_col > line_len {
                self.cursor_col = line_len;
            }
            self.lines[self.cursor_line].insert(self.cursor_col, ' ');
            self.cursor_col += 1;
        }
        self.modified = true;
    }

    // =========================================================================
    // PAGE VIEW (Phase 3)
    // =========================================================================

    /// Get total page count based on lines_per_page
    pub fn total_pages(&self) -> usize {
        self.lines.len().div_ceil(self.lines_per_page)
    }

    /// Get the page number for a given line (0-indexed)
    pub fn line_to_page(&self, line: usize) -> usize {
        line / self.lines_per_page
    }

    /// Move cursor to start of a specific page
    pub fn goto_page(&mut self, page: usize) {
        let target_line = page * self.lines_per_page;
        self.cursor_line = target_line.min(self.lines.len().saturating_sub(1));
        self.cursor_col = 0;
        self.scroll_offset = target_line;
    }

    /// Move to next page
    pub fn next_page(&mut self) {
        let current = self.line_to_page(self.cursor_line);
        if current + 1 < self.total_pages() {
            self.goto_page(current + 1);
        }
    }

    /// Move to previous page
    pub fn prev_page(&mut self) {
        let current = self.line_to_page(self.cursor_line);
        if current > 0 {
            self.goto_page(current - 1);
        }
    }
}
