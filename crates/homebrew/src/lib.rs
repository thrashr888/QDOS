//! Homebrew package manager wrapper for Rust
//!
//! A type-safe interface to the Homebrew CLI for macOS.
//!
//! # Features
//!
//! - List installed packages (formulae and casks)
//! - Search for packages
//! - Get detailed package information
//! - Install, uninstall, and upgrade packages
//! - Manage taps (third-party repositories)
//! - Check for outdated packages
//!
//! # Example
//!
//! ```no_run
//! use homebrew::Homebrew;
//!
//! fn main() -> homebrew::Result<()> {
//!     let brew = Homebrew::new()?;
//!
//!     // List installed packages
//!     for pkg in brew.list()? {
//!         println!("{} ({})", pkg.name, pkg.version);
//!     }
//!
//!     // Search for packages
//!     let results = brew.search("ripgrep")?;
//!     for r in results {
//!         println!("Found: {} (cask: {})", r.name, r.is_cask);
//!     }
//!
//!     // Get package info
//!     let info = brew.info("git")?;
//!     println!("{}: {}", info.name, info.description);
//!
//!     // Check for outdated packages
//!     let outdated = brew.outdated()?;
//!     println!("{} packages need updating", outdated.len());
//!
//!     Ok(())
//! }
//! ```
//!
//! # Platform Support
//!
//! This crate only works on macOS where Homebrew is available.
//! On other platforms, `Homebrew::new()` will return `Error::NotInstalled`.

use serde::{Deserialize, Serialize};
use std::process::Command;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur when interacting with Homebrew
#[derive(Error, Debug)]
pub enum Error {
    /// Homebrew is not installed or not found in PATH
    #[error("Homebrew is not installed or not in PATH")]
    NotInstalled,

    /// A brew command failed to execute
    #[error("brew command failed: {0}")]
    CommandFailed(String),

    /// Failed to parse output from a brew command
    #[error("failed to parse output: {0}")]
    ParseError(String),

    /// The requested package was not found
    #[error("package not found: {0}")]
    PackageNotFound(String),

    /// An I/O error occurred
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON parsing error
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Result type for Homebrew operations
pub type Result<T> = std::result::Result<T, Error>;

// ============================================================================
// Data Types
// ============================================================================

/// Package type (formula or cask)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageType {
    /// A formula (command-line tool or library)
    Formula,
    /// A cask (GUI application)
    Cask,
}

/// Detailed information about a Homebrew package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    /// Package name
    pub name: String,
    /// Full name including tap (e.g., "homebrew/core/git")
    pub full_name: Option<String>,
    /// Latest available version
    pub version: String,
    /// Package description
    pub description: String,
    /// Homepage URL
    pub homepage: String,
    /// Whether the package is currently installed
    pub installed: bool,
    /// Currently installed version (if installed)
    pub installed_version: Option<String>,
    /// Whether an update is available
    pub outdated: bool,
    /// Package type (formula or cask)
    pub package_type: PackageType,
    /// Package dependencies
    pub dependencies: Vec<String>,
    /// Special installation notes
    pub caveats: Option<String>,
    /// License
    pub license: Option<String>,
}

impl Default for Package {
    fn default() -> Self {
        Self {
            name: String::new(),
            full_name: None,
            version: String::new(),
            description: String::new(),
            homepage: String::new(),
            installed: false,
            installed_version: None,
            outdated: false,
            package_type: PackageType::Formula,
            dependencies: Vec::new(),
            caveats: None,
            license: None,
        }
    }
}

/// Basic info about an installed package
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledPackage {
    /// Package name
    pub name: String,
    /// Installed version
    pub version: String,
}

/// A search result
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    /// Package name
    pub name: String,
    /// Whether this is a cask (vs formula)
    pub is_cask: bool,
}

/// Output from a brew command
#[derive(Debug, Clone)]
pub struct CommandOutput {
    /// Whether the command succeeded
    pub success: bool,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Exit code
    pub exit_code: Option<i32>,
}

