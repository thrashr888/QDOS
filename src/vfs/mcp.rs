//! MCP-based virtual filesystem provider
//!
//! Implements the FileSystemProvider trait using MCP tools.
//! Uses tools like `list_directory` and `read_text_file` from
//! the MCP filesystem server.

use super::{FileSystemProvider, VfsDirEntry, VfsMetadata};
use crate::mcp::{McpClient, ServerConfig};
use anyhow::{anyhow, Result};
use serde_json::json;
use std::collections::HashMap;
use std::fs::Permissions;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Cache entry for directory listings
struct CachedDir {
    entries: Vec<VfsDirEntry>,
    cached_at: Instant,
}

/// Cache entry for file contents
struct CachedFile {
    content: Vec<u8>,
    cached_at: Instant,
}

/// MCP-based filesystem provider
///
/// Provides read-only access to files via MCP server tools.
/// Uses `list_directory`, `read_text_file`, and `get_file_info` tools.
pub struct McpFS {
    /// MCP client connection
    client: Arc<Mutex<McpClient>>,
    /// Server name for display
    server_name: String,
    /// Base path for the MCP server (e.g., "/tmp")
    base_path: String,
    /// Directory cache
    dir_cache: Arc<Mutex<HashMap<PathBuf, CachedDir>>>,
    /// File cache
    file_cache: Arc<Mutex<HashMap<PathBuf, CachedFile>>>,
    /// Cache TTL
    cache_ttl: Duration,
}

impl McpFS {
    /// Create a new MCP filesystem from a client
    pub fn new(client: McpClient, server_name: String, base_path: String) -> Self {
        Self {
            client: Arc::new(Mutex::new(client)),
            server_name,
            base_path,
            dir_cache: Arc::new(Mutex::new(HashMap::new())),
            file_cache: Arc::new(Mutex::new(HashMap::new())),
            cache_ttl: Duration::from_secs(60), // 1 minute default
        }
    }

    /// Create a new MCP filesystem by spawning a server
    pub fn spawn(config: &ServerConfig, server_name: String, base_path: String) -> Result<Self> {
        let mut client =
            McpClient::spawn(config).map_err(|e| anyhow!("Failed to spawn MCP server: {}", e))?;
        client
            .initialize()
            .map_err(|e| anyhow!("Failed to initialize MCP: {}", e))?;
        Ok(Self::new(client, server_name, base_path))
    }

    /// Set the cache TTL
    #[allow(dead_code)]
    pub fn set_cache_ttl(&mut self, ttl: Duration) {
        self.cache_ttl = ttl;
    }

