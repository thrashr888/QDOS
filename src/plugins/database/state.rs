//! Database plugin state types

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Current view in the database plugin
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum DatabaseView {
    #[default]
    /// Database type selection
    TypeSelect,
    /// Saved connection profiles
    Profiles,
    /// Schema browser - list of tables
    Tables,
    /// Table detail - columns and info
    TableDetail,
    /// Query editor
    Query,
    /// Query results
    Results,
    /// Connection configuration (for PostgreSQL/MySQL)
    Connect,
    /// Save current connection as profile
    SaveProfile,
    /// Error state
    Error,
}

/// Connection configuration for remote databases
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub password: String,
    pub database: String,
}

/// Saved connection profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionProfile {
    pub name: String,
    pub db_type: String, // "postgresql" or "mysql"
    pub config: ConnectionConfig,
}

/// Database plugin configuration (saved to config file)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DatabasePluginConfig {
    #[serde(default)]
    pub profiles: Vec<ConnectionProfile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_connection: Option<ConnectionConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_db_type: Option<String>,
}

impl ConnectionConfig {
    pub fn new_postgres() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 5432,
            user: "postgres".to_string(),
            password: String::new(),
            database: "postgres".to_string(),
        }
    }

    pub fn new_mysql() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 3306,
            user: "root".to_string(),
            password: String::new(),
            database: "mysql".to_string(),
        }
    }

    /// Build PostgreSQL connection string
    pub fn postgres_url(&self) -> String {
        if self.password.is_empty() {
            format!(
                "host={} port={} user={} dbname={}",
                self.host, self.port, self.user, self.database
            )
        } else {
            format!(
                "host={} port={} user={} password={} dbname={}",
                self.host, self.port, self.user, self.password, self.database
            )
        }
    }

    /// Build MySQL connection URL
    pub fn mysql_url(&self) -> String {
        if self.password.is_empty() {
            format!(
                "mysql://{}@{}:{}/{}",
                self.user, self.host, self.port, self.database
            )
        } else {
            format!(
                "mysql://{}:{}@{}:{}/{}",
                self.user, self.password, self.host, self.port, self.database
            )
        }
    }
}

/// Connection form field
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectField {
    #[default]
    Host,
    Port,
    User,
    Password,
    Database,
}

/// Column information
#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub primary_key: bool,
    pub default_value: Option<String>,
}

/// Table information
#[derive(Debug, Clone)]
pub struct TableInfo {
    pub name: String,
    pub row_count: Option<i64>,
    pub columns: Vec<ColumnInfo>,
}

/// Query result row (dynamic columns)
#[derive(Debug, Clone)]
pub struct ResultRow {
    pub values: Vec<String>,
}

/// Query results
#[derive(Debug, Clone, Default)]
pub struct QueryResults {
    pub columns: Vec<String>,
    pub rows: Vec<ResultRow>,
    pub row_count: usize,
    pub execution_time_ms: u128,
}

/// Database connection type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatabaseType {
    SQLite,
    PostgreSQL,
    MySQL,
}

impl DatabaseType {
    pub fn name(&self) -> &'static str {
        match self {
            DatabaseType::SQLite => "SQLite",
            DatabaseType::PostgreSQL => "PostgreSQL",
            DatabaseType::MySQL => "MySQL",
        }
    }

    pub fn file_extensions(&self) -> &'static [&'static str] {
        match self {
            DatabaseType::SQLite => &["db", "sqlite", "sqlite3", "db3"],
            DatabaseType::PostgreSQL => &[],
            DatabaseType::MySQL => &[],
        }
    }

    pub fn default_port(&self) -> u16 {
        match self {
            DatabaseType::SQLite => 0,
            DatabaseType::PostgreSQL => 5432,
            DatabaseType::MySQL => 3306,
        }
    }
}

