//! Git Operations Module
//!
//! Provides helper functions for Git integration in Q-DOS II.
//! These are standalone functions to avoid borrow checker issues in the main app.

use crate::app::{
    BlameLine, ConflictFile, ConflictResolution, ConflictSection, FileHistoryEntry, GitBranch,
    GitConfigEntry, GitFileStatus, GitLogEntry, GitRemote, GitStashEntry, GitState, GitSubmodule,
    GitTag, SubmoduleStatus,
};
use std::path::PathBuf;
use std::process::Command;

/// Load git status into state
pub fn load_git_status(state: &mut GitState, cwd: &PathBuf) {
    state.files.clear();
    state.error = None;

    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.len() >= 3 {
                    let status_char = line.chars().nth(1).unwrap_or(' ');
                    let staged = line.chars().next().unwrap_or(' ') != ' ';
                    let path = line[3..].to_string();
                    state.files.push(GitFileStatus {
                        path,
                        status: if status_char == ' ' {
                            line.chars().next().unwrap_or('?')
                        } else {
                            status_char
                        },
                        staged,
                    });
                }
            }
        }
        Err(e) => {
            state.error = Some(format!("Failed to get git status: {}", e));
        }
    }
}

/// Load git log into state
pub fn load_git_log(state: &mut GitState, cwd: &PathBuf) {
    state.log_entries.clear();
    state.error = None;
    state.scroll_offset = 0;
    state.selected_log = 0;

    let output = Command::new("git")
        .args(["log", "--oneline", "-20", "--format=%h|%an|%ar|%s"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.splitn(4, '|').collect();
                if parts.len() == 4 {
                    state.log_entries.push(GitLogEntry {
                        hash: parts[0].to_string(),
                        author: parts[1].to_string(),
                        date: parts[2].to_string(),
                        message: parts[3].to_string(),
                    });
                }
            }
        }
        Err(e) => {
            state.error = Some(format!("Failed to get git log: {}", e));
        }
    }
}

/// Load git diff into state
pub fn load_git_diff(state: &mut GitState, cwd: &PathBuf) {
    state.diff_content.clear();
    state.error = None;
    state.scroll_offset = 0;

    let output = Command::new("git").args(["diff"]).current_dir(cwd).output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            state.diff_content = stdout.lines().map(|s| s.to_string()).collect();
            if state.diff_content.is_empty() {
                state.diff_content.push("No unstaged changes".to_string());
            }
        }
        Err(e) => {
            state.error = Some(format!("Failed to get git diff: {}", e));
        }
    }
}

/// Toggle staging of a file
pub fn toggle_git_stage(state: &mut GitState, cwd: &PathBuf) {
    if state.files.is_empty() {
        return;
    }

    let file = &state.files[state.selected_file];
    let file_path = file.path.clone();
    let staged = file.staged;

    let args: Vec<&str> = if staged {
        vec!["reset", "HEAD", &file_path]
    } else {
        vec!["add", &file_path]
    };

    let _ = Command::new("git").args(&args).current_dir(cwd).output();

    // Reload status
    load_git_status(state, cwd);
}

/// Execute git commit
pub fn execute_git_commit(message: &str, cwd: &PathBuf) -> Result<(), String> {
    let output = Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Commit failed: {}", stderr))
            }
        }
        Err(e) => Err(format!("Failed to commit: {}", e)),
    }
}

/// Execute git push (legacy - use execute_git_push_to for remote selection)
#[allow(dead_code)]
pub fn execute_git_push(cwd: &PathBuf) -> Result<String, String> {
    let output = Command::new("git").args(["push"]).current_dir(cwd).output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                // Git push outputs to stderr for progress info
                let msg = if stdout.is_empty() {
                    if stderr.is_empty() {
                        "Push successful".to_string()
                    } else {
                        format!("Push successful: {}", stderr.lines().last().unwrap_or(""))
                    }
                } else {
                    stdout.to_string()
                };
                Ok(msg)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Push failed: {}", stderr))
            }
        }
        Err(e) => Err(format!("Failed to push: {}", e)),
    }
}

