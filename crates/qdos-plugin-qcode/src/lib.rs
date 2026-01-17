//! Q-CODE: Simple Code Editor Plugin for R-DOS
//!
//! A lightweight code editor/IDE with file tree, syntax highlighting,
//! and basic editing capabilities.

mod modal;
mod state;

pub use state::{EditorBuffer, QCodeState, QCodeView};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use qdos_plugin_api::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, ThemeColors,
};
use ratatui::{layout::Rect, Frame};
use std::any::Any;
use std::path::PathBuf;

// =============================================================================
// PLUGIN
// =============================================================================

pub struct QCodePlugin {
    state: QCodeState,
}

impl Default for QCodePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl QCodePlugin {
    pub fn new() -> Self {
        Self {
            state: QCodeState::new(),
        }
    }

    // =========================================================================
    // KEY HANDLERS
    // =========================================================================

    fn handle_file_tree_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => KeyHandleResult::CloseModal,
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.file_tree_up();
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.file_tree_down();
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                match self.state.open_selected_file() {
                    Ok(()) => {}
                    Err(e) => {
                        self.state.status_message = Some((e, 60));
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                self.state.go_up_directory();
                KeyHandleResult::Handled
            }
            KeyCode::Tab => {
                // Switch to editor if there's a buffer
                if !self.state.buffers.is_empty() {
                    self.state.view = QCodeView::Editor;
                }
                KeyHandleResult::Handled
            }
            KeyCode::F(1) => {
                self.state.view = QCodeView::Help;
                KeyHandleResult::Handled
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.state.refresh_file_tree();
                self.state.status_message = Some(("Refreshed".to_string(), 30));
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_editor_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            KeyCode::Esc => {
                // Switch back to file tree
                self.state.view = QCodeView::FileTree;
                KeyHandleResult::Handled
            }
            KeyCode::Tab if !ctrl => {
                // Insert tab in editor
                if let Some(buffer) = self.state.current_buffer_mut() {
                    buffer.insert_tab();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Tab if ctrl => {
                // Ctrl+Tab switches buffers
                self.state.next_buffer();
                KeyHandleResult::Handled
            }
            KeyCode::F(1) => {
                self.state.view = QCodeView::Help;
                KeyHandleResult::Handled
            }

            // Save
            KeyCode::Char('s') if ctrl => {
                match self.state.save_current_buffer() {
                    Ok(()) => {}
                    Err(e) => {
                        self.state.status_message = Some((e, 60));
                    }
                }
                KeyHandleResult::Handled
            }

            // Close buffer
            KeyCode::Char('w') if ctrl => {
                self.state.close_current_buffer();
                if self.state.buffers.iter().all(|b| b.file_path.is_none()) {
                    self.state.view = QCodeView::FileTree;
                }
                KeyHandleResult::Handled
            }

            // Navigation
            KeyCode::Up => {
                if let Some(buffer) = self.state.current_buffer_mut() {
                    buffer.move_up();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                if let Some(buffer) = self.state.current_buffer_mut() {
                    buffer.move_down();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Left => {
                if let Some(buffer) = self.state.current_buffer_mut() {
                    buffer.move_left();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Right => {
                if let Some(buffer) = self.state.current_buffer_mut() {
                    buffer.move_right();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Home => {
                if let Some(buffer) = self.state.current_buffer_mut() {
                    buffer.move_home();
                }
                KeyHandleResult::Handled
            }
            KeyCode::End => {
                if let Some(buffer) = self.state.current_buffer_mut() {
                    buffer.move_end();
                }
                KeyHandleResult::Handled
            }
            KeyCode::PageUp => {
                if let Some(buffer) = self.state.current_buffer_mut() {
                    buffer.move_page_up(20);
                }
                KeyHandleResult::Handled
            }
            KeyCode::PageDown => {
                if let Some(buffer) = self.state.current_buffer_mut() {
                    buffer.move_page_down(20);
                }
                KeyHandleResult::Handled
            }

            // Editing
            KeyCode::Enter => {
                if let Some(buffer) = self.state.current_buffer_mut() {
                    buffer.insert_newline();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                if let Some(buffer) = self.state.current_buffer_mut() {
                    buffer.backspace();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Delete => {
                if let Some(buffer) = self.state.current_buffer_mut() {
                    buffer.delete_char();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) if !ctrl => {
                if let Some(buffer) = self.state.current_buffer_mut() {
                    buffer.insert_char(c);
                }
                KeyHandleResult::Handled
            }

            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_help_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc | KeyCode::F(1) => {
                self.state.view = QCodeView::FileTree;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }
}

// =============================================================================
// PLUGIN TRAIT
// =============================================================================

impl Plugin for QCodePlugin {
    fn id(&self) -> &str {
        "qcode"
    }

    fn name(&self) -> &str {
        "Q-CODE"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: false,
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

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "Q-CODE".to_string(),
            description: "Simple code editor with syntax highlighting".to_string(),
            category: PluginCategory::Tools,
            key: 'C',
        })
    }

    fn launch(&mut self, cwd: &PathBuf, selected_file: Option<&PathBuf>) -> Result<(), String> {
        self.state = QCodeState::new();
        self.state.set_cwd(cwd.clone());

        // If a file is selected, try to open it
        if let Some(path) = selected_file {
            if path.is_file() {
                match EditorBuffer::from_file(path.clone()) {
                    Ok(buffer) => {
                        self.state.buffers = vec![buffer];
                        self.state.current_buffer = 0;
                        self.state.view = QCodeView::Editor;
                    }
                    Err(e) => {
                        self.state.status_message = Some((format!("Failed to open: {}", e), 60));
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_global_key(
        &mut self,
        _key: KeyEvent,
        _cwd: &PathBuf,
        _selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
        // Q-CODE is launched via Apps menu (F12) which calls launch()
        KeyHandleResult::NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match self.state.view {
            QCodeView::FileTree => self.handle_file_tree_key(key),
            QCodeView::Editor => self.handle_editor_key(key),
            QCodeView::Terminal => {
                // Terminal view not yet implemented
                if key.code == KeyCode::Esc {
                    self.state.view = QCodeView::FileTree;
                }
                KeyHandleResult::Handled
            }
            QCodeView::Help => self.handle_help_key(key),
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        modal::draw_qcode(&self.state, frame, area, colors);
    }

    fn tick(&mut self) {
        // Decrement status message timer
        if let Some((_, ref mut ticks)) = self.state.status_message {
            if *ticks > 0 {
                *ticks -= 1;
            } else {
                self.state.status_message = None;
            }
        }

        // Ensure cursor is visible in editor
        if self.state.view == QCodeView::Editor {
            if let Some(buffer) = self.state.current_buffer_mut() {
                buffer.ensure_visible(20);
            }
        }
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "Q-CODE - Simple Code Editor".to_string(),
            "".to_string(),
            "A lightweight code editor with syntax highlighting.".to_string(),
            "".to_string(),
            "File Tree:".to_string(),
            "  Up/Down      Navigate files".to_string(),
            "  Enter        Open file/directory".to_string(),
            "  Backspace    Go up directory".to_string(),
            "  Tab          Switch to editor".to_string(),
            "  R            Refresh file list".to_string(),
            "".to_string(),
            "Editor:".to_string(),
            "  Arrow keys   Move cursor".to_string(),
            "  Home/End     Line start/end".to_string(),
            "  PgUp/PgDn    Scroll pages".to_string(),
            "  Ctrl+S       Save file".to_string(),
            "  Ctrl+W       Close buffer".to_string(),
            "  Ctrl+Tab     Next buffer".to_string(),
            "  Esc          Back to file tree".to_string(),
            "".to_string(),
            "General:".to_string(),
            "  F1           Show help".to_string(),
            "  Esc          Exit Q-CODE".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
