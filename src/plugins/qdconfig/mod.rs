//! QDCONFIG Plugin for R-DOS
//!
//! Provides startup configuration (Ctrl+S) as a self-contained plugin.

mod modal;
mod state;

pub use state::{QdconfigField, QdconfigState};

use super::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, PluginMenuItem,
};
use crate::app::{ColorTheme, SortMode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::Frame;
use std::any::Any;
use std::path::PathBuf;

/// QDCONFIG plugin for startup configuration
pub struct QdconfigPlugin {
    /// Whether modal is open
    modal_open: bool,
    /// Configuration state
    state: Option<QdconfigState>,
    /// Result state (set when applied/saved)
    result_state: Option<QdconfigState>,
    /// Whether settings were saved (vs just applied)
    settings_saved: bool,
}

impl QdconfigPlugin {
    pub fn new() -> Self {
        Self {
            modal_open: false,
            state: None,
            result_state: None,
            settings_saved: false,
        }
    }

    /// Check if modal is open
    pub fn is_modal_open(&self) -> bool {
        self.modal_open
    }

    /// Open the modal with current settings
    pub fn open_modal(
        &mut self,
        search_spec: String,
        sort_mode: SortMode,
        show_hidden: bool,
        confirm_delete: bool,
        editor: Option<String>,
        color_theme: ColorTheme,
        mouse_support: bool,
        uppercase_names: bool,
        auto_refresh_interval: u64,
        plugins: Vec<(String, String, String)>,
    ) {
        self.state = Some(QdconfigState::new(
            search_spec,
            sort_mode,
            show_hidden,
            confirm_delete,
            editor,
            color_theme,
            mouse_support,
            uppercase_names,
            auto_refresh_interval,
            plugins,
        ));
        self.result_state = None;
        self.settings_saved = false;
        self.modal_open = true;
    }

    /// Close the modal
    pub fn close_modal(&mut self) {
        self.modal_open = false;
        self.state = None;
    }

    /// Get the result state (if applied)
    pub fn take_result(&mut self) -> Option<QdconfigState> {
        self.result_state.take()
    }

    /// Check if settings were saved to disk
    pub fn was_saved(&self) -> bool {
        self.settings_saved
    }

    /// Get current preview theme (for live preview)
    pub fn preview_theme(&self) -> Option<ColorTheme> {
        self.state.as_ref().map(|s| s.theme())
    }
}

impl Default for QdconfigPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for QdconfigPlugin {
    fn id(&self) -> &str {
        "qdconfig"
    }

    fn name(&self) -> &str {
        "Configuration"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: true,
            has_keys: true,
            has_modal: true,
            has_status: false,
            has_cli: false,
            has_help: true,
        }
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        true
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "Config".to_string(),
            key: 'S', // Ctrl+S
            description: "Configure startup options".to_string(),
            priority: 40, // After SearchSpec
        })
    }

    fn handle_global_key(
        &mut self,
        key: KeyEvent,
        _cwd: &PathBuf,
        _selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
        // Ctrl+S opens configuration
        if key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
            // We'll get the settings synced from app when modal opens
            // Open with defaults for now - app will re-sync in handle_plugin_result
            self.state = Some(QdconfigState::new(
                "*.*".to_string(),
                SortMode::NameAsc,
                false,
                true,
                None,
                ColorTheme::Default,
                false,
                false,
                5,          // default auto-refresh
                Vec::new(), // plugins will be synced from app
            ));
            self.result_state = None;
            self.settings_saved = false;
            self.modal_open = true;
            KeyHandleResult::OpenModal
        } else {
            KeyHandleResult::NotHandled
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        let Some(ref mut state) = self.state else {
            return KeyHandleResult::CloseModal;
        };

        if state.editing {
            // Handle text input mode
            match key.code {
                KeyCode::Enter => {
                    // Apply the edited value
                    if let Some(current_field) = state.current_field() {
                        match current_field {
                            QdconfigField::SearchSpec => {
                                state.search_spec = state.input_buffer.clone();
                            }
                            QdconfigField::Editor => {
                                if state.input_buffer.is_empty() || state.input_buffer == "$EDITOR"
                                {
                                    state.editor = None;
                                } else {
                                    state.editor = Some(state.input_buffer.clone());
                                }
                            }
                            _ => {}
                        }
                    }
                    state.editing = false;
                    state.input_buffer.clear();
                    KeyHandleResult::Handled
                }
                KeyCode::Esc => {
                    state.editing = false;
                    state.input_buffer.clear();
                    KeyHandleResult::Handled
                }
                KeyCode::Backspace => {
                    state.input_buffer.pop();
                    KeyHandleResult::Handled
                }
                KeyCode::Char(c) => {
                    state.input_buffer.push(c);
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            }
        } else {
            // Handle navigation/selection mode
            match key.code {
                KeyCode::Esc => {
                    // Restore original theme on cancel
                    let original_theme = state.original_theme();
                    self.close_modal();
                    KeyHandleResult::CloseWithError(format!("theme:{}", original_theme.name()))
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if state.selected > 0 {
                        state.selected -= 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    // Total items = config fields + plugins
                    let total_items = QdconfigField::ALL.len() + state.plugins.len();
                    if state.selected < total_items - 1 {
                        state.selected += 1;
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    // Toggle or edit based on field type (plugins are info-only)
                    if let Some(current_field) = state.current_field() {
                        match current_field {
                            QdconfigField::SearchSpec | QdconfigField::Editor => {
                                // Enter editing mode
                                state.editing = true;
                                match current_field {
                                    QdconfigField::SearchSpec => {
                                        state.input_buffer = state.search_spec.clone();
                                    }
                                    QdconfigField::Editor => {
                                        state.input_buffer = state
                                            .editor
                                            .clone()
                                            .unwrap_or_else(|| "$EDITOR".to_string());
                                    }
                                    _ => {}
                                }
                            }
                            QdconfigField::SortMethod => {
                                state.cycle_sort_method();
                            }
                            QdconfigField::SortDirection => {
                                state.toggle_sort_direction();
                            }
                            QdconfigField::ShowHidden => {
                                state.show_hidden = !state.show_hidden;
                            }
                            QdconfigField::ConfirmDelete => {
                                state.confirm_delete = !state.confirm_delete;
                            }
                            QdconfigField::ColorTheme => {
                                state.cycle_theme();
                                // Live preview handled by returning Handled
                                // App checks preview_theme() in handle_plugin_result
                            }
                            QdconfigField::MouseSupport => {
                                state.mouse_support = !state.mouse_support;
                            }
                            QdconfigField::UppercaseNames => {
                                state.uppercase_names = !state.uppercase_names;
                            }
                            QdconfigField::AutoRefresh => {
                                state.cycle_auto_refresh();
                            }
                        }
                    }
                    // Plugins are info-only, pressing Enter does nothing
                    KeyHandleResult::Handled
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    // Save configuration
                    self.result_state = Some(state.clone());
                    self.settings_saved = true;
                    self.close_modal();
                    KeyHandleResult::CloseWithSuccess("qdconfig:saved".to_string())
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    // Reload configuration from disk
                    if let Ok(config) = crate::config::Config::load() {
                        state.reload_from_config(&config);
                    }
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            }
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &crate::app::ThemeColors) {
        if let Some(ref state) = self.state {
            modal::draw_config_modal(frame, area, state, colors);
        }
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "           S -- CONFIGURATION".to_string(),
            "".to_string(),
            "Purpose:   Configure R-DOS startup options and preferences.".to_string(),
            "           Changes can be applied for current session or saved".to_string(),
            "           permanently to the config file.".to_string(),
            "".to_string(),
            "To use:    Press Ctrl+S to open the configuration dialog.".to_string(),
            "".to_string(),
            "Settings:".to_string(),
            "  Search Spec     - Default file pattern (e.g., *.*)".to_string(),
            "  Sort Method     - Name, Extension, Size, or Date".to_string(),
            "  Sort Direction  - Ascending or Descending".to_string(),
            "  Show Hidden     - Display hidden files".to_string(),
            "  Confirm Delete  - Prompt before file deletion".to_string(),
            "  Editor          - External editor command ($EDITOR)".to_string(),
            "  Color Theme     - Visual theme selection".to_string(),
            "  Mouse Support   - Enable/disable mouse interaction".to_string(),
            "  Uppercase Names - Display filenames in uppercase".to_string(),
            "  Auto Refresh    - Directory refresh interval".to_string(),
            "".to_string(),
            "Navigation:".to_string(),
            "  ↑↓       - Navigate settings list".to_string(),
            "  Enter    - Toggle or edit setting".to_string(),
            "  S        - Save settings to config file".to_string(),
            "  R        - Reload settings from config file".to_string(),
            "  Esc      - Cancel and close".to_string(),
            "".to_string(),
            "Tip:       Config is stored in ~/Library/Application Support/rdos/".to_string(),
            "           Plugins section shows installed plugins.".to_string(),
        ]
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "Config".to_string(),
            description: "RDOS configuration".to_string(),
            category: PluginCategory::System,
            key: 'C',
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
mod tests {
    use super::*;

    #[test]
    fn test_qdconfig_plugin_creation() {
        let plugin = QdconfigPlugin::new();
        assert_eq!(plugin.id(), "qdconfig");
        assert!(!plugin.is_modal_open());
    }

    #[test]
    fn test_plugin_capabilities() {
        let plugin = QdconfigPlugin::new();
        let caps = plugin.capabilities();
        assert!(caps.has_menu);
        assert!(caps.has_keys);
        assert!(caps.has_modal);
        assert!(!caps.has_status);
        assert!(!caps.has_cli);
        assert!(caps.has_help);
    }

    #[test]
    fn test_modal_open_close() {
        let mut plugin = QdconfigPlugin::new();
        plugin.open_modal(
            "*.*".to_string(),
            SortMode::NameAsc,
            false,
            true,
            None,
            ColorTheme::Default,
            false,
            false,
            5,
            Vec::new(),
        );
        assert!(plugin.is_modal_open());
        plugin.close_modal();
        assert!(!plugin.is_modal_open());
    }

    #[test]
    fn test_preview_theme() {
        let mut plugin = QdconfigPlugin::new();
        assert!(plugin.preview_theme().is_none());

        plugin.open_modal(
            "*.*".to_string(),
            SortMode::NameAsc,
            false,
            true,
            None,
            ColorTheme::Default,
            false,
            false,
            5,
            Vec::new(),
        );
        assert!(plugin.preview_theme().is_some());
    }

    #[test]
    fn test_take_result() {
        let mut plugin = QdconfigPlugin::new();
        assert!(plugin.take_result().is_none());
    }

    #[test]
    fn test_was_saved() {
        let plugin = QdconfigPlugin::new();
        assert!(!plugin.was_saved());
    }
}
