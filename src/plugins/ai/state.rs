//! AI Coding Agents plugin state types
//!
//! State structures for monitoring AI coding agent CLI tools.

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

/// Command palette parsing status
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CommandPaletteStatus {
    /// Waiting for input
    #[default]
    Ready,
    /// Parsing the command (calling LLM)
    Parsing,
    /// Command parsed successfully
    Parsed,
    /// Parse error occurred
    Error(String),
}

/// State for the natural language command palette
#[derive(Debug, Clone, Default)]
pub struct CommandPaletteState {
    /// User input text
    pub input: String,
    /// Cursor position in input
    pub cursor: usize,
    /// Current status
    pub status: CommandPaletteStatus,
    /// Parsed command result (if successful)
    pub parsed_action: Option<String>,
    /// Parsed targets
    pub parsed_targets: Vec<String>,
    /// Parsed destination
    pub parsed_dest: Option<String>,
    /// Explanation of parsed command
    pub explanation: String,
    /// Confidence score
    pub confidence: f32,
    /// Error message if failed
    pub error: Option<String>,
}

impl CommandPaletteState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset to initial state
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Set error state
    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.status = CommandPaletteStatus::Error(msg.into());
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

/// Supported AI CLI providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AIProvider {
    #[default]
    Claude,
    Codex,
    Gemini,
    Cursor,
    Copilot,
}

impl AIProvider {
    pub const ALL: [AIProvider; 5] = [
        AIProvider::Claude,
        AIProvider::Codex,
        AIProvider::Gemini,
        AIProvider::Cursor,
        AIProvider::Copilot,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            AIProvider::Claude => "Claude Code",
            AIProvider::Codex => "OpenAI Codex",
            AIProvider::Gemini => "Gemini CLI",
            AIProvider::Cursor => "Cursor",
            AIProvider::Copilot => "GitHub Copilot",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            AIProvider::Claude => "Claude",
            AIProvider::Codex => "Codex",
            AIProvider::Gemini => "Gemini",
            AIProvider::Cursor => "Cursor",
            AIProvider::Copilot => "Copilot",
        }
    }

    pub fn config_dir(&self) -> Option<PathBuf> {
        let home = dirs::home_dir()?;
        Some(match self {
            AIProvider::Claude => home.join(".claude"),
            AIProvider::Codex => home.join(".codex"),
            AIProvider::Gemini => home.join(".gemini"),
            AIProvider::Cursor => home.join(".cursor"),
            AIProvider::Copilot => home, // Copilot uses gh CLI auth
        })
    }
}

/// Modal view states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AIView {
    #[default]
    Overview,
    Claude,
    Codex,
    Gemini,
    Cursor,
    Copilot,
    /// Dry run confirmation view for AI operations
    DryRun,
    /// Natural language command palette
    CommandPalette,
}

impl AIView {
    pub fn title(&self) -> &'static str {
        match self {
            AIView::Overview => "AI Coding Agents",
            AIView::Claude => "Claude Code Status",
            AIView::Codex => "OpenAI Codex Status",
            AIView::Gemini => "Gemini CLI Status",
            AIView::Cursor => "Cursor Status",
            AIView::Copilot => "GitHub Copilot Status",
            AIView::DryRun => "AI Operation Preview",
            AIView::CommandPalette => "Q-MIND Command",
        }
    }
}

/// Menu items for the AI modal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AIMenuItem {
    Claude,
    Codex,
    Gemini,
    Cursor,
    Copilot,
}

impl AIMenuItem {
    pub const ALL: [AIMenuItem; 5] = [
        AIMenuItem::Claude,
        AIMenuItem::Codex,
        AIMenuItem::Gemini,
        AIMenuItem::Cursor,
        AIMenuItem::Copilot,
    ];

    pub fn key(&self) -> char {
        match self {
            AIMenuItem::Claude => 'C',
            AIMenuItem::Codex => 'X',
            AIMenuItem::Gemini => 'G',
            AIMenuItem::Cursor => 'U',
            AIMenuItem::Copilot => 'P',
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            AIMenuItem::Claude => "Claude Code",
            AIMenuItem::Codex => "Codex",
            AIMenuItem::Gemini => "Gemini",
            AIMenuItem::Cursor => "Cursor",
            AIMenuItem::Copilot => "Copilot",
        }
    }
}

/// Claude Code daily activity stats
#[derive(Debug, Clone, Default)]
pub struct ClaudeDailyStats {
    pub date: String,
    pub message_count: u64,
    pub session_count: u64,
    pub tool_call_count: u64,
}

