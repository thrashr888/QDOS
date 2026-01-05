//! Jujutsu (jj) version control wrapper for Rust
//!
//! A type-safe interface to the jj CLI for version control operations.
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
//! println!("{}", status.stdout);
//!
//! // View log
//! let changes = jj.log(Some(10))?;
//! for change in changes {
//!     println!("{}: {}", change.change_id, change.description);
//! }
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

impl CommandOutput {
    /// Get combined stdout and stderr output
    pub fn combined(&self) -> String {
        if self.stderr.is_empty() {
            self.stdout.clone()
        } else if self.stdout.is_empty() {
            self.stderr.clone()
        } else {
            format!("{}\n{}", self.stdout, self.stderr)
        }
    }
}

/// A change/commit in jj
#[derive(Debug, Clone)]
pub struct Change {
    /// Short change ID (e.g., "abc123")
    pub change_id: String,
    /// Full commit ID
    pub commit_id: String,
    /// Change description/message
    pub description: String,
    /// Author name/email
    pub author: String,
    /// Timestamp string
    pub timestamp: String,
    /// Whether this is the current working copy
    pub is_working_copy: bool,
    /// Whether this change is empty
    pub is_empty: bool,
    /// Associated bookmarks
    pub bookmarks: Vec<String>,
}

/// Status of a file in jj
#[derive(Debug, Clone)]
pub struct FileStatus {
    /// File path
    pub path: PathBuf,
    /// Status character: M (modified), A (added), D (deleted), R (renamed)
    pub status: char,
    /// Lines added (if available)
    pub added: Option<usize>,
    /// Lines removed (if available)
    pub removed: Option<usize>,
}

/// A bookmark (branch) in jj
#[derive(Debug, Clone)]
pub struct Bookmark {
    /// Bookmark name
    pub name: String,
    /// Remote name (if tracking)
    pub remote: Option<String>,
    /// Target change ID
    pub target: Option<String>,
    /// Whether this is the current bookmark
    pub is_current: bool,
    /// Whether tracking a remote
    pub is_tracking: bool,
}

/// An operation in jj history
#[derive(Debug, Clone)]
pub struct Operation {
    /// Operation ID (short)
    pub id: String,
    /// Whether this is the current operation
    pub is_current: bool,
    /// Time ago string
    pub time: String,
    /// Operation description
    pub description: String,
}

/// Working copy status information
#[derive(Debug, Clone, Default)]
pub struct WorkingCopyStatus {
    /// Current change ID
    pub change_id: Option<String>,
    /// Whether working copy is empty
    pub is_empty: bool,
    /// Current bookmark (if any)
    pub bookmark: Option<String>,
    /// Number of modified files
    pub modified_count: usize,
    /// Has uncommitted changes
    pub has_changes: bool,
}

/// Diff format options
#[derive(Debug, Clone, Copy, Default)]
pub enum DiffFormat {
    /// Default diff format
    #[default]
    Default,
    /// Git diff format
    Git,
    /// Summary only (file list with status)
    Summary,
    /// Stat format (file list with +/- counts)
    Stat,
    /// Color words diff
    ColorWords,
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

    /// Set the working directory
    pub fn set_workdir(&mut self, path: impl Into<PathBuf>) {
        self.workdir = Some(path.into());
    }

    /// Check if jj is available
    pub fn is_available(&self) -> bool {
        self.run_command(&["--version"]).is_ok()
    }

    /// Check if current directory is a jj repository
    pub fn is_repo(&self) -> bool {
        self.run_command(&["status"]).is_ok()
    }

    // --- Status operations ---

    /// Get working copy status
    pub fn status(&self) -> Result<CommandOutput> {
        self.run_command(&["status"])
    }

    /// Get detailed working copy status
    pub fn working_copy_status(&self) -> Result<WorkingCopyStatus> {
        let mut status = WorkingCopyStatus::default();

        // Get working copy change ID and empty status
        let output = self.run_command(&[
            "log",
            "-r",
            "@",
            "-T",
            r#"change_id.short(8) ++ "\t" ++ if(empty, "true", "false")"#,
            "--no-graph",
        ])?;

        if let Some(line) = output.stdout.lines().next() {
            let parts: Vec<&str> = line.split('\t').collect();
            if !parts.is_empty() {
                status.change_id = Some(parts[0].to_string());
            }
            if parts.len() > 1 {
                status.is_empty = parts[1] == "true";
            }
        }

        // Get current bookmark
        let bookmark_output =
            self.run_command(&["log", "-r", "@-", "-T", "bookmarks", "--no-graph"])?;
        let bookmark = bookmark_output.stdout.trim();
        if !bookmark.is_empty() {
            status.bookmark = Some(
                bookmark
                    .split_whitespace()
                    .next()
                    .unwrap_or(bookmark)
                    .to_string(),
            );
        }

        // Get modified file count
        let diff_output = self.run_command(&["diff", "--summary"])?;
        status.modified_count = diff_output.stdout.lines().count();
        status.has_changes = status.modified_count > 0;

        Ok(status)
    }