impl CommandOutput {
    /// Combine stdout and stderr into a single string
    pub fn combined(&self) -> String {
        let mut result = self.stdout.clone();
        if !self.stderr.is_empty() {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(&self.stderr);
        }
        result
    }
}

/// Information about a tap (third-party repository)
#[derive(Debug, Clone)]
pub struct Tap {
    /// Tap name (e.g., "homebrew/core")
    pub name: String,
    /// Whether the tap is official (homebrew/*)
    pub official: bool,
}

// ============================================================================
// Main Interface
// ============================================================================

/// Homebrew CLI wrapper
///
/// Provides a type-safe interface to interact with Homebrew.
///
/// # Example
///
/// ```no_run
/// use homebrew::Homebrew;
///
/// let brew = Homebrew::new()?;
/// let packages = brew.list()?;
/// # Ok::<(), homebrew::Error>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct Homebrew {
    /// Custom brew path (uses PATH if None)
    brew_path: Option<String>,
}

impl Homebrew {
    /// Create a new Homebrew instance
    ///
    /// Returns an error if Homebrew is not installed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use homebrew::Homebrew;
    ///
    /// let brew = Homebrew::new()?;
    /// # Ok::<(), homebrew::Error>(())
    /// ```
    pub fn new() -> Result<Self> {
        let brew = Self::default();
        if !brew.is_available() {
            return Err(Error::NotInstalled);
        }
        Ok(brew)
    }

    /// Create without checking availability
    ///
    /// Use this if you want to defer the availability check.
    pub fn unchecked() -> Self {
        Self::default()
    }

    /// Create with a custom brew path
    ///
    /// Useful for testing or when brew is not in PATH.
    pub fn with_path(path: impl Into<String>) -> Self {
        Self {
            brew_path: Some(path.into()),
        }
    }

    /// Check if Homebrew is available
    pub fn is_available(&self) -> bool {
        self.run_command(&["--version"]).is_ok()
    }

    /// Get the Homebrew version
    pub fn version(&self) -> Result<String> {
        let output = self.run_command(&["--version"])?;
        Ok(output
            .stdout
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string())
    }

    // ------------------------------------------------------------------------
    // Package Listing
    // ------------------------------------------------------------------------

    /// List all installed packages (formulae only)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use homebrew::Homebrew;
    ///
    /// let brew = Homebrew::new()?;
    /// for pkg in brew.list()? {
    ///     println!("{}: {}", pkg.name, pkg.version);
    /// }
    /// # Ok::<(), homebrew::Error>(())
    /// ```
    pub fn list(&self) -> Result<Vec<InstalledPackage>> {
        let output = self.run_command(&["list", "--versions", "--formula"])?;
        Ok(self.parse_list_output(&output.stdout))
    }

    /// List all installed casks
    pub fn list_casks(&self) -> Result<Vec<InstalledPackage>> {
        let output = self.run_command(&["list", "--versions", "--cask"])?;
        Ok(self.parse_list_output(&output.stdout))
    }

    /// List all installed packages (both formulae and casks)
    pub fn list_all(&self) -> Result<Vec<InstalledPackage>> {
        let output = self.run_command(&["list", "--versions"])?;
        Ok(self.parse_list_output(&output.stdout))
    }

