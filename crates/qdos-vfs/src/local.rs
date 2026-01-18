//! Local filesystem implementation of FileSystemProvider
//!
//! This module provides `LocalFS`, which wraps the standard library's `std::fs`
//! functions to implement the `FileSystemProvider` trait.

use crate::{FileSystemProvider, VfsDirEntry, VfsMetadata};
use anyhow::Result;
use std::fs::{self, Permissions};
use std::path::{Path, PathBuf};

/// Local filesystem provider wrapping std::fs
///
/// This is the default filesystem provider that operates on the local
/// filesystem using the standard library's fs functions.
#[derive(Debug, Clone, Default)]
pub struct LocalFS;

impl LocalFS {
    /// Create a new LocalFS instance
    pub fn new() -> Self {
        Self
    }
}

impl FileSystemProvider for LocalFS {
    fn read_dir(&self, path: &Path) -> Result<Vec<VfsDirEntry>> {
        let entries = fs::read_dir(path)?;
        let mut result = Vec::new();

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();
            let file_type = entry.file_type()?;

            // Try to get metadata, but don't fail if unavailable
            let metadata = entry.metadata().ok().map(|m| convert_metadata(&m));

            result.push(VfsDirEntry {
                path,
                file_name,
                metadata,
                is_dir: file_type.is_dir(),
                is_file: file_type.is_file(),
                is_symlink: file_type.is_symlink(),
            });
        }

        Ok(result)
    }

    fn read_file(&self, path: &Path) -> Result<Vec<u8>> {
        fs::read(path).map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))
    }

    fn read_to_string(&self, path: &Path) -> Result<String> {
        fs::read_to_string(path).map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))
    }

    fn write_file(&self, path: &Path, content: &[u8]) -> Result<()> {
        fs::write(path, content).map_err(|e| anyhow::anyhow!("Failed to write file: {}", e))
    }

    fn metadata(&self, path: &Path) -> Result<VfsMetadata> {
        let m = fs::metadata(path)?;
        Ok(convert_metadata(&m))
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn create_dir(&self, path: &Path) -> Result<()> {
        fs::create_dir(path).map_err(|e| anyhow::anyhow!("Failed to create directory: {}", e))
    }

    fn create_dir_all(&self, path: &Path) -> Result<()> {
        fs::create_dir_all(path).map_err(|e| anyhow::anyhow!("Failed to create directories: {}", e))
    }

    fn remove_file(&self, path: &Path) -> Result<()> {
        fs::remove_file(path).map_err(|e| anyhow::anyhow!("Failed to remove file: {}", e))
    }

    fn remove_dir(&self, path: &Path) -> Result<()> {
        fs::remove_dir(path).map_err(|e| anyhow::anyhow!("Failed to remove directory: {}", e))
    }

    fn remove_dir_all(&self, path: &Path) -> Result<()> {
        fs::remove_dir_all(path)
            .map_err(|e| anyhow::anyhow!("Failed to remove directory tree: {}", e))
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        fs::rename(from, to).map_err(|e| anyhow::anyhow!("Failed to rename: {}", e))
    }

    fn copy(&self, from: &Path, to: &Path) -> Result<u64> {
        fs::copy(from, to).map_err(|e| anyhow::anyhow!("Failed to copy: {}", e))
    }

    fn read_link(&self, path: &Path) -> Result<PathBuf> {
        fs::read_link(path).map_err(|e| anyhow::anyhow!("Failed to read link: {}", e))
    }

    fn set_permissions(&self, path: &Path, permissions: Permissions) -> Result<()> {
        fs::set_permissions(path, permissions)
            .map_err(|e| anyhow::anyhow!("Failed to set permissions: {}", e))
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        fs::canonicalize(path).map_err(|e| anyhow::anyhow!("Failed to canonicalize path: {}", e))
    }

    fn provider_name(&self) -> &str {
        "local"
    }

    fn is_local(&self) -> bool {
        true
    }
}

/// Convert std::fs::Metadata to VfsMetadata
fn convert_metadata(m: &fs::Metadata) -> VfsMetadata {
    VfsMetadata {
        len: m.len(),
        is_dir: m.is_dir(),
        is_file: m.is_file(),
        is_symlink: m.is_symlink(),
        modified: m.modified().ok(),
        created: m.created().ok(),
        permissions: Some(m.permissions()),
        readonly: m.permissions().readonly(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_local_fs_read_dir() {
        let fs = LocalFS::new();
        let current_dir = env::current_dir().unwrap();
        let entries = fs.read_dir(&current_dir);
        assert!(entries.is_ok());
        let entries = entries.unwrap();
        assert!(!entries.is_empty());
    }

    #[test]
    fn test_local_fs_exists() {
        let fs = LocalFS::new();
        let current_dir = env::current_dir().unwrap();
        assert!(fs.exists(&current_dir));
        assert!(fs.is_dir(&current_dir));
        assert!(!fs.is_file(&current_dir));
    }

    #[test]
    fn test_local_fs_metadata() {
        let fs = LocalFS::new();
        let current_dir = env::current_dir().unwrap();
        let meta = fs.metadata(&current_dir);
        assert!(meta.is_ok());
        let meta = meta.unwrap();
        assert!(meta.is_dir());
        assert!(!meta.is_file());
    }

    #[test]
    fn test_local_fs_provider_info() {
        let fs = LocalFS::new();
        assert_eq!(fs.provider_name(), "local");
        assert!(fs.is_local());
    }

    #[test]
    fn test_local_fs_canonicalize() {
        let fs = LocalFS::new();
        let path = PathBuf::from(".");
        let canonical = fs.canonicalize(&path);
        assert!(canonical.is_ok());
        let canonical = canonical.unwrap();
        assert!(canonical.is_absolute());
    }
}
