//! Beads Operations Module
//!
//! Provides helper functions for Beads issue tracker integration in Q-DOS II.
//! These are standalone functions to avoid borrow checker issues in the main app.

use super::state::{BeadsComment, BeadsIssue, BeadsState, BeadsSubIssue};
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Command;

/// JSON structure for beads issue from bd list --json
#[derive(Debug, Deserialize)]
struct BeadsJsonIssue {
    id: String,
    title: String,
    #[serde(default)]
    description: Option<String>,
    status: String,
    priority: u8,
    issue_type: String,
    #[serde(default)]
    blocked_by: Option<Vec<String>>,
    #[serde(default)]
    dependents: Option<Vec<BeadsJsonDependent>>,
}

/// JSON structure for dependent issues (epic children)
#[derive(Debug, Deserialize)]
struct BeadsJsonDependent {
    id: String,
    title: String,
    status: String,
    issue_type: String,
}

/// JSON structure for comments
#[derive(Debug, Deserialize)]
struct BeadsJsonComment {
    author: String,
    text: String,
    created_at: String,
}

/// Parse JSON output from bd commands
fn parse_beads_json(stdout: &str) -> Result<Vec<BeadsIssue>, String> {
    let json_issues: Vec<BeadsJsonIssue> =
        serde_json::from_str(stdout).map_err(|e| format!("JSON parse error: {}", e))?;

    Ok(json_issues
        .into_iter()
        .map(|j| BeadsIssue {
            id: j.id,
            title: j.title,
            description: j.description,
            status: j.status,
            priority: j.priority.to_string(),
            issue_type: j.issue_type,
            blocked_by: j.blocked_by.unwrap_or_default(),
            dependents: j
                .dependents
                .unwrap_or_default()
                .into_iter()
                .map(|d| BeadsSubIssue {
                    id: d.id,
                    title: d.title,
                    status: d.status,
                    issue_type: d.issue_type,
                })
                .collect(),
            comments: Vec::new(), // Comments loaded separately
        })
        .collect())
}

/// Load comments for an issue
fn load_comments(issue_id: &str, cwd: &PathBuf) -> Vec<BeadsComment> {
    let output = Command::new("bd")
        .args(["comments", issue_id, "--json"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Ok(comments) = serde_json::from_str::<Vec<BeadsJsonComment>>(&stdout) {
                comments
                    .into_iter()
                    .map(|c| BeadsComment {
                        author: c.author,
                        text: c.text,
                        created_at: c.created_at,
                    })
                    .collect()
            } else {
                Vec::new()
            }
        }
        Err(_) => Vec::new(),
    }
}

/// Load beads issues list
pub fn load_beads_list(state: &mut BeadsState, cwd: &PathBuf, _status_filter: Option<&str>) {
    state.issues.clear();
    state.error = None;
    state.selected_issue = 0;

    let output = Command::new("bd")
        .args(["list", "--status=open", "--json"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            match parse_beads_json(&stdout) {
                Ok(issues) => state.issues = issues,
                Err(e) => state.error = Some(e),
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
        .args(["ready", "--json"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            match parse_beads_json(&stdout) {
                Ok(issues) => state.issues = issues,
                Err(e) => state.error = Some(e),
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
        .args(["blocked", "--json"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            match parse_beads_json(&stdout) {
                Ok(issues) => state.issues = issues,
                Err(e) => state.error = Some(e),
            }
        }
        Err(e) => {
            state.error = Some(format!("Failed to load blocked issues: {}", e));
        }
    }
}

/// JSON structure for beads stats from bd stats --json
#[derive(Debug, Deserialize)]
struct BeadsStatsJson {
    summary: BeadsStatsSummary,
}

#[derive(Debug, Deserialize)]
struct BeadsStatsSummary {
    total_issues: usize,
    open_issues: usize,
    in_progress_issues: usize,
    closed_issues: usize,
    blocked_issues: usize,
}

/// Load beads stats
pub fn load_beads_stats(state: &mut BeadsState, cwd: &PathBuf) {
    state.error = None;

    let output = Command::new("bd")
        .args(["stats", "--json"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            match serde_json::from_str::<BeadsStatsJson>(&stdout) {
                Ok(stats) => {
                    state.stats.total = stats.summary.total_issues;
                    state.stats.open = stats.summary.open_issues;
                    state.stats.in_progress = stats.summary.in_progress_issues;
                    state.stats.closed = stats.summary.closed_issues;
                    state.stats.blocked = stats.summary.blocked_issues;
                }
                Err(e) => {
                    state.error = Some(format!("JSON parse error: {}", e));
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
pub fn execute_beads_update_status(
    issue_id: &str,
    status: &str,
    cwd: &PathBuf,
) -> Result<String, String> {
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

/// Execute beads reopen
pub fn execute_beads_reopen(issue_id: &str, cwd: &PathBuf) -> Result<String, String> {
    let output = Command::new("bd")
        .args(["reopen", issue_id])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                Ok(format!("Issue {} reopened", issue_id))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Reopen failed: {}", stderr))
            }
        }
        Err(e) => Err(format!("Failed to reopen issue: {}", e)),
    }
}

/// Execute beads sync
pub fn execute_beads_sync(cwd: &PathBuf) -> Result<String, String> {
    let output = Command::new("bd").args(["sync"]).current_dir(cwd).output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Get the last non-empty line for status
                let msg = stdout
                    .lines()
                    .rfind(|l| !l.is_empty())
                    .unwrap_or("Sync complete")
                    .to_string();
                Ok(msg)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Sync failed: {}", stderr))
            }
        }
        Err(e) => Err(format!("Failed to sync: {}", e)),
    }
}

/// Load detailed issue info (including dependents for epics and comments)
pub fn load_beads_issue_detail(issue_id: &str, cwd: &PathBuf) -> Result<BeadsIssue, String> {
    let output = Command::new("bd")
        .args(["show", issue_id, "--json"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let issues = parse_beads_json(&stdout)?;
            let mut issue = issues
                .into_iter()
                .next()
                .ok_or_else(|| "Issue not found".to_string())?;

            // Load comments separately
            issue.comments = load_comments(issue_id, cwd);

            Ok(issue)
        }
        Err(e) => Err(format!("Failed to load issue: {}", e)),
    }
}

/// Execute beads init
pub fn execute_beads_init(cwd: &PathBuf) -> Result<String, String> {
    let output = Command::new("bd")
        .args(["init"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                Ok("Beads initialized successfully".to_string())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Init failed: {}", stderr))
            }
        }
        Err(e) => Err(format!("Failed to init: {}", e)),
    }
}

/// Execute beads human (returns help text)
pub fn execute_beads_human(cwd: &PathBuf) -> Result<Vec<String>, String> {
    let output = Command::new("bd")
        .args(["human"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(stdout.lines().map(|s| s.to_string()).collect())
        }
        Err(e) => Err(format!("Failed to run human: {}", e)),
    }
}

/// Execute beads doctor (returns health check output)
pub fn execute_beads_doctor(cwd: &PathBuf) -> Result<Vec<String>, String> {
    let output = Command::new("bd")
        .args(["doctor"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Doctor outputs to both stdout and stderr
            let mut lines: Vec<String> = stdout.lines().map(|s| s.to_string()).collect();
            lines.extend(stderr.lines().map(|s| s.to_string()));
            Ok(lines)
        }
        Err(e) => Err(format!("Failed to run doctor: {}", e)),
    }
}
