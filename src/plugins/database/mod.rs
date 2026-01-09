//! Database client plugin
//!
//! Browse and query SQLite, PostgreSQL, and MySQL databases.

mod modal;
mod mysql;
mod postgres;
mod sqlite;
pub mod state;

use crate::app::ThemeColors;
use crate::plugins::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, PluginMenuItem,
    PluginStatusInfo,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{layout::Rect, Frame};
use state::{
    is_sqlite_file, ConnectionConfig, ConnectionProfile, DatabasePluginConfig, DatabaseState,
    DatabaseType, DatabaseView,
};
use std::any::Any;
use std::fs;
use std::path::PathBuf;

/// Database client plugin
pub struct DatabasePlugin {
    pub state: DatabaseState,
}

impl DatabasePlugin {
    /// Get the config file path for database profiles
    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("rdos").join("database.toml"))
    }

    /// Load profiles from config file
    fn load_profiles(&mut self) {
        if let Some(path) = Self::config_path() {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(config) = toml::from_str::<DatabasePluginConfig>(&content) {
                        self.state.profiles = config.profiles;
                        // Restore last used connection
                        if let Some(last_conn) = config.last_connection {
                            self.state.connection = last_conn;
                        }
                        if let Some(last_type) = config.last_db_type {
                            self.state.last_db_type = Some(last_type);
                        }
                    }
                }
            }
        }
    }

    /// Save profiles to config file
    fn save_profiles(&self) {
        if let Some(path) = Self::config_path() {
            // Create directory if needed
            if let Some(parent) = path.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    eprintln!("Failed to create config dir: {}", e);
                    return;
                }
            }

            let config = DatabasePluginConfig {
                profiles: self.state.profiles.clone(),
                last_connection: Some(self.state.connection.clone()),
                last_db_type: self.state.db_type.as_ref().map(|t| match t {
                    DatabaseType::PostgreSQL => "postgresql".to_string(),
                    DatabaseType::MySQL => "mysql".to_string(),
                    DatabaseType::SQLite => "sqlite".to_string(),
                }),
            };

            match toml::to_string_pretty(&config) {
                Ok(content) => {
                    if let Err(e) = fs::write(&path, &content) {
                        eprintln!("Failed to write config: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to serialize config: {}", e);
                }
            }
        }
    }

    /// Save just the last connection (without profiles changes)
    fn save_last_connection(&self) {
        self.save_profiles(); // Reuses save_profiles which now includes last_connection
    }
}

