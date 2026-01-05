//! Apps launcher plugin (F12)
//!
//! Centralized launcher for accessing QDOS plugins and tools.

mod modal;
pub mod state;

use crate::plugins::{
    KeyHandleResult, Plugin, PluginCapabilities, PluginMenuItem, PluginStatusInfo,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, Frame};
use state::{AppEntry, AppsState, PluginCategory};
use std::any::Any;
use std::path::PathBuf;

/// Apps launcher plugin
pub struct AppsPlugin {
    initialized: bool,
    pub state: AppsState,
    /// ID of plugin to launch after closing Apps modal
    pub launch_plugin: Option<String>,
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
        }
    }

    /// Build the list of available apps/plugins
    fn build_app_list(&mut self) {
        self.state.apps = vec![
            // Tools category
            AppEntry {
                id: "ai".to_string(),
                name: "AI Assistant".to_string(),
                description: "Monitor AI coding tools".to_string(),
                category: PluginCategory::Tools,
                key: 'A',
                available: true,
            },
            AppEntry {
                id: "proc".to_string(),
                name: "Processes".to_string(),
                description: "System process monitor".to_string(),
                category: PluginCategory::System,
                key: 'P',
                available: true,
            },
            AppEntry {
                id: "theme".to_string(),
                name: "Theme".to_string(),
                description: "Color theme settings".to_string(),
                category: PluginCategory::System,
                key: 'T',
                available: true,
            },
            AppEntry {
                id: "qdconfig".to_string(),
                name: "Config".to_string(),
                description: "QDOS configuration".to_string(),
                category: PluginCategory::System,
                key: 'C',
                available: true,
            },
            // VCS category
            AppEntry {
                id: "git".to_string(),
                name: "Git".to_string(),
                description: "Git version control".to_string(),
                category: PluginCategory::Vcs,
                key: 'G',
                available: true,
            },
            AppEntry {
                id: "jj".to_string(),
                name: "Jujutsu".to_string(),
                description: "Jujutsu VCS".to_string(),
                category: PluginCategory::Vcs,
                key: 'J',
                available: true,
            },
            AppEntry {
                id: "beads".to_string(),
                name: "Beads".to_string(),
                description: "Issue tracker".to_string(),
                category: PluginCategory::Tools,
                key: 'B',
                available: true,
            },
            // Files category
            AppEntry {
                id: "dirmap".to_string(),
                name: "Dir Map".to_string(),
                description: "Directory tree view".to_string(),
                category: PluginCategory::Files,
                key: 'D',
                available: true,
            },
            AppEntry {
                id: "space".to_string(),
                name: "Disk Space".to_string(),
                description: "Disk usage info".to_string(),
                category: PluginCategory::Files,
                key: 'S',
                available: true,
            },
        ];
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
        self.build_app_list();
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
                // Build app list if empty (lazy init)
                if self.state.apps.is_empty() {
                    self.build_app_list();
                }
                // Open Apps launcher
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
                // Launch selected app - clone id first to avoid borrow issues
                if let Some(app_id) = self.state.selected_app().map(|a| a.id.clone()) {
                    self.launch_plugin = Some(app_id.clone());
                    self.state.clear_filter();
                    KeyHandleResult::CloseWithSuccess(format!("launch:{}", app_id))
                } else {
                    KeyHandleResult::Handled
                }
            }
            KeyCode::Backspace => {
                self.state.filter.pop();
                self.state.selected_index = 0;
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                // Check for direct key launch first - clone id to avoid borrow issues
                if self.state.filter.is_empty() {
                    if let Some(app_id) = self.find_app_by_key(c).map(|a| a.id.clone()) {
                        self.launch_plugin = Some(app_id.clone());
                        return KeyHandleResult::CloseWithSuccess(format!("launch:{}", app_id));
                    }
                }
                // Otherwise add to filter
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
            "  F12     Open Apps launcher".to_string(),
            "  ↑↓      Navigate list".to_string(),
            "  Enter   Open selected app".to_string(),
            "  A-Z     Quick launch by key".to_string(),
            "  Type    Filter apps by name".to_string(),
            "  Esc     Close".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
