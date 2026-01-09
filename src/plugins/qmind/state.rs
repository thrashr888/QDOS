//! Q-MIND plugin state types
//!
//! State structures for the AI Intelligence Layer.

use crate::plugins::qmind::command::ParsedCommand;
use std::path::PathBuf;

/// Type of file operation for dry run preview
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DryRunOpType {
    /// Create a new file
    Create,
    /// Modify an existing file
    Modify,
    /// Delete a file (DANGEROUS)
    Delete,
    /// Rename/move a file
    Rename,
    /// Copy a file
    Copy,
    /// Execute a command
    Execute,
}

impl DryRunOpType {
    pub fn label(&self) -> &'static str {
        match self {
            DryRunOpType::Create => "CREATE",
            DryRunOpType::Modify => "MODIFY",
            DryRunOpType::Delete => "DELETE",
            DryRunOpType::Rename => "RENAME",
            DryRunOpType::Copy => "COPY",
            DryRunOpType::Execute => "EXEC",
        }
    }

    pub fn is_destructive(&self) -> bool {
        matches!(self, DryRunOpType::Delete | DryRunOpType::Modify)
    }
}

/// A single planned operation in the dry run preview
#[derive(Debug, Clone)]
pub struct DryRunOperation {
    /// Type of operation
    pub op_type: DryRunOpType,
    /// Primary path affected
    pub path: PathBuf,
    /// Secondary path (for rename/copy destination)
    pub dest_path: Option<PathBuf>,
    /// Description of the change
    pub description: String,
    /// Preview of content changes (for diffs)
    pub preview: Option<String>,
}

impl DryRunOperation {
    pub fn new(op_type: DryRunOpType, path: PathBuf, description: impl Into<String>) -> Self {
        Self {
            op_type,
            path,
            dest_path: None,
            description: description.into(),
            preview: None,
        }
    }

    pub fn with_dest(mut self, dest: PathBuf) -> Self {
        self.dest_path = Some(dest);
        self
    }

    pub fn with_preview(mut self, preview: impl Into<String>) -> Self {
        self.preview = Some(preview.into());
        self
    }
}

/// State for the dry run confirmation view
#[derive(Debug, Clone, Default)]
pub struct DryRunState {
    /// List of operations to preview
    pub operations: Vec<DryRunOperation>,
    /// Currently selected operation index
    pub selected: usize,
    /// Scroll offset for long lists
    pub scroll_offset: usize,
    /// Whether user has confirmed (Y pressed)
    pub confirmed: bool,
    /// Whether user has cancelled (N/Esc pressed)
    pub cancelled: bool,
    /// Source description (e.g., "AI command: organize files")
    pub source: String,
}

impl DryRunState {
    pub fn new(source: impl Into<String>, operations: Vec<DryRunOperation>) -> Self {
        Self {
            operations,
            selected: 0,
            scroll_offset: 0,
            confirmed: false,
            cancelled: false,
            source: source.into(),
        }
    }

    /// Count destructive operations
    pub fn destructive_count(&self) -> usize {
        self.operations
            .iter()
            .filter(|op| op.op_type.is_destructive())
            .count()
    }

    /// Check if any operations are destructive
    pub fn has_destructive(&self) -> bool {
        self.operations.iter().any(|op| op.op_type.is_destructive())
    }

    /// Check if any operations are deletions
    pub fn has_deletions(&self) -> bool {
        self.operations
            .iter()
            .any(|op| op.op_type == DryRunOpType::Delete)
    }

    /// Move selection up
    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            if self.selected < self.scroll_offset {
                self.scroll_offset = self.selected;
            }
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if self.selected < self.operations.len().saturating_sub(1) {
            self.selected += 1;
        }
    }

    /// Adjust scroll for visible height
    pub fn adjust_scroll(&mut self, visible_height: usize) {
        if self.selected >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected.saturating_sub(visible_height - 1);
        }
    }
}

/// Text input state for command palette and search
#[derive(Debug, Clone, Default)]
pub struct TextInputState {
    /// User input text
    pub input: String,
    /// Cursor position in input
    pub cursor: usize,
}

impl TextInputState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset to initial state
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Handle text input
    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor, c);
        self.cursor += 1;
    }

    /// Handle backspace
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.input.remove(self.cursor);
        }
    }

    /// Handle delete
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

    /// Check if input is empty
    pub fn is_empty(&self) -> bool {
        self.input.is_empty()
    }

    /// Get the input text
    pub fn text(&self) -> &str {
        &self.input
    }
}

/// Q-MIND modal view states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QMindView {
    #[default]
    Overview,
    /// Natural language command palette
    CommandPalette,
    /// Semantic file search
    SemanticSearch,
    /// Index status and management
    IndexStatus,
    /// File summary view
    FileSummary,
    /// Dry run confirmation for destructive operations
    DryRun,
}

