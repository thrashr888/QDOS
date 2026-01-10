//! Q-LINK: MCP Client Plugin
//!
//! Enables R-DOS to connect to MCP (Model Context Protocol) servers
//! and browse their resources as virtual filesystems.
//!
//! Press `L` from the App Launcher to open Q-LINK.

mod modal;
pub mod state;

use crate::mcp::ServerConfig as McpServerConfig;
use crate::plugins::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, PluginMenuItem,
    PluginStatusInfo,
};
use crate::vfs::{FileSystemProvider, McpFS, RoutingFS};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, Frame};
use state::{QLinkState, QLinkView, ServerConfig};
use std::any::Any;
use std::path::PathBuf;
use std::sync::Arc;

/// Q-LINK MCP Client Plugin
pub struct QLinkPlugin {
    /// Plugin state
    pub state: QLinkState,
    /// Routing filesystem for mounts
    routing_fs: Arc<RoutingFS>,
    /// Whether currently connecting
    connecting: bool,
}

impl Default for QLinkPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl QLinkPlugin {
    /// Create a new Q-LINK plugin
    pub fn new() -> Self {
        let routing_fs = Arc::new(RoutingFS::new());
        let mut state = QLinkState::new();

        // Add some example servers (in a real app, these would come from config)
        state.add_server(
            ServerConfig::new("filesystem", "Local Filesystem", "npx")
                .with_args(vec![
                    "-y".to_string(),
                    "@modelcontextprotocol/server-filesystem".to_string(),
                    "/tmp".to_string(),
                ])
                .with_mount_path("/mcp/filesystem"),
        );

        Self {
            state,
            routing_fs,
            connecting: false,
        }
    }

    /// Create with a custom routing filesystem
    pub fn with_routing_fs(routing_fs: Arc<RoutingFS>) -> Self {
        let mut plugin = Self::new();
        plugin.routing_fs = routing_fs;
        plugin
    }

    /// Get the routing filesystem
    pub fn routing_fs(&self) -> Arc<RoutingFS> {
        Arc::clone(&self.routing_fs)
    }

    /// Connect to the selected server
    fn connect_selected(&mut self) {
        if let Some(server) = self.state.selected_server_mut() {
            if server.is_connected() {
                // Already connected, navigate instead
                return;
            }

            server.set_connecting();
            self.state.view = QLinkView::Mounting;
            self.connecting = true;
        }
    }

    /// Actually perform the connection (called from tick)
    fn do_connect(&mut self) {
        let server_index = self.state.selected_index;

        if let Some(server) = self.state.servers.get(server_index) {
            let config = McpServerConfig::new(&server.config.command, server.config.args.clone());
            let mount_path = server.config.mount_path.clone();
            let server_name = server.config.name.clone();
            let base_uri = server.config.base_uri.clone();

            // Try to create the MCP filesystem
            match McpFS::spawn(&config, server_name.clone(), base_uri) {
                Ok(mcp_fs) => {
                    let provider: Arc<dyn FileSystemProvider> = Arc::new(mcp_fs);

                    // Mount the filesystem
                    if let Err(e) = self.routing_fs.mount(mount_path.clone(), provider) {
                        if let Some(s) = self.state.servers.get_mut(server_index) {
                            s.set_error(format!("Mount failed: {}", e));
                        }
                        self.state.view = QLinkView::ServerList;
                    } else {
                        if let Some(s) = self.state.servers.get_mut(server_index) {
                            s.set_connected();
                        }
                        self.state
                            .set_status(format!("Connected to {}", server_name));
                        self.state.view = QLinkView::ServerList;
                    }
                }
                Err(e) => {
                    if let Some(s) = self.state.servers.get_mut(server_index) {
                        s.set_error(format!("{}", e));
                    }
                    self.state.view = QLinkView::ServerList;
                }
            }
        }

        self.connecting = false;
    }

    /// Disconnect the selected server
    fn disconnect_selected(&mut self) {
        if let Some(server) = self.state.selected_server_mut() {
            if !server.is_connected() {
                return;
            }

            let mount_path = server.config.mount_path.clone();
            let server_name = server.config.name.clone();

            // Unmount
            if let Err(e) = self.routing_fs.unmount(&mount_path) {
                self.state.set_error(format!("Unmount failed: {}", e));
            } else {
                server.set_disconnected();
                self.state
                    .set_status(format!("Disconnected from {}", server_name));
            }
        }
    }
}

