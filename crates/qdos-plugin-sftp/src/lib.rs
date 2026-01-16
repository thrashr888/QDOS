//! SFTP Plugin
//!
//! Provides SFTP file browser with connection management.

#![allow(clippy::ptr_arg)]

mod modal;
mod ops;
mod state;

pub use state::*;

use qdos_plugin_api::prelude::*;

use crossterm::event::{KeyCode, KeyEvent};
use ops::SftpSession;
use ratatui::{layout::Rect, Frame};
use std::any::Any;
use std::path::PathBuf;

/// SFTP plugin
pub struct SftpPlugin {
    pub state: SftpState,
    session: Option<SftpSession>,
}

impl Default for SftpPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl SftpPlugin {
    pub fn new() -> Self {
        let mut state = SftpState::new();
        ops::load_profiles(&mut state);

        Self {
            state,
            session: None,
        }
    }

    fn connect(&mut self) -> Result<(), String> {
        let session = SftpSession::connect(&self.state.connection)?;

        // Get initial directory
        let start_dir = if self.state.connection.default_path.is_empty() {
            // Try to get home directory
            session.realpath(".").unwrap_or_else(|_| "/".to_string())
        } else {
            self.state.connection.default_path.clone()
        };

        // Load initial directory
        ops::load_directory(&mut self.state, &session, &start_dir);

        self.session = Some(session);
        self.state.connected = true;
        self.state.view = SftpView::Browser;

        // Save last connection
        ops::save_profiles(&self.state);

        Ok(())
    }

    fn disconnect(&mut self) {
        self.session = None;
        self.state.connected = false;
        self.state.files.clear();
        self.state.current_dir = "/".to_string();
        self.state.view = SftpView::Connections;
    }

    fn enter_directory(&mut self) {
        if let Some(file) = self.state.selected_file().cloned() {
            if file.is_dir {
                if let Some(ref session) = self.session {
                    ops::load_directory(&mut self.state, session, &file.path);
                }
            }
        }
    }

    fn download_selected(&mut self) {
        if let Some(file) = self.state.selected_file().cloned() {
            if file.is_dir {
                self.state.error = Some("Cannot download directory".to_string());
                return;
            }

            let local_path = self.state.local_dir.join(&file.name);

            if let Some(ref session) = self.session {
                self.state.transfer = Some(state::TransferStatus {
                    direction: state::TransferDirection::Download,
                    filename: file.name.clone(),
                    total_bytes: file.size,
                    transferred_bytes: 0,
                    complete: false,
                    error: None,
                });
                self.state.view = SftpView::Transfer;

                match session.download(&file.path, &local_path) {
                    Ok(bytes) => {
                        if let Some(ref mut transfer) = self.state.transfer {
                            transfer.transferred_bytes = bytes;
                            transfer.complete = true;
                        }
                        self.state.message = Some(format!(
                            "Downloaded {} to {}",
                            file.name,
                            local_path.display()
                        ));
                        self.state.view = SftpView::Browser;
                        self.state.transfer = None;
                    }
                    Err(e) => {
                        if let Some(ref mut transfer) = self.state.transfer {
                            transfer.error = Some(e.clone());
                        }
                        self.state.error = Some(e);
                        self.state.view = SftpView::Error;
                    }
                }
            }
        }
    }

