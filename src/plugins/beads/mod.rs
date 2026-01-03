//! Beads Plugin for R-DOS
//!
//! Provides Beads issue tracker integration as a plugin with self-contained operations.

pub mod ops;
pub mod state;

// Re-export state types for external use
#[allow(unused_imports)]
pub use state::{
    BeadsActivityEntry, BeadsComment, BeadsIssue, BeadsMenuItem, BeadsState, BeadsStats,
    BeadsSubIssue, BeadsView,
};

use super::{KeyHandleResult, Plugin, PluginCapabilities, PluginMenuItem, PluginStatusInfo};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::Frame;
use std::any::Any;
use std::path::PathBuf;
use std::process::Command;

/// Beads plugin that provides issue tracking integration
pub struct BeadsPlugin {
    /// Whether the plugin is initialized
    initialized: bool,
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
            initialized: false,
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
        self.modal_state = Some(BeadsState::new(is_beads));
    }

    /// Close the beads modal
    pub fn close_modal(&mut self) {
        self.modal_state = None;
    }

    /// Get mutable reference to modal state
    pub fn modal_state_mut(&mut self) -> Option<&mut BeadsState> {
        self.modal_state.as_mut()
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
                let items = BeadsMenuItem::items(state.is_beads_project);
                if state.menu_selected < items.len() - 1 {
                    state.menu_selected += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                let items = BeadsMenuItem::items(state.is_beads_project);
                let item = items[state.menu_selected];
                self.activate_menu_item(item, cwd);
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
            BeadsMenuItem::Sync => {
                match ops::execute_beads_sync(cwd) {
                    Ok(msg) => {
                        state.success_message = Some(msg);
                    }
                    Err(e) => {
                        state.error = Some(e);
                    }
                }
            }
            BeadsMenuItem::Human => {
                match ops::execute_beads_human(cwd) {
                    Ok(lines) => {
                        state.output_lines = lines;
                        state.scroll_offset = 0;
                        state.view = BeadsView::Human;
                    }
                    Err(e) => {
                        state.error = Some(e);
                    }
                }
            }
            BeadsMenuItem::Init => {
                match ops::execute_beads_init(cwd) {
                    Ok(msg) => {
                        state.success_message = Some(msg);
                        state.is_beads_project = true;
                        state.menu_selected = 0;
                    }
                    Err(e) => {
                        state.error = Some(e);
                    }
                }
            }
            BeadsMenuItem::Doctor => {
                match ops::execute_beads_doctor(cwd) {
                    Ok(lines) => {
                        state.output_lines = lines;
                        state.scroll_offset = 0;
                        state.view = BeadsView::Doctor;
                    }
                    Err(e) => {
                        state.error = Some(e);
                    }
                }
            }
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

    fn init(&mut self, cwd: &PathBuf) -> Result<(), String> {
        self.refresh_status(cwd);
        self.initialized = true;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), String> {
        self.initialized = false;
        Ok(())
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
            BeadsView::List | BeadsView::Ready | BeadsView::Blocked => {
                self.handle_list_key(key, cwd)
            }
            BeadsView::Stats => self.handle_stats_key(key),
            BeadsView::Detail => self.handle_detail_key(key, cwd),
            BeadsView::Human | BeadsView::Doctor => self.handle_human_doctor_key(key),
            // TODO: Add remaining view handlers
            BeadsView::Create
            | BeadsView::Edit
            | BeadsView::Comments
            | BeadsView::Dependencies
            | BeadsView::Kanban
            | BeadsView::History
            | BeadsView::FileIssues => {
                // For now, return NotHandled to let app handle these
                KeyHandleResult::NotHandled
            }
        }
    }

    fn draw_modal(&self, _frame: &mut Frame, _area: Rect) {
        // Modal drawing is done by the existing draw_beads_modal
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "B - Open Beads menu".to_string(),
            "  Issues - View and manage issues".to_string(),
            "  Create - Create new issue".to_string(),
            "  Ready - Show issues ready to work".to_string(),
            "  Stats - View project statistics".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beads_plugin_creation() {
        let plugin = BeadsPlugin::new();
        assert_eq!(plugin.id(), "beads");
        assert_eq!(plugin.name(), "Beads Issue Tracker");
        assert!(plugin.capabilities().has_menu);
        assert!(plugin.capabilities().has_status);
    }

    #[test]
    fn test_beads_plugin_menu_item() {
        let plugin = BeadsPlugin::new();
        let menu = plugin.menu_item().unwrap();
        assert_eq!(menu.key, 'B');
        assert_eq!(menu.name, "Beads");
    }
}
