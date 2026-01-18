//! Q-LINK plugin state
//!
//! State types for the MCP client plugin.

use std::path::PathBuf;
use std::time::Instant;

/// View modes for the Q-LINK modal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QLinkView {
    /// Server list view (default)
    #[default]
    ServerList,
    /// Mounting a server
    Mounting,
    /// Server details view
    Details,
}

/// Connection status for an MCP server
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionStatus {
    /// Server not connected
    #[default]
    Disconnected,
    /// Currently connecting
    Connecting,
    /// Connected and ready
    Connected,
    /// Connection failed
    Error,
}

/// Configuration for an MCP server
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Unique identifier for this server
    pub id: String,
    /// Display name
    pub name: String,
    /// Command to execute (e.g., "npx", "python")
    pub command: String,
    /// Arguments to pass to the command
    pub args: Vec<String>,
    /// Mount point path (e.g., "/mcp/github")
    pub mount_path: PathBuf,
    /// Server's root path (e.g., "/tmp" for filesystem server)
    /// This is the base path that the MCP server exposes
    pub server_root: String,
    /// Whether to auto-mount on startup
    pub auto_mount: bool,
}

impl ServerConfig {
    /// Create a new server configuration
    pub fn new(id: impl Into<String>, name: impl Into<String>, command: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            mount_path: PathBuf::from(format!("/mcp/{}", id)),
            id,
            name: name.into(),
            command: command.into(),
            args: Vec::new(),
            server_root: "/".to_string(),
            auto_mount: false,
        }
    }

    /// Set command arguments
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Set mount path
    pub fn with_mount_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.mount_path = path.into();
        self
    }

    /// Set the server's root path (the base path the MCP server exposes)
    pub fn with_server_root(mut self, root: impl Into<String>) -> Self {
        self.server_root = root.into();
        self
    }

    /// Set auto-mount
    pub fn with_auto_mount(mut self, auto: bool) -> Self {
        self.auto_mount = auto;
        self
    }
}

/// A mounted MCP server
#[derive(Debug)]
pub struct MountedServer {
    /// Server configuration
    pub config: ServerConfig,
    /// Connection status
    pub status: ConnectionStatus,
    /// Time when connection was established
    pub connected_at: Option<Instant>,
    /// Last error message
    pub error: Option<String>,
}

impl MountedServer {
    /// Create a new mounted server entry
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            status: ConnectionStatus::Disconnected,
            connected_at: None,
            error: None,
        }
    }

    /// Mark as connecting
    pub fn set_connecting(&mut self) {
        self.status = ConnectionStatus::Connecting;
        self.error = None;
    }

    /// Mark as connected
    pub fn set_connected(&mut self) {
        self.status = ConnectionStatus::Connected;
        self.connected_at = Some(Instant::now());
        self.error = None;
    }

    /// Mark as disconnected
    pub fn set_disconnected(&mut self) {
        self.status = ConnectionStatus::Disconnected;
        self.connected_at = None;
    }

    /// Mark as error
    pub fn set_error(&mut self, error: impl Into<String>) {
        self.status = ConnectionStatus::Error;
        self.error = Some(error.into());
    }

    /// Get status text
    pub fn status_text(&self) -> &str {
        match self.status {
            ConnectionStatus::Disconnected => "Offline",
            ConnectionStatus::Connecting => "Connecting...",
            ConnectionStatus::Connected => "Connected",
            ConnectionStatus::Error => "Error",
        }
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        matches!(self.status, ConnectionStatus::Connected)
    }
}

/// Q-LINK plugin state
#[derive(Debug, Default)]
pub struct QLinkState {
    /// Current view
    pub view: QLinkView,
    /// Available servers
    pub servers: Vec<MountedServer>,
    /// Currently selected server index
    pub selected_index: usize,
    /// Status message to display
    pub status_message: Option<String>,
    /// Error message to display
    pub error: Option<String>,
}

impl QLinkState {
    /// Create new state
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a server configuration
    pub fn add_server(&mut self, config: ServerConfig) {
        self.servers.push(MountedServer::new(config));
    }

    /// Get the currently selected server
    pub fn selected_server(&self) -> Option<&MountedServer> {
        self.servers.get(self.selected_index)
    }

    /// Get the currently selected server mutably
    pub fn selected_server_mut(&mut self) -> Option<&mut MountedServer> {
        self.servers.get_mut(self.selected_index)
    }

    /// Move selection up
    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if self.selected_index < self.servers.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    /// Set status message
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
        self.error = None;
    }

    /// Set error message
    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.error = Some(msg.into());
        self.status_message = None;
    }

    /// Clear messages
    pub fn clear_messages(&mut self) {
        self.status_message = None;
        self.error = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config() {
        let config = ServerConfig::new("test", "Test Server", "echo")
            .with_args(vec!["hello".to_string()])
            .with_mount_path("/mcp/test")
            .with_auto_mount(true);

        assert_eq!(config.id, "test");
        assert_eq!(config.name, "Test Server");
        assert_eq!(config.mount_path, PathBuf::from("/mcp/test"));
        assert!(config.auto_mount);
    }

    #[test]
    fn test_state_navigation() {
        let mut state = QLinkState::new();
        state.add_server(ServerConfig::new("a", "A", "cmd"));
        state.add_server(ServerConfig::new("b", "B", "cmd"));
        state.add_server(ServerConfig::new("c", "C", "cmd"));

        assert_eq!(state.selected_index, 0);

        state.select_next();
        assert_eq!(state.selected_index, 1);

        state.select_next();
        assert_eq!(state.selected_index, 2);

        state.select_next(); // Should not go past end
        assert_eq!(state.selected_index, 2);

        state.select_prev();
        assert_eq!(state.selected_index, 1);
    }

    #[test]
    fn test_mounted_server_status() {
        let config = ServerConfig::new("test", "Test", "cmd");
        let mut server = MountedServer::new(config);

        assert!(!server.is_connected());
        assert_eq!(server.status_text(), "Offline");

        server.set_connecting();
        assert_eq!(server.status_text(), "Connecting...");

        server.set_connected();
        assert!(server.is_connected());
        assert_eq!(server.status_text(), "Connected");

        server.set_error("Connection failed");
        assert!(!server.is_connected());
        assert_eq!(server.error, Some("Connection failed".to_string()));
    }
}