/// Database plugin state
#[derive(Debug, Default)]
pub struct DatabaseState {
    /// Current view
    pub view: DatabaseView,
    /// Database type
    pub db_type: Option<DatabaseType>,
    /// Selected type index (for type selection)
    pub selected_type_idx: usize,
    /// File path (for SQLite)
    pub file_path: Option<PathBuf>,
    /// Connection config (for PostgreSQL/MySQL)
    pub connection: ConnectionConfig,
    /// Currently selected connection field
    pub connect_field: ConnectField,
    /// Database name for display
    pub db_name: String,
    /// Saved connection profiles
    pub profiles: Vec<ConnectionProfile>,
    /// Selected profile index
    pub selected_profile: usize,
    /// Profile name being entered (for save)
    pub profile_name: String,
    /// Last used database type (for restoring)
    pub last_db_type: Option<String>,
    /// List of tables
    pub tables: Vec<TableInfo>,
    /// Currently selected table index
    pub selected_table: usize,
    /// Current table detail (when viewing a table)
    pub current_table: Option<TableInfo>,
    /// Query text
    pub query: String,
    /// Query cursor position
    pub query_cursor: usize,
    /// Query results
    pub results: Option<QueryResults>,
    /// Selected result row
    pub selected_row: usize,
    /// Scroll offset for results
    pub scroll_offset: usize,
    /// Error message
    pub error: Option<String>,
    /// Is connected
    pub connected: bool,
}

