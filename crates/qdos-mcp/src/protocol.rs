//! JSON-RPC 2.0 protocol implementation for MCP
//!
//! Handles serialization and deserialization of JSON-RPC messages.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};

/// Global request ID counter
static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

/// Generate a new unique request ID
pub fn next_request_id() -> u64 {
    REQUEST_ID.fetch_add(1, Ordering::SeqCst)
}

/// JSON-RPC 2.0 request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    /// Create a new JSON-RPC request
    pub fn new(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: next_request_id(),
            method: method.into(),
            params,
        }
    }

    /// Create an initialize request
    pub fn initialize(params: &crate::types::InitializeParams) -> Self {
        Self::new(
            "initialize",
            Some(serde_json::to_value(params).unwrap_or(Value::Null)),
        )
    }

    /// Create an initialized notification (no response expected)
    pub fn initialized() -> JsonRpcNotification {
        JsonRpcNotification::new("notifications/initialized", None)
    }

    /// Create a resources/list request
    pub fn list_resources(cursor: Option<&str>) -> Self {
        let params = if let Some(c) = cursor {
            Some(serde_json::json!({ "cursor": c }))
        } else {
            Some(serde_json::json!({}))
        };
        Self::new("resources/list", params)
    }

    /// Create a resources/read request
    pub fn read_resource(uri: &str) -> Self {
        Self::new("resources/read", Some(serde_json::json!({ "uri": uri })))
    }

    /// Create a tools/list request
    pub fn list_tools() -> Self {
        Self::new("tools/list", Some(serde_json::json!({})))
    }

    /// Create a tools/call request
    pub fn call_tool(name: &str, arguments: Option<Value>) -> Self {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments.unwrap_or(Value::Object(serde_json::Map::new()))
        });
        Self::new("tools/call", Some(params))
    }

    /// Serialize to JSON string with newline (for stdio transport)
    pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
        let mut json = serde_json::to_string(self)?;
        json.push('\n');
        Ok(json)
    }
}

/// JSON-RPC 2.0 notification (no id, no response expected)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcNotification {
    /// Create a new notification
    pub fn new(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params,
        }
    }

    /// Serialize to JSON string with newline
    pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
        let mut json = serde_json::to_string(self)?;
        json.push('\n');
        Ok(json)
    }
}

/// JSON-RPC 2.0 response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Check if this is an error response
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// Get the result, returning an error if this is an error response
    pub fn into_result(self) -> Result<Value, McpError> {
        if let Some(error) = self.error {
            Err(McpError::RpcError {
                code: error.code,
                message: error.message,
            })
        } else {
            self.result.ok_or(McpError::EmptyResponse)
        }
    }

    /// Parse the result as a specific type
    pub fn parse_result<T: for<'de> Deserialize<'de>>(self) -> Result<T, McpError> {
        let value = self.into_result()?;
        serde_json::from_value(value).map_err(|e| McpError::ParseError(e.to_string()))
    }
}

/// JSON-RPC 2.0 error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// MCP-specific errors
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Transport error: {0}")]
    TransportError(String),

    #[error("JSON-RPC error {code}: {message}")]
    RpcError { code: i32, message: String },

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Empty response")]
    EmptyResponse,

    #[error("Request timeout")]
    Timeout,

    #[error("Server disconnected")]
    Disconnected,

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("Not initialized")]
    NotInitialized,

    #[error("IO error: {0}")]
    IoError(String),
}

impl From<std::io::Error> for McpError {
    fn from(e: std::io::Error) -> Self {
        McpError::IoError(e.to_string())
    }
}

impl From<serde_json::Error> for McpError {
    fn from(e: serde_json::Error) -> Self {
        McpError::ParseError(e.to_string())
    }
}

/// Standard JSON-RPC error codes
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
}

/// Parse a JSON-RPC message from a line of text
pub fn parse_message(line: &str) -> Result<JsonRpcMessage, McpError> {
    let value: Value = serde_json::from_str(line)?;

    // Check if it's a response (has id and result/error)
    if value.get("id").is_some() && (value.get("result").is_some() || value.get("error").is_some())
    {
        let response: JsonRpcResponse = serde_json::from_value(value)?;
        return Ok(JsonRpcMessage::Response(response));
    }

    // Check if it's a notification (has method but no id)
    if value.get("method").is_some() && value.get("id").is_none() {
        let notification: JsonRpcNotification = serde_json::from_value(value)?;
        return Ok(JsonRpcMessage::Notification(notification));
    }

    // Check if it's a request (has method and id)
    if value.get("method").is_some() && value.get("id").is_some() {
        let request: JsonRpcRequest = serde_json::from_value(value)?;
        return Ok(JsonRpcMessage::Request(request));
    }

    Err(McpError::ParseError("Unknown message type".to_string()))
}

/// A parsed JSON-RPC message
#[derive(Debug)]
pub enum JsonRpcMessage {
    Request(JsonRpcRequest),
    Response(JsonRpcResponse),
    Notification(JsonRpcNotification),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let request = JsonRpcRequest::new("test/method", Some(serde_json::json!({"key": "value"})));
        let json = request.to_json_line().unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"test/method\""));
        assert!(json.ends_with('\n'));
    }

    #[test]
    fn test_initialize_request() {
        let params = crate::types::InitializeParams::default();
        let request = JsonRpcRequest::initialize(&params);
        assert_eq!(request.method, "initialize");
    }

    #[test]
    fn test_list_resources_request() {
        let request = JsonRpcRequest::list_resources(None);
        assert_eq!(request.method, "resources/list");
    }

    #[test]
    fn test_read_resource_request() {
        let request = JsonRpcRequest::read_resource("file:///test.txt");
        assert_eq!(request.method, "resources/read");
        let params = request.params.unwrap();
        assert_eq!(params["uri"], "file:///test.txt");
    }

    #[test]
    fn test_parse_response() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"key":"value"}}"#;
        let msg = parse_message(json).unwrap();
        match msg {
            JsonRpcMessage::Response(resp) => {
                assert_eq!(resp.id, Some(1));
                assert!(resp.result.is_some());
            }
            _ => panic!("Expected response"),
        }
    }

    #[test]
    fn test_parse_error_response() {
        let json =
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"Invalid Request"}}"#;
        let msg = parse_message(json).unwrap();
        match msg {
            JsonRpcMessage::Response(resp) => {
                assert!(resp.is_error());
                let err = resp.into_result().unwrap_err();
                assert!(matches!(err, McpError::RpcError { code: -32600, .. }));
            }
            _ => panic!("Expected response"),
        }
    }

    #[test]
    fn test_parse_notification() {
        let json = r#"{"jsonrpc":"2.0","method":"notifications/message","params":{}}"#;
        let msg = parse_message(json).unwrap();
        assert!(matches!(msg, JsonRpcMessage::Notification(_)));
    }
}
