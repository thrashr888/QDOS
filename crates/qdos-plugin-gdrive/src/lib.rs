//! Google Drive Plugin
//!
//! Provides Google Drive integration with sync status display.

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
use state::{GDriveState, GDriveView};
use std::any::Any;
use std::collections::HashMap;
use std::path::PathBuf;

/// Google Drive plugin
pub struct GDrivePlugin {
    pub state: GDriveState,
    modal_open: bool,
}

impl Default for GDrivePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl GDrivePlugin {
    pub fn new() -> Self {
        let mut state = GDriveState::new();
        let (installed, variant, path) = ops::detect_gdrive();
        state.is_installed = installed;
        state.is_running = ops::is_gdrive_running();
        state.drive_variant = variant;
        if let Some(p) = path {
            state.current_dir = p;
        }
        // If path is None, current_dir stays empty (PathBuf::new())
        // The modal will show an appropriate error when opened

        Self {
            state,
            modal_open: false,
        }
    }

    pub fn open_modal(&mut self) {
        let (installed, variant, path) = ops::detect_gdrive();
        self.state.is_installed = installed;
        self.state.is_running = ops::is_gdrive_running();
        self.state.drive_variant = variant;
        self.state.storage_info = ops::get_storage_info();
        self.state.view = GDriveView::Browser;
        self.state.error = None;
        self.state.message = None;

        if let Some(p) = path {
            self.state.current_dir = p.clone();
            ops::load_directory(&mut self.state, &p);
        } else {
            // No Google Drive path found - show error and clear files
            self.state.files.clear();
            self.state.current_dir = PathBuf::new();
            if !installed {
                self.state.error = Some("Google Drive for Desktop is not installed".to_string());
            } else if !self.state.is_running {
                self.state.error =
                    Some("Google Drive is not running. Please start Google Drive.".to_string());
            } else {
                self.state.error = Some("Google Drive folder not found".to_string());
            }
        }

        self.modal_open = true;
    }

    pub fn close_modal(&mut self) {
        self.modal_open = false;
    }

    fn enter_directory(&mut self) {
        // Get Google Drive root to validate navigation
        let gdrive_root = match ops::get_gdrive_path() {
            Some(root) => root,
            None => {
                self.state.error =
                    Some("Google Drive is not available. Please start Google Drive.".to_string());
                return;
            }
        };

        if let Some(file) = self.state.selected_file() {
            if file.is_dir {
                let path = file.path.clone();
                // Validate that the target path is within Google Drive
                if !path.starts_with(&gdrive_root) {
                    self.state.error = Some("Cannot navigate outside Google Drive".to_string());
                    return;
                }
                ops::load_directory(&mut self.state, &path);
            } else if file.is_google_doc {
                // Open Google Doc in browser
                let path = file.path.clone();
                if let Err(e) = ops::open_in_browser(&path) {
                    self.state.error = Some(e);
                }
            }
        }
    }

    fn go_up(&mut self) {
        // Get Google Drive root to validate navigation
        let gdrive_root = match ops::get_gdrive_path() {
            Some(root) => root,
            None => {
                self.state.error =
                    Some("Google Drive is not available. Please start Google Drive.".to_string());
                return;
            }
        };

        if let Some(parent) = self.state.current_dir.parent() {
            // Don't go above the Google Drive root
            if self.state.current_dir != gdrive_root {
                let parent_path = parent.to_path_buf();
                // Validate parent is still within Google Drive
                if parent_path.starts_with(&gdrive_root) || parent_path == gdrive_root {
                    ops::load_directory(&mut self.state, &parent_path);
                }
            }
        }
    }

    fn open_in_web(&mut self) {
        if let Some(file) = self.state.selected_file() {
            let path = file.path.clone();
            if let Err(e) = ops::open_in_browser(&path) {
                self.state.error = Some(e);
            }
        }
    }
}

impl Plugin for GDrivePlugin {
    fn id(&self) -> &str {
        "gdrive"
    }

