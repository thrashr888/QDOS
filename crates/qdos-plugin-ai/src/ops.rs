//! AI Coding Agents plugin operations
//!
//! Functions to read status from AI coding agent config/data files.

use super::state::{
    AIProvider, ClaudeDailyStats, ClaudeStatus, ClaudeTokenUsage, CodexStatus, CodexTokenUsage,
    CopilotStatus, CursorStatus, GeminiStatus,
};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Command;

/// Claude API pricing (per million tokens) as of 2026
/// Using Claude Sonnet 4 pricing as default (Claude Code default model)
/// See: https://platform.claude.com/docs/en/about-claude/pricing
const CLAUDE_INPUT_PRICE_PER_M: f64 = 3.0;
const CLAUDE_OUTPUT_PRICE_PER_M: f64 = 15.0;
const CLAUDE_CACHE_READ_PRICE_PER_M: f64 = 0.30; // 0.1× base input price
const CLAUDE_CACHE_WRITE_PRICE_PER_M: f64 = 3.75; // 1.25× base input price (5-min cache)

/// OpenAI API pricing (per million tokens) as of 2026
/// Using gpt-5-codex Standard tier pricing as default
/// See: https://platform.openai.com/docs/pricing
const CODEX_INPUT_PRICE_PER_M: f64 = 1.25;
const CODEX_CACHED_INPUT_PRICE_PER_M: f64 = 0.125;
const CODEX_OUTPUT_PRICE_PER_M: f64 = 10.0;
const CODEX_REASONING_PRICE_PER_M: f64 = 8.0; // o3 output pricing for reasoning

/// Read Claude Code status from ~/.claude/
pub fn read_claude_status() -> ClaudeStatus {
    let Some(config_dir) = AIProvider::Claude.config_dir() else {
        return ClaudeStatus::default();
    };

    if !config_dir.exists() {
        return ClaudeStatus::default();
    }

    let mut status = ClaudeStatus {
        available: true,
        ..Default::default()
    };

    // Read stats-cache.json
    let stats_path = config_dir.join("stats-cache.json");
    if let Ok(content) = fs::read_to_string(&stats_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            // Get last computed date
            if let Some(date) = json.get("lastComputedDate").and_then(|v| v.as_str()) {
                status.last_computed = Some(date.to_string());
            }

            // Get daily activity
            if let Some(daily) = json.get("dailyActivity").and_then(|v| v.as_array()) {
                let today = chrono::Local::now().format("%Y-%m-%d").to_string();

                for entry in daily.iter().rev().take(7) {
                    let stats = ClaudeDailyStats {
                        date: entry
                            .get("date")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        message_count: entry
                            .get("messageCount")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0),
                        session_count: entry
                            .get("sessionCount")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0),
                        tool_call_count: entry
                            .get("toolCallCount")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0),
                    };

                    if stats.date == today {
                        status.today = Some(stats.clone());
                    }
                    status.recent_days.push(stats);
                }
            }
        }
    }

    // Read token usage from session JSONL files in projects/
    let projects_dir = config_dir.join("projects");
    if projects_dir.exists() {
        let (usage, session_count) = read_claude_token_usage(&projects_dir);
        status.token_usage = usage;
        status.session_count = session_count;
    }

    status
}

/// Read token usage from Claude session JSONL files
fn read_claude_token_usage(projects_dir: &Path) -> (ClaudeTokenUsage, usize) {
    let mut usage = ClaudeTokenUsage::default();
    let mut session_count = 0;

    // Find all JSONL files in projects subdirectories
    if let Ok(entries) = fs::read_dir(projects_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Ok(files) = fs::read_dir(&path) {
                    for file in files.flatten() {
                        let file_path = file.path();
                        if file_path.extension().is_some_and(|e| e == "jsonl") {
                            session_count += 1;
                            parse_claude_jsonl(&file_path, &mut usage);
                        }
                    }
                }
            }
        }
    }

    // Calculate cost
    usage.total_cost_usd = (usage.input_tokens as f64 * CLAUDE_INPUT_PRICE_PER_M / 1_000_000.0)
        + (usage.output_tokens as f64 * CLAUDE_OUTPUT_PRICE_PER_M / 1_000_000.0)
        + (usage.cache_read_tokens as f64 * CLAUDE_CACHE_READ_PRICE_PER_M / 1_000_000.0)
        + (usage.cache_creation_tokens as f64 * CLAUDE_CACHE_WRITE_PRICE_PER_M / 1_000_000.0);

    (usage, session_count)
}

