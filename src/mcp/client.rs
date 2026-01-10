//! High-level MCP client
//!
//! Provides a convenient API for interacting with MCP servers.

use super::protocol::{JsonRpcMessage, JsonRpcRequest, McpError};
use super::transport::{ServerConfig, StdioTransport};
use super::types::{
    CallToolResult, InitializeParams, InitializeResult, ListResourcesResult, ListToolsResult,
    ReadResourceResult, Resource, ServerCapabilities, ServerInfo, Tool,
};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

/// Default timeout for MCP operations
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// MCP client for communicating with an MCP server
pub struct McpClient {
    /// Transport layer for I/O
    transport: StdioTransport,
    /// Server information from initialization
    server_info: Option<ServerInfo>,
    /// Server capabilities from initialization
    capabilities: Option<ServerCapabilities>,
    /// Pending requests awaiting responses (id -> request method for context)
    pending: HashMap<u64, String>,
    /// Request timeout
    timeout: Duration,
}

impl McpClient {
    /// Spawn an MCP server and create a client
    pub fn spawn(config: &ServerConfig) -> Result<Self, McpError> {
        let transport = StdioTransport::spawn(config)?;
        Ok(Self {
            transport,
            server_info: None,
            capabilities: None,
            pending: HashMap::new(),
            timeout: DEFAULT_TIMEOUT,
        })
    }

    /// Set the request timeout
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Check if the client is connected
    pub fn is_connected(&self) -> bool {
        self.transport.is_connected()
    }

    /// Check if the client has been initialized
    pub fn is_initialized(&self) -> bool {
        self.server_info.is_some()
    }

    /// Get server info (available after initialization)
    pub fn server_info(&self) -> Option<&ServerInfo> {
        self.server_info.as_ref()
    }

    /// Get server capabilities (available after initialization)
    pub fn capabilities(&self) -> Option<&ServerCapabilities> {
        self.capabilities.as_ref()
    }

    /// Initialize the connection with the MCP server
    pub fn initialize(&mut self) -> Result<InitializeResult, McpError> {
        let params = InitializeParams::default();
        let request = JsonRpcRequest::initialize(&params);
        let id = request.id;

        self.pending.insert(id, "initialize".to_string());
        self.transport.send_request(&request)?;

        let result: InitializeResult = self.wait_for_response(id)?;

        // Send initialized notification
        let notification = JsonRpcRequest::initialized();
        self.transport.send_notification(&notification)?;

        // Store server info and capabilities
        self.server_info = Some(result.server_info.clone());
        self.capabilities = Some(result.capabilities.clone());

        Ok(result)
    }

    /// List available resources
    pub fn list_resources(&mut self) -> Result<Vec<Resource>, McpError> {
        self.ensure_initialized()?;

        let mut all_resources = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let request = JsonRpcRequest::list_resources(cursor.as_deref());
            let id = request.id;

            self.pending.insert(id, "resources/list".to_string());
            self.transport.send_request(&request)?;

            let result: ListResourcesResult = self.wait_for_response(id)?;
            all_resources.extend(result.resources);

            if let Some(next) = result.next_cursor {
                cursor = Some(next);
            } else {
                break;
            }
        }

