//! Cloud Storage Plugin Foundation
//!
//! Shared infrastructure for cloud storage plugins (Dropbox, iCloud, Google Drive).
//! This module provides:
//! - CloudStoragePlugin trait extending the Plugin trait
//! - SyncStatus enum for file sync states
//! - Shared UI components for status overlay and storage info
//! - Integration hooks for the Chg Drive modal

#![allow(clippy::ptr_arg)]

pub mod state;
pub mod ui;

// Re-export types for use by cloud storage plugin implementations
#[allow(unused_imports)]
pub use state::*;

use qdos_plugin_api::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;

/// Extended plugin trait for cloud storage integrations
///
/// Cloud storage plugins implement this trait in addition to the base Plugin trait
/// to provide sync status information and cloud-specific operations.
pub trait CloudStoragePlugin: Plugin {
    /// Get the cloud provider type
    fn provider(&self) -> CloudProvider;

    /// Get the root path for this cloud storage
    fn root_path(&self) -> Option<PathBuf>;

    /// Check if a path is within this cloud storage
    fn contains_path(&self, path: &PathBuf) -> bool {
        if let Some(root) = self.root_path() {
            path.starts_with(&root)
        } else {
            false
        }
    }

    /// Get sync status for a specific file
    fn get_file_status(&self, path: &PathBuf) -> SyncStatus;

    /// Get sync status for multiple files (batch operation for efficiency)
    fn get_batch_status(&self, paths: &[PathBuf]) -> HashMap<PathBuf, SyncStatus> {
        paths
            .iter()
            .map(|p| (p.clone(), self.get_file_status(p)))
            .collect()
    }

    /// Get storage usage information
    fn get_storage_info(&self) -> StorageInfo;

    /// Trigger download for a cloud-only file
    fn download_file(&mut self, path: &PathBuf) -> Result<(), String>;

    /// Get a shareable link for a file
    fn get_share_link(&self, path: &PathBuf) -> Result<String, String>;

    /// Open the file in the web interface
    fn open_in_browser(&self, path: &PathBuf) -> Result<(), String>;

    /// Force sync a specific file
    fn force_sync(&mut self, path: &PathBuf) -> Result<(), String>;

    /// Check if the service is available/installed
    fn is_service_installed(&self) -> bool;

    /// Check if the service is currently running
    fn is_service_running(&self) -> bool;

    /// Refresh cached status information
    fn refresh(&mut self);
}

/// Registry for cloud storage plugins
///
/// This allows the drives plugin to discover and interact with cloud storage plugins.
pub struct CloudStorageRegistry {
    plugins: Vec<CloudProvider>,
}

impl CloudStorageRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    pub fn register(&mut self, provider: CloudProvider) {
        if !self.plugins.contains(&provider) {
            self.plugins.push(provider);
        }
    }

    pub fn providers(&self) -> &[CloudProvider] {
        &self.plugins
    }

    /// Find which cloud provider (if any) contains a given path
    pub fn find_provider_for_path(&self, path: &PathBuf) -> Option<CloudProvider> {
        for provider in &self.plugins {
            if let Some(root) = provider.default_path() {
                if path.starts_with(&root) {
                    return Some(*provider);
                }
            }
        }
        None
    }
}

impl Default for CloudStorageRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper trait for plugins that want to show per-file cloud status in the file table
pub trait FileStatusOverlay {
    /// Get status indicator character for a file (for display in file table)
    fn get_status_char(&self, path: &PathBuf) -> Option<char>;

    /// Get status color for a file
    fn get_status_color(&self, path: &PathBuf) -> Option<ratatui::style::Color>;
}

/// Base implementation helper for cloud storage plugins
///
/// This provides common functionality that cloud storage plugins can use.
pub struct CloudStorageBase {
    /// Cached file statuses
    pub status_cache: HashMap<PathBuf, SyncStatus>,
    /// Cached storage info
    pub storage_info: StorageInfo,
    /// Provider type
    pub provider: CloudProvider,
    /// Root path
    pub root_path: Option<PathBuf>,
    /// Last refresh timestamp
    pub last_refresh: std::time::Instant,
    /// Cache TTL in seconds
    pub cache_ttl: u64,
}

impl CloudStorageBase {
    pub fn new(provider: CloudProvider) -> Self {
        Self {
            status_cache: HashMap::new(),
            storage_info: StorageInfo::default(),
            provider,
            root_path: provider.default_path(),
            last_refresh: std::time::Instant::now(),
            cache_ttl: 30, // Default 30 second cache
        }
    }

    /// Check if the cache is stale and needs refresh
    pub fn is_cache_stale(&self) -> bool {
        self.last_refresh.elapsed().as_secs() > self.cache_ttl
    }

    /// Update the cache timestamp
    pub fn mark_refreshed(&mut self) {
        self.last_refresh = std::time::Instant::now();
    }

    /// Clear all cached data
    pub fn clear_cache(&mut self) {
        self.status_cache.clear();
    }

    /// Get cached status or return Unknown
    pub fn get_cached_status(&self, path: &PathBuf) -> SyncStatus {
        self.status_cache
            .get(path)
            .copied()
            .unwrap_or(SyncStatus::Unknown)
    }

    /// Update status in cache
    pub fn set_status(&mut self, path: PathBuf, status: SyncStatus) {
        self.status_cache.insert(path, status);
    }
}

/// Stub plugin implementation for when a cloud service is not installed
///
/// This can be used to show the service in the drives list but indicate it's not available.
pub struct CloudStorageStub {
    provider: CloudProvider,
    capabilities: PluginCapabilities,
}

impl CloudStorageStub {
    pub fn new(provider: CloudProvider) -> Self {
        Self {
            provider,
            capabilities: PluginCapabilities {
                has_menu: false,
                has_keys: false,
                has_modal: false,
                has_status: true,
                has_cli: false,
                has_help: true,
            },
        }
    }
}

impl Plugin for CloudStorageStub {
    fn id(&self) -> &str {
        match self.provider {
            CloudProvider::Dropbox => "dropbox_stub",
            CloudProvider::ICloud => "icloud_stub",
            CloudProvider::GoogleDrive => "gdrive_stub",
            CloudProvider::OneDrive => "onedrive_stub",
        }
    }

    fn name(&self) -> &str {
        self.provider.as_str()
    }

    fn capabilities(&self) -> PluginCapabilities {
        self.capabilities
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        false // Stub is never available
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            format!(
                "        {} -- NOT INSTALLED",
                self.provider.as_str().to_uppercase()
            ),
            "".to_string(),
            format!(
                "{} is not installed or configured on this system.",
                self.provider.as_str()
            ),
            "".to_string(),
            "Install the desktop application to enable cloud storage features.".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloud_storage_base() {
        let base = CloudStorageBase::new(CloudProvider::Dropbox);
        assert_eq!(base.provider, CloudProvider::Dropbox);
        assert!(!base.is_cache_stale()); // Fresh cache
    }

    #[test]
    fn test_cloud_storage_registry() {
        let mut registry = CloudStorageRegistry::new();
        registry.register(CloudProvider::Dropbox);
        registry.register(CloudProvider::ICloud);
        registry.register(CloudProvider::Dropbox); // Duplicate

        assert_eq!(registry.providers().len(), 2);
    }

    #[test]
    fn test_cloud_provider_paths() {
        // Just verify it doesn't panic
        for provider in [
            CloudProvider::Dropbox,
            CloudProvider::ICloud,
            CloudProvider::GoogleDrive,
            CloudProvider::OneDrive,
        ] {
            let _ = provider.default_path();
            let _ = provider.as_str();
        }
    }
}
