//! Redis plugin - Key-value browser
//!
//! Provides Redis key browsing and management with connection profiles.

mod modal;
mod ops;
mod state;

use crate::app::ThemeColors;
use crate::plugins::{AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, Frame};
use std::any::Any;
use std::path::PathBuf;

pub use state::RedisState;

/// Redis plugin
pub struct RedisPlugin {
    state: RedisState,
    client: ops::RedisClient,
    modal_open: bool,
}

impl RedisPlugin {
    pub fn new() -> Self {
        Self {
            state: RedisState::new(),
            client: ops::RedisClient::new(),
            modal_open: false,
        }
    }

    /// Connect to Redis
    fn do_connect(&mut self) {
        self.state.set_loading("Connecting...");

        match self.client.connect(&self.state.connection) {
            Ok(()) => {
                self.state.connected = true;
                self.state.view = state::RedisView::KeyBrowser;
                self.state.clear_loading();
                self.do_scan();
            }
            Err(e) => {
                self.state.error = Some(e);
                self.state.clear_loading();
            }
        }
    }

    /// Scan keys
    fn do_scan(&mut self) {
        self.state.set_loading("Scanning keys...");

        match self.client.scan_keys(self.state.scan_cursor, "", 100) {
            Ok((cursor, keys)) => {
                self.state.keys.extend(keys);
                self.state.scan_cursor = cursor;
                self.state.scan_complete = cursor == 0;
                self.state.apply_filter();
            }
            Err(e) => {
                self.state.error = Some(e);
            }
        }

        self.state.clear_loading();
    }

    /// Load more keys
    fn load_more(&mut self) {
        if !self.state.scan_complete && !self.state.loading {
            self.do_scan();
        }
    }

    /// View key detail
    fn view_key(&mut self) {
        if let Some(key) = self.state.selected_key().cloned() {
            self.state.set_loading("Loading value...");
            match self.client.get_value(&key) {
                Ok(value) => {
                    self.state.current_key = Some(key);
                    self.state.current_value = value;
                    self.state.detail_scroll = 0;
                    self.state.view = state::RedisView::KeyDetail;
                }
                Err(e) => {
                    self.state.error = Some(e);
                }
            }
            self.state.clear_loading();
        }
    }

    /// Delete key
    fn delete_key(&mut self, name: &str) {
        match self.client.delete_key(name) {
            Ok(()) => {
                self.state.message = Some(format!("Deleted key: {}", name));
                // Remove from local list
                self.state.keys.retain(|k| k.name != name);
                self.state.apply_filter();
            }
            Err(e) => {
                self.state.error = Some(e);
            }
        }
    }

    /// Get server info
    fn get_info(&mut self) {
        self.state.set_loading("Loading info...");
        match self.client.get_info() {
            Ok(info) => {
                self.state.server_info = info;
                self.state.info_scroll = 0;
                self.state.view = state::RedisView::ServerInfo;
            }
            Err(e) => {
                self.state.error = Some(e);
            }
        }
        self.state.clear_loading();
    }