    /// Clear all caches
    #[allow(dead_code)]
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.dir_cache.lock() {
            cache.clear();
        }
        if let Ok(mut cache) = self.file_cache.lock() {
            cache.clear();
        }
    }

    /// Check if a cached entry is still valid
    fn is_cache_valid(&self, cached_at: Instant) -> bool {
        cached_at.elapsed() < self.cache_ttl
    }

    /// Get cached directory entries if valid
    fn get_cached_dir(&self, path: &Path) -> Option<Vec<VfsDirEntry>> {
        let cache = self.dir_cache.lock().ok()?;
        let entry = cache.get(path)?;
        if self.is_cache_valid(entry.cached_at) {
            Some(entry.entries.clone())
        } else {
            None
        }
    }

    /// Cache directory entries
    fn cache_dir(&self, path: &Path, entries: Vec<VfsDirEntry>) {
        if let Ok(mut cache) = self.dir_cache.lock() {
            cache.insert(
                path.to_path_buf(),
                CachedDir {
                    entries,
                    cached_at: Instant::now(),
                },
            );
        }
    }

    /// Get cached file contents if valid
    fn get_cached_file(&self, path: &Path) -> Option<Vec<u8>> {
        let cache = self.file_cache.lock().ok()?;
        let entry = cache.get(path)?;
        if self.is_cache_valid(entry.cached_at) {
            Some(entry.content.clone())
        } else {
            None
        }
    }

    /// Cache file contents
    fn cache_file(&self, path: &Path, content: Vec<u8>) {
        if let Ok(mut cache) = self.file_cache.lock() {
            cache.insert(
                path.to_path_buf(),
                CachedFile {
                    content,
                    cached_at: Instant::now(),
                },
            );
        }
    }

    /// Extract text content from MCP tool result
    fn extract_text_content(&self, content: &[crate::mcp::types::Content]) -> Result<String> {
        for item in content {
            if let Some(text) = item.as_text() {
                return Ok(text.to_string());
            }
        }
        Err(anyhow!("No text content in MCP response"))
    }

    /// Convert a relative path to an absolute path using the server's base path
    fn to_server_path(&self, path: &Path) -> PathBuf {
        let path_str = path.to_string_lossy();
        if path_str == "/" || path_str.is_empty() {
            PathBuf::from(&self.base_path)
        } else {
            // Strip leading slash if present and join with base_path
            let relative = path_str.trim_start_matches('/');
            PathBuf::from(&self.base_path).join(relative)
        }
    }

    /// Call the list_directory tool
    fn call_list_directory(&self, path: &Path) -> Result<Vec<VfsDirEntry>> {
        let server_path = self.to_server_path(path);
        let path_str = server_path.to_string_lossy().to_string();

        let result = {
            let mut client = self
                .client
                .lock()
                .map_err(|_| anyhow!("Failed to lock MCP client"))?;

            client
                .call_tool("list_directory", Some(json!({"path": path_str})))
                .map_err(|e| anyhow!("MCP list_directory error: {}", e))?
        };

        // Extract text content from result
        let content = self.extract_text_content(&result.content)?;

        // Parse the result - format is "[DIR] name" or "[FILE] name" per line
        McpFS::parse_list_directory_result(&content, path)
    }

    /// Parse the list_directory tool result into VfsDirEntry list
    fn parse_list_directory_result(content: &str, parent: &Path) -> Result<Vec<VfsDirEntry>> {
        let mut entries = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let (is_dir, name) = if let Some(name) = line.strip_prefix("[DIR] ") {
                (true, name.to_string())
            } else if let Some(name) = line.strip_prefix("[FILE] ") {
                (false, name.to_string())
            } else {
                // Unknown format, skip
                continue;
            };

            let entry_path = parent.join(&name);

            entries.push(VfsDirEntry {
                path: entry_path,
                file_name: name,
                metadata: Some(VfsMetadata {
                    len: 0, // Size not provided by list_directory
                    is_dir,
                    is_file: !is_dir,
                    is_symlink: false,
                    modified: None,
                    created: None,
                    permissions: None,
                    readonly: true,
                }),
                is_dir,
                is_file: !is_dir,
                is_symlink: false,
            });
        }

        Ok(entries)
    }

    /// Call the read_text_file tool
    fn call_read_text_file(&self, path: &Path) -> Result<String> {
        let server_path = self.to_server_path(path);
        let path_str = server_path.to_string_lossy().to_string();

        let result = {
            let mut client = self
                .client
                .lock()
                .map_err(|_| anyhow!("Failed to lock MCP client"))?;

            client
                .call_tool("read_text_file", Some(json!({"path": path_str})))
                .map_err(|e| anyhow!("MCP read_text_file error: {}", e))?
        };

        self.extract_text_content(&result.content)
    }

    /// Call the get_file_info tool
    #[allow(dead_code)]
    fn call_get_file_info(&self, path: &Path) -> Result<VfsMetadata> {
        let server_path = self.to_server_path(path);
        let path_str = server_path.to_string_lossy().to_string();

        let result = {
            let mut client = self
                .client
                .lock()
                .map_err(|_| anyhow!("Failed to lock MCP client"))?;

            client
                .call_tool("get_file_info", Some(json!({"path": path_str})))
                .map_err(|e| anyhow!("MCP get_file_info error: {}", e))?
        };

        // Extract text and parse
        let content = self.extract_text_content(&result.content)?;
        self.parse_file_info_result(&content)
    }

    /// Parse the get_file_info tool result
    fn parse_file_info_result(&self, content: &str) -> Result<VfsMetadata> {
        // The result is a formatted string, parse it
        let is_dir = content.contains("Type: directory");
        let is_file = content.contains("Type: file");

        // Try to extract size
        let len = content
            .lines()
            .find(|l| l.starts_with("Size:"))
            .and_then(|l| l.split(':').nth(1))
            .and_then(|s| s.split_whitespace().next())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        Ok(VfsMetadata {
            len,
            is_dir,
            is_file,
            is_symlink: false,
            modified: None,
            created: None,
            permissions: None,
            readonly: true,
        })
    }
}

impl FileSystemProvider for McpFS {
    fn read_dir(&self, path: &Path) -> Result<Vec<VfsDirEntry>> {
        // Check cache first
        if let Some(cached) = self.get_cached_dir(path) {
            return Ok(cached);
        }

        // Call list_directory tool
        let entries = self.call_list_directory(path)?;

        // Cache result
        self.cache_dir(path, entries.clone());

        Ok(entries)
    }

