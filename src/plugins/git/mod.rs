//! Git Plugin for R-DOS
//!
//! Provides Git integration as a plugin with self-contained operations.

pub mod modal;
pub mod ops;
pub mod state;

// Re-export state types for external use
#[allow(unused_imports)]
pub use state::{
    BlameLine, ConflictFile, ConflictResolution, ConflictSection, FileHistoryEntry, GitBranch,
    GitConfigEntry, GitFileStatus, GitLogEntry, GitMenuItem, GitReflogEntry, GitRemote,
    GitStashEntry, GitState, GitSubmodule, GitTag, GitView, GitWorktree, RemoteAction,
    SubmoduleStatus,
};

use super::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, PluginMenuItem,
    PluginStatusInfo,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::Frame;
use std::any::Any;
use std::path::PathBuf;
use std::process::Command;

/// Git plugin that provides version control integration
pub struct GitPlugin {
    /// Cached info about whether we're in a git repo
    is_repo: bool,
    /// Current branch name
    branch: String,
    /// Number of staged files
    staged: usize,
    /// Number of modified files
    modified: usize,
    /// Commits ahead of remote
    ahead: u32,
    /// Commits behind remote
    behind: u32,
    /// Modal state when git modal is open (plugin owns this state)
    pub modal_state: Option<GitState>,
}

impl GitPlugin {
    pub fn new() -> Self {
        Self {
            is_repo: false,
            branch: String::new(),
            staged: 0,
            modified: 0,
            ahead: 0,
            behind: 0,
            modal_state: None,
        }
    }

    /// Open the git modal with fresh state
    pub fn open_modal(&mut self, cwd: &PathBuf) {
        let is_repo = self.check_is_repo(cwd);
        self.modal_state = Some(GitState::new(is_repo));
    }

    /// Close the git modal
    pub fn close_modal(&mut self) {
        self.modal_state = None;
    }

    /// Get mutable reference to modal state
    pub fn modal_state_mut(&mut self) -> Option<&mut GitState> {
        self.modal_state.as_mut()
    }

    /// Handle key event with external state (for Modal::Git delegation)
    /// This method temporarily uses the provided state for key handling,
    /// allowing the app to delegate key handling while keeping state in Modal::Git.
    pub fn handle_external_state_key(
        &mut self,
        key: KeyEvent,
        state: &mut GitState,
        cwd: &PathBuf,
    ) -> KeyHandleResult {
        // Store the external state temporarily
        self.modal_state = Some(std::mem::take(state));
        let result = self.handle_modal_key(key, cwd);
        // Copy updated state back
        if let Some(updated) = self.modal_state.take() {
            *state = updated;
        }
        result
    }

