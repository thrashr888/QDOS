//! Beads Operations Module
//!
//! Provides helper functions for Beads issue tracker integration in Q-DOS II.
//! These are standalone functions to avoid borrow checker issues in the main app.

use crate::app::{BeadsActivityEntry, BeadsComment, BeadsIssue, BeadsState, BeadsSubIssue};
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
    description: &str,
    issue_type_idx: usize,
    priority: usize,
    cwd: &PathBuf,
) -> Result<(), String> {
    let issue_types = ["task", "bug", "feature"];
    let issue_type = issue_types[issue_type_idx];

    let mut args = vec![
        "create".to_string(),
        "--title".to_string(),
        title.to_string(),
        "--type".to_string(),
        issue_type.to_string(),
        "--priority".to_string(),
        priority.to_string(),
    ];

    if !description.is_empty() {
        args.push("--description".to_string());
        args.push(description.to_string());
    }

    let output = Command::new("bd")
        .args(&args)
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
    let output = Command::new("bd").args(["init"]).current_dir(cwd).output();

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
    let output = Command::new("bd").args(["human"]).current_dir(cwd).output();

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

/// Execute beads add comment
pub fn execute_beads_add_comment(
    issue_id: &str,
    comment: &str,
    cwd: &PathBuf,
) -> Result<String, String> {
    let output = Command::new("bd")
        .args(["comments", "add", issue_id, comment])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                Ok(format!("Comment added to {}", issue_id))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Add comment failed: {}", stderr))
            }
        }
        Err(e) => Err(format!("Failed to add comment: {}", e)),
    }
}

/// Beads status info for status bar display
#[derive(Debug, Clone)]
pub struct BeadsStatusInfo {
    pub open: usize,
    pub in_progress: usize,
    pub ready: usize,
}

/// Quick status for status bar display
/// Returns BeadsStatusInfo or None if beads not available
pub fn get_beads_status_info(cwd: &PathBuf) -> Option<BeadsStatusInfo> {
    // Get stats
    let stats_output = Command::new("bd")
        .args(["stats", "--json"])
        .current_dir(cwd)
        .output()
        .ok()?;

    if !stats_output.status.success() {
        return None;
    }

    let stats_stdout = String::from_utf8_lossy(&stats_output.stdout);
    let stats: BeadsStatsJson = serde_json::from_str(&stats_stdout).ok()?;

    // Get ready count
    let ready_output = Command::new("bd")
        .args(["ready", "--json"])
        .current_dir(cwd)
        .output()
        .ok()?;

    let ready_count = if ready_output.status.success() {
        let ready_stdout = String::from_utf8_lossy(&ready_output.stdout);
        let ready_issues: Vec<BeadsJsonIssue> =
            serde_json::from_str(&ready_stdout).unwrap_or_default();
        ready_issues.len()
    } else {
        0
    };

    Some(BeadsStatusInfo {
        open: stats.summary.open_issues,
        in_progress: stats.summary.in_progress_issues,
        ready: ready_count,
    })
}

/// JSON structure for beads activity from bd activity --json
#[derive(Debug, Deserialize)]
struct BeadsJsonActivity {
    timestamp: String,
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    symbol: String,
    #[serde(default)]
    message: String,
    #[serde(default)]
    old_status: Option<String>,
    #[serde(default)]
    new_status: Option<String>,
    #[serde(default)]
    actor: Option<String>,
}

/// Load activity/history for an issue
pub fn load_issue_activity(state: &mut BeadsState, issue_id: &str, cwd: &PathBuf) {
    state.activity_entries.clear();
    state.selected_activity = 0;
    state.error = None;

    let output = Command::new("bd")
        .args(["activity", "--mol", issue_id, "--limit", "50", "--json"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Ok(activities) = serde_json::from_str::<Vec<BeadsJsonActivity>>(&stdout) {
                    state.activity_entries = activities
                        .into_iter()
                        .map(|a| BeadsActivityEntry {
                            timestamp: a.timestamp,
                            event_type: a.event_type,
                            symbol: a.symbol,
                            message: a.message,
                            old_status: a.old_status,
                            new_status: a.new_status,
                            actor: a.actor,
                        })
                        .collect();
                }
            } else {
                state.error = Some("Failed to load activity".to_string());
            }
        }
        Err(e) => {
            state.error = Some(format!("Failed to load activity: {}", e));
        }
    }
}

/// Load all recent activity
#[allow(dead_code)]
pub fn load_recent_activity(state: &mut BeadsState, cwd: &PathBuf) {
    state.activity_entries.clear();
    state.selected_activity = 0;
    state.error = None;

    let output = Command::new("bd")
        .args(["activity", "--limit", "100", "--json"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Ok(activities) = serde_json::from_str::<Vec<BeadsJsonActivity>>(&stdout) {
                    state.activity_entries = activities
                        .into_iter()
                        .map(|a| BeadsActivityEntry {
                            timestamp: a.timestamp,
                            event_type: a.event_type,
                            symbol: a.symbol,
                            message: a.message,
                            old_status: a.old_status,
                            new_status: a.new_status,
                            actor: a.actor,
                        })
                        .collect();
                }
            } else {
                state.error = Some("Failed to load activity".to_string());
            }
        }
        Err(e) => {
            state.error = Some(format!("Failed to load activity: {}", e));
        }
    }
}

