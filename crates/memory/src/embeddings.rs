//! Real embedding model support for semantic search.
//!
//! This module provides embedding generation using actual ML models:
//! - Ollama: Local embedding models (nomic-embed-text, mxbai-embed-large, etc.)
//! - OpenAI: Cloud embedding models (text-embedding-3-small, text-embedding-3-large, etc.)
//! - Fallback: Simple hash-based embedding when no model is configured

use async_trait::async_trait;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Duration;

pub use super::error::MemoryError as Error;

/// Embedding model provider.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddingProvider {
    /// Ollama local embedding models
    Ollama,
    /// OpenAI embedding API
    OpenAI,
    /// Simple hash-based fallback (no ML)
    Simple,
}

/// Embedding model configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Provider (ollama, openai, simple)
    pub provider: EmbeddingProvider,

    /// Model name
    pub model: String,

    /// API endpoint (for Ollama)
    pub endpoint: Option<String>,

    /// API key (for OpenAI)
    pub api_key: Option<String>,

    /// Request timeout in seconds
    pub timeout_secs: Option<u64>,

    /// Cache size (number of embeddings to cache)
    pub cache_size: Option<usize>,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: EmbeddingProvider::Simple,
            model: "default".to_string(),
            endpoint: None,
            api_key: None,
            timeout_secs: Some(30),
            cache_size: Some(1000),
        }
    }
}

impl EmbeddingConfig {
    /// Create Ollama configuration.
    pub fn ollama(model: impl Into<String>) -> Self {
        Self {
            provider: EmbeddingProvider::Ollama,
            model: model.into(),
            endpoint: Some("http://localhost:11434".to_string()),
            api_key: None,
            timeout_secs: Some(60),
            cache_size: Some(1000),
        }
    }

    /// Create Ollama configuration with custom endpoint.
    pub fn ollama_with_endpoint(model: impl Into<String>, endpoint: impl Into<String>) -> Self {
        Self {
            provider: EmbeddingProvider::Ollama,
            model: model.into(),
            endpoint: Some(endpoint.into()),
            api_key: None,
            timeout_secs: Some(60),
            cache_size: Some(1000),
        }
    }

    /// Create OpenAI configuration.
    pub fn openai(model: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            provider: EmbeddingProvider::OpenAI,
            model: model.into(),
            endpoint: None,
            api_key: Some(api_key.into()),
            timeout_secs: Some(30),
            cache_size: Some(1000),
        }
    }

    /// Create simple (hash-based) configuration.
    pub fn simple() -> Self {
        Self {
            provider: EmbeddingProvider::Simple,
            model: "simple".to_string(),
            endpoint: None,
            api_key: None,
            timeout_secs: None,
            cache_size: None,
        }
    }

    /// Set the endpoint.
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// Set the API key.
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set the timeout.
    pub fn with_timeout_secs(mut self, timeout: u64) -> Self {
        self.timeout_secs = Some(timeout);
        self
    }

    /// Set the cache size.
    pub fn with_cache_size(mut self, size: usize) -> Self {
        self.cache_size = Some(size);
        self
    }
}

/// Trait for embedding models.
#[async_trait]
pub trait EmbeddingModel: Send + Sync {
    /// Generate embedding for a single text.
    async fn embed(&self, text: &str) -> Result<Vec<f32>, Error>;

    /// Generate embeddings for multiple texts (batch).
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, Error>;

    /// Get the embedding dimension.
    fn dimension(&self) -> usize;

    /// Get the model name.
    fn model_name(&self) -> &str;
}

/// Simple hash-based embedding (fallback).
#[derive(Debug, Clone)]
pub struct SimpleEmbedding {
    dim: usize,
}

impl SimpleEmbedding {
    /// Create a new simple embedding generator.
    pub fn new(dim: usize) -> Self {
        Self { dim }
    }

