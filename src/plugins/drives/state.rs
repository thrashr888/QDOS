//! Network Drives plugin state types
//!
//! State for the Drives modal showing mounted volumes.

use std::path::PathBuf;

/// Network share protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShareProtocol {
    Smb,
    Afp,
}

impl ShareProtocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            ShareProtocol::Smb => "SMB",
            ShareProtocol::Afp => "AFP",
        }
    }
}

/// A discovered but unmounted network share
#[derive(Debug, Clone)]
pub struct NetworkShare {
    pub name: String,
    pub hostname: String,
    pub protocol: ShareProtocol,
}

/// Volume type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeType {
    /// Local disk (boot drive, external drives)
    Local,
    /// Network share (SMB, AFP, NFS)
    Network,
    /// Cloud storage (Dropbox, iCloud, Google Drive)
    Cloud(CloudStorageType),
    /// Disk image (DMG)
    DiskImage,
    /// Time Machine backup
    TimeMachine,
    /// Unknown type
    Unknown,
}

/// Cloud storage provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudStorageType {
    Dropbox,
    ICloud,
    GoogleDrive,
    OneDrive,
}

impl VolumeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            VolumeType::Local => "Local",
            VolumeType::Network => "Network",
            VolumeType::Cloud(ct) => ct.as_str(),
            VolumeType::DiskImage => "Image",
            VolumeType::TimeMachine => "Time Machine",
            VolumeType::Unknown => "Unknown",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            VolumeType::Local => "HD",
            VolumeType::Network => "NET",
            VolumeType::Cloud(ct) => ct.icon(),
            VolumeType::DiskImage => "DMG",
            VolumeType::TimeMachine => "TM",
            VolumeType::Unknown => "?",
        }
    }
}

impl CloudStorageType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CloudStorageType::Dropbox => "Dropbox",
            CloudStorageType::ICloud => "iCloud",
            CloudStorageType::GoogleDrive => "Google Drive",
            CloudStorageType::OneDrive => "OneDrive",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            CloudStorageType::Dropbox => "DBX",
            CloudStorageType::ICloud => "iCL",
            CloudStorageType::GoogleDrive => "GDR",
            CloudStorageType::OneDrive => "OD",
        }
    }
}

/// A mounted volume/drive entry
#[derive(Debug, Clone)]
pub struct VolumeEntry {
    pub name: String,
    pub path: PathBuf,
    pub volume_type: VolumeType,
    pub mount_point: String,
    pub filesystem: String,
    /// Size in bytes (if available)
    pub total_size: Option<u64>,
    /// Free space in bytes (if available)
    pub free_space: Option<u64>,
    /// Whether volume is writable
    pub writable: bool,
}

impl VolumeEntry {
    /// Format size for display
    pub fn formatted_size(&self) -> String {
        match self.total_size {
            Some(size) => format_bytes(size),
            None => "?".to_string(),
        }
    }

    /// Format free space for display
    pub fn formatted_free(&self) -> String {
        match self.free_space {
            Some(size) => format_bytes(size),
            None => "?".to_string(),
        }
    }

    /// Get usage percentage (0-100)
    pub fn usage_percent(&self) -> Option<u8> {
        match (self.total_size, self.free_space) {
            (Some(total), Some(free)) if total > 0 => {
                let used = total.saturating_sub(free);
                Some(((used as f64 / total as f64) * 100.0) as u8)
            }
            _ => None,
        }
    }
}

/// Format bytes to human-readable string
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1}T", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1}G", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}M", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0}K", bytes as f64 / KB as f64)
    } else {
        format!("{}", bytes)
    }
}

/// Current view/section in the drives modal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DrivesSection {
    #[default]
    Volumes,
    NetworkShares,
}

/// Drives plugin state
#[derive(Debug, Clone, Default)]
pub struct DrivesState {
    pub volumes: Vec<VolumeEntry>,
    pub network_shares: Vec<NetworkShare>,
    pub selected_index: usize,
    pub section: DrivesSection,
    /// Path to navigate to after modal closes
    pub navigate_path: Option<PathBuf>,
    /// Share to mount after modal closes
    pub mount_share: Option<NetworkShare>,
}

impl DrivesState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the currently selected volume (only in Volumes section)
    pub fn selected_volume(&self) -> Option<&VolumeEntry> {
        if self.section == DrivesSection::Volumes {
            self.volumes.get(self.selected_index)
        } else {
            None
        }
    }

    /// Get the currently selected network share (only in NetworkShares section)
    pub fn selected_share(&self) -> Option<&NetworkShare> {
        if self.section == DrivesSection::NetworkShares {
            self.network_shares.get(self.selected_index)
        } else {
            None
        }
    }

    /// Get current list length based on section
    fn current_list_len(&self) -> usize {
        match self.section {
            DrivesSection::Volumes => self.volumes.len(),
            DrivesSection::NetworkShares => self.network_shares.len(),
        }
    }

    /// Move selection up
    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        let max = self.current_list_len().saturating_sub(1);
        if self.selected_index < max {
            self.selected_index += 1;
        }
    }

    /// Switch to next section
    pub fn next_section(&mut self) {
        self.section = match self.section {
            DrivesSection::Volumes => DrivesSection::NetworkShares,
            DrivesSection::NetworkShares => DrivesSection::Volumes,
        };
        self.selected_index = 0;
    }
}
