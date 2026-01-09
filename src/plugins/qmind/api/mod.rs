//! AI API adapters for Q-MIND
//!
//! Provides unified interface for embeddings and chat completions
//! across multiple providers (OpenAI, Anthropic).

pub mod chat;
pub mod embeddings;
mod provider;

// Re-export for use by other modules
#[allow(unused_imports)]
pub use chat::{ChatMessage, ChatProvider, ChatResponse};
#[allow(unused_imports)]
pub use embeddings::{EmbeddingsProvider, EmbeddingsResponse};
pub use provider::{AIApiConfig, AIProvider, ApiError};