impl Plugin for QLinkPlugin {
    fn id(&self) -> &str {
        "qlink"
    }

    fn name(&self) -> &str {
        "Q-LINK"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: false,
            has_keys: true,
            has_modal: true,
            has_status: true,
            has_cli: false,
            has_help: true,
        }
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        true
    }

    fn menu_item(&self) -> Option<PluginMenuItem> {
        None
    }

    fn status_info(&self, _cwd: &PathBuf) -> Option<PluginStatusInfo> {
        let connected_count = self
            .state
            .servers
            .iter()
            .filter(|s| s.is_connected())
            .count();

        if connected_count > 0 {
            Some(PluginStatusInfo {
                text: format!("Q-LINK {}", connected_count),
                active: true,
            })
        } else {
            None
        }
    }

    fn handle_global_key(
        &mut self,
        key: KeyEvent,
        _cwd: &PathBuf,
        _selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
        // L key opens Q-LINK (but we'll use App Launcher primarily)
        if let KeyCode::Char('L') = key.code {
            self.state.view = QLinkView::ServerList;
            self.state.clear_messages();
            return KeyHandleResult::OpenModal;
        }
        KeyHandleResult::NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match self.state.view {
            QLinkView::ServerList => match key.code {
                KeyCode::Esc => KeyHandleResult::CloseModal,
                KeyCode::Up | KeyCode::Char('k') => {
                    self.state.select_prev();
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.state.select_next();
                    KeyHandleResult::Handled
                }
                KeyCode::Enter => {
                    if let Some(server) = self.state.selected_server() {
                        if server.is_connected() {
                            // Navigate to mount point
                            let path = server.config.mount_path.clone();
                            return KeyHandleResult::NavigateToFile(path);
                        } else {
                            // Connect
                            self.connect_selected();
                        }
                    }
                    KeyHandleResult::Handled
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    self.disconnect_selected();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('i') | KeyCode::Char('I') => {
                    self.state.view = QLinkView::Details;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            QLinkView::Mounting => match key.code {
                KeyCode::Esc => {
                    // Cancel connection
                    self.connecting = false;
                    if let Some(server) = self.state.selected_server_mut() {
                        server.set_disconnected();
                    }
                    self.state.view = QLinkView::ServerList;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            QLinkView::Details => match key.code {
                KeyCode::Esc => {
                    self.state.view = QLinkView::ServerList;
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &crate::app::ThemeColors) {
        modal::draw_qlink_modal(frame, area, &self.state, colors);
    }

    fn tick(&mut self) {
        // Handle async connection
        if self.connecting {
            self.do_connect();
        }
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "Q-LINK - MCP Network Client".to_string(),
            "".to_string(),
            "Connect to MCP servers and browse their resources".to_string(),
            "as virtual filesystems.".to_string(),
            "".to_string(),
            "Keys:".to_string(),
            "  Enter     Connect / Navigate to mount".to_string(),
            "  D         Disconnect selected server".to_string(),
            "  I         View server details".to_string(),
            "  Esc       Close / Back".to_string(),
            "".to_string(),
            "Status Indicators:".to_string(),
            "  [*]  Disconnected".to_string(),
            "  [~]  Connecting".to_string(),
            "  [+]  Connected".to_string(),
            "  [!]  Error".to_string(),
        ]
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "Q-LINK".to_string(),
            description: "MCP network connections".to_string(),
            category: PluginCategory::Tools,
            key: 'L',
        })
    }

    fn launch(&mut self, _cwd: &PathBuf, _selected_file: Option<&PathBuf>) -> Result<(), String> {
        self.state.view = QLinkView::ServerList;
        self.state.clear_messages();
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let plugin = QLinkPlugin::new();
        assert_eq!(plugin.id(), "qlink");
        assert_eq!(plugin.name(), "Q-LINK");
    }

    #[test]
    fn test_plugin_capabilities() {
        let plugin = QLinkPlugin::new();
        let caps = plugin.capabilities();
        assert!(caps.has_modal);
        assert!(caps.has_status);
    }

    #[test]
    fn test_app_entry() {
        let plugin = QLinkPlugin::new();
        let entry = plugin.app_entry().unwrap();
        assert_eq!(entry.key, 'L');
    }
}
