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
//! for m in results.matches {
//!     println!("{}:{}: {}", m.file, m.line_number.unwrap_or(0), m.line);
//! }
//! # Ok::<(), search_tools::Error>(())
//! ```

use std::path::PathBuf;
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

    /// Get a display name for this tool
    pub fn name(&self) -> &'static str {
        match self {
            SearchTool::Ripgrep => "ripgrep",
            SearchTool::Ag => "The Silver Searcher",
            SearchTool::Grep => "GNU grep",
            SearchTool::Ack => "ack",
        }
    }

    /// Check if this tool is available
    pub fn is_available(&self) -> bool {
        Command::new(self.command())
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Get version string for this tool
    pub fn version(&self) -> Option<String> {
        let output = Command::new(self.command())
            .arg("--version")
            .output()
            .ok()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Some(stdout.lines().next().unwrap_or("unknown").to_string())
        } else {
            None
        }
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

impl std::fmt::Display for SearchTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// A search match
#[derive(Debug, Clone)]
pub struct Match {
    /// File path containing the match
    pub file: String,
    /// Line number (1-indexed)
    pub line_number: Option<usize>,
    /// Column number (1-indexed)
    pub column: Option<usize>,
    /// Matched line content
    pub line: String,
}

impl Match {
    /// Get the file path as a PathBuf
    pub fn path(&self) -> PathBuf {
        PathBuf::from(&self.file)
    }
}

/// Search output
#[derive(Debug, Clone)]
pub struct SearchOutput {
    /// Tool used for the search
    pub tool: SearchTool,
    /// All matches found
    pub matches: Vec<Match>,
    /// Total match count
    pub match_count: usize,
    /// Whether the search was successful
    pub success: bool,
    /// Raw output (if needed)
    pub raw_output: String,
}

impl SearchOutput {
    /// Get matches grouped by file
    pub fn by_file(&self) -> std::collections::HashMap<String, Vec<&Match>> {
        let mut map: std::collections::HashMap<String, Vec<&Match>> =
            std::collections::HashMap::new();
        for m in &self.matches {
            map.entry(m.file.clone()).or_default().push(m);
        }
        map
    }

    /// Get unique files containing matches
    pub fn files(&self) -> Vec<String> {
        let mut files: Vec<String> = self.matches.iter().map(|m| m.file.clone()).collect();
        files.sort();
        files.dedup();
        files
    }

    /// Check if any matches were found
    pub fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }
}

/// Search options
#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    /// Case insensitive search
    pub ignore_case: bool,
    /// Case sensitive search (override smart case)
    pub case_sensitive: bool,
    /// Search hidden files
    pub hidden: bool,
    /// Follow symlinks
    pub follow: bool,
    /// File type filter (e.g., "rust", "py", "js")
    pub file_type: Option<String>,
    /// Glob pattern filter (e.g., "*.rs", "src/**/*.js")
    pub glob: Option<String>,
    /// Inverse glob (exclude pattern)
    pub exclude: Option<String>,
    /// Max results to return
    pub max_count: Option<usize>,
    /// Max depth to search
    pub max_depth: Option<usize>,
    /// Context lines before match
    pub before_context: Option<usize>,
    /// Context lines after match
    pub after_context: Option<usize>,
    /// Search only in specific file (not directory)
    pub file_only: bool,
    /// Match whole words only
    pub word_boundary: bool,
    /// Use regex pattern
    pub regex: bool,
    /// Fixed string (literal) search
    pub fixed_string: bool,
    /// Return only file names (no content)
    pub files_only: bool,
    /// Include line numbers
    pub line_numbers: bool,
    /// Include column numbers
    pub column_numbers: bool,
    /// Count matches only
    pub count_only: bool,
    /// Quiet mode (just check if matches exist)
    pub quiet: bool,
    /// Sort results
    pub sort: bool,
}

impl SearchOptions {
    /// Create options for a case-insensitive search
    pub fn case_insensitive() -> Self {
        Self {
            ignore_case: true,
            line_numbers: true,
            ..Default::default()
        }
    }

