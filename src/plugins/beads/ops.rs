//! Beads Operations Module
//!
//! Provides helper functions for Beads issue tracker integration in Q-DOS II.
//! These are standalone functions to avoid borrow checker issues in the main app.

use crate::app::{BeadsActivityEntry, BeadsComment, BeadsIssue, BeadsState};
use beads_cli::{Beads, Comment as BeadsCrateComment, Issue as BeadsCrateIssue};
use serde::Deserialize;
use std::path::PathBuf;

/// Convert crate Issue to plugin BeadsIssue
fn convert_issue(issue: BeadsCrateIssue) -> BeadsIssue {
    // Call blocker_ids() before moving fields
    let blocked_by = issue.blocker_ids();
    BeadsIssue {
        id: issue.id,
        title: issue.title,
        description: issue.description,
        status: issue.status,
        priority: issue
            .priority
            .map(|p| p.to_string())
            .unwrap_or_else(|| "2".to_string()),
        issue_type: issue.issue_type,
        blocked_by,
        dependents: Vec::new(), // Would need separate struct in crate for full support
        comments: Vec::new(),   // Comments loaded separately
    }
}

/// Convert multiple crate Issues to plugin BeadsIssues
fn convert_issues(issues: Vec<BeadsCrateIssue>) -> Vec<BeadsIssue> {
    issues.into_iter().map(convert_issue).collect()
}

/// Convert crate Comment to plugin BeadsComment
fn convert_comment(comment: BeadsCrateComment) -> BeadsComment {
    BeadsComment {
        author: comment.author,
        text: comment.content,
        created_at: comment.created_at.unwrap_or_default(),
    }
}

/// Convert multiple crate Comments to plugin BeadsComments
fn convert_comments(comments: Vec<BeadsCrateComment>) -> Vec<BeadsComment> {
    comments.into_iter().map(convert_comment).collect()
}

/// Load comments for an issue using typed API
fn load_comments(issue_id: &str, cwd: &PathBuf) -> Vec<BeadsComment> {
    let bd = Beads::with_workdir(cwd);
    bd.comments(issue_id)
        .map(convert_comments)
        .unwrap_or_default()
}

/// Load beads issues list using typed API
pub fn load_beads_list(state: &mut BeadsState, cwd: &PathBuf, status_filter: Option<&str>) {
    state.issues.clear();
    state.error = None;
    state.selected_issue = 0;

    let status = match status_filter {
        Some("all") => Some("all"),
        Some("in_progress") => Some("in_progress"),
        Some("closed") => Some("closed"),
        _ => Some("open"),
    };

    let bd = Beads::with_workdir(cwd);
    match bd.list(status, None) {
        Ok(issues) => state.issues = convert_issues(issues),
        Err(e) => state.error = Some(format!("Failed to load issues: {}", e)),
    }
}

/// Load ready issues using typed API
pub fn load_beads_ready(state: &mut BeadsState, cwd: &PathBuf) {
    state.issues.clear();
    state.error = None;
    state.selected_issue = 0;

    let bd = Beads::with_workdir(cwd);
    match bd.ready() {
        Ok(issues) => state.issues = convert_issues(issues),
        Err(e) => state.error = Some(format!("Failed to load ready issues: {}", e)),
    }
}

/// Load blocked issues using typed API
pub fn load_beads_blocked(state: &mut BeadsState, cwd: &PathBuf) {
    state.issues.clear();
    state.error = None;
    state.selected_issue = 0;

    let bd = Beads::with_workdir(cwd);
    match bd.blocked() {
        Ok(issues) => state.issues = convert_issues(issues),
        Err(e) => state.error = Some(format!("Failed to load blocked issues: {}", e)),
    }
}

/// Load all epics with their progress info using typed API
pub fn load_beads_epics(state: &mut BeadsState, cwd: &PathBuf) {
    state.issues.clear();
    state.error = None;
    state.selected_issue = 0;

    let bd = Beads::with_workdir(cwd);
    match bd.list_open_epics() {
        Ok(issues) => state.issues = convert_issues(issues),
        Err(e) => state.error = Some(format!("Failed to load epics: {}", e)),
    }
}

/// Load top 5 open epics for main menu display using typed API
pub fn load_top_epics(state: &mut BeadsState, cwd: &PathBuf) {
    state.top_epics.clear();

    let bd = Beads::with_workdir(cwd);
    if let Ok(issues) = bd.list_open_epics() {
        // Take top 5 epics
        state.top_epics = convert_issues(issues).into_iter().take(5).collect();
    }
}

