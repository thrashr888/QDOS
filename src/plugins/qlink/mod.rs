//! Q-LINK: MCP Client Plugin
//!
//! Enables R-DOS to connect to MCP (Model Context Protocol) servers
//! and browse their resources as virtual filesystems.
//!
//! Press `L` from the App Launcher to open Q-LINK.

mod modal;
pub mod state;

use crate::mcp::ServerConfig as McpServerConfig;
use crate::vfs::{FileSystemProvider, McpFS, RoutingFS};
use crossterm::event::{KeyCode, KeyEvent};
use qdos_plugin_api::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, PluginMenuItem,
    PluginStatusInfo,
};
use ratatui::{layout::Rect, Frame};
use state::{QLinkState, QLinkView, ServerConfig};
use std::any::Any;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

/// Result of a connection attempt
type ConnectResult = Result<Arc<dyn FileSystemProvider>, String>;

/// Message sent from background thread
struct ConnectComplete {
    server_index: usize,
    server_name: String,
    mount_path: PathBuf,
    result: ConnectResult,
}

/// Q-LINK MCP Client Plugin
pub struct QLinkPlugin {
    /// Plugin state
    pub state: QLinkState,
    /// Routing filesystem for mounts
    routing_fs: Arc<RoutingFS>,
    /// Background connection thread
    #[allow(dead_code)]
    connect_thread: Option<JoinHandle<()>>,
    /// Channel to receive connection results (wrapped in Mutex for Sync)
    connect_rx: Arc<Mutex<Option<Receiver<ConnectComplete>>>>,
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
                .with_mount_path("/tmp/mcp/filesystem")
                .with_server_root("/tmp"),
        );

        Self {
            state,
            routing_fs,
            connect_thread: None,
            connect_rx: Arc::new(Mutex::new(None)),
        }
    }

    /// Create with a custom routing filesystem
    pub fn with_routing_fs(routing_fs: Arc<RoutingFS>) -> Self {
        let mut plugin = Self::new();
        plugin.routing_fs = routing_fs;
        plugin
    }

    /// Get the routing filesystem
    #[allow(dead_code)]
    pub fn routing_fs(&self) -> Arc<RoutingFS> {
        Arc::clone(&self.routing_fs)
    }

    /// Connect to the selected server (spawns background thread)
    fn connect_selected(&mut self) {
        // Get server info before borrowing state mutably
        let server_index = self.state.selected_index;
        let server_info = self.state.servers.get(server_index).map(|s| {
            (
                s.is_connected(),
                s.config.command.clone(),
                s.config.args.clone(),
                s.config.mount_path.clone(),
                s.config.name.clone(),
                s.config.server_root.clone(),
            )
        });

        let Some((is_connected, command, args, mount_path, server_name, server_root)) = server_info
        else {
            return;
        };

        if is_connected {
            // Already connected, navigate instead
            return;
        }

        // Now we can mutate state
        if let Some(server) = self.state.servers.get_mut(server_index) {
            server.set_connecting();
        }
        self.state.view = QLinkView::Mounting;

        // Create MCP config
        let config = McpServerConfig::new(&command, args);

        // Create channel for result
        let (tx, rx): (Sender<ConnectComplete>, Receiver<ConnectComplete>) = channel();
        if let Ok(mut rx_guard) = self.connect_rx.lock() {
            *rx_guard = Some(rx);
        }

        // Spawn background thread
        let handle = thread::spawn(move || {
            let result = match McpFS::spawn(&config, server_name.clone(), server_root) {
                Ok(mcp_fs) => Ok(Arc::new(mcp_fs) as Arc<dyn FileSystemProvider>),
                Err(e) => Err(format!("{}", e)),
            };

            let _ = tx.send(ConnectComplete {
                server_index,
                server_name,
                mount_path,
                result,
            });
        });

        self.connect_thread = Some(handle);
    }

    /// Check for connection completion (called from tick)
    fn check_connection(&mut self) {
        // Try to get the receiver from the mutex
        let complete = if let Ok(mut rx_guard) = self.connect_rx.lock() {
            if let Some(ref rx) = *rx_guard {
                match rx.try_recv() {
                    Ok(c) => {
                        // Clear the receiver since we got the result
                        *rx_guard = None;
                        Some(c)
                    }
                    Err(_) => None,
                }
            } else {
                None
            }
        } else {
            None
        };

        if let Some(complete) = complete {
            // Connection attempt finished
            self.connect_thread = None;

            match complete.result {
                Ok(provider) => {
                    // Mount the filesystem
                    if let Err(e) = self.routing_fs.mount(complete.mount_path.clone(), provider) {
                        if let Some(s) = self.state.servers.get_mut(complete.server_index) {
                            s.set_error(format!("Mount failed: {}", e));
                        }
                    } else {
                        if let Some(s) = self.state.servers.get_mut(complete.server_index) {
                            s.set_connected();
                        }
                        self.state
                            .set_status(format!("Connected to {}", complete.server_name));
                    }
                }
                Err(e) => {
                    if let Some(s) = self.state.servers.get_mut(complete.server_index) {
                        s.set_error(e);
                    }
                }
            }

            self.state.view = QLinkView::ServerList;
        }
    }

    /// Cancel ongoing connection
    fn cancel_connection(&mut self) {
        // Drop the receiver to signal we don't care about the result
        if let Ok(mut rx_guard) = self.connect_rx.lock() {
            *rx_guard = None;
        }
        // The thread will finish on its own, we just won't use its result
        self.connect_thread = None;

        if let Some(server) = self.state.selected_server_mut() {
            server.set_disconnected();
        }
        self.state.view = QLinkView::ServerList;
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
                            // Navigate to mount point directory
                            let path = server.config.mount_path.clone();
                            return KeyHandleResult::NavigateToDir(path);
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
                    self.cancel_connection();
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

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &qdos_plugin_api::ThemeColors) {
        modal::draw_qlink_modal(frame, area, &self.state, colors);
    }

    fn tick(&mut self) {
        // Check for connection completion
        self.check_connection();
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
