//! Jujutsu (jj) version control wrapper for Rust
//!
//! A type-safe interface to the jj CLI.
//!
//! # Example
//!
//! ```no_run
//! use jj_cli::Jj;
//!
//! let jj = Jj::new()?;
//!
//! // Get status
//! let status = jj.status()?;
//!
//! // View log
//! let log = jj.log(None)?;
//!
//! // Create new change
//! jj.new_change()?;
//! # Ok::<(), jj_cli::Error>(())
//! ```

use std::path::PathBuf;
use std::process::Command;
use thiserror::Error;

/// Errors that can occur when interacting with jj
#[derive(Error, Debug)]
pub enum Error {
    #[error("jj is not installed or not in PATH")]
    NotInstalled,

    #[error("Not in a jj repository")]
    NotInRepo,

    #[error("Failed to execute jj command: {0}")]
    CommandFailed(String),

    #[error("Failed to parse output: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for jj operations
pub type Result<T> = std::result::Result<T, Error>;

/// Output from a jj command
#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Status of a file in jj
#[derive(Debug, Clone)]
pub struct FileStatus {
    pub path: PathBuf,
    pub status: char, // M, A, D, R, etc.
}

/// A change/commit in jj
#[derive(Debug, Clone)]
pub struct Change {
    pub change_id: String,
    pub commit_id: String,
    pub description: String,
    pub author: String,
    pub timestamp: String,
}

/// Jujutsu CLI wrapper
#[derive(Debug, Clone, Default)]
pub struct Jj {
    /// Working directory
    workdir: Option<PathBuf>,
}

impl Jj {
    /// Create a new Jj instance
    pub fn new() -> Result<Self> {
        let jj = Self::default();
        if !jj.is_available() {
            return Err(Error::NotInstalled);
        }
        Ok(jj)
    }

    /// Create with a specific working directory
    pub fn with_workdir(path: impl Into<PathBuf>) -> Self {
        Self {
            workdir: Some(path.into()),
        }
    }

    /// Check if jj is available
    pub fn is_available(&self) -> bool {
        self.run_command(&["--version"]).is_ok()
    }

    /// Get working copy status
    pub fn status(&self) -> Result<CommandOutput> {
        self.run_command(&["status"])
    }

    /// View change log
    pub fn log(&self, limit: Option<usize>) -> Result<CommandOutput> {
        match limit {
            Some(n) => self.run_command(&["log", "-n", &n.to_string()]),
            None => self.run_command(&["log"]),
        }
    }

    /// Show diff
    pub fn diff(&self, revision: Option<&str>) -> Result<CommandOutput> {
        match revision {
            Some(rev) => self.run_command(&["diff", "-r", rev]),
            None => self.run_command(&["diff"]),
        }
    }

    /// Update change description
    pub fn describe(&self, message: &str) -> Result<CommandOutput> {
        self.run_command(&["describe", "-m", message])
    }

    /// Create new change
    pub fn new_change(&self) -> Result<CommandOutput> {
        self.run_command(&["new"])
    }

    /// Undo last operation
    pub fn undo(&self) -> Result<CommandOutput> {
        self.run_command(&["undo"])
    }

    /// Squash changes
    pub fn squash(&self) -> Result<CommandOutput> {
        self.run_command(&["squash"])
    }

    /// Abandon a change
    pub fn abandon(&self, revision: Option<&str>) -> Result<CommandOutput> {
        match revision {
            Some(rev) => self.run_command(&["abandon", rev]),
            None => self.run_command(&["abandon"]),
        }
    }

    /// Git push
    pub fn git_push(&self) -> Result<CommandOutput> {
        self.run_command(&["git", "push"])
    }

    /// Git fetch
    pub fn git_fetch(&self) -> Result<CommandOutput> {
        self.run_command(&["git", "fetch"])
    }

    /// List bookmarks (branches)
    pub fn bookmark_list(&self) -> Result<CommandOutput> {
        self.run_command(&["bookmark", "list"])
    }

    // --- Private helpers ---

    fn run_command(&self, args: &[&str]) -> Result<CommandOutput> {
        let mut cmd = Command::new("jj");
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
    fn test_jj_available() {
        // This test only passes if jj is installed
        if let Ok(jj) = Jj::new() {
            assert!(jj.is_available());
        }
    }
}