/// Execute git pull (legacy - use execute_git_pull_from for remote selection)
#[allow(dead_code)]
pub fn execute_git_pull(cwd: &PathBuf) -> Result<String, String> {
    let output = Command::new("git").args(["pull"]).current_dir(cwd).output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let msg = if stdout.trim().is_empty() {
                    "Already up to date".to_string()
                } else {
                    stdout
                        .lines()
                        .last()
                        .unwrap_or("Pull successful")
                        .to_string()
                };
                Ok(msg)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Pull failed: {}", stderr))
            }
        }
        Err(e) => Err(format!("Failed to pull: {}", e)),
    }
}

/// Load diff for a specific commit
pub fn load_commit_diff(state: &mut GitState, cwd: &PathBuf, commit_hash: &str) {
    state.diff_content.clear();
    state.error = None;
    state.scroll_offset = 0;

    let output = Command::new("git")
        .args(["show", "--stat", "--patch", commit_hash])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            state.diff_content = stdout.lines().map(|s| s.to_string()).collect();
            if state.diff_content.is_empty() {
                state.diff_content.push("No diff available".to_string());
            }
        }
        Err(e) => {
            state.error = Some(format!("Failed to get commit diff: {}", e));
        }
    }
}

/// Load diff for a specific file
pub fn load_file_diff(state: &mut GitState, cwd: &PathBuf, file_path: &str) {
    state.diff_content.clear();
    state.error = None;
    state.scroll_offset = 0;

    // First try unstaged changes
    let output = Command::new("git")
        .args(["diff", "--", file_path])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.is_empty() {
                // Try staged changes
                let staged_output = Command::new("git")
                    .args(["diff", "--cached", "--", file_path])
                    .current_dir(cwd)
                    .output();

                match staged_output {
                    Ok(staged_output) => {
                        let staged_stdout = String::from_utf8_lossy(&staged_output.stdout);
                        if staged_stdout.is_empty() {
                            state
                                .diff_content
                                .push(format!("No changes for: {}", file_path));
                        } else {
                            state.diff_content =
                                staged_stdout.lines().map(|s| s.to_string()).collect();
                        }
                    }
                    Err(e) => {
                        state.error = Some(format!("Failed to get staged diff: {}", e));
                    }
                }
            } else {
                state.diff_content = stdout.lines().map(|s| s.to_string()).collect();
            }
        }
        Err(e) => {
            state.error = Some(format!("Failed to get file diff: {}", e));
        }
    }
}

/// Check if a path is in a git repository
pub fn is_git_repo(cwd: &PathBuf) -> bool {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(cwd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Git status info for status bar display
#[derive(Debug, Clone)]
pub struct GitStatusInfo {
    pub branch: String,
    pub ahead: usize,
    pub behind: usize,
    pub staged: usize,
    pub modified: usize,
}

/// Quick status for status bar display
/// Returns GitStatusInfo or None if not in git repo
pub fn get_git_status_info(cwd: &PathBuf) -> Option<GitStatusInfo> {
    // Get branch and ahead/behind using status -sb
    let branch_output = Command::new("git")
        .args(["status", "-sb"])
        .current_dir(cwd)
        .output()
        .ok()?;

    if !branch_output.status.success() {
        return None;
    }

    let branch_stdout = String::from_utf8_lossy(&branch_output.stdout);
    let first_line = branch_stdout.lines().next().unwrap_or("");

    // Parse "## branch...origin/branch [ahead 1, behind 2]" or "## branch"
    let branch = first_line
        .strip_prefix("## ")
        .unwrap_or(first_line)
        .split("...")
        .next()
        .unwrap_or("unknown")
        .to_string();

    let mut ahead = 0;
    let mut behind = 0;
    if let Some(bracket_start) = first_line.find('[') {
        if let Some(bracket_end) = first_line.find(']') {
            let tracking_info = &first_line[bracket_start + 1..bracket_end];
            for part in tracking_info.split(", ") {
                if let Some(num) = part.strip_prefix("ahead ") {
                    ahead = num.parse().unwrap_or(0);
                } else if let Some(num) = part.strip_prefix("behind ") {
                    behind = num.parse().unwrap_or(0);
                }
            }
        }
    }

    // Get file status counts using porcelain
    let status_output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(cwd)
        .output()
        .ok()?;

    let status_stdout = String::from_utf8_lossy(&status_output.stdout);
    let mut modified = 0;
    let mut staged = 0;

    for line in status_stdout.lines() {
        if line.len() >= 2 {
            let index_char = line.chars().next().unwrap_or(' ');
            let worktree_char = line.chars().nth(1).unwrap_or(' ');

            // Index (staged) changes
            if index_char != ' ' && index_char != '?' {
                staged += 1;
            }
            // Worktree (unstaged) changes
            if worktree_char != ' ' {
                modified += 1;
            }
            // Untracked files count as modified
            if index_char == '?' {
                modified += 1;
            }
        }
    }

    Some(GitStatusInfo {
        branch,
        ahead,
        behind,
        staged,
        modified,
    })
}

/// Load git history for a specific file
/// Returns commits from newest to oldest, but we store them oldest to newest
/// so index 0 = oldest, last = newest (before working copy)
pub fn load_file_history(file_path: &PathBuf, cwd: &PathBuf) -> Vec<FileHistoryEntry> {
    // Get relative path from cwd
    let rel_path = file_path
        .strip_prefix(cwd)
        .unwrap_or(file_path)
        .to_string_lossy();

    let output = Command::new("git")
        .args([
            "log",
            "--follow",
            "--format=%H|%ar|%s",
            "-50", // Limit to 50 commits
            "--",
            &rel_path,
        ])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut entries: Vec<FileHistoryEntry> = stdout
                .lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.splitn(3, '|').collect();
                    if parts.len() == 3 {
                        Some(FileHistoryEntry {
                            hash: parts[0].to_string(),
                            date: parts[1].to_string(),
                            message: parts[2].to_string(),
                        })
                    } else {
                        None
                    }
                })
                .collect();
            // Reverse so oldest is first (index 0)
            entries.reverse();
            entries
        }
        Err(_) => Vec::new(),
    }
}