    /// Check if a directory is a git repository
    fn check_is_repo(&self, cwd: &PathBuf) -> bool {
        Command::new("git")
            .args(["rev-parse", "--git-dir"])
            .current_dir(cwd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Update cached git status
    fn refresh_status(&mut self, cwd: &PathBuf) {
        self.is_repo = self.check_is_repo(cwd);
        if !self.is_repo {
            self.branch = String::new();
            self.staged = 0;
            self.modified = 0;
            self.ahead = 0;
            self.behind = 0;
            return;
        }

        // Get branch name
        if let Ok(output) = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(cwd)
            .output()
        {
            self.branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        }

        // Get status counts
        if let Ok(output) = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(cwd)
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            self.staged = 0;
            self.modified = 0;
            for line in stdout.lines() {
                if line.len() >= 2 {
                    let first = line.chars().next().unwrap_or(' ');
                    let second = line.chars().nth(1).unwrap_or(' ');
                    if first != ' ' && first != '?' {
                        self.staged += 1;
                    }
                    if second != ' ' {
                        self.modified += 1;
                    }
                }
            }
        }

        // Get ahead/behind counts
        if let Ok(output) = Command::new("git")
            .args(["rev-list", "--left-right", "--count", "@{u}...HEAD"])
            .current_dir(cwd)
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let parts: Vec<&str> = stdout.split_whitespace().collect();
                if parts.len() == 2 {
                    self.behind = parts[0].parse().unwrap_or(0);
                    self.ahead = parts[1].parse().unwrap_or(0);
                }
            }
        }
    }

    // --- Key Handlers for each Git view ---

    fn handle_menu_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        match key.code {
            KeyCode::Esc => {
                self.close_modal();
                KeyHandleResult::CloseModal
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if state.menu_selected > 0 {
                    state.menu_selected -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.menu_selected < GitMenuItem::ALL.len() - 1 {
                    state.menu_selected += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                let item = GitMenuItem::ALL[state.menu_selected];
                self.activate_menu_item(item, cwd);
                KeyHandleResult::Handled
            }
            // Shortcut keys
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.activate_menu_item(GitMenuItem::Status, cwd);
                KeyHandleResult::Handled
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                self.activate_menu_item(GitMenuItem::Log, cwd);
                KeyHandleResult::Handled
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                self.activate_menu_item(GitMenuItem::Diff, cwd);
                KeyHandleResult::Handled
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.activate_menu_item(GitMenuItem::Commit, cwd);
                KeyHandleResult::Handled
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                self.activate_menu_item(GitMenuItem::Push, cwd);
                KeyHandleResult::Handled
            }
            KeyCode::Char('u') | KeyCode::Char('U') => {
                self.activate_menu_item(GitMenuItem::Pull, cwd);
                KeyHandleResult::Handled
            }
            KeyCode::Char('b') | KeyCode::Char('B') => {
                self.activate_menu_item(GitMenuItem::Branch, cwd);
                KeyHandleResult::Handled
            }
            KeyCode::Char('h') | KeyCode::Char('H') => {
                self.activate_menu_item(GitMenuItem::Stash, cwd);
                KeyHandleResult::Handled
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                self.activate_menu_item(GitMenuItem::Tag, cwd);
                KeyHandleResult::Handled
            }
            KeyCode::Char('g') | KeyCode::Char('G') => {
                self.activate_menu_item(GitMenuItem::Config, cwd);
                KeyHandleResult::Handled
            }
            KeyCode::Char('x') | KeyCode::Char('X') => {
                self.activate_menu_item(GitMenuItem::Conflicts, cwd);
                KeyHandleResult::Handled
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                self.activate_menu_item(GitMenuItem::Submodules, cwd);
                KeyHandleResult::Handled
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.activate_menu_item(GitMenuItem::Reflog, cwd);
                KeyHandleResult::Handled
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                self.activate_menu_item(GitMenuItem::Remotes, cwd);
                KeyHandleResult::Handled
            }
            KeyCode::Char('w') | KeyCode::Char('W') => {
                self.activate_menu_item(GitMenuItem::Worktrees, cwd);
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn activate_menu_item(&mut self, item: GitMenuItem, cwd: &PathBuf) {
        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return,
        };

        match item {
            GitMenuItem::Status => {
                state.view = GitView::Status;
                ops::load_git_status(state, cwd);
            }
            GitMenuItem::Log => {
                state.view = GitView::Log;
                ops::load_git_log(state, cwd);
            }
            GitMenuItem::Diff => {
                state.view = GitView::Diff;
                ops::load_git_diff(state, cwd);
            }
            GitMenuItem::Commit => {
                state.view = GitView::Commit;
                state.commit_input_mode = true;
            }
            GitMenuItem::Push => {
                state.remote_action = RemoteAction::Push;
                ops::load_remotes(state, cwd);
                state.view = GitView::Remote;
            }
            GitMenuItem::Pull => {
                state.remote_action = RemoteAction::Pull;
                ops::load_remotes(state, cwd);
                state.view = GitView::Remote;
            }
            GitMenuItem::Branch => {
                state.view = GitView::Branch;
                ops::load_branches(state, cwd);
            }
            GitMenuItem::Stash => {
                state.view = GitView::Stash;
                ops::load_stashes(state, cwd);
            }
            GitMenuItem::Tag => {
                state.view = GitView::Tag;
                ops::load_tags(state, cwd);
            }
            GitMenuItem::Config => {
                state.view = GitView::Config;
                ops::load_git_config(state, cwd);
            }
            GitMenuItem::Conflicts => {
                state.view = GitView::Conflicts;
                ops::load_conflict_files(state, cwd);
            }
            GitMenuItem::Submodules => {
                state.view = GitView::Submodules;
                ops::load_submodules(state, cwd);
            }
            GitMenuItem::Reflog => {
                state.view = GitView::Reflog;
                ops::load_reflog(state, cwd);
            }
            GitMenuItem::Remotes => {
                ops::load_remotes(state, cwd);
                state.view = GitView::Remote;
            }
            GitMenuItem::Worktrees => {
                state.view = GitView::Worktrees;
                ops::load_worktrees(state, cwd);
            }
        }
    }

    fn handle_status_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        match key.code {
            KeyCode::Esc => {
                state.view = GitView::Menu;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if state.selected_file > 0 {
                    state.selected_file -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.selected_file < state.files.len().saturating_sub(1) {
                    state.selected_file += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                // Toggle stage/unstage file
                ops::toggle_git_stage(state, cwd);
                KeyHandleResult::Handled
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                // Refresh status
                ops::load_git_status(state, cwd);
                KeyHandleResult::Handled
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                // View diff of selected file
                if let Some(file) = state.files.get(state.selected_file) {
                    let path = file.path.clone();
                    state.prev_view = Some(GitView::Status);
                    state.view = GitView::Diff;
                    ops::load_file_diff(state, cwd, &path);
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_log_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        match key.code {
            KeyCode::Esc => {
                state.view = GitView::Menu;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if state.selected_log > 0 {
                    state.selected_log -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.selected_log < state.log_entries.len().saturating_sub(1) {
                    state.selected_log += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                // Show commit diff
                if let Some(entry) = state.log_entries.get(state.selected_log) {
                    let hash = entry.hash.clone();
                    state.prev_view = Some(GitView::Log);
                    state.view = GitView::Diff;
                    ops::load_commit_diff(state, cwd, &hash);
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_diff_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        match key.code {
            KeyCode::Esc => {
                // Return to previous view or menu
                state.view = state.prev_view.take().unwrap_or(GitView::Menu);
                state.diff_content.clear();
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if state.scroll_offset > 0 {
                    state.scroll_offset -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                state.scroll_offset += 1;
                KeyHandleResult::Handled
            }
            KeyCode::PageUp => {
                state.scroll_offset = state.scroll_offset.saturating_sub(20);
                KeyHandleResult::Handled
            }
            KeyCode::PageDown => {
                state.scroll_offset += 20;
                KeyHandleResult::Handled
            }
            KeyCode::Home => {
                state.scroll_offset = 0;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_commit_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        if state.commit_input_mode {
            match key.code {
                KeyCode::Esc => {
                    state.commit_input_mode = false;
                    state.commit_message.clear();
                    state.view = GitView::Menu;
                    KeyHandleResult::Handled
                }
                KeyCode::Enter => {
                    if !state.commit_message.is_empty() {
                        match ops::execute_git_commit(&state.commit_message, cwd) {
                            Ok(()) => {
                                let msg = format!("Committed: {}", state.commit_message);
                                state.commit_message.clear();
                                state.commit_input_mode = false;
                                self.close_modal();
                                return KeyHandleResult::CloseWithSuccess(msg);
                            }
                            Err(e) => {
                                state.error = Some(e);
                            }
                        }
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Backspace => {
                    state.commit_message.pop();
                    KeyHandleResult::Handled
                }
                KeyCode::Char(c) => {
                    state.commit_message.push(c);
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            }
        } else {
            match key.code {
                KeyCode::Esc => {
                    state.view = GitView::Menu;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            }
        }
    }

    fn handle_branch_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        if state.branch_input_mode {
            match key.code {
                KeyCode::Esc => {
                    state.branch_input_mode = false;
                    state.branch_name_input.clear();
                    KeyHandleResult::Handled
                }
                KeyCode::Enter => {
                    if !state.branch_name_input.is_empty() {
                        match ops::create_branch(&state.branch_name_input, cwd) {
                            Ok(msg) => {
                                state.branch_name_input.clear();
                                state.branch_input_mode = false;
                                ops::load_branches(state, cwd);
                                state.error = None;
                                // Could return CloseWithSuccess here, but stay in branch view
                                state.error = Some(msg); // Use error field as message temporarily
                            }
                            Err(e) => {
                                state.error = Some(e);
                            }
                        }
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Backspace => {
                    state.branch_name_input.pop();
                    KeyHandleResult::Handled
                }
                KeyCode::Char(c) => {
                    state.branch_name_input.push(c);
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            }
        } else {
            match key.code {
                KeyCode::Esc => {
                    state.view = GitView::Menu;
                    KeyHandleResult::Handled
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if state.selected_branch > 0 {
                        state.selected_branch -= 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if state.selected_branch < state.branches.len().saturating_sub(1) {
                        state.selected_branch += 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    // Switch to selected branch
                    if let Some(branch) = state.branches.get(state.selected_branch) {
                        if !branch.is_current {
                            let name = branch.name.clone();
                            match ops::switch_branch(&name, cwd) {
                                Ok(msg) => {
                                    ops::load_branches(state, cwd);
                                    self.close_modal();
                                    return KeyHandleResult::CloseWithSuccess(msg);
                                }
                                Err(e) => {
                                    state.error = Some(e);
                                }
                            }
                        }
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    state.branch_input_mode = true;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    // Delete branch
                    if let Some(branch) = state.branches.get(state.selected_branch) {
                        if !branch.is_current && !branch.is_remote {
                            let name = branch.name.clone();
                            match ops::delete_branch(&name, cwd) {
                                Ok(_) => {
                                    ops::load_branches(state, cwd);
                                    if state.selected_branch >= state.branches.len() {
                                        state.selected_branch =
                                            state.branches.len().saturating_sub(1);
                                    }
                                }
                                Err(e) => {
                                    state.error = Some(e);
                                }
                            }
                        }
                    }
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            }
        }
    }

    fn handle_stash_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        if state.stash_input_mode {
            match key.code {
                KeyCode::Esc => {
                    state.stash_input_mode = false;
                    state.stash_message_input.clear();
                    KeyHandleResult::Handled
                }
                KeyCode::Enter => {
                    let msg = if state.stash_message_input.is_empty() {
                        None
                    } else {
                        Some(state.stash_message_input.as_str())
                    };
                    match ops::create_stash(msg, cwd) {
                        Ok(result) => {
                            state.stash_message_input.clear();
                            state.stash_input_mode = false;
                            ops::load_stashes(state, cwd);
                            self.close_modal();
                            return KeyHandleResult::CloseWithSuccess(result);
                        }
                        Err(e) => {
                            state.error = Some(e);
                        }
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Backspace => {
                    state.stash_message_input.pop();
                    KeyHandleResult::Handled
                }
                KeyCode::Char(c) => {
                    state.stash_message_input.push(c);
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            }
        } else {
            match key.code {
                KeyCode::Esc => {
                    state.view = GitView::Menu;
                    KeyHandleResult::Handled
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if state.selected_stash > 0 {
                        state.selected_stash -= 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if state.selected_stash < state.stashes.len().saturating_sub(1) {
                        state.selected_stash += 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    // Push to stash
                    state.stash_input_mode = true;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('p') | KeyCode::Char('P') | KeyCode::Enter => {
                    // Apply and drop stash (pop)
                    if !state.stashes.is_empty() {
                        let idx = state.selected_stash;
                        match ops::apply_stash(idx, cwd) {
                            Ok(_) => {
                                // Apply succeeded, now drop
                                let _ = ops::drop_stash(idx, cwd);
                                ops::load_stashes(state, cwd);
                                if state.selected_stash >= state.stashes.len() {
                                    state.selected_stash = state.stashes.len().saturating_sub(1);
                                }
                                self.close_modal();
                                return KeyHandleResult::CloseWithSuccess(format!(
                                    "Applied stash@{{{}}}",
                                    idx
                                ));
                            }
                            Err(e) => {
                                state.error = Some(e);
                            }
                        }
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    // Drop stash
                    if !state.stashes.is_empty() {
                        let idx = state.selected_stash;
                        match ops::drop_stash(idx, cwd) {
                            Ok(_) => {
                                ops::load_stashes(state, cwd);
                                if state.selected_stash >= state.stashes.len() {
                                    state.selected_stash = state.stashes.len().saturating_sub(1);
                                }
                            }
                            Err(e) => {
                                state.error = Some(e);
                            }
                        }
                    }
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            }
        }
    }

    fn handle_tag_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        if state.tag_input_mode {
            match key.code {
                KeyCode::Esc => {
                    state.tag_input_mode = false;
                    state.tag_name_input.clear();
                    KeyHandleResult::Handled
                }
                KeyCode::Enter => {
                    if !state.tag_name_input.is_empty() {
                        match ops::create_tag(&state.tag_name_input, cwd) {
                            Ok(msg) => {
                                state.tag_name_input.clear();
                                state.tag_input_mode = false;
                                ops::load_tags(state, cwd);
                                self.close_modal();
                                return KeyHandleResult::CloseWithSuccess(msg);
                            }
                            Err(e) => {
                                state.error = Some(e);
                            }
                        }
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Backspace => {
                    state.tag_name_input.pop();
                    KeyHandleResult::Handled
                }
                KeyCode::Char(c) => {
                    state.tag_name_input.push(c);
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            }
        } else {
            match key.code {
                KeyCode::Esc => {
                    state.view = GitView::Menu;
                    KeyHandleResult::Handled
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if state.selected_tag > 0 {
                        state.selected_tag -= 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if state.selected_tag < state.tags.len().saturating_sub(1) {
                        state.selected_tag += 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    state.tag_input_mode = true;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    // Delete tag
                    if let Some(tag) = state.tags.get(state.selected_tag) {
                        let name = tag.name.clone();
                        match ops::delete_tag(&name, cwd) {
                            Ok(_) => {
                                ops::load_tags(state, cwd);
                                if state.selected_tag >= state.tags.len() {
                                    state.selected_tag = state.tags.len().saturating_sub(1);
                                }
                            }
                            Err(e) => {
                                state.error = Some(e);
                            }
                        }
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    // Show tag details (diff)
                    if let Some(tag) = state.tags.get(state.selected_tag) {
                        let commit = tag.commit.clone();
                        state.prev_view = Some(GitView::Tag);
                        state.view = GitView::Diff;
                        ops::load_commit_diff(state, cwd, &commit);
                    }
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            }
        }
    }

    fn handle_remote_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        match key.code {
            KeyCode::Esc => {
                state.view = GitView::Menu;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if state.selected_remote > 0 {
                    state.selected_remote -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.selected_remote < state.remotes.len().saturating_sub(1) {
                    state.selected_remote += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(remote) = state.remotes.get(state.selected_remote) {
                    let remote_name = remote.name.clone();
                    let result = match state.remote_action {
                        RemoteAction::Push => ops::execute_git_push_to(&remote_name, cwd),
                        RemoteAction::Pull => ops::execute_git_pull_from(&remote_name, cwd),
                    };
                    match result {
                        Ok(msg) => {
                            self.close_modal();
                            return KeyHandleResult::CloseWithSuccess(msg);
                        }
                        Err(e) => {
                            state.error = Some(e);
                        }
                    }
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_config_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        match key.code {
            KeyCode::Esc => {
                state.view = GitView::Menu;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if state.selected_config > 0 {
                    state.selected_config -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.selected_config < state.config_entries.len().saturating_sub(1) {
                    state.selected_config += 1;
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_conflicts_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        match key.code {
            KeyCode::Esc => {
                state.view = GitView::Menu;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if state.selected_conflict_file > 0 {
                    state.selected_conflict_file -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.selected_conflict_file < state.conflict_files.len().saturating_sub(1) {
                    state.selected_conflict_file += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('o') | KeyCode::Char('O') => {
                // Accept ours
                if let Some(file) = state.conflict_files.get(state.selected_conflict_file) {
                    let path = file.path.clone();
                    match ops::resolve_conflict_ours(&path, cwd) {
                        Ok(msg) => {
                            ops::load_conflict_files(state, cwd);
                            self.close_modal();
                            return KeyHandleResult::CloseWithSuccess(msg);
                        }
                        Err(e) => {
                            state.error = Some(e);
                        }
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                // Accept theirs
                if let Some(file) = state.conflict_files.get(state.selected_conflict_file) {
                    let path = file.path.clone();
                    match ops::resolve_conflict_theirs(&path, cwd) {
                        Ok(msg) => {
                            ops::load_conflict_files(state, cwd);
                            self.close_modal();
                            return KeyHandleResult::CloseWithSuccess(msg);
                        }
                        Err(e) => {
                            state.error = Some(e);
                        }
                    }
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_submodules_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        match key.code {
            KeyCode::Esc => {
                state.view = GitView::Menu;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if state.selected_submodule > 0 {
                    state.selected_submodule -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.selected_submodule < state.submodules.len().saturating_sub(1) {
                    state.selected_submodule += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                // Initialize submodule
                if let Some(submodule) = state.submodules.get(state.selected_submodule) {
                    let path = submodule.path.clone();
                    match ops::init_submodule(Some(&path), cwd) {
                        Ok(msg) => {
                            ops::load_submodules(state, cwd);
                            state.error = Some(msg); // Use as success message
                        }
                        Err(e) => {
                            state.error = Some(e);
                        }
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('u') | KeyCode::Char('U') => {
                // Update submodule
                if let Some(submodule) = state.submodules.get(state.selected_submodule) {
                    let path = submodule.path.clone();
                    match ops::update_submodule(Some(&path), cwd) {
                        Ok(msg) => {
                            ops::load_submodules(state, cwd);
                            self.close_modal();
                            return KeyHandleResult::CloseWithSuccess(msg);
                        }
                        Err(e) => {
                            state.error = Some(e);
                        }
                    }
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_reflog_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        match key.code {
            KeyCode::Esc => {
                state.view = GitView::Menu;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if state.selected_reflog > 0 {
                    state.selected_reflog -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.selected_reflog < state.reflog_entries.len().saturating_sub(1) {
                    state.selected_reflog += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                // View diff for selected reflog entry
                if let Some(entry) = state.reflog_entries.get(state.selected_reflog) {
                    let hash = entry.hash.clone();
                    state.prev_view = Some(GitView::Reflog);
                    state.view = GitView::Diff;
                    ops::load_commit_diff(state, cwd, &hash);
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                // Checkout this reflog entry
                if let Some(entry) = state.reflog_entries.get(state.selected_reflog) {
                    let selector = entry.selector.clone();
                    match ops::checkout_reflog_entry(&selector, cwd) {
                        Ok(msg) => {
                            self.close_modal();
                            return KeyHandleResult::CloseWithSuccess(msg);
                        }
                        Err(e) => {
                            state.error = Some(e);
                        }
                    }
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_worktrees_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        match key.code {
            KeyCode::Esc => {
                state.view = GitView::Menu;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if state.selected_worktree > 0 {
                    state.selected_worktree -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.selected_worktree < state.worktrees.len().saturating_sub(1) {
                    state.selected_worktree += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                // Lock/unlock worktree
                if let Some(worktree) = state.worktrees.get(state.selected_worktree) {
                    if worktree.is_main {
                        state.error = Some("Cannot lock main worktree".to_string());
                    } else {
                        let path = worktree.path.clone();
                        let result = if worktree.is_locked {
                            ops::unlock_worktree(&path, cwd)
                        } else {
                            ops::lock_worktree(&path, cwd)
                        };
                        match result {
                            Ok(msg) => {
                                ops::load_worktrees(state, cwd);
                                state.error = Some(msg);
                            }
                            Err(e) => {
                                state.error = Some(e);
                            }
                        }
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                // Remove worktree
                if let Some(worktree) = state.worktrees.get(state.selected_worktree) {
                    if worktree.is_main {
                        state.error = Some("Cannot remove main worktree".to_string());
                    } else {
                        let path = worktree.path.clone();
                        match ops::remove_worktree(&path, false, cwd) {
                            Ok(msg) => {
                                ops::load_worktrees(state, cwd);
                                if state.selected_worktree >= state.worktrees.len() {
                                    state.selected_worktree =
                                        state.worktrees.len().saturating_sub(1);
                                }
                                state.error = Some(msg);
                            }
                            Err(e) => {
                                state.error = Some(e);
                            }
                        }
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                // Prune stale worktrees
                match ops::prune_worktrees(cwd) {
                    Ok(msg) => {
                        ops::load_worktrees(state, cwd);
                        state.error = Some(msg);
                    }
                    Err(e) => {
                        state.error = Some(e);
                    }
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }
}

impl Default for GitPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for GitPlugin {
    fn id(&self) -> &str {
        "git"
    }

    fn name(&self) -> &str {
        "Git Integration"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: true,
            has_keys: true,
            has_modal: true, // Plugin owns modal state
            has_status: true,
            has_cli: false,
            has_help: true,
        }
    }

    fn is_available(&self, cwd: &PathBuf) -> bool {
        self.check_is_repo(cwd)
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "Git".to_string(),
            key: 'G',
            description: "Git version control menu".to_string(),
            priority: 10, // Show early in menu
        })
    }

    fn status_info(&self, cwd: &PathBuf) -> Option<PluginStatusInfo> {
        if !self.check_is_repo(cwd) {
            return None;
        }

        // Build status text similar to existing format
        let mut parts = Vec::new();
        if self.ahead > 0 || self.behind > 0 {
            parts.push(format!("↑{}↓{}", self.ahead, self.behind));
        }
        if self.staged > 0 {
            parts.push(format!("+{}", self.staged));
        }
        if self.modified > 0 {
            parts.push(format!("!{}", self.modified));
        }

        let text = if parts.is_empty() {
            self.branch.clone()
        } else {
            format!("{} {}", parts.join(" "), self.branch)
        };

        Some(PluginStatusInfo { text, active: true })
    }

    fn handle_global_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        match key.code {
            KeyCode::Char('g') | KeyCode::Char('G') => {
                // Open git modal with plugin-owned state
                self.open_modal(cwd);
                KeyHandleResult::OpenModal
            }
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        // Not a git repo - any key closes
        if !state.is_repo {
            match key.code {
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ') => {
                    self.close_modal();
                    return KeyHandleResult::CloseModal;
                }
                _ => return KeyHandleResult::Handled,
            }
        }

        match state.view {
            GitView::Menu => self.handle_menu_key(key, cwd),
            GitView::Status => self.handle_status_key(key, cwd),
            GitView::Log => self.handle_log_key(key, cwd),
            GitView::Diff => self.handle_diff_key(key),
            GitView::Commit => self.handle_commit_key(key, cwd),
            GitView::Branch => self.handle_branch_key(key, cwd),
            GitView::Stash => self.handle_stash_key(key, cwd),
            GitView::Tag => self.handle_tag_key(key, cwd),
            GitView::Reflog => self.handle_reflog_key(key, cwd),
            GitView::Remote => self.handle_remote_key(key, cwd),
            GitView::Worktrees => self.handle_worktrees_key(key, cwd),
            GitView::Config => self.handle_config_key(key),
            GitView::Conflicts => self.handle_conflicts_key(key, cwd),
            GitView::Submodules => self.handle_submodules_key(key, cwd),
        }
    }

    fn draw_modal(&self, _frame: &mut Frame, _area: Rect, _colors: &crate::app::ThemeColors) {
        // Modal drawing is done by the existing draw_git_modal
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "        G -- GIT VERSION CONTROL".to_string(),
            "".to_string(),
            "Purpose:   Git integration for version control operations.".to_string(),
            "           View changes, commit, push/pull, and manage branches.".to_string(),
            "".to_string(),
            "To use:    Press G to open the Git menu. Only available in".to_string(),
            "           directories with a .git folder (git repositories).".to_string(),
            "".to_string(),
            "Status bar shows: branch name, ↑ahead/↓behind counts,".to_string(),
            "                  +staged and !modified file counts.".to_string(),
            "".to_string(),
            "Navigation:".to_string(),
            "  Tab       Switch between views".to_string(),
            "  Enter     Select item or confirm".to_string(),
            "  Esc       Go back or close".to_string(),
            "  ↑/↓       Navigate lists".to_string(),
            "  Space     Stage/unstage files (in Status)".to_string(),
            "".to_string(),
            "Common workflow:".to_string(),
            "  1. Check Status to see changes".to_string(),
            "  2. Stage files with Space".to_string(),
            "  3. Select Commit, enter message".to_string(),
            "  4. Push to share with remote".to_string(),
            "".to_string(),
            "Views: Status, Log, Diff, Commit, Push, Pull, Fetch,".to_string(),
            "       Branches, Stash, Remotes, Submodules".to_string(),
        ]
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "Git".to_string(),
            description: "Git version control".to_string(),
            category: PluginCategory::Vcs,
            key: 'G',
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests;
