//! Cloud Storage Plugin State Types
//!
//! Shared state types for cloud storage plugins (Dropbox, iCloud, Google Drive).

use std::path::PathBuf;

/// Sync status for a file in cloud storage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SyncStatus {
    /// File is fully synced with cloud
    #[default]
    Synced,
    /// File is currently syncing (uploading or downloading)
    Syncing,
    /// File has local changes pending upload
    Pending,
    /// Sync error occurred
    Error,
    /// File exists only in cloud, not downloaded locally
    CloudOnly,
    /// File is available offline (downloaded)
    Offline,
    /// File is excluded from sync (selective sync)
    Excluded,
    /// Status unknown or not applicable
    Unknown,
}

impl SyncStatus {
    /// Get a short display string for the status
    pub fn as_str(&self) -> &'static str {
        match self {
            SyncStatus::Synced => "Synced",
            SyncStatus::Syncing => "Syncing",
            SyncStatus::Pending => "Pending",
            SyncStatus::Error => "Error",
            SyncStatus::CloudOnly => "Cloud",
            SyncStatus::Offline => "Offline",
            SyncStatus::Excluded => "Excluded",
            SyncStatus::Unknown => "Unknown",
        }
    }

    /// Get a single character indicator for the status
    pub fn indicator(&self) -> char {
        match self {
            SyncStatus::Synced => '✓',
            SyncStatus::Syncing => '↻',
            SyncStatus::Pending => '↑',
            SyncStatus::Error => '✗',
            SyncStatus::CloudOnly => '☁',
            SyncStatus::Offline => '●',
            SyncStatus::Excluded => '○',
            SyncStatus::Unknown => '?',
        }
    }

    /// Get ASCII-safe indicator for the status (for DOS-style display)
    pub fn ascii_indicator(&self) -> char {
        match self {
            SyncStatus::Synced => '*',
            SyncStatus::Syncing => '~',
            SyncStatus::Pending => '^',
            SyncStatus::Error => '!',
            SyncStatus::CloudOnly => 'C',
            SyncStatus::Offline => 'O',
            SyncStatus::Excluded => '-',
            SyncStatus::Unknown => '?',
        }
    }
}

/// Storage usage information for a cloud service
#[derive(Debug, Clone, Default)]
pub struct StorageInfo {
    /// Total storage capacity in bytes
    pub total_bytes: Option<u64>,
    /// Used storage in bytes
    pub used_bytes: Option<u64>,
    /// Account email or identifier
    pub account: Option<String>,
    /// Whether the service is connected/authenticated
    pub connected: bool,
}

impl StorageInfo {
    /// Get free space in bytes
    pub fn free_bytes(&self) -> Option<u64> {
        match (self.total_bytes, self.used_bytes) {
            (Some(total), Some(used)) => Some(total.saturating_sub(used)),
            _ => None,
        }
    }

    /// Get usage percentage (0-100)
    pub fn usage_percent(&self) -> Option<u8> {
        match (self.total_bytes, self.used_bytes) {
            (Some(total), Some(used)) if total > 0 => {
                Some(((used as f64 / total as f64) * 100.0) as u8)
            }
            _ => None,
        }
    }

    /// Format bytes as human-readable string
    pub fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;
        const TB: u64 = GB * 1024;

        if bytes >= TB {
            format!("{:.1} TB", bytes as f64 / TB as f64)
        } else if bytes >= GB {
            format!("{:.1} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.1} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.1} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }
}

/// File entry with sync status for cloud storage display
#[derive(Debug, Clone)]
pub struct CloudFileEntry {
    /// File name
    pub name: String,
    /// Full path
    pub path: PathBuf,
    /// Sync status
    pub status: SyncStatus,
    /// File size in bytes (if known)
    pub size: Option<u64>,
    /// Whether this is a directory
    pub is_dir: bool,
    /// Whether the file is shared
    pub is_shared: bool,
}

/// Cloud service provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudProvider {
    Dropbox,
    ICloud,
    GoogleDrive,
    OneDrive,
}

impl CloudProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            CloudProvider::Dropbox => "Dropbox",
            CloudProvider::ICloud => "iCloud Drive",
            CloudProvider::GoogleDrive => "Google Drive",
            CloudProvider::OneDrive => "OneDrive",
        }
    }

    /// Get the typical root path for this provider on macOS
    #[cfg(target_os = "macos")]
    pub fn default_path(&self) -> Option<PathBuf> {
        let home = dirs::home_dir()?;
        match self {
            CloudProvider::Dropbox => Some(home.join("Dropbox")),
            CloudProvider::ICloud => {
                Some(home.join("Library/Mobile Documents/com~apple~CloudDocs"))
            }
            CloudProvider::GoogleDrive => {
                // Google Drive for Desktop mounts at /Volumes/GoogleDrive or ~/Google Drive
                let volumes_path = PathBuf::from("/Volumes/GoogleDrive");
                if volumes_path.exists() {
                    Some(volumes_path)
                } else {
                    Some(home.join("Google Drive"))
                }
            }
            CloudProvider::OneDrive => Some(home.join("OneDrive")),
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn default_path(&self) -> Option<PathBuf> {
        let home = dirs::home_dir()?;
        match self {
            CloudProvider::Dropbox => Some(home.join("Dropbox")),
            CloudProvider::ICloud => None, // iCloud not available on non-macOS
            CloudProvider::GoogleDrive => Some(home.join("Google Drive")),
            CloudProvider::OneDrive => Some(home.join("OneDrive")),
        }
    }
}

/// Quick action available for cloud files
#[derive(Debug, Clone)]
pub struct CloudQuickAction {
    /// Action identifier
    pub id: &'static str,
    /// Display name
    pub name: &'static str,
    /// Keyboard shortcut
    pub key: char,
    /// Description
    pub description: &'static str,
}

/// Common quick actions for cloud storage
pub const CLOUD_QUICK_ACTIONS: &[CloudQuickAction] = &[
    CloudQuickAction {
        id: "share",
        name: "Share",
        key: 'S',
        description: "Get shareable link",
    },
    CloudQuickAction {
        id: "download",
        name: "Download",
        key: 'D',
        description: "Download cloud-only file",
    },
    CloudQuickAction {
        id: "open_web",
        name: "Open in Browser",
        key: 'W',
        description: "Open in web interface",
    },
    CloudQuickAction {
        id: "sync",
        name: "Force Sync",
        key: 'Y',
        description: "Force sync this file",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_status_indicators() {
        assert_eq!(SyncStatus::Synced.indicator(), '✓');
        assert_eq!(SyncStatus::Syncing.indicator(), '↻');
        assert_eq!(SyncStatus::Error.indicator(), '✗');
    }

    #[test]
    fn test_storage_info_calculations() {
        let info = StorageInfo {
            total_bytes: Some(100 * 1024 * 1024 * 1024), // 100 GB
            used_bytes: Some(75 * 1024 * 1024 * 1024),   // 75 GB
            account: Some("test@example.com".to_string()),
            connected: true,
        };

        assert_eq!(info.usage_percent(), Some(75));
        assert_eq!(
            info.free_bytes(),
            Some(25 * 1024 * 1024 * 1024) // 25 GB
        );
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(StorageInfo::format_bytes(500), "500 B");
        assert_eq!(StorageInfo::format_bytes(1024), "1.0 KB");
        assert_eq!(StorageInfo::format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(StorageInfo::format_bytes(1024 * 1024 * 1024), "1.0 GB");
    }
}
