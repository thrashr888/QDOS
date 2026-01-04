//! Beads Plugin State Types
//!
//! All state types for the Beads plugin, moved from app/state.rs for self-containment.

/// Beads menu item options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BeadsMenuItem {
    #[default]
    List,
    Ready,
    Blocked,
    Epics,
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
    pub const INITIALIZED: [BeadsMenuItem; 11] = [
        BeadsMenuItem::List,
        BeadsMenuItem::Ready,
        BeadsMenuItem::Blocked,
        BeadsMenuItem::Epics,
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
            BeadsMenuItem::Epics => "Epics",
            BeadsMenuItem::Stats => "Stats",
            BeadsMenuItem::Create => "Create",
            BeadsMenuItem::Graph => "Graph",
            BeadsMenuItem::Kanban => "Kanban",
            BeadsMenuItem::Sync => "Sync",
            BeadsMenuItem::Human => "Help",
            BeadsMenuItem::Init => "Init",
            BeadsMenuItem::Doctor => "Doctor",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            BeadsMenuItem::List => "List all open issues",
            BeadsMenuItem::Ready => "Show issues ready to work on",
            BeadsMenuItem::Blocked => "Show blocked issues",
            BeadsMenuItem::Epics => "Show all epics and their progress",
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
    Epics,
    Stats,
    Create,
    Detail,
    Edit,
    Comments,
    Dependencies,
    Kanban,
    History,
    FileIssues,
    Human,
    Doctor,
}

/// Kanban board sort mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KanbanSort {
    #[default]
    PriorityAsc,
    PriorityDesc,
    TitleAsc,
    TitleDesc,
    IdAsc,
    IdDesc,
}

impl KanbanSort {
    /// Get display name for the sort mode
    pub fn as_str(&self) -> &'static str {
        match self {
            KanbanSort::PriorityAsc => "Priority ↑",
            KanbanSort::PriorityDesc => "Priority ↓",
            KanbanSort::TitleAsc => "Title A-Z",
            KanbanSort::TitleDesc => "Title Z-A",
            KanbanSort::IdAsc => "ID ↑",
            KanbanSort::IdDesc => "ID ↓",
        }
    }

    /// Cycle to next sort mode
    pub fn next(&self) -> Self {
        match self {
            KanbanSort::PriorityAsc => KanbanSort::PriorityDesc,
            KanbanSort::PriorityDesc => KanbanSort::TitleAsc,
            KanbanSort::TitleAsc => KanbanSort::TitleDesc,
            KanbanSort::TitleDesc => KanbanSort::IdAsc,
            KanbanSort::IdAsc => KanbanSort::IdDesc,
            KanbanSort::IdDesc => KanbanSort::PriorityAsc,
        }
    }
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
    /// Scroll offset for detail view
    pub detail_scroll: usize,
    /// Stats data
    pub stats: BeadsStats,
    /// Create form state
    pub create_title: String,
    pub create_description: String,
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
    /// Kanban sort mode
    pub kanban_sort: KanbanSort,
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
    /// Edit mode - issue ID being edited
    pub edit_issue_id: String,
    /// Edit mode - title input
    pub edit_title: String,
    /// Edit mode - description input
    pub edit_description: String,
    /// Edit mode - current field (0=title, 1=description, 2=status, 3=priority)
    pub edit_field: usize,
    /// Edit mode - status (0=open, 1=in_progress, 2=closed)
    pub edit_status: usize,
    /// Edit mode - priority (0-4)
    pub edit_priority: usize,
    /// Subtask creation - parent issue ID
    pub subtask_parent_id: String,
    /// Recent issues for quick access (in_progress and recently touched)
    pub recent_issues: Vec<BeadsIssue>,
    /// Top open epics for main menu display
    pub top_epics: Vec<BeadsIssue>,
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
            detail_scroll: 0,
            stats: BeadsStats::default(),
            create_title: String::new(),
            create_description: String::new(),
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
            kanban_sort: KanbanSort::default(),
            activity_entries: Vec::new(),
            selected_activity: 0,
            file_query_path: String::new(),
            file_related_issues: Vec::new(),
            file_issue_selected: 0,
            edit_issue_id: String::new(),
            edit_title: String::new(),
            edit_description: String::new(),
            edit_field: 0,
            edit_status: 0,
            edit_priority: 2,
            subtask_parent_id: String::new(),
            recent_issues: Vec::new(),
            top_epics: Vec::new(),
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
