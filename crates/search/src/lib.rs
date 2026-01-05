//! Unified search tools wrapper for Rust
//!
//! A unified interface to search tools: ripgrep (rg), silver searcher (ag), grep, and ack.
//!
//! # Example
//!
//! ```no_run
//! use search_tools::{Search, SearchTool};
//!
//! // Auto-detect best available tool
//! let search = Search::auto()?;
//!
//! // Or use a specific tool
//! let search = Search::new(SearchTool::Ripgrep)?;
//!
//! // Search for a pattern
//! let results = search.search("TODO", Some("."))?;
//! # Ok::<(), search_tools::Error>(())
//! ```

use std::process::Command;
use thiserror::Error;

/// Errors that can occur when searching
#[derive(Error, Debug)]
pub enum Error {
    #[error("No search tool available (tried rg, ag, grep, ack)")]
    NoToolAvailable,

    #[error("Search tool not installed: {0}")]
    ToolNotInstalled(String),

    #[error("Search failed: {0}")]
    SearchFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for search operations
pub type Result<T> = std::result::Result<T, Error>;

/// Available search tools
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchTool {
    /// ripgrep (rg) - fastest, recommended
    Ripgrep,
    /// The Silver Searcher (ag)
    Ag,
    /// GNU grep
    Grep,
    /// ack
    Ack,
}

impl SearchTool {
    /// Get the command name for this tool
    pub fn command(&self) -> &'static str {
        match self {
            SearchTool::Ripgrep => "rg",
            SearchTool::Ag => "ag",
            SearchTool::Grep => "grep",
            SearchTool::Ack => "ack",
        }
    }

    /// Check if this tool is available
    pub fn is_available(&self) -> bool {
        Command::new(self.command())
            .arg("--version")
            .output()
            .is_ok()
    }

    /// Get all tools in preference order
    pub fn all() -> &'static [SearchTool] {
        &[
            SearchTool::Ripgrep,
            SearchTool::Ag,
            SearchTool::Ack,
            SearchTool::Grep,
        ]
    }
}

/// A search match
#[derive(Debug, Clone)]
pub struct Match {
    pub file: String,
    pub line_number: Option<usize>,
    pub line: String,
}

/// Search output
#[derive(Debug, Clone)]
pub struct SearchOutput {
    pub tool: SearchTool,
    pub matches: Vec<Match>,
    pub match_count: usize,
}

/// Search options
#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    /// Case insensitive search
    pub ignore_case: bool,
    /// Search hidden files
    pub hidden: bool,
    /// Follow symlinks
    pub follow: bool,
    /// File type filter (e.g., "rust", "py")
    pub file_type: Option<String>,
    /// Glob pattern filter
    pub glob: Option<String>,
    /// Max results
    pub max_count: Option<usize>,
    /// Context lines before match
    pub before_context: Option<usize>,
    /// Context lines after match
    pub after_context: Option<usize>,
}

/// Unified search interface
#[derive(Debug, Clone)]
pub struct Search {
    tool: SearchTool,
}

impl Search {
    /// Create with a specific search tool
    pub fn new(tool: SearchTool) -> Result<Self> {
        if !tool.is_available() {
            return Err(Error::ToolNotInstalled(tool.command().to_string()));
        }
        Ok(Self { tool })
    }

    /// Auto-detect the best available search tool
    pub fn auto() -> Result<Self> {
        for tool in SearchTool::all() {
            if tool.is_available() {
                return Ok(Self { tool: *tool });
            }
        }
        Err(Error::NoToolAvailable)
    }

    /// Get the active search tool
    pub fn tool(&self) -> SearchTool {
        self.tool
    }

    /// Search for a pattern in a path
    pub fn search(&self, pattern: &str, path: Option<&str>) -> Result<SearchOutput> {
        self.search_with_options(pattern, path, &SearchOptions::default())
    }

    /// Search with options
    pub fn search_with_options(
        &self,
        pattern: &str,
        path: Option<&str>,
        options: &SearchOptions,
    ) -> Result<SearchOutput> {
        let mut cmd = Command::new(self.tool.command());

        // Add tool-specific arguments
        self.add_args(&mut cmd, pattern, path, options);

        let output = cmd.output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        let matches = self.parse_output(&stdout);
        let match_count = matches.len();

        Ok(SearchOutput {
            tool: self.tool,
            matches,
            match_count,
        })
    }