    /// Handle connect form keys
    fn handle_connect_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Tab | KeyCode::Down => {
                self.state.connect_field = self.state.connect_field.next();
                self.state.reset_cursor_for_field();
                KeyHandleResult::Handled
            }
            KeyCode::BackTab | KeyCode::Up => {
                self.state.connect_field = self.state.connect_field.prev();
                self.state.reset_cursor_for_field();
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                self.do_connect();
                KeyHandleResult::Handled
            }
            KeyCode::Char('p') if self.state.connect_field != state::ConnectField::Password => {
                self.state.view = state::RedisView::Profiles;
                KeyHandleResult::Handled
            }
            KeyCode::Char(' ') if self.state.connect_field == state::ConnectField::Tls => {
                self.state.connection.tls = !self.state.connection.tls;
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                self.state.insert_connect_char(c);
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                self.state.backspace_connect();
                KeyHandleResult::Handled
            }
            KeyCode::Esc => {
                self.state.reset();
                self.modal_open = false;
                KeyHandleResult::CloseModal
            }
            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle profiles view keys
    fn handle_profiles_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Up => {
                if self.state.selected_profile > 0 {
                    self.state.selected_profile -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                if self.state.selected_profile < self.state.profiles.len().saturating_sub(1) {
                    self.state.selected_profile += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                if let Some(profile) = self
                    .state
                    .profiles
                    .get(self.state.selected_profile)
                    .cloned()
                {
                    self.state.connection = profile;
                    self.do_connect();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('d') => {
                // Delete profile
                if !self.state.profiles.is_empty() {
                    self.state.profiles.remove(self.state.selected_profile);
                    if self.state.selected_profile >= self.state.profiles.len()
                        && self.state.selected_profile > 0
                    {
                        self.state.selected_profile -= 1;
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Esc => {
                self.state.view = state::RedisView::Connect;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle key browser keys
    fn handle_browser_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Up => {
                self.state.move_up();
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                self.state.move_down();
                // Load more if near end
                let visible = self.state.visible_keys();
                if self.state.selected_key >= visible.len().saturating_sub(10) {
                    self.load_more();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                self.view_key();
                KeyHandleResult::Handled
            }
            KeyCode::Char('/') => {
                // Filter mode - already typing in filter
                KeyHandleResult::Handled
            }
            KeyCode::Char('T') => {
                self.state.type_filter = self.state.type_filter.next();
                self.state.apply_filter();
                KeyHandleResult::Handled
            }
            KeyCode::Char('d') => {
                if let Some(key) = self.state.selected_key() {
                    self.state.confirm_action =
                        Some(state::ConfirmAction::DeleteKey(key.name.clone()));
                    self.state.view = state::RedisView::Confirm;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('I') => {
                self.get_info();
                KeyHandleResult::Handled
            }
            KeyCode::Char('r') => {
                // Refresh
                self.state.keys.clear();
                self.state.scan_cursor = 0;
                self.state.scan_complete = false;
                self.do_scan();
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                // Add to filter
                self.state.insert_filter_char(c);
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                self.state.backspace_filter();
                KeyHandleResult::Handled
            }
            KeyCode::Esc => {
                if !self.state.key_filter.is_empty() {
                    // Clear filter first
                    self.state.key_filter.clear();
                    self.state.filter_cursor = 0;
                    self.state.apply_filter();
                } else {
                    // Disconnect and close
                    self.client.disconnect();
                    self.state.reset();
                    self.modal_open = false;
                    return KeyHandleResult::CloseModal;
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle key detail keys
    fn handle_detail_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Up => {
                if self.state.detail_scroll > 0 {
                    self.state.detail_scroll -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                self.state.detail_scroll += 1;
                KeyHandleResult::Handled
            }
            KeyCode::Esc => {
                self.state.current_key = None;
                self.state.current_value = state::RedisValue::None;
                self.state.view = state::RedisView::KeyBrowser;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle server info keys
    fn handle_info_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Up => {
                if self.state.info_scroll > 0 {
                    self.state.info_scroll -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                if self.state.info_scroll < self.state.server_info.len().saturating_sub(1) {
                    self.state.info_scroll += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Esc => {
                self.state.view = state::RedisView::KeyBrowser;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle confirm dialog keys
    fn handle_confirm_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(action) = self.state.confirm_action.take() {
                    match action {
                        state::ConfirmAction::DeleteKey(name) => {
                            self.delete_key(&name);
                        }
                        state::ConfirmAction::Disconnect => {
                            self.client.disconnect();
                            self.state.reset();
                            self.modal_open = false;
                            return KeyHandleResult::CloseModal;
                        }
                    }
                }
                self.state.view = state::RedisView::KeyBrowser;
                KeyHandleResult::Handled
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.state.confirm_action = None;
                self.state.view = state::RedisView::KeyBrowser;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle error view keys
    fn handle_error_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                self.state.error = None;
                self.state.view = if self.state.connected {
                    state::RedisView::KeyBrowser
                } else {
                    state::RedisView::Connect
                };
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }
}

impl Default for RedisPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for RedisPlugin {
    fn id(&self) -> &str {
        "redis"
    }

    fn name(&self) -> &str {
        "Redis"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_modal: true,
            has_menu: true,
            ..Default::default()
        }
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        // Always available - connection is handled at runtime
        true
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: "redis".to_string(),
            name: "Redis".to_string(),
            description: "Key-value browser".to_string(),
            category: PluginCategory::Tools,
            key: 'R',
        })
    }

    fn launch(&mut self, _cwd: &PathBuf, _selected_file: Option<&PathBuf>) -> Result<(), String> {
        self.modal_open = true;
        // Reset to connect view
        self.state.view = state::RedisView::Connect;
        self.state.error = None;
        self.state.message = None;
        Ok(())
    }

    fn handle_global_key(
        &mut self,
        key: KeyEvent,
        _cwd: &PathBuf,
        _selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
        // Open with 'R' key
        if key.code == KeyCode::Char('R') {
            self.modal_open = true;
            self.state.view = state::RedisView::Connect;
            self.state.error = None;
            return KeyHandleResult::OpenModal;
        }
        KeyHandleResult::NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        // Clear messages on key press
        self.state.message = None;

        match self.state.view {
            state::RedisView::Connect => self.handle_connect_key(key),
            state::RedisView::Profiles => self.handle_profiles_key(key),
            state::RedisView::SaveProfile => {
                // Not fully implemented yet
                if key.code == KeyCode::Esc {
                    self.state.view = state::RedisView::Connect;
                }
                KeyHandleResult::Handled
            }
            state::RedisView::KeyBrowser => self.handle_browser_key(key),
            state::RedisView::KeyDetail => self.handle_detail_key(key),
            state::RedisView::ServerInfo => self.handle_info_key(key),
            state::RedisView::Confirm => self.handle_confirm_key(key),
            state::RedisView::Error => self.handle_error_key(key),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        modal::draw_redis_modal(frame, area, &self.state, colors);
    }
}
