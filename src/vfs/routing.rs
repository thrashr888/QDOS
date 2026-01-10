//! Routing filesystem provider
//!
//! Routes file operations to different providers based on path prefixes.
//! This enables mounting MCP filesystems at specific paths.

use super::{FileSystemProvider, LocalFS, VfsDirEntry, VfsMetadata};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::fs::Permissions;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// A mount point configuration
#[derive(Clone)]
struct MountPoint {
    /// The path prefix where this provider is mounted
    path: PathBuf,
    /// The filesystem provider
    provider: Arc<dyn FileSystemProvider>,
}

/// Routing filesystem that dispatches to different providers based on path
///
/// Paths starting with a mount point prefix are routed to the corresponding provider.
/// All other paths are routed to the local filesystem.
pub struct RoutingFS {
    /// Local filesystem for unmounted paths
    local: LocalFS,
    /// Mount points (path prefix -> provider)
    mounts: RwLock<HashMap<PathBuf, Arc<dyn FileSystemProvider>>>,
}

impl Default for RoutingFS {
    fn default() -> Self {
        Self::new()
    }
}

impl RoutingFS {
    /// Create a new routing filesystem
    pub fn new() -> Self {
        Self {
            local: LocalFS::new(),
            mounts: RwLock::new(HashMap::new()),
        }
    }

    /// Mount a provider at a path
    ///
    /// The path should be an absolute path that will serve as the mount point.
    /// All paths starting with this prefix will be routed to the provider.
    pub fn mount(&self, path: PathBuf, provider: Arc<dyn FileSystemProvider>) -> Result<()> {
        let mut mounts = self
            .mounts
            .write()
            .map_err(|_| anyhow!("Failed to lock mounts"))?;

        // Create the mount point directory if it doesn't exist
        if !self.local.exists(&path) {
            self.local.create_dir_all(&path)?;
        }

        // Canonicalize the path to handle symlinks (e.g., /tmp -> /private/tmp on macOS)
        let canonical_path = self.local.canonicalize(&path).unwrap_or(path);

        mounts.insert(canonical_path, provider);
        Ok(())
    }

    /// Unmount a provider at a path
    pub fn unmount(&self, path: &Path) -> Result<Option<Arc<dyn FileSystemProvider>>> {
        let mut mounts = self
            .mounts
            .write()
            .map_err(|_| anyhow!("Failed to lock mounts"))?;
        // Canonicalize to match how we stored the mount
        let canonical_path = self
            .local
            .canonicalize(path)
            .unwrap_or_else(|_| path.to_path_buf());
        Ok(mounts.remove(&canonical_path))
    }

    /// Check if a path is a mount point
    pub fn is_mounted(&self, path: &Path) -> bool {
        let canonical_path = self
            .local
            .canonicalize(path)
            .unwrap_or_else(|_| path.to_path_buf());
        if let Ok(mounts) = self.mounts.read() {
            mounts.contains_key(&canonical_path)
        } else {
            false
        }
    }

    /// Check if a path is under any mount point
    pub fn is_mounted_path(&self, path: &Path) -> bool {
        // Canonicalize the path to handle symlinks (e.g., /tmp -> /private/tmp on macOS)
        let canonical_path = self
            .local
            .canonicalize(path)
            .unwrap_or_else(|_| path.to_path_buf());
        if let Ok(mounts) = self.mounts.read() {
            for mount_point in mounts.keys() {
                if canonical_path.starts_with(mount_point) {
                    return true;
                }
            }
        }
        false
    }

