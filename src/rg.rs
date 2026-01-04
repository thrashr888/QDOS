//! Ripgrep integration for content search
//!
//! Uses ripgrep (rg) for fast content search when available.

use std::path::PathBuf;
use std::process::Command;

/// Check if ripgrep is available
pub fn is_available() -> bool {
    Command::new("rg")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// A search result from ripgrep
#[derive(Debug, Clone)]
pub struct RgMatch {
    /// File path
    pub path: PathBuf,
    /// Line number (1-indexed)
    pub line_num: usize,
    /// Line content
    pub line_text: String,
}

/// Search for content in files using ripgrep
/// Returns a list of (file_path, display_string) tuples for compatibility with Find modal
pub fn search_content(root: &PathBuf, pattern: &str) -> Vec<(PathBuf, String)> {
    if !is_available() {
        return Vec::new();
    }

    let output = Command::new("rg")
        .arg("--line-number")
        .arg("--no-heading")
        .arg("--color=never")
        .arg("--max-count=10") // Limit matches per file
        .arg("--max-columns=200") // Truncate long lines
        .arg("--ignore-case")
        .arg("--") // Separator for pattern
        .arg(pattern)
        .arg(root)
        .output();

    let output = match output {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut results: Vec<(PathBuf, String)> = Vec::new();
    let mut seen_files: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();

    for line in stdout.lines() {
        if let Some(result) = parse_rg_line(line) {
            // Show unique files only, with first match context
            if !seen_files.contains(&result.path) {
                seen_files.insert(result.path.clone());
                let name = result
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| result.path.to_string_lossy().to_string());
                // Truncate line text for display
                let context = if result.line_text.len() > 50 {
                    format!("{}...", &result.line_text[..50])
                } else {
                    result.line_text.clone()
                };
                let display = format!("{}:{} - {}", name, result.line_num, context);
                results.push((result.path, display));
            }
        }
    }

    results
}

/// Parse a line from ripgrep output in format: path:line:content
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

/// Search for files matching a pattern using ripgrep's glob feature
/// This is faster than recursive dir walking for large directories
#[allow(dead_code)]
pub fn search_files(root: &PathBuf, pattern: &str) -> Vec<(PathBuf, String)> {
    if !is_available() {
        return Vec::new();
    }

    // Use rg --files with glob pattern
    let glob_pattern = format!("**/{}*", pattern.replace('*', ""));

    let output = Command::new("rg")
        .arg("--files")
        .arg("--glob")
        .arg(&glob_pattern)
        .arg("--color=never")
        .arg(root)
        .output();

    let output = match output {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut results: Vec<(PathBuf, String)> = Vec::new();

    for line in stdout.lines() {
        if !line.is_empty() {
            let path = PathBuf::from(line);
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string());
            let parent = path
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            let display = format!("{} - {}", name, parent);
            results.push((path, display));
        }
    }

    results
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
}
