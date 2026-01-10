//! Embeddings API adapter for semantic search
//!
//! Supports OpenAI text-embedding-3-small and Anthropic Claude embeddings.

use super::provider::{AIApiConfig, AIProvider, ApiError};
use serde::{Deserialize, Serialize};

/// Response from embeddings API
#[derive(Debug, Clone)]
pub struct EmbeddingsResponse {
    /// The embedding vector
    pub embedding: Vec<f32>,
    /// Number of tokens used
    pub tokens_used: u32,
}

/// Trait for embedding providers
pub trait EmbeddingsProvider: Send + Sync {
    /// Generate embedding for a single text
    fn embed(&self, text: &str) -> Result<EmbeddingsResponse, ApiError>;

    /// Generate embeddings for multiple texts (batch)
    fn embed_batch(&self, texts: &[String]) -> Result<Vec<EmbeddingsResponse>, ApiError>;

    /// Get the dimension of embeddings from this provider
    fn dimension(&self) -> usize;
}

/// OpenAI embeddings request
#[derive(Debug, Serialize)]
struct OpenAIEmbeddingsRequest {
    model: String,
    input: Vec<String>,
}

/// OpenAI embeddings response
#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingsResponse {
    data: Vec<OpenAIEmbeddingData>,
    usage: OpenAIUsage,
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingData {
    embedding: Vec<f32>,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    total_tokens: u32,
}

/// OpenAI embeddings provider
pub struct OpenAIEmbeddings {
    config: AIApiConfig,
}

impl OpenAIEmbeddings {
    pub fn new(config: AIApiConfig) -> Self {
        Self { config }
    }
}

impl EmbeddingsProvider for OpenAIEmbeddings {
    fn embed(&self, text: &str) -> Result<EmbeddingsResponse, ApiError> {
        let results = self.embed_batch(&[text.to_string()])?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| ApiError::InvalidResponse("No embedding returned".to_string()))
    }

    fn embed_batch(&self, texts: &[String]) -> Result<Vec<EmbeddingsResponse>, ApiError> {
        use std::time::Duration;

        let api_key = self.config.require_api_key()?;

        let request = OpenAIEmbeddingsRequest {
            model: self.config.embedding_model.clone(),
            input: texts.to_vec(),
        };

        // Use a 30 second timeout for embedding API calls
        let config = ureq::config::Config::builder()
            .timeout_global(Some(Duration::from_secs(30)))
            .build();
        let agent = config.new_agent();

        let response = agent
            .post(&format!("{}/embeddings", self.config.provider.api_base()))
            .header("Authorization", &format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .send_json(&request)
            .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

        let body: OpenAIEmbeddingsResponse = response
            .into_body()
            .read_json()
            .map_err(|e| ApiError::ParseError(e.to_string()))?;

        let tokens_per_item = body.usage.total_tokens / texts.len().max(1) as u32;

        Ok(body
            .data
            .into_iter()
            .map(|d| EmbeddingsResponse {
                embedding: d.embedding,
                tokens_used: tokens_per_item,
            })
            .collect())
    }

    fn dimension(&self) -> usize {
        // text-embedding-3-small = 1536 dimensions
        // text-embedding-3-large = 3072 dimensions
        match self.config.embedding_model.as_str() {
            "text-embedding-3-large" => 3072,
            _ => 1536,
        }
    }
}

/// Anthropic embeddings using Claude (via message API with embedding prompt)
pub struct AnthropicEmbeddings {
    config: AIApiConfig,
}

impl AnthropicEmbeddings {
    pub fn new(config: AIApiConfig) -> Self {
        Self { config }
    }

    /// Generate a pseudo-embedding using Claude's understanding
    /// This is a fallback for when native embeddings aren't available
    fn generate_embedding_via_chat(&self, text: &str) -> Result<Vec<f32>, ApiError> {
        use std::time::Duration;

        // Use Claude to generate a semantic fingerprint
        // This is less efficient than native embeddings but works as fallback
        let api_key = self.config.require_api_key()?;

        let request = serde_json::json!({
            "model": self.config.chat_model,
            "max_tokens": 256,
            "messages": [{
                "role": "user",
                "content": format!(
                    "Generate a 64-dimensional semantic embedding vector for the following text. \
                    Output ONLY a JSON array of 64 floating point numbers between -1 and 1, \
                    representing semantic features. No explanation.\n\nText: {}",
                    text
                )
            }]
        });

        // Use a 60 second timeout for Claude API calls (they can be slow)
        let config = ureq::config::Config::builder()
            .timeout_global(Some(Duration::from_secs(60)))
            .build();
        let agent = config.new_agent();

        let response = agent
            .post(&format!("{}/messages", self.config.provider.api_base()))
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .send_json(&request)
            .map_err(|e| ApiError::RequestFailed(e.to_string()))?;

        let body: serde_json::Value = response
            .into_body()
            .read_json()
            .map_err(|e| ApiError::ParseError(e.to_string()))?;

        // Extract the text content
        let content = body["content"][0]["text"]
            .as_str()
            .ok_or_else(|| ApiError::InvalidResponse("No content in response".to_string()))?;

        // Parse the JSON array
        let embedding: Vec<f32> = serde_json::from_str(content)
            .map_err(|e| ApiError::ParseError(format!("Failed to parse embedding: {}", e)))?;

        Ok(embedding)
    }
}

impl EmbeddingsProvider for AnthropicEmbeddings {
    fn embed(&self, text: &str) -> Result<EmbeddingsResponse, ApiError> {
        let embedding = self.generate_embedding_via_chat(text)?;
        Ok(EmbeddingsResponse {
            embedding,
            tokens_used: 100, // Estimate
        })
    }

    fn embed_batch(&self, texts: &[String]) -> Result<Vec<EmbeddingsResponse>, ApiError> {
        // Anthropic doesn't have batch embeddings, process sequentially
        texts.iter().map(|t| self.embed(t)).collect()
    }

    fn dimension(&self) -> usize {
        64 // Our custom pseudo-embedding dimension
    }
}

/// Create an embeddings provider from config
pub fn create_embeddings_provider(
    config: AIApiConfig,
) -> Result<Box<dyn EmbeddingsProvider>, ApiError> {
    if !config.is_configured() {
        return Err(ApiError::NoApiKey("No provider configured".to_string()));
    }

    match config.provider {
        AIProvider::OpenAI => Ok(Box::new(OpenAIEmbeddings::new(config))),
        AIProvider::Anthropic => Ok(Box::new(AnthropicEmbeddings::new(config))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_dimension() {
        let config = AIApiConfig {
            embedding_model: "text-embedding-3-small".to_string(),
            ..Default::default()
        };
        let provider = OpenAIEmbeddings::new(config);
        assert_eq!(provider.dimension(), 1536);
    }

    #[test]
    fn test_create_provider_no_key() {
        let config = AIApiConfig::default();
        let result = create_embeddings_provider(config);
        assert!(result.is_err());
    }
}
