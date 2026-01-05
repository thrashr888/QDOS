//! Jj Plugin Operations
//!
//! CLI operations for the jj (Jujutsu) VCS plugin.

use super::state::{JjBookmark, JjChange, JjFileStatus, JjOperation};
use std::path::Path;
use std::process::Command;

/// Check if jj is available on the system
pub fn is_jj_available() -> bool {
    Command::new("jj")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if the given directory is a jj repository
pub fn is_jj_repo(cwd: &Path) -> bool {
    cwd.join(".jj").is_dir()
}

/// Get status info for the status bar
pub fn get_jj_status_info(cwd: &Path) -> Option<(String, bool)> {
    if !is_jj_repo(cwd) {
        return None;
    }

    // Get working copy change ID
    let output = Command::new("jj")
        .args([
            "log",
            "-r",
            "@",
            "-T",
            r#"change_id.shortest(8)"#,
            "--no-graph",
        ])
        .current_dir(cwd)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let change_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Check if there are changes
    let status_output = Command::new("jj")
        .args(["status"])
        .current_dir(cwd)
        .output()
        .ok()?;

    let has_changes =
        !String::from_utf8_lossy(&status_output.stdout).contains("The working copy has no changes");

    Some((change_id, has_changes))
}

/// Load jj status (working copy info and file changes)
#[allow(clippy::type_complexity)]
pub fn load_jj_status(
    cwd: &Path,
) -> Result<(Option<JjChange>, Option<JjChange>, Vec<JjFileStatus>), String> {
    // Get working copy info
    let wc_output = Command::new("jj")
        .args([
            "log",
            "-r",
            "@",
            "-T",
            r#"change_id.shortest(8) ++ "\t" ++ commit_id.shortest(8) ++ "\t" ++ author.email() ++ "\t" ++ committer.timestamp().ago() ++ "\t" ++ if(empty, "true", "false") ++ "\t" ++ if(description, description.first_line(), "(no description set)")"#,
            "--no-graph",
        ])
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("Failed to run jj: {}", e))?;

    let working_copy = if wc_output.status.success() {
        parse_change_line(&String::from_utf8_lossy(&wc_output.stdout), true)
    } else {
        None
    };

    // Get parent info
    let parent_output = Command::new("jj")
        .args([
            "log",
            "-r",
            "@-",
            "-T",
            r#"change_id.shortest(8) ++ "\t" ++ commit_id.shortest(8) ++ "\t" ++ author.email() ++ "\t" ++ committer.timestamp().ago() ++ "\t" ++ if(empty, "true", "false") ++ "\t" ++ if(description, description.first_line(), "(no description set)")"#,
            "--no-graph",
        ])
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("Failed to run jj: {}", e))?;

    let parent = if parent_output.status.success() {
        parse_change_line(&String::from_utf8_lossy(&parent_output.stdout), false)
    } else {
        None
    };

    // Get file changes
    let diff_output = Command::new("jj")
        .args(["diff", "--stat"])
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("Failed to run jj diff: {}", e))?;

    let files = if diff_output.status.success() {
        parse_diff_stat(&String::from_utf8_lossy(&diff_output.stdout))
    } else {
        Vec::new()
    };

    Ok((working_copy, parent, files))
}

fn parse_change_line(line: &str, is_working_copy: bool) -> Option<JjChange> {
    let parts: Vec<&str> = line.trim().split('\t').collect();
    if parts.len() >= 6 {
        Some(JjChange {
            change_id: parts[0].to_string(),
            commit_id: parts[1].to_string(),
            author: parts[2].to_string(),
            date: parts[3].to_string(),
            is_empty: parts[4] == "true",
            description: parts[5].to_string(),
            is_working_copy,
        })
    } else {
        None
    }
}

fn parse_diff_stat(output: &str) -> Vec<JjFileStatus> {
    let mut files = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() || line.contains("changed,") || line.contains("file changed") {
            continue;
        }
        // Format: "path/to/file | 5 +++++"
        if let Some(pipe_idx) = line.find('|') {
            let path = line[..pipe_idx].trim().to_string();
            let stats = &line[pipe_idx + 1..];
            let status = if stats.contains('+') && stats.contains('-') {
                'M'
            } else if stats.contains('+') {
                'A'
            } else if stats.contains('-') {
                'D'
            } else {
                'M'
            };
            files.push(JjFileStatus { path, status });
        }
    }
    files
}

