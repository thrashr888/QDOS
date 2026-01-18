// Allow dead code - many types/functions are for future P3 features (MCP Tools menu, etc.)
#![allow(dead_code)]

//! MCP (Model Context Protocol) client implementation
//!
//! This module provides a client for communicating with MCP servers,
//! enabling R-DOS to browse remote filesystems and call tools.
//!
//! # Overview
//!
//! MCP is a protocol for connecting AI assistants to external tools and data sources.
//! This implementation focuses on the client side, allowing R-DOS to:
//!
//! - Connect to MCP servers via stdio transport
//! - List and read resources (files, directories, data)
//! - List and call tools
//!
//! # Example
//!
//! ```ignore
//! use qdos_mcp::{McpClientBuilder, McpError};
//!
//! // Connect to a filesystem MCP server
//! let mut client = McpClientBuilder::new("npx")
//!     .args(vec!["-y".into(), "@modelcontextprotocol/server-filesystem".into(), "/tmp".into()])
//!     .build()?;
//!
//! // List resources
//! let resources = client.list_resources()?;
//! for resource in resources {
//!     println!("{}: {}", resource.name, resource.uri);
//! }
//!
//! // Read a specific resource
//! let result = client.read_resource("file:///tmp/test.txt")?;
//! for content in result.contents {
//!     if let Some(text) = content.text {
//!         println!("{}", text);
//!     }
//! }
//! ```
//!
//! # Architecture
//!
//! - `types`: Core MCP data types (Resource, Tool, Content, etc.)
//! - `protocol`: JSON-RPC 2.0 message serialization
//! - `transport`: Stdio transport for child process communication
//! - `client`: High-level client API

pub mod client;
pub mod protocol;
pub mod transport;
pub mod types;

// Re-export commonly used types
pub use client::{McpClient, McpClientBuilder};
pub use protocol::McpError;
pub use transport::ServerConfig;
pub use types::*;
