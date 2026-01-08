//! Beads Plugin for R-DOS
//!
//! Provides Beads issue tracker integration as a plugin with self-contained operations.

pub mod modal;
pub mod ops;
pub mod state;

// Re-export state types for external use
#[allow(unused_imports)]
pub use state::{
    BeadsActivityEntry, BeadsComment, BeadsIssue, BeadsMenuItem, BeadsState, BeadsStats,
    BeadsSubIssue, BeadsView, KanbanSort,
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

/// Beads plugin that provides issue tracking integration
pub struct BeadsPlugin {
    /// Cached info about whether we're in a beads project
    is_beads: bool,
    /// Number of open issues
    open_count: u32,
    /// Number of in-progress issues
    in_progress_count: u32,
    /// Number of ready issues (no blockers)
    ready_count: u32,
    /// Modal state when beads modal is open (plugin owns this state)
    pub modal_state: Option<BeadsState>,
}

impl BeadsPlugin {
    pub fn new() -> Self {
        Self {
            is_beads: false,
            open_count: 0,
            in_progress_count: 0,
            ready_count: 0,
            modal_state: None,
        }
    }

    /// Open the beads modal with fresh state
    pub fn open_modal(&mut self, cwd: &PathBuf) {
        let is_beads = self.check_is_beads(cwd);
        let mut state = BeadsState::new(is_beads);
        if is_beads {
            ops::load_recent_issues(&mut state, cwd);
            ops::load_top_epics(&mut state, cwd);
        }
        self.modal_state = Some(state);
    }

    /// Close the beads modal
    pub fn close_modal(&mut self) {
        self.modal_state = None;
    }

    /// Get mutable reference to modal state
    pub fn modal_state_mut(&mut self) -> Option<&mut BeadsState> {
        self.modal_state.as_mut()
    }

    /// Handle key events for external state (for Modal::Beads delegation)
    ///
    /// This method temporarily uses the provided state for key handling,
    /// allowing the app to delegate key handling while keeping state in Modal::Beads.
    pub fn handle_external_state_key(
        &mut self,
        key: KeyEvent,
        state: &mut BeadsState,
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

    /// Check if a directory has beads initialized
    fn check_is_beads(&self, cwd: &PathBuf) -> bool {
        cwd.join(".beads").exists()
    }

    /// Update cached beads status
    fn refresh_status(&mut self, cwd: &PathBuf) {
        self.is_beads = self.check_is_beads(cwd);
        if !self.is_beads {
            self.open_count = 0;
            self.in_progress_count = 0;
            self.ready_count = 0;
            return;
        }

        // Get stats using bd stats command
        if let Ok(output) = Command::new("bd")
            .args(["stats", "--json"])
            .current_dir(cwd)
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Parse JSON for counts - simplified parsing
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                    self.open_count = json["open"].as_u64().unwrap_or(0) as u32;
                    self.in_progress_count = json["in_progress"].as_u64().unwrap_or(0) as u32;
                    self.ready_count = json["ready"].as_u64().unwrap_or(0) as u32;
                }
            }
        }
    }

    // --- Key Handlers for each Beads view ---

    fn handle_menu_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        let items = BeadsMenuItem::items(state.is_beads_project);
        let menu_count = items.len();
        let epic_count = state.top_epics.len();
        // Total navigable items: menu items + epics (if any)
        let total_items = if epic_count > 0 {
            menu_count + epic_count
        } else {
            menu_count
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
                if state.menu_selected < total_items - 1 {
                    state.menu_selected += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if state.menu_selected < menu_count {
                    // Regular menu item
                    let item = items[state.menu_selected];
                    self.activate_menu_item(item, cwd);
                } else {
                    // Epic selected - go to detail view
                    let epic_idx = state.menu_selected - menu_count;
                    if let Some(epic) = state.top_epics.get(epic_idx) {
                        let issue_id = epic.id.clone();
                        match ops::load_beads_issue_detail(&issue_id, cwd) {
                            Ok(detail) => {
                                state.detail_issue = Some(detail);
                                state.detail_scroll = 0;
                                state.view = BeadsView::Detail;
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

    fn activate_menu_item(&mut self, item: BeadsMenuItem, cwd: &PathBuf) {
        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return,
        };

        match item {
            BeadsMenuItem::List => {
                state.view = BeadsView::List;
                ops::load_beads_list(state, cwd, None);
            }
            BeadsMenuItem::Ready => {
                state.view = BeadsView::Ready;
                ops::load_beads_ready(state, cwd);
            }
            BeadsMenuItem::Blocked => {
                state.view = BeadsView::Blocked;
                ops::load_beads_blocked(state, cwd);
            }
            BeadsMenuItem::Epics => {
                state.view = BeadsView::Epics;
                ops::load_beads_epics(state, cwd);
            }
            BeadsMenuItem::Stats => {
                state.view = BeadsView::Stats;
                ops::load_beads_stats(state, cwd);
            }
            BeadsMenuItem::Create => {
                state.view = BeadsView::Create;
                state.create_title.clear();
                state.create_description.clear();
                state.create_type = 0;
                state.create_priority = 2;
                state.create_field = 0;
            }
            BeadsMenuItem::Graph => {
                state.view = BeadsView::Dependencies;
                ops::load_beads_list(state, cwd, None);
                state.selected_issue = 0;
                state.scroll_offset = 0;
            }
            BeadsMenuItem::Kanban => {
                state.view = BeadsView::Kanban;
                ops::load_beads_list(state, cwd, Some("all"));
                state.kanban_column = 0;
                state.kanban_row = 0;
            }
            BeadsMenuItem::Sync => match ops::execute_beads_sync(cwd) {
                Ok(msg) => {
                    state.success_message = Some(msg);
                }
                Err(e) => {
                    state.error = Some(e);
                }
            },
            BeadsMenuItem::Human => match ops::execute_beads_human(cwd) {
                Ok(lines) => {
                    state.output_lines = lines;
                    state.scroll_offset = 0;
                    state.view = BeadsView::Human;
                }
                Err(e) => {
                    state.error = Some(e);
                }
            },
            BeadsMenuItem::Init => match ops::execute_beads_init(cwd) {
                Ok(msg) => {
                    state.success_message = Some(msg);
                    state.is_beads_project = true;
                    state.menu_selected = 0;
                }
                Err(e) => {
                    state.error = Some(e);
                }
            },
            BeadsMenuItem::Doctor => match ops::execute_beads_doctor(cwd) {
                Ok(lines) => {
                    state.output_lines = lines;
                    state.scroll_offset = 0;
                    state.view = BeadsView::Doctor;
                }
                Err(e) => {
                    state.error = Some(e);
                }
            },
        }
    }

    fn handle_list_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        // Get issue ID first (immutable borrow) before mutable operations
        let issue_id_for_detail = if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
            self.get_filtered_issue_at_index().map(|id| id.to_string())
        } else {
            None
        };

        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        let current_view = state.view;

        // Handle search input mode
        if state.search_active {
            match key.code {
                KeyCode::Esc => {
                    state.search_active = false;
                    state.search_query.clear();
                    state.selected_issue = 0;
                }
                KeyCode::Enter => {
                    state.search_active = false;
                    state.selected_issue = 0;
                }
                KeyCode::Backspace => {
                    state.search_query.pop();
                    state.selected_issue = 0;
                }
                KeyCode::Char(c) => {
                    state.search_query.push(c);
                    state.selected_issue = 0;
                }
                _ => {}
            }
            return KeyHandleResult::Handled;
        }

        // Normal mode
        match key.code {
            KeyCode::Esc => {
                if !state.search_query.is_empty() {
                    state.search_query.clear();
                    state.selected_issue = 0;
                } else {
                    state.view = BeadsView::Menu;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('/') => {
                state.search_active = true;
                state.search_query.clear();
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if state.selected_issue > 0 {
                    state.selected_issue -= 1;
                    if state.selected_issue < state.scroll_offset {
                        state.scroll_offset = state.selected_issue;
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let query_lower = state.search_query.to_lowercase();
                let filtered_count = if state.search_query.is_empty() {
                    state.issues.len()
                } else {
                    state
                        .issues
                        .iter()
                        .filter(|i| {
                            i.id.to_lowercase().contains(&query_lower)
                                || i.title.to_lowercase().contains(&query_lower)
                                || i.issue_type.to_lowercase().contains(&query_lower)
                                || i.status.to_lowercase().contains(&query_lower)
                        })
                        .count()
                };
                if state.selected_issue + 1 < filtered_count {
                    state.selected_issue += 1;
                    let visible_height = 15;
                    if state.selected_issue >= state.scroll_offset + visible_height {
                        state.scroll_offset = state.selected_issue - visible_height + 1;
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                // Refresh current view
                match current_view {
                    BeadsView::List => ops::load_beads_list(state, cwd, None),
                    BeadsView::Ready => ops::load_beads_ready(state, cwd),
                    BeadsView::Blocked => ops::load_beads_blocked(state, cwd),
                    BeadsView::Epics => ops::load_beads_epics(state, cwd),
                    _ => {}
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                // View issue detail
                if let Some(issue_id) = issue_id_for_detail {
                    match ops::load_beads_issue_detail(&issue_id, cwd) {
                        Ok(detail) => {
                            state.detail_issue = Some(detail);
                            state.selected_subtask = 0;
                            state.detail_scroll = 0;
                            state.view = BeadsView::Detail;
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

    /// Helper to get the issue ID at the currently selected index (for borrow checker)
    fn get_filtered_issue_at_index(&self) -> Option<&str> {
        let state = self.modal_state.as_ref()?;
        let index = state.selected_issue;
        if state.search_query.is_empty() {
            state.issues.get(index).map(|i| i.id.as_str())
        } else {
            let query_lower = state.search_query.to_lowercase();
            state
                .issues
                .iter()
                .filter(|i| {
                    i.id.to_lowercase().contains(&query_lower)
                        || i.title.to_lowercase().contains(&query_lower)
                        || i.issue_type.to_lowercase().contains(&query_lower)
                        || i.status.to_lowercase().contains(&query_lower)
                })
                .nth(index)
                .map(|i| i.id.as_str())
        }
    }

    fn handle_stats_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        match key.code {
            KeyCode::Esc => {
                state.view = BeadsView::Menu;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_detail_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        match key.code {
            KeyCode::Esc => {
                state.view = BeadsView::List;
                state.detail_issue = None;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if state.selected_subtask > 0 {
                    state.selected_subtask -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(ref detail) = state.detail_issue {
                    if state.selected_subtask + 1 < detail.dependents.len() {
                        state.selected_subtask += 1;
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                // View comments - comments are already loaded in detail_issue
                state.view = BeadsView::Comments;
                state.selected_comment = 0;
                KeyHandleResult::Handled
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                // Edit issue
                if let Some(ref detail) = state.detail_issue {
                    state.edit_issue_id = detail.id.clone();
                    state.edit_title = detail.title.clone();
                    state.edit_description = detail.description.clone().unwrap_or_default();
                    state.edit_field = 0;
                    state.edit_status = match detail.status.as_str() {
                        "in_progress" => 1,
                        "closed" => 2,
                        _ => 0,
                    };
                    state.edit_priority = detail
                        .priority
                        .chars()
                        .last()
                        .and_then(|c| c.to_digit(10))
                        .unwrap_or(2) as usize;
                    state.view = BeadsView::Edit;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('h') | KeyCode::Char('H') => {
                // View history
                if let Some(ref detail) = state.detail_issue {
                    let issue_id = detail.id.clone();
                    ops::load_issue_activity(state, &issue_id, cwd);
                    state.view = BeadsView::History;
                    state.selected_activity = 0;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                // Cycle status: open -> in_progress -> closed -> open
                if let Some(ref detail) = state.detail_issue {
                    let issue_id = detail.id.clone();
                    let new_status = match detail.status.as_str() {
                        "open" => 1,        // -> in_progress
                        "in_progress" => 2, // -> closed
                        "closed" => 0,      // -> open
                        _ => 0,
                    };
                    match ops::execute_beads_update(&issue_id, None, Some(new_status), None, cwd) {
                        Ok(_) => {
                            // Reload detail
                            if let Ok(updated) = ops::load_beads_issue_detail(&issue_id, cwd) {
                                state.detail_issue = Some(updated);
                                state.success_message = Some(format!(
                                    "Status updated to {}",
                                    match new_status {
                                        0 => "open",
                                        1 => "in_progress",
                                        2 => "closed",
                                        _ => "unknown",
                                    }
                                ));
                            }
                        }
                        Err(e) => {
                            state.error = Some(e);
                        }
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                // Navigate to selected subtask/dependent issue
                if let Some(ref detail) = state.detail_issue {
                    if !detail.dependents.is_empty()
                        && state.selected_subtask < detail.dependents.len()
                    {
                        let subtask = &detail.dependents[state.selected_subtask];
                        let subtask_id = subtask.id.clone();
                        // Load the subtask's full details
                        match ops::load_beads_issue_detail(&subtask_id, cwd) {
                            Ok(subtask_detail) => {
                                state.detail_issue = Some(subtask_detail);
                                state.selected_subtask = 0;
                                state.detail_scroll = 0;
                            }
                            Err(e) => {
                                state.error = Some(e);
                            }
                        }
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::PageUp => {
                // Scroll detail view up
                if state.detail_scroll >= 5 {
                    state.detail_scroll -= 5;
                } else {
                    state.detail_scroll = 0;
                }
                KeyHandleResult::Handled
            }
            KeyCode::PageDown => {
                // Scroll detail view down
                state.detail_scroll += 5;
                KeyHandleResult::Handled
            }
            KeyCode::Home => {
                // Scroll to top
                state.detail_scroll = 0;
                KeyHandleResult::Handled
            }
            KeyCode::End => {
                // Scroll to bottom (will be clamped in render)
                state.detail_scroll = usize::MAX / 2; // Large value, will be clamped
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_human_doctor_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        match key.code {
            KeyCode::Esc => {
                state.view = BeadsView::Menu;
                state.output_lines.clear();
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
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_create_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        // Text fields: 0=title, 1=description
        // Selector fields: 2=type, 3=priority
        let in_text_field = state.create_field <= 1;

        match key.code {
            KeyCode::Esc => {
                state.view = BeadsView::Menu;
                KeyHandleResult::Handled
            }
            KeyCode::Up => {
                if state.create_field > 0 {
                    state.create_field -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                if state.create_field < 3 {
                    state.create_field += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Left => {
                match state.create_field {
                    2 => {
                        if state.create_type > 0 {
                            state.create_type -= 1;
                        }
                    }
                    3 => {
                        if state.create_priority > 0 {
                            state.create_priority -= 1;
                        }
                    }
                    _ => {}
                }
                KeyHandleResult::Handled
            }
            KeyCode::Right => {
                match state.create_field {
                    2 => {
                        if state.create_type < 2 {
                            state.create_type += 1;
                        }
                    }
                    3 => {
                        if state.create_priority < 4 {
                            state.create_priority += 1;
                        }
                    }
                    _ => {}
                }
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                match state.create_field {
                    0 => {
                        state.create_title.pop();
                    }
                    1 => {
                        state.create_description.pop();
                    }
                    _ => {}
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('k') if !in_text_field => {
                if state.create_field > 0 {
                    state.create_field -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('j') if !in_text_field => {
                if state.create_field < 3 {
                    state.create_field += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('h') if !in_text_field => {
                match state.create_field {
                    2 => {
                        if state.create_type > 0 {
                            state.create_type -= 1;
                        }
                    }
                    3 => {
                        if state.create_priority > 0 {
                            state.create_priority -= 1;
                        }
                    }
                    _ => {}
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('l') if !in_text_field => {
                match state.create_field {
                    2 => {
                        if state.create_type < 2 {
                            state.create_type += 1;
                        }
                    }
                    3 => {
                        if state.create_priority < 4 {
                            state.create_priority += 1;
                        }
                    }
                    _ => {}
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                match state.create_field {
                    0 => {
                        state.create_title.push(c);
                    }
                    1 => {
                        state.create_description.push(c);
                    }
                    _ => {}
                }
                KeyHandleResult::Handled
            }
            KeyCode::Tab => {
                state.create_field = (state.create_field + 1) % 4;
                KeyHandleResult::Handled
            }
            KeyCode::BackTab => {
                if state.create_field > 0 {
                    state.create_field -= 1;
                } else {
                    state.create_field = 3;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                // In description field, Enter adds newline
                if state.create_field == 1 {
                    state.create_description.push('\n');
                    KeyHandleResult::Handled
                }
                // In title field, Enter moves to next field
                else if state.create_field == 0 {
                    state.create_field = 1;
                    KeyHandleResult::Handled
                }
                // In type/priority fields, Enter submits the form
                else if !state.create_title.is_empty() {
                    let title = state.create_title.clone();
                    let description = state.create_description.clone();
                    let issue_type = state.create_type;
                    let priority = state.create_priority;
                    let parent_id = state.subtask_parent_id.clone();

                    let result = if parent_id.is_empty() {
                        ops::execute_beads_create(&title, &description, issue_type, priority, cwd)
                            .map(|_| "Issue created".to_string())
                    } else {
                        ops::execute_beads_create_subtask(
                            &parent_id,
                            &title,
                            &description,
                            issue_type,
                            priority,
                            cwd,
                        )
                        .map(|id| format!("Subtask {} created", id))
                    };

                    match result {
                        Ok(msg) => {
                            // Clear create form
                            state.create_title.clear();
                            state.create_description.clear();
                            state.create_field = 0;
                            state.create_type = 0;
                            state.create_priority = 2;
                            state.subtask_parent_id.clear();
                            // Show success message and return to list
                            state.success_message = Some(msg);
                            ops::load_beads_list(state, cwd, None);
                            state.view = BeadsView::List;
                            KeyHandleResult::Handled
                        }
                        Err(e) => {
                            state.error = Some(e);
                            KeyHandleResult::Handled
                        }
                    }
                } else {
                    KeyHandleResult::Handled
                }
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_edit_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        let in_text_field = state.edit_field == 0 || state.edit_field == 1;

        match key.code {
            KeyCode::Esc => {
                state.view = BeadsView::Detail;
                KeyHandleResult::Handled
            }
            KeyCode::Up => {
                if state.edit_field > 0 {
                    state.edit_field -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                if state.edit_field < 3 {
                    state.edit_field += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Left => {
                match state.edit_field {
                    2 => {
                        if state.edit_status > 0 {
                            state.edit_status -= 1;
                        }
                    }
                    3 => {
                        if state.edit_priority > 0 {
                            state.edit_priority -= 1;
                        }
                    }
                    _ => {}
                }
                KeyHandleResult::Handled
            }
            KeyCode::Right => {
                match state.edit_field {
                    2 => {
                        if state.edit_status < 2 {
                            state.edit_status += 1;
                        }
                    }
                    3 => {
                        if state.edit_priority < 4 {
                            state.edit_priority += 1;
                        }
                    }
                    _ => {}
                }
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                match state.edit_field {
                    0 => {
                        state.edit_title.pop();
                    }
                    1 => {
                        state.edit_description.pop();
                    }
                    _ => {}
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('k') if !in_text_field => {
                if state.edit_field > 0 {
                    state.edit_field -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('j') if !in_text_field => {
                if state.edit_field < 3 {
                    state.edit_field += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                match state.edit_field {
                    0 => {
                        state.edit_title.push(c);
                    }
                    1 => {
                        state.edit_description.push(c);
                    }
                    _ => {}
                }
                KeyHandleResult::Handled
            }
            KeyCode::Tab => {
                state.edit_field = (state.edit_field + 1) % 4;
                KeyHandleResult::Handled
            }
            KeyCode::BackTab => {
                if state.edit_field > 0 {
                    state.edit_field -= 1;
                } else {
                    state.edit_field = 3;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                // In description, add newline
                if state.edit_field == 1 {
                    state.edit_description.push('\n');
                    return KeyHandleResult::Handled;
                }
                // In title, move to next field
                if state.edit_field == 0 {
                    state.edit_field = 1;
                    return KeyHandleResult::Handled;
                }
                // Submit update
                let issue_id = state.edit_issue_id.clone();
                let title = state.edit_title.clone();
                let status = Some(state.edit_status);
                let priority = Some(state.edit_priority);

                match ops::execute_beads_update(
                    &issue_id,
                    Some(title.as_str()),
                    status,
                    priority,
                    cwd,
                ) {
                    Ok(_) => {
                        // Reload detail
                        if let Ok(detail) = ops::load_beads_issue_detail(&issue_id, cwd) {
                            state.detail_issue = Some(detail);
                        }
                        state.view = BeadsView::Detail;
                        KeyHandleResult::Handled
                    }
                    Err(e) => {
                        state.error = Some(e);
                        KeyHandleResult::Handled
                    }
                }
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_kanban_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        use state::KanbanSort;

        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        // Build and sort column lists
        let sort_fn = |a: &&BeadsIssue, b: &&BeadsIssue| -> std::cmp::Ordering {
            match state.kanban_sort {
                KanbanSort::PriorityAsc => a.priority.cmp(&b.priority),
                KanbanSort::PriorityDesc => b.priority.cmp(&a.priority),
                KanbanSort::TitleAsc => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
                KanbanSort::TitleDesc => b.title.to_lowercase().cmp(&a.title.to_lowercase()),
                KanbanSort::IdAsc => a.id.cmp(&b.id),
                KanbanSort::IdDesc => b.id.cmp(&a.id),
            }
        };

        let mut open_issues: Vec<_> = state.issues.iter().filter(|i| i.status == "open").collect();
        open_issues.sort_by(sort_fn);

        let mut in_progress_issues: Vec<_> = state
            .issues
            .iter()
            .filter(|i| i.status == "in_progress")
            .collect();
        in_progress_issues.sort_by(sort_fn);

        let mut closed_issues: Vec<_> = state
            .issues
            .iter()
            .filter(|i| i.status == "closed")
            .collect();
        closed_issues.sort_by(sort_fn);

        let columns = [&open_issues, &in_progress_issues, &closed_issues];
        let current_col = &columns[state.kanban_column];

        match key.code {
            KeyCode::Esc => {
                state.view = BeadsView::Menu;
                KeyHandleResult::Handled
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if state.kanban_column > 0 {
                    state.kanban_column -= 1;
                    // Clamp row to new column length
                    let new_col = &columns[state.kanban_column];
                    if state.kanban_row >= new_col.len() {
                        state.kanban_row = new_col.len().saturating_sub(1);
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if state.kanban_column < 2 {
                    state.kanban_column += 1;
                    let new_col = &columns[state.kanban_column];
                    if state.kanban_row >= new_col.len() {
                        state.kanban_row = new_col.len().saturating_sub(1);
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if state.kanban_row > 0 {
                    state.kanban_row -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.kanban_row + 1 < current_col.len() {
                    state.kanban_row += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                ops::load_beads_list(state, cwd, Some("all"));
                state.kanban_row = 0;
                KeyHandleResult::Handled
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                // View detail of selected issue
                if let Some(issue) = current_col.get(state.kanban_row) {
                    let issue_id = issue.id.clone();
                    match ops::load_beads_issue_detail(&issue_id, cwd) {
                        Ok(detail) => {
                            state.detail_issue = Some(detail);
                            state.selected_subtask = 0;
                            state.view = BeadsView::Detail;
                        }
                        Err(e) => {
                            state.error = Some(e);
                        }
                    }
                }
                KeyHandleResult::Handled
            }
            // Move issue left (to previous status)
            KeyCode::Char('<') | KeyCode::Char(',') => {
                if let Some(issue) = current_col.get(state.kanban_row) {
                    let issue_id = issue.id.clone();
                    let new_status = match state.kanban_column {
                        1 => Some(0), // in_progress -> open
                        2 => Some(1), // closed -> in_progress
                        _ => None,
                    };
                    if let Some(status_idx) = new_status {
                        match ops::execute_beads_update(
                            &issue_id,
                            None,
                            Some(status_idx),
                            None,
                            cwd,
                        ) {
                            Ok(_) => {
                                // Refresh kanban and stay in same column
                                ops::load_beads_list(state, cwd, Some("all"));
                                state.kanban_row = 0;
                                state.success_message =
                                    Some(format!("Moved {} to left column", issue_id));
                            }
                            Err(e) => {
                                state.error = Some(e);
                            }
                        }
                    }
                }
                KeyHandleResult::Handled
            }
            // Move issue right (to next status)
            KeyCode::Char('>') | KeyCode::Char('.') => {
                if let Some(issue) = current_col.get(state.kanban_row) {
                    let issue_id = issue.id.clone();
                    let new_status = match state.kanban_column {
                        0 => Some(1), // open -> in_progress
                        1 => Some(2), // in_progress -> closed
                        _ => None,
                    };
                    if let Some(status_idx) = new_status {
                        match ops::execute_beads_update(
                            &issue_id,
                            None,
                            Some(status_idx),
                            None,
                            cwd,
                        ) {
                            Ok(_) => {
                                // Refresh kanban and stay in same column
                                ops::load_beads_list(state, cwd, Some("all"));
                                state.kanban_row = 0;
                                state.success_message =
                                    Some(format!("Moved {} to right column", issue_id));
                            }
                            Err(e) => {
                                state.error = Some(e);
                            }
                        }
                    }
                }
                KeyHandleResult::Handled
            }
            // Cycle sort mode
            KeyCode::Char('s') | KeyCode::Char('S') => {
                state.kanban_sort = state.kanban_sort.next();
                state.kanban_row = 0;
                state.success_message = Some(format!("Sort: {}", state.kanban_sort.as_str()));
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_dependencies_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        match key.code {
            KeyCode::Esc => {
                state.view = BeadsView::Menu;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if state.selected_issue > 0 {
                    state.selected_issue -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.selected_issue + 1 < state.issues.len() {
                    state.selected_issue += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(issue) = state.issues.get(state.selected_issue) {
                    let issue_id = issue.id.clone();
                    match ops::load_beads_issue_detail(&issue_id, cwd) {
                        Ok(detail) => {
                            state.detail_issue = Some(detail);
                            state.selected_subtask = 0;
                            state.view = BeadsView::Detail;
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

    fn handle_comments_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        if state.comment_input_active {
            match key.code {
                KeyCode::Esc => {
                    state.comment_input_active = false;
                    state.comment_input.clear();
                    KeyHandleResult::Handled
                }
                KeyCode::Enter => {
                    if !state.comment_input.is_empty() {
                        if let Some(ref detail) = state.detail_issue {
                            let issue_id = detail.id.clone();
                            let comment_text = state.comment_input.clone();
                            match ops::execute_beads_add_comment(&issue_id, &comment_text, cwd) {
                                Ok(_) => {
                                    state.comment_input.clear();
                                    state.comment_input_active = false;
                                    // Reload detail to get new comments
                                    if let Ok(detail) = ops::load_beads_issue_detail(&issue_id, cwd)
                                    {
                                        state.detail_issue = Some(detail);
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
                KeyCode::Backspace => {
                    state.comment_input.pop();
                    KeyHandleResult::Handled
                }
                KeyCode::Char(c) => {
                    state.comment_input.push(c);
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            }
        } else {
            match key.code {
                KeyCode::Esc => {
                    state.view = BeadsView::Detail;
                    KeyHandleResult::Handled
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if state.selected_comment > 0 {
                        state.selected_comment -= 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(ref detail) = state.detail_issue {
                        if state.selected_comment + 1 < detail.comments.len() {
                            state.selected_comment += 1;
                        }
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    state.comment_input_active = true;
                    state.comment_input.clear();
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            }
        }
    }

    fn handle_history_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        let state = match self.modal_state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        match key.code {
            KeyCode::Esc => {
                state.view = BeadsView::Detail;
                state.activity_entries.clear();
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if state.selected_activity > 0 {
                    state.selected_activity -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.selected_activity + 1 < state.activity_entries.len() {
                    state.selected_activity += 1;
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    /// Helper to get filtered issue at index
    fn get_filtered_issue_at(&self, index: usize) -> Option<&BeadsIssue> {
        let state = self.modal_state.as_ref()?;
        if state.search_query.is_empty() {
            state.issues.get(index)
        } else {
            let query_lower = state.search_query.to_lowercase();
            state
                .issues
                .iter()
                .filter(|i| {
                    i.id.to_lowercase().contains(&query_lower)
                        || i.title.to_lowercase().contains(&query_lower)
                        || i.issue_type.to_lowercase().contains(&query_lower)
                        || i.status.to_lowercase().contains(&query_lower)
                })
                .nth(index)
        }
    }
}

impl Default for BeadsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for BeadsPlugin {
    fn id(&self) -> &str {
        "beads"
    }

    fn name(&self) -> &str {
        "Beads Issue Tracker"
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
        self.check_is_beads(cwd)
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "Beads".to_string(),
            key: 'B',
            description: "Beads issue tracker menu".to_string(),
            priority: 20, // Show after Git
        })
    }

    fn status_info(&self, cwd: &PathBuf) -> Option<PluginStatusInfo> {
        if !self.check_is_beads(cwd) {
            return None;
        }

        // Build status text similar to existing format: "bd: ○19 ●3 ✓12"
        let mut parts = Vec::new();
        if self.open_count > 0 {
            parts.push(format!("○{}", self.open_count));
        }
        if self.in_progress_count > 0 {
            parts.push(format!("●{}", self.in_progress_count));
        }
        if self.ready_count > 0 {
            parts.push(format!("✓{}", self.ready_count));
        }

        let text = if parts.is_empty() {
            "bd: ✓".to_string() // All clear
        } else {
            format!("bd: {}", parts.join(" "))
        };

        Some(PluginStatusInfo { text, active: true })
    }

    fn handle_global_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        match key.code {
            KeyCode::Char('b') | KeyCode::Char('B') => {
                // Open beads modal with plugin-owned state
                self.open_modal(cwd);
                KeyHandleResult::OpenModal
            }
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let state = match self.modal_state.as_ref() {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        // Not a beads project - any key closes
        if !state.is_beads_project {
            match key.code {
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ') => {
                    self.close_modal();
                    return KeyHandleResult::CloseModal;
                }
                _ => return KeyHandleResult::Handled,
            }
        }

        let view = state.view;
        match view {
            BeadsView::Menu => self.handle_menu_key(key, cwd),
            BeadsView::List | BeadsView::Ready | BeadsView::Blocked | BeadsView::Epics => {
                self.handle_list_key(key, cwd)
            }
            BeadsView::Stats => self.handle_stats_key(key),
            BeadsView::Detail => self.handle_detail_key(key, cwd),
            BeadsView::Human | BeadsView::Doctor => self.handle_human_doctor_key(key),
            BeadsView::Kanban => self.handle_kanban_key(key, cwd),
            BeadsView::Create => self.handle_create_key(key, cwd),
            BeadsView::Edit => self.handle_edit_key(key, cwd),
            BeadsView::Comments => self.handle_comments_key(key, cwd),
            BeadsView::Dependencies => self.handle_dependencies_key(key, cwd),
            BeadsView::History => self.handle_history_key(key),
            BeadsView::FileIssues => {
                // For now, return NotHandled to let app handle these
                KeyHandleResult::NotHandled
            }
        }
    }

    fn draw_modal(&self, _frame: &mut Frame, _area: Rect, _colors: &crate::app::ThemeColors) {
        // Modal drawing is done by the existing draw_beads_modal
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "        B -- BEADS ISSUE TRACKER".to_string(),
            "".to_string(),
            "Purpose:   Git-native issue tracking stored in .beads/ folder.".to_string(),
            "           Issues sync with your repository automatically.".to_string(),
            "".to_string(),
            "To use:    Press B to open Beads menu. Only available in".to_string(),
            "           directories with a .beads folder (beads projects).".to_string(),
            "".to_string(),
            "Status bar shows: open issue count when in a beads project.".to_string(),
            "".to_string(),
            "Navigation:".to_string(),
            "  Tab       Switch between views".to_string(),
            "  Enter     View issue details or confirm".to_string(),
            "  Esc       Go back or close".to_string(),
            "  ↑/↓       Navigate issue list".to_string(),
            "  C         Create new issue".to_string(),
            "".to_string(),
            "Issue workflow:".to_string(),
            "  1. Use Ready to find available work".to_string(),
            "  2. Update status to in_progress".to_string(),
            "  3. Complete work, close issue".to_string(),
            "  4. Sync to push changes".to_string(),
            "".to_string(),
            "Views: List, Ready, Blocked, Create, Show, Kanban, Stats".to_string(),
            "".to_string(),
            "Tip: Issues support dependencies - use Blocked view to see".to_string(),
            "     which issues are waiting on others.".to_string(),
        ]
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "Beads".to_string(),
            description: "Issue tracker".to_string(),
            category: PluginCategory::Vcs,
            key: 'B',
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
