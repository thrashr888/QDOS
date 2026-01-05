//! AI Assistant plugin state types
//!
//! State structures for monitoring AI coding assistant CLI tools.

use std::path::PathBuf;

/// Supported AI CLI providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AIProvider {
    #[default]
    Claude,
    Codex,
    Gemini,
}

impl AIProvider {
    pub const ALL: [AIProvider; 3] = [AIProvider::Claude, AIProvider::Codex, AIProvider::Gemini];

    pub fn as_str(&self) -> &'static str {
        match self {
            AIProvider::Claude => "Claude Code",
            AIProvider::Codex => "OpenAI Codex",
            AIProvider::Gemini => "Gemini CLI",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            AIProvider::Claude => "Claude",
            AIProvider::Codex => "Codex",
            AIProvider::Gemini => "Gemini",
        }
    }

    pub fn config_dir(&self) -> Option<PathBuf> {
        let home = dirs::home_dir()?;
        Some(match self {
            AIProvider::Claude => home.join(".claude"),
            AIProvider::Codex => home.join(".codex"),
            AIProvider::Gemini => home.join(".gemini"),
        })
    }
}

/// Modal view states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AIView {
    #[default]
    Overview,
    Claude,
    Codex,
    Gemini,
}

impl AIView {
    pub fn title(&self) -> &'static str {
        match self {
            AIView::Overview => "AI Assistants Overview",
            AIView::Claude => "Claude Code Status",
            AIView::Codex => "OpenAI Codex Status",
            AIView::Gemini => "Gemini CLI Status",
        }
    }
}

/// Menu items for the AI modal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AIMenuItem {
    Overview,
    Claude,
    Codex,
    Gemini,
}

impl AIMenuItem {
    pub const ALL: [AIMenuItem; 4] = [
        AIMenuItem::Overview,
        AIMenuItem::Claude,
        AIMenuItem::Codex,
        AIMenuItem::Gemini,
    ];

    pub fn key(&self) -> char {
        match self {
            AIMenuItem::Overview => 'O',
            AIMenuItem::Claude => 'C',
            AIMenuItem::Codex => 'X',
            AIMenuItem::Gemini => 'G',
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            AIMenuItem::Overview => "Overview",
            AIMenuItem::Claude => "Claude Code",
            AIMenuItem::Codex => "Codex",
            AIMenuItem::Gemini => "Gemini",
        }
    }
}

/// Claude Code daily activity stats
#[derive(Debug, Clone, Default)]
pub struct ClaudeDailyStats {
    pub date: String,
    pub message_count: u64,
    pub session_count: u64,
    pub tool_call_count: u64,
}

/// Claude Code status
#[derive(Debug, Clone, Default)]
pub struct ClaudeStatus {
    pub available: bool,
    pub today: Option<ClaudeDailyStats>,
    pub recent_days: Vec<ClaudeDailyStats>,
    pub last_computed: Option<String>,
}

/// Codex CLI status
#[derive(Debug, Clone, Default)]
pub struct CodexStatus {
    pub available: bool,
    pub model: Option<String>,
    pub reasoning_effort: Option<String>,
    pub trusted_projects: Vec<String>,
    pub latest_version: Option<String>,
    pub last_checked: Option<String>,
}

/// Gemini CLI status
#[derive(Debug, Clone, Default)]
pub struct GeminiStatus {
    pub available: bool,
    pub auth_type: Option<String>,
    pub preferred_editor: Option<String>,
    pub theme: Option<String>,
    pub preview_features: bool,
}

/// Overall AI plugin state
#[derive(Debug, Clone, Default)]
pub struct AIState {
    pub view: AIView,
    pub menu_index: usize,
    pub selected_provider: AIProvider,
    pub claude: ClaudeStatus,
    pub codex: CodexStatus,
    pub gemini: GeminiStatus,
    pub scroll_offset: usize,
}

impl AIState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Count how many providers are available
    pub fn available_count(&self) -> usize {
        let mut count = 0;
        if self.claude.available {
            count += 1;
        }
        if self.codex.available {
            count += 1;
        }
        if self.gemini.available {
            count += 1;
        }
        count
    }
}
