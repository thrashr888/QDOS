//! Dropbox Plugin
//!
//! Provides Dropbox integration with per-file sync status display.

mod modal;
pub mod ops;
pub mod state;

use super::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, PluginMenuItem,
    PluginStatusInfo,
};
use crate::app::ThemeColors;
use crate::plugins::cloud::{CloudProvider, CloudStoragePlugin, StorageInfo, SyncStatus};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, Frame};
use state::{DropboxState, DropboxView};
use std::any::Any;
use std::collections::HashMap;
use std::path::PathBuf;

/// Dropbox plugin
pub struct DropboxPlugin {
    /// Plugin state
    pub state: DropboxState,
    /// Whether the modal is open
    modal_open: bool,
}

impl Default for DropboxPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl DropboxPlugin {
    pub fn new() -> Self {
        let mut state = DropboxState::new();
        state.is_installed = ops::is_dropbox_installed();
        state.is_running = ops::is_dropbox_running();

        Self {
            state,
            modal_open: false,
        }
    }

    /// Open the Dropbox modal
    pub fn open_modal(&mut self) {
        self.state.is_installed = ops::is_dropbox_installed();
        self.state.is_running = ops::is_dropbox_running();
        self.state.storage_info = ops::get_storage_info();
        self.state.view = DropboxView::Browser;
        self.state.error = None;
        self.state.message = None;

        // Load initial directory
        if let Some(dropbox_path) = ops::get_dropbox_path() {
            self.state.current_dir = dropbox_path.clone();
            ops::load_directory(&mut self.state, &dropbox_path);
        } else {
            // Dropbox not available - show error
            self.state.files.clear();
            self.state.current_dir = PathBuf::new();
            if !self.state.is_installed {
                self.state.error = Some("Dropbox is not installed".to_string());
            } else if !self.state.is_running {
                self.state.error =
                    Some("Dropbox is not running. Please start Dropbox.".to_string());
            } else {
                self.state.error = Some("Dropbox folder not found".to_string());
            }
        }

        self.modal_open = true;
    }

    /// Close the modal
    pub fn close_modal(&mut self) {
        self.modal_open = false;
    }

    /// Navigate into selected directory
    fn enter_directory(&mut self) {
        // Get Dropbox root to validate navigation
        let dropbox_root = match ops::get_dropbox_path() {
            Some(root) => root,
            None => {
                self.state.error =
                    Some("Dropbox is not available. Please start Dropbox.".to_string());
                return;
            }
        };

        if let Some(file) = self.state.selected_file() {
            if file.is_dir {
                let path = file.path.clone();
                // Validate that the target path is within Dropbox
                if !path.starts_with(&dropbox_root) {
                    self.state.error = Some("Cannot navigate outside Dropbox".to_string());
                    return;
                }
                ops::load_directory(&mut self.state, &path);
            }
        }
    }

    /// Navigate to parent directory
    fn go_up(&mut self) {
        // Get Dropbox root to validate navigation
        let dropbox_root = match ops::get_dropbox_path() {
            Some(root) => root,
            None => {
                self.state.error =
                    Some("Dropbox is not available. Please start Dropbox.".to_string());
                return;
            }
        };

        if let Some(parent) = self.state.current_dir.parent() {
            // Don't go above Dropbox root
            if self.state.current_dir != dropbox_root {
                let parent_path = parent.to_path_buf();
                // Validate parent is still within Dropbox
                if parent_path.starts_with(&dropbox_root) || parent_path == dropbox_root {
                    ops::load_directory(&mut self.state, &parent_path);
                }
            }
        }
    }

    /// Open selected file in web browser
    fn open_in_web(&mut self) {
        if let Some(file) = self.state.selected_file() {
            let path = file.path.clone();
            match ops::open_in_browser(&path) {
                Ok(()) => {
                    self.state.message = Some("Opening in browser...".to_string());
                }
                Err(e) => {
                    self.state.error = Some(e);
                }
            }
        }
    }

    /// Copy share link to clipboard
    fn copy_share_link(&mut self) {
        if let Some(file) = self.state.selected_file() {
            let path = file.path.clone();
            match ops::get_share_link(&path) {
                Ok(url) => {
                    // Try to copy to clipboard
                    #[cfg(target_os = "macos")]
                    {
                        use std::process::Command;
                        let _ = Command::new("pbcopy")
                            .stdin(std::process::Stdio::piped())
                            .spawn()
                            .and_then(|mut child| {
                                use std::io::Write;
                                if let Some(stdin) = child.stdin.as_mut() {
                                    stdin.write_all(url.as_bytes())?;
                                }
                                child.wait()
                            });
                    }
                    self.state.message = Some(format!("Link copied: {}", url));
                }
                Err(e) => {
                    self.state.error = Some(e);
                }
            }
        }
    }
}

impl Plugin for DropboxPlugin {
    fn id(&self) -> &str {
        "dropbox"
    }