        Ok(all_resources)
    }

    /// Read a specific resource by URI
    pub fn read_resource(&mut self, uri: &str) -> Result<ReadResourceResult, McpError> {
        self.ensure_initialized()?;

        let request = JsonRpcRequest::read_resource(uri);
        let id = request.id;

        self.pending.insert(id, "resources/read".to_string());
        self.transport.send_request(&request)?;

        self.wait_for_response(id)
    }

    /// List available tools
    pub fn list_tools(&mut self) -> Result<Vec<Tool>, McpError> {
        self.ensure_initialized()?;

        let request = JsonRpcRequest::list_tools();
        let id = request.id;

        self.pending.insert(id, "tools/list".to_string());
        self.transport.send_request(&request)?;

        let result: ListToolsResult = self.wait_for_response(id)?;
        Ok(result.tools)
    }

    /// Call a tool with the given arguments
    pub fn call_tool(
        &mut self,
        name: &str,
        arguments: Option<Value>,
    ) -> Result<CallToolResult, McpError> {
        self.ensure_initialized()?;

        let request = JsonRpcRequest::call_tool(name, arguments);
        let id = request.id;

        self.pending.insert(id, "tools/call".to_string());
        self.transport.send_request(&request)?;

        self.wait_for_response(id)
    }

    /// Ensure the client has been initialized
    fn ensure_initialized(&self) -> Result<(), McpError> {
        if !self.is_initialized() {
            return Err(McpError::NotInitialized);
        }
        Ok(())
    }

    /// Wait for a response to a specific request
    fn wait_for_response<T: for<'de> serde::Deserialize<'de>>(
        &mut self,
        request_id: u64,
    ) -> Result<T, McpError> {
        let deadline = std::time::Instant::now() + self.timeout;

        loop {
            if std::time::Instant::now() > deadline {
                self.pending.remove(&request_id);
                return Err(McpError::Timeout);
            }

            // Check for messages with a short timeout
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            let recv_timeout = remaining.min(Duration::from_millis(100));

            match self.transport.recv_timeout(recv_timeout) {
                Ok(JsonRpcMessage::Response(response)) => {
                    if response.id == Some(request_id) {
                        self.pending.remove(&request_id);
                        return response.parse_result();
                    }
                    // Not our response, might be for another pending request
                    // In a more complex implementation, we'd queue this
                }
                Ok(JsonRpcMessage::Notification(_notification)) => {
                    // Handle notifications (logging, progress, etc.)
                    // For now, just ignore them
                }
                Ok(JsonRpcMessage::Request(_request)) => {
                    // Server-initiated requests (sampling, roots)
                    // For now, just ignore them
                }
                Err(McpError::Timeout) => {
                    // Recv timeout, loop and check deadline
                    continue;
                }
                Err(e) => {
                    self.pending.remove(&request_id);
                    return Err(e);
                }
            }
        }
    }

    /// Close the client connection
    pub fn close(&mut self) {
        self.transport.close();
        self.server_info = None;
        self.capabilities = None;
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        self.close();
    }
}

/// Builder for creating MCP clients with custom configuration
pub struct McpClientBuilder {
    config: ServerConfig,
    timeout: Duration,
    auto_initialize: bool,
}

impl McpClientBuilder {
    /// Create a new builder with the given command
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            config: ServerConfig::new(command, Vec::new()),
            timeout: DEFAULT_TIMEOUT,
            auto_initialize: true,
        }
    }

    /// Add command arguments
    pub fn args(mut self, args: Vec<String>) -> Self {
        self.config.args = args;
        self
    }

    /// Add an environment variable
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.env.insert(key.into(), value.into());
        self
    }

    /// Set the working directory
    pub fn cwd(mut self, cwd: impl Into<String>) -> Self {
        self.config.cwd = Some(cwd.into());
        self
    }

    /// Set the request timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Disable auto-initialization
    pub fn no_auto_init(mut self) -> Self {
        self.auto_initialize = false;
        self
    }

    /// Build and optionally initialize the client
    pub fn build(self) -> Result<McpClient, McpError> {
        let mut client = McpClient::spawn(&self.config)?;
        client.set_timeout(self.timeout);

        if self.auto_initialize {
            client.initialize()?;
        }

        Ok(client)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_builder() {
        let builder = McpClientBuilder::new("echo")
            .args(vec!["test".to_string()])
            .env("FOO", "bar")
            .timeout(Duration::from_secs(10))
            .no_auto_init();

        assert_eq!(builder.config.command, "echo");
        assert_eq!(builder.timeout, Duration::from_secs(10));
        assert!(!builder.auto_initialize);
    }

    #[test]
    fn test_not_initialized_error() {
        // Create a mock client without initialization
        let config = ServerConfig::new("cat", vec![]);
        // This will likely fail to spawn properly, which is fine for this test
        // We just want to verify the error handling logic
        let result = StdioTransport::spawn(&config);
        if let Ok(transport) = result {
            let client = McpClient {
                transport,
                server_info: None,
                capabilities: None,
                pending: HashMap::new(),
                timeout: DEFAULT_TIMEOUT,
            };
            // Should fail because not initialized
            assert!(!client.is_initialized());
        }
    }
}
