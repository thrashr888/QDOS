//! Google Drive Plugin State Types

use qdos_plugin_cloud::{StorageInfo, SyncStatus};
use std::path::PathBuf;

/// Google Drive sync status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GDriveSyncState {
    #[default]
    Unknown,
    /// File is available locally
    Available,
    /// File is streaming (virtual file)
    Streaming,
    /// File is syncing
    Syncing,
    /// Sync error
    Error,
}

impl From<GDriveSyncState> for SyncStatus {
    fn from(state: GDriveSyncState) -> Self {
        match state {
            GDriveSyncState::Unknown => SyncStatus::Unknown,
            GDriveSyncState::Available => SyncStatus::Synced,
            GDriveSyncState::Streaming => SyncStatus::CloudOnly,
            GDriveSyncState::Syncing => SyncStatus::Syncing,
            GDriveSyncState::Error => SyncStatus::Error,
        }
    }
}

/// File entry with Google Drive status
#[derive(Debug, Clone)]
pub struct GDriveFileEntry {
    pub name: String,
    pub path: PathBuf,
    pub sync_state: GDriveSyncState,
    pub size: Option<u64>,
    pub is_dir: bool,
    /// Whether this is a Google Docs/Sheets/Slides file
    pub is_google_doc: bool,
}

/// Google Drive view mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GDriveView {
    #[default]
    Browser,
    Info,
}

/// Google Drive plugin state
#[derive(Debug, Clone)]
pub struct GDriveState {
    pub view: GDriveView,
    pub current_dir: PathBuf,
    pub files: Vec<GDriveFileEntry>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub storage_info: StorageInfo,
    pub error: Option<String>,
    pub message: Option<String>,
    pub is_installed: bool,
    pub is_running: bool,
    /// Which Google Drive path variant is in use
    pub drive_variant: GDriveVariant,
}

/// Google Drive installation variant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GDriveVariant {
    #[default]
    None,
    /// Mounted at /Volumes/GoogleDrive
    VolumesMount,
    /// In home directory as ~/Google Drive
    HomeFolder,
    /// Google Drive Stream (older style)
    Stream,
}

impl Default for GDriveState {
    fn default() -> Self {
        Self::new()
    }
}

impl GDriveState {
    pub fn new() -> Self {
        Self {
            view: GDriveView::Browser,
            current_dir: PathBuf::new(), // Empty path - will be set when Google Drive is detected
            files: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            storage_info: StorageInfo::default(),
            error: None,
            message: None,
            is_installed: false,
            is_running: false,
            drive_variant: GDriveVariant::None,
        }
    }

    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn select_next(&mut self) {
        if !self.files.is_empty() && self.selected < self.files.len() - 1 {
            self.selected += 1;
        }
    }

    pub fn selected_file(&self) -> Option<&GDriveFileEntry> {
        self.files.get(self.selected)
    }
}
