//! iCloud Drive Plugin State Types

use qdos_plugin_cloud::{StorageInfo, SyncStatus};
use std::path::PathBuf;

/// iCloud-specific sync status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ICloudSyncState {
    #[default]
    Unknown,
    /// File is downloaded and synced
    Downloaded,
    /// File is cloud-only (not downloaded)
    CloudOnly,
    /// File is downloading
    Downloading,
    /// File is uploading
    Uploading,
    /// File has sync error
    Error,
}

impl From<ICloudSyncState> for SyncStatus {
    fn from(state: ICloudSyncState) -> Self {
        match state {
            ICloudSyncState::Unknown => SyncStatus::Unknown,
            ICloudSyncState::Downloaded => SyncStatus::Synced,
            ICloudSyncState::CloudOnly => SyncStatus::CloudOnly,
            ICloudSyncState::Downloading => SyncStatus::Syncing,
            ICloudSyncState::Uploading => SyncStatus::Syncing,
            ICloudSyncState::Error => SyncStatus::Error,
        }
    }
}

/// File entry with iCloud status
#[derive(Debug, Clone)]
pub struct ICloudFileEntry {
    pub name: String,
    pub path: PathBuf,
    pub sync_state: ICloudSyncState,
    pub size: Option<u64>,
    pub is_dir: bool,
    /// Original name if this is a .icloud placeholder
    pub original_name: Option<String>,
}

/// iCloud view mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ICloudView {
    #[default]
    Browser,
    Info,
}

/// iCloud filter mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ICloudFilter {
    #[default]
    All,
    CloudOnly,
    Downloaded,
    Syncing,
}

impl ICloudFilter {
    pub fn as_str(&self) -> &'static str {
        match self {
            ICloudFilter::All => "All Files",
            ICloudFilter::CloudOnly => "Cloud Only",
            ICloudFilter::Downloaded => "Downloaded",
            ICloudFilter::Syncing => "Syncing",
        }
    }

    pub fn matches(&self, state: ICloudSyncState) -> bool {
        match self {
            ICloudFilter::All => true,
            ICloudFilter::CloudOnly => state == ICloudSyncState::CloudOnly,
            ICloudFilter::Downloaded => state == ICloudSyncState::Downloaded,
            ICloudFilter::Syncing => {
                state == ICloudSyncState::Downloading || state == ICloudSyncState::Uploading
            }
        }
    }
}

/// iCloud plugin state
#[derive(Debug, Clone)]
pub struct ICloudState {
    pub view: ICloudView,
    pub current_dir: PathBuf,
    pub files: Vec<ICloudFileEntry>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub filter: ICloudFilter,
    pub storage_info: StorageInfo,
    pub error: Option<String>,
    pub message: Option<String>,
    pub is_available: bool,
}

impl Default for ICloudState {
    fn default() -> Self {
        Self::new()
    }
}

impl ICloudState {
    pub fn new() -> Self {
        // Don't default to "/" - let open_modal() set the correct path
        let icloud_path = dirs::home_dir()
            .map(|h| h.join("Library/Mobile Documents/com~apple~CloudDocs"))
            .unwrap_or_default();

        Self {
            view: ICloudView::Browser,
            current_dir: icloud_path,
            files: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            filter: ICloudFilter::All,
            storage_info: StorageInfo::default(),
            error: None,
            message: None,
            is_available: false,
        }
    }

    pub fn filtered_files(&self) -> Vec<&ICloudFileEntry> {
        self.files
            .iter()
            .filter(|f| self.filter.matches(f.sync_state))
            .collect()
    }

    pub fn select_prev(&mut self) {
        let len = self.filtered_files().len();
        if len > 0 && self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn select_next(&mut self) {
        let len = self.filtered_files().len();
        if len > 0 && self.selected < len - 1 {
            self.selected += 1;
        }
    }

    pub fn next_filter(&mut self) {
        self.filter = match self.filter {
            ICloudFilter::All => ICloudFilter::CloudOnly,
            ICloudFilter::CloudOnly => ICloudFilter::Downloaded,
            ICloudFilter::Downloaded => ICloudFilter::Syncing,
            ICloudFilter::Syncing => ICloudFilter::All,
        };
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn selected_file(&self) -> Option<&ICloudFileEntry> {
        self.filtered_files().get(self.selected).copied()
    }
}