    /// Generate an embedding from text using hash.
    pub fn embed(&self, text: &str) -> Vec<f32> {
        let mut embedding = vec![0.0_f32; self.dim];

        // Simple hash-based embedding (for demonstration/fallback)
        for (i, byte) in text.bytes().enumerate() {
            let pos = i % self.dim;
            embedding[pos] = embedding[pos] * 31.0 + (byte as f32) * 0.1;
            embedding[pos] = (embedding[pos] % 10.0 - 5.0) / 5.0; // Normalize to [-1, 1]
        }

        // Normalize to unit length
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in embedding.iter_mut() {
                *v /= norm;
            }
        }

        embedding
    }

    /// Get the dimension of this embedding.
    pub fn dimension(&self) -> usize {
        self.dim
    }

    /// Get default dimension.
    pub fn default_dimension() -> usize {
        768 // Common embedding dimension
    }
}

impl Default for SimpleEmbedding {
    fn default() -> Self {
        Self::new(768)
    }
}

/// Ollama embedding model.
pub struct OllamaEmbedding {
    client: reqwest::Client,
    model: String,
    endpoint: String,
    dimension: usize,
}

impl OllamaEmbedding {
    /// Create a new Ollama embedding model.
    pub fn new(model: impl Into<String>, endpoint: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            model: model.into(),
            endpoint: endpoint.into(),
            dimension: 768, // nomic-embed-text default
        }
    }

    /// Set the embedding dimension.
    pub fn with_dimension(mut self, dimension: usize) -> Self {
        self.dimension = dimension;
        self
    }

    /// Get dimension for common models.
    fn model_dimension(model: &str) -> usize {
        let model_lower = model.to_lowercase();
        if model_lower.contains("nomic-embed") {
            768
        } else if model_lower.contains("mxbai-embed-large") {
            1024
        } else if model_lower.contains("llama3") {
            4096
        } else if model_lower.contains("all-minilm") {
            384
        } else {
            768 // Default
        }
    }
}

#[derive(Debug, Serialize)]
struct OllamaEmbedRequest<'a> {
    model: &'a str,
    input: &'a str,
}

#[derive(Debug, Deserialize)]
struct OllamaEmbedResponse {
    embedding: Vec<f32>,
}

#[async_trait]
impl EmbeddingModel for OllamaEmbedding {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, Error> {
        let url = format!("{}/api/embeddings", self.endpoint);
        let req = OllamaEmbedRequest {
            model: &self.model,
            input: text,
        };