/// Load jj log (revision history)
pub fn load_jj_log(cwd: &Path) -> Result<Vec<JjChange>, String> {
    let output = Command::new("jj")
        .args([
            "log",
            "-r",
            "ancestors(@, 20)",
            "-T",
            r#"change_id.shortest(8) ++ "\t" ++ commit_id.shortest(8) ++ "\t" ++ author.email() ++ "\t" ++ committer.timestamp().ago() ++ "\t" ++ if(empty, "true", "false") ++ "\t" ++ if(current_working_copy, "true", "false") ++ "\t" ++ if(description, description.first_line(), "(no description set)") ++ "\n""#,
            "--no-graph",
        ])
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("Failed to run jj log: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let mut changes = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let parts: Vec<&str> = line.trim().split('\t').collect();
        if parts.len() >= 7 {
            changes.push(JjChange {
                change_id: parts[0].to_string(),
                commit_id: parts[1].to_string(),
                author: parts[2].to_string(),
                date: parts[3].to_string(),
                is_empty: parts[4] == "true",
                is_working_copy: parts[5] == "true",
                description: parts[6].to_string(),
            });
        }
    }

    Ok(changes)
}

/// Load jj diff
pub fn load_jj_diff(cwd: &Path) -> Result<Vec<String>, String> {
    let output = Command::new("jj")
        .args(["diff", "--git"])
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("Failed to run jj diff: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let content = String::from_utf8_lossy(&output.stdout);
    Ok(content.lines().map(|s| s.to_string()).collect())
}

/// Load diff for a specific change
pub fn load_change_diff(cwd: &Path, change_id: &str) -> Result<Vec<String>, String> {
    let output = Command::new("jj")
        .args(["diff", "-r", change_id, "--git"])
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("Failed to run jj diff: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let content = String::from_utf8_lossy(&output.stdout);
    Ok(content.lines().map(|s| s.to_string()).collect())
}

/// Update change description
pub fn describe_change(cwd: &Path, description: &str) -> Result<(), String> {
    let output = Command::new("jj")
        .args(["describe", "-m", description])
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("Failed to run jj describe: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    Ok(())
}

/// Create a new change
pub fn create_new_change(cwd: &Path) -> Result<(), String> {
    let output = Command::new("jj")
        .args(["new"])
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("Failed to run jj new: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    Ok(())
}

/// Load bookmarks
pub fn load_bookmarks(cwd: &Path) -> Result<Vec<JjBookmark>, String> {
    let output = Command::new("jj")
        .args(["bookmark", "list", "--all-remotes"])
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("Failed to run jj bookmark list: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let mut bookmarks = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if let Some(bookmark) = parse_bookmark_line(line) {
            bookmarks.push(bookmark);
        }
    }

    Ok(bookmarks)
}

fn parse_bookmark_line(line: &str) -> Option<JjBookmark> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    // Format: "name: change_id commit_id" or "name@remote: change_id commit_id"
    let (name_part, rest) = line.split_once(':')?;
    let name_part = name_part.trim();
    let rest = rest.trim();

    let (name, remote) = if name_part.contains('@') {
        let (n, r) = name_part.split_once('@')?;
        (n.to_string(), Some(r.to_string()))
    } else {
        (name_part.to_string(), None)
    };

    let target = rest.split_whitespace().next().unwrap_or("").to_string();
    let is_conflicted = line.contains("(conflicted)");

    Some(JjBookmark {
        name,
        target,
        is_remote: remote.is_some(),
        remote,
        is_conflicted,
    })
}

/// Create a new bookmark
pub fn create_bookmark(cwd: &Path, name: &str) -> Result<(), String> {
    let output = Command::new("jj")
        .args(["bookmark", "create", name])
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("Failed to run jj bookmark create: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    Ok(())
}

/// Delete a bookmark
pub fn delete_bookmark(cwd: &Path, name: &str) -> Result<(), String> {
    let output = Command::new("jj")
        .args(["bookmark", "delete", name])
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("Failed to run jj bookmark delete: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    Ok(())
}

/// Load operation log
pub fn load_operations(cwd: &Path) -> Result<Vec<JjOperation>, String> {
    let output = Command::new("jj")
        .args([
            "operation",
            "log",
            "-T",
            r#"id.short(8) ++ "\t" ++ if(current_operation, "true", "false") ++ "\t" ++ time.start().ago() ++ "\t" ++ description ++ "\n""#,
            "--limit",
            "20",
        ])
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("Failed to run jj operation log: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let mut operations = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let parts: Vec<&str> = line.trim().split('\t').collect();
        if parts.len() >= 4 {
            operations.push(JjOperation {
                id: parts[0].to_string(),
                is_current: parts[1] == "true",
                time: parts[2].to_string(),
                description: parts[3].to_string(),
            });
        }
    }

    Ok(operations)
}

/// Undo the last operation
pub fn undo_operation(cwd: &Path) -> Result<(), String> {
    let output = Command::new("jj")
        .args(["undo"])
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("Failed to run jj undo: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    Ok(())
}

/// Git fetch
pub fn git_fetch(cwd: &Path) -> Result<String, String> {
    let output = Command::new("jj")
        .args(["git", "fetch"])
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("Failed to run jj git fetch: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Git push
pub fn git_push(cwd: &Path) -> Result<String, String> {
    let output = Command::new("jj")
        .args(["git", "push"])
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("Failed to run jj git push: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