impl Default for DatabasePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl DatabasePlugin {
    pub fn new() -> Self {
        Self {
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

    /// Open PostgreSQL connection dialog
    pub fn open_postgres(&mut self) {
        // Save current connection before reset if we have one
        let saved_conn = self.state.connection.clone();
        let saved_type = self.state.last_db_type.clone();
        self.state.reset();
        self.state.db_type = Some(DatabaseType::PostgreSQL);
        // Restore last connection if it was PostgreSQL
        if saved_type.as_deref() == Some("postgresql") {
            self.state.connection = saved_conn;
        } else {
            self.state.connection = ConnectionConfig::new_postgres();
        }
        self.state.last_db_type = Some("postgresql".to_string());
        self.state.db_name = "PostgreSQL".to_string();
        self.state.view = DatabaseView::Connect;
    }

    /// Open MySQL connection dialog
    pub fn open_mysql(&mut self) {
        // Save current connection before reset if we have one
        let saved_conn = self.state.connection.clone();
        let saved_type = self.state.last_db_type.clone();
        self.state.reset();
        self.state.db_type = Some(DatabaseType::MySQL);
        // Restore last connection if it was MySQL
        if saved_type.as_deref() == Some("mysql") {
            self.state.connection = saved_conn;
        } else {
            self.state.connection = ConnectionConfig::new_mysql();
        }
        self.state.last_db_type = Some("mysql".to_string());
        self.state.db_name = "MySQL".to_string();
        self.state.view = DatabaseView::Connect;
    }

    /// Open the database type selection modal
    pub fn open_modal(&mut self) {
        self.load_profiles(); // Load saved profiles when modal opens
        self.state.reset();
        self.state.view = DatabaseView::TypeSelect;
        self.state.db_name = "Database".to_string();
    }

    /// Connect to the configured database
    fn connect(&mut self) {
        match self.state.db_type {
            Some(DatabaseType::PostgreSQL) => {
                self.state.db_name = format!("PostgreSQL - {}", self.state.connection.database);
                self.load_tables();
                // Save last connection on successful connect
                if self.state.connected {
                    self.save_last_connection();
                }
            }
            Some(DatabaseType::MySQL) => {
                self.state.db_name = format!("MySQL - {}", self.state.connection.database);
                self.load_tables();
                // Save last connection on successful connect
                if self.state.connected {
                    self.save_last_connection();
                }
            }
            _ => {}
        }
    }

    /// Handle selection of a database type
    fn select_database_type(&mut self) {
        if let Some(db_type) = self.state.selected_type().cloned() {
            self.state.db_type = Some(db_type.clone());
            match db_type {
                DatabaseType::SQLite => {
                    // For SQLite, show error - need to select a file
                    self.state.error =
                        Some("Select a .db/.sqlite file to open SQLite database".to_string());
                    self.state.view = DatabaseView::Error;
                }
                DatabaseType::PostgreSQL => {
                    // Use last connection if it was PostgreSQL, otherwise use defaults
                    if self.state.last_db_type.as_deref() != Some("postgresql") {
                        self.state.connection = ConnectionConfig::new_postgres();
                    }
                    self.state.last_db_type = Some("postgresql".to_string());
                    self.state.db_name = "PostgreSQL".to_string();
                    self.state.view = DatabaseView::Connect;
                }
                DatabaseType::MySQL => {
                    // Use last connection if it was MySQL, otherwise use defaults
                    if self.state.last_db_type.as_deref() != Some("mysql") {
                        self.state.connection = ConnectionConfig::new_mysql();
                    }
                    self.state.last_db_type = Some("mysql".to_string());
                    self.state.db_name = "MySQL".to_string();
                    self.state.view = DatabaseView::Connect;
                }
            }
        }
    }

    /// Load tables from the database
    fn load_tables(&mut self) {
        let result = match self.state.db_type {
            Some(DatabaseType::SQLite) => {
                let Some(ref path) = self.state.file_path else {
                    self.state.error = Some("No database file selected".to_string());
                    self.state.view = DatabaseView::Error;
                    return;
                };
                sqlite::get_tables(path)
            }
            Some(DatabaseType::PostgreSQL) => postgres::get_tables(&self.state.connection),
            Some(DatabaseType::MySQL) => mysql::get_tables(&self.state.connection),
            None => {
                self.state.error = Some("No database type selected".to_string());
                self.state.view = DatabaseView::Error;
                return;
            }
        };

        match result {
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

        let result = match self.state.db_type {
            Some(DatabaseType::SQLite) => {
                let Some(ref path) = self.state.file_path else {
                    return;
                };
                sqlite::select_from_table(path, &table_name, 100)
            }
            Some(DatabaseType::PostgreSQL) => {
                postgres::select_from_table(&self.state.connection, &table_name, 100)
            }
            Some(DatabaseType::MySQL) => {
                mysql::select_from_table(&self.state.connection, &table_name, 100)
            }
            None => return,
        };

        match result {
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

        let result = match self.state.db_type {
            Some(DatabaseType::SQLite) => {
                let Some(ref path) = self.state.file_path else {
                    return;
                };
                sqlite::execute_query(path, &self.state.query)
            }
            Some(DatabaseType::PostgreSQL) => {
                postgres::execute_query(&self.state.connection, &self.state.query)
            }
            Some(DatabaseType::MySQL) => {
                mysql::execute_query(&self.state.connection, &self.state.query)
            }
            None => return,
        };

        match result {
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

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        true // SQLite is always available (bundled)
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "Database".to_string(),
            key: 'D',
            description: "Browse SQLite, PostgreSQL, MySQL".to_string(),
            priority: 40,
        })
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: "database".to_string(),
            name: "Database".to_string(),
            description: "SQLite, PostgreSQL, MySQL browser".to_string(),
            category: PluginCategory::Tools,
            key: 'D',
        })
    }

    fn launch(&mut self, _cwd: &PathBuf, _selected_file: Option<&PathBuf>) -> Result<(), String> {
        self.open_modal();
        Ok(())
    }

    fn status_info(&self, _cwd: &PathBuf) -> Option<PluginStatusInfo> {
        None
    }

    fn handle_global_key(
        &mut self,
        _key: KeyEvent,
        _cwd: &PathBuf,
        _selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
        KeyHandleResult::NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match self.state.view {
            DatabaseView::TypeSelect => self.handle_type_select_key(key),
            DatabaseView::Profiles => self.handle_profiles_key(key),
            DatabaseView::Tables => self.handle_tables_key(key),
            DatabaseView::TableDetail => self.handle_table_detail_key(key),
            DatabaseView::Query => self.handle_query_key(key),
            DatabaseView::Results => self.handle_results_key(key),
            DatabaseView::Connect => self.handle_connect_key(key),
            DatabaseView::SaveProfile => self.handle_save_profile_key(key),
            DatabaseView::Error => match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    if self.state.connected {
                        self.state.view = DatabaseView::Tables;
                    } else if self.state.db_type.is_some() {
                        self.state.view = DatabaseView::Connect;
                    } else {
                        self.state.view = DatabaseView::TypeSelect;
                    }
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
            "Browse and query databases.".to_string(),
            "".to_string(),
            "Supported:".to_string(),
            "  SQLite - .db, .sqlite, .sqlite3, .db3".to_string(),
            "  PostgreSQL - Connect via host/port".to_string(),
            "  MySQL - Connect via host/port".to_string(),
            "".to_string(),
            "Features:".to_string(),
            "  - Browse tables and columns".to_string(),
            "  - Execute SQL queries".to_string(),
            "  - View query results".to_string(),
            "".to_string(),
            "Keys:".to_string(),
            "  Enter  - View table details / Connect".to_string(),
            "  S      - SELECT * FROM table".to_string(),
            "  Q      - Open query editor".to_string(),
            "  Tab    - Next field (connection form)".to_string(),
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

    fn handle_connect_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.reset();
                KeyHandleResult::CloseModal
            }
            KeyCode::Tab | KeyCode::Down => {
                self.state.next_connect_field();
                KeyHandleResult::Handled
            }
            KeyCode::BackTab | KeyCode::Up => {
                self.state.prev_connect_field();
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                self.connect();
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                self.state.backspace_connect();
                KeyHandleResult::Handled
            }
            KeyCode::F(2) => {
                // Save as profile
                self.state.profile_name.clear();
                self.state.view = DatabaseView::SaveProfile;
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                self.state.insert_connect_char(c);
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_type_select_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.reset();
                KeyHandleResult::CloseModal
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.select_prev_type();
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.select_next_type();
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                self.select_database_type();
                KeyHandleResult::Handled
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                // Reload profiles from disk each time (allows external edits)
                self.load_profiles();
                self.state.view = DatabaseView::Profiles;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_profiles_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.view = DatabaseView::TypeSelect;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.select_prev_profile();
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.select_next_profile();
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                // Load selected profile and connect
                if let Some(profile) = self.state.selected_profile().cloned() {
                    self.state.connection = profile.config;
                    match profile.db_type.as_str() {
                        "postgresql" => {
                            self.state.db_type = Some(DatabaseType::PostgreSQL);
                            self.state.db_name = format!("PostgreSQL - {}", profile.name);
                        }
                        "mysql" => {
                            self.state.db_type = Some(DatabaseType::MySQL);
                            self.state.db_name = format!("MySQL - {}", profile.name);
                        }
                        _ => {}
                    }
                    self.connect();
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Delete => {
                // Delete selected profile
                self.state.delete_selected_profile();
                self.save_profiles();
                if self.state.profiles.is_empty() {
                    self.state.view = DatabaseView::TypeSelect;
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_save_profile_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.profile_name.clear();
                self.state.view = DatabaseView::Connect;
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                if !self.state.profile_name.trim().is_empty() {
                    // Save the profile
                    let db_type_str = match self.state.db_type {
                        Some(DatabaseType::PostgreSQL) => "postgresql",
                        Some(DatabaseType::MySQL) => "mysql",
                        _ => return KeyHandleResult::Handled,
                    };

                    let profile = ConnectionProfile {
                        name: self.state.profile_name.trim().to_string(),
                        db_type: db_type_str.to_string(),
                        config: self.state.connection.clone(),
                    };

                    self.state.add_profile(profile);
                    self.save_profiles();
                    self.state.profile_name.clear();
                    self.state.view = DatabaseView::Connect;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                self.state.profile_name.pop();
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                self.state.profile_name.push(c);
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }
}
