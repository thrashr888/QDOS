//! Q-EDIT - Built-in text editor plugin
//!
//! Provides a text editor with:
//! - Insert and overwrite modes
//! - Hex mode display
//! - Find/replace
//! - Block operations (buffer, copy, delete)
//! - Markers and jump

#![allow(clippy::ptr_arg)]

pub mod modal;
pub mod state;

pub use state::{DisplayMode, EditorMode, QEditMenuItem, QEditState};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use qdos_plugin_api::prelude::*;
use ratatui::{layout::Rect, Frame};
use std::any::Any;
use std::path::PathBuf;

/// Q-EDIT plugin
pub struct QEditPlugin {
    pub modal_state: Option<QEditState>,
}

impl QEditPlugin {
    pub fn new() -> Self {
        Self { modal_state: None }
    }

    /// Open the editor with an optional file
    pub fn open(&mut self, file_path: Option<PathBuf>) -> anyhow::Result<()> {
        let state = if let Some(path) = file_path {
            QEditState::load_file(path)?
        } else {
            QEditState::new()
        };
        self.modal_state = Some(state);
        Ok(())
    }

    /// Check if the editor is open
    pub fn is_open(&self) -> bool {
        self.modal_state.is_some()
    }

    /// Handle menu commands
    fn handle_menu_command(&mut self, item: QEditMenuItem) -> KeyHandleResult {
        let state = match &mut self.modal_state {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        match item {
            QEditMenuItem::Edit => {
                state.mode = EditorMode::Insert;
            }
            QEditMenuItem::Hex => {
                state.display_mode = if state.display_mode == DisplayMode::Ascii {
                    DisplayMode::Hex
                } else {
                    DisplayMode::Ascii
                };
            }
            QEditMenuItem::Quit => {
                if state.modified {
                    // TODO: Ask to save
                }
                self.modal_state = None;
                return KeyHandleResult::CloseModal;
            }
            _ => {
                // TODO: Implement other commands
            }
        }

        KeyHandleResult::Handled
    }
}

impl Default for QEditPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for QEditPlugin {
    fn id(&self) -> &str {
        "qedit"
    }