    /// Get all mount points
    pub fn mount_points(&self) -> Vec<PathBuf> {
        if let Ok(mounts) = self.mounts.read() {
            mounts.keys().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Find the provider for a path
    ///
    /// Returns the provider and the relative path within that provider.
    fn find_provider(&self, path: &Path) -> Option<(Arc<dyn FileSystemProvider>, PathBuf)> {
        let mounts = self.mounts.read().ok()?;

        // Canonicalize the path to handle symlinks
        let canonical_path = self
            .local
            .canonicalize(path)
            .unwrap_or_else(|_| path.to_path_buf());

        // Find the longest matching prefix
        let mut best_match: Option<(&PathBuf, &Arc<dyn FileSystemProvider>)> = None;

        for (mount_path, provider) in mounts.iter() {
            if canonical_path.starts_with(mount_path) {
                match &best_match {
                    None => best_match = Some((mount_path, provider)),
                    Some((best_path, _)) => {
                        if mount_path.as_os_str().len() > best_path.as_os_str().len() {
                            best_match = Some((mount_path, provider));
                        }
                    }
                }
            }
        }

        best_match.map(|(mount_path, provider)| {
            // Calculate relative path using the canonical path
            let relative = canonical_path
                .strip_prefix(mount_path)
                .unwrap_or(Path::new("/"));
            let relative_path = if relative.as_os_str().is_empty() {
                PathBuf::from("/")
            } else {
                PathBuf::from("/").join(relative)
            };
            (Arc::clone(provider), relative_path)
        })
    }

    /// Route a path operation to the appropriate provider
    fn route<F, T>(&self, path: &Path, op: F) -> Result<T>
    where
        F: FnOnce(&dyn FileSystemProvider, &Path) -> Result<T>,
    {
        if let Some((provider, relative_path)) = self.find_provider(path) {
            op(provider.as_ref(), &relative_path)
        } else {
            op(&self.local, path)
        }
    }
}

impl FileSystemProvider for RoutingFS {
    fn read_dir(&self, path: &Path) -> Result<Vec<VfsDirEntry>> {
        // Special handling for mount point directories
        // Show both local entries and mount points
        if let Some((provider, relative_path)) = self.find_provider(path) {
            let mut entries = provider.read_dir(&relative_path)?;
            // Adjust paths to be absolute
            for entry in &mut entries {
                if let Some((mount_path, _)) = self.find_provider(path).map(|(_, rp)| {
                    let mount = path
                        .ancestors()
                        .find(|p| self.is_mounted(p))
                        .unwrap_or(path);
                    (mount.to_path_buf(), rp)
                }) {
                    let full_path =
                        mount_path.join(entry.path.strip_prefix("/").unwrap_or(&entry.path));
                    entry.path = full_path;
                }
            }
            Ok(entries)
        } else {
            let mut entries = self.local.read_dir(path)?;

            // Add mount points as virtual directories
            if let Ok(mounts) = self.mounts.read() {
                for mount_path in mounts.keys() {
                    if mount_path.parent() == Some(path) {
                        // This mount point is a direct child
                        let name = mount_path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "mcp".to_string());

                        // Check if already in entries
                        if !entries.iter().any(|e| e.path == *mount_path) {
                            entries.push(VfsDirEntry {
                                path: mount_path.clone(),
                                file_name: name,
                                metadata: Some(VfsMetadata {
                                    len: 0,
                                    is_dir: true,
                                    is_file: false,
                                    is_symlink: false,
                                    modified: None,
                                    created: None,
                                    permissions: None,
                                    readonly: true,
                                }),
                                is_dir: true,
                                is_file: false,
                                is_symlink: false,
                            });
                        }
                    }
                }
            }

            Ok(entries)
        }
    }

    fn read_file(&self, path: &Path) -> Result<Vec<u8>> {
        self.route(path, |p, path| p.read_file(path))
    }

    fn read_to_string(&self, path: &Path) -> Result<String> {
        self.route(path, |p, path| p.read_to_string(path))
    }

    fn write_file(&self, path: &Path, content: &[u8]) -> Result<()> {
        self.route(path, |p, path| p.write_file(path, content))
    }

    fn metadata(&self, path: &Path) -> Result<VfsMetadata> {
        self.route(path, |p, path| p.metadata(path))
    }

    fn exists(&self, path: &Path) -> bool {
        if let Some((provider, relative_path)) = self.find_provider(path) {
            provider.exists(&relative_path)
        } else {
            // Check if it's a mount point
            if self.is_mounted(path) {
                return true;
            }
            self.local.exists(path)
        }
    }

    fn is_dir(&self, path: &Path) -> bool {
        if let Some((provider, relative_path)) = self.find_provider(path) {
            provider.is_dir(&relative_path)
        } else {
            // Mount points are always directories
            if self.is_mounted(path) {
                return true;
            }
            self.local.is_dir(path)
        }
    }

    fn is_file(&self, path: &Path) -> bool {
        if let Some((provider, relative_path)) = self.find_provider(path) {
            provider.is_file(&relative_path)
        } else {
            self.local.is_file(path)
        }
    }

    fn create_dir(&self, path: &Path) -> Result<()> {
        self.route(path, |p, path| p.create_dir(path))
    }

    fn create_dir_all(&self, path: &Path) -> Result<()> {
        self.route(path, |p, path| p.create_dir_all(path))
    }

    fn remove_file(&self, path: &Path) -> Result<()> {
        self.route(path, |p, path| p.remove_file(path))
    }

    fn remove_dir(&self, path: &Path) -> Result<()> {
        self.route(path, |p, path| p.remove_dir(path))
    }

    fn remove_dir_all(&self, path: &Path) -> Result<()> {
        self.route(path, |p, path| p.remove_dir_all(path))
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        // Both paths must be in the same provider
        let from_provider = self.find_provider(from);
        let to_provider = self.find_provider(to);

        match (from_provider, to_provider) {
            (Some((fp, from_rel)), Some((tp, to_rel))) => {
                // Check if same provider (by name)
                if fp.provider_name() == tp.provider_name() {
                    fp.rename(&from_rel, &to_rel)
                } else {
                    Err(anyhow!("Cannot rename across different filesystems"))
                }
            }
            (None, None) => self.local.rename(from, to),
            _ => Err(anyhow!("Cannot rename across local and remote filesystems")),
        }
    }

    fn copy(&self, from: &Path, to: &Path) -> Result<u64> {
        // Copy can work across providers by reading and writing
        let content = self.read_file(from)?;
        self.write_file(to, &content)?;
        Ok(content.len() as u64)
    }

    fn read_link(&self, path: &Path) -> Result<PathBuf> {
        self.route(path, |p, path| p.read_link(path))
    }

    fn set_permissions(&self, path: &Path, permissions: Permissions) -> Result<()> {
        self.route(path, |p, path| p.set_permissions(path, permissions))
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        if let Some((provider, relative_path)) = self.find_provider(path) {
            // For remote paths, just normalize
            let canonical = provider.canonicalize(&relative_path)?;
            // Re-add the mount prefix
            let mount = path
                .ancestors()
                .find(|p| self.is_mounted(p))
                .unwrap_or(path);
            Ok(mount.join(canonical.strip_prefix("/").unwrap_or(&canonical)))
        } else {
            self.local.canonicalize(path)
        }
    }

    fn provider_name(&self) -> &str {
        "routing"
    }

    fn is_local(&self) -> bool {
        false // Routing FS may contain non-local providers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routing_fs_new() {
        let fs = RoutingFS::new();
        assert!(fs.mount_points().is_empty());
    }

    #[test]
    fn test_routing_fs_mount_unmount() {
        let fs = RoutingFS::new();
        let provider: Arc<dyn FileSystemProvider> = Arc::new(LocalFS::new());
        let path = PathBuf::from("/tmp/test_mount");

        // Mount
        fs.mount(path.clone(), provider).unwrap();
        assert!(fs.is_mounted(&path));
        assert_eq!(fs.mount_points().len(), 1);

        // Unmount
        let unmounted = fs.unmount(&path).unwrap();
        assert!(unmounted.is_some());
        assert!(!fs.is_mounted(&path));
        assert!(fs.mount_points().is_empty());
    }

    #[test]
    fn test_routing_fs_local_passthrough() {
        let fs = RoutingFS::new();

        // Should work with local paths
        assert!(fs.exists(Path::new("/")));
        assert!(fs.is_dir(Path::new("/")));
    }
}
