//! AI Assistant plugin operations
//!
//! Functions to read status from AI CLI tool config/data files.

use super::state::{
    AIProvider, ClaudeDailyStats, ClaudeStatus, CodexStatus, GeminiStatus,
};
use std::fs;

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

    status
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

    status
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
            if let Some(theme) = json.get("ui").and_then(|v| v.get("theme")).and_then(|v| v.as_str()) {
                status.theme = Some(theme.to_string());
            }
        }
    }

    status
}

/// Refresh all provider statuses
pub fn refresh_all_status() -> (ClaudeStatus, CodexStatus, GeminiStatus) {
    (read_claude_status(), read_codex_status(), read_gemini_status())
}
