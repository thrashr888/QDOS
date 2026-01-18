//! Q-DOCS Word Processor
//!
//! A Markdown-focused word processor with preview mode and export capabilities.

pub mod modal;
pub mod state;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use qdos_plugin_api::ThemeColors;
use qdos_plugin_api::{AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory};
use ratatui::{layout::Rect, Frame};
use state::{DocsMode, DocsState, ExportFormat, InputMode, MenuCategory, MenuItem};
use std::any::Any;
use std::path::PathBuf;

pub mod ops;

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
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);

        match key.code {
            // Exit
            KeyCode::Esc => {
                if state.has_selection() {
                    state.clear_selection();
                    KeyHandleResult::Handled
                } else if state.input_mode != InputMode::Normal {
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

            // Page view toggle
            KeyCode::F(8) => {
                state.page_view_enabled = !state.page_view_enabled;
                let msg = if state.page_view_enabled {
                    "Page view ON"
                } else {
                    "Page view OFF"
                };
                state.status_message = Some((msg.to_string(), 30));
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

            // Clipboard operations (Phase 1)
            KeyCode::Char('x') if ctrl => {
                self.cut_selection();
                KeyHandleResult::Handled
            }
            KeyCode::Char('c') if ctrl => {
                self.copy_selection();
                KeyHandleResult::Handled
            }
            KeyCode::Char('v') if ctrl => {
                self.paste();
                KeyHandleResult::Handled
            }
            KeyCode::Char('a') if ctrl => {
                state.select_all();
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

            // Selection navigation (Shift+Arrow)
            KeyCode::Up if shift => {
                state.extend_selection();
                state.move_up();
                state.selection_end = Some((state.cursor_line, state.cursor_col));
                KeyHandleResult::Handled
            }
            KeyCode::Down if shift => {
                state.extend_selection();
                state.move_down();
                state.selection_end = Some((state.cursor_line, state.cursor_col));
                KeyHandleResult::Handled
            }
            KeyCode::Left if shift => {
                state.extend_selection();
                state.move_left();
                state.selection_end = Some((state.cursor_line, state.cursor_col));
                KeyHandleResult::Handled
            }
            KeyCode::Right if shift => {
                state.extend_selection();
                state.move_right();
                state.selection_end = Some((state.cursor_line, state.cursor_col));
                KeyHandleResult::Handled
            }
            KeyCode::Home if shift => {
                state.extend_selection();
                state.move_home();
                state.selection_end = Some((state.cursor_line, state.cursor_col));
                KeyHandleResult::Handled
            }
            KeyCode::End if shift => {
                state.extend_selection();
                state.move_end();
                state.selection_end = Some((state.cursor_line, state.cursor_col));
                KeyHandleResult::Handled
            }

            // Normal navigation (clears selection)
            KeyCode::Up => {
                state.clear_selection();
                state.move_up();
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                state.clear_selection();
                state.move_down();
                KeyHandleResult::Handled
            }
            KeyCode::Left => {
                state.clear_selection();
                state.move_left();
                KeyHandleResult::Handled
            }
            KeyCode::Right => {
                state.clear_selection();
                state.move_right();
                KeyHandleResult::Handled
            }
            KeyCode::Home if ctrl => {
                state.clear_selection();
                state.move_top();
                KeyHandleResult::Handled
            }
            KeyCode::End if ctrl => {
                state.clear_selection();
                state.move_bottom();
                KeyHandleResult::Handled
            }
            KeyCode::Home => {
                state.clear_selection();
                state.move_home();
                KeyHandleResult::Handled
            }
            KeyCode::End => {
                state.clear_selection();
                state.move_end();
                KeyHandleResult::Handled
            }

            // Page navigation
            KeyCode::PageUp if ctrl => {
                state.clear_selection();
                state.prev_page();
                KeyHandleResult::Handled
            }
            KeyCode::PageDown if ctrl => {
                state.clear_selection();
                state.next_page();
                KeyHandleResult::Handled
            }
            KeyCode::PageUp => {
                state.clear_selection();
                let visible = 20; // Approximate
                for _ in 0..visible {
                    state.move_up();
                }
                KeyHandleResult::Handled
            }
            KeyCode::PageDown => {
                state.clear_selection();
                let visible = 20;
                for _ in 0..visible {
                    state.move_down();
                }
                KeyHandleResult::Handled
            }

            // Editing (only in insert/overwrite mode)
            KeyCode::Enter if state.input_mode != InputMode::Normal => {
                state.delete_selection(); // Delete selection first if any
                state.insert_newline();
                KeyHandleResult::Handled
            }
            KeyCode::Backspace if state.input_mode != InputMode::Normal => {
                if !state.delete_selection() {
                    state.backspace();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Delete if state.input_mode != InputMode::Normal => {
                if !state.delete_selection() {
                    state.delete_char();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Tab if state.input_mode != InputMode::Normal => {
                state.delete_selection(); // Delete selection first if any
                state.insert_tab(); // Use tab stops instead of fixed 4 spaces
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) if state.input_mode != InputMode::Normal => {
                state.delete_selection(); // Delete selection first if any
                state.insert_char(c);
                KeyHandleResult::Handled
            }

            _ => KeyHandleResult::Handled,
        }
    }

    // =========================================================================
    // CLIPBOARD OPERATIONS (Phase 1)
    // =========================================================================

    fn cut_selection(&mut self) {
        let state = self.state.as_mut().unwrap();
        if let Some(text) = state.selected_text() {
            // Try system clipboard
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(&text);
            }
            // Also store in internal clipboard
            state.clipboard = text;
            state.delete_selection();
            state.status_message = Some(("Cut to clipboard".to_string(), 30));
        }
    }

    fn copy_selection(&mut self) {
        let state = self.state.as_mut().unwrap();
        if let Some(text) = state.selected_text() {
            // Try system clipboard
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(&text);
            }
            // Also store in internal clipboard
            state.clipboard = text;
            state.status_message = Some(("Copied to clipboard".to_string(), 30));
        }
    }

    fn paste(&mut self) {
        let state = self.state.as_mut().unwrap();

        // Try system clipboard first
        let text = if let Ok(mut clipboard) = arboard::Clipboard::new() {
            clipboard.get_text().ok()
        } else {
            None
        }
        .unwrap_or_else(|| state.clipboard.clone());

        if !text.is_empty() {
            // Delete existing selection first
            state.delete_selection();

            state.save_undo();
            // Insert text at cursor
            for c in text.chars() {
                if c == '\n' {
                    // Insert newline without auto-indent for paste
                    let rest = state.lines[state.cursor_line][state.cursor_col..].to_string();
                    state.lines[state.cursor_line].truncate(state.cursor_col);
                    state.cursor_line += 1;
                    state.lines.insert(state.cursor_line, rest);
                    state.cursor_col = 0;
                } else {
                    let line_len = state.lines[state.cursor_line].len();
                    if state.cursor_col > line_len {
                        state.cursor_col = line_len;
                    }
                    state.lines[state.cursor_line].insert(state.cursor_col, c);
                    state.cursor_col += 1;
                }
            }
            state.modified = true;
            state.status_message = Some(("Pasted".to_string(), 30));
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
        // Handle clipboard operations specially (need to call self methods)
        match item {
            MenuItem::Cut => {
                self.state.as_mut().unwrap().mode = DocsMode::Edit;
                self.cut_selection();
                return KeyHandleResult::Handled;
            }
            MenuItem::Copy => {
                self.state.as_mut().unwrap().mode = DocsMode::Edit;
                self.copy_selection();
                return KeyHandleResult::Handled;
            }
            MenuItem::Paste => {
                self.state.as_mut().unwrap().mode = DocsMode::Edit;
                self.paste();
                return KeyHandleResult::Handled;
            }
            _ => {}
        }

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
            MenuItem::Cut | MenuItem::Copy | MenuItem::Paste => {
                // Already handled above
                KeyHandleResult::Handled
            }
            MenuItem::Export => {
                state.mode = DocsMode::Export;
                state.export_format = ExportFormat::Html;
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

    fn handle_export_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let state = self.state.as_mut().unwrap();

        match key.code {
            KeyCode::Esc => {
                state.mode = DocsMode::Edit;
                KeyHandleResult::Handled
            }
            KeyCode::Up => {
                let formats = ExportFormat::all();
                let current = formats
                    .iter()
                    .position(|f| *f == state.export_format)
                    .unwrap_or(0);
                if current > 0 {
                    state.export_format = formats[current - 1];
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                let formats = ExportFormat::all();
                let current = formats
                    .iter()
                    .position(|f| *f == state.export_format)
                    .unwrap_or(0);
                if current + 1 < formats.len() {
                    state.export_format = formats[current + 1];
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                // Build default filename
                let base_name = state
                    .file_path
                    .as_ref()
                    .and_then(|p| p.file_stem())
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "document".to_string());
                let ext = state.export_format.extension();
                let output_name = format!("{}.{}", base_name, ext);
                let output_path = cwd.join(&output_name);

                let result = match state.export_format {
                    ExportFormat::Html => ops::export_html(&state.lines, &base_name, &output_path),
                    ExportFormat::Pdf => ops::export_pdf(&state.lines, &output_path, cwd),
                    ExportFormat::PlainText => ops::export_plain_text(&state.lines, &output_path),
                };

                match result {
                    Ok(()) => {
                        state.status_message = Some((format!("Exported to {}", output_name), 60));
                    }
                    Err(e) => {
                        state.status_message = Some((format!("Export error: {}", e), 90));
                    }
                }
                state.mode = DocsMode::Edit;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
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
            DocsMode::Export => self.handle_export_key(key, cwd),
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