    fn name(&self) -> &str {
        "Dropbox"
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
        // Available if Dropbox is installed and we're in or have a Dropbox folder
        if !ops::is_dropbox_installed() {
            return false;
        }

        if let Some(dropbox_path) = ops::get_dropbox_path() {
            cwd.starts_with(&dropbox_path) || dropbox_path.exists()
        } else {
            false
        }
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "Dropbox".to_string(),
            key: 'D',
            description: "Browse Dropbox with sync status".to_string(),
            priority: 25,
        })
    }

    fn status_info(&self, cwd: &PathBuf) -> Option<PluginStatusInfo> {
        if let Some(dropbox_path) = ops::get_dropbox_path() {
            if cwd.starts_with(&dropbox_path) {
                let running = ops::is_dropbox_running();
                let text = if running {
                    "Dropbox ●".to_string()
                } else {
                    "Dropbox ○".to_string()
                };
                return Some(PluginStatusInfo {
                    text,
                    active: running,
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
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if ops::is_dropbox_installed() {
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
        // Handle error/message dismissal
        if self.state.error.is_some() || self.state.message.is_some() {
            self.state.error = None;
            self.state.message = None;
            return KeyHandleResult::Handled;
        }

        match self.state.view {
            DropboxView::Browser | DropboxView::Filter => match key.code {
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
                KeyCode::Char('i') | KeyCode::Char('I') => {
                    // Refresh status when opening info view
                    self.state.is_running = ops::is_dropbox_running();
                    self.state.storage_info = ops::get_storage_info();
                    self.state.view = DropboxView::Info;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('w') | KeyCode::Char('W') => {
                    self.open_in_web();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.copy_share_link();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    // Re-detect Dropbox on refresh
                    self.state.is_installed = ops::is_dropbox_installed();
                    self.state.is_running = ops::is_dropbox_running();
                    self.state.storage_info = ops::get_storage_info();

                    if let Some(dropbox_root) = ops::get_dropbox_path() {
                        // If current_dir is valid and within Dropbox, refresh it
                        // Otherwise, reset to Dropbox root
                        let dir = if self.state.current_dir.starts_with(&dropbox_root) {
                            self.state.current_dir.clone()
                        } else {
                            self.state.current_dir = dropbox_root.clone();
                            dropbox_root
                        };
                        self.state.error = None;
                        ops::load_directory(&mut self.state, &dir);
                    } else {
                        // Dropbox not available
                        self.state.files.clear();
                        self.state.current_dir = PathBuf::new();
                        if !self.state.is_installed {
                            self.state.error = Some("Dropbox is not installed".to_string());
                        } else if !self.state.is_running {
                            self.state.error =
                                Some("Dropbox is not running. Please start Dropbox.".to_string());
                        } else {
                            self.state.error = Some("Dropbox folder not found".to_string());
                        }
                    }
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            DropboxView::Info => match key.code {
                KeyCode::Esc | KeyCode::Char('i') | KeyCode::Char('I') => {
                    self.state.view = DropboxView::Browser;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        modal::draw_dropbox_modal(frame, area, &self.state, colors);
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "        D -- DROPBOX".to_string(),
            "".to_string(),
            "Purpose:   Browse Dropbox folder with sync status display.".to_string(),
            "           View which files are synced, syncing, or have errors.".to_string(),
            "".to_string(),
            "To use:    Press D to open Dropbox browser. Requires Dropbox".to_string(),
            "           desktop application to be installed.".to_string(),
            "".to_string(),
            "Navigation:".to_string(),
            "  ↑/↓       Navigate files".to_string(),
            "  Enter     Open directory".to_string(),
            "  Backspace Go to parent directory".to_string(),
            "  F         Cycle through filters (All/Syncing/Errors/Excluded)".to_string(),
            "  I         Show Dropbox info".to_string(),
            "  W         Open in web browser".to_string(),
            "  S         Copy share link".to_string(),
            "  R         Refresh".to_string(),
            "  Esc       Close".to_string(),
            "".to_string(),
            "Status indicators:".to_string(),
            "  *  Synced      ~  Syncing".to_string(),
            "  !  Error       -  Excluded".to_string(),
        ]
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "Dropbox".to_string(),
            description: "Browse Dropbox with sync status".to_string(),
            category: PluginCategory::Files,
            key: 'D',
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

impl CloudStoragePlugin for DropboxPlugin {
    fn provider(&self) -> CloudProvider {
        CloudProvider::Dropbox
    }

    fn root_path(&self) -> Option<PathBuf> {
        ops::get_dropbox_path()
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

    fn download_file(&mut self, _path: &PathBuf) -> Result<(), String> {
        // Dropbox automatically syncs files, no manual download needed
        Err("Dropbox files sync automatically".to_string())
    }

    fn get_share_link(&self, path: &PathBuf) -> Result<String, String> {
        ops::get_share_link(path)
    }

    fn open_in_browser(&self, path: &PathBuf) -> Result<(), String> {
        ops::open_in_browser(path)
    }

    fn force_sync(&mut self, _path: &PathBuf) -> Result<(), String> {
        // Would need Dropbox CLI or API
        Err("Force sync requires Dropbox CLI".to_string())
    }

    fn is_service_installed(&self) -> bool {
        ops::is_dropbox_installed()
    }

    fn is_service_running(&self) -> bool {
        ops::is_dropbox_running()
    }

    fn refresh(&mut self) {
        self.state.is_running = ops::is_dropbox_running();
        self.state.storage_info = ops::get_storage_info();
        let dir = self.state.current_dir.clone();
        ops::load_directory(&mut self.state, &dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dropbox_plugin_creation() {
        let plugin = DropboxPlugin::new();
        assert_eq!(plugin.id(), "dropbox");
        assert_eq!(plugin.name(), "Dropbox");
    }

    #[test]
    fn test_dropbox_capabilities() {
        let plugin = DropboxPlugin::new();
        let caps = plugin.capabilities();
        assert!(caps.has_modal);
        assert!(caps.has_menu);
    }
}