    // --- Log operations ---

    /// View change log
    pub fn log(&self, limit: Option<usize>) -> Result<Vec<Change>> {
        let rev = match limit {
            Some(n) => format!("ancestors(@, {})", n),
            None => "ancestors(@, 20)".to_string(),
        };

        let output = self.run_command(&[
            "log",
            "-r",
            &rev,
            "-T",
            r#"change_id.short(8) ++ "\t" ++ commit_id.short(8) ++ "\t" ++ description.first_line() ++ "\t" ++ author.name() ++ "\t" ++ committer.timestamp().ago() ++ "\t" ++ if(current_working_copy, "true", "false") ++ "\t" ++ if(empty, "true", "false") ++ "\t" ++ bookmarks ++ "\n""#,
            "--no-graph",
        ])?;

        let mut changes = Vec::new();
        for line in output.stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 6 {
                changes.push(Change {
                    change_id: parts[0].to_string(),
                    commit_id: parts.get(1).unwrap_or(&"").to_string(),
                    description: parts.get(2).unwrap_or(&"").to_string(),
                    author: parts.get(3).unwrap_or(&"").to_string(),
                    timestamp: parts.get(4).unwrap_or(&"").to_string(),
                    is_working_copy: parts.get(5).unwrap_or(&"false") == &"true",
                    is_empty: parts.get(6).unwrap_or(&"false") == &"true",
                    bookmarks: parts
                        .get(7)
                        .unwrap_or(&"")
                        .split_whitespace()
                        .map(String::from)
                        .collect(),
                });
            }
        }