/// Claude token usage from session JSONL files
#[derive(Debug, Clone, Default)]
pub struct ClaudeTokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub total_cost_usd: f64,
}

/// Claude Code status
#[derive(Debug, Clone, Default)]
pub struct ClaudeStatus {
    pub available: bool,
    pub today: Option<ClaudeDailyStats>,
    pub recent_days: Vec<ClaudeDailyStats>,
    pub last_computed: Option<String>,
    /// Token usage from session logs (last 30 days)
    pub token_usage: ClaudeTokenUsage,
    /// Number of session files found
    pub session_count: usize,
}

/// Codex token usage from session JSONL files
#[derive(Debug, Clone, Default)]
pub struct CodexTokenUsage {
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
    pub total_cost_usd: f64,
}

/// Codex CLI status
#[derive(Debug, Clone, Default)]
pub struct CodexStatus {
    pub available: bool,
    pub model: Option<String>,
    pub reasoning_effort: Option<String>,
    pub trusted_projects: Vec<String>,
    pub latest_version: Option<String>,
    pub last_checked: Option<String>,
    /// Token usage from session logs
    pub token_usage: CodexTokenUsage,
    /// Number of session files found
    pub session_count: usize,
}

/// Gemini CLI status
#[derive(Debug, Clone, Default)]
pub struct GeminiStatus {
    pub available: bool,
    pub auth_type: Option<String>,
    pub preferred_editor: Option<String>,
    pub theme: Option<String>,
    pub preview_features: bool,
}

/// Cursor IDE status
#[derive(Debug, Clone, Default)]
pub struct CursorStatus {
    pub available: bool,
    pub model: Option<String>,
    pub vim_mode: bool,
    /// Total AI code generations tracked
    pub code_generations: u64,
    /// Generations by source (composer, tab, etc.)
    pub generations_by_source: Vec<(String, u64)>,
}

/// GitHub Copilot status
#[derive(Debug, Clone, Default)]
pub struct CopilotStatus {
    pub available: bool,
    /// GitHub username if authenticated
    pub github_user: Option<String>,
    /// Whether gh CLI is authenticated
    pub gh_authenticated: bool,
}

/// Overall AI plugin state
#[derive(Debug, Clone, Default)]
pub struct AIState {
    pub view: AIView,
    pub menu_index: usize,
    pub selected_provider: AIProvider,
    pub claude: ClaudeStatus,
    pub codex: CodexStatus,
    pub gemini: GeminiStatus,
    pub cursor: CursorStatus,
    pub copilot: CopilotStatus,
    pub scroll_offset: usize,
    /// Dry run state for operation confirmation
    pub dry_run: Option<DryRunState>,
    /// Command palette state
    pub command_palette: CommandPaletteState,
}

impl AIState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Count how many providers are available
    pub fn available_count(&self) -> usize {
        let mut count = 0;
        if self.claude.available {
            count += 1;
        }
        if self.codex.available {
            count += 1;
        }
        if self.gemini.available {
            count += 1;
        }
        if self.cursor.available {
            count += 1;
        }
        if self.copilot.available {
            count += 1;
        }
        count
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
    fn test_dry_run_state_no_destructive() {
        let ops = vec![
            DryRunOperation::new(DryRunOpType::Create, PathBuf::from("/a"), "Create file"),
            DryRunOperation::new(DryRunOpType::Copy, PathBuf::from("/b"), "Copy file"),
        ];

        let state = DryRunState::new("Safe operation", ops);

        assert_eq!(state.destructive_count(), 0);
        assert!(!state.has_destructive());
        assert!(!state.has_deletions());
    }

    #[test]
    fn test_dry_run_state_navigation() {
        let ops = vec![
            DryRunOperation::new(DryRunOpType::Create, PathBuf::from("/a"), "Op 1"),
            DryRunOperation::new(DryRunOpType::Create, PathBuf::from("/b"), "Op 2"),
            DryRunOperation::new(DryRunOpType::Create, PathBuf::from("/c"), "Op 3"),
        ];

        let mut state = DryRunState::new("Test", ops);

        assert_eq!(state.selected, 0);

        state.select_next();
        assert_eq!(state.selected, 1);

        state.select_next();
        assert_eq!(state.selected, 2);

        // Should not go past end
        state.select_next();
        assert_eq!(state.selected, 2);

        state.select_prev();
        assert_eq!(state.selected, 1);

        state.select_prev();
        assert_eq!(state.selected, 0);

        // Should not go past start
        state.select_prev();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn test_ai_view_dry_run_title() {
        assert_eq!(AIView::DryRun.title(), "AI Operation Preview");
    }
}
