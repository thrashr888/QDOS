//! Jj (Jujutsu) VCS Plugin for R-DOS
//!
//! Provides integration with the Jujutsu version control system.

mod modal;
mod ops;
mod state;

pub use state::{JjMenuItem, JjState, JjView};

use super::{KeyHandleResult, Plugin, PluginCapabilities, PluginMenuItem, PluginStatusInfo};
use crate::app::ThemeColors;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::Frame;
use state::GitAction;
use std::any::Any;
use std::path::PathBuf;

/// Jj plugin for Jujutsu VCS integration
pub struct JjPlugin {
    /// Cached change ID for status bar
    change_id: Option<String>,
    /// Cached has_changes flag
    has_changes: bool,
    /// Modal state
    pub modal_state: Option<JjState>,
}

impl JjPlugin {
    pub fn new() -> Self {
        Self {
            change_id: None,
            has_changes: false,
            modal_state: None,
        }
    }

    pub fn open_modal(&mut self, cwd: &PathBuf) {
        let is_repo = ops::is_jj_repo(cwd);
        let mut state = JjState::new(is_repo);

        if is_repo {
            // Load initial status
            if let Ok((wc, parent, files)) = ops::load_jj_status(cwd) {
                state.working_copy = wc;
                state.parent = parent;
                state.files = files;
            }
        }

        self.modal_state = Some(state);
    }

    pub fn close_modal(&mut self) {
        self.modal_state = None;
    }

    fn handle_menu_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let Some(ref mut state) = self.modal_state else {
            return KeyHandleResult::CloseModal;
        };

