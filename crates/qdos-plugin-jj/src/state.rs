//! Jj Plugin State Types
//!
//! All state types for the jj (Jujutsu) VCS plugin.

/// Jj menu item options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JjMenuItem {
    #[default]
    Status,
    Log,
    Diff,
    Describe,
    New,
    Bookmark,
    Operations,
    Git,
}

impl JjMenuItem {
    pub const ALL: [JjMenuItem; 8] = [
        JjMenuItem::Status,
        JjMenuItem::Log,
        JjMenuItem::Diff,
        JjMenuItem::Describe,
        JjMenuItem::New,
        JjMenuItem::Bookmark,
        JjMenuItem::Operations,
        JjMenuItem::Git,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            JjMenuItem::Status => "Status",
            JjMenuItem::Log => "Log",
            JjMenuItem::Diff => "Diff",
            JjMenuItem::Describe => "Describe",
            JjMenuItem::New => "New",
            JjMenuItem::Bookmark => "Bookmark",
            JjMenuItem::Operations => "Operations",
            JjMenuItem::Git => "Git",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            JjMenuItem::Status => "Show working copy status",
            JjMenuItem::Log => "View revision history",
            JjMenuItem::Diff => "Show changes in working copy",
            JjMenuItem::Describe => "Update change description",
            JjMenuItem::New => "Create a new change",
            JjMenuItem::Bookmark => "Manage bookmarks",
            JjMenuItem::Operations => "View operation log, undo",
            JjMenuItem::Git => "Git remote operations",
        }
    }
}

/// Jj view type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JjView {
    #[default]
    Menu,
    Status,
    Log,
    Diff,
    Describe,
    Bookmark,
    Operations,
    Git,
}

/// A jj change (revision)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JjChange {
    /// Short change ID (e.g., "uyuqkvlm")
    pub change_id: String,
    /// Short commit ID (e.g., "fded3c77")
    pub commit_id: String,
    /// Author email
    pub author: String,
    /// Date string
    pub date: String,
    /// Change description (first line)
    pub description: String,
    /// Whether this is the working copy (@)
    pub is_working_copy: bool,
    /// Whether this change is empty
    pub is_empty: bool,
}

/// A jj bookmark
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JjBookmark {
    /// Bookmark name
    pub name: String,
    /// Target change ID
    pub target: String,
    /// Whether this is a remote-tracking bookmark
    pub is_remote: bool,
    /// Remote name if remote-tracking
    pub remote: Option<String>,
    /// Whether there's a conflict
    pub is_conflicted: bool,
}

/// A jj operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JjOperation {
    /// Operation ID
    pub id: String,
    /// Operation description
    pub description: String,
    /// Timestamp
    pub time: String,
    /// Whether this is the current operation
    pub is_current: bool,
}

/// File status in jj
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JjFileStatus {
    /// File path
    pub path: String,
    /// Status: M (modified), A (added), D (deleted)
    pub status: char,
}

/// Git remote action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GitAction {
    #[default]
    Fetch,
    Push,
}

/// Jj state for the modal
#[derive(Debug, Clone)]
pub struct JjState {
    /// Current view
    pub view: JjView,
    /// Selected menu item
    pub menu_selected: usize,
    /// Whether we're in a jj repository
    pub is_repo: bool,
    /// Error message if any
    pub error: Option<String>,

    // Status view
    /// Files with changes
    pub files: Vec<JjFileStatus>,
    /// Working copy change info
    pub working_copy: Option<JjChange>,
    /// Parent change info
    pub parent: Option<JjChange>,

    // Log view
    /// Change log entries
    pub changes: Vec<JjChange>,
    /// Selected change in log
    pub selected_change: usize,
    /// Scroll offset for log
    pub scroll_offset: usize,

    // Diff view
    /// Diff content lines
    pub diff_content: Vec<String>,
    /// Previous view to return to
    pub prev_view: Option<JjView>,

    // Describe view
    /// Description input buffer
    pub description_input: String,
    /// Whether in input mode
    pub input_mode: bool,

    // Bookmark view
    /// List of bookmarks
    pub bookmarks: Vec<JjBookmark>,
    /// Selected bookmark
    pub selected_bookmark: usize,
    /// New bookmark name input
    pub bookmark_input: String,
    /// Whether in bookmark input mode
    pub bookmark_input_mode: bool,

    // Operations view
    /// List of operations
    pub operations: Vec<JjOperation>,
    /// Selected operation
    pub selected_operation: usize,

    // Git view
    /// Current git action
    pub git_action: GitAction,
}

impl Default for JjState {
    fn default() -> Self {
        Self {
            view: JjView::Menu,
            menu_selected: 0,
            is_repo: false,
            error: None,
            files: Vec::new(),
            working_copy: None,
            parent: None,
            changes: Vec::new(),
            selected_change: 0,
            scroll_offset: 0,
            diff_content: Vec::new(),
            prev_view: None,
            description_input: String::new(),
            input_mode: false,
            bookmarks: Vec::new(),
            selected_bookmark: 0,
            bookmark_input: String::new(),
            bookmark_input_mode: false,
            operations: Vec::new(),
            selected_operation: 0,
            git_action: GitAction::Fetch,
        }
    }
}

impl JjState {
    pub fn new(is_repo: bool) -> Self {
        Self {
            is_repo,
            ..Default::default()
        }
    }
}
