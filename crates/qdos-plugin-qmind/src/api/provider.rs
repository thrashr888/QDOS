//! AI Provider configuration and error types

use std::env;
use std::fmt;

/// Error type for API operations
#[derive(Debug)]
pub enum ApiError {
    /// No API key configured
    NoApiKey(String),
    /// HTTP request failed
    RequestFailed(String),
    /// Failed to parse response
    ParseError(String),
    /// Rate limited
    RateLimited,
    /// Invalid response from API
    InvalidResponse(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::NoApiKey(provider) => write!(f, "No API key for {}", provider),
            ApiError::RequestFailed(msg) => write!(f, "Request failed: {}", msg),
            ApiError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ApiError::RateLimited => write!(f, "Rate limited"),
            ApiError::InvalidResponse(msg) => write!(f, "Invalid response: {}", msg),
        }
    }
}

impl std::error::Error for ApiError {}

/// Supported AI providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AIProvider {
    #[default]
    OpenAI,
    Anthropic,
}

impl AIProvider {
    pub fn name(&self) -> &'static str {
        match self {
            AIProvider::OpenAI => "OpenAI",
            AIProvider::Anthropic => "Anthropic",
        }
    }

    pub fn env_key_name(&self) -> &'static str {
        match self {
            AIProvider::OpenAI => "OPENAI_API_KEY",
            AIProvider::Anthropic => "ANTHROPIC_API_KEY",
        }
    }

    pub fn api_base(&self) -> &'static str {
        match self {
            AIProvider::OpenAI => "https://api.openai.com/v1",
            AIProvider::Anthropic => "https://api.anthropic.com/v1",
        }
    }
}

/// Configuration for AI API access
#[derive(Debug, Clone)]
pub struct AIApiConfig {
    /// Which provider to use
    pub provider: AIProvider,
    /// API key (from config or environment)
    pub api_key: Option<String>,
    /// Model for embeddings
    pub embedding_model: String,
    /// Model for chat/completions
    pub chat_model: String,
    /// Max tokens for chat responses
    pub max_tokens: u32,
    /// Temperature for chat responses
    pub temperature: f32,
}

impl Default for AIApiConfig {
    fn default() -> Self {
        Self {
            provider: AIProvider::OpenAI,
            api_key: None,
            embedding_model: "text-embedding-3-small".to_string(),
            chat_model: "gpt-4o-mini".to_string(),
            max_tokens: 1024,
            temperature: 0.0, // Deterministic for command parsing
        }
    }
}

impl AIApiConfig {
    /// Create config with API key from environment
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // Try OpenAI first
        if let Ok(key) = env::var("OPENAI_API_KEY") {
            config.provider = AIProvider::OpenAI;
            config.api_key = Some(key);
            return config;
        }

        // Fall back to Anthropic
        if let Ok(key) = env::var("ANTHROPIC_API_KEY") {
            config.provider = AIProvider::Anthropic;
            config.api_key = Some(key);
            // Adjust models for Anthropic
            config.chat_model = "claude-sonnet-4-20250514".to_string();
            config.embedding_model = "claude-sonnet-4-20250514".to_string(); // Anthropic uses chat for embeddings
        }

        config
    }

    /// Check if API is configured
    pub fn is_configured(&self) -> bool {
        self.api_key.is_some()
    }

    /// Get API key, returning error if not configured
    pub fn require_api_key(&self) -> Result<&str, ApiError> {
        self.api_key
            .as_deref()
            .ok_or_else(|| ApiError::NoApiKey(self.provider.name().to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AIApiConfig::default();
        assert_eq!(config.provider, AIProvider::OpenAI);
        assert!(config.api_key.is_none());
        assert!(!config.is_configured());
    }

    #[test]
    fn test_api_error_display() {
        let err = ApiError::NoApiKey("OpenAI".to_string());
        assert_eq!(err.to_string(), "No API key for OpenAI");
    }
}
