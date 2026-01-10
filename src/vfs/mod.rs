//! Virtual File System abstraction for R-DOS
//!
//! This module provides a `FileSystemProvider` trait that abstracts file system operations,
//! enabling support for virtual file systems like MCP-based remote filesystems.
//!
//! The default implementation (`LocalFS`) wraps the standard library's `std::fs` functions.

// VFS infrastructure for Q-LINK
#![allow(dead_code)]

mod local;
mod mcp;
mod routing;

pub use local::LocalFS;
pub use mcp::McpFS;
pub use routing::RoutingFS;

use anyhow::Result;
use std::fs::Permissions;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Metadata about a file or directory
#[derive(Debug, Clone)]
pub struct VfsMetadata {
    /// File size in bytes (0 for directories)
    pub len: u64,
    /// Whether this is a directory
    pub is_dir: bool,
    /// Whether this is a file
    pub is_file: bool,
    /// Whether this is a symlink
    pub is_symlink: bool,
    /// Last modification time
    pub modified: Option<SystemTime>,
    /// Creation time
    pub created: Option<SystemTime>,
    /// File permissions (Unix mode)
    pub permissions: Option<Permissions>,
    /// Whether the file is read-only
    pub readonly: bool,
}

impl VfsMetadata {
    /// Get the file size
    pub fn len(&self) -> u64 {
        self.len
    }

    /// Check if the file is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Check if this is a directory
    pub fn is_dir(&self) -> bool {
        self.is_dir
    }

    /// Check if this is a file
    pub fn is_file(&self) -> bool {
        self.is_file
    }

    /// Check if this is a symlink
    pub fn is_symlink(&self) -> bool {
        self.is_symlink
    }

    /// Get modification time
    pub fn modified(&self) -> Result<SystemTime> {
        self.modified
            .ok_or_else(|| anyhow::anyhow!("Modification time not available"))
    }

    /// Get creation time
    pub fn created(&self) -> Result<SystemTime> {
        self.created
            .ok_or_else(|| anyhow::anyhow!("Creation time not available"))
    }

    /// Get permissions
    ///
    /// Returns the file permissions. For VFS implementations that don't support
    /// permissions, returns a default permission set based on the readonly flag.
    pub fn permissions(&self) -> Option<Permissions> {
        self.permissions.clone()
    }
}

/// A directory entry from reading a directory
#[derive(Debug, Clone)]
pub struct VfsDirEntry {
    /// The path to this entry
    pub path: PathBuf,
    /// The file name
    pub file_name: String,
    /// Metadata for this entry (may be unavailable for some VFS implementations)
    pub metadata: Option<VfsMetadata>,
    /// File type info
    pub is_dir: bool,
    pub is_file: bool,
    pub is_symlink: bool,
}

impl VfsDirEntry {
    /// Get the path to this entry
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    /// Get the file name
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// Get metadata (fetches if not already cached)
    pub fn metadata(&self) -> Result<VfsMetadata> {
        self.metadata
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Metadata not available"))
    }

    /// Check if this is a directory
    pub fn is_dir(&self) -> bool {
        self.is_dir
    }

    /// Check if this is a file
    pub fn is_file(&self) -> bool {
        self.is_file
    }

    /// Check if this is a symlink
    pub fn is_symlink(&self) -> bool {
        self.is_symlink
    }
}

/// File system provider trait for abstracting file operations
///
/// This trait enables the application to work with different file systems:
/// - Local file system (default)
/// - Remote file systems via MCP (Q-LINK feature)
/// - Mock file systems for testing
///
/// All paths should be absolute paths. Implementations should handle
/// path normalization and canonicalization internally where needed.
pub trait FileSystemProvider: Send + Sync {
    /// Read the contents of a directory
    ///
    /// Returns a list of directory entries. The order is not guaranteed.
    fn read_dir(&self, path: &Path) -> Result<Vec<VfsDirEntry>>;

    /// Read the entire contents of a file as bytes
    fn read_file(&self, path: &Path) -> Result<Vec<u8>>;

    /// Read the entire contents of a file as a UTF-8 string
    fn read_to_string(&self, path: &Path) -> Result<String> {
        let bytes = self.read_file(path)?;
        String::from_utf8(bytes).map_err(|e| anyhow::anyhow!("Invalid UTF-8: {}", e))
    }

    /// Write data to a file, creating it if it doesn't exist
    ///
    /// This will completely replace any existing content.
    fn write_file(&self, path: &Path, content: &[u8]) -> Result<()>;

    /// Get metadata for a file or directory
    fn metadata(&self, path: &Path) -> Result<VfsMetadata>;

    /// Check if a path exists
    fn exists(&self, path: &Path) -> bool;

    /// Check if a path is a directory
    fn is_dir(&self, path: &Path) -> bool;

    /// Check if a path is a file
    fn is_file(&self, path: &Path) -> bool;

    /// Create a directory (fails if parent doesn't exist)
    fn create_dir(&self, path: &Path) -> Result<()>;

    /// Create a directory and all parent directories
    fn create_dir_all(&self, path: &Path) -> Result<()>;

    /// Remove a file
    fn remove_file(&self, path: &Path) -> Result<()>;

    /// Remove an empty directory
    fn remove_dir(&self, path: &Path) -> Result<()>;

    /// Remove a directory and all its contents
    fn remove_dir_all(&self, path: &Path) -> Result<()>;

    /// Rename a file or directory (move within the same filesystem)
    fn rename(&self, from: &Path, to: &Path) -> Result<()>;

    /// Copy a file from one location to another
    fn copy(&self, from: &Path, to: &Path) -> Result<u64>;

    /// Read the target of a symbolic link
    fn read_link(&self, path: &Path) -> Result<PathBuf>;

    /// Set permissions on a file or directory
    fn set_permissions(&self, path: &Path, permissions: Permissions) -> Result<()>;

    /// Canonicalize a path (resolve symlinks and relative components)
    fn canonicalize(&self, path: &Path) -> Result<PathBuf>;

    /// Get the name of this provider (for display purposes)
    fn provider_name(&self) -> &str;

    /// Check if this is a local filesystem provider
    fn is_local(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vfs_metadata_methods() {
        let meta = VfsMetadata {
            len: 1024,
            is_dir: false,
            is_file: true,
            is_symlink: false,
            modified: Some(SystemTime::now()),
            created: Some(SystemTime::now()),
            permissions: None,
            readonly: false,
        };

        assert_eq!(meta.len(), 1024);
        assert!(!meta.is_empty());
        assert!(meta.is_file());
        assert!(!meta.is_dir());
        assert!(meta.modified().is_ok());
    }

    #[test]
    fn test_vfs_dir_entry() {
        let entry = VfsDirEntry {
            path: PathBuf::from("/tmp/test.txt"),
            file_name: "test.txt".to_string(),
            metadata: None,
            is_dir: false,
            is_file: true,
            is_symlink: false,
        };

        assert_eq!(entry.file_name(), "test.txt");
        assert!(entry.is_file());
        assert!(!entry.is_dir());
    }
}
