//! SQLite database operations

use super::state::{ColumnInfo, QueryResults, ResultRow, TableInfo};
use rusqlite::{Connection, OpenFlags};
use std::path::Path;
use std::time::Instant;

/// Open a SQLite database and get list of tables
pub fn get_tables<P: AsRef<Path>>(path: P) -> Result<Vec<TableInfo>, String> {
    let conn = open_readonly(path)?;

    let mut stmt = conn
        .prepare(
            "SELECT name FROM sqlite_master
             WHERE type='table' AND name NOT LIKE 'sqlite_%'
             ORDER BY name",
        )
        .map_err(|e| format!("Failed to query tables: {}", e))?;

    let table_names: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| format!("Failed to fetch tables: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    let mut tables = Vec::new();
    for name in table_names {
        let row_count = get_table_row_count_inner(&conn, &name).ok();
        let columns = get_table_columns_inner(&conn, &name).unwrap_or_default();
        tables.push(TableInfo {
            name,
            row_count,
            columns,
        });
    }

    Ok(tables)
}

/// Get row count for a table
pub fn get_table_row_count<P: AsRef<Path>>(path: P, table_name: &str) -> Result<i64, String> {
    let conn = open_readonly(path)?;
    get_table_row_count_inner(&conn, table_name)
}

fn get_table_row_count_inner(conn: &Connection, table_name: &str) -> Result<i64, String> {
    let sql = format!(
        "SELECT COUNT(*) FROM \"{}\"",
        table_name.replace('"', "\"\"")
    );
    conn.query_row(&sql, [], |row| row.get(0))
        .map_err(|e| format!("Failed to count rows: {}", e))
}

/// Get column information for a table
pub fn get_table_columns<P: AsRef<Path>>(
    path: P,
    table_name: &str,
) -> Result<Vec<ColumnInfo>, String> {
    let conn = open_readonly(path)?;
    get_table_columns_inner(&conn, table_name)
}

fn get_table_columns_inner(conn: &Connection, table_name: &str) -> Result<Vec<ColumnInfo>, String> {
    let sql = format!("PRAGMA table_info(\"{}\")", table_name.replace('"', "\"\""));
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Failed to get table info: {}", e))?;

    let columns = stmt
        .query_map([], |row| {
            Ok(ColumnInfo {
                name: row.get(1)?,
                data_type: row.get(2)?,
                nullable: row.get::<_, i32>(3)? == 0,
                primary_key: row.get::<_, i32>(5)? > 0,
                default_value: row.get(4).ok(),
            })
        })
        .map_err(|e| format!("Failed to fetch columns: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(columns)
}

/// Execute a query and return results
pub fn execute_query<P: AsRef<Path>>(path: P, sql: &str) -> Result<QueryResults, String> {
    let conn = open_readonly(path)?;
    let start = Instant::now();

    let mut stmt = conn.prepare(sql).map_err(|e| format!("SQL error: {}", e))?;

    // Get column names
    let columns: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
    let column_count = columns.len();

    // Fetch rows (limit to 1000 for safety)
    let mut rows = Vec::new();
    let mut row_iter = stmt.query([]).map_err(|e| format!("Query failed: {}", e))?;

    while let Some(row) = row_iter.next().map_err(|e| format!("Fetch error: {}", e))? {
        if rows.len() >= 1000 {
            break;
        }

        let mut values = Vec::with_capacity(column_count);
        for i in 0..column_count {
            let value: String = match row.get_ref(i) {
                Ok(rusqlite::types::ValueRef::Null) => "NULL".to_string(),
                Ok(rusqlite::types::ValueRef::Integer(i)) => i.to_string(),
                Ok(rusqlite::types::ValueRef::Real(f)) => f.to_string(),
                Ok(rusqlite::types::ValueRef::Text(t)) => String::from_utf8_lossy(t).to_string(),
                Ok(rusqlite::types::ValueRef::Blob(b)) => format!("<BLOB {} bytes>", b.len()),
                Err(_) => "<error>".to_string(),
            };
            values.push(value);
        }
        rows.push(ResultRow { values });
    }

    let execution_time_ms = start.elapsed().as_millis();
    let row_count = rows.len();

    Ok(QueryResults {
        columns,
        rows,
        row_count,
        execution_time_ms,
    })
}

/// Quick select from table with limit
pub fn select_from_table<P: AsRef<Path>>(
    path: P,
    table_name: &str,
    limit: usize,
) -> Result<QueryResults, String> {
    let sql = format!(
        "SELECT * FROM \"{}\" LIMIT {}",
        table_name.replace('"', "\"\""),
        limit
    );
    execute_query(path, &sql)
}

/// Open a SQLite database read-only
fn open_readonly<P: AsRef<Path>>(path: P) -> Result<Connection, String> {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| format!("Failed to open database: {}", e))
}