    fn read_file(&self, path: &Path) -> Result<Vec<u8>> {
        // Check cache first
        if let Some(cached) = self.get_cached_file(path) {
            return Ok(cached);
        }

        // Call read_text_file tool
        let content = self.call_read_text_file(path)?;
        let bytes = content.into_bytes();

        // Cache result
        self.cache_file(path, bytes.clone());

        Ok(bytes)
    }

    fn read_to_string(&self, path: &Path) -> Result<String> {
        let bytes = self.read_file(path)?;
        String::from_utf8(bytes).map_err(|e| anyhow!("Invalid UTF-8: {}", e))
    }

    fn write_file(&self, _path: &Path, _content: &[u8]) -> Result<()> {
        Err(anyhow!(
            "Write operations not supported on MCP filesystem '{}'",
            self.server_name
        ))
    }

    fn metadata(&self, path: &Path) -> Result<VfsMetadata> {
        // Try to get from cache first
        if let Some(entries) = self.get_cached_dir(path.parent().unwrap_or(path)) {
            if let Some(entry) = entries.iter().find(|e| e.path == path) {
                if let Some(meta) = &entry.metadata {
                    return Ok(meta.clone());
                }
            }
        }

        // For root or uncached paths, return directory metadata
        Ok(VfsMetadata {
            len: 0,
            is_dir: true,
            is_file: false,
            is_symlink: false,
            modified: None,
            created: None,
            permissions: None,
            readonly: true,
        })
    }

    fn exists(&self, path: &Path) -> bool {
        // Check if in cache
        if self.get_cached_dir(path).is_some() {
            return true;
        }

        // Try to list it as a directory
        self.call_list_directory(path).is_ok()
    }

    fn is_dir(&self, path: &Path) -> bool {
        self.metadata(path).map(|m| m.is_dir).unwrap_or(false)
    }

    fn is_file(&self, path: &Path) -> bool {
        self.metadata(path).map(|m| m.is_file).unwrap_or(false)
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        // MCP paths are already canonical
        Ok(path.to_path_buf())
    }

    fn create_dir(&self, _path: &Path) -> Result<()> {
        Err(anyhow!(
            "Directory creation not supported on MCP filesystem '{}'",
            self.server_name
        ))
    }

    fn create_dir_all(&self, _path: &Path) -> Result<()> {
        Err(anyhow!(
            "Directory creation not supported on MCP filesystem '{}'",
            self.server_name
        ))
    }

    fn remove_file(&self, _path: &Path) -> Result<()> {
        Err(anyhow!(
            "File removal not supported on MCP filesystem '{}'",
            self.server_name
        ))
    }

    fn remove_dir(&self, _path: &Path) -> Result<()> {
        Err(anyhow!(
            "Directory removal not supported on MCP filesystem '{}'",
            self.server_name
        ))
    }

    fn remove_dir_all(&self, _path: &Path) -> Result<()> {
        Err(anyhow!(
            "Directory removal not supported on MCP filesystem '{}'",
            self.server_name
        ))
    }

    fn rename(&self, _from: &Path, _to: &Path) -> Result<()> {
        Err(anyhow!(
            "Rename not supported on MCP filesystem '{}'",
            self.server_name
        ))
    }

    fn copy(&self, _from: &Path, _to: &Path) -> Result<u64> {
        Err(anyhow!(
            "Copy not supported on MCP filesystem '{}'",
            self.server_name
        ))
    }

    fn read_link(&self, _path: &Path) -> Result<PathBuf> {
        Err(anyhow!(
            "Symlink reading not supported on MCP filesystem '{}'",
            self.server_name
        ))
    }

    fn set_permissions(&self, _path: &Path, _perm: Permissions) -> Result<()> {
        Err(anyhow!(
            "Permission changes not supported on MCP filesystem '{}'",
            self.server_name
        ))
    }

    fn is_local(&self) -> bool {
        false // This is a remote filesystem
    }

    fn provider_name(&self) -> &str {
        "mcp"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_list_directory() {
        let content = "[DIR] subdir\n[FILE] file.txt\n[DIR] another";
        let entries = McpFS::parse_list_directory_result(content, Path::new("/tmp")).unwrap();

        assert_eq!(entries.len(), 3);
        assert!(entries[0].is_dir);
        assert_eq!(entries[0].file_name, "subdir");
        assert!(!entries[1].is_dir);
        assert_eq!(entries[1].file_name, "file.txt");
        assert!(entries[2].is_dir);
        assert_eq!(entries[2].file_name, "another");
    }
}