        Ok(changes)
    }

    /// Get raw log output (for custom formatting)
    pub fn log_raw(&self, limit: Option<usize>) -> Result<CommandOutput> {
        match limit {
            Some(n) => self.run_command(&["log", "-n", &n.to_string()]),
            None => self.run_command(&["log"]),
        }
    }

    // --- Diff operations ---

    /// Show diff with specified format
    pub fn diff(&self, revision: Option<&str>, format: DiffFormat) -> Result<CommandOutput> {
        let format_arg = match format {
            DiffFormat::Default => None,
            DiffFormat::Git => Some("--git"),
            DiffFormat::Summary => Some("--summary"),
            DiffFormat::Stat => Some("--stat"),
            DiffFormat::ColorWords => Some("--color-words"),
        };

        let mut args = vec!["diff"];
        if let Some(rev) = revision {
            args.push("-r");
            args.push(rev);
        }
        if let Some(fmt) = format_arg {
            args.push(fmt);
        }

        self.run_command(&args)
    }

    /// Get file changes for current working copy
    pub fn diff_files(&self) -> Result<Vec<FileStatus>> {
        let output = self.run_command(&["diff", "--stat"])?;
        Ok(parse_diff_stat(&output.stdout))
    }

    /// Get file changes for a specific revision
    pub fn diff_files_revision(&self, revision: &str) -> Result<Vec<FileStatus>> {
        let output = self.run_command(&["diff", "-r", revision, "--stat"])?;
        Ok(parse_diff_stat(&output.stdout))
    }

    // --- Change operations ---

    /// Update change description
    pub fn describe(&self, message: &str) -> Result<CommandOutput> {
        self.run_command(&["describe", "-m", message])
    }

    /// Update description for a specific revision
    pub fn describe_revision(&self, revision: &str, message: &str) -> Result<CommandOutput> {
        self.run_command(&["describe", "-r", revision, "-m", message])
    }

    /// Create new change
    pub fn new_change(&self) -> Result<CommandOutput> {
        self.run_command(&["new"])
    }

    /// Create new change with message
    pub fn new_change_with_message(&self, message: &str) -> Result<CommandOutput> {
        self.run_command(&["new", "-m", message])
    }

    /// Edit a specific change
    pub fn edit(&self, revision: &str) -> Result<CommandOutput> {
        self.run_command(&["edit", revision])
    }

    /// Squash changes into parent
    pub fn squash(&self) -> Result<CommandOutput> {
        self.run_command(&["squash"])
    }

    /// Squash a specific revision
    pub fn squash_revision(&self, revision: &str) -> Result<CommandOutput> {
        self.run_command(&["squash", "-r", revision])
    }

    /// Abandon a change
    pub fn abandon(&self, revision: Option<&str>) -> Result<CommandOutput> {
        match revision {
            Some(rev) => self.run_command(&["abandon", rev]),
            None => self.run_command(&["abandon"]),
        }
    }

    /// Split the current change
    pub fn split(&self) -> Result<CommandOutput> {
        self.run_command(&["split"])
    }

    /// Duplicate a change
    pub fn duplicate(&self, revision: &str) -> Result<CommandOutput> {
        self.run_command(&["duplicate", revision])
    }

    /// Rebase changes
    pub fn rebase(&self, destination: &str) -> Result<CommandOutput> {
        self.run_command(&["rebase", "-d", destination])
    }

    /// Rebase specific revision
    pub fn rebase_revision(&self, revision: &str, destination: &str) -> Result<CommandOutput> {
        self.run_command(&["rebase", "-r", revision, "-d", destination])
    }

    // --- Bookmark operations ---

    /// List bookmarks
    pub fn bookmark_list(&self) -> Result<Vec<Bookmark>> {
        let output = self.run_command(&["bookmark", "list", "--all-remotes"])?;
        Ok(parse_bookmarks(&output.stdout))
    }

    /// Get raw bookmark list output
    pub fn bookmark_list_raw(&self) -> Result<CommandOutput> {
        self.run_command(&["bookmark", "list", "--all-remotes"])
    }

    /// Create a bookmark
    pub fn bookmark_create(&self, name: &str) -> Result<CommandOutput> {
        self.run_command(&["bookmark", "create", name])
    }

    /// Create a bookmark at a specific revision
    pub fn bookmark_create_at(&self, name: &str, revision: &str) -> Result<CommandOutput> {
        self.run_command(&["bookmark", "create", name, "-r", revision])
    }

    /// Delete a bookmark
    pub fn bookmark_delete(&self, name: &str) -> Result<CommandOutput> {
        self.run_command(&["bookmark", "delete", name])
    }

    /// Move/set a bookmark to current revision
    pub fn bookmark_set(&self, name: &str) -> Result<CommandOutput> {
        self.run_command(&["bookmark", "set", name])
    }

    /// Move/set a bookmark to a specific revision
    pub fn bookmark_set_at(&self, name: &str, revision: &str) -> Result<CommandOutput> {
        self.run_command(&["bookmark", "set", name, "-r", revision])
    }

    /// Track a remote bookmark
    pub fn bookmark_track(&self, bookmark: &str, remote: &str) -> Result<CommandOutput> {
        let full_name = format!("{}@{}", bookmark, remote);
        self.run_command(&["bookmark", "track", &full_name])
    }

    // --- Operation log ---

    /// List operations
    pub fn operation_log(&self) -> Result<Vec<Operation>> {
        let output = self.run_command(&[
            "operation",
            "log",
            "-T",
            r#"id.short(8) ++ "\t" ++ if(current_operation, "true", "false") ++ "\t" ++ time.start().ago() ++ "\t" ++ description ++ "\n""#,
            "--no-graph",
        ])?;

        let mut operations = Vec::new();
        for line in output.stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 4 {
                operations.push(Operation {
                    id: parts[0].to_string(),
                    is_current: parts[1] == "true",
                    time: parts[2].to_string(),
                    description: parts[3].to_string(),
                });
            }
        }

        Ok(operations)
    }

    /// Get raw operation log output
    pub fn operation_log_raw(&self) -> Result<CommandOutput> {
        self.run_command(&["operation", "log"])
    }

    /// Undo last operation
    pub fn undo(&self) -> Result<CommandOutput> {
        self.run_command(&["undo"])
    }

    /// Restore to a specific operation
    pub fn operation_restore(&self, operation_id: &str) -> Result<CommandOutput> {
        self.run_command(&["operation", "restore", operation_id])
    }

    // --- Git operations ---

    /// Git push
    pub fn git_push(&self) -> Result<CommandOutput> {
        self.run_command(&["git", "push"])
    }

    /// Git push with options
    pub fn git_push_all(&self) -> Result<CommandOutput> {
        self.run_command(&["git", "push", "--all"])
    }

    /// Git push a specific bookmark
    pub fn git_push_bookmark(&self, bookmark: &str) -> Result<CommandOutput> {
        self.run_command(&["git", "push", "-b", bookmark])
    }

    /// Git fetch
    pub fn git_fetch(&self) -> Result<CommandOutput> {
        self.run_command(&["git", "fetch"])
    }

    /// Git fetch from all remotes
    pub fn git_fetch_all(&self) -> Result<CommandOutput> {
        self.run_command(&["git", "fetch", "--all-remotes"])
    }

    /// Git init (create colocated repo)
    pub fn git_init(&self) -> Result<CommandOutput> {
        self.run_command(&["git", "init", "--colocate"])
    }

    /// Git clone
    pub fn git_clone(&self, url: &str, dest: &str) -> Result<CommandOutput> {
        self.run_command(&["git", "clone", "--colocate", url, dest])
    }

    // --- Conflict operations ---

    /// Check if there are conflicts
    pub fn has_conflicts(&self) -> Result<bool> {
        let output = self.status()?;
        Ok(output.stdout.contains("conflict") || output.stdout.contains("Conflict"))
    }

    /// List conflicting files
    pub fn conflict_list(&self) -> Result<Vec<String>> {
        let output = self.run_command(&["resolve", "--list"])?;
        Ok(output
            .stdout
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.trim().to_string())
            .collect())
    }

    // --- Workspace operations ---

    /// List workspaces
    pub fn workspace_list(&self) -> Result<CommandOutput> {
        self.run_command(&["workspace", "list"])
    }

    /// Add a workspace
    pub fn workspace_add(&self, path: &str, name: Option<&str>) -> Result<CommandOutput> {
        match name {
            Some(n) => self.run_command(&["workspace", "add", "--name", n, path]),
            None => self.run_command(&["workspace", "add", path]),
        }
    }

    /// Forget a workspace
    pub fn workspace_forget(&self, workspace: &str) -> Result<CommandOutput> {
        self.run_command(&["workspace", "forget", workspace])
    }

    // --- File operations ---

    /// Restore files to parent version
    pub fn restore(&self, paths: &[&str]) -> Result<CommandOutput> {
        let mut args = vec!["restore"];
        args.extend(paths);
        self.run_command(&args)
    }

    /// Show file content at revision
    pub fn file_show(&self, path: &str, revision: Option<&str>) -> Result<CommandOutput> {
        match revision {
            Some(rev) => self.run_command(&["file", "show", "-r", rev, path]),
            None => self.run_command(&["file", "show", path]),
        }
    }

    // --- Show operations ---

    /// Show a specific revision
    pub fn show(&self, revision: &str) -> Result<CommandOutput> {
        self.run_command(&["show", "-r", revision])
    }

    // --- Raw command execution ---

    /// Run an arbitrary jj command
    pub fn run(&self, args: &[&str]) -> Result<CommandOutput> {
        self.run_command(args)
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
            // Check for specific error conditions
            if stderr.contains("not a jujutsu repo")
                || stderr.contains("No jj repo")
                || stderr.contains("not in a repository")
            {
                return Err(Error::NotInRepo);
            }
            return Err(Error::CommandFailed(stderr));
        }

        Ok(CommandOutput {
            success: output.status.success(),
            stdout,
            stderr,
        })
    }
}