/// Load file content at a specific commit
pub fn load_file_at_commit(
    file_path: &PathBuf,
    commit_hash: &str,
    cwd: &PathBuf,
) -> Result<Vec<u8>, String> {
    // Get relative path from cwd
    let rel_path = file_path
        .strip_prefix(cwd)
        .unwrap_or(file_path)
        .to_string_lossy();

    let output = Command::new("git")
        .args(["show", &format!("{}:{}", commit_hash, rel_path)])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                Ok(output.stdout)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Failed to load file: {}", stderr))
            }
        }
        Err(e) => Err(format!("Git error: {}", e)),
    }
}

/// Load git blame for a file
/// Returns blame annotations for each line
pub fn load_file_blame(file_path: &PathBuf, cwd: &PathBuf) -> Vec<BlameLine> {
    // Get relative path from cwd
    let rel_path = file_path
        .strip_prefix(cwd)
        .unwrap_or(file_path)
        .to_string_lossy();

    let output = Command::new("git")
        .args(["blame", "--line-porcelain", "--", &rel_path])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if !output.status.success() {
                return Vec::new();
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut blame_lines = Vec::new();
            let mut current_hash = String::new();
            let mut current_author = String::new();
            let mut current_time_ago = String::new();

            for line in stdout.lines() {
                if line.len() == 40 && line.chars().all(|c| c.is_ascii_hexdigit()) {
                    // This is a commit hash line (first 40 chars)
                    current_hash = line[..7].to_string();
                } else if line.len() > 40 && line.chars().take(40).all(|c| c.is_ascii_hexdigit()) {
                    // Hash with line numbers
                    current_hash = line[..7].to_string();
                } else if let Some(author) = line.strip_prefix("author ") {
                    current_author = author.to_string();
                } else if let Some(time) = line.strip_prefix("author-time ") {
                    // Convert unix timestamp to relative time
                    if let Ok(timestamp) = time.parse::<i64>() {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs() as i64)
                            .unwrap_or(0);
                        let diff = now - timestamp;
                        current_time_ago = if diff < 60 {
                            "just now".to_string()
                        } else if diff < 3600 {
                            format!("{} min", diff / 60)
                        } else if diff < 86400 {
                            format!("{} hr", diff / 3600)
                        } else if diff < 2592000 {
                            format!("{} day", diff / 86400)
                        } else if diff < 31536000 {
                            format!("{} mo", diff / 2592000)
                        } else {
                            format!("{} yr", diff / 31536000)
                        };
                    }
                } else if let Some(content) = line.strip_prefix('\t') {
                    // This is the actual line content
                    blame_lines.push(BlameLine {
                        hash: current_hash.clone(),
                        author: current_author.clone(),
                        time_ago: current_time_ago.clone(),
                        line_content: content.to_string(),
                    });
                }
            }

            blame_lines
        }
        Err(_) => Vec::new(),
    }
}

