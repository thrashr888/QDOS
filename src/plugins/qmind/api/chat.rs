//! Chat/LLM API adapter for command parsing and file summaries
//!
//! Supports OpenAI and Anthropic Claude for natural language processing.

use super::provider::{AIApiConfig, AIProvider, ApiError};
use serde::{Deserialize, Serialize};

/// A chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }
}

/// Response from chat API
#[derive(Debug, Clone)]
pub struct ChatResponse {
    /// The assistant's response content
    pub content: String,
    /// Tokens used for input
    pub input_tokens: u32,
    /// Tokens used for output
    pub output_tokens: u32,
}

/// Trait for chat/completion providers
pub trait ChatProvider: Send + Sync {
    /// Send messages and get a completion
    fn complete(&self, messages: &[ChatMessage]) -> Result<ChatResponse, ApiError>;

    /// Send a single prompt with optional system message
    fn prompt(&self, system: Option<&str>, user: &str) -> Result<ChatResponse, ApiError> {
        let mut messages = Vec::new();
        if let Some(sys) = system {
            messages.push(ChatMessage::system(sys));
        }
        messages.push(ChatMessage::user(user));
        self.complete(&messages)
    }
}

/// OpenAI chat completions
#[derive(Debug, Deserialize)]
struct OpenAIChatResponse {
    choices: Vec<OpenAIChatChoice>,
    usage: OpenAIChatUsage,
}

#[derive(Debug, Deserialize)]
struct OpenAIChatChoice {
    message: OpenAIChatMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAIChatMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIChatUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

/// OpenAI chat provider
pub struct OpenAIChat {
    config: AIApiConfig,
}

impl OpenAIChat {
    pub fn new(config: AIApiConfig) -> Self {
        Self { config }
    }
}

impl ChatProvider for OpenAIChat {
    fn complete(&self, messages: &[ChatMessage]) -> Result<ChatResponse, ApiError> {
        let api_key = self.config.require_api_key()?;

        let request = serde_json::json!({
            "model": self.config.chat_model,
            "messages": messages,
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
        });

        let response = ureq::post(&format!("{}/chat/completions", self.config.provider.api_base()))
            .header("Authorization", &format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .send_json(&request)
            .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

        let body: OpenAIChatResponse = response
            .into_body()
            .read_json()
            .map_err(|e| ApiError::ParseError(e.to_string()))?;

        let choice = body
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| ApiError::InvalidResponse("No choices in response".to_string()))?;

        Ok(ChatResponse {
            content: choice.message.content,
            input_tokens: body.usage.prompt_tokens,
            output_tokens: body.usage.completion_tokens,
        })
    }
}

/// Anthropic Claude chat provider
pub struct AnthropicChat {
    config: AIApiConfig,
}

impl AnthropicChat {
    pub fn new(config: AIApiConfig) -> Self {
        Self { config }
    }
}

/// Anthropic messages response
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    text: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

impl ChatProvider for AnthropicChat {
    fn complete(&self, messages: &[ChatMessage]) -> Result<ChatResponse, ApiError> {
        let api_key = self.config.require_api_key()?;

        // Anthropic handles system message separately
        let system_content: Option<String> = messages
            .iter()
            .find(|m| m.role == "system")
            .map(|m| m.content.clone());

        let user_messages: Vec<_> = messages
            .iter()
            .filter(|m| m.role != "system")
            .cloned()
            .collect();

        let mut request = serde_json::json!({
            "model": self.config.chat_model,
            "messages": user_messages,
            "max_tokens": self.config.max_tokens,
        });

        if let Some(system) = system_content {
            request["system"] = serde_json::Value::String(system);
        }

        let response = ureq::post(&format!("{}/messages", self.config.provider.api_base()))
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .send_json(&request)
            .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

        let body: AnthropicResponse = response
            .into_body()
            .read_json()
            .map_err(|e| ApiError::ParseError(e.to_string()))?;

        let content = body
            .content
            .into_iter()
            .map(|c| c.text)
            .collect::<Vec<_>>()
            .join("");

        Ok(ChatResponse {
            content,
            input_tokens: body.usage.input_tokens,
            output_tokens: body.usage.output_tokens,
        })
    }
}

/// Create a chat provider from config
pub fn create_chat_provider(config: AIApiConfig) -> Result<Box<dyn ChatProvider>, ApiError> {
    if !config.is_configured() {
        return Err(ApiError::NoApiKey("No provider configured".to_string()));
    }

    match config.provider {
        AIProvider::OpenAI => Ok(Box::new(OpenAIChat::new(config))),
        AIProvider::Anthropic => Ok(Box::new(AnthropicChat::new(config))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_constructors() {
        let sys = ChatMessage::system("You are helpful");
        assert_eq!(sys.role, "system");

        let user = ChatMessage::user("Hello");
        assert_eq!(user.role, "user");

        let assistant = ChatMessage::assistant("Hi there");
        assert_eq!(assistant.role, "assistant");
    }
}