impl DatabaseState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset state for a new connection (keeps profiles and last connection)
    pub fn reset(&mut self) {
        self.view = DatabaseView::TypeSelect;
        self.db_type = None;
        self.selected_type_idx = 0;
        self.file_path = None;
        // Keep connection and last_db_type - don't clear them
        self.connect_field = ConnectField::default();
        self.db_name = String::new();
        // Keep profiles - don't clear them
        self.selected_profile = 0;
        self.profile_name.clear();
        self.tables.clear();
        self.selected_table = 0;
        self.current_table = None;
        self.query.clear();
        self.query_cursor = 0;
        self.results = None;
        self.selected_row = 0;
        self.scroll_offset = 0;
        self.error = None;
        self.connected = false;
    }

    /// Select next profile
    pub fn select_next_profile(&mut self) {
        if !self.profiles.is_empty() {
            self.selected_profile = (self.selected_profile + 1) % self.profiles.len();
        }
    }

    /// Select previous profile
    pub fn select_prev_profile(&mut self) {
        if !self.profiles.is_empty() {
            self.selected_profile = self
                .selected_profile
                .checked_sub(1)
                .unwrap_or(self.profiles.len() - 1);
        }
    }

    /// Get selected profile
    pub fn selected_profile(&self) -> Option<&ConnectionProfile> {
        self.profiles.get(self.selected_profile)
    }

    /// Add a profile
    pub fn add_profile(&mut self, profile: ConnectionProfile) {
        self.profiles.push(profile);
    }

    /// Delete selected profile
    pub fn delete_selected_profile(&mut self) {
        if !self.profiles.is_empty() && self.selected_profile < self.profiles.len() {
            self.profiles.remove(self.selected_profile);
            if self.selected_profile >= self.profiles.len() && self.selected_profile > 0 {
                self.selected_profile -= 1;
            }
        }
    }

    /// Get available database types
    pub fn available_types() -> &'static [DatabaseType] {
        &[
            DatabaseType::SQLite,
            DatabaseType::PostgreSQL,
            DatabaseType::MySQL,
        ]
    }

    /// Select next database type
    pub fn select_next_type(&mut self) {
        let types = Self::available_types();
        if !types.is_empty() {
            self.selected_type_idx = (self.selected_type_idx + 1) % types.len();
        }
    }

    /// Select previous database type
    pub fn select_prev_type(&mut self) {
        let types = Self::available_types();
        if !types.is_empty() {
            self.selected_type_idx = self
                .selected_type_idx
                .checked_sub(1)
                .unwrap_or(types.len() - 1);
        }
    }

    /// Get selected database type
    pub fn selected_type(&self) -> Option<&'static DatabaseType> {
        Self::available_types().get(self.selected_type_idx)
    }

    /// Select next table
    pub fn select_next_table(&mut self) {
        if !self.tables.is_empty() {
            self.selected_table = (self.selected_table + 1) % self.tables.len();
        }
    }

    /// Select previous table
    pub fn select_prev_table(&mut self) {
        if !self.tables.is_empty() {
            self.selected_table = self
                .selected_table
                .checked_sub(1)
                .unwrap_or(self.tables.len() - 1);
        }
    }

    /// Get selected table
    pub fn selected_table(&self) -> Option<&TableInfo> {
        self.tables.get(self.selected_table)
    }

    /// Select next result row
    pub fn select_next_row(&mut self) {
        if let Some(ref results) = self.results {
            if !results.rows.is_empty() {
                self.selected_row = (self.selected_row + 1) % results.rows.len();
            }
        }
    }

    /// Select previous result row
    pub fn select_prev_row(&mut self) {
        if let Some(ref results) = self.results {
            if !results.rows.is_empty() {
                self.selected_row = self
                    .selected_row
                    .checked_sub(1)
                    .unwrap_or(results.rows.len() - 1);
            }
        }
    }

    /// Insert character at cursor
    pub fn insert_char(&mut self, c: char) {
        self.query.insert(self.query_cursor, c);
        self.query_cursor += 1;
    }

    /// Delete character before cursor
    pub fn backspace(&mut self) {
        if self.query_cursor > 0 {
            self.query_cursor -= 1;
            self.query.remove(self.query_cursor);
        }
    }

    /// Delete character at cursor
    pub fn delete(&mut self) {
        if self.query_cursor < self.query.len() {
            self.query.remove(self.query_cursor);
        }
    }

    /// Move cursor left
    pub fn cursor_left(&mut self) {
        if self.query_cursor > 0 {
            self.query_cursor -= 1;
        }
    }

    /// Move cursor right
    pub fn cursor_right(&mut self) {
        if self.query_cursor < self.query.len() {
            self.query_cursor += 1;
        }
    }

    /// Move cursor to start
    pub fn cursor_home(&mut self) {
        self.query_cursor = 0;
    }

    /// Move cursor to end
    pub fn cursor_end(&mut self) {
        self.query_cursor = self.query.len();
    }

    /// Set query from template
    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.query_cursor = self.query.len();
    }

    /// Move to next connection field
    pub fn next_connect_field(&mut self) {
        self.connect_field = match self.connect_field {
            ConnectField::Host => ConnectField::Port,
            ConnectField::Port => ConnectField::User,
            ConnectField::User => ConnectField::Password,
            ConnectField::Password => ConnectField::Database,
            ConnectField::Database => ConnectField::Host,
        };
    }

    /// Move to previous connection field
    pub fn prev_connect_field(&mut self) {
        self.connect_field = match self.connect_field {
            ConnectField::Host => ConnectField::Database,
            ConnectField::Port => ConnectField::Host,
            ConnectField::User => ConnectField::Port,
            ConnectField::Password => ConnectField::User,
            ConnectField::Database => ConnectField::Password,
        };
    }

    /// Get current connection field value as string
    pub fn current_field_value(&self) -> String {
        match self.connect_field {
            ConnectField::Host => self.connection.host.clone(),
            ConnectField::Port => self.connection.port.to_string(),
            ConnectField::User => self.connection.user.clone(),
            ConnectField::Password => self.connection.password.clone(),
            ConnectField::Database => self.connection.database.clone(),
        }
    }

    /// Insert character into current connection field
    pub fn insert_connect_char(&mut self, c: char) {
        match self.connect_field {
            ConnectField::Host => self.connection.host.push(c),
            ConnectField::Port => {
                if c.is_ascii_digit() {
                    let mut s = self.connection.port.to_string();
                    s.push(c);
                    if let Ok(p) = s.parse::<u16>() {
                        self.connection.port = p;
                    }
                }
            }
            ConnectField::User => self.connection.user.push(c),
            ConnectField::Password => self.connection.password.push(c),
            ConnectField::Database => self.connection.database.push(c),
        }
    }

    /// Backspace on current connection field
    pub fn backspace_connect(&mut self) {
        match self.connect_field {
            ConnectField::Host => {
                self.connection.host.pop();
            }
            ConnectField::Port => {
                let mut s = self.connection.port.to_string();
                s.pop();
                self.connection.port = s.parse().unwrap_or(0);
            }
            ConnectField::User => {
                self.connection.user.pop();
            }
            ConnectField::Password => {
                self.connection.password.pop();
            }
            ConnectField::Database => {
                self.connection.database.pop();
            }
        }
    }
}

/// Check if a file is a SQLite database
pub fn is_sqlite_file(path: &PathBuf) -> bool {
    path.extension()
        .map(|ext| {
            let ext_lower = ext.to_string_lossy().to_lowercase();
            DatabaseType::SQLite
                .file_extensions()
                .contains(&ext_lower.as_str())
        })
        .unwrap_or(false)
}