/// Load recent issues (in_progress first, then high priority open) using typed API
pub fn load_recent_issues(state: &mut BeadsState, cwd: &PathBuf) {
    state.recent_issues.clear();

    let bd = Beads::with_workdir(cwd);

    // First get in_progress issues
    if let Ok(issues) = bd.list_in_progress() {
        // Take up to 3 in_progress issues
        let mut converted = convert_issues(issues);
        converted.truncate(3);
        state.recent_issues.extend(converted);
    }

    // If we have room, add ready issues
    if state.recent_issues.len() < 5 {
        let remaining = 5 - state.recent_issues.len();
        if let Ok(issues) = bd.ready() {
            let mut converted = convert_issues(issues);
            // Filter out already-included issues and take remaining
            converted.retain(|i| !state.recent_issues.iter().any(|r| r.id == i.id));
            converted.truncate(remaining);
            state.recent_issues.extend(converted);
        }
    }
}

/// Load beads stats using typed API
pub fn load_beads_stats(state: &mut BeadsState, cwd: &PathBuf) {
    state.error = None;

    let bd = Beads::with_workdir(cwd);
    match bd.stats() {
        Ok(stats) => {
            state.stats.total = stats.total;
            state.stats.open = stats.open;
            state.stats.in_progress = stats.in_progress;
            state.stats.closed = stats.closed;
            state.stats.blocked = stats.blocked;
        }
        Err(e) => state.error = Some(format!("Failed to load stats: {}", e)),
    }
}

/// Execute beads create using typed API
pub fn execute_beads_create(
    title: &str,
    description: &str,
    issue_type_idx: usize,
    priority: usize,
    cwd: &PathBuf,
) -> Result<(), String> {
    let issue_types = ["task", "bug", "feature"];
    let issue_type = issue_types[issue_type_idx];

    let bd = Beads::with_workdir(cwd);
    let desc = if description.is_empty() {
        None
    } else {
        Some(description)
    };

    match bd.create_full(
        title,
        issue_type,
        Some(priority as u8),
        desc,
        None,
        None,
        None,
    ) {
        Ok(output) => {
            if output.success {
                Ok(())
            } else {
                Err(format!("Create failed: {}", output.stderr))
            }
        }
        Err(e) => Err(format!("Failed to create issue: {}", e)),
    }
}

/// Execute beads close using typed API
pub fn execute_beads_close(issue_id: &str, cwd: &PathBuf) -> Result<String, String> {
    let bd = Beads::with_workdir(cwd);
    match bd.close(issue_id) {
        Ok(output) => {
            if output.success {
                Ok(format!("Issue {} closed", issue_id))
            } else {
                Err(format!("Close failed: {}", output.stderr))
            }
        }
        Err(e) => Err(format!("Failed to close issue: {}", e)),
    }
}

/// Execute beads update status using typed API
pub fn execute_beads_update_status(
    issue_id: &str,
    status: &str,
    cwd: &PathBuf,
) -> Result<String, String> {
    let bd = Beads::with_workdir(cwd);
    match bd.update_status(issue_id, status) {
        Ok(output) => {
            if output.success {
                Ok(format!("Issue {} updated to {}", issue_id, status))
            } else {
                Err(format!("Update failed: {}", output.stderr))
            }
        }
        Err(e) => Err(format!("Failed to update issue: {}", e)),
    }
}

/// Execute beads reopen using typed API
pub fn execute_beads_reopen(issue_id: &str, cwd: &PathBuf) -> Result<String, String> {
    let bd = Beads::with_workdir(cwd);
    match bd.reopen(issue_id) {
        Ok(output) => {
            if output.success {
                Ok(format!("Issue {} reopened", issue_id))
            } else {
                Err(format!("Reopen failed: {}", output.stderr))
            }
        }
        Err(e) => Err(format!("Failed to reopen issue: {}", e)),
    }
}

/// Execute beads sync using typed API
pub fn execute_beads_sync(cwd: &PathBuf) -> Result<String, String> {
    let bd = Beads::with_workdir(cwd);
    match bd.sync() {
        Ok(output) => {
            if output.success {
                // Get the last non-empty line for status
                let msg = output
                    .stdout
                    .lines()
                    .rfind(|l| !l.is_empty())
                    .unwrap_or("Sync complete")
                    .to_string();
                Ok(msg)
            } else {
                Err(format!("Sync failed: {}", output.stderr))
            }
        }
        Err(e) => Err(format!("Failed to sync: {}", e)),
    }
}

/// Load detailed issue info (including dependents for epics and comments) using typed API
pub fn load_beads_issue_detail(issue_id: &str, cwd: &PathBuf) -> Result<BeadsIssue, String> {
    let bd = Beads::with_workdir(cwd);
    match bd.show(issue_id) {
        Ok(crate_issue) => {
            let mut issue = convert_issue(crate_issue);
            // Load comments separately
            issue.comments = load_comments(issue_id, cwd);
            Ok(issue)
        }
        Err(e) => Err(format!("Failed to load issue: {}", e)),
    }
}

/// Execute beads init using typed API
pub fn execute_beads_init(cwd: &PathBuf) -> Result<String, String> {
    let bd = Beads::with_workdir(cwd);
    match bd.init() {
        Ok(output) => {
            if output.success {
                Ok("Beads initialized successfully".to_string())
            } else {
                Err(format!("Init failed: {}", output.stderr))
            }
        }
        Err(e) => Err(format!("Failed to init: {}", e)),
    }
}