    fn name(&self) -> &str {
        "Q-EDIT"
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        true
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: true,
            has_keys: true,
            has_modal: true,
            has_status: false,
            has_cli: true,
            has_help: true,
        }
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "Edit".to_string(),
            key: 'E',
            description: "Q-EDIT text editor".to_string(),
            priority: 90, // F9 = 90
        })
    }

    fn handle_global_key(
        &mut self,
        key: KeyEvent,
        _cwd: &PathBuf,
        _selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
        match key.code {
            KeyCode::F(9) if key.modifiers.contains(KeyModifiers::ALT) => {
                // Alt-F9 opens blank editor
                self.modal_state = Some(QEditState::new());
                KeyHandleResult::OpenModal
            }
            // F9 without Alt is handled by app to pass the selected file
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        let state = match &mut self.modal_state {
            Some(s) => s,
            None => return KeyHandleResult::NotHandled,
        };

        match state.mode {
            EditorMode::Command => {
                let menu_items = QEditMenuItem::all();
                match key.code {
                    KeyCode::Esc => {
                        if state.modified {
                            // Don't close if modified, show warning
                            KeyHandleResult::Handled
                        } else {
                            self.modal_state = None;
                            KeyHandleResult::CloseModal
                        }
                    }
                    KeyCode::Char(' ') => {
                        // Move menu cursor right
                        state.menu_index = (state.menu_index + 1) % menu_items.len();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Enter => {
                        // Select current menu item
                        let item = menu_items[state.menu_index];
                        self.handle_menu_command(item)
                    }
                    KeyCode::Char(c) => {
                        // Check if it matches a menu item key
                        let c_upper = c.to_ascii_uppercase();
                        if let Some(item) = menu_items.iter().find(|i| i.key() == c_upper) {
                            self.handle_menu_command(*item)
                        } else {
                            KeyHandleResult::Handled
                        }
                    }
                    _ => KeyHandleResult::Handled,
                }
            }
            EditorMode::Insert | EditorMode::Overwrite => {
                match key.code {
                    KeyCode::Esc => {
                        state.mode = EditorMode::Command;
                        KeyHandleResult::Handled
                    }
                    KeyCode::Insert => {
                        state.mode = state.mode.toggle_insert();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Up => {
                        state.move_up();
                        state.ensure_visible(20); // Approximate visible lines
                        KeyHandleResult::Handled
                    }
                    KeyCode::Down => {
                        state.move_down();
                        state.ensure_visible(20);
                        KeyHandleResult::Handled
                    }
                    KeyCode::Left => {
                        state.move_left();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Right => {
                        state.move_right();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Home if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.move_top();
                        KeyHandleResult::Handled
                    }
                    KeyCode::End if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.move_bottom();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Home => {
                        state.move_home();
                        KeyHandleResult::Handled
                    }
                    KeyCode::End => {
                        state.move_end();
                        KeyHandleResult::Handled
                    }
                    KeyCode::PageUp => {
                        state.page_up(20);
                        KeyHandleResult::Handled
                    }
                    KeyCode::PageDown => {
                        state.page_down(20);
                        KeyHandleResult::Handled
                    }
                    KeyCode::Enter => {
                        state.insert_newline();
                        state.ensure_visible(20);
                        KeyHandleResult::Handled
                    }
                    KeyCode::Backspace => {
                        state.backspace();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Delete => {
                        state.delete_char();
                        KeyHandleResult::Handled
                    }
                    KeyCode::Tab => {
                        // Insert spaces based on tab size
                        for _ in 0..state.tab_size {
                            state.insert_char(' ');
                        }
                        KeyHandleResult::Handled
                    }
                    KeyCode::Char(c) => {
                        state.insert_char(c);
                        KeyHandleResult::Handled
                    }
                    _ => KeyHandleResult::Handled,
                }
            }
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        if let Some(ref state) = self.modal_state {
            modal::draw_qedit_modal(frame, area, state, colors);
        }
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "           E -- Q-EDIT TEXT EDITOR".to_string(),
            "".to_string(),
            "Purpose:   Built-in text editor with insert/overwrite modes,".to_string(),
            "           hex display, and block operations.".to_string(),
            "".to_string(),
            "To use:    Press F9 to open with selected file, or Alt-F9 for".to_string(),
            "           a blank editor.".to_string(),
            "".to_string(),
            "Modes:".to_string(),
            "  Command  - Menu bar visible, press key to select command".to_string(),
            "  Insert   - Type to insert text at cursor".to_string(),
            "  Overwrite- Type to replace text at cursor".to_string(),
            "".to_string(),
            "Command Mode:".to_string(),
            "  Space    - Next menu item".to_string(),
            "  Enter    - Select menu item".to_string(),
            "  A-T      - Quick menu shortcuts (first letter)".to_string(),
            "  Esc      - Quit (prompts if modified)".to_string(),
            "".to_string(),
            "Edit Mode Navigation:".to_string(),
            "  ↑↓←→     - Move cursor".to_string(),
            "  Home/End - Start/end of line".to_string(),
            "  Ctrl+Home- Top of file".to_string(),
            "  Ctrl+End - Bottom of file".to_string(),
            "  PgUp/Dn  - Page up/down".to_string(),
            "".to_string(),
            "Edit Mode Actions:".to_string(),
            "  Insert   - Toggle insert/overwrite mode".to_string(),
            "  Bksp     - Delete before cursor".to_string(),
            "  Delete   - Delete at cursor".to_string(),
            "  Tab      - Insert spaces (tab size)".to_string(),
            "  Esc      - Return to command mode".to_string(),
            "".to_string(),
            "Tip:       Use H menu for Hex mode to view binary files.".to_string(),
            "           The status bar shows line, column, and mode.".to_string(),
        ]
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "Editor".to_string(),
            description: "Built-in text editor".to_string(),
            category: PluginCategory::Files,
            key: 'E',
        })
    }

    fn launch(&mut self, _cwd: &PathBuf, selected_file: Option<&PathBuf>) -> Result<(), String> {
        self.open(selected_file.cloned()).map_err(|e| e.to_string())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// Self-registration for automatic plugin discovery
inventory::submit! {
    PluginRegistration::new("qedit", || Box::new(QEditPlugin::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let plugin = QEditPlugin::new();
        assert!(plugin.modal_state.is_none());
    }

    #[test]
    fn test_plugin_id_name() {
        let plugin = QEditPlugin::new();
        assert_eq!(plugin.id(), "qedit");
        assert_eq!(plugin.name(), "Q-EDIT");
    }

    #[test]
    fn test_plugin_capabilities() {
        let plugin = QEditPlugin::new();
        let caps = plugin.capabilities();
        assert!(caps.has_menu);
        assert!(caps.has_keys);
        assert!(caps.has_modal);
        assert!(!caps.has_status);
        assert!(caps.has_cli);
        assert!(caps.has_help);
    }

    #[test]
    fn test_plugin_menu_item() {
        let plugin = QEditPlugin::new();
        let item = plugin.menu_item().expect("Should have menu item");
        assert_eq!(item.name, "Edit");
        assert_eq!(item.key, 'E');
        assert_eq!(item.priority, 90);
    }

    #[test]
    fn test_plugin_is_open() {
        let mut plugin = QEditPlugin::new();
        assert!(!plugin.is_open());

        plugin.modal_state = Some(QEditState::new());
        assert!(plugin.is_open());
    }

    #[test]
    fn test_plugin_is_available() {
        let plugin = QEditPlugin::new();
        let cwd = PathBuf::from("/tmp");
        assert!(plugin.is_available(&cwd));
    }

    #[test]
    fn test_plugin_help_content() {
        let plugin = QEditPlugin::new();
        let help = plugin.help_content();
        assert!(!help.is_empty());
        assert!(help[0].contains("Q-EDIT"));
    }

    #[test]
    fn test_handle_menu_edit_command() {
        let mut plugin = QEditPlugin::new();
        plugin.modal_state = Some(QEditState::new());

        let result = plugin.handle_menu_command(QEditMenuItem::Edit);
        assert!(matches!(result, KeyHandleResult::Handled));

        if let Some(ref state) = plugin.modal_state {
            assert_eq!(state.mode, EditorMode::Insert);
        }
    }

    #[test]
    fn test_handle_menu_hex_toggle() {
        let mut plugin = QEditPlugin::new();
        plugin.modal_state = Some(QEditState::new());

        // Initially ASCII
        assert_eq!(
            plugin.modal_state.as_ref().unwrap().display_mode,
            DisplayMode::Ascii
        );

        plugin.handle_menu_command(QEditMenuItem::Hex);
        assert_eq!(
            plugin.modal_state.as_ref().unwrap().display_mode,
            DisplayMode::Hex
        );

        plugin.handle_menu_command(QEditMenuItem::Hex);
        assert_eq!(
            plugin.modal_state.as_ref().unwrap().display_mode,
            DisplayMode::Ascii
        );
    }

    #[test]
    fn test_handle_menu_quit() {
        let mut plugin = QEditPlugin::new();
        plugin.modal_state = Some(QEditState::new());

        let result = plugin.handle_menu_command(QEditMenuItem::Quit);
        assert!(matches!(result, KeyHandleResult::CloseModal));
        assert!(plugin.modal_state.is_none());
    }
}
