//! Apps launcher plugin (F12)
//!
//! Centralized launcher for accessing QDOS plugins and tools.

mod modal;
pub mod state;

use crate::plugins::{
    KeyHandleResult, Plugin, PluginCapabilities, PluginMenuItem, PluginStatusInfo,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{layout::Rect, Frame};
use state::{AppEntry, AppsState};
use std::any::Any;
use std::path::PathBuf;

/// Apps launcher plugin
pub struct AppsPlugin {
    initialized: bool,
    pub state: AppsState,
    /// ID of plugin to launch after closing Apps modal
    pub launch_plugin: Option<String>,
    /// Plugin ID that was toggled (enabled/disabled)
    pub toggled_plugin: Option<(String, bool)>,
}

impl Default for AppsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl AppsPlugin {
    pub fn new() -> Self {
        Self {
            initialized: false,
            state: AppsState::new(),
            launch_plugin: None,
            toggled_plugin: None,
        }
    }

    /// Toggle the enabled status of the selected app
    fn toggle_selected(&mut self) -> Option<(String, bool)> {
        if let Some(app) = self.state.selected_app() {
            let id = app.id.clone();
            let new_enabled = !app.enabled;
            // Update local state
            for app in &mut self.state.apps {
                if app.id == id {
                    app.enabled = new_enabled;
                    break;
                }
            }
            return Some((id, new_enabled));
        }
        None
    }

    /// Set app entries from collected plugin info
    pub fn set_entries(&mut self, entries: Vec<AppEntry>) {
        self.state.apps = entries;
        // Sort by category then by name for consistent display
        self.state.apps.sort_by(|a, b| {
            a.category.cmp(&b.category).then(a.name.cmp(&b.name))
        });
    }

    /// Find app by key character
    fn find_app_by_key(&self, key: char) -> Option<&AppEntry> {
        let key_upper = key.to_ascii_uppercase();
        self.state
            .filtered_apps()
            .into_iter()
            .find(|app| app.key == key_upper && app.available)
    }
}

impl Plugin for AppsPlugin {
    fn id(&self) -> &str {
        "apps"
    }

    fn name(&self) -> &str {
        "Apps"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: false, // Not in plugin menu - accessed via F12
            has_keys: true,
            has_modal: true,
            has_status: false,
            has_cli: false,
            has_help: true,
        }
    }

    fn init(&mut self, _cwd: &PathBuf) -> Result<(), String> {
        self.initialized = true;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), String> {
        self.initialized = false;
        Ok(())
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        true // Always available
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        None // Accessed via F12, not plugin menu
    }

    fn status_info(&self, _cwd: &PathBuf) -> Option<PluginStatusInfo> {
        None
    }

    fn handle_global_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match key.code {
            KeyCode::F(12) => {
                // Open Apps launcher (entries are collected dynamically via set_entries)
                self.state.clear_filter();
                self.launch_plugin = None;
                KeyHandleResult::OpenModal
            }
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.clear_filter();
                self.toggled_plugin = None;
                KeyHandleResult::CloseModal
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.select_prev();
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.select_next();
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                // Check if there's a pending toggle - if so, save and close
                if self.toggled_plugin.is_some() {
                    let toggled = self.toggled_plugin.take();
                    self.state.clear_filter();
                    if let Some((id, enabled)) = toggled {
                        return KeyHandleResult::CloseWithSuccess(format!(
                            "plugin_toggle:{}:{}",
                            id, enabled
                        ));
                    }
                }
                // Launch selected app - clone id first to avoid borrow issues
                if let Some(app) = self.state.selected_app() {
                    if app.enabled && app.available {
                        let app_id = app.id.clone();
                        self.launch_plugin = Some(app_id.clone());
                        self.state.clear_filter();
                        KeyHandleResult::CloseWithSuccess(format!("launch:{}", app_id))
                    } else {
                        // Can't launch disabled or unavailable apps
                        KeyHandleResult::Handled
                    }
                } else {
                    KeyHandleResult::Handled
                }
            }
            KeyCode::Char(' ') => {
                // Space toggles enable/disable for selected plugin
                if let Some((id, enabled)) = self.toggle_selected() {
                    self.toggled_plugin = Some((id, enabled));
                }
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                self.state.filter.pop();
                self.state.selected_index = 0;
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                // SHIFT+letter = quick launch by key
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    if let Some(app) = self.find_app_by_key(c) {
                        if app.enabled && app.available {
                            let app_id = app.id.clone();
                            self.launch_plugin = Some(app_id.clone());
                            self.state.clear_filter();
                            return KeyHandleResult::CloseWithSuccess(format!("launch:{}", app_id));
                        }
                    }
                    // SHIFT+letter that doesn't match any app - ignore
                    return KeyHandleResult::Handled;
                }
                // Regular letter = add to filter
                self.state.filter.push(c);
                self.state.selected_index = 0;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &crate::app::ThemeColors) {
        modal::draw_apps_modal(frame, area, &self.state, colors);
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "F12 - Apps Launcher".to_string(),
            "".to_string(),
            "Quick access to QDOS plugins and tools.".to_string(),
            "".to_string(),
            "Keys:".to_string(),
            "  F12       Open Apps launcher".to_string(),
            "  ↑↓        Navigate list".to_string(),
            "  Enter     Open selected app".to_string(),
            "  Shift+Key Quick launch by key".to_string(),
            "  Space     Toggle enable/disable".to_string(),
            "  Type      Filter apps by name".to_string(),
            "  Bksp      Clear filter".to_string(),
            "  Esc       Close".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