/// Load git diff for a file against HEAD
/// Returns diff lines with +/- prefixes
pub fn load_file_diff_against_head(file_path: &PathBuf, cwd: &PathBuf) -> Vec<String> {
    // Get relative path from cwd
    let rel_path = file_path
        .strip_prefix(cwd)
        .unwrap_or(file_path)
        .to_string_lossy();

    // First try unstaged changes
    let output = Command::new("git")
        .args(["diff", "--", &rel_path])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.is_empty() {
                // Try staged changes
                let staged_output = Command::new("git")
                    .args(["diff", "--cached", "--", &rel_path])
                    .current_dir(cwd)
                    .output();

                match staged_output {
                    Ok(staged_output) => {
                        let staged_stdout = String::from_utf8_lossy(&staged_output.stdout);
                        if staged_stdout.is_empty() {
                            vec!["No changes compared to HEAD".to_string()]
                        } else {
                            staged_stdout.lines().map(|s| s.to_string()).collect()
                        }
                    }
                    Err(_) => vec!["Error loading staged diff".to_string()],
                }
            } else {
                stdout.lines().map(|s| s.to_string()).collect()
            }
        }
        Err(_) => vec!["Error loading diff".to_string()],
    }
}

// ============================================================================
// Branch Operations
// ============================================================================

/// Load branch list into state
pub fn load_branches(state: &mut GitState, cwd: &PathBuf) {
    state.branches.clear();
    state.error = None;
    state.selected_branch = 0;

    // Get local branches with last commit
    let output = Command::new("git")
        .args(["branch", "-v", "--no-color"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let is_current = line.starts_with('*');
                let line = line.trim_start_matches('*').trim();

                // Parse "branch_name hash commit_message"
                let parts: Vec<&str> = line.splitn(3, char::is_whitespace).collect();
                if parts.len() >= 2 {
                    let name = parts[0].to_string();
                    let last_commit = if parts.len() >= 3 {
                        format!("{} {}", parts[1], parts[2])
                    } else {
                        parts[1].to_string()
                    };

                    state.branches.push(GitBranch {
                        name,
                        is_current,
                        is_remote: false,
                        last_commit,
                    });
                }
            }
        }
        Err(e) => {
            state.error = Some(format!("Failed to list branches: {}", e));
        }
    }
}

/// Switch to a branch
pub fn switch_branch(branch_name: &str, cwd: &PathBuf) -> Result<String, String> {
    let output = Command::new("git")
        .args(["checkout", branch_name])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                Ok(format!("Switched to branch '{}'", branch_name))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Failed to switch: {}", stderr))
            }
        }
        Err(e) => Err(format!("Git error: {}", e)),
    }
}

/// Create a new branch
pub fn create_branch(branch_name: &str, cwd: &PathBuf) -> Result<String, String> {
    let output = Command::new("git")
        .args(["checkout", "-b", branch_name])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                Ok(format!("Created and switched to branch '{}'", branch_name))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Failed to create: {}", stderr))
            }
        }
        Err(e) => Err(format!("Git error: {}", e)),
    }
}

/// Delete a branch
pub fn delete_branch(branch_name: &str, cwd: &PathBuf) -> Result<String, String> {
    let output = Command::new("git")
        .args(["branch", "-d", branch_name])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                Ok(format!("Deleted branch '{}'", branch_name))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // Try force delete if normal delete fails
                if stderr.contains("not fully merged") {
                    Err(format!(
                        "Branch not merged. Use force delete? ({})",
                        stderr.trim()
                    ))
                } else {
                    Err(format!("Failed to delete: {}", stderr))
                }
            }
        }
        Err(e) => Err(format!("Git error: {}", e)),
    }
}

// ============================================================================
// Stash Operations
// ============================================================================