    /// Search and return just file names
    pub fn search_files(&self, pattern: &str, path: Option<&str>) -> Result<Vec<String>> {
        let mut cmd = Command::new(self.tool.command());

        match self.tool {
            SearchTool::Ripgrep => {
                cmd.args(["-l", pattern]);
            }
            SearchTool::Ag => {
                cmd.args(["-l", pattern]);
            }
            SearchTool::Grep => {
                cmd.args(["-rl", pattern]);
            }
            SearchTool::Ack => {
                cmd.args(["-l", pattern]);
            }
        }

        if let Some(p) = path {
            cmd.arg(p);
        }

        let output = cmd.output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        Ok(stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(|s| s.to_string())
            .collect())
    }

    // --- Private helpers ---

    fn add_args(
        &self,
        cmd: &mut Command,
        pattern: &str,
        path: Option<&str>,
        options: &SearchOptions,
    ) {
        // Common flags
        if options.ignore_case {
            cmd.arg("-i");
        }

        // Tool-specific flags
        match self.tool {
            SearchTool::Ripgrep => {
                cmd.arg("-n"); // line numbers
                if options.hidden {
                    cmd.arg("--hidden");
                }
                if options.follow {
                    cmd.arg("--follow");
                }
                if let Some(ref ft) = options.file_type {
                    cmd.args(["-t", ft]);
                }
                if let Some(ref g) = options.glob {
                    cmd.args(["-g", g]);
                }
                if let Some(n) = options.max_count {
                    cmd.args(["-m", &n.to_string()]);
                }
                if let Some(n) = options.before_context {
                    cmd.args(["-B", &n.to_string()]);
                }
                if let Some(n) = options.after_context {
                    cmd.args(["-A", &n.to_string()]);
                }
            }
            SearchTool::Ag => {
                cmd.arg("--nogroup");
                if options.hidden {
                    cmd.arg("--hidden");
                }
                if options.follow {
                    cmd.arg("--follow");
                }
                if let Some(ref ft) = options.file_type {
                    cmd.args(["--", ft]);
                }
            }
            SearchTool::Grep => {
                cmd.args(["-rn", "--color=never"]);
            }
            SearchTool::Ack => {
                cmd.arg("--nogroup");
                if options.follow {
                    cmd.arg("--follow");
                }
            }
        }

        cmd.arg(pattern);

        if let Some(p) = path {
            cmd.arg(p);
        }
    }

    fn parse_output(&self, output: &str) -> Vec<Match> {
        output
            .lines()
            .filter(|line| !line.is_empty())
            .filter_map(|line| self.parse_line(line))
            .collect()
    }

    fn parse_line(&self, line: &str) -> Option<Match> {
        // Most tools output: file:line:content or file:line_number:content
        let parts: Vec<&str> = line.splitn(3, ':').collect();

        match parts.len() {
            3 => Some(Match {
                file: parts[0].to_string(),
                line_number: parts[1].parse().ok(),
                line: parts[2].to_string(),
            }),
            2 => Some(Match {
                file: parts[0].to_string(),
                line_number: None,
                line: parts[1].to_string(),
            }),
            _ => Some(Match {
                file: String::new(),
                line_number: None,
                line: line.to_string(),
            }),
        }
    }
}

/// Check which search tools are available
pub fn available_tools() -> Vec<SearchTool> {
    SearchTool::all()
        .iter()
        .filter(|t| t.is_available())
        .copied()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_detect() {
        // Should find at least one tool on most systems
        if Search::auto().is_ok() {
            // Good - at least one tool is available
        }
    }

    #[test]
    fn test_tool_command() {
        assert_eq!(SearchTool::Ripgrep.command(), "rg");
        assert_eq!(SearchTool::Ag.command(), "ag");
        assert_eq!(SearchTool::Grep.command(), "grep");
        assert_eq!(SearchTool::Ack.command(), "ack");
    }
}
