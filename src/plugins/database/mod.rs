//! Database client plugin
//!
//! Browse and query SQLite databases (PostgreSQL/MySQL coming soon).

mod modal;
mod sqlite;
pub mod state;

use crate::app::ThemeColors;
use crate::plugins::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, PluginMenuItem,
    PluginStatusInfo,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{layout::Rect, Frame};
use state::{is_sqlite_file, DatabaseState, DatabaseType, DatabaseView};
use std::any::Any;
use std::path::PathBuf;

/// Database client plugin
pub struct DatabasePlugin {
    initialized: bool,
    pub state: DatabaseState,
}

impl Default for DatabasePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl DatabasePlugin {
    pub fn new() -> Self {
        Self {
            initialized: false,
            state: DatabaseState::new(),
        }
    }

    /// Open a SQLite database file
    pub fn open_sqlite(&mut self, file_path: &PathBuf) {
        self.state.reset();
        self.state.db_type = Some(DatabaseType::SQLite);
        self.state.file_path = Some(file_path.clone());
        self.state.db_name = file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "database".to_string());

        self.load_tables();
    }

    /// Load tables from the database
    fn load_tables(&mut self) {
        let Some(ref path) = self.state.file_path else {
            self.state.error = Some("No database file selected".to_string());
            self.state.view = DatabaseView::Error;
            return;
        };

        match sqlite::get_tables(path) {
            Ok(tables) => {
                self.state.tables = tables;
                self.state.connected = true;
                self.state.view = DatabaseView::Tables;
            }
            Err(e) => {
                self.state.error = Some(e);
                self.state.view = DatabaseView::Error;
            }
        }
    }

    /// Load detail for selected table
    fn load_table_detail(&mut self) {
        if let Some(table) = self.state.selected_table() {
            self.state.current_table = Some(table.clone());
            self.state.view = DatabaseView::TableDetail;
        }
    }

    /// Execute SELECT * on current table
    fn select_from_table(&mut self) {
        let table_name = if let Some(ref table) = self.state.current_table {
            table.name.clone()
        } else if let Some(table) = self.state.selected_table() {
            table.name.clone()
        } else {
            return;
        };

        let Some(ref path) = self.state.file_path else {
            return;
        };

        match sqlite::select_from_table(path, &table_name, 100) {
            Ok(results) => {
                self.state.results = Some(results);
                self.state.selected_row = 0;
                self.state.view = DatabaseView::Results;
            }
            Err(e) => {
                self.state.error = Some(e);
                self.state.view = DatabaseView::Error;
            }
        }
    }

    /// Execute the current query
    fn execute_query(&mut self) {
        if self.state.query.trim().is_empty() {
            return;
        }

        let Some(ref path) = self.state.file_path else {
            return;
        };

        match sqlite::execute_query(path, &self.state.query) {
            Ok(results) => {
                self.state.results = Some(results);
                self.state.selected_row = 0;
                self.state.view = DatabaseView::Results;
            }
            Err(e) => {
                self.state.error = Some(e);
                self.state.view = DatabaseView::Error;
            }
        }
    }

    /// Check if a file is a database file
    pub fn is_database_file(path: &PathBuf) -> bool {
        is_sqlite_file(path)
    }
}

impl Plugin for DatabasePlugin {
    fn id(&self) -> &str {
        "database"
    }

    fn name(&self) -> &str {
        "Database"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: true,
            has_keys: false,
            has_modal: true,
            has_status: false,
            has_cli: false,
            has_help: true,
        }
    }

    fn init(&mut self, _cwd: &PathBuf) -> Result<(), String> {
        self.initialized = true;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), String> {
        self.state.reset();
        self.initialized = false;
        Ok(())
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        true // SQLite is always available (bundled)
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "Database".to_string(),
            key: 'D',
            description: "Browse SQLite databases".to_string(),
            priority: 40,
        })
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: "database".to_string(),
            name: "Database".to_string(),
            description: "SQLite database browser".to_string(),
            category: PluginCategory::Tools,
            key: 'D',
        })
    }

    fn status_info(&self, _cwd: &PathBuf) -> Option<PluginStatusInfo> {
        None
    }

    fn handle_global_key(&mut self, _key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        KeyHandleResult::NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match self.state.view {
            DatabaseView::Tables => self.handle_tables_key(key),
            DatabaseView::TableDetail => self.handle_table_detail_key(key),
            DatabaseView::Query => self.handle_query_key(key),
            DatabaseView::Results => self.handle_results_key(key),
            DatabaseView::Error => match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.state.view = DatabaseView::Tables;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        modal::draw_database_modal(frame, area, &self.state, colors);
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "Database Client".to_string(),
            "".to_string(),
            "Browse and query SQLite databases.".to_string(),
            "".to_string(),
            "Supported formats:".to_string(),
            "  .db, .sqlite, .sqlite3, .db3".to_string(),
            "".to_string(),
            "Features:".to_string(),
            "  - Browse tables and columns".to_string(),
            "  - Execute SQL queries".to_string(),
            "  - View query results".to_string(),
            "".to_string(),
            "Keys:".to_string(),
            "  Enter  - View table details".to_string(),
            "  S      - SELECT * FROM table".to_string(),
            "  Q      - Open query editor".to_string(),
            "  Esc    - Go back / close".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl DatabasePlugin {
    fn handle_tables_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.reset();
                KeyHandleResult::CloseModal
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.select_prev_table();
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.select_next_table();
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                self.load_table_detail();
                KeyHandleResult::Handled
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.select_from_table();
                KeyHandleResult::Handled
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.state.query.clear();
                self.state.query_cursor = 0;
                self.state.view = DatabaseView::Query;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_table_detail_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.current_table = None;
                self.state.view = DatabaseView::Tables;
                KeyHandleResult::Handled
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.select_from_table();
                KeyHandleResult::Handled
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                // Pre-fill query with SELECT for current table
                if let Some(ref table) = self.state.current_table {
                    self.state
                        .set_query(format!("SELECT * FROM \"{}\" LIMIT 100", table.name));
                }
                self.state.view = DatabaseView::Query;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_query_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.view = DatabaseView::Tables;
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                self.execute_query();
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                self.state.backspace();
                KeyHandleResult::Handled
            }
            KeyCode::Delete => {
                self.state.delete();
                KeyHandleResult::Handled
            }
            KeyCode::Left => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.state.cursor_home();
                } else {
                    self.state.cursor_left();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Right => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.state.cursor_end();
                } else {
                    self.state.cursor_right();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Home => {
                self.state.cursor_home();
                KeyHandleResult::Handled
            }
            KeyCode::End => {
                self.state.cursor_end();
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                self.state.insert_char(c);
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_results_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.results = None;
                self.state.view = DatabaseView::Tables;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.select_prev_row();
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.select_next_row();
                KeyHandleResult::Handled
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.state.view = DatabaseView::Query;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }
}
