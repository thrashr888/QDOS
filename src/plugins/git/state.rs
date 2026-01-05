//! Git Plugin State Types
//!
//! All state types for the Git plugin, moved from app/state.rs for self-containment.

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
    Reflog,
    Remotes,
    Worktrees,
    Config,
    Conflicts,
    Submodules,
}

impl GitMenuItem {
    pub const ALL: [GitMenuItem; 15] = [
        GitMenuItem::Status,
        GitMenuItem::Log,
        GitMenuItem::Diff,
        GitMenuItem::Commit,
        GitMenuItem::Push,
        GitMenuItem::Pull,
        GitMenuItem::Branch,
        GitMenuItem::Stash,
        GitMenuItem::Tag,
        GitMenuItem::Reflog,
        GitMenuItem::Remotes,
        GitMenuItem::Worktrees,
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
            GitMenuItem::Reflog => "Reflog",
            GitMenuItem::Remotes => "Remotes",
            GitMenuItem::Worktrees => "Worktrees",
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
            GitMenuItem::Reflog => "View reference log history",
            GitMenuItem::Remotes => "Manage remote repositories",
            GitMenuItem::Worktrees => "Manage linked working trees",
            GitMenuItem::Config => "View git configuration",
            GitMenuItem::Conflicts => "Resolve merge conflicts",
            GitMenuItem::Submodules => "Manage git submodules",
        }
    }
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
    Reflog,
    Remote,
    Worktrees,
    Config,
    Conflicts,
    Submodules,
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
    #[allow(dead_code)] // Reserved for future use
    OutOfDate,
    Modified,
    Conflict,
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

/// Git file history entry (for file viewer)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileHistoryEntry {
    pub hash: String,
    pub date: String,
    pub message: String,
}

/// Git blame line entry (for file viewer)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlameLine {
    pub hash: String,
    pub author: String,
    pub time_ago: String,
    pub line_content: String,
}

/// Git reflog entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitReflogEntry {
    pub hash: String,
    pub selector: String, // e.g., "HEAD@{0}"
    pub action: String,   // e.g., "commit", "checkout", "reset"
    pub message: String,
    pub time_ago: String,
}

/// Git worktree entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitWorktree {
    pub path: String,
    pub branch: Option<String>,
    pub commit: String,
    pub is_main: bool,
    pub is_bare: bool,
    pub is_locked: bool,
    pub is_prunable: bool,
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
    /// Reflog entries
    pub reflog_entries: Vec<GitReflogEntry>,
    /// Selected reflog entry
    pub selected_reflog: usize,
    /// Worktrees list
    pub worktrees: Vec<GitWorktree>,
    /// Selected worktree
    pub selected_worktree: usize,
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
            reflog_entries: Vec::new(),
            selected_reflog: 0,
            worktrees: Vec::new(),
            selected_worktree: 0,
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