/// Load stash list into state
pub fn load_stashes(state: &mut GitState, cwd: &PathBuf) {
    state.stashes.clear();
    state.error = None;
    state.selected_stash = 0;

    let output = Command::new("git")
        .args(["stash", "list"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for (index, line) in stdout.lines().enumerate() {
                // Parse "stash@{0}: On branch: message"
                let parts: Vec<&str> = line.splitn(2, ": ").collect();
                if parts.len() >= 2 {
                    let branch_msg: Vec<&str> = parts[1].splitn(2, ": ").collect();
                    let (branch, message) = if branch_msg.len() >= 2 {
                        (
                            branch_msg[0].trim_start_matches("On ").to_string(),
                            branch_msg[1].to_string(),
                        )
                    } else {
                        ("".to_string(), parts[1].to_string())
                    };

                    state.stashes.push(GitStashEntry {
                        index,
                        message,
                        branch,
                    });
                }
            }
        }
        Err(e) => {
            state.error = Some(format!("Failed to list stashes: {}", e));
        }
    }
}

/// Create a new stash
pub fn create_stash(message: Option<&str>, cwd: &PathBuf) -> Result<String, String> {
    let mut args = vec!["stash", "push"];
    if let Some(msg) = message {
        args.push("-m");
        args.push(msg);
    }

    let output = Command::new("git").args(&args).current_dir(cwd).output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.contains("No local changes") {
                    Err("No local changes to stash".to_string())
                } else {
                    Ok("Changes stashed".to_string())
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Stash failed: {}", stderr))
            }
        }
        Err(e) => Err(format!("Git error: {}", e)),
    }
}

/// Pop the top stash
pub fn pop_stash(cwd: &PathBuf) -> Result<String, String> {
    let output = Command::new("git")
        .args(["stash", "pop"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                Ok("Stash applied and dropped".to_string())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Pop failed: {}", stderr))
            }
        }
        Err(e) => Err(format!("Git error: {}", e)),
    }
}

/// Apply a specific stash (don't drop it)
pub fn apply_stash(index: usize, cwd: &PathBuf) -> Result<String, String> {
    let stash_ref = format!("stash@{{{}}}", index);
    let output = Command::new("git")
        .args(["stash", "apply", &stash_ref])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                Ok(format!("Applied stash@{{{}}}", index))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Apply failed: {}", stderr))
            }
        }
        Err(e) => Err(format!("Git error: {}", e)),
    }
}

/// Drop a specific stash
pub fn drop_stash(index: usize, cwd: &PathBuf) -> Result<String, String> {
    let stash_ref = format!("stash@{{{}}}", index);
    let output = Command::new("git")
        .args(["stash", "drop", &stash_ref])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                Ok(format!("Dropped stash@{{{}}}", index))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Drop failed: {}", stderr))
            }
        }
        Err(e) => Err(format!("Git error: {}", e)),
    }
}

// ============================================================================
// Tag Operations
// ============================================================================

/// Load tag list into state
pub fn load_tags(state: &mut GitState, cwd: &PathBuf) {
    state.tags.clear();
    state.error = None;
    state.selected_tag = 0;

    // Get tags with commit info
    let output = Command::new("git")
        .args([
            "tag",
            "-l",
            "--format=%(refname:short)|%(objectname:short)|%(contents:subject)",
        ])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.splitn(3, '|').collect();
                if !parts.is_empty() {
                    state.tags.push(GitTag {
                        name: parts[0].to_string(),
                        commit: parts.get(1).unwrap_or(&"").to_string(),
                        message: parts
                            .get(2)
                            .map(|s| s.to_string())
                            .filter(|s| !s.is_empty()),
                    });
                }
            }
            // Sort tags reverse alphabetically (newest version first for semver)
            state.tags.sort_by(|a, b| b.name.cmp(&a.name));
        }
        Err(e) => {
            state.error = Some(format!("Failed to list tags: {}", e));
        }
    }
}

/// Create a new tag (lightweight)
pub fn create_tag(tag_name: &str, cwd: &PathBuf) -> Result<String, String> {
    let output = Command::new("git")
        .args(["tag", tag_name])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                Ok(format!("Created tag '{}'", tag_name))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Failed to create: {}", stderr))
            }
        }
        Err(e) => Err(format!("Git error: {}", e)),
    }
}

/// Delete a tag
pub fn delete_tag(tag_name: &str, cwd: &PathBuf) -> Result<String, String> {
    let output = Command::new("git")
        .args(["tag", "-d", tag_name])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                Ok(format!("Deleted tag '{}'", tag_name))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Failed to delete: {}", stderr))
            }
        }
        Err(e) => Err(format!("Git error: {}", e)),
    }
}

