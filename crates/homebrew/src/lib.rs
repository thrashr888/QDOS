//! Homebrew package manager wrapper for Rust
//!
//! A type-safe interface to the Homebrew CLI.
//!
//! # Example
//!
//! ```no_run
//! use homebrew::Homebrew;
//!
//! let brew = Homebrew::new()?;
//!
//! // List installed packages
//! let installed = brew.list()?;
//!
//! // Search for packages
//! let results = brew.search("ripgrep")?;
//!
//! // Get package info
//! let info = brew.info("ripgrep")?;
//!
//! // Install a package
//! brew.install("fd")?;
//! # Ok::<(), homebrew::Error>(())
//! ```

use serde::{Deserialize, Serialize};
use std::process::Command;
use thiserror::Error;

/// Errors that can occur when interacting with Homebrew
#[derive(Error, Debug)]
pub enum Error {
    #[error("Homebrew is not installed or not in PATH")]
    NotInstalled,

    #[error("Failed to execute brew command: {0}")]
    CommandFailed(String),

    #[error("Failed to parse output: {0}")]
    ParseError(String),

    #[error("Package not found: {0}")]
    PackageNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Result type for Homebrew operations
pub type Result<T> = std::result::Result<T, Error>;

/// Information about a Homebrew package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: String,
    pub homepage: String,
    pub installed: bool,
    pub installed_version: Option<String>,
    pub outdated: bool,
}

/// Simplified package info from list command
#[derive(Debug, Clone)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
}

/// Search result
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub name: String,
    pub is_cask: bool,
}

/// Output from a brew command
#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Homebrew CLI wrapper
#[derive(Debug, Clone, Default)]
pub struct Homebrew {
    /// Custom brew path (uses PATH if None)
    brew_path: Option<String>,
}

impl Homebrew {
    /// Create a new Homebrew instance
    pub fn new() -> Result<Self> {
        let brew = Self::default();
        if !brew.is_available() {
            return Err(Error::NotInstalled);
        }
        Ok(brew)
    }

    /// Create with a custom brew path
    pub fn with_path(path: impl Into<String>) -> Self {
        Self {
            brew_path: Some(path.into()),
        }
    }

    /// Check if Homebrew is available
    pub fn is_available(&self) -> bool {
        self.run_command(&["--version"]).is_ok()
    }

    /// List installed packages
    pub fn list(&self) -> Result<Vec<InstalledPackage>> {
        let output = self.run_command(&["list", "--versions"])?;
        Ok(output
            .stdout
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    Some(InstalledPackage {
                        name: parts[0].to_string(),
                        version: parts[1].to_string(),
                    })
                } else {
                    None
                }
            })
            .collect())
    }

    /// Search for packages
    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        let output = self.run_command(&["search", query])?;
        Ok(output
            .stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| {
                let is_cask = line.contains("cask");
                SearchResult {
                    name: line.trim().to_string(),
                    is_cask,
                }
            })
            .collect())
    }

    /// Get detailed info about a package
    pub fn info(&self, name: &str) -> Result<Package> {
        let output = self.run_command(&["info", "--json=v2", name])?;
        self.parse_info_json(&output.stdout)
    }

    /// Get list of outdated packages
    pub fn outdated(&self) -> Result<Vec<String>> {
        let output = self.run_command(&["outdated", "--formula"])?;
        Ok(output
            .stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(|s| s.to_string())
            .collect())
    }

    /// Install a package
    pub fn install(&self, name: &str) -> Result<CommandOutput> {
        self.run_command(&["install", name])
    }

    /// Uninstall a package
    pub fn uninstall(&self, name: &str) -> Result<CommandOutput> {
        self.run_command(&["uninstall", name])
    }

    /// Update Homebrew package list
    pub fn update(&self) -> Result<CommandOutput> {
        self.run_command(&["update"])
    }

    /// Upgrade a specific package or all packages
    pub fn upgrade(&self, name: Option<&str>) -> Result<CommandOutput> {
        match name {
            Some(pkg) => self.run_command(&["upgrade", pkg]),
            None => self.run_command(&["upgrade"]),
        }
    }

    /// Open package homepage in browser
    pub fn home(&self, name: &str) -> Result<()> {
        Command::new(self.brew_cmd())
            .args(["home", name])
            .spawn()?;
        Ok(())
    }

    /// Add a tap
    pub fn tap(&self, name: &str) -> Result<CommandOutput> {
        self.run_command(&["tap", name])
    }

    /// List taps
    pub fn list_taps(&self) -> Result<Vec<String>> {
        let output = self.run_command(&["tap"])?;
        Ok(output
            .stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(|s| s.to_string())
            .collect())
    }

    // --- Private helpers ---

    fn brew_cmd(&self) -> &str {
        self.brew_path.as_deref().unwrap_or("brew")
    }

    fn run_command(&self, args: &[&str]) -> Result<CommandOutput> {
        let output = Command::new(self.brew_cmd()).args(args).output()?;

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

    fn parse_info_json(&self, json: &str) -> Result<Package> {
        let value: serde_json::Value = serde_json::from_str(json)?;

        // Handle both formulae and casks
        let formulae = value.get("formulae").and_then(|f| f.as_array());
        let casks = value.get("casks").and_then(|c| c.as_array());

        let pkg = formulae
            .and_then(|f| f.first())
            .or_else(|| casks.and_then(|c| c.first()))
            .ok_or_else(|| Error::ParseError("No package info found".to_string()))?;

        let name = pkg
            .get("name")
            .or_else(|| pkg.get("token"))
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();

        let version = pkg
            .get("versions")
            .and_then(|v| v.get("stable"))
            .and_then(|s| s.as_str())
            .or_else(|| pkg.get("version").and_then(|v| v.as_str()))
            .unwrap_or("")
            .to_string();

        let description = pkg
            .get("desc")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();

        let homepage = pkg
            .get("homepage")
            .and_then(|h| h.as_str())
            .unwrap_or("")
            .to_string();

        let installed = pkg
            .get("installed")
            .map(|i| !i.as_array().is_none_or(|a| a.is_empty()))
            .unwrap_or(false);

        let installed_version = pkg
            .get("installed")
            .and_then(|i| i.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("version"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let outdated = pkg
            .get("outdated")
            .and_then(|o| o.as_bool())
            .unwrap_or(false);

        Ok(Package {
            name,
            version,
            description,
            homepage,
            installed,
            installed_version,
            outdated,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_homebrew_available() {
        // This test only passes if Homebrew is installed
        if let Ok(brew) = Homebrew::new() {
            assert!(brew.is_available());
        }
    }
}