        match key.code {
            KeyCode::Esc => KeyHandleResult::CloseModal,
            KeyCode::Up | KeyCode::Char('k') => {
                if state.menu_selected > 0 {
                    state.menu_selected -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.menu_selected < JjMenuItem::ALL.len() - 1 {
                    state.menu_selected += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                let item = JjMenuItem::ALL[state.menu_selected];
                match item {
                    JjMenuItem::Status => {
                        if let Ok((wc, parent, files)) = ops::load_jj_status(cwd) {
                            state.working_copy = wc;
                            state.parent = parent;
                            state.files = files;
                        }
                        state.view = JjView::Status;
                    }
                    JjMenuItem::Log => {
                        match ops::load_jj_log(cwd) {
                            Ok(changes) => state.changes = changes,
                            Err(e) => state.error = Some(e),
                        }
                        state.view = JjView::Log;
                    }
                    JjMenuItem::Diff => {
                        match ops::load_jj_diff(cwd) {
                            Ok(diff) => state.diff_content = diff,
                            Err(e) => state.error = Some(e),
                        }
                        state.prev_view = Some(JjView::Menu);
                        state.view = JjView::Diff;
                    }
                    JjMenuItem::Describe => {
                        if let Ok((wc, _, _)) = ops::load_jj_status(cwd) {
                            state.working_copy = wc;
                            if let Some(ref wc) = state.working_copy {
                                state.description_input =
                                    if wc.description == "(no description set)" {
                                        String::new()
                                    } else {
                                        wc.description.clone()
                                    };
                            }
                        }
                        state.view = JjView::Describe;
                    }
                    JjMenuItem::New => {
                        match ops::create_new_change(cwd) {
                            Ok(()) => {
                                // Refresh status after creating new change
                                if let Ok((wc, parent, files)) = ops::load_jj_status(cwd) {
                                    state.working_copy = wc;
                                    state.parent = parent;
                                    state.files = files;
                                }
                                state.view = JjView::Status;
                            }
                            Err(e) => state.error = Some(e),
                        }
                    }
                    JjMenuItem::Bookmark => {
                        match ops::load_bookmarks(cwd) {
                            Ok(bookmarks) => state.bookmarks = bookmarks,
                            Err(e) => state.error = Some(e),
                        }
                        state.view = JjView::Bookmark;
                    }
                    JjMenuItem::Operations => {
                        match ops::load_operations(cwd) {
                            Ok(ops) => state.operations = ops,
                            Err(e) => state.error = Some(e),
                        }
                        state.view = JjView::Operations;
                    }
                    JjMenuItem::Git => {
                        state.view = JjView::Git;
                    }
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_status_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let Some(ref mut state) = self.modal_state else {
            return KeyHandleResult::CloseModal;
        };

        match key.code {
            KeyCode::Esc => {
                state.view = JjView::Menu;
                KeyHandleResult::Handled
            }
            KeyCode::Char('d') => {
                match ops::load_jj_diff(cwd) {
                    Ok(diff) => state.diff_content = diff,
                    Err(e) => state.error = Some(e),
                }
                state.prev_view = Some(JjView::Status);
                state.view = JjView::Diff;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_log_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let Some(ref mut state) = self.modal_state else {
            return KeyHandleResult::CloseModal;
        };

        match key.code {
            KeyCode::Esc => {
                state.view = JjView::Menu;
                state.scroll_offset = 0;
                state.selected_change = 0;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if state.selected_change > 0 {
                    state.selected_change -= 1;
                    if state.selected_change < state.scroll_offset {
                        state.scroll_offset = state.selected_change;
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.selected_change < state.changes.len().saturating_sub(1) {
                    state.selected_change += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                if !state.changes.is_empty() {
                    let change_id = &state.changes[state.selected_change].change_id;
                    match ops::load_change_diff(cwd, change_id) {
                        Ok(diff) => state.diff_content = diff,
                        Err(e) => state.error = Some(e),
                    }
                    state.prev_view = Some(JjView::Log);
                    state.view = JjView::Diff;
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_diff_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        let Some(ref mut state) = self.modal_state else {
            return KeyHandleResult::CloseModal;
        };

        match key.code {
            KeyCode::Esc => {
                state.view = state.prev_view.unwrap_or(JjView::Menu);
                state.scroll_offset = 0;
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
                if state.scroll_offset < state.diff_content.len().saturating_sub(1) {
                    state.scroll_offset += 1;
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_describe_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let Some(ref mut state) = self.modal_state else {
            return KeyHandleResult::CloseModal;
        };

        if state.input_mode {
            match key.code {
                KeyCode::Esc => {
                    state.input_mode = false;
                    state.description_input.clear();
                    KeyHandleResult::Handled
                }
                KeyCode::Enter => {
                    if !state.description_input.is_empty() {
                        match ops::describe_change(cwd, &state.description_input) {
                            Ok(()) => {
                                // Refresh working copy info
                                if let Ok((wc, _, _)) = ops::load_jj_status(cwd) {
                                    state.working_copy = wc;
                                }
                            }
                            Err(e) => state.error = Some(e),
                        }
                    }
                    state.input_mode = false;
                    state.description_input.clear();
                    KeyHandleResult::Handled
                }
                KeyCode::Backspace => {
                    state.description_input.pop();
                    KeyHandleResult::Handled
                }
                KeyCode::Char(c) => {
                    state.description_input.push(c);
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            }
        } else {
            match key.code {
                KeyCode::Esc => {
                    state.view = JjView::Menu;
                    KeyHandleResult::Handled
                }
                KeyCode::Enter => {
                    state.input_mode = true;
                    if let Some(ref wc) = state.working_copy {
                        if wc.description != "(no description set)" {
                            state.description_input = wc.description.clone();
                        }
                    }
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            }
        }
    }

    fn handle_bookmark_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let Some(ref mut state) = self.modal_state else {
            return KeyHandleResult::CloseModal;
        };

        if state.bookmark_input_mode {
            match key.code {
                KeyCode::Esc => {
                    state.bookmark_input_mode = false;
                    state.bookmark_input.clear();
                    KeyHandleResult::Handled
                }
                KeyCode::Enter => {
                    if !state.bookmark_input.is_empty() {
                        match ops::create_bookmark(cwd, &state.bookmark_input) {
                            Ok(()) => {
                                // Refresh bookmarks
                                if let Ok(bookmarks) = ops::load_bookmarks(cwd) {
                                    state.bookmarks = bookmarks;
                                }
                            }
                            Err(e) => state.error = Some(e),
                        }
                    }
                    state.bookmark_input_mode = false;
                    state.bookmark_input.clear();
                    KeyHandleResult::Handled
                }
                KeyCode::Backspace => {
                    state.bookmark_input.pop();
                    KeyHandleResult::Handled
                }
                KeyCode::Char(c) => {
                    state.bookmark_input.push(c);
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            }
        } else {
            match key.code {
                KeyCode::Esc => {
                    state.view = JjView::Menu;
                    state.selected_bookmark = 0;
                    KeyHandleResult::Handled
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if state.selected_bookmark > 0 {
                        state.selected_bookmark -= 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if state.selected_bookmark < state.bookmarks.len().saturating_sub(1) {
                        state.selected_bookmark += 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Char('n') => {
                    state.bookmark_input_mode = true;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('d') => {
                    if !state.bookmarks.is_empty() {
                        let bookmark = &state.bookmarks[state.selected_bookmark];
                        if !bookmark.is_remote {
                            match ops::delete_bookmark(cwd, &bookmark.name) {
                                Ok(()) => {
                                    if let Ok(bookmarks) = ops::load_bookmarks(cwd) {
                                        state.bookmarks = bookmarks;
                                    }
                                    if state.selected_bookmark >= state.bookmarks.len()
                                        && state.selected_bookmark > 0
                                    {
                                        state.selected_bookmark -= 1;
                                    }
                                }
                                Err(e) => state.error = Some(e),
                            }
                        }
                    }
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            }
        }
    }

    fn handle_operations_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let Some(ref mut state) = self.modal_state else {
            return KeyHandleResult::CloseModal;
        };

        match key.code {
            KeyCode::Esc => {
                state.view = JjView::Menu;
                state.selected_operation = 0;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if state.selected_operation > 0 {
                    state.selected_operation -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.selected_operation < state.operations.len().saturating_sub(1) {
                    state.selected_operation += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('u') => {
                match ops::undo_operation(cwd) {
                    Ok(()) => {
                        // Refresh operations
                        if let Ok(ops) = ops::load_operations(cwd) {
                            state.operations = ops;
                        }
                    }
                    Err(e) => state.error = Some(e),
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_git_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let Some(ref mut state) = self.modal_state else {
            return KeyHandleResult::CloseModal;
        };

        match key.code {
            KeyCode::Esc => {
                state.view = JjView::Menu;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('f') | KeyCode::Char('F') => {
                state.git_action = GitAction::Fetch;
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('p') | KeyCode::Char('P') => {
                state.git_action = GitAction::Push;
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                let result = match state.git_action {
                    GitAction::Fetch => ops::git_fetch(cwd),
                    GitAction::Push => ops::git_push(cwd),
                };
                match result {
                    Ok(_) => {
                        // Success - could show message
                    }
                    Err(e) => state.error = Some(e),
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }
}

impl Default for JjPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for JjPlugin {
    fn id(&self) -> &str {
        "jj"
    }

    fn name(&self) -> &str {
        "Jujutsu VCS"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: true,
            has_keys: true,
            has_modal: true,
            has_status: true,
            has_cli: false,
            has_help: true,
        }
    }

    fn init(&mut self, cwd: &PathBuf) -> Result<(), String> {
        if let Some((change_id, has_changes)) = ops::get_jj_status_info(cwd) {
            self.change_id = Some(change_id);
            self.has_changes = has_changes;
        }
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), String> {
        self.modal_state = None;
        Ok(())
    }

    fn is_available(&self, cwd: &PathBuf) -> bool {
        ops::is_jj_repo(cwd) && ops::is_jj_available()
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "Jj".to_string(),
            key: 'J',
            description: "Jujutsu VCS operations".to_string(),
            priority: 35, // After Git (30)
        })
    }

    fn status_info(&self, _cwd: &PathBuf) -> Option<PluginStatusInfo> {
        self.change_id.as_ref().map(|id| {
            let text = if self.has_changes {
                format!("jj:{} *", id)
            } else {
                format!("jj:{}", id)
            };
            PluginStatusInfo { text, active: true }
        })
    }

    fn handle_global_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        // 'J' opens jj modal (only from main view, not in modals)
        if key.code == KeyCode::Char('J') && self.is_available(cwd) {
            self.open_modal(cwd);
            return KeyHandleResult::OpenModal;
        }
        KeyHandleResult::NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let Some(ref state) = self.modal_state else {
            return KeyHandleResult::CloseModal;
        };

        // Clear error on any key
        if state.error.is_some() {
            if let Some(ref mut s) = self.modal_state {
                s.error = None;
            }
            return KeyHandleResult::Handled;
        }

        match state.view {
            JjView::Menu => self.handle_menu_key(key, cwd),
            JjView::Status => self.handle_status_key(key, cwd),
            JjView::Log => self.handle_log_key(key, cwd),
            JjView::Diff => self.handle_diff_key(key, cwd),
            JjView::Describe => self.handle_describe_key(key, cwd),
            JjView::Bookmark => self.handle_bookmark_key(key, cwd),
            JjView::Operations => self.handle_operations_key(key, cwd),
            JjView::Git => self.handle_git_key(key, cwd),
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        if let Some(ref state) = self.modal_state {
            modal::draw_jj_modal(frame, area, state, colors);
        }
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "           J -- JUJUTSU VCS".to_string(),
            "".to_string(),
            "Purpose:   Jujutsu (jj) is a modern version control system that".to_string(),
            "           tracks changes automatically without staging. Every".to_string(),
            "           edit creates a new change that can be described later.".to_string(),
            "".to_string(),
            "To use:    Press J to open the Jujutsu menu. Only available in".to_string(),
            "           directories with a .jj folder (jj repositories).".to_string(),
            "".to_string(),
            "Key concepts:".to_string(),
            "  - Changes: Like commits, but mutable until pushed".to_string(),
            "  - Working copy (@): Your current change, auto-updated".to_string(),
            "  - Bookmarks: Named references (like git branches)".to_string(),
            "  - Operations: Every action is recorded and undoable".to_string(),
            "".to_string(),
            "Navigation:".to_string(),
            "  Tab       Switch between views".to_string(),
            "  Enter     Select item or confirm action".to_string(),
            "  Esc       Go back or close".to_string(),
            "  ↑/↓       Navigate lists".to_string(),
            "".to_string(),
            "Common workflow:".to_string(),
            "  1. Edit files (changes tracked automatically)".to_string(),
            "  2. Use Describe to add a message".to_string(),
            "  3. Use New to start fresh change".to_string(),
            "  4. Use Git > Push to share".to_string(),
            "".to_string(),
            "Tip: Use Operations > Undo to reverse any jj action.".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