/// Parse diff --stat output into FileStatus entries
fn parse_diff_stat(output: &str) -> Vec<FileStatus> {
    let mut files = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("diff") || line.contains(" file") {
            continue;
        }

        // Format: "path | N +++---" or just "path"
        let parts: Vec<&str> = line.split('|').collect();
        if let Some(path_part) = parts.first() {
            let path = path_part.trim();
            if path.is_empty() {
                continue;
            }

            let (added, removed) = if parts.len() > 1 {
                let stats = parts[1].trim();
                let add_count = stats.matches('+').count();
                let rem_count = stats.matches('-').count();
                (Some(add_count), Some(rem_count))
            } else {
                (None, None)
            };

            // Determine status from the line pattern
            let status = if line.contains(" | ") {
                'M' // Modified
            } else if line.starts_with("A ") {
                'A' // Added
            } else if line.starts_with("D ") {
                'D' // Deleted
            } else {
                'M' // Default to modified
            };

            files.push(FileStatus {
                path: PathBuf::from(path),
                status,
                added,
                removed,
            });
        }
    }

    files
}

/// Parse bookmark list output
fn parse_bookmarks(output: &str) -> Vec<Bookmark> {
    let mut bookmarks = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Format varies: "name: change_id" or "name@remote: change_id" or just "name"
        let (name_part, rest) = line.split_once(':').unwrap_or((line, ""));

        // Check for remote
        let (name, remote) = if name_part.contains('@') {
            let parts: Vec<&str> = name_part.split('@').collect();
            (
                parts[0].to_string(),
                Some(parts.get(1).unwrap_or(&"").to_string()),
            )
        } else {
            (name_part.to_string(), None)
        };

        let target = if !rest.trim().is_empty() {
            Some(rest.trim().to_string())
        } else {
            None
        };

        let is_current = line.contains("(current)") || line.starts_with('*');
        let is_tracking = remote.is_some();

        bookmarks.push(Bookmark {
            name,
            remote,
            target,
            is_current,
            is_tracking,
        });
    }

    bookmarks
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

    #[test]
    fn test_with_workdir() {
        let jj = Jj::with_workdir("/tmp");
        assert_eq!(jj.workdir, Some(PathBuf::from("/tmp")));
    }

    #[test]
    fn test_command_output_combined() {
        let output = CommandOutput {
            success: true,
            stdout: "output".to_string(),
            stderr: "".to_string(),
        };
        assert_eq!(output.combined(), "output");

        let output_with_err = CommandOutput {
            success: false,
            stdout: "out".to_string(),
            stderr: "err".to_string(),
        };
        assert_eq!(output_with_err.combined(), "out\nerr");
    }

    #[test]
    fn test_parse_diff_stat() {
        let stat_output = r#"
src/main.rs | 10 +++++-----
src/lib.rs  |  5 +++++
README.md   |  2 --
"#;
        let files = parse_diff_stat(stat_output);
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].path, PathBuf::from("src/main.rs"));
        assert_eq!(files[1].path, PathBuf::from("src/lib.rs"));
        assert_eq!(files[2].path, PathBuf::from("README.md"));
    }

    #[test]
    fn test_parse_bookmarks() {
        let bookmark_output = r#"
main: abc123
feature@origin: def456
dev: ghi789
"#;
        let bookmarks = parse_bookmarks(bookmark_output);
        assert_eq!(bookmarks.len(), 3);
        assert_eq!(bookmarks[0].name, "main");
        assert_eq!(bookmarks[0].remote, None);
        assert_eq!(bookmarks[1].name, "feature");
        assert_eq!(bookmarks[1].remote, Some("origin".to_string()));
    }

    #[test]
    fn test_diff_format() {
        assert!(matches!(DiffFormat::default(), DiffFormat::Default));
    }

    #[test]
    fn test_working_copy_status_default() {
        let status = WorkingCopyStatus::default();
        assert!(status.change_id.is_none());
        assert!(!status.is_empty);
        assert_eq!(status.modified_count, 0);
    }

    #[test]
    fn test_change_struct() {
        let change = Change {
            change_id: "abc123".to_string(),
            commit_id: "def456".to_string(),
            description: "Test change".to_string(),
            author: "Test Author".to_string(),
            timestamp: "1 hour ago".to_string(),
            is_working_copy: true,
            is_empty: false,
            bookmarks: vec!["main".to_string()],
        };
        assert_eq!(change.change_id, "abc123");
        assert!(change.is_working_copy);
        assert!(!change.is_empty);
    }

    #[test]
    fn test_operation_struct() {
        let op = Operation {
            id: "abc123".to_string(),
            is_current: true,
            time: "1 minute ago".to_string(),
            description: "new empty commit".to_string(),
        };
        assert_eq!(op.id, "abc123");
        assert!(op.is_current);
    }

    #[test]
    fn test_bookmark_struct() {
        let bookmark = Bookmark {
            name: "main".to_string(),
            remote: Some("origin".to_string()),
            target: Some("abc123".to_string()),
            is_current: false,
            is_tracking: true,
        };
        assert_eq!(bookmark.name, "main");
        assert!(bookmark.is_tracking);
    }

    // Integration tests (require jj to be installed and in a repo)
    #[test]
    #[ignore]
    fn test_status_in_repo() {
        if let Ok(jj) = Jj::new() {
            let result = jj.status();
            assert!(result.is_ok());
        }
    }

    #[test]
    #[ignore]
    fn test_log_in_repo() {
        if let Ok(jj) = Jj::new() {
            let result = jj.log(Some(5));
            assert!(result.is_ok());
        }
    }
}