/// Search for issues that mention a specific file path
pub fn find_issues_for_file(state: &mut BeadsState, file_path: &str, cwd: &PathBuf) {
    state.file_related_issues.clear();
    state.file_issue_selected = 0;
    state.file_query_path = file_path.to_string();
    state.error = None;

    // Extract filename from path for broader matching
    let filename = std::path::Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(file_path);

    // Use bd search to find issues mentioning the file
    let output = Command::new("bd")
        .args(["search", filename, "--json"])
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Parse JSON array of issues
                if let Ok(issues) = serde_json::from_str::<Vec<BeadsJsonIssue>>(&stdout) {
                    state.file_related_issues = issues
                        .into_iter()
                        .filter(|issue| {
                            // Check if issue title or description contains the file path or filename
                            let title_match =
                                issue.title.contains(file_path) || issue.title.contains(filename);
                            let desc_match = issue
                                .description
                                .as_ref()
                                .map(|d| d.contains(file_path) || d.contains(filename))
                                .unwrap_or(false);
                            title_match || desc_match
                        })
                        .map(|i| BeadsIssue {
                            id: i.id,
                            title: i.title,
                            status: i.status,
                            priority: i.priority.to_string(),
                            issue_type: i.issue_type,
                            description: i.description,
                            blocked_by: i.blocked_by.unwrap_or_default(),
                            dependents: i
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
                            comments: Vec::new(),
                        })
                        .collect();
                }
            } else {
                // If search fails, try listing all and filtering manually
                load_beads_list(state, cwd, None);
                let all_issues = state.issues.clone();
                state.file_related_issues = all_issues
                    .into_iter()
                    .filter(|issue| {
                        let title_match =
                            issue.title.contains(file_path) || issue.title.contains(filename);
                        let desc_match = issue
                            .description
                            .as_ref()
                            .map(|d| d.contains(file_path) || d.contains(filename))
                            .unwrap_or(false);
                        title_match || desc_match
                    })
                    .collect();
            }
        }
        Err(e) => {
            state.error = Some(format!("Failed to search issues: {}", e));
        }
    }
}

/// Check if any issues mention a specific file (for highlighting)
#[allow(dead_code)]
pub fn file_has_issues(file_path: &str, issues: &[BeadsIssue]) -> bool {
    let filename = std::path::Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(file_path);

    issues.iter().any(|issue| {
        issue.title.contains(file_path)
            || issue.title.contains(filename)
            || issue
                .description
                .as_ref()
                .map(|d| d.contains(file_path) || d.contains(filename))
                .unwrap_or(false)
    })
}

/// Update an existing issue
pub fn execute_beads_update(
    issue_id: &str,
    title: Option<&str>,
    status: Option<usize>,
    priority: Option<usize>,
    cwd: &PathBuf,
) -> Result<(), String> {
    let mut args = vec!["update".to_string(), issue_id.to_string()];

    if let Some(t) = title {
        args.push("--title".to_string());
        args.push(t.to_string());
    }

    if let Some(s) = status {
        let statuses = ["open", "in_progress", "closed"];
        if s < statuses.len() {
            args.push("--status".to_string());
            args.push(statuses[s].to_string());
        }
    }

    if let Some(p) = priority {
        args.push("--priority".to_string());
        args.push(p.to_string());
    }

    let output = Command::new("bd").args(&args).current_dir(cwd).output();

    match output {
        Ok(output) => {
            if output.status.success() {
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Update failed: {}", stderr))
            }
        }
        Err(e) => Err(format!("Failed to run bd: {}", e)),
    }
}

/// Create a subtask under a parent issue (epic)
pub fn execute_beads_create_subtask(
    parent_id: &str,
    title: &str,
    description: &str,
    issue_type_idx: usize,
    priority: usize,
    cwd: &PathBuf,
) -> Result<String, String> {
    let issue_types = ["task", "bug", "feature"];
    let issue_type = issue_types[issue_type_idx];

    let mut args = vec![
        "create".to_string(),
        "--title".to_string(),
        title.to_string(),
        "--type".to_string(),
        issue_type.to_string(),
        "--priority".to_string(),
        priority.to_string(),
    ];

    if !description.is_empty() {
        args.push("--description".to_string());
        args.push(description.to_string());
    }

    // First create the issue
    let output = Command::new("bd")
        .args(&args)
        .current_dir(cwd)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                // Parse the created issue ID from output
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Output format: "✓ Created issue: QDOS-xxx"
                if let Some(id) = stdout.split("Created issue: ").nth(1) {
                    let new_id = id.split_whitespace().next().unwrap_or("");
                    if !new_id.is_empty() {
                        // Add dependency: new issue depends on parent (parent blocks new issue)
                        let dep_output = Command::new("bd")
                            .args(["dep", "add", new_id, parent_id])
                            .current_dir(cwd)
                            .output();

                        if let Ok(dep_out) = dep_output {
                            if dep_out.status.success() {
                                return Ok(new_id.to_string());
                            }
                        }
                        return Ok(new_id.to_string()); // Created but dep might have failed
                    }
                }
                Ok(String::new())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Create failed: {}", stderr))
            }
        }
        Err(e) => Err(format!("Failed to run bd: {}", e)),
    }
}
