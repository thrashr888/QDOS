//! Search tool integration for content search
//!
//! Uses the search-tools crate for unified search across multiple tools:
//! ripgrep, ag, grep, and ack.

use crate::config::SearchTool as ConfigSearchTool;
use search_tools::{Search, SearchOptions, SearchTool};
use std::path::PathBuf;

/// Check if ripgrep is available
pub fn is_available() -> bool {
    SearchTool::Ripgrep.is_available()
}

/// A search result from ripgrep
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RgMatch {
    /// File path
    pub path: PathBuf,
    /// Line number (1-indexed)
    pub line_num: usize,
    /// Line content
    pub line_text: String,
}

/// Convert config SearchTool to search-tools SearchTool
fn config_to_search_tool(tool: ConfigSearchTool) -> Option<SearchTool> {
    match tool.resolve() {
        ConfigSearchTool::Rg | ConfigSearchTool::Auto => Some(SearchTool::Ripgrep),
        ConfigSearchTool::Ag => Some(SearchTool::Ag),
        ConfigSearchTool::Grep => Some(SearchTool::Grep),
        ConfigSearchTool::Ack => Some(SearchTool::Ack),
        ConfigSearchTool::Basic => None,
    }
}

/// Create a Search instance from config tool
fn search_from_config(tool: ConfigSearchTool) -> Option<Search> {
    if let Some(search_tool) = config_to_search_tool(tool) {
        Search::new(search_tool).ok()
    } else {
        // For Auto, use auto-detection
        if tool == ConfigSearchTool::Auto {
            Search::auto().ok()
        } else {
            None
        }
    }
}

/// Search for content in files using ripgrep
/// Returns a list of (file_path, display_string) tuples for compatibility with Find modal
#[allow(dead_code)]
pub fn search_content(root: &PathBuf, pattern: &str) -> Vec<(PathBuf, String)> {
    let search = match Search::auto() {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    search_content_with_search(&search, root, pattern)
}

/// Search for content using the specified search tool
/// Returns a list of (file_path, display_string) tuples for compatibility with Find modal
pub fn search_content_with_tool(
    root: &PathBuf,
    pattern: &str,
    tool: ConfigSearchTool,
) -> Vec<(PathBuf, String)> {
    let resolved = tool.resolve();

    // Handle Basic tool (no content search)
    if resolved == ConfigSearchTool::Basic {
        return Vec::new();
    }

    let search = match search_from_config(tool) {
        Some(s) => s,
        None => return Vec::new(),
    };

    search_content_with_search(&search, root, pattern)
}

/// Internal search implementation using the Search instance
fn search_content_with_search(
    search: &Search,
    root: &PathBuf,
    pattern: &str,
) -> Vec<(PathBuf, String)> {
    let options = SearchOptions {
        line_numbers: true,
        ignore_case: true,
        max_count: Some(10), // Limit matches per file
        ..Default::default()
    };

    let result = match search.search_with_options(pattern, Some(&root.to_string_lossy()), &options)
    {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let mut results: Vec<(PathBuf, String)> = Vec::new();
    let mut seen_files: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();

    for m in result.matches {
        let path = m.path();

        // Show unique files only, with first match context
        if !seen_files.contains(&path) {
            seen_files.insert(path.clone());

            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string());

            // Truncate line text for display
            let context = if m.line.len() > 50 {
                format!("{}...", &m.line[..50])
            } else {
                m.line.clone()
            };

            let line_num = m.line_number.unwrap_or(0);
            let display = format!("{}:{} - {}", name, line_num, context);
            results.push((path, display));
        }
    }

    results
}

/// Search for files matching a pattern using ripgrep's glob feature
/// This is faster than recursive dir walking for large directories
#[allow(dead_code)]
pub fn search_files(root: &PathBuf, pattern: &str) -> Vec<(PathBuf, String)> {
    let search = match Search::auto() {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    // Use files_only mode with glob pattern
    let glob_pattern = format!("**/{}*", pattern.replace('*', ""));
    let options = SearchOptions {
        files_only: true,
        glob: Some(glob_pattern),
        ..Default::default()
    };

    // Search for empty pattern with glob filter to list matching files
    let result = match search.search_with_options(".", Some(&root.to_string_lossy()), &options) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    result
        .matches
        .into_iter()
        .map(|m| {
            let path = m.path();
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string());
            let parent = path
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            let display = format!("{} - {}", name, parent);
            (path, display)
        })
        .collect()
}

/// Parse a line from ripgrep output in format: path:line:content
/// Kept for backwards compatibility and testing
#[allow(dead_code)]
fn parse_rg_line(line: &str) -> Option<RgMatch> {
    // Format: path:line_num:line_text
    // Need to handle paths with colons (rare but possible)

    let parts: Vec<&str> = line.splitn(3, ':').collect();
    if parts.len() < 3 {
        return None;
    }

    let path = PathBuf::from(parts[0]);
    let line_num = parts[1].parse().ok()?;
    let line_text = parts[2].trim().to_string();

    Some(RgMatch {
        path,
        line_num,
        line_text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_available() {
        // May or may not be available depending on the system
        let _ = is_available();
    }

    #[test]
    fn test_parse_rg_line() {
        let line = "/path/to/file.rs:42:fn main() {";
        let result = parse_rg_line(line).unwrap();
        assert_eq!(result.path, PathBuf::from("/path/to/file.rs"));
        assert_eq!(result.line_num, 42);
        assert_eq!(result.line_text, "fn main() {");
    }

    #[test]
    fn test_parse_rg_line_with_colons() {
        let line = "/path/to/file.rs:10:let x: i32 = 0;";
        let result = parse_rg_line(line).unwrap();
        assert_eq!(result.path, PathBuf::from("/path/to/file.rs"));
        assert_eq!(result.line_num, 10);
        assert_eq!(result.line_text, "let x: i32 = 0;");
    }

    #[test]
    fn test_config_to_search_tool() {
        assert_eq!(
            config_to_search_tool(ConfigSearchTool::Rg),
            Some(SearchTool::Ripgrep)
        );
        assert_eq!(
            config_to_search_tool(ConfigSearchTool::Ag),
            Some(SearchTool::Ag)
        );
        assert_eq!(
            config_to_search_tool(ConfigSearchTool::Grep),
            Some(SearchTool::Grep)
        );
        assert_eq!(
            config_to_search_tool(ConfigSearchTool::Ack),
            Some(SearchTool::Ack)
        );
        assert_eq!(config_to_search_tool(ConfigSearchTool::Basic), None);
    }
}
