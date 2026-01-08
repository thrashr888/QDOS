//! Dropbox Plugin State Types

use crate::plugins::cloud::{StorageInfo, SyncStatus};
use std::path::PathBuf;

/// Dropbox-specific sync status (maps to SyncStatus)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DropboxSyncState {
    #[default]
    Unknown,
    /// File is fully synced
    UpToDate,
    /// File is syncing
    Syncing,
    /// File has unsyncable error
    Unsyncable,
    /// File is in selective sync (not downloaded)
    SelectiveSync,
}

impl From<DropboxSyncState> for SyncStatus {
    fn from(state: DropboxSyncState) -> Self {
        match state {
            DropboxSyncState::Unknown => SyncStatus::Unknown,
            DropboxSyncState::UpToDate => SyncStatus::Synced,
            DropboxSyncState::Syncing => SyncStatus::Syncing,
            DropboxSyncState::Unsyncable => SyncStatus::Error,
            DropboxSyncState::SelectiveSync => SyncStatus::Excluded,
        }
    }
}

/// File entry with Dropbox status
#[derive(Debug, Clone)]
pub struct DropboxFileEntry {
    pub name: String,
    pub path: PathBuf,
    pub sync_state: DropboxSyncState,
    pub size: Option<u64>,
    pub is_dir: bool,
    pub is_shared: bool,
}

/// Dropbox view mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DropboxView {
    #[default]
    Browser,
    Info,
    Filter,
}

/// Dropbox filter mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DropboxFilter {
    #[default]
    All,
    Syncing,
    Errors,
    Excluded,
}

impl DropboxFilter {
    pub fn as_str(&self) -> &'static str {
        match self {
            DropboxFilter::All => "All Files",
            DropboxFilter::Syncing => "Syncing",
            DropboxFilter::Errors => "Errors Only",
            DropboxFilter::Excluded => "Excluded",
        }
    }

    pub fn matches(&self, state: DropboxSyncState) -> bool {
        match self {
            DropboxFilter::All => true,
            DropboxFilter::Syncing => state == DropboxSyncState::Syncing,
            DropboxFilter::Errors => state == DropboxSyncState::Unsyncable,
            DropboxFilter::Excluded => state == DropboxSyncState::SelectiveSync,
        }
    }
}

/// Dropbox plugin state
#[derive(Debug, Clone)]
pub struct DropboxState {
    /// Current view
    pub view: DropboxView,
    /// Current directory being browsed
    pub current_dir: PathBuf,
    /// Files in current directory
    pub files: Vec<DropboxFileEntry>,
    /// Selected file index
    pub selected: usize,
    /// Scroll offset for file list
    pub scroll_offset: usize,
    /// Current filter
    pub filter: DropboxFilter,
    /// Storage info
    pub storage_info: StorageInfo,
    /// Error message if any
    pub error: Option<String>,
    /// Success message if any
    pub message: Option<String>,
    /// Whether Dropbox is installed
    pub is_installed: bool,
    /// Whether Dropbox is running
    pub is_running: bool,
}

impl Default for DropboxState {
    fn default() -> Self {
        Self::new()
    }
}

impl DropboxState {
    pub fn new() -> Self {
        let dropbox_path = dirs::home_dir()
            .map(|h| h.join("Dropbox"))
            .unwrap_or_else(|| PathBuf::from("/"));

        Self {
            view: DropboxView::Browser,
            current_dir: dropbox_path,
            files: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            filter: DropboxFilter::All,
            storage_info: StorageInfo::default(),
            error: None,
            message: None,
            is_installed: false,
            is_running: false,
        }
    }

    /// Get filtered files
    pub fn filtered_files(&self) -> Vec<&DropboxFileEntry> {
        self.files
            .iter()
            .filter(|f| self.filter.matches(f.sync_state))
            .collect()
    }

    /// Select previous file
    pub fn select_prev(&mut self) {
        let len = self.filtered_files().len();
        if len > 0 && self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Select next file
    pub fn select_next(&mut self) {
        let len = self.filtered_files().len();
        if len > 0 && self.selected < len - 1 {
            self.selected += 1;
        }
    }

    /// Cycle through filters
    pub fn next_filter(&mut self) {
        self.filter = match self.filter {
            DropboxFilter::All => DropboxFilter::Syncing,
            DropboxFilter::Syncing => DropboxFilter::Errors,
            DropboxFilter::Errors => DropboxFilter::Excluded,
            DropboxFilter::Excluded => DropboxFilter::All,
        };
        self.selected = 0;
        self.scroll_offset = 0;
    }

    /// Get selected file
    pub fn selected_file(&self) -> Option<&DropboxFileEntry> {
        self.filtered_files().get(self.selected).copied()
    }
}