/// Parse a single Claude JSONL file for token usage
fn parse_claude_jsonl(path: &Path, usage: &mut ClaudeTokenUsage) {
    let Ok(file) = fs::File::open(path) else {
        return;
    };
    let reader = BufReader::new(file);

    for line in reader.lines().map_while(Result::ok) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
            // Look for message.usage field
            if let Some(msg_usage) = json.get("message").and_then(|m| m.get("usage")) {
                usage.input_tokens += msg_usage
                    .get("input_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                usage.output_tokens += msg_usage
                    .get("output_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                usage.cache_read_tokens += msg_usage
                    .get("cache_read_input_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                usage.cache_creation_tokens += msg_usage
                    .get("cache_creation_input_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
            }
        }
    }
}

/// Read Codex CLI status from ~/.codex/
pub fn read_codex_status() -> CodexStatus {
    let Some(config_dir) = AIProvider::Codex.config_dir() else {
        return CodexStatus::default();
    };

    if !config_dir.exists() {
        return CodexStatus::default();
    }

    let mut status = CodexStatus {
        available: true,
        ..Default::default()
    };

    // Read config.toml
    let config_path = config_dir.join("config.toml");
    if let Ok(content) = fs::read_to_string(&config_path) {
        if let Ok(toml) = content.parse::<toml::Table>() {
            // Get model
            if let Some(model) = toml.get("model").and_then(|v| v.as_str()) {
                status.model = Some(model.to_string());
            }

            // Get reasoning effort
            if let Some(effort) = toml.get("model_reasoning_effort").and_then(|v| v.as_str()) {
                status.reasoning_effort = Some(effort.to_string());
            }

            // Get trusted projects
            if let Some(projects) = toml.get("projects").and_then(|v| v.as_table()) {
                for (path, value) in projects {
                    if let Some(table) = value.as_table() {
                        if let Some(trust) = table.get("trust_level").and_then(|v| v.as_str()) {
                            if trust == "trusted" {
                                // Extract just the project name from path
                                let name = path.rsplit('/').next().unwrap_or(path);
                                status.trusted_projects.push(name.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    // Read version.json
    let version_path = config_dir.join("version.json");
    if let Ok(content) = fs::read_to_string(&version_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(version) = json.get("latest_version").and_then(|v| v.as_str()) {
                status.latest_version = Some(version.to_string());
            }
            if let Some(checked) = json.get("last_checked_at").and_then(|v| v.as_str()) {
                // Parse and format date
                if let Some(date_part) = checked.split('T').next() {
                    status.last_checked = Some(date_part.to_string());
                }
            }
        }
    }

    // Read token usage from session JSONL files in sessions/
    let sessions_dir = config_dir.join("sessions");
    if sessions_dir.exists() {
        let (usage, session_count) = read_codex_token_usage(&sessions_dir);
        status.token_usage = usage;
        status.session_count = session_count;
    }

    status
}

/// Read token usage from Codex session JSONL files
fn read_codex_token_usage(sessions_dir: &Path) -> (CodexTokenUsage, usize) {
    let mut usage = CodexTokenUsage::default();
    let mut session_count = 0;

    // Walk the sessions directory structure: sessions/YYYY/MM/DD/*.jsonl
    if let Ok(years) = fs::read_dir(sessions_dir) {
        for year_entry in years.flatten() {
            let year_path = year_entry.path();
            if !year_path.is_dir() {
                continue;
            }
            if let Ok(months) = fs::read_dir(&year_path) {
                for month_entry in months.flatten() {
                    let month_path = month_entry.path();
                    if !month_path.is_dir() {
                        continue;
                    }
                    if let Ok(days) = fs::read_dir(&month_path) {
                        for day_entry in days.flatten() {
                            let day_path = day_entry.path();
                            if !day_path.is_dir() {
                                continue;
                            }
                            if let Ok(files) = fs::read_dir(&day_path) {
                                for file in files.flatten() {
                                    let file_path = file.path();
                                    if file_path.extension().is_some_and(|e| e == "jsonl") {
                                        session_count += 1;
                                        parse_codex_jsonl(&file_path, &mut usage);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Calculate cost
    // Note: cached tokens are already counted in input_tokens for Codex,
    // so we calculate: (input - cached) * full_price + cached * cached_price
    let non_cached_input = usage.input_tokens.saturating_sub(usage.cached_input_tokens);
    usage.total_cost_usd = (non_cached_input as f64 * CODEX_INPUT_PRICE_PER_M / 1_000_000.0)
        + (usage.cached_input_tokens as f64 * CODEX_CACHED_INPUT_PRICE_PER_M / 1_000_000.0)
        + (usage.output_tokens as f64 * CODEX_OUTPUT_PRICE_PER_M / 1_000_000.0)
        + (usage.reasoning_tokens as f64 * CODEX_REASONING_PRICE_PER_M / 1_000_000.0);

    (usage, session_count)
}

/// Parse a single Codex JSONL file for token usage
fn parse_codex_jsonl(path: &Path, usage: &mut CodexTokenUsage) {
    let Ok(file) = fs::File::open(path) else {
        return;
    };
    let reader = BufReader::new(file);

    // Track last seen token counts per session to avoid double-counting
    // (Codex reports cumulative totals, we want the final values)
    let mut last_input = 0u64;
    let mut last_cached = 0u64;
    let mut last_output = 0u64;
    let mut last_reasoning = 0u64;

    for line in reader.lines().map_while(Result::ok) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
            // Look for token_count event messages
            if json.get("type").and_then(|t| t.as_str()) == Some("event_msg") {
                if let Some(payload) = json.get("payload") {
                    if payload.get("type").and_then(|t| t.as_str()) == Some("token_count") {
                        if let Some(info) = payload.get("info") {
                            if let Some(total) = info.get("total_token_usage") {
                                last_input = total
                                    .get("input_tokens")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);
                                last_cached = total
                                    .get("cached_input_tokens")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);
                                last_output = total
                                    .get("output_tokens")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);
                                last_reasoning = total
                                    .get("reasoning_output_tokens")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);
                            }
                        }
                    }
                }
            }
        }
    }

    // Add the final token counts from this session
    usage.input_tokens += last_input;
    usage.cached_input_tokens += last_cached;
    usage.output_tokens += last_output;
    usage.reasoning_tokens += last_reasoning;
}

/// Read Gemini CLI status from ~/.gemini/
pub fn read_gemini_status() -> GeminiStatus {
    let Some(config_dir) = AIProvider::Gemini.config_dir() else {
        return GeminiStatus::default();
    };

    if !config_dir.exists() {
        return GeminiStatus::default();
    }

    let mut status = GeminiStatus {
        available: true,
        ..Default::default()
    };

    // Read settings.json
    let settings_path = config_dir.join("settings.json");
    if let Ok(content) = fs::read_to_string(&settings_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            // Get auth type
            if let Some(auth_type) = json
                .get("security")
                .and_then(|v| v.get("auth"))
                .and_then(|v| v.get("selectedType"))
                .and_then(|v| v.as_str())
            {
                status.auth_type = Some(auth_type.to_string());
            }

            // Get general settings
            if let Some(general) = json.get("general") {
                if let Some(editor) = general.get("preferredEditor").and_then(|v| v.as_str()) {
                    status.preferred_editor = Some(editor.to_string());
                }
                if let Some(preview) = general.get("previewFeatures").and_then(|v| v.as_bool()) {
                    status.preview_features = preview;
                }
            }

            // Get UI theme
            if let Some(theme) = json
                .get("ui")
                .and_then(|v| v.get("theme"))
                .and_then(|v| v.as_str())
            {
                status.theme = Some(theme.to_string());
            }
        }
    }

    status
}

/// Read Cursor IDE status from ~/.cursor/
pub fn read_cursor_status() -> CursorStatus {
    let Some(config_dir) = AIProvider::Cursor.config_dir() else {
        return CursorStatus::default();
    };

    if !config_dir.exists() {
        return CursorStatus::default();
    }

    let mut status = CursorStatus {
        available: true,
        ..Default::default()
    };

    // Read cli-config.json
    let config_path = config_dir.join("cli-config.json");
    if let Ok(content) = fs::read_to_string(&config_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            // Get model
            if let Some(model) = json
                .get("model")
                .and_then(|m| m.get("displayName"))
                .and_then(|v| v.as_str())
            {
                status.model = Some(model.to_string());
            }

            // Get vim mode
            if let Some(vim) = json
                .get("editor")
                .and_then(|e| e.get("vimMode"))
                .and_then(|v| v.as_bool())
            {
                status.vim_mode = vim;
            }
        }
    }

    // Read AI tracking stats from SQLite database
    let db_path = config_dir.join("ai-tracking/ai-code-tracking.db");
    if db_path.exists() {
        read_cursor_tracking_stats(&db_path, &mut status);
    }

    status
}

/// Read Cursor AI tracking stats from SQLite database using sqlite3 CLI
fn read_cursor_tracking_stats(db_path: &Path, status: &mut CursorStatus) {
    // Get total count
    if let Ok(output) = Command::new("sqlite3")
        .arg(db_path)
        .arg("SELECT COUNT(*) FROM ai_code_hashes")
        .output()
    {
        if output.status.success() {
            if let Ok(count_str) = String::from_utf8(output.stdout) {
                status.code_generations = count_str.trim().parse().unwrap_or(0);
            }
        }
    }

    // Get breakdown by source
    if let Ok(output) = Command::new("sqlite3")
        .arg(db_path)
        .arg("SELECT source, COUNT(*) FROM ai_code_hashes GROUP BY source ORDER BY COUNT(*) DESC")
        .output()
    {
        if output.status.success() {
            if let Ok(result) = String::from_utf8(output.stdout) {
                for line in result.lines() {
                    let parts: Vec<&str> = line.split('|').collect();
                    if parts.len() == 2 {
                        let source = if parts[0].is_empty() {
                            "unknown".to_string()
                        } else {
                            parts[0].to_string()
                        };
                        let count: u64 = parts[1].parse().unwrap_or(0);
                        status.generations_by_source.push((source, count));
                    }
                }
            }
        }
    }
}

/// Read GitHub Copilot status via gh CLI
pub fn read_copilot_status() -> CopilotStatus {
    let mut status = CopilotStatus::default();

    // Check if gh CLI is authenticated
    if let Ok(output) = Command::new("gh").args(["auth", "status"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{}{}", stdout, stderr);

        // Check if logged in
        if combined.contains("Logged in to") {
            status.available = true;
            status.gh_authenticated = true;

            // Extract username from "account <username>"
            for line in combined.lines() {
                if line.contains("account") {
                    // Format: "  ✓ Logged in to github.com account <username>"
                    if let Some(idx) = line.find("account") {
                        let after = &line[idx + 8..];
                        let username = after.split_whitespace().next().unwrap_or("");
                        if !username.is_empty() {
                            status.github_user = Some(username.to_string());
                        }
                    }
                }
            }
        }
    }

    status
}

/// Refresh all provider statuses
pub fn refresh_all_status() -> (
    ClaudeStatus,
    CodexStatus,
    GeminiStatus,
    CursorStatus,
    CopilotStatus,
) {
    (
        read_claude_status(),
        read_codex_status(),
        read_gemini_status(),
        read_cursor_status(),
        read_copilot_status(),
    )
}
