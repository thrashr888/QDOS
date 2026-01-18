//! iCloud Drive Plugin (macOS only)
//!
//! Provides iCloud Drive integration with cloud-only file handling.

#![allow(dead_code)]
#![allow(clippy::ptr_arg)]

mod modal;
pub mod ops;
pub mod state;

use crossterm::event::{KeyCode, KeyEvent};
use qdos_plugin_api::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, PluginMenuItem,
    PluginStatusInfo, ThemeColors,
};
use qdos_plugin_cloud::{CloudProvider, CloudStoragePlugin, StorageInfo, SyncStatus};
use ratatui::{layout::Rect, Frame};
use state::{ICloudState, ICloudView};
use std::any::Any;
use std::collections::HashMap;
use std::path::PathBuf;

/// iCloud Drive plugin
pub struct ICloudPlugin {
    pub state: ICloudState,
    modal_open: bool,
}

impl Default for ICloudPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl ICloudPlugin {
    pub fn new() -> Self {
        let mut state = ICloudState::new();
        state.is_available = ops::is_icloud_available();

        Self {
            state,
            modal_open: false,
        }
    }

    pub fn open_modal(&mut self) {
        self.state.is_available = ops::is_icloud_available();
        self.state.storage_info = ops::get_storage_info();
        self.state.view = ICloudView::Browser;
        self.state.error = None;
        self.state.message = None;

        if let Some(icloud_path) = ops::get_icloud_path() {
            self.state.current_dir = icloud_path.clone();
            ops::load_directory(&mut self.state, &icloud_path);
        } else {
            // iCloud not available - show error
            self.state.files.clear();
            self.state.current_dir = PathBuf::new();
            self.state.error = Some("iCloud Drive is not available".to_string());
        }

        self.modal_open = true;
    }

    pub fn close_modal(&mut self) {
        self.modal_open = false;
    }

    fn enter_directory(&mut self) {
        // Get iCloud root to validate navigation
        let icloud_root = match ops::get_icloud_path() {
            Some(root) => root,
            None => {
                self.state.error = Some("iCloud Drive is not available".to_string());
                return;
            }
        };

        if let Some(file) = self.state.selected_file() {
            if file.is_dir {
                let path = file.path.clone();
                // Validate that the target path is within iCloud
                if !path.starts_with(&icloud_root) {
                    self.state.error = Some("Cannot navigate outside iCloud Drive".to_string());
                    return;
                }
                ops::load_directory(&mut self.state, &path);
            }
        }
    }

    fn go_up(&mut self) {
        // Get iCloud root to validate navigation
        let icloud_root = match ops::get_icloud_path() {
            Some(root) => root,
            None => {
                self.state.error = Some("iCloud Drive is not available".to_string());
                return;
            }
        };

        if let Some(parent) = self.state.current_dir.parent() {
            // Don't go above iCloud root
            if self.state.current_dir != icloud_root {
                let parent_path = parent.to_path_buf();
                // Validate parent is still within iCloud
                if parent_path.starts_with(&icloud_root) || parent_path == icloud_root {
                    ops::load_directory(&mut self.state, &parent_path);
                }
            }
        }
    }

    fn download_selected(&mut self) {
        if let Some(file) = self.state.selected_file() {
            if file.sync_state == state::ICloudSyncState::CloudOnly {
                let path = file.path.clone();
                match ops::download_file(&path) {
                    Ok(()) => {
                        self.state.message = Some("Download started...".to_string());
                        // Refresh to update status
                        let dir = self.state.current_dir.clone();
                        ops::load_directory(&mut self.state, &dir);
                    }
                    Err(e) => {
                        self.state.error = Some(e);
                    }
                }
            } else {
                self.state.message = Some("File is already downloaded".to_string());
            }
        }
    }

    fn evict_selected(&mut self) {
        if let Some(file) = self.state.selected_file() {
            if file.sync_state == state::ICloudSyncState::Downloaded {
                let path = file.path.clone();
                match ops::evict_file(&path) {
                    Ok(()) => {
                        self.state.message = Some("File evicted (cloud-only)".to_string());
                        let dir = self.state.current_dir.clone();
                        ops::load_directory(&mut self.state, &dir);
                    }
                    Err(e) => {
                        self.state.error = Some(e);
                    }
                }
            } else {
                self.state.message = Some("File is not downloaded".to_string());
            }
        }
    }
}

impl Plugin for ICloudPlugin {
    fn id(&self) -> &str {
        "icloud"
    }

