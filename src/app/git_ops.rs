//! Git Operations Module
//!
//! Provides helper functions for Git integration in Q-DOS II.
//! These are standalone functions to avoid borrow checker issues in the main app.

use super::state::{GitFileStatus, GitLogEntry, GitState};
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

/// Execute git push
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

/// Execute git pull
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