    fn handle_connections_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.reset();
                KeyHandleResult::CloseModal
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.select_prev_profile();
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.select_next_profile();
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                if let Some(profile) = self.state.selected_profile().cloned() {
                    self.state.connection = profile.connection;
                    match self.connect() {
                        Ok(()) => KeyHandleResult::Handled,
                        Err(e) => {
                            self.state.error = Some(e);
                            self.state.view = SftpView::Error;
                            KeyHandleResult::Handled
                        }
                    }
                } else {
                    KeyHandleResult::Handled
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.state.connection = state::SftpConnection::new();
                self.state.view = SftpView::Connect;
                KeyHandleResult::Handled
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                self.state.delete_selected_profile();
                ops::save_profiles(&self.state);
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_connect_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.view = SftpView::Connections;
                KeyHandleResult::Handled
            }
            KeyCode::Tab => {
                if self.state.connect_field == state::ConnectField::AuthMethod {
                    self.state.cycle_auth_method();
                } else {
                    self.state.next_connect_field();
                }
                KeyHandleResult::Handled
            }
            KeyCode::BackTab => {
                self.state.prev_connect_field();
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                if self.state.connection.host.is_empty() {
                    self.state.error = Some("Host is required".to_string());
                    self.state.view = SftpView::Error;
                    return KeyHandleResult::Handled;
                }

                match self.connect() {
                    Ok(()) => KeyHandleResult::Handled,
                    Err(e) => {
                        self.state.error = Some(e);
                        self.state.view = SftpView::Error;
                        KeyHandleResult::Handled
                    }
                }
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.state.profile_name.clear();
                self.state.view = SftpView::SaveProfile;
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                self.state.backspace_connect();
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                self.state.insert_connect_char(c);
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_browser_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        // Clear messages on any key
        if self.state.message.is_some() {
            self.state.message = None;
        }

        match key.code {
            KeyCode::Esc => {
                self.disconnect();
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.select_prev_file();
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.select_next_file();
                KeyHandleResult::Handled
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                self.enter_directory();
                KeyHandleResult::Handled
            }
            KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => {
                // Go up if not at root
                if self.state.current_dir != "/" {
                    if let Some(ref session) = self.session {
                        let parent = std::path::Path::new(&self.state.current_dir)
                            .parent()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|| "/".to_string());
                        ops::load_directory(&mut self.state, session, &parent);
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char('g') | KeyCode::Char('G') => {
                self.download_selected();
                KeyHandleResult::Handled
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                // Refresh
                if let Some(ref session) = self.session {
                    let dir = self.state.current_dir.clone();
                    ops::load_directory(&mut self.state, session, &dir);
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_save_profile_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.view = SftpView::Connect;
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                if !self.state.profile_name.is_empty() {
                    let profile = SftpProfile {
                        name: self.state.profile_name.clone(),
                        connection: self.state.connection.clone(),
                    };
                    self.state.profiles.push(profile);
                    ops::save_profiles(&self.state);
                    self.state.view = SftpView::Connect;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                self.state.profile_name.pop();
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                self.state.profile_name.push(c);
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_error_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                self.state.error = None;
                self.state.view = if self.state.connected {
                    SftpView::Browser
                } else {
                    SftpView::Connections
                };
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }
}

impl Plugin for SftpPlugin {
    fn id(&self) -> &str {
        "sftp"
    }

    fn name(&self) -> &str {
        "SFTP"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: true,
            has_keys: true,
            has_modal: true,
            has_status: false,
            has_cli: false,
            has_help: true,
        }
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        true // SFTP is always available
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        Some(PluginMenuItem {
            name: "SFTP".to_string(),
            key: 'F',
            description: "Browse remote SFTP servers".to_string(),
            priority: 28,
        })
    }

    fn handle_global_key(
        &mut self,
        key: KeyEvent,
        cwd: &PathBuf,
        _selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
        match key.code {
            KeyCode::Char('f') | KeyCode::Char('F') => {
                self.state.local_dir = cwd.clone();
                self.state.view = SftpView::Connections;
                ops::load_profiles(&mut self.state);
                KeyHandleResult::OpenModal
            }
            _ => KeyHandleResult::NotHandled,
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match self.state.view {
            SftpView::Connections => self.handle_connections_key(key),
            SftpView::Connect => self.handle_connect_key(key),
            SftpView::Browser => self.handle_browser_key(key),
            SftpView::Transfer => {
                // Cancel transfer on Esc
                if key.code == KeyCode::Esc {
                    self.state.transfer = None;
                    self.state.view = SftpView::Browser;
                }
                KeyHandleResult::Handled
            }
            SftpView::SaveProfile => self.handle_save_profile_key(key),
            SftpView::Error => self.handle_error_key(key),
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        modal::draw_sftp_modal(frame, area, &self.state, colors);
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "        F -- SFTP".to_string(),
            "".to_string(),
            "Purpose:   Browse and transfer files via SFTP.".to_string(),
            "           Connect to remote servers securely.".to_string(),
            "".to_string(),
            "To use:    Press F to open SFTP browser.".to_string(),
            "           Create and save connection profiles.".to_string(),
            "".to_string(),
            "Connections:".to_string(),
            "  Enter     Connect to selected profile".to_string(),
            "  N         New connection".to_string(),
            "  D         Delete selected profile".to_string(),
            "".to_string(),
            "Browser:".to_string(),
            "  ↑/↓       Navigate files".to_string(),
            "  Enter     Open directory".to_string(),
            "  G         Download selected file".to_string(),
            "  R         Refresh".to_string(),
            "  Esc       Disconnect".to_string(),
            "".to_string(),
            "Auth:      SSH key, password, or SSH agent".to_string(),
        ]
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "SFTP".to_string(),
            description: "Browse remote SFTP servers".to_string(),
            category: PluginCategory::Files,
            key: 'F',
        })
    }

    fn launch(&mut self, cwd: &PathBuf, _selected_file: Option<&PathBuf>) -> Result<(), String> {
        self.state.local_dir = cwd.clone();
        self.state.view = SftpView::Connections;
        ops::load_profiles(&mut self.state);
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

inventory::submit! { PluginRegistration::new("sftp", || Box::new(SftpPlugin::new())) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sftp_plugin_creation() {
        let plugin = SftpPlugin::new();
        assert_eq!(plugin.id(), "sftp");
        assert_eq!(plugin.name(), "SFTP");
    }

    #[test]
    fn test_sftp_capabilities() {
        let plugin = SftpPlugin::new();
        let caps = plugin.capabilities();
        assert!(caps.has_menu);
        assert!(caps.has_modal);
        assert!(caps.has_help);
    }
}
