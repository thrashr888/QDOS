//! Q-SHEET Spreadsheet Editor
//!
//! A VisiCalc/Lotus 1-2-3 inspired spreadsheet editor with formula support.

pub mod csv;
pub mod formula;
pub mod modal;
pub mod state;
pub mod xlsx;

use crate::app::ThemeColors;
use crate::plugins::{AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{layout::Rect, Frame};
use state::{SheetMode, SheetState};
use std::any::Any;
use std::path::PathBuf;

/// Q-SHEET plugin implementation
pub struct SheetPlugin {
    state: Option<SheetState>,
}

impl Default for SheetPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl SheetPlugin {
    pub fn new() -> Self {
        Self { state: None }
    }

    /// Launch the spreadsheet editor
    pub fn launch(&mut self) -> KeyHandleResult {
        self.state = Some(SheetState::new());
        KeyHandleResult::Handled
    }

    /// Handle modal key events
    pub fn handle_modal_key(&mut self, key: KeyEvent, cwd: &std::path::PathBuf) -> KeyHandleResult {
        let state = match self.state.as_mut() {
            Some(s) => s,
            None => return KeyHandleResult::CloseModal,
        };

        match state.mode {
            SheetMode::Navigate => self.handle_navigate_key(key, cwd),
            SheetMode::Edit => self.handle_edit_key(key),
            SheetMode::Menu => self.handle_menu_key(key, cwd),
            SheetMode::SaveAs => self.handle_save_as_key(key, cwd),
        }
    }

    /// Handle keys in navigate mode
    fn handle_navigate_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        let state = self.state.as_mut().unwrap();

        // Check for Ctrl modifier
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            // Lotus 1-2-3 style menu (/ key)
            KeyCode::Char('/') => {
                state.enter_menu_mode();
                KeyHandleResult::Handled
            }

            // Navigation
            KeyCode::Up | KeyCode::Char('k') => {
                state.move_up();
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                state.move_down();
                KeyHandleResult::Handled
            }
            KeyCode::Left | KeyCode::Char('h') => {
                state.move_left();
                KeyHandleResult::Handled
            }
            KeyCode::Right | KeyCode::Char('l') => {
                state.move_right();
                KeyHandleResult::Handled
            }

            // Tab moves right
            KeyCode::Tab => {
                state.move_right();
                KeyHandleResult::Handled
            }

            // Enter moves down (and starts edit if on empty cell)
            KeyCode::Enter => {
                if state
                    .get_cell(state.cursor_col, state.cursor_row)
                    .is_empty()
                {
                    state.enter_edit_mode();
                } else {
                    state.move_down();
                }
                KeyHandleResult::Handled
            }

            // Home/End
            KeyCode::Home => {
                state.move_home();
                KeyHandleResult::Handled
            }
            KeyCode::End => {
                state.move_end();
                KeyHandleResult::Handled
            }

            // Page up/down (15 rows)
            KeyCode::PageUp => {
                state.page_up(15);
                KeyHandleResult::Handled
            }
            KeyCode::PageDown => {
                state.page_down(15);
                KeyHandleResult::Handled
            }

            // F2 to edit
            KeyCode::F(2) => {
                state.enter_edit_mode();
                KeyHandleResult::Handled
            }

            // Ctrl+S to save (or Save As if no path)
            KeyCode::Char('s') if ctrl => {
                if let Some(path) = state.file_path.clone() {
                    match csv::save_csv(state, &path) {
                        Ok(()) => {
                            state.modified = false;
                            KeyHandleResult::Handled
                        }
                        Err(e) => KeyHandleResult::CloseWithError(e),
                    }
                } else {
                    // No file path, open Save As dialog
                    state.enter_save_as_mode();
                    KeyHandleResult::Handled
                }
            }

            // Typing starts edit (but not / which is menu)
            KeyCode::Char(c) if !ctrl && !c.is_control() && c != '/' => {
                state.start_typing(c);
                KeyHandleResult::Handled
            }

            // Backspace/Delete clears cell
            KeyCode::Backspace | KeyCode::Delete => {
                state.set_cell(state.cursor_col, state.cursor_row, state::CellValue::Empty);
                state.recalculate();
                KeyHandleResult::Handled
            }

            // Escape closes
            KeyCode::Esc => {
                if state.modified {
                    // TODO: Confirm discard changes
                }
                KeyHandleResult::CloseModal
            }

            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle keys in menu mode (Lotus 1-2-3 style)
    fn handle_menu_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let state = self.state.as_mut().unwrap();

        match key.code {
            // Navigate categories
            KeyCode::Left => {
                state.menu_left();
                KeyHandleResult::Handled
            }
            KeyCode::Right => {
                state.menu_right();
                KeyHandleResult::Handled
            }

            // Navigate items within category
            KeyCode::Up => {
                state.menu_up();
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                state.menu_down();
                KeyHandleResult::Handled
            }

            // Execute current selection
            KeyCode::Enter => self.execute_menu_action(cwd),

            // Letter keys for quick selection
            KeyCode::Char(c) => {
                // First try to select category
                if state.menu_select_category(c) {
                    KeyHandleResult::Handled
                }
                // Then try to select item within current category
                else if state.menu_select_item(c) {
                    self.execute_menu_action(cwd)
                } else {
                    KeyHandleResult::Handled
                }
            }

            // Cancel menu
            KeyCode::Esc => {
                state.exit_menu_mode();
                KeyHandleResult::Handled
            }

            _ => KeyHandleResult::Handled,
        }
    }

    /// Execute the currently selected menu action
    fn execute_menu_action(&mut self, cwd: &PathBuf) -> KeyHandleResult {
        let state = self.state.as_mut().unwrap();
        let category = state.current_menu_category();

        match category {
            state::MenuCategory::File => {
                let item = state.current_file_menu_item();
                match item {
                    state::FileMenuItem::Save => {
                        state.exit_menu_mode();
                        if let Some(path) = state.file_path.clone() {
                            match csv::save_csv(state, &path) {
                                Ok(()) => {
                                    state.modified = false;
                                    KeyHandleResult::Handled
                                }
                                Err(e) => KeyHandleResult::CloseWithError(e),
                            }
                        } else {
                            // No path, open Save As
                            state.enter_save_as_mode();
                            KeyHandleResult::Handled
                        }
                    }
                    state::FileMenuItem::Retrieve => {
                        // Open file - not implemented yet
                        state.exit_menu_mode();
                        KeyHandleResult::Handled
                    }
                    state::FileMenuItem::Directory => {
                        // Change directory - show current cwd
                        state.exit_menu_mode();
                        KeyHandleResult::CloseWithSuccess(format!(
                            "Current directory: {}",
                            cwd.display()
                        ))
                    }
                    _ => {
                        // Other items not implemented
                        state.exit_menu_mode();
                        KeyHandleResult::Handled
                    }
                }
            }
            state::MenuCategory::Quit => {
                state.exit_menu_mode();
                KeyHandleResult::CloseModal
            }
            _ => {
                // Other menus not implemented yet
                state.exit_menu_mode();
                KeyHandleResult::Handled
            }
        }
    }

    /// Handle keys in Save As mode
    fn handle_save_as_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let state = self.state.as_mut().unwrap();

        match key.code {
            // Text input
            KeyCode::Char(c) if !c.is_control() => {
                // Filter invalid filename characters
                if !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                    state.save_as_insert(c);
                }
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                state.save_as_backspace();
                KeyHandleResult::Handled
            }
            KeyCode::Delete => {
                state.save_as_delete();
                KeyHandleResult::Handled
            }
            KeyCode::Left => {
                state.save_as_cursor_left();
                KeyHandleResult::Handled
            }
            KeyCode::Right => {
                state.save_as_cursor_right();
                KeyHandleResult::Handled
            }

            // Confirm save
            KeyCode::Enter => {
                if !state.save_as_input.is_empty() {
                    let path = state.save_as_full_path(cwd);
                    state.exit_save_as_mode();

                    // Determine format based on extension
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("csv")
                        .to_lowercase();

                    // Save file and stay in spreadsheet (Lotus 1-2-3 behavior)
                    let result = match ext.as_str() {
                        "csv" | "tsv" => csv::save_csv(state, &path),
                        "xlsx" => xlsx::save_xlsx(state, &path),
                        _ => csv::save_csv(state, &path), // Default to CSV
                    };

                    match result {
                        Ok(()) => {
                            state.file_path = Some(path);
                            state.modified = false;
                            state.status_message = Some(("Saved successfully".to_string(), 30));
                            KeyHandleResult::Handled
                        }
                        Err(e) => {
                            state.status_message = Some((format!("Error: {}", e), 60));
                            KeyHandleResult::Handled
                        }
                    }
                } else {
                    KeyHandleResult::Handled
                }
            }

            // Cancel
            KeyCode::Esc => {
                state.exit_save_as_mode();
                KeyHandleResult::Handled
            }

            _ => KeyHandleResult::Handled,
        }
    }

    /// Handle keys in edit mode
    fn handle_edit_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        let state = self.state.as_mut().unwrap();

        match key.code {
            // Text editing
            KeyCode::Char(c) => {
                state.edit_insert(c);
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                state.edit_backspace();
                KeyHandleResult::Handled
            }
            KeyCode::Delete => {
                state.edit_delete();
                KeyHandleResult::Handled
            }

            // Cursor movement within edit
            KeyCode::Left => {
                state.edit_cursor_left();
                KeyHandleResult::Handled
            }
            KeyCode::Right => {
                state.edit_cursor_right();
                KeyHandleResult::Handled
            }
            KeyCode::Home => {
                state.edit_cursor_home();
                KeyHandleResult::Handled
            }
            KeyCode::End => {
                state.edit_cursor_end();
                KeyHandleResult::Handled
            }

            // Confirm edit
            KeyCode::Enter => {
                state.confirm_edit();
                state.move_down();
                KeyHandleResult::Handled
            }
            KeyCode::Tab => {
                state.confirm_edit();
                state.move_right();
                KeyHandleResult::Handled
            }

            // Cancel edit
            KeyCode::Esc => {
                state.cancel_edit();
                KeyHandleResult::Handled
            }

            _ => KeyHandleResult::Handled,
        }
    }

    /// Draw the spreadsheet modal
    pub fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        if let Some(state) = &self.state {
            modal::draw_sheet_modal(frame, area, state, colors);
        }
    }

    /// Tick for animations
    pub fn tick(&mut self) {
        if let Some(state) = self.state.as_mut() {
            state.tick_count = state.tick_count.wrapping_add(1);

            // Decrement status message timer
            if let Some((_, ticks)) = &mut state.status_message {
                if *ticks > 0 {
                    *ticks -= 1;
                } else {
                    state.status_message = None;
                }
            }
        }
    }
}

