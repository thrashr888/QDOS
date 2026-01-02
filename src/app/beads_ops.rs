//! Beads Operations Module
//!
//! Provides helper functions for Beads issue tracker integration in Q-DOS II.
//! These are standalone functions to avoid borrow checker issues in the main app.

use super::state::{BeadsIssue, BeadsState};
use std::path::PathBuf;
use std::process::Command;

/// Load beads issues list
pub fn load_beads_list(state: &mut BeadsState, cwd: &PathBuf, _status_filter: Option<&str>) {
    state.issues.clear();
    state.error = None;
    state.selected_issue = 0;

    let output = Command::new("bd")
        .args(["list", "--status=open"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines().skip(1) {
                // Skip header
                let parts: Vec<&str> = line.split('\t').collect();
                if !parts.is_empty() {
                    state.issues.push(BeadsIssue {
                        id: parts.first().unwrap_or(&"").to_string(),
                        title: parts.get(1).unwrap_or(&"").to_string(),
                        status: parts.get(2).unwrap_or(&"open").to_string(),
                        priority: parts.get(3).unwrap_or(&"2").to_string(),
                        issue_type: parts.get(4).unwrap_or(&"task").to_string(),
                        blocked_by: Vec::new(),
                    });
                }
            }
        }
        Err(e) => {
            state.error = Some(format!("Failed to load issues: {}", e));
        }
    }
}

/// Load ready issues
pub fn load_beads_ready(state: &mut BeadsState, cwd: &PathBuf) {
    state.issues.clear();
    state.error = None;
    state.selected_issue = 0;

    let output = Command::new("bd")
        .args(["ready"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines().skip(1) {
                let parts: Vec<&str> = line.split('\t').collect();
                if !parts.is_empty() {
                    state.issues.push(BeadsIssue {
                        id: parts.first().unwrap_or(&"").to_string(),
                        title: parts.get(1).unwrap_or(&"").to_string(),
                        status: "open".to_string(),
                        priority: parts.get(2).unwrap_or(&"2").to_string(),
                        issue_type: "task".to_string(),
                        blocked_by: Vec::new(),
                    });
                }
            }
        }
        Err(e) => {
            state.error = Some(format!("Failed to load ready issues: {}", e));
        }
    }
}

/// Load blocked issues
pub fn load_beads_blocked(state: &mut BeadsState, cwd: &PathBuf) {
    state.issues.clear();
    state.error = None;
    state.selected_issue = 0;

    let output = Command::new("bd")
        .args(["blocked"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines().skip(1) {
                let parts: Vec<&str> = line.split('\t').collect();
                if !parts.is_empty() {
                    state.issues.push(BeadsIssue {
                        id: parts.first().unwrap_or(&"").to_string(),
                        title: parts.get(1).unwrap_or(&"").to_string(),
                        status: "blocked".to_string(),
                        priority: parts.get(2).unwrap_or(&"2").to_string(),
                        issue_type: "task".to_string(),
                        blocked_by: Vec::new(),
                    });
                }
            }
        }
        Err(e) => {
            state.error = Some(format!("Failed to load blocked issues: {}", e));
        }
    }
}

/// Load beads stats
pub fn load_beads_stats(state: &mut BeadsState, cwd: &PathBuf) {
    state.error = None;

    let output = Command::new("bd")
        .args(["stats"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Parse stats output - basic parsing
            for line in stdout.lines() {
                if let Some(num_str) = line.split(':').nth(1) {
                    let num = num_str.trim().parse::<usize>().unwrap_or(0);
                    if line.contains("Total") {
                        state.stats.total = num;
                    } else if line.contains("Open") {
                        state.stats.open = num;
                    } else if line.contains("In Progress") {
                        state.stats.in_progress = num;
                    } else if line.contains("Closed") {
                        state.stats.closed = num;
                    } else if line.contains("Blocked") {
                        state.stats.blocked = num;
                    }
                }
            }
        }
        Err(e) => {
            state.error = Some(format!("Failed to load stats: {}", e));
        }
    }
}

/// Execute beads create
pub fn execute_beads_create(
    title: &str,
    issue_type_idx: usize,
    priority: usize,
    cwd: &PathBuf,
) -> Result<(), String> {
    let issue_types = ["task", "bug", "feature"];
    let issue_type = issue_types[issue_type_idx];

    let output = Command::new("bd")
        .args([
            "create",
            "--title",
            title,
            "--type",
            issue_type,
            "--priority",
            &priority.to_string(),
        ])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Create failed: {}", stderr))
            }
        }
        Err(e) => Err(format!("Failed to create issue: {}", e)),
    }
}

/// Execute beads close
pub fn execute_beads_close(issue_id: &str, cwd: &PathBuf) -> Result<String, String> {
    let output = Command::new("bd")
        .args(["close", issue_id])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                Ok(format!("Issue {} closed", issue_id))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Close failed: {}", stderr))
            }
        }
        Err(e) => Err(format!("Failed to close issue: {}", e)),
    }
}

/// Execute beads update status
pub fn execute_beads_update_status(issue_id: &str, status: &str, cwd: &PathBuf) -> Result<String, String> {
    let output = Command::new("bd")
        .args(["update", issue_id, &format!("--status={}", status)])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                Ok(format!("Issue {} updated to {}", issue_id, status))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Update failed: {}", stderr))
            }
        }
        Err(e) => Err(format!("Failed to update issue: {}", e)),
    }
}