    /// Create options for searching hidden files
    pub fn with_hidden() -> Self {
        Self {
            hidden: true,
            line_numbers: true,
            ..Default::default()
        }
    }

    /// Create options for listing files only
    pub fn files_list() -> Self {
        Self {
            files_only: true,
            ..Default::default()
        }
    }

    /// Create options with context lines
    pub fn with_context(lines: usize) -> Self {
        Self {
            before_context: Some(lines),
            after_context: Some(lines),
            line_numbers: true,
            ..Default::default()
        }
    }

    /// Create options for a specific file type
    pub fn for_file_type(file_type: &str) -> Self {
        Self {
            file_type: Some(file_type.to_string()),
            line_numbers: true,
            ..Default::default()
        }
    }
}

/// Unified search interface
#[derive(Debug, Clone)]
pub struct Search {
    tool: SearchTool,
    working_dir: Option<PathBuf>,
}

impl Search {
    /// Create with a specific search tool
    pub fn new(tool: SearchTool) -> Result<Self> {
        if !tool.is_available() {
            return Err(Error::ToolNotInstalled(tool.command().to_string()));
        }
        Ok(Self {
            tool,
            working_dir: None,
        })
    }

    /// Auto-detect the best available search tool
    pub fn auto() -> Result<Self> {
        for tool in SearchTool::all() {
            if tool.is_available() {
                return Ok(Self {
                    tool: *tool,
                    working_dir: None,
                });
            }
        }
        Err(Error::NoToolAvailable)
    }