    fn name(&self) -> &str {
        "Google Drive"
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
        if !ops::is_gdrive_installed() {
            return false;
        }

        if let Some(gdrive_path) = ops::get_gdrive_path() {
            cwd.starts_with(&gdrive_path) || gdrive_path.exists()
        } else {
            false
        }
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "Google Drive".to_string(),
            key: 'O',
            description: "Browse Google Drive".to_string(),
            priority: 27,
        })
    }

    fn status_info(&self, cwd: &PathBuf) -> Option<PluginStatusInfo> {
        if let Some(gdrive_path) = ops::get_gdrive_path() {
            if cwd.starts_with(&gdrive_path) {
                let running = ops::is_gdrive_running();
                let text = if running {
                    "GDrive ●".to_string()
                } else {
                    "GDrive ○".to_string()
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
            KeyCode::Char('o') | KeyCode::Char('O') => {
                if ops::is_gdrive_installed() {
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
            GDriveView::Browser => match key.code {
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
                KeyCode::Char('i') | KeyCode::Char('I') => {
                    // Refresh status when opening info view
                    let (installed, variant, _) = ops::detect_gdrive();
                    self.state.is_installed = installed;
                    self.state.is_running = ops::is_gdrive_running();
                    self.state.drive_variant = variant;
                    self.state.storage_info = ops::get_storage_info();
                    self.state.view = GDriveView::Info;
                    KeyHandleResult::Handled
                }
                KeyCode::Char('w') | KeyCode::Char('W') => {
                    self.open_in_web();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    // Re-detect Google Drive on refresh
                    let (installed, variant, path) = ops::detect_gdrive();
                    self.state.is_installed = installed;
                    self.state.is_running = ops::is_gdrive_running();
                    self.state.drive_variant = variant;
                    self.state.storage_info = ops::get_storage_info();

                    if let Some(gdrive_root) = path {
                        // If current_dir is valid and within Google Drive, refresh it
                        // Otherwise, reset to Google Drive root
                        let dir = if self.state.current_dir.starts_with(&gdrive_root) {
                            self.state.current_dir.clone()
                        } else {
                            self.state.current_dir = gdrive_root.clone();
                            gdrive_root
                        };
                        self.state.error = None;
                        ops::load_directory(&mut self.state, &dir);
                    } else {
                        // Google Drive not available
                        self.state.files.clear();
                        self.state.current_dir = PathBuf::new();
                        if !installed {
                            self.state.error =
                                Some("Google Drive for Desktop is not installed".to_string());
                        } else if !self.state.is_running {
                            self.state.error = Some(
                                "Google Drive is not running. Please start Google Drive."
                                    .to_string(),
                            );
                        } else {
                            self.state.error = Some("Google Drive folder not found".to_string());
                        }
                    }
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            GDriveView::Info => match key.code {
                KeyCode::Esc | KeyCode::Char('i') | KeyCode::Char('I') => {
                    self.state.view = GDriveView::Browser;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        modal::draw_gdrive_modal(frame, area, &self.state, colors);
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "        O -- GOOGLE DRIVE".to_string(),
            "".to_string(),
            "Purpose:   Browse Google Drive with sync status display.".to_string(),
            "           Open Google Docs, Sheets, and Slides directly.".to_string(),
            "".to_string(),
            "To use:    Press O to open Google Drive browser.".to_string(),
            "           Requires Google Drive for Desktop.".to_string(),
            "".to_string(),
            "Navigation:".to_string(),
            "  ↑/↓       Navigate files".to_string(),
            "  Enter     Open directory or Google Doc".to_string(),
            "  Backspace Go to parent directory".to_string(),
            "  I         Show Google Drive info".to_string(),
            "  W         Open in web browser".to_string(),
            "  R         Refresh".to_string(),
            "  Esc       Close".to_string(),
            "".to_string(),
            "File types:".to_string(),
            "  <DIR>  Folder     <DOC>  Google Doc".to_string(),
        ]
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "Google Drive".to_string(),
            description: "Browse Google Drive".to_string(),
            category: PluginCategory::Files,
            key: 'O',
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

impl CloudStoragePlugin for GDrivePlugin {
    fn provider(&self) -> CloudProvider {
        CloudProvider::GoogleDrive
    }

    fn root_path(&self) -> Option<PathBuf> {
        ops::get_gdrive_path()
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
        Err("Google Drive files sync automatically".to_string())
    }

    fn get_share_link(&self, _path: &PathBuf) -> Result<String, String> {
        Err("Use Google Drive web to share files".to_string())
    }

    fn open_in_browser(&self, path: &PathBuf) -> Result<(), String> {
        ops::open_in_browser(path)
    }

    fn force_sync(&mut self, _path: &PathBuf) -> Result<(), String> {
        Err("Google Drive syncs automatically".to_string())
    }

    fn is_service_installed(&self) -> bool {
        ops::is_gdrive_installed()
    }

    fn is_service_running(&self) -> bool {
        ops::is_gdrive_running()
    }

    fn refresh(&mut self) {
        let (installed, variant, _) = ops::detect_gdrive();
        self.state.is_installed = installed;
        self.state.is_running = ops::is_gdrive_running();
        self.state.drive_variant = variant;
        self.state.storage_info = ops::get_storage_info();
        let dir = self.state.current_dir.clone();
        ops::load_directory(&mut self.state, &dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gdrive_plugin_creation() {
        let plugin = GDrivePlugin::new();
        assert_eq!(plugin.id(), "gdrive");
        assert_eq!(plugin.name(), "Google Drive");
    }
}
