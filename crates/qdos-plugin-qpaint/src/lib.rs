//! Q-PAINT Plugin
//!
//! MS Paint / Mario Paint inspired pixel art editor for R-DOS.

mod canvas;
mod file_io;
mod modal;
mod state;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use qdos_plugin_api::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, ThemeColors,
};
use ratatui::{layout::Rect, Frame};
use state::{FileMode, QPaintState, QPaintView, Tool};
use std::any::Any;
use std::path::PathBuf;

/// Q-PAINT Plugin
pub struct QPaintPlugin {
    state: QPaintState,
    modal_open: bool,
}

impl Default for QPaintPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl QPaintPlugin {
    pub fn new() -> Self {
        let mut state = QPaintState::default();
        file_io::apply_config(&mut state);
        Self {
            state,
            modal_open: false,
        }
    }

    /// Handle editor view keys
    #[allow(clippy::ptr_arg)]
    fn handle_editor_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);

        match key.code {
            // Navigation
            KeyCode::Up => {
                self.state.move_cursor(0, -1);
                if shift && matches!(self.state.tool, Tool::Pencil | Tool::Brush | Tool::Eraser) {
                    self.draw_at_cursor();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                self.state.move_cursor(0, 1);
                if shift && matches!(self.state.tool, Tool::Pencil | Tool::Brush | Tool::Eraser) {
                    self.draw_at_cursor();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Left => {
                self.state.move_cursor(-1, 0);
                if shift && matches!(self.state.tool, Tool::Pencil | Tool::Brush | Tool::Eraser) {
                    self.draw_at_cursor();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Right => {
                self.state.move_cursor(1, 0);
                if shift && matches!(self.state.tool, Tool::Pencil | Tool::Brush | Tool::Eraser) {
                    self.draw_at_cursor();
                }
                KeyHandleResult::Handled
            }

            // Draw/Apply tool
            KeyCode::Char(' ') => {
                self.apply_current_tool();
                KeyHandleResult::Handled
            }

            // Tool selection
            KeyCode::Char('p') | KeyCode::Char('P') if !ctrl => {
                self.state.tool = Tool::Pencil;
                self.state.status = "Pencil tool selected".to_string();
                KeyHandleResult::Handled
            }
            KeyCode::Char('b') | KeyCode::Char('B') if !ctrl => {
                self.state.tool = Tool::Brush;
                self.state.status = "Brush tool selected".to_string();
                KeyHandleResult::Handled
            }
            KeyCode::Char('l') | KeyCode::Char('L') if !ctrl => {
                self.state.tool = Tool::Line;
                self.state.shape_start = None;
                self.state.status = "Line tool - Space to set points".to_string();
                KeyHandleResult::Handled
            }
            KeyCode::Char('e') | KeyCode::Char('E') if !ctrl => {
                self.state.tool = Tool::Eraser;
                self.state.status = "Eraser tool selected".to_string();
                KeyHandleResult::Handled
            }
            KeyCode::Char('t') | KeyCode::Char('T') if !ctrl => {
                self.state.tool = Tool::Text;
                self.state.status = "Text tool - Space to place text".to_string();
                KeyHandleResult::Handled
            }
            KeyCode::Char('s') | KeyCode::Char('S') if !ctrl => {
                self.state.tool = Tool::Select;
                self.state.selection.active = false;
                self.state.status = "Select tool - Space to set corners".to_string();
                KeyHandleResult::Handled
            }
            KeyCode::Char('i') | KeyCode::Char('I') if !ctrl => {
                self.state.tool = Tool::ColorPicker;
                self.state.status = "Color picker - Space to pick color".to_string();
                KeyHandleResult::Handled
            }

            // Zoom
            KeyCode::Char('z') if !ctrl && !shift => {
                self.state.zoom_in();
                self.state.status = format!("Zoom: {}x", self.state.zoom);
                KeyHandleResult::Handled
            }
            KeyCode::Char('Z') if !ctrl => {
                self.state.zoom_out();
                self.state.status = format!("Zoom: {}x", self.state.zoom);
                KeyHandleResult::Handled
            }

            // Brush size
            KeyCode::Char('+') | KeyCode::Char('=') => {
                if self.state.brush_size < 16 {
                    self.state.brush_size += 1;
                }
                self.state.status = format!("Brush size: {}", self.state.brush_size);
                KeyHandleResult::Handled
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                if self.state.brush_size > 1 {
                    self.state.brush_size -= 1;
                }
                self.state.status = format!("Brush size: {}", self.state.brush_size);
                KeyHandleResult::Handled
            }

            // Quick palette (1-9, 0)
            KeyCode::Char(c @ '1'..='9') if !ctrl => {
                let idx = (c as u8 - b'1') as usize;
                self.state.set_fg_from_palette(idx);
                self.state.status = format!("Color {} selected", idx);
                KeyHandleResult::Handled
            }
            KeyCode::Char('0') if !ctrl => {
                self.state.set_fg_from_palette(9);
                self.state.status = "Color 9 selected".to_string();
                KeyHandleResult::Handled
            }

            // Palette view
            KeyCode::Tab => {
                self.state.view = QPaintView::Palette;
                KeyHandleResult::Handled
            }

            // File operations
            KeyCode::Char('n') | KeyCode::Char('N') if ctrl => {
                self.state.view = QPaintView::FileMenu;
                self.state.file_mode = FileMode::New;
                self.state.file_input = "32x32".to_string();
                KeyHandleResult::Handled
            }
            KeyCode::Char('o') | KeyCode::Char('O') if ctrl => {
                self.state.view = QPaintView::FileMenu;
                self.state.file_mode = FileMode::Open;
                self.state.file_list = file_io::list_images(cwd);
                self.state.file_selected = 0;
                KeyHandleResult::Handled
            }
            KeyCode::Char('s') | KeyCode::Char('S') if ctrl && !shift => {
                if let Some(path) = self.state.file_path.clone() {
                    match file_io::save_image(&self.state, &path) {
                        Ok(()) => {
                            self.state.modified = false;
                            let _ = file_io::save_config(&self.state);
                            self.state.status = "Saved!".to_string();
                        }
                        Err(e) => {
                            self.state.status = format!("Error: {}", e);
                        }
                    }
                } else {
                    self.state.view = QPaintView::FileMenu;
                    self.state.file_mode = FileMode::Save;
                    self.state.file_list = file_io::list_images(cwd);
                    self.state.file_input = "untitled.png".to_string();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('S') if ctrl && shift => {
                self.state.view = QPaintView::FileMenu;
                self.state.file_mode = FileMode::Save;
                self.state.file_list = file_io::list_images(cwd);
                self.state.file_input = self
                    .state
                    .file_path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "untitled.png".to_string());
                KeyHandleResult::Handled
            }
            KeyCode::Char('e') | KeyCode::Char('E') if ctrl => {
                // Export as ANSI
                let path = cwd.join("export.ans");
                match file_io::export_ansi(&self.state, &path) {
                    Ok(()) => {
                        self.state.status = format!("Exported to {}", path.display());
                    }
                    Err(e) => {
                        self.state.status = format!("Export error: {}", e);
                    }
                }
                KeyHandleResult::Handled
            }

            // Undo/Redo
            KeyCode::Char('z') | KeyCode::Char('Z') if ctrl => {
                if self.state.undo() {
                    self.state.status = "Undo".to_string();
                } else {
                    self.state.status = "Nothing to undo".to_string();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('y') | KeyCode::Char('Y') if ctrl => {
                if self.state.redo() {
                    self.state.status = "Redo".to_string();
                } else {
                    self.state.status = "Nothing to redo".to_string();
                }
                KeyHandleResult::Handled
            }

            // Clipboard
            KeyCode::Char('c') | KeyCode::Char('C') if ctrl => {
                self.state.copy_selection();
                self.state.status = "Copied to clipboard".to_string();
                KeyHandleResult::Handled
            }
            KeyCode::Char('v') | KeyCode::Char('V') if ctrl => {
                self.state.paste();
                self.state.status = "Pasted from clipboard".to_string();
                KeyHandleResult::Handled
            }
            KeyCode::Delete => {
                self.state.clear_selection();
                self.state.status = "Selection cleared".to_string();
                KeyHandleResult::Handled
            }

            // Help
            KeyCode::Char('?') | KeyCode::F(1) => {
                self.state.view = QPaintView::Help;
                KeyHandleResult::Handled
            }

            // Exit
            KeyCode::Esc => {
                self.modal_open = false;
                let _ = file_io::save_config(&self.state);
                KeyHandleResult::CloseModal
            }

            _ => KeyHandleResult::Handled,
        }
    }

    /// Draw at current cursor position
    fn draw_at_cursor(&mut self) {
        self.state.save_undo();
        let x = self.state.cursor_x;
        let y = self.state.cursor_y;
        let color = if self.state.tool == Tool::Eraser {
            self.state.bg_color
        } else {
            self.state.fg_color
        };
        canvas::draw_pixel(&mut self.state.canvas, x, y, color, self.state.brush_size);
    }

    /// Apply the current tool at cursor position
    fn apply_current_tool(&mut self) {
        let x = self.state.cursor_x;
        let y = self.state.cursor_y;

        match self.state.tool {
            Tool::Pencil | Tool::Brush => {
                self.state.save_undo();
                canvas::draw_pixel(
                    &mut self.state.canvas,
                    x,
                    y,
                    self.state.fg_color,
                    self.state.brush_size,
                );
            }
            Tool::Eraser => {
                self.state.save_undo();
                canvas::draw_pixel(
                    &mut self.state.canvas,
                    x,
                    y,
                    self.state.bg_color,
                    self.state.brush_size,
                );
            }
            Tool::Line => {
                if let Some((sx, sy)) = self.state.shape_start {
                    self.state.save_undo();
                    canvas::draw_line(&mut self.state.canvas, sx, sy, x, y, self.state.fg_color);
                    self.state.shape_start = None;
                    self.state.status = "Line drawn".to_string();
                } else {
                    self.state.shape_start = Some((x, y));
                    self.state.status = "Start point set - move and press Space".to_string();
                }
            }
            Tool::Text => {
                // Text tool placeholder - will show text input dialog
                self.state.status = "Text tool: Type to add text (not yet implemented)".to_string();
            }
            Tool::ColorPicker => {
                let color = self.state.canvas.get_pixel(x, y);
                self.state.fg_color = color;
                self.state.status =
                    format!("Picked color: RGB({},{},{})", color.0, color.1, color.2);
            }
            Tool::Select => {
                if self.state.selection.active {
                    // End selection
                    self.state.selection.end_x = x;
                    self.state.selection.end_y = y;
                    let (w, h) = self.state.selection.size();
                    self.state.status = format!("Selected {}x{}", w, h);
                } else {
                    // Start selection
                    self.state.selection.start_x = x;
                    self.state.selection.start_y = y;
                    self.state.selection.end_x = x;
                    self.state.selection.end_y = y;
                    self.state.selection.active = true;
                    self.state.status = "Selection started - move and press Space".to_string();
                }
            }
        }
    }

    /// Handle palette view keys
    fn handle_palette_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);

        match key.code {
            // Hex palette selection
            KeyCode::Char(c @ '0'..='9') => {
                let idx = (c as u8 - b'0') as usize;
                self.state.palette_index = idx;
                KeyHandleResult::Handled
            }
            KeyCode::Char(c @ 'a'..='f') | KeyCode::Char(c @ 'A'..='F') => {
                let idx = (c.to_ascii_lowercase() as u8 - b'a' + 10) as usize;
                self.state.palette_index = idx;
                KeyHandleResult::Handled
            }

            // Arrow navigation
            KeyCode::Left => {
                if self.state.palette_index > 0 {
                    self.state.palette_index -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Right => {
                if self.state.palette_index < 15 {
                    self.state.palette_index += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Up => {
                if self.state.palette_index >= 8 {
                    self.state.palette_index -= 8;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                if self.state.palette_index < 8 {
                    self.state.palette_index += 8;
                }
                KeyHandleResult::Handled
            }

            // Set color
            KeyCode::Enter => {
                if shift {
                    self.state.set_bg_from_palette(self.state.palette_index);
                } else {
                    self.state.set_fg_from_palette(self.state.palette_index);
                }
                self.state.view = QPaintView::Editor;
                KeyHandleResult::Handled
            }

            KeyCode::Esc | KeyCode::Tab => {
                self.state.view = QPaintView::Editor;
                KeyHandleResult::Handled
            }

            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle file menu keys
    #[allow(clippy::ptr_arg)]
    fn handle_file_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        match key.code {
            KeyCode::Up => {
                if self.state.file_selected > 0 {
                    self.state.file_selected -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                if self.state.file_selected < self.state.file_list.len().saturating_sub(1) {
                    self.state.file_selected += 1;
                }
                KeyHandleResult::Handled
            }

            KeyCode::Enter => {
                match self.state.file_mode {
                    FileMode::Open => {
                        if let Some(filename) =
                            self.state.file_list.get(self.state.file_selected).cloned()
                        {
                            let path = cwd.join(&filename);
                            match file_io::load_image(&mut self.state, &path) {
                                Ok(()) => {
                                    self.state.status = format!("Loaded {}", filename);
                                }
                                Err(e) => {
                                    self.state.status = format!("Error: {}", e);
                                }
                            }
                        }
                        self.state.view = QPaintView::Editor;
                    }
                    FileMode::Save => {
                        if !self.state.file_input.is_empty() {
                            let path = cwd.join(&self.state.file_input);
                            match file_io::save_image(&self.state, &path) {
                                Ok(()) => {
                                    self.state.file_path = Some(path);
                                    self.state.modified = false;
                                    let _ = file_io::save_config(&self.state);
                                    self.state.status = format!("Saved {}", self.state.file_input);
                                }
                                Err(e) => {
                                    self.state.status = format!("Error: {}", e);
                                }
                            }
                        }
                        self.state.view = QPaintView::Editor;
                    }
                    FileMode::New => {
                        // Parse dimensions
                        let parts: Vec<&str> = self.state.file_input.split('x').collect();
                        if parts.len() == 2 {
                            if let (Ok(w), Ok(h)) =
                                (parts[0].parse::<u32>(), parts[1].parse::<u32>())
                            {
                                let w = w.clamp(1, 256);
                                let h = h.clamp(1, 256);
                                self.state.new_canvas(w, h);
                                self.state.status = format!("New canvas {}x{}", w, h);
                            }
                        }
                        self.state.view = QPaintView::Editor;
                    }
                }
                KeyHandleResult::Handled
            }

            KeyCode::Char(c) => {
                if matches!(self.state.file_mode, FileMode::Save | FileMode::New) {
                    self.state.file_input.push(c);
                }
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                if matches!(self.state.file_mode, FileMode::Save | FileMode::New) {
                    self.state.file_input.pop();
                }
                KeyHandleResult::Handled
            }

            KeyCode::Esc => {
                self.state.view = QPaintView::Editor;
                KeyHandleResult::Handled
            }

            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle help view keys
    fn handle_help_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('?') | KeyCode::F(1) => {
                self.state.view = QPaintView::Editor;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }
}

impl Plugin for QPaintPlugin {
    fn id(&self) -> &str {
        "qpaint"
    }

    fn name(&self) -> &str {
        "Q-PAINT"
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
        true // Always available
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: "qpaint".to_string(),
            name: "Q-PAINT".to_string(),
            description: "Pixel art editor - MS Paint/Mario Paint inspired".to_string(),
            category: PluginCategory::Tools,
            key: 'P',
        })
    }

    fn launch(&mut self, _cwd: &PathBuf, _selected_file: Option<&PathBuf>) -> Result<(), String> {
        self.modal_open = true;
        self.state.view = QPaintView::Editor;
        self.state.status.clear();

        // Scale canvas to terminal size if it's still the default small size
        if self.state.canvas.width == 32 && self.state.canvas.height == 32 {
            // Query terminal size and create appropriately sized canvas
            if let Ok((cols, rows)) = crossterm::terminal::size() {
                // Calculate usable canvas area:
                // - Subtract 4 for UI chrome (title, separators, help, status bar)
                // - Subtract 2 for toolbar rows
                // - Account for zoom level (default is 4x)
                let available_height = rows.saturating_sub(8) as u32;
                let available_width = cols as u32;

                // At default zoom 4x, each pixel takes ~4 chars wide
                // Calculate canvas size that fits well
                let zoom = self.state.zoom as u32;
                let canvas_width = (available_width / zoom).clamp(32, 256);
                let canvas_height = (available_height * 2 / zoom).clamp(32, 256);

                // Round to nice multiples of 8 for pixel art
                let canvas_width = (canvas_width / 8) * 8;
                let canvas_height = (canvas_height / 8) * 8;

                if canvas_width > 32 || canvas_height > 32 {
                    self.state
                        .new_canvas(canvas_width.max(32), canvas_height.max(32));
                    self.state.status = format!(
                        "Canvas: {}x{}",
                        self.state.canvas.width, self.state.canvas.height
                    );
                }
            }
        }

        Ok(())
    }

    fn handle_global_key(
        &mut self,
        key: KeyEvent,
        _cwd: &PathBuf,
        _selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
        // Open Q-PAINT from Apps menu
        if key.code == KeyCode::Enter {
            self.modal_open = true;
            self.state.view = QPaintView::Editor;
            self.state.status.clear();
            return KeyHandleResult::OpenModal;
        }
        KeyHandleResult::NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        match self.state.view {
            QPaintView::Editor => self.handle_editor_key(key, cwd),
            QPaintView::Palette => self.handle_palette_key(key),
            QPaintView::FileMenu => self.handle_file_key(key, cwd),
            QPaintView::Help => self.handle_help_key(key),
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        modal::render(frame, area, &self.state, colors);
    }

    fn tick(&mut self) {
        // No animation needed
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// Plugin registration
inventory::submit! {
    qdos_plugin_api::PluginRegistration::new("qpaint", || Box::new(QPaintPlugin::new()))
}