// =============================================================================
// PLUGIN TRAIT IMPLEMENTATION (for direct launch)
// =============================================================================

impl Plugin for SheetPlugin {
    fn id(&self) -> &str {
        "sheet"
    }

    fn name(&self) -> &str {
        "Q-SHEET"
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
            name: "Q-SHEET".to_string(),
            description: "Spreadsheet editor with formulas".to_string(),
            category: PluginCategory::Tools,
            key: 'S',
        })
    }

    fn launch(&mut self, _cwd: &PathBuf, _selected_file: Option<&PathBuf>) -> Result<(), String> {
        self.state = Some(SheetState::new());
        Ok(())
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
        SheetPlugin::handle_modal_key(self, key, cwd)
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        SheetPlugin::draw_modal(self, frame, area, colors);
    }

    fn tick(&mut self) {
        SheetPlugin::tick(self);
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "Q-SHEET - Lotus 1-2-3 Style Spreadsheet".to_string(),
            "".to_string(),
            "Inspired by Lotus 1-2-3 Release 4 for DOS (1994),".to_string(),
            "the legendary spreadsheet that defined the industry.".to_string(),
            "".to_string(),
            "MENU SYSTEM (press / to activate):".to_string(),
            "  /             Open Lotus-style menu bar".to_string(),
            "  Left/Right    Navigate menu categories".to_string(),
            "  Up/Down       Navigate submenu items".to_string(),
            "  Letter key    Quick-select by first letter".to_string(),
            "  Enter         Execute selected command".to_string(),
            "  Esc           Close menu".to_string(),
            "".to_string(),
            "FILE MENU (/F):".to_string(),
            "  Retrieve      Open a worksheet file".to_string(),
            "  Save          Save current worksheet".to_string(),
            "  Combine       Combine data from another file".to_string(),
            "  Xtract        Extract portion to new file".to_string(),
            "  Erase         Delete a file".to_string(),
            "  List          List files".to_string(),
            "  Import        Import from other formats".to_string(),
            "  Directory     Change current directory".to_string(),
            "".to_string(),
            "STATUS INDICATORS:".to_string(),
            "  READY         Normal navigation mode".to_string(),
            "  EDIT          Editing a cell".to_string(),
            "  MENU          Menu bar active".to_string(),
            "  INPUT         Dialog input active".to_string(),
            "".to_string(),
            "NAVIGATION:".to_string(),
            "  Arrow keys    Navigate cells".to_string(),
            "  Tab           Move right".to_string(),
            "  Enter         Move down / Confirm edit".to_string(),
            "  Home/End      Jump to row start/end".to_string(),
            "  PgUp/PgDn     Jump 15 rows".to_string(),
            "".to_string(),
            "EDITING:".to_string(),
            "  F2            Edit current cell".to_string(),
            "  Typing        Start entering value".to_string(),
            "  Backspace     Clear cell".to_string(),
            "  Esc           Cancel edit / Close".to_string(),
            "  Ctrl+S        Save file".to_string(),
            "".to_string(),
            "FILE FORMATS:".to_string(),
            "  .csv          Comma-separated values".to_string(),
            "  .xlsx         Microsoft Excel format".to_string(),
            "".to_string(),
            "@ FUNCTIONS (Lotus 1-2-3 style):".to_string(),
            "  Formulas start with = (or @ for Lotus style)".to_string(),
            "".to_string(),
            "Math Functions:".to_string(),
            "  =SUM(range)       Sum of values".to_string(),
            "  =ABS(n)           Absolute value".to_string(),
            "  =SQRT(n)          Square root".to_string(),
            "  =INT(n)           Integer portion".to_string(),
            "  =MOD(n,d)         Modulo/remainder".to_string(),
            "  =ROUND(n,d)       Round to decimals".to_string(),
            "  =EXP(n)           e raised to power".to_string(),
            "  =LN(n)            Natural logarithm".to_string(),
            "  =LOG(n)           Base-10 logarithm".to_string(),
            "  =PI()             Pi constant".to_string(),
            "  =RAND()           Random 0-1".to_string(),
            "  =POWER(n,p)       Raise to power".to_string(),
            "  =SIGN(n)          Sign (-1,0,1)".to_string(),
            "".to_string(),
            "Trigonometric:".to_string(),
            "  =SIN(n)           Sine (radians)".to_string(),
            "  =COS(n)           Cosine (radians)".to_string(),
            "  =TAN(n)           Tangent (radians)".to_string(),
            "  =ASIN(n)          Arc sine".to_string(),
            "  =ACOS(n)          Arc cosine".to_string(),
            "  =ATAN(n)          Arc tangent".to_string(),
            "  =ATAN2(y,x)       Arc tangent of y/x".to_string(),
            "".to_string(),
            "Statistical:".to_string(),
            "  =AVG(range)       Average".to_string(),
            "  =COUNT(range)     Count non-empty".to_string(),
            "  =MIN(range)       Minimum value".to_string(),
            "  =MAX(range)       Maximum value".to_string(),
            "  =STDEV(range)     Standard deviation".to_string(),
            "  =VAR(range)       Variance".to_string(),
            "".to_string(),
            "Logical:".to_string(),
            "  =IF(cond,t,f)     If-then-else".to_string(),
            "  =TRUE()           Returns 1".to_string(),
            "  =FALSE()          Returns 0".to_string(),
            "  =AND(a,b,...)     Logical AND".to_string(),
            "  =OR(a,b,...)      Logical OR".to_string(),
            "  =NOT(n)           Logical NOT".to_string(),
            "".to_string(),
            "Financial:".to_string(),
            "  =PMT(r,n,pv)      Payment amount".to_string(),
            "  =FV(r,n,pmt)      Future value".to_string(),
            "  =PV(r,n,pmt)      Present value".to_string(),
            "".to_string(),
            "Text:".to_string(),
            "  =LEN(text)        String length".to_string(),
            "  =LEFT(s,n)        Left characters".to_string(),
            "  =RIGHT(s,n)       Right characters".to_string(),
            "  =MID(s,i,n)       Middle characters".to_string(),
            "  =UPPER(s)         Uppercase".to_string(),
            "  =LOWER(s)         Lowercase".to_string(),
            "  =TRIM(s)          Remove whitespace".to_string(),
            "  =CONCAT(a,b,...)  Join strings".to_string(),
            "".to_string(),
            "Cell references: A1 (relative), $A$1 (absolute)".to_string(),
            "Ranges: A1:C10 (rectangular block)".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let plugin = SheetPlugin::new();
        assert!(plugin.state.is_none());
    }

    #[test]
    fn test_plugin_launch() {
        let mut plugin = SheetPlugin::new();
        plugin.launch();
        assert!(plugin.state.is_some());
    }

    #[test]
    fn test_cell_operations() {
        let mut plugin = SheetPlugin::new();
        plugin.launch();

        let state = plugin.state.as_mut().unwrap();
        assert_eq!(state.cursor_col, 0);
        assert_eq!(state.cursor_row, 0);

        state.move_right();
        assert_eq!(state.cursor_col, 1);

        state.move_down();
        assert_eq!(state.cursor_row, 1);
    }
}
