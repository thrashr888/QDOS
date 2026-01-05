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
    Cursor,
    Copilot,
}

impl AIProvider {
    pub const ALL: [AIProvider; 5] = [
        AIProvider::Claude,
        AIProvider::Codex,
        AIProvider::Gemini,
        AIProvider::Cursor,
        AIProvider::Copilot,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            AIProvider::Claude => "Claude Code",
            AIProvider::Codex => "OpenAI Codex",
            AIProvider::Gemini => "Gemini CLI",
            AIProvider::Cursor => "Cursor",
            AIProvider::Copilot => "GitHub Copilot",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            AIProvider::Claude => "Claude",
            AIProvider::Codex => "Codex",
            AIProvider::Gemini => "Gemini",
            AIProvider::Cursor => "Cursor",
            AIProvider::Copilot => "Copilot",
        }
    }

    pub fn config_dir(&self) -> Option<PathBuf> {
        let home = dirs::home_dir()?;
        Some(match self {
            AIProvider::Claude => home.join(".claude"),
            AIProvider::Codex => home.join(".codex"),
            AIProvider::Gemini => home.join(".gemini"),
            AIProvider::Cursor => home.join(".cursor"),
            AIProvider::Copilot => home, // Copilot uses gh CLI auth
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
    Cursor,
    Copilot,
}

impl AIView {
    pub fn title(&self) -> &'static str {
        match self {
            AIView::Overview => "AI Assistants Overview",
            AIView::Claude => "Claude Code Status",
            AIView::Codex => "OpenAI Codex Status",
            AIView::Gemini => "Gemini CLI Status",
            AIView::Cursor => "Cursor Status",
            AIView::Copilot => "GitHub Copilot Status",
        }
    }
}

/// Menu items for the AI modal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AIMenuItem {
    Claude,
    Codex,
    Gemini,
    Cursor,
    Copilot,
}

impl AIMenuItem {
    pub const ALL: [AIMenuItem; 5] = [
        AIMenuItem::Claude,
        AIMenuItem::Codex,
        AIMenuItem::Gemini,
        AIMenuItem::Cursor,
        AIMenuItem::Copilot,
    ];

    pub fn key(&self) -> char {
        match self {
            AIMenuItem::Claude => 'C',
            AIMenuItem::Codex => 'X',
            AIMenuItem::Gemini => 'G',
            AIMenuItem::Cursor => 'U',
            AIMenuItem::Copilot => 'P',
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            AIMenuItem::Claude => "Claude Code",
            AIMenuItem::Codex => "Codex",
            AIMenuItem::Gemini => "Gemini",
            AIMenuItem::Cursor => "Cursor",
            AIMenuItem::Copilot => "Copilot",
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

/// Claude token usage from session JSONL files
#[derive(Debug, Clone, Default)]
pub struct ClaudeTokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub total_cost_usd: f64,
}

/// Claude Code status
#[derive(Debug, Clone, Default)]
pub struct ClaudeStatus {
    pub available: bool,
    pub today: Option<ClaudeDailyStats>,
    pub recent_days: Vec<ClaudeDailyStats>,
    pub last_computed: Option<String>,
    /// Token usage from session logs (last 30 days)
    pub token_usage: ClaudeTokenUsage,
    /// Number of session files found
    pub session_count: usize,
}

/// Codex token usage from session JSONL files
#[derive(Debug, Clone, Default)]
pub struct CodexTokenUsage {
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
    pub total_cost_usd: f64,
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
    /// Token usage from session logs
    pub token_usage: CodexTokenUsage,
    /// Number of session files found
    pub session_count: usize,
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

/// Cursor IDE status
#[derive(Debug, Clone, Default)]
pub struct CursorStatus {
    pub available: bool,
    pub model: Option<String>,
    pub vim_mode: bool,
    /// Total AI code generations tracked
    pub code_generations: u64,
    /// Generations by source (composer, tab, etc.)
    pub generations_by_source: Vec<(String, u64)>,
}

/// GitHub Copilot status
#[derive(Debug, Clone, Default)]
pub struct CopilotStatus {
    pub available: bool,
    /// GitHub username if authenticated
    pub github_user: Option<String>,
    /// Whether gh CLI is authenticated
    pub gh_authenticated: bool,
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
    pub cursor: CursorStatus,
    pub copilot: CopilotStatus,
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
        if self.cursor.available {
            count += 1;
        }
        if self.copilot.available {
            count += 1;
        }
        count
    }
}
