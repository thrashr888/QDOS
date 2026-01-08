//! MySQL database operations
//!
//! Uses separate threads for database operations for consistency with PostgreSQL
//! and to avoid potential blocking the UI.

use super::state::{ColumnInfo, ConnectionConfig, QueryResults, ResultRow, TableInfo};
use mysql::prelude::*;
use mysql::{Opts, Pool, PooledConn};
use std::thread;
use std::time::Instant;

/// Run a blocking mysql operation on a separate thread
fn run_blocking<F, T>(f: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    thread::spawn(f)
        .join()
        .map_err(|_| "Database thread panicked".to_string())?
}

/// Connect to MySQL and get list of tables
pub fn get_tables(config: &ConnectionConfig) -> Result<Vec<TableInfo>, String> {
    let config = config.clone();
    run_blocking(move || {
        let mut conn = connect(&config)?;

        let table_names: Vec<String> = conn
            .query("SHOW TABLES")
            .map_err(|e| format!("Failed to query tables: {}", e))?;

        let mut tables = Vec::new();
        for name in table_names {
            let row_count = get_table_row_count_inner(&mut conn, &name).ok();
            let columns = get_table_columns_inner(&mut conn, &name).unwrap_or_default();
            tables.push(TableInfo {
                name,
                row_count,
                columns,
            });
        }

        Ok(tables)
    })
}

/// Get row count for a table
#[allow(dead_code)]
pub fn get_table_row_count(config: &ConnectionConfig, table_name: &str) -> Result<i64, String> {
    let config = config.clone();
    let table_name = table_name.to_string();
    run_blocking(move || {
        let mut conn = connect(&config)?;
        get_table_row_count_inner(&mut conn, &table_name)
    })
}

fn get_table_row_count_inner(conn: &mut PooledConn, table_name: &str) -> Result<i64, String> {
    let sql = format!("SELECT COUNT(*) FROM `{}`", table_name.replace('`', "``"));
    conn.query_first(&sql)
        .map_err(|e| format!("Failed to count rows: {}", e))?
        .ok_or_else(|| "No result".to_string())
}

/// Get column information for a table
#[allow(dead_code)]
pub fn get_table_columns(
    config: &ConnectionConfig,
    table_name: &str,
) -> Result<Vec<ColumnInfo>, String> {
    let config = config.clone();
    let table_name = table_name.to_string();
    run_blocking(move || {
        let mut conn = connect(&config)?;
        get_table_columns_inner(&mut conn, &table_name)
    })
}

fn get_table_columns_inner(
    conn: &mut PooledConn,
    table_name: &str,
) -> Result<Vec<ColumnInfo>, String> {
    let sql = format!("DESCRIBE `{}`", table_name.replace('`', "``"));
    let rows: Vec<(String, String, String, String, Option<String>, String)> = conn
        .query(&sql)
        .map_err(|e| format!("Failed to get columns: {}", e))?;

    let columns = rows
        .into_iter()
        .map(|(field, col_type, null, key, default, _extra)| ColumnInfo {
            name: field,
            data_type: col_type,
            nullable: null == "YES",
            primary_key: key == "PRI",
            default_value: default,
        })
        .collect();

    Ok(columns)
}

/// Execute a query and return results
pub fn execute_query(config: &ConnectionConfig, sql: &str) -> Result<QueryResults, String> {
    let config = config.clone();
    let sql = sql.to_string();
    run_blocking(move || {
        let mut conn = connect(&config)?;
        let start = Instant::now();

        // Execute the query and get rows as generic values
        let result = conn
            .query_iter(&sql)
            .map_err(|e| format!("SQL error: {}", e))?;

        // Get column names
        let columns: Vec<String> = result
            .columns()
            .as_ref()
            .iter()
            .map(|c| c.name_str().to_string())
            .collect();

        // Collect rows
        let mut rows = Vec::new();
        for row_result in result {
            if rows.len() >= 1000 {
                break;
            }
            let row = row_result.map_err(|e| format!("Fetch error: {}", e))?;
            let values: Vec<String> = (0..columns.len())
                .map(|i| {
                    let value: mysql::Value = row.get(i).unwrap_or(mysql::Value::NULL);
                    match value {
                        mysql::Value::NULL => "NULL".to_string(),
                        mysql::Value::Int(n) => n.to_string(),
                        mysql::Value::UInt(n) => n.to_string(),
                        mysql::Value::Float(f) => f.to_string(),
                        mysql::Value::Double(f) => f.to_string(),
                        mysql::Value::Bytes(b) => String::from_utf8_lossy(&b).to_string(),
                        mysql::Value::Date(y, m, d, h, mi, s, _us) => {
                            format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, m, d, h, mi, s)
                        }
                        mysql::Value::Time(neg, d, h, mi, s, _us) => {
                            let sign = if neg { "-" } else { "" };
                            let hours = d * 24 + h as u32;
                            format!("{}{}:{:02}:{:02}", sign, hours, mi, s)
                        }
                    }
                })
                .collect();
            rows.push(ResultRow { values });
        }

        let row_count = rows.len();
        let execution_time_ms = start.elapsed().as_millis();

        Ok(QueryResults {
            columns,
            rows,
            row_count,
            execution_time_ms,
        })
    })
}

/// Quick select from table with limit
pub fn select_from_table(
    config: &ConnectionConfig,
    table_name: &str,
    limit: usize,
) -> Result<QueryResults, String> {
    let sql = format!(
        "SELECT * FROM `{}` LIMIT {}",
        table_name.replace('`', "``"),
        limit
    );
    execute_query(config, &sql)
}

/// Connect to MySQL
fn connect(config: &ConnectionConfig) -> Result<PooledConn, String> {
    let opts = Opts::from_url(&config.mysql_url()).map_err(|e| format!("Invalid URL: {}", e))?;
    let pool = Pool::new(opts).map_err(|e| format!("Connection failed: {}", e))?;
    pool.get_conn()
        .map_err(|e| format!("Failed to get connection: {}", e))
}

/// Test connection
#[allow(dead_code)]
pub fn test_connection(config: &ConnectionConfig) -> Result<String, String> {
    let config = config.clone();
    run_blocking(move || {
        let mut conn = connect(&config)?;
        let version: Option<String> = conn
            .query_first("SELECT VERSION()")
            .map_err(|e| format!("Query failed: {}", e))?;
        Ok(version.unwrap_or_else(|| "Unknown".to_string()))
    })
}
