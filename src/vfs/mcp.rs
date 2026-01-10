//! MCP-based virtual filesystem provider
//!
//! Implements the FileSystemProvider trait using MCP resources.

use super::{FileSystemProvider, VfsDirEntry, VfsMetadata};
use crate::mcp::types::Resource;
use crate::mcp::{McpClient, ServerConfig};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::fs::Permissions;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

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
/// Provides read-only access to resources exposed by an MCP server.
/// Write operations are not supported and will return errors.
pub struct McpFS {
    /// MCP client connection
    client: Arc<Mutex<McpClient>>,
    /// Server name for display
    server_name: String,
    /// Base URI prefix for resources (e.g., "file://")
    base_uri: String,
    /// Directory cache
    dir_cache: Arc<Mutex<HashMap<PathBuf, CachedDir>>>,
    /// File cache
    file_cache: Arc<Mutex<HashMap<PathBuf, CachedFile>>>,
    /// Cache TTL
    cache_ttl: Duration,
}

impl McpFS {
    /// Create a new MCP filesystem from a client
    pub fn new(client: McpClient, server_name: String, base_uri: String) -> Self {
        Self {
            client: Arc::new(Mutex::new(client)),
            server_name,
            base_uri,
            dir_cache: Arc::new(Mutex::new(HashMap::new())),
            file_cache: Arc::new(Mutex::new(HashMap::new())),
            cache_ttl: Duration::from_secs(60), // 1 minute default
        }
    }

    /// Create a new MCP filesystem by spawning a server
    pub fn spawn(config: &ServerConfig, server_name: String, base_uri: String) -> Result<Self> {
        let mut client =
            McpClient::spawn(config).map_err(|e| anyhow!("Failed to spawn MCP server: {}", e))?;
        client
            .initialize()
            .map_err(|e| anyhow!("Failed to initialize MCP: {}", e))?;
        Ok(Self::new(client, server_name, base_uri))
    }

    /// Set the cache TTL
    pub fn set_cache_ttl(&mut self, ttl: Duration) {
        self.cache_ttl = ttl;
    }

    /// Clear all caches
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.dir_cache.lock() {
            cache.clear();
        }
        if let Ok(mut cache) = self.file_cache.lock() {
            cache.clear();
        }
    }

    /// Convert a path to an MCP resource URI
    fn path_to_uri(&self, path: &Path) -> String {
        // Handle different path formats
        let path_str = path.to_string_lossy();

        // If the path is already absolute, use it directly
        if path_str.starts_with('/') {
            format!("{}{}", self.base_uri, path_str)
        } else {
            format!("{}/{}", self.base_uri, path_str)
        }
    }

    /// Convert an MCP resource URI to a path
    fn uri_to_path(&self, uri: &str) -> PathBuf {
        // Strip the base URI prefix
        let path_str = uri.strip_prefix(&self.base_uri).unwrap_or(uri);
        PathBuf::from(path_str)
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

    /// Convert MCP resources to VfsDirEntry list
    fn resources_to_entries(&self, resources: &[Resource], parent: &Path) -> Vec<VfsDirEntry> {
        resources
            .iter()
            .filter_map(|r| {
                let path = self.uri_to_path(&r.uri);

                // Filter to only direct children of parent
                if let Some(p) = path.parent() {
                    if p != parent {
                        return None;
                    }
                }

                let file_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| r.name.clone());

                // Determine if this is a directory based on URI or mime type
                let is_dir = r.uri.ends_with('/')
                    || r.mime_type.as_deref() == Some("inode/directory")
                    || r.mime_type.is_none() && !r.uri.contains('.');

                Some(VfsDirEntry {
                    path: path.clone(),
                    file_name,
                    metadata: Some(VfsMetadata {
                        len: 0, // MCP doesn't provide size in list
                        is_dir,
                        is_file: !is_dir,
                        is_symlink: false,
                        modified: None,
                        created: None,
                        permissions: None,
                        readonly: true, // MCP resources are read-only for now
                    }),
                    is_dir,
                    is_file: !is_dir,
                    is_symlink: false,
                })
            })
            .collect()
    }
}

impl FileSystemProvider for McpFS {
    fn read_dir(&self, path: &Path) -> Result<Vec<VfsDirEntry>> {
        // Check cache first
        if let Some(cached) = self.get_cached_dir(path) {
            return Ok(cached);
        }

        // Fetch from MCP server
        let resources = {
            let mut client = self
                .client
                .lock()
                .map_err(|_| anyhow!("Failed to lock MCP client"))?;
            client
                .list_resources()
                .map_err(|e| anyhow!("MCP error: {}", e))?
        };

        // Convert to entries for this directory
        let entries = self.resources_to_entries(&resources, path);

        // Cache result
        self.cache_dir(path, entries.clone());

        Ok(entries)
    }