/// Push tags to remote
pub fn push_tags(cwd: &PathBuf) -> Result<String, String> {
    let output = Command::new("git")
        .args(["push", "--tags"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                Ok("Tags pushed to remote".to_string())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Push failed: {}", stderr))
            }
        }
        Err(e) => Err(format!("Git error: {}", e)),
    }
}

// ============================================================
// Remote Operations
// ============================================================

/// Load git remotes into state
pub fn load_remotes(state: &mut GitState, cwd: &PathBuf) {
    state.remotes.clear();
    state.selected_remote = 0;

    // Get remote names and URLs
    let output = Command::new("git")
        .args(["remote", "-v"])
        .current_dir(cwd)
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut seen = std::collections::HashSet::new();

            for line in stdout.lines() {
                // Format: "origin  https://github.com/user/repo.git (fetch)"
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let name = parts[0].to_string();
                    // Only add each remote once (skip the (push) duplicate)
                    if !seen.contains(&name) {
                        seen.insert(name.clone());
                        let url = parts[1].to_string();
                        state.remotes.push(GitRemote { name, url });
                    }
                }
            }
        }
    }
}

/// Execute git push to a specific remote
pub fn execute_git_push_to(remote: &str, cwd: &PathBuf) -> Result<String, String> {
    // Get current branch name
    let branch_output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(cwd)
        .output();

    let branch = match branch_output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => "HEAD".to_string(),
    };

    let output = Command::new("git")
        .args(["push", remote, &branch])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let msg = if stderr.is_empty() {
                    format!("Pushed to {} successfully", remote)
                } else {
                    format!(
                        "Pushed to {}: {}",
                        remote,
                        stderr.lines().last().unwrap_or("")
                    )
                };
                Ok(msg)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Push to {} failed: {}", remote, stderr))
            }
        }
        Err(e) => Err(format!("Failed to push: {}", e)),
    }
}

/// Execute git pull from a specific remote
pub fn execute_git_pull_from(remote: &str, cwd: &PathBuf) -> Result<String, String> {
    // Get current branch name
    let branch_output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(cwd)
        .output();

    let branch = match branch_output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => "HEAD".to_string(),
    };

    let output = Command::new("git")
        .args(["pull", remote, &branch])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let msg = if stdout.trim().is_empty() {
                    format!("Already up to date with {}", remote)
                } else {
                    format!(
                        "Pulled from {}: {}",
                        remote,
                        stdout.lines().last().unwrap_or("success")
                    )
                };
                Ok(msg)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Pull from {} failed: {}", remote, stderr))
            }
        }
        Err(e) => Err(format!("Failed to pull: {}", e)),
    }
}

// ============================================================
// Config Operations
// ============================================================

/// Load git config into state
pub fn load_git_config(state: &mut GitState, cwd: &PathBuf) {
    state.config_entries.clear();
    state.selected_config = 0;

    // Load config from all scopes
    for (scope, args) in [
        ("local", vec!["config", "--local", "--list"]),
        ("global", vec!["config", "--global", "--list"]),
        ("system", vec!["config", "--system", "--list"]),
    ] {
        let output = Command::new("git").args(&args).current_dir(cwd).output();

        if let Ok(output) = output {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    // Format: "key=value"
                    if let Some((key, value)) = line.split_once('=') {
                        state.config_entries.push(GitConfigEntry {
                            key: key.to_string(),
                            value: value.to_string(),
                            scope: scope.to_string(),
                        });
                    }
                }
            }
        }
    }

    // Sort by key for easier reading
    state.config_entries.sort_by(|a, b| a.key.cmp(&b.key));
}

// ============================================================
// Conflict Resolution Operations
// ============================================================

/// Check if there are merge conflicts
#[allow(dead_code)]
pub fn has_conflicts(cwd: &PathBuf) -> bool {
    let output = Command::new("git")
        .args(["diff", "--name-only", "--diff-filter=U"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => output.status.success() && !output.stdout.is_empty(),
        Err(_) => false,
    }
}

/// Load list of conflicting files
pub fn load_conflict_files(state: &mut GitState, cwd: &PathBuf) {
    state.conflict_files.clear();
    state.selected_conflict_file = 0;
    state.error = None;

    let output = Command::new("git")
        .args(["diff", "--name-only", "--diff-filter=U"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for path in stdout.lines() {
                    if !path.is_empty() {
                        // Parse conflict sections for this file
                        let sections = parse_conflict_sections(cwd, path);
                        state.conflict_files.push(ConflictFile {
                            path: path.to_string(),
                            sections,
                            selected_section: 0,
                        });
                    }
                }
            }
        }
        Err(e) => {
            state.error = Some(format!("Failed to check conflicts: {}", e));
        }
    }
}

