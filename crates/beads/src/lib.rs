//! Beads (bd) issue tracker wrapper for Rust
//!
//! A type-safe interface to the bd CLI.
//!
//! # Example
//!
//! ```no_run
//! use beads_cli::Beads;
//!
//! let bd = Beads::new()?;
//!
//! // List issues
//! let issues = bd.list(None)?;
//!
//! // Show ready issues
//! let ready = bd.ready()?;
//!
//! // Create an issue
//! bd.create("Fix the bug", "bug", Some(2))?;
//! # Ok::<(), beads_cli::Error>(())
//! ```

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use thiserror::Error;

/// Errors that can occur when interacting with beads
#[derive(Error, Debug)]
pub enum Error {
    #[error("bd is not installed or not in PATH")]
    NotInstalled,

    #[error("Not in a beads-enabled repository")]
    NotInRepo,

    #[error("Failed to execute bd command: {0}")]
    CommandFailed(String),

    #[error("Failed to parse output: {0}")]
    ParseError(String),

    #[error("Issue not found: {0}")]
    IssueNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Result type for beads operations
pub type Result<T> = std::result::Result<T, Error>;

/// Issue status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Open,
    InProgress,
    Closed,
}

/// Issue type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueType {
    Bug,
    Feature,
    Task,
    Epic,
    Chore,
}

/// A beads issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: String,
    pub title: String,
    pub status: String,
    #[serde(rename = "type")]
    pub issue_type: String,
    pub priority: Option<u8>,
    pub description: Option<String>,
    pub assignee: Option<String>,
    pub parent: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Output from a bd command
#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Beads CLI wrapper
#[derive(Debug, Clone, Default)]
pub struct Beads {
    /// Working directory
    workdir: Option<PathBuf>,
}

impl Beads {
    /// Create a new Beads instance
    pub fn new() -> Result<Self> {
        let bd = Self::default();
        if !bd.is_available() {
            return Err(Error::NotInstalled);
        }
        Ok(bd)
    }

    /// Create with a specific working directory
    pub fn with_workdir(path: impl Into<PathBuf>) -> Self {
        Self {
            workdir: Some(path.into()),
        }
    }

    /// Check if bd is available
    pub fn is_available(&self) -> bool {
        self.run_command(&["--version"]).is_ok()
    }

    /// List issues
    pub fn list(&self, status: Option<&str>) -> Result<Vec<Issue>> {
        let output = match status {
            Some(s) => self.run_command(&["list", "--status", s, "--json"])?,
            None => self.run_command(&["list", "--json"])?,
        };
        serde_json::from_str(&output.stdout).map_err(Error::from)
    }

    /// Show a specific issue
    pub fn show(&self, id: &str) -> Result<Issue> {
        let output = self.run_command(&["show", id, "--json"])?;
        serde_json::from_str(&output.stdout).map_err(Error::from)
    }

    /// Get ready issues (no blockers)
    pub fn ready(&self) -> Result<Vec<Issue>> {
        let output = self.run_command(&["ready", "--json"])?;
        serde_json::from_str(&output.stdout).map_err(Error::from)
    }

    /// Get blocked issues
    pub fn blocked(&self) -> Result<Vec<Issue>> {
        let output = self.run_command(&["blocked", "--json"])?;
        serde_json::from_str(&output.stdout).map_err(Error::from)
    }

    /// Create a new issue
    pub fn create(
        &self,
        title: &str,
        issue_type: &str,
        priority: Option<u8>,
    ) -> Result<CommandOutput> {
        let mut args = vec!["create", "--title", title, "--type", issue_type];

        let priority_str;
        if let Some(p) = priority {
            priority_str = p.to_string();
            args.extend(["--priority", &priority_str]);
        }

        self.run_command(&args)
    }

    /// Update an issue
    pub fn update(&self, id: &str, status: Option<&str>) -> Result<CommandOutput> {
        let mut args = vec!["update", id];

        if let Some(s) = status {
            args.extend(["--status", s]);
        }

        self.run_command(&args)
    }

    /// Close an issue
    pub fn close(&self, id: &str) -> Result<CommandOutput> {
        self.run_command(&["close", id])
    }

    /// Add a dependency
    pub fn dep_add(&self, issue: &str, depends_on: &str) -> Result<CommandOutput> {
        self.run_command(&["dep", "add", issue, depends_on])
    }

    /// Sync with remote
    pub fn sync(&self) -> Result<CommandOutput> {
        self.run_command(&["sync"])
    }

    /// Get project stats
    pub fn stats(&self) -> Result<CommandOutput> {
        self.run_command(&["stats"])
    }

    // --- Private helpers ---

    fn run_command(&self, args: &[&str]) -> Result<CommandOutput> {
        let mut cmd = Command::new("bd");
        cmd.args(args);

        if let Some(ref dir) = self.workdir {
            cmd.current_dir(dir);
        }

        let output = cmd.output()?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() && !stderr.is_empty() {
            return Err(Error::CommandFailed(stderr));
        }

        Ok(CommandOutput {
            success: output.status.success(),
            stdout,
            stderr,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beads_available() {
        // This test only passes if bd is installed
        if let Ok(bd) = Beads::new() {
            assert!(bd.is_available());
        }
    }
}