    /// Set the working directory for searches
    pub fn with_working_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(path.into());
        self
    }

    /// Get the active search tool
    pub fn tool(&self) -> SearchTool {
        self.tool
    }

    /// Search for a pattern in a path
    pub fn search(&self, pattern: &str, path: Option<&str>) -> Result<SearchOutput> {
        let options = SearchOptions {
            line_numbers: true,
            ..Default::default()
        };
        self.search_with_options(pattern, path, &options)
    }

    /// Search with options
    pub fn search_with_options(
        &self,
        pattern: &str,
        path: Option<&str>,
        options: &SearchOptions,
    ) -> Result<SearchOutput> {
        let mut cmd = Command::new(self.tool.command());

        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        // Add tool-specific arguments
        self.add_args(&mut cmd, pattern, path, options);

        let output = cmd.output()?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // grep returns exit code 1 when no matches, which isn't an error
        let success = output.status.success() || output.status.code() == Some(1);

        if !success && !stderr.is_empty() {
            return Err(Error::SearchFailed(stderr));
        }

        let matches = if options.count_only || options.quiet {
            Vec::new()
        } else {
            self.parse_output(&stdout, options)
        };
        let match_count = matches.len();

        Ok(SearchOutput {
            tool: self.tool,
            matches,
            match_count,
            success,
            raw_output: stdout,
        })
    }

    /// Search and return just file names
    pub fn search_files(&self, pattern: &str, path: Option<&str>) -> Result<Vec<String>> {
        let options = SearchOptions::files_list();
        let result = self.search_with_options(pattern, path, &options)?;
        Ok(result.files())
    }

    /// Count matches
    pub fn count(&self, pattern: &str, path: Option<&str>) -> Result<usize> {
        let mut cmd = Command::new(self.tool.command());

        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        match self.tool {
            SearchTool::Ripgrep => {
                cmd.args(["--count-matches", pattern]);
            }
            SearchTool::Ag => {
                cmd.args(["-c", pattern]);
            }
            SearchTool::Grep => {
                cmd.args(["-rc", pattern]);
            }
            SearchTool::Ack => {
                cmd.args(["-c", pattern]);
            }
        }

        if let Some(p) = path {
            cmd.arg(p);
        }

        let output = cmd.output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Sum up counts from all files
        let count: usize = stdout
            .lines()
            .filter_map(|line| {
                // Format is typically "file:count" or just "count"
                line.split(':').next_back()?.trim().parse::<usize>().ok()
            })
            .sum();

        Ok(count)
    }

    /// Check if pattern exists anywhere (fast existence check)
    pub fn exists(&self, pattern: &str, path: Option<&str>) -> Result<bool> {
        let mut cmd = Command::new(self.tool.command());

        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        match self.tool {
            SearchTool::Ripgrep => {
                cmd.args(["--quiet", pattern]);
            }
            SearchTool::Ag => {
                cmd.args(["-q", pattern]);
            }
            SearchTool::Grep => {
                cmd.args(["-rq", pattern]);
            }
            SearchTool::Ack => {
                cmd.args(["--count", pattern]);
            }
        }

        if let Some(p) = path {
            cmd.arg(p);
        }

        let output = cmd.output()?;
        Ok(output.status.success())
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
        if options.case_sensitive {
            cmd.arg("-s");
        }

        // Tool-specific flags
        match self.tool {
            SearchTool::Ripgrep => {
                self.add_ripgrep_args(cmd, options);
            }
            SearchTool::Ag => {
                self.add_ag_args(cmd, options);
            }
            SearchTool::Grep => {
                self.add_grep_args(cmd, options);
            }
            SearchTool::Ack => {
                self.add_ack_args(cmd, options);
            }
        }

        cmd.arg(pattern);

        if let Some(p) = path {
            cmd.arg(p);
        }
    }

    fn add_ripgrep_args(&self, cmd: &mut Command, options: &SearchOptions) {
        if options.line_numbers && !options.files_only {
            cmd.arg("-n");
        }
        if options.column_numbers {
            cmd.arg("--column");
        }
        if options.hidden {
            cmd.arg("--hidden");
        }
        if options.follow {
            cmd.arg("--follow");
        }
        if options.word_boundary {
            cmd.arg("-w");
        }
        if options.fixed_string {
            cmd.arg("-F");
        }
        if options.files_only {
            cmd.arg("-l");
        }
        if options.count_only {
            cmd.arg("-c");
        }
        if options.quiet {
            cmd.arg("-q");
        }
        if options.sort {
            cmd.arg("--sort=path");
        }
        if let Some(ref ft) = options.file_type {
            cmd.args(["-t", ft]);
        }
        if let Some(ref g) = options.glob {
            cmd.args(["-g", g]);
        }
        if let Some(ref exc) = options.exclude {
            cmd.args(["--glob", &format!("!{}", exc)]);
        }
        if let Some(n) = options.max_count {
            cmd.args(["-m", &n.to_string()]);
        }
        if let Some(n) = options.max_depth {
            cmd.args(["--max-depth", &n.to_string()]);
        }
        if let Some(n) = options.before_context {
            cmd.args(["-B", &n.to_string()]);
        }
        if let Some(n) = options.after_context {
            cmd.args(["-A", &n.to_string()]);
        }
    }

    fn add_ag_args(&self, cmd: &mut Command, options: &SearchOptions) {
        cmd.arg("--nogroup");
        if options.hidden {
            cmd.arg("--hidden");
        }
        if options.follow {
            cmd.arg("--follow");
        }
        if options.word_boundary {
            cmd.arg("-w");
        }
        if options.fixed_string {
            cmd.arg("-Q");
        }
        if options.files_only {
            cmd.arg("-l");
        }
        if options.count_only {
            cmd.arg("-c");
        }
        if let Some(ref ft) = options.file_type {
            cmd.args(["--", ft]);
        }
        if let Some(n) = options.max_depth {
            cmd.args(["--depth", &n.to_string()]);
        }
        if let Some(n) = options.before_context {
            cmd.args(["-B", &n.to_string()]);
        }
        if let Some(n) = options.after_context {
            cmd.args(["-A", &n.to_string()]);
        }
    }

    fn add_grep_args(&self, cmd: &mut Command, options: &SearchOptions) {
        cmd.args(["-r", "--color=never"]);
        if options.line_numbers && !options.files_only {
            cmd.arg("-n");
        }
        if options.word_boundary {
            cmd.arg("-w");
        }
        if options.fixed_string {
            cmd.arg("-F");
        }
        if options.files_only {
            cmd.arg("-l");
        }
        if options.count_only {
            cmd.arg("-c");
        }
        if options.quiet {
            cmd.arg("-q");
        }
        if let Some(ref g) = options.glob {
            cmd.args(["--include", g]);
        }
        if let Some(ref exc) = options.exclude {
            cmd.args(["--exclude", exc]);
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

    fn add_ack_args(&self, cmd: &mut Command, options: &SearchOptions) {
        cmd.arg("--nogroup");
        if options.follow {
            cmd.arg("--follow");
        }
        if options.word_boundary {
            cmd.arg("-w");
        }
        if options.fixed_string {
            cmd.arg("-Q");
        }
        if options.files_only {
            cmd.arg("-l");
        }
        if options.count_only {
            cmd.arg("-c");
        }
        if let Some(ref ft) = options.file_type {
            cmd.args(["--type", ft]);
        }
        if let Some(n) = options.before_context {
            cmd.args(["-B", &n.to_string()]);
        }
        if let Some(n) = options.after_context {
            cmd.args(["-A", &n.to_string()]);
        }
    }

    fn parse_output(&self, output: &str, options: &SearchOptions) -> Vec<Match> {
        if options.files_only {
            return output
                .lines()
                .filter(|line| !line.is_empty())
                .map(|line| Match {
                    file: line.to_string(),
                    line_number: None,
                    column: None,
                    line: String::new(),
                })
                .collect();
        }

        output
            .lines()
            .filter(|line| !line.is_empty())
            .filter_map(|line| self.parse_line(line, options))
            .collect()
    }

    fn parse_line(&self, line: &str, options: &SearchOptions) -> Option<Match> {
        // Most tools output: file:line:content or file:line:column:content
        let parts: Vec<&str> = line.splitn(4, ':').collect();

        match parts.len() {
            4 if options.column_numbers => Some(Match {
                file: parts[0].to_string(),
                line_number: parts[1].parse().ok(),
                column: parts[2].parse().ok(),
                line: parts[3].to_string(),
            }),
            3 => Some(Match {
                file: parts[0].to_string(),
                line_number: parts[1].parse().ok(),
                column: None,
                line: parts[2].to_string(),
            }),
            2 => Some(Match {
                file: parts[0].to_string(),
                line_number: None,
                column: None,
                line: parts[1].to_string(),
            }),
            _ => Some(Match {
                file: String::new(),
                line_number: None,
                column: None,
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

/// Get the best available search tool
pub fn best_tool() -> Option<SearchTool> {
    available_tools().first().copied()
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

    #[test]
    fn test_tool_name() {
        assert_eq!(SearchTool::Ripgrep.name(), "ripgrep");
        assert_eq!(SearchTool::Ag.name(), "The Silver Searcher");
        assert_eq!(SearchTool::Grep.name(), "GNU grep");
        assert_eq!(SearchTool::Ack.name(), "ack");
    }

    #[test]
    fn test_tool_display() {
        assert_eq!(SearchTool::Ripgrep.to_string(), "ripgrep");
    }

    #[test]
    fn test_search_options_defaults() {
        let opts = SearchOptions::default();
        assert!(!opts.ignore_case);
        assert!(!opts.hidden);
        assert!(opts.file_type.is_none());
    }

    #[test]
    fn test_search_options_case_insensitive() {
        let opts = SearchOptions::case_insensitive();
        assert!(opts.ignore_case);
        assert!(opts.line_numbers);
    }

    #[test]
    fn test_search_options_with_hidden() {
        let opts = SearchOptions::with_hidden();
        assert!(opts.hidden);
    }

    #[test]
    fn test_search_options_files_list() {
        let opts = SearchOptions::files_list();
        assert!(opts.files_only);
    }

    #[test]
    fn test_search_options_with_context() {
        let opts = SearchOptions::with_context(3);
        assert_eq!(opts.before_context, Some(3));
        assert_eq!(opts.after_context, Some(3));
    }

    #[test]
    fn test_search_options_for_file_type() {
        let opts = SearchOptions::for_file_type("rust");
        assert_eq!(opts.file_type, Some("rust".to_string()));
    }

    #[test]
    fn test_match_path() {
        let m = Match {
            file: "/path/to/file.rs".to_string(),
            line_number: Some(42),
            column: None,
            line: "let x = 1;".to_string(),
        };
        assert_eq!(m.path(), PathBuf::from("/path/to/file.rs"));
    }

    #[test]
    fn test_search_output_by_file() {
        let output = SearchOutput {
            tool: SearchTool::Ripgrep,
            matches: vec![
                Match {
                    file: "a.rs".to_string(),
                    line_number: Some(1),
                    column: None,
                    line: "line 1".to_string(),
                },
                Match {
                    file: "b.rs".to_string(),
                    line_number: Some(2),
                    column: None,
                    line: "line 2".to_string(),
                },
                Match {
                    file: "a.rs".to_string(),
                    line_number: Some(3),
                    column: None,
                    line: "line 3".to_string(),
                },
            ],
            match_count: 3,
            success: true,
            raw_output: String::new(),
        };

        let by_file = output.by_file();
        assert_eq!(by_file.len(), 2);
        assert_eq!(by_file["a.rs"].len(), 2);
        assert_eq!(by_file["b.rs"].len(), 1);
    }

    #[test]
    fn test_search_output_files() {
        let output = SearchOutput {
            tool: SearchTool::Ripgrep,
            matches: vec![
                Match {
                    file: "c.rs".to_string(),
                    line_number: Some(1),
                    column: None,
                    line: "".to_string(),
                },
                Match {
                    file: "a.rs".to_string(),
                    line_number: Some(1),
                    column: None,
                    line: "".to_string(),
                },
                Match {
                    file: "c.rs".to_string(),
                    line_number: Some(2),
                    column: None,
                    line: "".to_string(),
                },
            ],
            match_count: 3,
            success: true,
            raw_output: String::new(),
        };

        let files = output.files();
        assert_eq!(files, vec!["a.rs", "c.rs"]);
    }

    #[test]
    fn test_search_output_is_empty() {
        let empty = SearchOutput {
            tool: SearchTool::Ripgrep,
            matches: vec![],
            match_count: 0,
            success: true,
            raw_output: String::new(),
        };
        assert!(empty.is_empty());

        let not_empty = SearchOutput {
            tool: SearchTool::Ripgrep,
            matches: vec![Match {
                file: "a.rs".to_string(),
                line_number: None,
                column: None,
                line: "".to_string(),
            }],
            match_count: 1,
            success: true,
            raw_output: String::new(),
        };
        assert!(!not_empty.is_empty());
    }

    #[test]
    fn test_available_tools() {
        let tools = available_tools();
        // We should have at least grep on most Unix systems
        // Don't assert this as CI might not have any tools
        let _ = tools;
    }

    #[test]
    fn test_best_tool() {
        // Just test it doesn't panic
        let _ = best_tool();
    }

    // Integration tests
    #[test]
    #[ignore]
    fn test_search_in_current_dir() {
        if let Ok(search) = Search::auto() {
            let result = search.search("test", Some("."));
            assert!(result.is_ok());
        }
    }

    #[test]
    #[ignore]
    fn test_count_in_current_dir() {
        if let Ok(search) = Search::auto() {
            let result = search.count("fn", Some("."));
            assert!(result.is_ok());
        }
    }

    #[test]
    #[ignore]
    fn test_exists_check() {
        if let Ok(search) = Search::auto() {
            let result = search.exists("test", Some("."));
            assert!(result.is_ok());
        }
    }
}
