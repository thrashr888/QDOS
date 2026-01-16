//! Q-DOCS Word Processor
//!
//! A Markdown-focused word processor with preview mode and export capabilities.

pub mod modal;
pub mod state;

use crate::app::ThemeColors;
use crate::plugins::{AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{layout::Rect, Frame};
use state::{DocsMode, DocsState, InputMode, MenuCategory, MenuItem};
use std::any::Any;
use std::path::PathBuf;

// =============================================================================
// DOCS PLUGIN
// =============================================================================

pub struct DocsPlugin {
    state: Option<DocsState>,
}

impl Default for DocsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl DocsPlugin {
    pub fn new() -> Self {
        Self { state: None }
    }

    pub fn launch(&mut self) {
        self.state = Some(DocsState::new());
    }

    pub fn launch_with_file(&mut self, path: PathBuf) -> Result<(), String> {
        match DocsState::load_file(path) {
            Ok(state) => {
                self.state = Some(state);
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    }

    // =========================================================================
    // KEY HANDLING
    // =========================================================================

    fn handle_edit_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        let state = self.state.as_mut().unwrap();
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            // Exit
            KeyCode::Esc => {
                if state.input_mode != InputMode::Normal {
                    state.input_mode = InputMode::Normal;
                    KeyHandleResult::Handled
                } else {
                    KeyHandleResult::CloseModal
                }
            }

            // Menu
            KeyCode::F(10) => {
                state.mode = DocsMode::Menu;
                state.menu_category = 0;
                state.menu_item = 0;
                KeyHandleResult::Handled
            }

            // Preview mode
            KeyCode::F(9) => {
                state.mode = DocsMode::Preview;
                state.preview_scroll = state.scroll_offset;
                KeyHandleResult::Handled
            }

            // Save
            KeyCode::Char('s') if ctrl => {
                if state.file_path.is_some() {
                    match state.save() {
                        Ok(()) => {
                            state.status_message = Some(("Saved".to_string(), 30));
                        }
                        Err(e) => {
                            state.status_message = Some((format!("Error: {}", e), 60));
                        }
                    }
                } else {
                    state.mode = DocsMode::SaveAs;
                    state.save_as_input.clear();
                    state.save_as_cursor = 0;
                }
                KeyHandleResult::Handled
            }

            // Undo/Redo
            KeyCode::Char('z') if ctrl => {
                state.undo();
                KeyHandleResult::Handled
            }
            KeyCode::Char('y') if ctrl => {
                state.redo();
                KeyHandleResult::Handled
            }

            // Find/Replace
            KeyCode::Char('f') if ctrl => {
                state.mode = DocsMode::Find;
                state.find_query.clear();
                KeyHandleResult::Handled
            }
            KeyCode::Char('h') if ctrl => {
                state.mode = DocsMode::Replace;
                state.find_query.clear();
                state.replace_text.clear();
                KeyHandleResult::Handled
            }

            // Formatting shortcuts
            KeyCode::Char('b') if ctrl => {
                state.toggle_bold();
                KeyHandleResult::Handled
            }
            KeyCode::Char('i') if ctrl => {
                state.toggle_italic();
                KeyHandleResult::Handled
            }

            // Help
            KeyCode::F(1) => {
                state.mode = DocsMode::Help;
                KeyHandleResult::Handled
            }

            // Enter insert mode
            KeyCode::Char('i') if state.input_mode == InputMode::Normal => {
                state.input_mode = InputMode::Insert;
                KeyHandleResult::Handled
            }

            // Insert key toggles insert/overwrite
            KeyCode::Insert => {
                state.input_mode = state.input_mode.toggle();
                KeyHandleResult::Handled
            }

            // Navigation
            KeyCode::Up => {
                state.move_up();
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                state.move_down();
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
            KeyCode::Home if ctrl => {
                state.move_top();
                KeyHandleResult::Handled
            }
            KeyCode::End if ctrl => {
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
                let visible = 20; // Approximate
                for _ in 0..visible {
                    state.move_up();
                }
                KeyHandleResult::Handled
            }
            KeyCode::PageDown => {
                let visible = 20;
                for _ in 0..visible {
                    state.move_down();
                }
                KeyHandleResult::Handled
            }

            // Editing (only in insert/overwrite mode)
            KeyCode::Enter if state.input_mode != InputMode::Normal => {
                state.insert_newline();
                KeyHandleResult::Handled
            }
            KeyCode::Backspace if state.input_mode != InputMode::Normal => {
                state.backspace();
                KeyHandleResult::Handled
            }
            KeyCode::Delete if state.input_mode != InputMode::Normal => {
                state.delete_char();
                KeyHandleResult::Handled
            }
            KeyCode::Tab if state.input_mode != InputMode::Normal => {
                // Insert 4 spaces for tab
                for _ in 0..4 {
                    state.insert_char(' ');
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) if state.input_mode != InputMode::Normal => {
                state.insert_char(c);
                KeyHandleResult::Handled
            }

            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_preview_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        let state = self.state.as_mut().unwrap();

        match key.code {
            KeyCode::Esc | KeyCode::F(9) => {
                state.mode = DocsMode::Edit;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                state.preview_scroll = state.preview_scroll.saturating_sub(1);
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                state.preview_scroll += 1;
                KeyHandleResult::Handled
            }
            KeyCode::PageUp => {
                state.preview_scroll = state.preview_scroll.saturating_sub(20);
                KeyHandleResult::Handled
            }
            KeyCode::PageDown => {
                state.preview_scroll += 20;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_menu_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        let state = self.state.as_mut().unwrap();

        match key.code {
            KeyCode::Esc => {
                state.mode = DocsMode::Edit;
                KeyHandleResult::Handled
            }
            KeyCode::Left => {
                if state.menu_category > 0 {
                    state.menu_category -= 1;
                    state.menu_item = 0;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Right => {
                if state.menu_category < MenuCategory::all().len() - 1 {
                    state.menu_category += 1;
                    state.menu_item = 0;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Up => {
                if state.menu_item > 0 {
                    state.menu_item -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                let items = MenuCategory::all()[state.menu_category].items();
                if state.menu_item < items.len() - 1 {
                    state.menu_item += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                let category = MenuCategory::all()[state.menu_category];
                let item = category.items()[state.menu_item];
                self.execute_menu_item(item)
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn execute_menu_item(&mut self, item: MenuItem) -> KeyHandleResult {
        let state = self.state.as_mut().unwrap();
        state.mode = DocsMode::Edit;

        match item {
            MenuItem::New => {
                self.state = Some(DocsState::new());
                KeyHandleResult::Handled
            }
            MenuItem::Save => {
                if state.file_path.is_some() {
                    match state.save() {
                        Ok(()) => state.status_message = Some(("Saved".to_string(), 30)),
                        Err(e) => state.status_message = Some((format!("Error: {}", e), 60)),
                    }
                } else {
                    state.mode = DocsMode::SaveAs;
                }
                KeyHandleResult::Handled
            }
            MenuItem::SaveAs => {
                state.mode = DocsMode::SaveAs;
                state.save_as_input.clear();
                KeyHandleResult::Handled
            }
            MenuItem::Quit => KeyHandleResult::CloseModal,
            MenuItem::Undo => {
                state.undo();
                KeyHandleResult::Handled
            }
            MenuItem::Redo => {
                state.redo();
                KeyHandleResult::Handled
            }
            MenuItem::Find => {
                state.mode = DocsMode::Find;
                KeyHandleResult::Handled
            }
            MenuItem::Replace => {
                state.mode = DocsMode::Replace;
                KeyHandleResult::Handled
            }
            MenuItem::Preview => {
                state.mode = DocsMode::Preview;
                KeyHandleResult::Handled
            }
            MenuItem::WordWrap => {
                state.word_wrap = !state.word_wrap;
                KeyHandleResult::Handled
            }
            MenuItem::LineNumbers => {
                state.show_line_numbers = !state.show_line_numbers;
                KeyHandleResult::Handled
            }
            MenuItem::Heading => {
                state.insert_heading(1);
                KeyHandleResult::Handled
            }
            MenuItem::Bold => {
                state.toggle_bold();
                KeyHandleResult::Handled
            }
            MenuItem::Italic => {
                state.toggle_italic();
                KeyHandleResult::Handled
            }
            MenuItem::Link => {
                state.insert_link();
                KeyHandleResult::Handled
            }
            MenuItem::Code => {
                state.insert_code_block();
                KeyHandleResult::Handled
            }
            MenuItem::HorizontalRule => {
                state.insert_horizontal_rule();
                KeyHandleResult::Handled
            }
            MenuItem::WordCount => {
                let count = state.word_count();
                state.status_message = Some((format!("Word count: {}", count), 60));
                KeyHandleResult::Handled
            }
            MenuItem::Statistics => {
                let words = state.word_count();
                let chars = state.char_count();
                let lines = state.line_count();
                let pages = state.page_count();
                state.status_message = Some((
                    format!(
                        "{} words, {} chars, {} lines, ~{} pages",
                        words, chars, lines, pages
                    ),
                    90,
                ));
                KeyHandleResult::Handled
            }
            MenuItem::HelpTopic => {
                state.mode = DocsMode::Help;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_find_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        let state = self.state.as_mut().unwrap();

        match key.code {
            KeyCode::Esc => {
                state.mode = DocsMode::Edit;
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                // Perform search
                let query = state.find_query.to_lowercase();
                state.find_results.clear();

                for (line_idx, line) in state.lines.iter().enumerate() {
                    let line_lower = line.to_lowercase();
                    let mut start = 0;
                    while let Some(pos) = line_lower[start..].find(&query) {
                        state.find_results.push((line_idx, start + pos));
                        start += pos + 1;
                    }
                }

                state.find_index = 0;
                if !state.find_results.is_empty() {
                    let (line, col) = state.find_results[0];
                    state.cursor_line = line;
                    state.cursor_col = col;
                    state.status_message =
                        Some((format!("Found {} matches", state.find_results.len()), 30));
                } else {
                    state.status_message = Some(("No matches found".to_string(), 30));
                }
                state.mode = DocsMode::Edit;
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                state.find_query.push(c);
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                state.find_query.pop();
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_replace_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        let state = self.state.as_mut().unwrap();

        match key.code {
            KeyCode::Esc => {
                state.mode = DocsMode::Edit;
                KeyHandleResult::Handled
            }
            KeyCode::Tab => {
                // Toggle between find and replace fields
                // For simplicity, we just handle the replace field
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                state.replace_text.push(c);
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                state.replace_text.pop();
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_save_as_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let state = self.state.as_mut().unwrap();

        match key.code {
            KeyCode::Esc => {
                state.mode = DocsMode::Edit;
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                if !state.save_as_input.is_empty() {
                    let path = if state.save_as_input.starts_with('/') {
                        PathBuf::from(&state.save_as_input)
                    } else {
                        cwd.join(&state.save_as_input)
                    };

                    state.file_path = Some(path);
                    match state.save() {
                        Ok(()) => {
                            state.status_message =
                                Some((format!("Saved as {}", state.display_name()), 60));
                        }
                        Err(e) => {
                            state.status_message = Some((format!("Error: {}", e), 60));
                        }
                    }
                }
                state.mode = DocsMode::Edit;
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                state.save_as_input.insert(state.save_as_cursor, c);
                state.save_as_cursor += 1;
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                if state.save_as_cursor > 0 {
                    state.save_as_cursor -= 1;
                    state.save_as_input.remove(state.save_as_cursor);
                }
                KeyHandleResult::Handled
            }
            KeyCode::Left => {
                state.save_as_cursor = state.save_as_cursor.saturating_sub(1);
                KeyHandleResult::Handled
            }
            KeyCode::Right => {
                if state.save_as_cursor < state.save_as_input.len() {
                    state.save_as_cursor += 1;
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_help_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        let state = self.state.as_mut().unwrap();

        if matches!(key.code, KeyCode::Esc | KeyCode::F(1)) {
            state.mode = DocsMode::Edit;
        }
        KeyHandleResult::Handled
    }
}

// =============================================================================
// PLUGIN IMPLEMENTATION
// =============================================================================

impl Plugin for DocsPlugin {
    fn id(&self) -> &str {
        "docs"
    }

    fn name(&self) -> &str {
        "Q-DOCS"
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
            name: "Q-DOCS".to_string(),
            description: "Word processor with Markdown support".to_string(),
            category: PluginCategory::Tools,
            key: 'D',
        })
    }

    fn launch(&mut self, _cwd: &PathBuf, selected_file: Option<&PathBuf>) -> Result<(), String> {
        if let Some(path) = selected_file {
            self.launch_with_file(path.clone())
        } else {
            self.launch();
            Ok(())
        }
    }

    fn handle_global_key(
        &mut self,
        _key: KeyEvent,
        _cwd: &PathBuf,
        _selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
        KeyHandleResult::NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        if self.state.is_none() {
            return KeyHandleResult::NotHandled;
        }

        let mode = self.state.as_ref().unwrap().mode;

        match mode {
            DocsMode::Edit => self.handle_edit_key(key),
            DocsMode::Preview => self.handle_preview_key(key),
            DocsMode::Menu => self.handle_menu_key(key),
            DocsMode::Find => self.handle_find_key(key),
            DocsMode::Replace => self.handle_replace_key(key),
            DocsMode::SaveAs => self.handle_save_as_key(key, cwd),
            DocsMode::Help => self.handle_help_key(key),
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        if let Some(state) = &self.state {
            modal::draw_docs_modal(frame, area, state, colors);
        }
    }

    fn tick(&mut self) {
        if let Some(state) = &mut self.state {
            // Decrement status message timer
            if let Some((_, ref mut ticks)) = state.status_message {
                if *ticks > 0 {
                    *ticks -= 1;
                } else {
                    state.status_message = None;
                }
            }

            // Ensure cursor is visible
            state.ensure_visible(20);
        }
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "Q-DOCS Word Processor".to_string(),
            "".to_string(),
            "A Markdown-focused document editor.".to_string(),
            "".to_string(),
            "Navigation:".to_string(),
            "  Arrow keys    Move cursor".to_string(),
            "  Home/End      Line start/end".to_string(),
            "  PgUp/PgDn     Scroll pages".to_string(),
            "  Ctrl+Home     Document start".to_string(),
            "  Ctrl+End      Document end".to_string(),
            "".to_string(),
            "Editing:".to_string(),
            "  i             Enter insert mode".to_string(),
            "  Ins           Toggle insert/overwrite".to_string(),
            "  Ctrl+Z        Undo".to_string(),
            "  Ctrl+Y        Redo".to_string(),
            "".to_string(),
            "Formatting:".to_string(),
            "  Ctrl+B        Bold".to_string(),
            "  Ctrl+I        Italic".to_string(),
            "".to_string(),
            "Commands:".to_string(),
            "  F10           Open menu".to_string(),
            "  F9            Preview mode".to_string(),
            "  Ctrl+S        Save".to_string(),
            "  Ctrl+F        Find".to_string(),
            "  Ctrl+H        Replace".to_string(),
            "  Esc           Close".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