/// Execute beads human (returns help text) using typed API
pub fn execute_beads_human(cwd: &PathBuf) -> Result<Vec<String>, String> {
    let bd = Beads::with_workdir(cwd);
    match bd.human() {
        Ok(output) => Ok(output.stdout.lines().map(|s| s.to_string()).collect()),
        Err(e) => Err(format!("Failed to run human: {}", e)),
    }
}

/// Execute beads doctor (returns health check output) using typed API
pub fn execute_beads_doctor(cwd: &PathBuf) -> Result<Vec<String>, String> {
    let bd = Beads::with_workdir(cwd);
    match bd.doctor() {
        Ok(output) => {
            // Doctor outputs to both stdout and stderr
            let mut lines: Vec<String> = output.stdout.lines().map(|s| s.to_string()).collect();
            lines.extend(output.stderr.lines().map(|s| s.to_string()));
            Ok(lines)
        }
        Err(e) => Err(format!("Failed to run doctor: {}", e)),
    }
}

/// Execute beads add comment using typed API
pub fn execute_beads_add_comment(
    issue_id: &str,
    comment: &str,
    cwd: &PathBuf,
) -> Result<String, String> {
    let bd = Beads::with_workdir(cwd);
    match bd.comment_add(issue_id, comment) {
        Ok(output) => {
            if output.success {
                Ok(format!("Comment added to {}", issue_id))
            } else {
                Err(format!("Add comment failed: {}", output.stderr))
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

/// Quick status for status bar display using typed API
/// Returns BeadsStatusInfo or None if beads not available
pub fn get_beads_status_info(cwd: &PathBuf) -> Option<BeadsStatusInfo> {
    let bd = Beads::with_workdir(cwd);

    // Use the combined status_info method from the crate
    let status = bd.status_info().ok()?;

    Some(BeadsStatusInfo {
        open: status.open,
        in_progress: status.in_progress,
        ready: status.ready,
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

    let bd = Beads::with_workdir(cwd);
    let output = bd.run(&["activity", "--mol", issue_id, "--limit", "50", "--json"]);

    match output {
        Ok(output) => {
            if output.success {
                if let Ok(activities) =
                    serde_json::from_str::<Vec<BeadsJsonActivity>>(&output.stdout)
                {
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

    let bd = Beads::with_workdir(cwd);
    let output = bd.run(&["activity", "--limit", "100", "--json"]);

    match output {
        Ok(output) => {
            if output.success {
                if let Ok(activities) =
                    serde_json::from_str::<Vec<BeadsJsonActivity>>(&output.stdout)
                {
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

/// Search for issues that mention a specific file path using typed API
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
    let bd = Beads::with_workdir(cwd);
    match bd.search(filename) {
        Ok(crate_issues) => {
            let issues = convert_issues(crate_issues);
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
                .collect();
        }
        Err(_) => {
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

/// Update an existing issue using typed API
pub fn execute_beads_update(
    issue_id: &str,
    title: Option<&str>,
    status: Option<usize>,
    priority: Option<usize>,
    cwd: &PathBuf,
) -> Result<(), String> {
    let statuses = ["open", "in_progress", "closed"];
    let status_str = status.and_then(|s| statuses.get(s).copied());
    let priority_u8 = priority.map(|p| p as u8);

    let bd = Beads::with_workdir(cwd);
    match bd.update(issue_id, status_str, priority_u8, None, title) {
        Ok(output) => {
            if output.success {
                Ok(())
            } else {
                Err(format!("Update failed: {}", output.stderr))
            }
        }
        Err(e) => Err(format!("Failed to run bd: {}", e)),
    }
}

/// Create a subtask under a parent issue (epic) using typed API
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

    let bd = Beads::with_workdir(cwd);
    let desc = if description.is_empty() {
        None
    } else {
        Some(description)
    };

    // Create the issue with full options
    match bd.create_full(
        title,
        issue_type,
        Some(priority as u8),
        desc,
        None,
        Some(parent_id),
        None,
    ) {
        Ok(output) => {
            if output.success {
                // Parse the created issue ID from output
                // Output format: "✓ Created issue: QDOS-xxx"
                if let Some(id) = output.stdout.split("Created issue: ").nth(1) {
                    let new_id = id.split_whitespace().next().unwrap_or("");
                    if !new_id.is_empty() {
                        // Add dependency: new issue depends on parent (parent blocks new issue)
                        let _ = bd.dep_add(new_id, parent_id);
                        return Ok(new_id.to_string());
                    }
                }
                Ok(String::new())
            } else {
                Err(format!("Create failed: {}", output.stderr))
            }
        }
        Err(e) => Err(format!("Failed to run bd: {}", e)),
    }
}
