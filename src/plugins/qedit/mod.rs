//! Q-EDIT - Built-in text editor plugin
//!
//! Provides a text editor with:
//! - Insert and overwrite modes
//! - Hex mode display
//! - Find/replace
//! - Block operations (buffer, copy, delete)
//! - Markers and jump

pub mod modal;
pub mod state;

pub use state::{DisplayMode, EditorMode, QEditMenuItem, QEditState};

use crate::app::ThemeColors;
use crate::plugins::{KeyHandleResult, Plugin, PluginCapabilities, PluginMenuItem};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{layout::Rect, Frame};
use std::any::Any;
use std::path::PathBuf;

/// Q-EDIT plugin
pub struct QEditPlugin {
    initialized: bool,
    pub modal_state: Option<QEditState>,
}

impl QEditPlugin {
    pub fn new() -> Self {
        Self {
            initialized: false,
            modal_state: None,
        }
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

    fn init(&mut self, _cwd: &PathBuf) -> Result<(), String> {
        self.initialized = true;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), String> {
        Ok(())
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

    fn handle_global_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match key.code {
            KeyCode::F(9) if !key.modifiers.contains(KeyModifiers::ALT) => {
                // F9 opens editor (file must be provided externally)
                KeyHandleResult::OpenModal
            }
            KeyCode::F(9) if key.modifiers.contains(KeyModifiers::ALT) => {
                // Alt-F9 opens blank editor
                self.modal_state = Some(QEditState::new());
                KeyHandleResult::OpenModal
            }
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
            "Q-EDIT - Text Editor".to_string(),
            "".to_string(),
            "F9       Open editor with selected file".to_string(),
            "Alt-F9   Open blank editor".to_string(),
            "".to_string(),
            "In Command mode (menu bar visible):".to_string(),
            "Space    Next menu item".to_string(),
            "Enter    Select menu item".to_string(),
            "A-T      Quick menu shortcuts".to_string(),
            "Esc      Quit (if not modified)".to_string(),
            "".to_string(),
            "In Edit mode:".to_string(),
            "Arrow keys   Move cursor".to_string(),
            "Home/End     Start/end of line".to_string(),
            "Ctrl+Home    Top of file".to_string(),
            "Ctrl+End     Bottom of file".to_string(),
            "PgUp/PgDn    Page up/down".to_string(),
            "Insert       Toggle insert/overwrite".to_string(),
            "Backspace    Delete before cursor".to_string(),
            "Delete       Delete at cursor".to_string(),
            "Esc          Return to command mode".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
