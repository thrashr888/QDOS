//! Routing filesystem provider
//!
//! Routes file operations to different providers based on path prefixes.
//! This enables mounting MCP filesystems at specific paths.

use super::{FileSystemProvider, LocalFS, VfsDirEntry, VfsMetadata};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::fs::Permissions;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Default TTL for cached directory listings (5 seconds)
const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(5);

/// A cached directory listing entry
#[derive(Clone)]
struct CacheEntry {
    /// The cached entries
    entries: Vec<VfsDirEntry>,
    /// When this entry was cached
    cached_at: Instant,
    /// TTL for this entry
    ttl: Duration,
}

impl CacheEntry {
    fn new(entries: Vec<VfsDirEntry>, ttl: Duration) -> Self {
        Self {
            entries,
            cached_at: Instant::now(),
            ttl,
        }
    }

    fn is_valid(&self) -> bool {
        self.cached_at.elapsed() < self.ttl
    }
}

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
    /// TTL cache for directory listings
    dir_cache: RwLock<HashMap<PathBuf, CacheEntry>>,
    /// Cache TTL
    cache_ttl: Duration,
    /// Whether a VFS operation is currently loading
    loading: AtomicBool,
    /// Count of active loading operations (for nested calls)
    loading_count: AtomicUsize,
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
            dir_cache: RwLock::new(HashMap::new()),
            cache_ttl: DEFAULT_CACHE_TTL,
            loading: AtomicBool::new(false),
            loading_count: AtomicUsize::new(0),
        }
    }

    /// Check if a VFS operation is currently loading
    pub fn is_loading(&self) -> bool {
        self.loading.load(Ordering::Relaxed)
    }

    /// Start a loading operation
    fn start_loading(&self) {
        let count = self.loading_count.fetch_add(1, Ordering::SeqCst);
        if count == 0 {
            self.loading.store(true, Ordering::SeqCst);
        }
    }

    /// End a loading operation
    fn end_loading(&self) {
        let count = self.loading_count.fetch_sub(1, Ordering::SeqCst);
        if count == 1 {
            self.loading.store(false, Ordering::SeqCst);
        }
    }

    /// Set the cache TTL
    pub fn set_cache_ttl(&mut self, ttl: Duration) {
        self.cache_ttl = ttl;
    }

    /// Clear the directory cache
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.dir_cache.write() {
            cache.clear();
        }
    }

    /// Invalidate a specific path in the cache
    pub fn invalidate_cache(&self, path: &Path) {
        if let Ok(mut cache) = self.dir_cache.write() {
            cache.remove(path);
            // Also invalidate parent directory
            if let Some(parent) = path.parent() {
                cache.remove(parent);
            }
        }
    }

    /// Get cached directory listing if valid
    fn get_cached(&self, path: &Path) -> Option<Vec<VfsDirEntry>> {
        if let Ok(cache) = self.dir_cache.read() {
            if let Some(entry) = cache.get(path) {
                if entry.is_valid() {
                    return Some(entry.entries.clone());
                }
            }
        }
        None
    }

    /// Store directory listing in cache
    fn cache_dir(&self, path: &Path, entries: Vec<VfsDirEntry>) {
        if let Ok(mut cache) = self.dir_cache.write() {
            cache.insert(path.to_path_buf(), CacheEntry::new(entries, self.cache_ttl));
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
        if let Ok(mounts) = self.mounts.read() {
            for mount_point in mounts.keys() {
                // Try to match the path against the mount point
                // Handle both canonicalized and non-canonicalized paths

                // First, try direct prefix match
                if path.starts_with(mount_point) {
                    return true;
                }

                // Try canonicalizing the path (may fail for virtual subdirs)
                if let Ok(canonical_path) = self.local.canonicalize(path) {
                    if canonical_path.starts_with(mount_point) {
                        return true;
                    }
                }

                // For virtual subdirectories, check if the path's existing prefix matches
                // E.g., /tmp/mcp/filesystem/subdir -> canonicalize /tmp/mcp/filesystem -> /private/tmp/mcp/filesystem
                let mut check_path = path.to_path_buf();
                while let Some(parent) = check_path.parent().map(|p| p.to_path_buf()) {
                    if let Ok(canonical_parent) = self.local.canonicalize(&parent) {
                        // Reconstruct the path with the canonical prefix
                        if let Ok(suffix) = path.strip_prefix(&parent) {
                            let canonical_full = canonical_parent.join(suffix);
                            if canonical_full.starts_with(mount_point) {
                                return true;
                            }
                        }
                    }

                    if parent.as_os_str().is_empty() || parent == Path::new("/") {
                        break;
                    }
                    check_path = parent;
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

        // Try to canonicalize the path. For virtual subdirectories that don't exist
        // on the local filesystem, canonicalization will fail. In that case, we need
        // to canonicalize the existing parent and reconstruct the full path.
        let canonical_path = if let Ok(canonical) = self.local.canonicalize(path) {
            canonical
        } else {
            // Path doesn't exist locally - try to canonicalize parent directories
            // E.g., /tmp/mcp/filesystem/virtual-subdir -> /private/tmp/mcp/filesystem/virtual-subdir
            let mut check_path = path.to_path_buf();
            let mut canonical_path = path.to_path_buf();

            while let Some(parent) = check_path.parent().map(|p| p.to_path_buf()) {
                if let Ok(canonical_parent) = self.local.canonicalize(&parent) {
                    // Found a real parent we can canonicalize
                    if let Ok(suffix) = path.strip_prefix(&parent) {
                        canonical_path = canonical_parent.join(suffix);
                        // Log the canonicalization
                        if let Ok(mut f) = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open("/tmp/rdos-keys.log")
                        {
                            use std::io::Write;
                            let _ = writeln!(
                                f,
                                "find_provider: canonicalized {:?} -> {:?}",
                                path, canonical_path
                            );
                        }
                        break;
                    }
                }
                if parent.as_os_str().is_empty() || parent == Path::new("/") {
                    break;
                }
                check_path = parent;
            }
            canonical_path
        };

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

        // Log the result
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/rdos-keys.log")
        {
            use std::io::Write;
            let mount_keys: Vec<_> = mounts.keys().collect();
            let _ = writeln!(
                f,
                "find_provider: path={:?}, canonical={:?}, mounts={:?}, found={:?}",
                path,
                canonical_path,
                mount_keys,
                best_match.map(|(p, _)| p)
            );
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
            // Check cache first for remote paths
            if let Some(cached) = self.get_cached(path) {
                return Ok(cached);
            }

            // Signal loading start for remote operations
            self.start_loading();
            let result = provider.read_dir(&relative_path);
            self.end_loading();

            let mut entries = result?;

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

            // Cache the result
            self.cache_dir(path, entries.clone());

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
        let result = self.route(path, |p, path| p.write_file(path, content));
        if result.is_ok() {
            self.invalidate_cache(path);
        }
        result
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
        let result = self.route(path, |p, path| p.create_dir(path));
        if result.is_ok() {
            self.invalidate_cache(path);
        }
        result
    }

    fn create_dir_all(&self, path: &Path) -> Result<()> {
        let result = self.route(path, |p, path| p.create_dir_all(path));
        if result.is_ok() {
            self.invalidate_cache(path);
        }
        result
    }

    fn remove_file(&self, path: &Path) -> Result<()> {
        let result = self.route(path, |p, path| p.remove_file(path));
        if result.is_ok() {
            self.invalidate_cache(path);
        }
        result
    }

    fn remove_dir(&self, path: &Path) -> Result<()> {
        let result = self.route(path, |p, path| p.remove_dir(path));
        if result.is_ok() {
            self.invalidate_cache(path);
        }
        result
    }

    fn remove_dir_all(&self, path: &Path) -> Result<()> {
        let result = self.route(path, |p, path| p.remove_dir_all(path));
        if result.is_ok() {
            self.invalidate_cache(path);
        }
        result
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        // Both paths must be in the same provider
        let from_provider = self.find_provider(from);
        let to_provider = self.find_provider(to);

        let result = match (from_provider, to_provider) {
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
        };

        if result.is_ok() {
            self.invalidate_cache(from);
            self.invalidate_cache(to);
        }
        result
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
