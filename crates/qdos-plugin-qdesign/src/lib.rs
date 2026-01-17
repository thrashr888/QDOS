//! Q-DESIGN: Print Designer Plugin for R-DOS
//!
//! A print/layout designer for creating cards, banners, flyers, and other printable designs.
//! Inspired by Print Shop, PrintMaster, and Canva.

mod modal;
mod state;

pub use state::{Frame, Page, QDesignState, QDesignView, Template, TextAlignment, Tool};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use qdos_plugin_api::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, ThemeColors,
};
use ratatui::{layout::Rect, Frame as RatatuiFrame};
use std::any::Any;
use std::path::PathBuf;

// =============================================================================
// PLUGIN
// =============================================================================

pub struct QDesignPlugin {
    state: QDesignState,
}

impl Default for QDesignPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl QDesignPlugin {
    pub fn new() -> Self {
        Self {
            state: QDesignState::new(),
        }
    }

    // =========================================================================
    // KEY HANDLERS
    // =========================================================================

    fn handle_template_select_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => KeyHandleResult::CloseModal,
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.template_cursor_up();
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.template_cursor_down();
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                let cursor = self.state.template_cursor;
                self.state.create_from_template(cursor);
                self.state.view = QDesignView::Canvas;
                KeyHandleResult::Handled
            }
            KeyCode::Char('?') => {
                self.state.view = QDesignView::Help;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_canvas_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        // Handle Ctrl+E for export
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('e') {
            self.state.view = QDesignView::Export;
            return KeyHandleResult::Handled;
        }

        // Check if a frame is selected
        if self.state.selected_frame.is_some() {
            return self.handle_selected_frame_key(key);
        }

        // Handle based on current tool
        match self.state.tool {
            Tool::Select => self.handle_select_tool_key(key),
            Tool::TextFrame => self.handle_text_frame_tool_key(key),
        }
    }

    fn handle_select_tool_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.view = QDesignView::TemplateSelect;
                KeyHandleResult::Handled
            }
            KeyCode::Tab => {
                // Try to select next frame, or cycle tool
                let page = self.state.current_page();
                if page.map(|p| p.frames.is_empty()).unwrap_or(true) {
                    self.state.cycle_tool();
                } else {
                    self.state.select_next_frame();
                }
                KeyHandleResult::Handled
            }
            KeyCode::BackTab => {
                self.state.select_prev_frame();
                KeyHandleResult::Handled
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                self.state.tool = Tool::TextFrame;
                KeyHandleResult::Handled
            }
            KeyCode::Char('?') => {
                self.state.view = QDesignView::Help;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_text_frame_tool_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.tool = Tool::Select;
                KeyHandleResult::Handled
            }
            KeyCode::Tab => {
                self.state.cycle_tool();
                KeyHandleResult::Handled
            }
            KeyCode::Up => {
                self.state.move_cursor(0, -1);
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                self.state.move_cursor(0, 1);
                KeyHandleResult::Handled
            }
            KeyCode::Left => {
                self.state.move_cursor(-1, 0);
                KeyHandleResult::Handled
            }
            KeyCode::Right => {
                self.state.move_cursor(1, 0);
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                // Create a new text frame at cursor position
                let x = self.state.cursor_x;
                let y = self.state.cursor_y;
                self.state.add_text_frame(x, y, 20, 3);
                self.state.tool = Tool::Select;
                // Start editing the new frame
                self.state.start_text_edit();
                KeyHandleResult::Handled
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.state.tool = Tool::Select;
                KeyHandleResult::Handled
            }
            KeyCode::Char('?') => {
                self.state.view = QDesignView::Help;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_selected_frame_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.selected_frame = None;
                KeyHandleResult::Handled
            }
            KeyCode::Tab => {
                self.state.select_next_frame();
                KeyHandleResult::Handled
            }
            KeyCode::BackTab => {
                self.state.select_prev_frame();
                KeyHandleResult::Handled
            }
            KeyCode::Up => {
                self.state.move_selected_frame(0, -1);
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                self.state.move_selected_frame(0, 1);
                KeyHandleResult::Handled
            }
            KeyCode::Left => {
                self.state.move_selected_frame(-1, 0);
                KeyHandleResult::Handled
            }
            KeyCode::Right => {
                self.state.move_selected_frame(1, 0);
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                self.state.start_text_edit();
                KeyHandleResult::Handled
            }
            KeyCode::Delete | KeyCode::Backspace => {
                self.state.delete_selected_frame();
                KeyHandleResult::Handled
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.state.cycle_alignment();
                KeyHandleResult::Handled
            }
            KeyCode::Char('b') | KeyCode::Char('B') => {
                self.state.toggle_border();
                KeyHandleResult::Handled
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                self.state.selected_frame = None;
                self.state.tool = Tool::TextFrame;
                KeyHandleResult::Handled
            }
            KeyCode::Char('?') => {
                self.state.view = QDesignView::Help;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_text_edit_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.cancel_text_edit();
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                self.state.apply_text_edit();
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                self.state.text_edit_buffer.pop();
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                self.state.text_edit_buffer.push(c);
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_export_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.view = QDesignView::Canvas;
                self.state.status_message = None;
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                let path = self.state.export_path.clone();
                match self.state.export_ascii(&path) {
                    Ok(msg) => {
                        self.state.status_message = Some(msg);
                    }
                    Err(e) => {
                        self.state.status_message = Some(format!("Error: {}", e));
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                self.state.export_path.pop();
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                self.state.export_path.push(c);
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_help_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                // Return to previous view (canvas or template)
                if self.state.pages.is_empty() {
                    self.state.view = QDesignView::TemplateSelect;
                } else {
                    self.state.view = QDesignView::Canvas;
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }
}

// =============================================================================
// PLUGIN TRAIT
// =============================================================================

impl Plugin for QDesignPlugin {
    fn id(&self) -> &str {
        "qdesign"
    }

    fn name(&self) -> &str {
        "Q-DESIGN"
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
            name: "Q-DESIGN".to_string(),
            description: "Print designer for cards and banners".to_string(),
            category: PluginCategory::Tools,
            key: 'D',
        })
    }

    fn launch(&mut self, _cwd: &PathBuf, _selected_file: Option<&PathBuf>) -> Result<(), String> {
        self.state = QDesignState::new();
        self.state.view = QDesignView::TemplateSelect;
        Ok(())
    }

    fn handle_global_key(
        &mut self,
        _key: KeyEvent,
        _cwd: &PathBuf,
        _selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
        // Q-DESIGN is launched via Apps menu (F12) which calls launch()
        KeyHandleResult::NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match self.state.view {
            QDesignView::TemplateSelect => self.handle_template_select_key(key),
            QDesignView::Canvas => self.handle_canvas_key(key),
            QDesignView::TextEdit => self.handle_text_edit_key(key),
            QDesignView::Export => self.handle_export_key(key),
            QDesignView::Help => self.handle_help_key(key),
        }
    }

    fn draw_modal(&self, frame: &mut RatatuiFrame, area: Rect, colors: &ThemeColors) {
        modal::draw_qdesign(&self.state, frame, area, colors);
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "Q-DESIGN - Print Designer".to_string(),
            "".to_string(),
            "Create cards, banners, flyers and more.".to_string(),
            "".to_string(),
            "Templates:".to_string(),
            "  Up/Down    Select template".to_string(),
            "  Enter      Start designing".to_string(),
            "".to_string(),
            "Canvas:".to_string(),
            "  Tab        Cycle tools / Select frames".to_string(),
            "  T          Text frame tool".to_string(),
            "  Arrows     Move cursor/frame".to_string(),
            "  Enter      Create frame / Edit text".to_string(),
            "  A          Cycle alignment".to_string(),
            "  B          Toggle border".to_string(),
            "  Del        Delete frame".to_string(),
            "  Ctrl+E     Export".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
