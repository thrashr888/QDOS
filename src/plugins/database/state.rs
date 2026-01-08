//! Database plugin state types

use std::path::PathBuf;

/// Current view in the database plugin
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum DatabaseView {
    #[default]
    /// Schema browser - list of tables
    Tables,
    /// Table detail - columns and info
    TableDetail,
    /// Query editor
    Query,
    /// Query results
    Results,
    /// Error state
    Error,
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
    // PostgreSQL and MySQL will be added later
}

impl DatabaseType {
    pub fn name(&self) -> &'static str {
        match self {
            DatabaseType::SQLite => "SQLite",
        }
    }

    pub fn file_extensions(&self) -> &'static [&'static str] {
        match self {
            DatabaseType::SQLite => &["db", "sqlite", "sqlite3", "db3"],
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
    /// File path (for SQLite)
    pub file_path: Option<PathBuf>,
    /// Database name for display
    pub db_name: String,
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

    /// Reset state for a new connection
    pub fn reset(&mut self) {
        self.view = DatabaseView::Tables;
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