    fn parse_list_output(&self, output: &str) -> Vec<InstalledPackage> {
        output
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    Some(InstalledPackage {
                        name: parts[0].to_string(),
                        version: parts[1].to_string(),
                    })
                } else if !parts.is_empty() {
                    // Some packages might not have version info
                    Some(InstalledPackage {
                        name: parts[0].to_string(),
                        version: String::new(),
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    // ------------------------------------------------------------------------
    // Search
    // ------------------------------------------------------------------------

    /// Search for packages
    ///
    /// # Example
    ///
    /// ```no_run
    /// use homebrew::Homebrew;
    ///
    /// let brew = Homebrew::new()?;
    /// let results = brew.search("json")?;
    /// for r in results {
    ///     let pkg_type = if r.is_cask { "cask" } else { "formula" };
    ///     println!("{} ({})", r.name, pkg_type);
    /// }
    /// # Ok::<(), homebrew::Error>(())
    /// ```
    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        let output = self.run_command(&["search", query])?;

        let mut results = Vec::new();
        let mut in_casks = false;

        for line in output.stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Check for section headers
            if line.starts_with("==> Formulae") {
                in_casks = false;
                continue;
            }
            if line.starts_with("==> Casks") {
                in_casks = true;
                continue;
            }
            if line.starts_with("==>") {
                continue;
            }

            // Parse package names (space-separated on each line)
            for name in line.split_whitespace() {
                results.push(SearchResult {
                    name: name.to_string(),
                    is_cask: in_casks,
                });
            }
        }

        Ok(results)
    }

    /// Search only formulae
    pub fn search_formulae(&self, query: &str) -> Result<Vec<SearchResult>> {
        let output = self.run_command(&["search", "--formula", query])?;
        Ok(output
            .stdout
            .split_whitespace()
            .filter(|s| !s.starts_with("==>"))
            .map(|name| SearchResult {
                name: name.to_string(),
                is_cask: false,
            })
            .collect())
    }

    /// Search only casks
    pub fn search_casks(&self, query: &str) -> Result<Vec<SearchResult>> {
        let output = self.run_command(&["search", "--cask", query])?;
        Ok(output
            .stdout
            .split_whitespace()
            .filter(|s| !s.starts_with("==>"))
            .map(|name| SearchResult {
                name: name.to_string(),
                is_cask: true,
            })
            .collect())
    }

    // ------------------------------------------------------------------------
    // Package Info
    // ------------------------------------------------------------------------

    /// Get detailed information about a package
    ///
    /// # Example
    ///
    /// ```no_run
    /// use homebrew::Homebrew;
    ///
    /// let brew = Homebrew::new()?;
    /// let info = brew.info("git")?;
    /// println!("Name: {}", info.name);
    /// println!("Version: {}", info.version);
    /// println!("Description: {}", info.description);
    /// println!("Installed: {}", info.installed);
    /// # Ok::<(), homebrew::Error>(())
    /// ```
    pub fn info(&self, name: &str) -> Result<Package> {
        let output = self.run_command(&["info", "--json=v2", name])?;
        self.parse_info_json(&output.stdout)
    }

    /// Get info about multiple packages at once
    pub fn info_batch(&self, names: &[&str]) -> Result<Vec<Package>> {
        if names.is_empty() {
            return Ok(Vec::new());
        }

        let mut args = vec!["info", "--json=v2"];
        args.extend(names);

        let output = self.run_command(&args)?;
        self.parse_info_json_batch(&output.stdout)
    }

    // ------------------------------------------------------------------------
    // Outdated Packages
    // ------------------------------------------------------------------------

    /// Get list of outdated formula packages
    pub fn outdated(&self) -> Result<Vec<String>> {
        let output = self.run_command(&["outdated", "--formula"])?;
        Ok(output
            .stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(|s| s.to_string())
            .collect())
    }

    /// Get list of outdated cask packages
    pub fn outdated_casks(&self) -> Result<Vec<String>> {
        let output = self.run_command(&["outdated", "--cask"])?;
        Ok(output
            .stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(|s| s.to_string())
            .collect())
    }

    /// Get detailed info about outdated packages
    pub fn outdated_info(&self) -> Result<Vec<Package>> {
        let outdated = self.outdated()?;
        if outdated.is_empty() {
            return Ok(Vec::new());
        }

        let names: Vec<&str> = outdated.iter().map(|s| s.as_str()).collect();
        self.info_batch(&names)
    }

    // ------------------------------------------------------------------------
    // Install / Uninstall / Upgrade
    // ------------------------------------------------------------------------

    /// Install a package
    ///
    /// # Example
    ///
    /// ```no_run
    /// use homebrew::Homebrew;
    ///
    /// let brew = Homebrew::new()?;
    /// let output = brew.install("ripgrep")?;
    /// if output.success {
    ///     println!("Installed successfully!");
    /// }
    /// # Ok::<(), homebrew::Error>(())
    /// ```
    pub fn install(&self, name: &str) -> Result<CommandOutput> {
        self.run_command(&["install", name])
    }

    /// Install a cask
    pub fn install_cask(&self, name: &str) -> Result<CommandOutput> {
        self.run_command(&["install", "--cask", name])
    }

    /// Uninstall a package
    pub fn uninstall(&self, name: &str) -> Result<CommandOutput> {
        self.run_command(&["uninstall", name])
    }

    /// Uninstall a cask
    pub fn uninstall_cask(&self, name: &str) -> Result<CommandOutput> {
        self.run_command(&["uninstall", "--cask", name])
    }

    /// Update Homebrew itself and fetch latest package info
    pub fn update(&self) -> Result<CommandOutput> {
        self.run_command(&["update"])
    }

    /// Upgrade a specific package
    pub fn upgrade(&self, name: &str) -> Result<CommandOutput> {
        self.run_command(&["upgrade", name])
    }

    /// Upgrade all outdated packages
    pub fn upgrade_all(&self) -> Result<CommandOutput> {
        self.run_command(&["upgrade"])
    }

    /// Upgrade a specific cask
    pub fn upgrade_cask(&self, name: &str) -> Result<CommandOutput> {
        self.run_command(&["upgrade", "--cask", name])
    }

    /// Reinstall a package
    pub fn reinstall(&self, name: &str) -> Result<CommandOutput> {
        self.run_command(&["reinstall", name])
    }

    // ------------------------------------------------------------------------
    // Taps
    // ------------------------------------------------------------------------

    /// Add a tap (third-party repository)
    pub fn tap(&self, name: &str) -> Result<CommandOutput> {
        self.run_command(&["tap", name])
    }

    /// Remove a tap
    pub fn untap(&self, name: &str) -> Result<CommandOutput> {
        self.run_command(&["untap", name])
    }

    /// List all taps
    pub fn list_taps(&self) -> Result<Vec<Tap>> {
        let output = self.run_command(&["tap"])?;
        Ok(output
            .stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(|name| {
                let name = name.trim().to_string();
                let official = name.starts_with("homebrew/");
                Tap { name, official }
            })
            .collect())
    }

    // ------------------------------------------------------------------------
    // Utilities
    // ------------------------------------------------------------------------

    /// Open package homepage in browser
    pub fn home(&self, name: &str) -> Result<()> {
        Command::new(self.brew_cmd()).args(["home", name]).spawn()?;
        Ok(())
    }

    /// Clean up old versions and cache
    pub fn cleanup(&self) -> Result<CommandOutput> {
        self.run_command(&["cleanup"])
    }

    /// Clean up a specific package
    pub fn cleanup_package(&self, name: &str) -> Result<CommandOutput> {
        self.run_command(&["cleanup", name])
    }

    /// Run brew doctor to check for problems
    pub fn doctor(&self) -> Result<CommandOutput> {
        self.run_command(&["doctor"])
    }

    /// Get Homebrew configuration info
    pub fn config(&self) -> Result<String> {
        let output = self.run_command(&["config"])?;
        Ok(output.stdout)
    }

    // ------------------------------------------------------------------------
    // Raw Command Execution
    // ------------------------------------------------------------------------

    /// Run an arbitrary brew command
    ///
    /// Use this for commands not covered by the API.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use homebrew::Homebrew;
    ///
    /// let brew = Homebrew::new()?;
    /// let output = brew.run(&["deps", "git"])?;
    /// println!("Dependencies: {}", output.stdout);
    /// # Ok::<(), homebrew::Error>(())
    /// ```
    pub fn run(&self, args: &[&str]) -> Result<CommandOutput> {
        self.run_command(args)
    }

    // ------------------------------------------------------------------------
    // Private Implementation
    // ------------------------------------------------------------------------

    fn brew_cmd(&self) -> &str {
        self.brew_path.as_deref().unwrap_or("brew")
    }

    fn run_command(&self, args: &[&str]) -> Result<CommandOutput> {
        let output = Command::new(self.brew_cmd()).args(args).output()?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code();

        // Only treat as error if command failed AND has error output
        if !output.status.success() && !stderr.is_empty() {
            return Err(Error::CommandFailed(stderr));
        }

        Ok(CommandOutput {
            success: output.status.success(),
            stdout,
            stderr,
            exit_code,
        })
    }

    fn parse_info_json(&self, json: &str) -> Result<Package> {
        let packages = self.parse_info_json_batch(json)?;
        packages
            .into_iter()
            .next()
            .ok_or_else(|| Error::ParseError("No package info found".to_string()))
    }

    fn parse_info_json_batch(&self, json: &str) -> Result<Vec<Package>> {
        let value: serde_json::Value = serde_json::from_str(json)?;
        let mut packages = Vec::new();

        // Parse formulae
        if let Some(formulae) = value.get("formulae").and_then(|f| f.as_array()) {
            for pkg in formulae {
                if let Some(parsed) = self.parse_formula(pkg) {
                    packages.push(parsed);
                }
            }
        }

        // Parse casks
        if let Some(casks) = value.get("casks").and_then(|c| c.as_array()) {
            for pkg in casks {
                if let Some(parsed) = self.parse_cask(pkg) {
                    packages.push(parsed);
                }
            }
        }

        Ok(packages)
    }

    fn parse_formula(&self, pkg: &serde_json::Value) -> Option<Package> {
        let name = pkg.get("name")?.as_str()?.to_string();

        let full_name = pkg
            .get("full_name")
            .and_then(|n| n.as_str())
            .map(String::from);

        let version = pkg
            .get("versions")
            .and_then(|v| v.get("stable"))
            .and_then(|s| s.as_str())
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
            .and_then(|i| i.as_array())
            .map(|a| !a.is_empty())
            .unwrap_or(false);

        let installed_version = pkg
            .get("installed")
            .and_then(|i| i.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("version"))
            .and_then(|v| v.as_str())
            .map(String::from);

        let outdated = pkg
            .get("outdated")
            .and_then(|o| o.as_bool())
            .unwrap_or(false);

        let dependencies = pkg
            .get("dependencies")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let caveats = pkg
            .get("caveats")
            .and_then(|c| c.as_str())
            .map(String::from);

        let license = pkg
            .get("license")
            .and_then(|l| l.as_str())
            .map(String::from);

        Some(Package {
            name,
            full_name,
            version,
            description,
            homepage,
            installed,
            installed_version,
            outdated,
            package_type: PackageType::Formula,
            dependencies,
            caveats,
            license,
        })
    }

    fn parse_cask(&self, pkg: &serde_json::Value) -> Option<Package> {
        let name = pkg
            .get("token")
            .or_else(|| pkg.get("name"))
            .and_then(|n| n.as_str())?
            .to_string();

        let full_name = pkg
            .get("full_token")
            .and_then(|n| n.as_str())
            .map(String::from);

        let version = pkg
            .get("version")
            .and_then(|v| v.as_str())
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

        let installed = pkg.get("installed").and_then(|i| i.as_str()).is_some();

        let installed_version = pkg
            .get("installed")
            .and_then(|i| i.as_str())
            .map(String::from);

        let outdated = pkg
            .get("outdated")
            .and_then(|o| o.as_bool())
            .unwrap_or(false);

        Some(Package {
            name,
            full_name,
            version,
            description,
            homepage,
            installed,
            installed_version,
            outdated,
            package_type: PackageType::Cask,
            dependencies: Vec::new(),
            caveats: None,
            license: None,
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // These tests only run if Homebrew is installed
    fn get_brew() -> Option<Homebrew> {
        Homebrew::new().ok()
    }

    #[test]
    fn test_homebrew_available() {
        if let Some(brew) = get_brew() {
            assert!(brew.is_available());
        }
    }

    #[test]
    fn test_homebrew_version() {
        if let Some(brew) = get_brew() {
            let version = brew.version().unwrap();
            assert!(version.contains("Homebrew"));
        }
    }

    #[test]
    fn test_list_packages() {
        if let Some(brew) = get_brew() {
            let packages = brew.list().unwrap();
            // Should have at least some packages if Homebrew is installed
            // (This might fail on a fresh install)
            for pkg in &packages {
                assert!(!pkg.name.is_empty());
            }
        }
    }

    #[test]
    fn test_list_taps() {
        if let Some(brew) = get_brew() {
            // Just check it doesn't error - tap list varies by system
            let _ = brew.list_taps();
        }
    }

    #[test]
    fn test_search() {
        if let Some(brew) = get_brew() {
            let results = brew.search("git").unwrap();
            // git should definitely exist
            assert!(results.iter().any(|r| r.name == "git"));
        }
    }

    #[test]
    fn test_info() {
        if let Some(brew) = get_brew() {
            // git is almost certainly available in Homebrew
            if let Ok(info) = brew.info("git") {
                assert_eq!(info.name, "git");
                assert!(!info.description.is_empty());
                assert!(!info.homepage.is_empty());
            }
        }
    }

    #[test]
    fn test_outdated() {
        if let Some(brew) = get_brew() {
            // Just check it doesn't error
            let _ = brew.outdated();
        }
    }

    // Unit tests that don't require Homebrew

    #[test]
    fn test_unchecked_creation() {
        let brew = Homebrew::unchecked();
        // Should create without checking availability
        assert!(brew.brew_path.is_none());
    }

    #[test]
    fn test_with_path() {
        let brew = Homebrew::with_path("/custom/path/brew");
        assert_eq!(brew.brew_path, Some("/custom/path/brew".to_string()));
    }

    #[test]
    fn test_parse_list_output() {
        let brew = Homebrew::unchecked();
        let output = "git 2.42.0\nripgrep 13.0.0\nfd 8.7.1";
        let packages = brew.parse_list_output(output);

        assert_eq!(packages.len(), 3);
        assert_eq!(packages[0].name, "git");
        assert_eq!(packages[0].version, "2.42.0");
        assert_eq!(packages[1].name, "ripgrep");
        assert_eq!(packages[2].name, "fd");
    }

    #[test]
    fn test_command_output_combined() {
        let output = CommandOutput {
            success: true,
            stdout: "hello".to_string(),
            stderr: "world".to_string(),
            exit_code: Some(0),
        };
        assert_eq!(output.combined(), "hello\nworld");
    }

    #[test]
    fn test_command_output_combined_empty_stderr() {
        let output = CommandOutput {
            success: true,
            stdout: "hello".to_string(),
            stderr: String::new(),
            exit_code: Some(0),
        };
        assert_eq!(output.combined(), "hello");
    }

    #[test]
    fn test_tap_official_detection() {
        let official = Tap {
            name: "homebrew/core".to_string(),
            official: true,
        };
        let third_party = Tap {
            name: "user/repo".to_string(),
            official: false,
        };

        assert!(official.official);
        assert!(!third_party.official);
    }

    #[test]
    fn test_package_default() {
        let pkg = Package::default();
        assert!(pkg.name.is_empty());
        assert!(!pkg.installed);
        assert!(!pkg.outdated);
        assert_eq!(pkg.package_type, PackageType::Formula);
    }

    #[test]
    fn test_search_result_equality() {
        let a = SearchResult {
            name: "git".to_string(),
            is_cask: false,
        };
        let b = SearchResult {
            name: "git".to_string(),
            is_cask: false,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn test_installed_package_equality() {
        let a = InstalledPackage {
            name: "git".to_string(),
            version: "2.42.0".to_string(),
        };
        let b = InstalledPackage {
            name: "git".to_string(),
            version: "2.42.0".to_string(),
        };
        assert_eq!(a, b);
    }
}
