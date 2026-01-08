//! PostgreSQL database operations
//!
//! Uses separate threads for database operations to avoid tokio runtime conflicts.
//! The `postgres` crate internally creates a runtime, which conflicts with
//! our app's existing tokio runtime.

use super::state::{ColumnInfo, ConnectionConfig, QueryResults, ResultRow, TableInfo};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use postgres::{Client, NoTls};
use std::thread;
use std::time::Instant;

/// Run a blocking postgres operation on a separate thread to avoid runtime conflicts
fn run_blocking<F, T>(f: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    thread::spawn(f)
        .join()
        .map_err(|_| "Database thread panicked".to_string())?
}

/// Connect to PostgreSQL and get list of tables
pub fn get_tables(config: &ConnectionConfig) -> Result<Vec<TableInfo>, String> {
    let config = config.clone();
    run_blocking(move || {
        let mut client = connect(&config)?;

        let rows = client
            .query(
                "SELECT table_name FROM information_schema.tables
                 WHERE table_schema = 'public'
                 ORDER BY table_name",
                &[],
            )
            .map_err(|e| format!("Failed to query tables: {}", e))?;

        let table_names: Vec<String> = rows.iter().map(|row| row.get(0)).collect();

        let mut tables = Vec::new();
        for name in table_names {
            let row_count = get_table_row_count_inner(&mut client, &name).ok();
            let columns = get_table_columns_inner(&mut client, &name).unwrap_or_default();
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
        let mut client = connect(&config)?;
        get_table_row_count_inner(&mut client, &table_name)
    })
}

fn get_table_row_count_inner(client: &mut Client, table_name: &str) -> Result<i64, String> {
    let sql = format!(
        "SELECT COUNT(*) FROM \"{}\"",
        table_name.replace('"', "\"\"")
    );
    let row = client
        .query_one(&sql, &[])
        .map_err(|e| format!("Failed to count rows: {}", e))?;
    Ok(row.get(0))
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
        let mut client = connect(&config)?;
        get_table_columns_inner(&mut client, &table_name)
    })
}

fn get_table_columns_inner(
    client: &mut Client,
    table_name: &str,
) -> Result<Vec<ColumnInfo>, String> {
    let rows = client
        .query(
            "SELECT column_name, data_type, is_nullable, column_default
             FROM information_schema.columns
             WHERE table_name = $1
             ORDER BY ordinal_position",
            &[&table_name],
        )
        .map_err(|e| format!("Failed to get columns: {}", e))?;

    // Get primary key columns
    let pk_rows = client
        .query(
            "SELECT a.attname
             FROM pg_index i
             JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey)
             WHERE i.indrelid = $1::regclass AND i.indisprimary",
            &[&table_name],
        )
        .unwrap_or_default();

    let pk_columns: Vec<String> = pk_rows.iter().map(|row| row.get(0)).collect();

    let columns = rows
        .iter()
        .map(|row| {
            let name: String = row.get(0);
            let data_type: String = row.get(1);
            let is_nullable: String = row.get(2);
            let default_value: Option<String> = row.get(3);

            ColumnInfo {
                primary_key: pk_columns.contains(&name),
                name,
                data_type,
                nullable: is_nullable == "YES",
                default_value,
            }
        })
        .collect();

    Ok(columns)
}

/// Execute a query and return results
pub fn execute_query(config: &ConnectionConfig, sql: &str) -> Result<QueryResults, String> {
    let config = config.clone();
    let sql = sql.to_string();
    run_blocking(move || {
        let mut client = connect(&config)?;
        let start = Instant::now();

        let rows = client
            .query(&sql, &[])
            .map_err(|e| format!("SQL error: {}", e))?;

        if rows.is_empty() {
            return Ok(QueryResults {
                columns: Vec::new(),
                rows: Vec::new(),
                row_count: 0,
                execution_time_ms: start.elapsed().as_millis(),
            });
        }

        // Get column names from first row
        let columns: Vec<String> = rows[0]
            .columns()
            .iter()
            .map(|c| c.name().to_string())
            .collect();

        // Fetch rows (limit to 1000 for safety)
        let result_rows: Vec<ResultRow> = rows
            .iter()
            .take(1000)
            .map(|row| {
                let values: Vec<String> = (0..columns.len())
                    .map(|i| {
                        // Try to get value as various types
                        if let Ok(v) = row.try_get::<_, Option<String>>(i) {
                            v.unwrap_or_else(|| "NULL".to_string())
                        } else if let Ok(v) = row.try_get::<_, Option<i64>>(i) {
                            v.map(|n| n.to_string())
                                .unwrap_or_else(|| "NULL".to_string())
                        } else if let Ok(v) = row.try_get::<_, Option<i32>>(i) {
                            v.map(|n| n.to_string())
                                .unwrap_or_else(|| "NULL".to_string())
                        } else if let Ok(v) = row.try_get::<_, Option<f64>>(i) {
                            v.map(|n| n.to_string())
                                .unwrap_or_else(|| "NULL".to_string())
                        } else if let Ok(v) = row.try_get::<_, Option<bool>>(i) {
                            v.map(|b| b.to_string())
                                .unwrap_or_else(|| "NULL".to_string())
                        } else if let Ok(v) = row.try_get::<_, Option<NaiveDateTime>>(i) {
                            v.map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                                .unwrap_or_else(|| "NULL".to_string())
                        } else if let Ok(v) = row.try_get::<_, Option<DateTime<Utc>>>(i) {
                            v.map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                                .unwrap_or_else(|| "NULL".to_string())
                        } else if let Ok(v) = row.try_get::<_, Option<NaiveDate>>(i) {
                            v.map(|d| d.format("%Y-%m-%d").to_string())
                                .unwrap_or_else(|| "NULL".to_string())
                        } else if let Ok(v) = row.try_get::<_, Option<NaiveTime>>(i) {
                            v.map(|t| t.format("%H:%M:%S").to_string())
                                .unwrap_or_else(|| "NULL".to_string())
                        } else {
                            "<unsupported>".to_string()
                        }
                    })
                    .collect();
                ResultRow { values }
            })
            .collect();

        let row_count = result_rows.len();
        let execution_time_ms = start.elapsed().as_millis();

        Ok(QueryResults {
            columns,
            rows: result_rows,
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
        "SELECT * FROM \"{}\" LIMIT {}",
        table_name.replace('"', "\"\""),
        limit
    );
    execute_query(config, &sql)
}

/// Connect to PostgreSQL
fn connect(config: &ConnectionConfig) -> Result<Client, String> {
    Client::connect(&config.postgres_url(), NoTls).map_err(|e| format!("Connection failed: {}", e))
}

/// Test connection
#[allow(dead_code)]
pub fn test_connection(config: &ConnectionConfig) -> Result<String, String> {
    let config = config.clone();
    run_blocking(move || {
        let mut client = connect(&config)?;
        let row = client
            .query_one("SELECT version()", &[])
            .map_err(|e| format!("Query failed: {}", e))?;
        let version: String = row.get(0);
        Ok(version)
    })
}