/// Parse conflict sections from a file
fn parse_conflict_sections(cwd: &PathBuf, file_path: &str) -> Vec<ConflictSection> {
    let full_path = cwd.join(file_path);
    let content = match std::fs::read_to_string(&full_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut sections = Vec::new();
    let mut in_conflict = false;
    let mut in_ours = false;
    let mut current_section: Option<ConflictSection> = None;

    for (line_num, line) in content.lines().enumerate() {
        if line.starts_with("<<<<<<<") {
            in_conflict = true;
            in_ours = true;
            current_section = Some(ConflictSection {
                start_line: line_num + 1,
                ours: Vec::new(),
                theirs: Vec::new(),
                resolved: None,
            });
        } else if line.starts_with("=======") && in_conflict {
            in_ours = false;
        } else if line.starts_with(">>>>>>>") && in_conflict {
            if let Some(section) = current_section.take() {
                sections.push(section);
            }
            in_conflict = false;
        } else if in_conflict {
            if let Some(ref mut section) = current_section {
                if in_ours {
                    section.ours.push(line.to_string());
                } else {
                    section.theirs.push(line.to_string());
                }
            }
        }
    }

    sections
}

/// Resolve a conflict section with specified resolution
pub fn resolve_conflict_section(
    file_path: &str,
    section_idx: usize,
    resolution: ConflictResolution,
    cwd: &PathBuf,
) -> Result<String, String> {
    let full_path = cwd.join(file_path);
    let content =
        std::fs::read_to_string(&full_path).map_err(|e| format!("Failed to read file: {}", e))?;

    let mut new_content = String::new();
    let mut in_conflict = false;
    let mut in_ours = false;
    let mut current_section_idx = 0;
    let mut ours_lines: Vec<String> = Vec::new();
    let mut theirs_lines: Vec<String> = Vec::new();

    for line in content.lines() {
        if line.starts_with("<<<<<<<") {
            in_conflict = true;
            in_ours = true;
            ours_lines.clear();
            theirs_lines.clear();
        } else if line.starts_with("=======") && in_conflict {
            in_ours = false;
        } else if line.starts_with(">>>>>>>") && in_conflict {
            // End of conflict - apply resolution if this is the target section
            if current_section_idx == section_idx {
                match resolution {
                    ConflictResolution::Ours => {
                        for l in &ours_lines {
                            new_content.push_str(l);
                            new_content.push('\n');
                        }
                    }
                    ConflictResolution::Theirs => {
                        for l in &theirs_lines {
                            new_content.push_str(l);
                            new_content.push('\n');
                        }
                    }
                    ConflictResolution::Both => {
                        for l in &ours_lines {
                            new_content.push_str(l);
                            new_content.push('\n');
                        }
                        for l in &theirs_lines {
                            new_content.push_str(l);
                            new_content.push('\n');
                        }
                    }
                }
            } else {
                // Keep the conflict markers for unresolved sections
                new_content.push_str("<<<<<<<\n");
                for l in &ours_lines {
                    new_content.push_str(l);
                    new_content.push('\n');
                }
                new_content.push_str("=======\n");
                for l in &theirs_lines {
                    new_content.push_str(l);
                    new_content.push('\n');
                }
                new_content.push_str(">>>>>>>\n");
            }
            current_section_idx += 1;
            in_conflict = false;
        } else if in_conflict {
            if in_ours {
                ours_lines.push(line.to_string());
            } else {
                theirs_lines.push(line.to_string());
            }
        } else {
            new_content.push_str(line);
            new_content.push('\n');
        }
    }

    // Write the resolved content back
    std::fs::write(&full_path, &new_content).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(format!(
        "Resolved conflict {} in {}",
        section_idx + 1,
        file_path
    ))
}

/// Mark a file as resolved (stage it)
pub fn mark_conflict_resolved(file_path: &str, cwd: &PathBuf) -> Result<String, String> {
    let output = Command::new("git")
        .args(["add", file_path])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                Ok(format!("Marked {} as resolved", file_path))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Failed to stage: {}", stderr))
            }
        }
        Err(e) => Err(format!("Git error: {}", e)),
    }
}

