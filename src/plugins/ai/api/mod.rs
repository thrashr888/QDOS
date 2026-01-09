//! AI API adapters for Q-MIND
//!
//! Provides unified interface for embeddings and chat completions
//! across multiple providers (OpenAI, Anthropic).

pub mod chat;
pub mod embeddings;
mod provider;

pub use chat::{ChatMessage, ChatProvider, ChatResponse};
pub use embeddings::{EmbeddingsProvider, EmbeddingsResponse};
pub use provider::{AIApiConfig, AIProvider, ApiError};