        let resp: reqwest::Response = self.client
            .post(&url)
            .json(&req)
            .timeout(Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| Error::Embedding(format!("HTTP error: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text: String = resp.text().await.unwrap_or_default();
            return Err(Error::Embedding(format!("Ollama API error {}: {}", status, text)));
        }

        let data: OllamaEmbedResponse = resp
            .json::<OllamaEmbedResponse>()
            .await
            .map_err(|e| Error::Embedding(format!("JSON decode error: {}", e)))?;

        Ok(data.embedding)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, Error> {
        // Ollama doesn't support native batch processing, so we run sequentially
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}

/// OpenAI embedding model.
pub struct OpenAIEmbedding {
    client: reqwest::Client,
    model: String,
    api_key: String,
}

impl OpenAIEmbedding {
    /// Create a new OpenAI embedding model.
    pub fn new(model: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            model: model.into(),
            api_key: api_key.into(),
        }
    }

    /// Get dimension for OpenAI models.
    fn model_dimension(model: &str) -> usize {
        match model {
            "text-embedding-3-small" => 1536,
            "text-embedding-3-large" => 3072,
            "text-embedding-ada-002" => 1536,
            _ => 1536, // Default
        }
    }
}

#[derive(Debug, Serialize)]
struct OpenAIEmbedRequest<'a> {
    model: &'a str,
    input: Vec<&'a str>,
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbedResponse {
    data: Vec<OpenAIEmbedData>,
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbedData {
    embedding: Vec<f32>,
}

#[async_trait]
impl EmbeddingModel for OpenAIEmbedding {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, Error> {
        let results: Vec<Vec<f32>> = self.embed_batch(&[text.to_string()]).await?;
        Ok(results.into_iter().next().unwrap())
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, Error> {
        let url = "https://api.openai.com/v1/embeddings";
        let inputs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();

        let resp: reqwest::Response = self.client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&OpenAIEmbedRequest {
                model: &self.model,
                input: inputs,
            })
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| Error::Embedding(format!("HTTP error: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text: String = resp.text().await.unwrap_or_default();
            return Err(Error::Embedding(format!("OpenAI API error {}: {}", status, text)));
        }

        let data: OpenAIEmbedResponse = resp
            .json::<OpenAIEmbedResponse>()
            .await
            .map_err(|e| Error::Embedding(format!("JSON decode error: {}", e)))?;

        Ok(data.data.into_iter().map(|d| d.embedding).collect())
    }

    fn dimension(&self) -> usize {
        Self::model_dimension(&self.model)
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}

/// Cached embedding model wrapper.
pub struct CachedEmbeddingModel {
    inner: Box<dyn EmbeddingModel>,
    cache: Arc<tokio::sync::Mutex<LruCache<u64, Vec<f32>>>>,
}

impl CachedEmbeddingModel {
    /// Create a new cached embedding model.
    pub fn new(inner: Box<dyn EmbeddingModel>, cache_size: usize) -> Self {
        let capacity = NonZeroUsize::new(cache_size).unwrap_or(NonZeroUsize::new(1000).unwrap());
        Self {
            inner,
            cache: Arc::new(tokio::sync::Mutex::new(LruCache::new(capacity))),
        }
    }

    /// Calculate hash for text.
    fn hash_text(text: &str) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }

    /// Get cache stats.
    pub async fn cache_len(&self) -> usize {
        let cache = self.cache.lock().await;
        cache.len()
    }
}

#[async_trait]
impl EmbeddingModel for CachedEmbeddingModel {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, Error> {
        // Check cache first
        let key = Self::hash_text(text);
        {
            let mut cache = self.cache.lock().await;
            if let Some(cached) = cache.get(&key) {
                return Ok(cached.clone());
            }
        }

        // Compute embedding
        let embedding: Vec<f32> = self.inner.embed(text).await?;

        // Store in cache
        {
            let mut cache = self.cache.lock().await;
            cache.put(key, embedding.clone());
        }

        Ok(embedding)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, Error> {
        let mut results = Vec::with_capacity(texts.len());
        let mut uncached_indices = Vec::new();
        let mut uncached_texts = Vec::new();

        // Check cache
        for (i, text) in texts.iter().enumerate() {
            let key = Self::hash_text(text);
            let mut cache = self.cache.lock().await;
            if let Some(cached) = cache.get(&key) {
                results.push(Some(cached.clone()));
            } else {
                results.push(None);
                uncached_indices.push(i);
                uncached_texts.push(text.clone());
            }
        }

        // Compute uncached embeddings
        if !uncached_texts.is_empty() {
            let uncached_embeddings: Vec<Vec<f32>> = self.inner.embed_batch(&uncached_texts).await?;

            let mut cache = self.cache.lock().await;
            for (idx, (text, embedding)) in uncached_texts
                .iter()
                .zip(uncached_embeddings.into_iter())
                .enumerate()
            {
                let result_idx = uncached_indices[idx];
                results[result_idx] = Some(embedding.clone());

                let key = Self::hash_text(text);
                cache.put(key, embedding);
            }
        }

        Ok(results.into_iter().map(|r| r.unwrap()).collect())
    }

    fn dimension(&self) -> usize {
        self.inner.dimension()
    }

    fn model_name(&self) -> &str {
        self.inner.model_name()
    }
}

/// Create an embedding model from configuration.
pub fn create_embedding_model(config: EmbeddingConfig) -> Result<Box<dyn EmbeddingModel>, Error> {
    let model: Box<dyn EmbeddingModel> = match config.provider {
        EmbeddingProvider::Ollama => {
            let endpoint = config.endpoint
                .unwrap_or_else(|| "http://localhost:11434".to_string());
            let mut ollama = OllamaEmbedding::new(&config.model, &endpoint);
            ollama.dimension = OllamaEmbedding::model_dimension(&config.model);
            Box::new(ollama)
        }
        EmbeddingProvider::OpenAI => {
            let api_key = config
                .api_key
                .ok_or_else(|| Error::Config("OpenAI API key is required".to_string()))?;
            Box::new(OpenAIEmbedding::new(&config.model, api_key))
        }
        EmbeddingProvider::Simple => {
            Box::new(SimpleEmbeddingWrapper(SimpleEmbedding::default()))
        }
    };

    // Wrap with cache if cache_size is set
    let cache_size = config.cache_size.unwrap_or(1000);
    if cache_size > 0 {
        Ok(Box::new(CachedEmbeddingModel::new(model, cache_size)))
    } else {
        Ok(model)
    }
}

/// Wrapper to make SimpleEmbedding implement EmbeddingModel.
struct SimpleEmbeddingWrapper(SimpleEmbedding);

#[async_trait]
impl EmbeddingModel for SimpleEmbeddingWrapper {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, Error> {
        Ok(self.0.embed(text))
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, Error> {
        Ok(texts.iter().map(|t| self.0.embed(t)).collect())
    }

    fn dimension(&self) -> usize {
        self.0.dim
    }

    fn model_name(&self) -> &str {
        "simple"
    }
}

/// Compute cosine similarity between two embeddings.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}

/// Compute dot product similarity.
pub fn dot_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_config_default() {
        let config = EmbeddingConfig::default();
        assert_eq!(config.provider, EmbeddingProvider::Simple);
    }

    #[test]
    fn test_embedding_config_ollama() {
        let config = EmbeddingConfig::ollama("nomic-embed-text");
        assert_eq!(config.provider, EmbeddingProvider::Ollama);
        assert_eq!(config.model, "nomic-embed-text");
        assert_eq!(config.endpoint, Some("http://localhost:11434".to_string()));
    }

    #[test]
    fn test_embedding_config_openai() {
        let config = EmbeddingConfig::openai("text-embedding-3-small", "sk-test");
        assert_eq!(config.provider, EmbeddingProvider::OpenAI);
        assert_eq!(config.model, "text-embedding-3-small");
        assert_eq!(config.api_key, Some("sk-test".to_string()));
    }

    #[test]
    fn test_simple_embedding() {
        let embed = SimpleEmbedding::new(128);
        let text = "Hello, world!";
        let embedding = embed.embed(text);

        assert_eq!(embedding.len(), 128);
        // Check normalized (approximately)
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let c = vec![0.0, 1.0, 0.0];

        // Identical vectors
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        // Orthogonal vectors
        assert!(cosine_similarity(&a, &c) < 0.001);
    }

    #[test]
    fn test_dot_similarity() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![2.0, 3.0, 4.0];

        let dot = dot_similarity(&a, &b);
        assert_eq!(dot, 20.0);
    }

    #[tokio::test]
    async fn test_create_embedding_model_simple() {
        let config = EmbeddingConfig::simple();
        let model = create_embedding_model(config).unwrap();

        let embedding = model.embed("test").await.unwrap();
        assert!(!embedding.is_empty());
        assert_eq!(model.dimension(), 768);
    }

    #[tokio::test]
    async fn test_embed_batch_simple() {
        let config = EmbeddingConfig::simple();
        let model = create_embedding_model(config).unwrap();

        let texts = vec![
            "Hello world".to_string(),
            "How are you?".to_string(),
            "Goodbye".to_string(),
        ];

        let embeddings = model.embed_batch(&texts).await.unwrap();
        assert_eq!(embeddings.len(), 3);
        for emb in embeddings {
            assert_eq!(emb.len(), 768);
        }
    }

    #[test]
    fn test_model_dimension() {
        assert_eq!(OpenAIEmbedding::model_dimension("text-embedding-3-small"), 1536);
        assert_eq!(OpenAIEmbedding::model_dimension("text-embedding-3-large"), 3072);
        assert_eq!(OpenAIEmbedding::model_dimension("text-embedding-ada-002"), 1536);
    }
}