/// Abort the current merge
pub fn abort_merge(cwd: &PathBuf) -> Result<String, String> {
    let output = Command::new("git")
        .args(["merge", "--abort"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                Ok("Merge aborted".to_string())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Failed to abort: {}", stderr))
            }
        }
        Err(e) => Err(format!("Git error: {}", e)),
    }
}

// ============================================================
// Submodule Operations
// ============================================================

/// Load git submodules into state
pub fn load_submodules(state: &mut GitState, cwd: &PathBuf) {
    state.submodules.clear();
    state.selected_submodule = 0;
    state.error = None;

    // First, get submodule URLs from config
    let config_output = Command::new("git")
        .args([
            "config",
            "--file",
            ".gitmodules",
            "--get-regexp",
            "submodule\\..*\\.url",
        ])
        .current_dir(cwd)
        .output();

    let mut url_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if let Ok(output) = config_output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                // Format: "submodule.path/to/submod.url https://github.com/..."
                let parts: Vec<&str> = line.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    // Extract submodule name from "submodule.NAME.url"
                    if let Some(name) = parts[0]
                        .strip_prefix("submodule.")
                        .and_then(|s| s.strip_suffix(".url"))
                    {
                        url_map.insert(name.to_string(), parts[1].to_string());
                    }
                }
            }
        }
    }

    // Get submodule status
    let output = Command::new("git")
        .args(["submodule", "status", "--recursive"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if line.is_empty() {
                        continue;
                    }

                    // Parse status line: " abc1234 path (branch)" or "-abc1234 path" or "+abc1234 path"
                    let status_char = line.chars().next().unwrap_or(' ');
                    let status = match status_char {
                        '-' => SubmoduleStatus::Uninitialized,
                        '+' => SubmoduleStatus::Modified,
                        'U' => SubmoduleStatus::Conflict,
                        _ => SubmoduleStatus::Initialized,
                    };

                    // Skip status char and parse rest
                    let rest = line.trim_start_matches(['-', '+', 'U', ' ']);
                    let parts: Vec<&str> = rest.split_whitespace().collect();

                    if parts.len() >= 2 {
                        let commit = parts[0].to_string();
                        let path = parts[1].to_string();
                        let name = path.clone(); // Use path as name
                        let url = url_map.get(&path).cloned().unwrap_or_default();

                        state.submodules.push(GitSubmodule {
                            name,
                            path,
                            url,
                            status,
                            commit,
                        });
                    }
                }
            }
        }
        Err(e) => {
            state.error = Some(format!("Failed to list submodules: {}", e));
        }
    }
}

/// Initialize a submodule
pub fn init_submodule(path: Option<&str>, cwd: &PathBuf) -> Result<String, String> {
    let mut args = vec!["submodule", "init"];
    if let Some(p) = path {
        args.push(p);
    }

    let output = Command::new("git").args(&args).current_dir(cwd).output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let msg = if let Some(p) = path {
                    format!("Initialized submodule '{}'", p)
                } else {
                    "Initialized all submodules".to_string()
                };
                Ok(msg)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Init failed: {}", stderr))
            }
        }
        Err(e) => Err(format!("Git error: {}", e)),
    }
}

/// Update a submodule
pub fn update_submodule(path: Option<&str>, cwd: &PathBuf) -> Result<String, String> {
    let mut args = vec!["submodule", "update", "--init"];
    if let Some(p) = path {
        args.push(p);
    }

    let output = Command::new("git").args(&args).current_dir(cwd).output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let msg = if let Some(p) = path {
                    format!("Updated submodule '{}'", p)
                } else {
                    "Updated all submodules".to_string()
                };
                Ok(msg)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Update failed: {}", stderr))
            }
        }
        Err(e) => Err(format!("Git error: {}", e)),
    }
}

/// Sync submodule URLs
pub fn sync_submodules(cwd: &PathBuf) -> Result<String, String> {
    let output = Command::new("git")
        .args(["submodule", "sync", "--recursive"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                Ok("Synced submodule URLs".to_string())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Sync failed: {}", stderr))
            }
        }
        Err(e) => Err(format!("Git error: {}", e)),
    }
}