    fn name(&self) -> &str {
        "iCloud Drive"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: true,
            has_keys: true,
            has_modal: true,
            has_status: true,
            has_cli: false,
            has_help: true,
        }
    }

    fn is_available(&self, cwd: &PathBuf) -> bool {
        if !ops::is_icloud_available() {
            return false;
        }

        if let Some(icloud_path) = ops::get_icloud_path() {
            cwd.starts_with(&icloud_path) || icloud_path.exists()
        } else {
            false
        }
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "iCloud".to_string(),
            key: 'I',
            description: "Browse iCloud Drive with sync status".to_string(),
            priority: 26,
        })
    }

    fn status_info(&self, cwd: &PathBuf) -> Option<PluginStatusInfo> {
        if let Some(icloud_path) = ops::get_icloud_path() {
            if cwd.starts_with(&icloud_path) {
                return Some(PluginStatusInfo {
                    text: "iCloud ●".to_string(),
                    active: true,
                });
            }
        }
        None
    }

    fn handle_global_key(
        &mut self,
        key: KeyEvent,
        _cwd: &PathBuf,
        _selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
        match key.code {
            KeyCode::Char('i') | KeyCode::Char('I') => {
                if ops::is_icloud_available() {
                    self.open_modal();
                    KeyHandleResult::OpenModal
                } else {
                    KeyHandleResult::NotHandled
                }
            }
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        if self.state.error.is_some() || self.state.message.is_some() {
            self.state.error = None;
            self.state.message = None;
            return KeyHandleResult::Handled;
        }

        match self.state.view {
            ICloudView::Browser => match key.code {
                KeyCode::Esc => {
                    self.close_modal();
                    KeyHandleResult::CloseModal
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.state.select_prev();
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.state.select_next();
                    KeyHandleResult::Handled
                }
                KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                    self.enter_directory();
                    KeyHandleResult::Handled
                }
                KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => {
                    self.go_up();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('f') | KeyCode::Char('F') => {
                    self.state.next_filter();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('i') => {
                    // Refresh status when opening info view
                    self.state.is_available = ops::is_icloud_available();
                    self.state.storage_info = ops::get_storage_info();
                    self.state.view = ICloudView::Info;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    self.download_selected();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('e') | KeyCode::Char('E') => {
                    self.evict_selected();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    // Re-detect iCloud on refresh
                    self.state.is_available = ops::is_icloud_available();
                    self.state.storage_info = ops::get_storage_info();

                    if let Some(icloud_root) = ops::get_icloud_path() {
                        // If current_dir is valid and within iCloud, refresh it
                        // Otherwise, reset to iCloud root
                        let dir = if self.state.current_dir.starts_with(&icloud_root) {
                            self.state.current_dir.clone()
                        } else {
                            self.state.current_dir = icloud_root.clone();
                            icloud_root
                        };
                        self.state.error = None;
                        ops::load_directory(&mut self.state, &dir);
                    } else {
                        // iCloud not available
                        self.state.files.clear();
                        self.state.current_dir = PathBuf::new();
                        self.state.error = Some("iCloud Drive is not available".to_string());
                    }
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            ICloudView::Info => match key.code {
                KeyCode::Esc | KeyCode::Char('i') | KeyCode::Char('I') => {
                    self.state.view = ICloudView::Browser;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        modal::draw_icloud_modal(frame, area, &self.state, colors);
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "        I -- ICLOUD DRIVE".to_string(),
            "".to_string(),
            "Purpose:   Browse iCloud Drive with cloud-only file handling.".to_string(),
            "           Download files on demand, evict to free space.".to_string(),
            "".to_string(),
            "To use:    Press I to open iCloud Drive browser.".to_string(),
            "           Only available on macOS with iCloud signed in.".to_string(),
            "".to_string(),
            "Navigation:".to_string(),
            "  ↑/↓       Navigate files".to_string(),
            "  Enter     Open directory".to_string(),
            "  Backspace Go to parent directory".to_string(),
            "  F         Cycle through filters".to_string(),
            "  I         Show iCloud info".to_string(),
            "  D         Download cloud-only file".to_string(),
            "  E         Evict file (remove local copy)".to_string(),
            "  R         Refresh".to_string(),
            "  Esc       Close".to_string(),
            "".to_string(),
            "Status indicators:".to_string(),
            "  *  Downloaded    C  Cloud-only".to_string(),
            "  ~  Syncing       !  Error".to_string(),
        ]
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "iCloud Drive".to_string(),
            description: "Browse iCloud with cloud-only file support".to_string(),
            category: PluginCategory::Files,
            key: 'I',
        })
    }

    fn launch(&mut self, _cwd: &PathBuf, _selected_file: Option<&PathBuf>) -> Result<(), String> {
        self.open_modal();
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl CloudStoragePlugin for ICloudPlugin {
    fn provider(&self) -> CloudProvider {
        CloudProvider::ICloud
    }

    fn root_path(&self) -> Option<PathBuf> {
        ops::get_icloud_path()
    }

    fn get_file_status(&self, path: &PathBuf) -> SyncStatus {
        ops::get_file_sync_status(path).into()
    }

    fn get_batch_status(&self, paths: &[PathBuf]) -> HashMap<PathBuf, SyncStatus> {
        paths
            .iter()
            .map(|p| (p.clone(), self.get_file_status(p)))
            .collect()
    }

    fn get_storage_info(&self) -> StorageInfo {
        ops::get_storage_info()
    }

    fn download_file(&mut self, path: &PathBuf) -> Result<(), String> {
        ops::download_file(path)
    }

    fn get_share_link(&self, _path: &PathBuf) -> Result<String, String> {
        Err("iCloud sharing requires iCloud.com".to_string())
    }

    fn open_in_browser(&self, _path: &PathBuf) -> Result<(), String> {
        // Open iCloud.com
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg("https://www.icloud.com/iclouddrive")
                .spawn()
                .map_err(|e| format!("Failed to open browser: {}", e))?;
            Ok(())
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err("iCloud web not supported".to_string())
        }
    }

    fn force_sync(&mut self, _path: &PathBuf) -> Result<(), String> {
        Err("iCloud syncs automatically".to_string())
    }

    fn is_service_installed(&self) -> bool {
        ops::is_icloud_available()
    }

    fn is_service_running(&self) -> bool {
        ops::is_icloud_available()
    }

    fn refresh(&mut self) {
        self.state.is_available = ops::is_icloud_available();
        self.state.storage_info = ops::get_storage_info();
        let dir = self.state.current_dir.clone();
        ops::load_directory(&mut self.state, &dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icloud_plugin_creation() {
        let plugin = ICloudPlugin::new();
        assert_eq!(plugin.id(), "icloud");
        assert_eq!(plugin.name(), "iCloud Drive");
    }
}