impl QMindView {
    pub fn title(&self) -> &'static str {
        match self {
            QMindView::Overview => "Q-MIND Intelligence Layer",
            QMindView::CommandPalette => "Q-MIND Command",
            QMindView::SemanticSearch => "Semantic Search",
            QMindView::IndexStatus => "Index Status",
            QMindView::FileSummary => "File Summary",
            QMindView::DryRun => "Confirm Operation",
        }
    }
}

/// Search result from semantic search
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// File path
    pub path: PathBuf,
    /// Similarity score (0.0 to 1.0)
    pub score: f32,
    /// Brief description
    pub summary: Option<String>,
}

/// Q-MIND plugin state
#[derive(Debug, Clone, Default)]
pub struct QMindState {
    /// Current view
    pub view: QMindView,
    /// Whether AI API is available
    pub api_available: bool,
    /// Number of indexed files
    pub indexed_count: usize,
    /// Command palette text input
    pub command_input: TextInputState,
    /// Search text input
    pub search_input: TextInputState,
    /// Search results
    pub search_results: Vec<SearchResult>,
    /// Selected search result index
    pub search_selected: usize,
    /// Current file summary (if viewing)
    pub current_summary: Option<String>,
    /// Last parsed command (for display/execution)
    pub last_parsed_command: Option<ParsedCommand>,
    /// Found files from Find/List operations
    pub found_files: Vec<std::path::PathBuf>,
    /// Dry run state for operation confirmation
    pub dry_run: Option<DryRunState>,
    /// Error message (if any)
    pub error: Option<String>,
}

impl QMindState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if API key is configured
    pub fn check_api_availability(&mut self) {
        // Check for OpenAI or Anthropic API keys
        self.api_available = std::env::var("OPENAI_API_KEY").is_ok()
            || std::env::var("ANTHROPIC_API_KEY").is_ok();
    }

    /// Set error message
    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.error = Some(msg.into());
    }

    /// Clear error
    pub fn clear_error(&mut self) {
        self.error = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dry_run_op_type_labels() {
        assert_eq!(DryRunOpType::Create.label(), "CREATE");
        assert_eq!(DryRunOpType::Modify.label(), "MODIFY");
        assert_eq!(DryRunOpType::Delete.label(), "DELETE");
        assert_eq!(DryRunOpType::Rename.label(), "RENAME");
        assert_eq!(DryRunOpType::Copy.label(), "COPY");
        assert_eq!(DryRunOpType::Execute.label(), "EXEC");
    }

    #[test]
    fn test_dry_run_op_type_destructive() {
        assert!(!DryRunOpType::Create.is_destructive());
        assert!(DryRunOpType::Modify.is_destructive());
        assert!(DryRunOpType::Delete.is_destructive());
        assert!(!DryRunOpType::Rename.is_destructive());
        assert!(!DryRunOpType::Copy.is_destructive());
        assert!(!DryRunOpType::Execute.is_destructive());
    }

    #[test]
    fn test_dry_run_operation_builder() {
        let op = DryRunOperation::new(
            DryRunOpType::Rename,
            PathBuf::from("/old/path.txt"),
            "Rename file",
        )
        .with_dest(PathBuf::from("/new/path.txt"))
        .with_preview("File will be moved");

        assert_eq!(op.op_type, DryRunOpType::Rename);
        assert_eq!(op.path, PathBuf::from("/old/path.txt"));
        assert_eq!(op.dest_path, Some(PathBuf::from("/new/path.txt")));
        assert_eq!(op.description, "Rename file");
        assert_eq!(op.preview, Some("File will be moved".to_string()));
    }

    #[test]
    fn test_dry_run_state_destructive_counts() {
        let ops = vec![
            DryRunOperation::new(DryRunOpType::Create, PathBuf::from("/a"), "Create file"),
            DryRunOperation::new(DryRunOpType::Delete, PathBuf::from("/b"), "Delete file"),
            DryRunOperation::new(DryRunOpType::Modify, PathBuf::from("/c"), "Modify file"),
            DryRunOperation::new(DryRunOpType::Copy, PathBuf::from("/d"), "Copy file"),
        ];

        let state = DryRunState::new("Test operation", ops);

        assert_eq!(state.destructive_count(), 2); // Delete + Modify
        assert!(state.has_destructive());
        assert!(state.has_deletions());
    }

    #[test]
    fn test_text_input_state() {
        let mut input = TextInputState::new();
        assert!(input.is_empty());

        input.insert_char('h');
        input.insert_char('i');
        assert_eq!(input.text(), "hi");
        assert_eq!(input.cursor, 2);

        input.cursor_left();
        assert_eq!(input.cursor, 1);

        input.insert_char('!');
        assert_eq!(input.text(), "h!i");

        input.backspace();
        assert_eq!(input.text(), "hi");

        input.reset();
        assert!(input.is_empty());
    }

    #[test]
    fn test_qmind_view_titles() {
        assert_eq!(QMindView::Overview.title(), "Q-MIND Intelligence Layer");
        assert_eq!(QMindView::CommandPalette.title(), "Q-MIND Command");
        assert_eq!(QMindView::SemanticSearch.title(), "Semantic Search");
    }
}
