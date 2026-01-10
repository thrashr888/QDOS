//! Stdio transport for MCP communication
//!
//! Manages child process spawning and JSON-RPC message I/O over stdin/stdout.

use super::protocol::{
    parse_message, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, McpError,
};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Log a message to /tmp/mcp-debug.log
fn mcp_log(msg: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/mcp-debug.log")
    {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let _ = writeln!(file, "[{}] {}", timestamp, msg);
    }
}

/// Configuration for spawning an MCP server
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Command to execute (e.g., "npx", "python")
    pub command: String,
    /// Arguments to pass to the command
    pub args: Vec<String>,
    /// Environment variables to set
    pub env: HashMap<String, String>,
    /// Working directory (optional)
    pub cwd: Option<String>,
}

impl ServerConfig {
    /// Create a new server config
    pub fn new(command: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            command: command.into(),
            args,
            env: HashMap::new(),
            cwd: None,
        }
    }

    /// Add an environment variable
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Set the working directory
    pub fn with_cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }
}

/// Stdio transport for MCP server communication
pub struct StdioTransport {
    /// The child process
    child: Child,
    /// Writer to child's stdin
    stdin: ChildStdin,
    /// Receiver for parsed messages from the reader thread
    message_rx: Receiver<Result<JsonRpcMessage, McpError>>,
    /// Handle to the reader thread
    #[allow(dead_code)]
    reader_thread: JoinHandle<()>,
    /// Flag indicating if the transport is connected
    connected: Arc<Mutex<bool>>,
}

impl StdioTransport {
    /// Spawn a new MCP server process and create a transport
    pub fn spawn(config: &ServerConfig) -> Result<Self, McpError> {
        mcp_log(&format!(
            "Spawning MCP server: {} {:?}",
            config.command, config.args
        ));

        // Create stderr log file
        let stderr_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/mcp-stderr.log")
            .ok()
            .map(Stdio::from)
            .unwrap_or(Stdio::null());

        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(stderr_file);

        // Set environment variables
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        // Set working directory if specified
        if let Some(ref cwd) = config.cwd {
            cmd.current_dir(cwd);
        }

        let mut child = cmd.spawn().map_err(|e| {
            McpError::ConnectionFailed(format!("Failed to spawn '{}': {}", config.command, e))
        })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| McpError::ConnectionFailed("Failed to open stdin".to_string()))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| McpError::ConnectionFailed("Failed to open stdout".to_string()))?;

        // Create channel for message passing
        let (tx, rx) = channel();
        let connected = Arc::new(Mutex::new(true));
        let connected_clone = Arc::clone(&connected);

        // Spawn reader thread
        let reader_thread = thread::spawn(move || {
            Self::reader_loop(stdout, tx, connected_clone);
        });

        Ok(Self {
            child,
            stdin,
            message_rx: rx,
            reader_thread,
            connected,
        })
    }

    /// Reader loop that runs in a separate thread
    fn reader_loop(
        stdout: ChildStdout,
        tx: Sender<Result<JsonRpcMessage, McpError>>,
        connected: Arc<Mutex<bool>>,
    ) {
        mcp_log("Reader loop started");
        let reader = BufReader::new(stdout);

        for line in reader.lines() {
            match line {
                Ok(text) => {
                    if text.trim().is_empty() {
                        continue;
                    }
                    mcp_log(&format!("Received: {}", &text[..text.len().min(200)]));
                    let result = parse_message(&text);
                    if tx.send(result).is_err() {
                        mcp_log("Reader: receiver dropped");
                        break;
                    }
                }
                Err(e) => {
                    mcp_log(&format!("Reader error: {}", e));
                    let _ = tx.send(Err(McpError::IoError(e.to_string())));
                    break;
                }
            }
        }

        // Mark as disconnected
        if let Ok(mut c) = connected.lock() {
            *c = false;
        }
    }

    /// Send a JSON-RPC request
    pub fn send_request(&mut self, request: &JsonRpcRequest) -> Result<(), McpError> {
        let json = request.to_json_line()?;
        mcp_log(&format!(
            "Sending request: {}",
            &json[..json.len().min(200)]
        ));
        self.stdin
            .write_all(json.as_bytes())
            .map_err(|e| McpError::TransportError(e.to_string()))?;
        self.stdin
            .flush()
            .map_err(|e| McpError::TransportError(e.to_string()))?;
        mcp_log("Request sent");
        Ok(())
    }

    /// Send a JSON-RPC notification
    pub fn send_notification(
        &mut self,
        notification: &JsonRpcNotification,
    ) -> Result<(), McpError> {
        let json = notification.to_json_line()?;
        self.stdin
            .write_all(json.as_bytes())
            .map_err(|e| McpError::TransportError(e.to_string()))?;
        self.stdin
            .flush()
            .map_err(|e| McpError::TransportError(e.to_string()))?;
        Ok(())
    }

    /// Try to receive a message without blocking
    pub fn try_recv(&self) -> Option<Result<JsonRpcMessage, McpError>> {
        match self.message_rx.try_recv() {
            Ok(msg) => Some(msg),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err(McpError::Disconnected)),
        }
    }

    /// Receive a message with timeout
    pub fn recv_timeout(&self, timeout: Duration) -> Result<JsonRpcMessage, McpError> {
        match self.message_rx.recv_timeout(timeout) {
            Ok(Ok(msg)) => {
                mcp_log("recv_timeout: got message");
                Ok(msg)
            }
            Ok(Err(e)) => {
                mcp_log(&format!("recv_timeout: parse error {:?}", e));
                Err(e)
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Don't log every timeout - too noisy
                Err(McpError::Timeout)
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                mcp_log("recv_timeout: disconnected");
                Err(McpError::Disconnected)
            }
        }
    }

    /// Check if the transport is still connected
    pub fn is_connected(&self) -> bool {
        self.connected.lock().map(|c| *c).unwrap_or(false)
    }

    /// Close the transport and kill the child process
    pub fn close(&mut self) {
        if let Ok(mut c) = self.connected.lock() {
            *c = false;
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for StdioTransport {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config() {
        let config = ServerConfig::new("echo", vec!["hello".to_string()])
            .with_env("FOO", "bar")
            .with_cwd("/tmp");

        assert_eq!(config.command, "echo");
        assert_eq!(config.args, vec!["hello"]);
        assert_eq!(config.env.get("FOO"), Some(&"bar".to_string()));
        assert_eq!(config.cwd, Some("/tmp".to_string()));
    }

    #[test]
    fn test_spawn_simple_command() {
        // Test with a simple command that exits immediately
        let config = ServerConfig::new("echo", vec!["test".to_string()]);
        let result = StdioTransport::spawn(&config);
        // This may succeed or fail depending on the environment
        // Just verify it doesn't panic
        drop(result);
    }
}
