//! Redis operations

use super::state::{RedisConnection, RedisKey, RedisKeyType, RedisValue};
use redis::{Client, Commands, Connection, ConnectionLike};

/// Thread-safe connection wrapper
pub struct RedisClient {
    connection: Option<Connection>,
}

impl RedisClient {
    pub fn new() -> Self {
        Self { connection: None }
    }

    /// Connect to Redis
    pub fn connect(&mut self, config: &RedisConnection) -> Result<(), String> {
        let url = config.to_url();
        let client =
            Client::open(url.as_str()).map_err(|e| format!("Failed to create client: {}", e))?;

        let conn = client
            .get_connection()
            .map_err(|e| format!("Failed to connect: {}", e))?;

        self.connection = Some(conn);
        Ok(())
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    /// Disconnect
    pub fn disconnect(&mut self) {
        self.connection = None;
    }

    /// Ping the server
    pub fn ping(&mut self) -> Result<bool, String> {
        let conn = self
            .connection
            .as_mut()
            .ok_or_else(|| "Not connected".to_string())?;
        conn.check_connection();
        Ok(conn.is_open())
    }

    /// Scan keys with cursor-based pagination
    pub fn scan_keys(
        &mut self,
        cursor: u64,
        pattern: &str,
        count: usize,
    ) -> Result<(u64, Vec<RedisKey>), String> {
        let conn = self
            .connection
            .as_mut()
            .ok_or_else(|| "Not connected".to_string())?;

        // Use SCAN command with pattern and count
        let scan_pattern = if pattern.is_empty() { "*" } else { pattern };

        let result: redis::RedisResult<(u64, Vec<String>)> = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(scan_pattern)
            .arg("COUNT")
            .arg(count)
            .query(conn);

        let (new_cursor, key_names) = result.map_err(|e| format!("SCAN failed: {}", e))?;

        // Get type for each key
        let mut keys = Vec::new();
        for name in key_names {
            let key_type = self.get_key_type(&name).unwrap_or(RedisKeyType::Unknown);
            let ttl = self.get_ttl(&name).ok();
            keys.push(RedisKey {
                name,
                key_type,
                ttl,
                memory_bytes: None,
            });
        }

        Ok((new_cursor, keys))
    }

    /// Get key type
    fn get_key_type(&mut self, key: &str) -> Result<RedisKeyType, String> {
        let conn = self
            .connection
            .as_mut()
            .ok_or_else(|| "Not connected".to_string())?;

        let type_str: String = redis::cmd("TYPE")
            .arg(key)
            .query(conn)
            .map_err(|e| format!("TYPE failed: {}", e))?;

        Ok(RedisKeyType::from_str(&type_str))
    }

    /// Get TTL for key
    fn get_ttl(&mut self, key: &str) -> Result<i64, String> {
        let conn = self
            .connection
            .as_mut()
            .ok_or_else(|| "Not connected".to_string())?;

        let ttl: i64 = conn.ttl(key).map_err(|e| format!("TTL failed: {}", e))?;
        Ok(ttl)
    }

    /// Get key value
    pub fn get_value(&mut self, key: &RedisKey) -> Result<RedisValue, String> {
        let conn = self
            .connection
            .as_mut()
            .ok_or_else(|| "Not connected".to_string())?;

        match key.key_type {
            RedisKeyType::String => {
                let value: String = conn
                    .get(&key.name)
                    .map_err(|e| format!("GET failed: {}", e))?;
                Ok(RedisValue::String(value))
            }
            RedisKeyType::List => {
                let values: Vec<String> = conn
                    .lrange(&key.name, 0, -1)
                    .map_err(|e| format!("LRANGE failed: {}", e))?;
                Ok(RedisValue::List(values))
            }
            RedisKeyType::Set => {
                let values: Vec<String> = conn
                    .smembers(&key.name)
                    .map_err(|e| format!("SMEMBERS failed: {}", e))?;
                Ok(RedisValue::Set(values))
            }
            RedisKeyType::ZSet => {
                let values: Vec<(String, f64)> = conn
                    .zrange_withscores(&key.name, 0, -1)
                    .map_err(|e| format!("ZRANGE failed: {}", e))?;
                Ok(RedisValue::ZSet(values))
            }
            RedisKeyType::Hash => {
                let values: Vec<(String, String)> = conn
                    .hgetall(&key.name)
                    .map_err(|e| format!("HGETALL failed: {}", e))?;
                Ok(RedisValue::Hash(values))
            }
            RedisKeyType::Stream => {
                // Simplified stream reading - just get last 100 entries
                let result: Vec<redis::Value> = redis::cmd("XRANGE")
                    .arg(&key.name)
                    .arg("-")
                    .arg("+")
                    .arg("COUNT")
                    .arg(100)
                    .query(conn)
                    .map_err(|e| format!("XRANGE failed: {}", e))?;

                // Convert to strings for display
                let entries: Vec<String> = result.iter().map(|v| format!("{:?}", v)).collect();
                Ok(RedisValue::Stream(entries))
            }
            RedisKeyType::Unknown => Ok(RedisValue::None),
        }
    }

    /// Set string value
    pub fn set_string(&mut self, key: &str, value: &str) -> Result<(), String> {
        let conn = self
            .connection
            .as_mut()
            .ok_or_else(|| "Not connected".to_string())?;

        conn.set::<_, _, ()>(key, value)
            .map_err(|e| format!("SET failed: {}", e))?;
        Ok(())
    }

    /// Delete key
    pub fn delete_key(&mut self, key: &str) -> Result<(), String> {
        let conn = self
            .connection
            .as_mut()
            .ok_or_else(|| "Not connected".to_string())?;

        conn.del::<_, ()>(key)
            .map_err(|e| format!("DEL failed: {}", e))?;
        Ok(())
    }

    /// Get server info
    pub fn get_info(&mut self) -> Result<Vec<String>, String> {
        let conn = self
            .connection
            .as_mut()
            .ok_or_else(|| "Not connected".to_string())?;

        let info: String = redis::cmd("INFO")
            .query(conn)
            .map_err(|e| format!("INFO failed: {}", e))?;

        Ok(info.lines().map(String::from).collect())
    }

    /// Get database size (number of keys)
    pub fn dbsize(&mut self) -> Result<usize, String> {
        let conn = self
            .connection
            .as_mut()
            .ok_or_else(|| "Not connected".to_string())?;

        let size: usize = redis::cmd("DBSIZE")
            .query(conn)
            .map_err(|e| format!("DBSIZE failed: {}", e))?;

        Ok(size)
    }
}

impl Default for RedisClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if redis-cli is available (for testing)
pub fn check_redis_cli() -> bool {
    std::process::Command::new("redis-cli")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