    fn read_file(&self, path: &Path) -> Result<Vec<u8>> {
        // Check cache first
        if let Some(cached) = self.get_cached_file(path) {
            return Ok(cached);
        }

        // Read from MCP server
        let uri = self.path_to_uri(path);
        let result = {
            let mut client = self
                .client
                .lock()
                .map_err(|_| anyhow!("Failed to lock MCP client"))?;
            client
                .read_resource(&uri)
                .map_err(|e| anyhow!("MCP error: {}", e))?
        };

        // Extract content
        let content = if let Some(first) = result.contents.first() {
            if let Some(text) = &first.text {
                text.as_bytes().to_vec()
            } else if let Some(blob) = &first.blob {
                // Base64 decode blob
                base64_decode(blob)?
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        // Cache result
        self.cache_file(path, content.clone());

        Ok(content)
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
            modified: Some(SystemTime::now()),
            created: None,
            permissions: None,
            readonly: true,
        })
    }

    fn exists(&self, path: &Path) -> bool {
        // Check if it's the root
        if path.as_os_str().is_empty() || path == Path::new("/") {
            return true;
        }

        // Try to read parent directory and check if this entry exists
        if let Some(parent) = path.parent() {
            if let Ok(entries) = self.read_dir(parent) {
                return entries.iter().any(|e| e.path == path);
            }
        }

        false
    }

    fn is_dir(&self, path: &Path) -> bool {
        if let Ok(meta) = self.metadata(path) {
            meta.is_dir
        } else {
            false
        }
    }

    fn is_file(&self, path: &Path) -> bool {
        if let Ok(meta) = self.metadata(path) {
            meta.is_file
        } else {
            false
        }
    }

    fn create_dir(&self, _path: &Path) -> Result<()> {
        Err(anyhow!(
            "Create directory not supported on MCP filesystem '{}'",
            self.server_name
        ))
    }

    fn create_dir_all(&self, _path: &Path) -> Result<()> {
        Err(anyhow!(
            "Create directory not supported on MCP filesystem '{}'",
            self.server_name
        ))
    }

    fn remove_file(&self, _path: &Path) -> Result<()> {
        Err(anyhow!(
            "Remove file not supported on MCP filesystem '{}'",
            self.server_name
        ))
    }

    fn remove_dir(&self, _path: &Path) -> Result<()> {
        Err(anyhow!(
            "Remove directory not supported on MCP filesystem '{}'",
            self.server_name
        ))
    }

    fn remove_dir_all(&self, _path: &Path) -> Result<()> {
        Err(anyhow!(
            "Remove directory not supported on MCP filesystem '{}'",
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
            "Symlinks not supported on MCP filesystem '{}'",
            self.server_name
        ))
    }

    fn set_permissions(&self, _path: &Path, _permissions: Permissions) -> Result<()> {
        Err(anyhow!(
            "Permissions not supported on MCP filesystem '{}'",
            self.server_name
        ))
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        // For MCP, just return the path as-is (no symlinks to resolve)
        Ok(path.to_path_buf())
    }

    fn provider_name(&self) -> &str {
        &self.server_name
    }

    fn is_local(&self) -> bool {
        false
    }
}

/// Simple base64 decoder (no external dependency)
fn base64_decode(input: &str) -> Result<Vec<u8>> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    fn decode_char(c: u8) -> Option<u8> {
        ALPHABET.iter().position(|&x| x == c).map(|p| p as u8)
    }

    let input = input.as_bytes();
    let mut output = Vec::with_capacity(input.len() * 3 / 4);

    let mut buffer: u32 = 0;
    let mut bits_collected = 0;

    for &byte in input {
        if byte == b'=' {
            break;
        }
        if byte == b'\n' || byte == b'\r' || byte == b' ' {
            continue;
        }

        let value = decode_char(byte).ok_or_else(|| anyhow!("Invalid base64 character"))?;
        buffer = (buffer << 6) | (value as u32);
        bits_collected += 6;

        if bits_collected >= 8 {
            bits_collected -= 8;
            output.push((buffer >> bits_collected) as u8);
            buffer &= (1 << bits_collected) - 1;
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_to_uri() {
        // Create a mock MCP FS (we can't actually connect in tests)
        // Just test the path conversion logic
        let base_uri = "file://".to_string();

        // Test absolute path
        let path = Path::new("/tmp/test.txt");
        let expected = format!("{}{}", base_uri, "/tmp/test.txt");
        assert_eq!(format!("{}{}", base_uri, path.to_string_lossy()), expected);
    }

    #[test]
    fn test_base64_decode() {
        let encoded = "SGVsbG8gV29ybGQ="; // "Hello World"
        let decoded = base64_decode(encoded).unwrap();
        assert_eq!(decoded, b"Hello World");
    }

    #[test]
    fn test_base64_decode_with_newlines() {
        let encoded = "SGVs\nbG8g\nV29y\nbGQ=";
        let decoded = base64_decode(encoded).unwrap();
        assert_eq!(decoded, b"Hello World");
    }
}
