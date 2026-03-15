use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// OpenAI embeddings API response
#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

/// Embedding with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEmbedding {
    pub text: String,
    pub embedding: Vec<f32>,
    pub url: String,
    pub content_type: String,
    pub cached_at: u64,
}

/// Semantic search result
#[derive(Debug, Clone)]
pub struct SemanticMatch {
    pub url: String,
    pub similarity: f32,
    pub text: String,
    pub content_type: String,
}

#[derive(Clone)]
pub struct EmbeddingStore {
    openai_api_key: Option<String>,
    embeddings_path: PathBuf,
    embeddings: Arc<RwLock<HashMap<String, StoredEmbedding>>>, // key -> embedding
}

impl EmbeddingStore {
    pub fn new(cache_root: PathBuf, openai_api_key: Option<String>) -> Self {
        let embeddings_path = cache_root.join("embeddings.json");
        
        // Load existing embeddings
        let embeddings_map: HashMap<String, StoredEmbedding> = if embeddings_path.exists() {
            std::fs::read_to_string(&embeddings_path)
                .ok()
                .and_then(|json| serde_json::from_str(&json).ok())
                .unwrap_or_default()
        } else {
            HashMap::new()
        };
        
        info!("Loaded {} embeddings from cache", embeddings_map.len());
        
        Self {
            openai_api_key,
            embeddings_path,
            embeddings: Arc::new(RwLock::new(embeddings_map)),
        }
    }
    
    /// Generate embedding for text using OpenAI API
    pub async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let api_key = self.openai_api_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No OpenAI API key configured"))?;
        
        let client = reqwest::Client::new();
        
        // Truncate text to ~8000 tokens (rough estimate: 4 chars per token)
        let truncated_text: String = text.chars().take(32000).collect();
        
        let request_body = serde_json::json!({
            "model": "text-embedding-3-small",
            "input": truncated_text,
        });
        
        let response = client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to call OpenAI embeddings API")?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("OpenAI API error: {}", error_text);
        }
        
        let embedding_response: EmbeddingResponse = response
            .json()
            .await
            .context("Failed to parse embedding response")?;
        
        embedding_response.data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| anyhow::anyhow!("No embedding in response"))
    }
    
    /// Store an embedding
    pub async fn store_embedding(
        &self,
        key: &str,
        text: &str,
        url: &str,
        content_type: &str,
    ) -> Result<()> {
        // Don't generate embeddings if no API key
        if self.openai_api_key.is_none() {
            return Ok(());
        }
        
        // Generate embedding
        let embedding = match self.generate_embedding(text).await {
            Ok(emb) => emb,
            Err(e) => {
                debug!("Failed to generate embedding: {}", e);
                return Ok(()); // Don't fail the whole operation
            }
        };
        
        let stored = StoredEmbedding {
            text: text.chars().take(1000).collect(), // Store snippet for reference
            embedding,
            url: url.to_string(),
            content_type: content_type.to_string(),
            cached_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        {
            let mut embeddings = self.embeddings.write().await;
            embeddings.insert(key.to_string(), stored);
        }
        
        // Persist to disk
        self.save().await?;
        
        Ok(())
    }
    
    /// Find semantically similar content
    pub async fn find_similar(
        &self,
        query: &str,
        threshold: f32,
        limit: usize,
    ) -> Result<Vec<SemanticMatch>> {
        let embeddings = self.embeddings.read().await;
        
        if embeddings.is_empty() {
            return Ok(vec![]);
        }
        
        // Generate query embedding
        let query_embedding = self.generate_embedding(query).await?;
        
        // Calculate similarities
        let mut matches: Vec<(String, f32, StoredEmbedding)> = embeddings
            .iter()
            .map(|(key, stored)| {
                let similarity = cosine_similarity(&query_embedding, &stored.embedding);
                (key.clone(), similarity, stored.clone())
            })
            .filter(|(_, sim, _)| *sim >= threshold)
            .collect();
        
        // Sort by similarity (descending)
        matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        // Take top N
        let results: Vec<SemanticMatch> = matches
            .into_iter()
            .take(limit)
            .map(|(_, similarity, stored)| SemanticMatch {
                url: stored.url.clone(),
                similarity,
                text: stored.text.clone(),
                content_type: stored.content_type.clone(),
            })
            .collect();
        
        info!("Found {} semantically similar matches (threshold: {})", results.len(), threshold);
        
        Ok(results)
    }
    
    /// Save embeddings to disk
    async fn save(&self) -> Result<()> {
        let embeddings = self.embeddings.read().await;
        let json = serde_json::to_string_pretty(&*embeddings)?;
        tokio::fs::write(&self.embeddings_path, json).await?;
        Ok(())
    }
    
    /// Cleanup old embeddings (older than TTL)
    pub async fn cleanup(&self, ttl_hours: u64) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let mut embeddings = self.embeddings.write().await;
        let before_count = embeddings.len();
        
        embeddings.retain(|_, stored| {
            let age_hours = (now - stored.cached_at) / 3600;
            age_hours <= ttl_hours
        });
        
        let removed = before_count - embeddings.len();
        drop(embeddings); // Release write lock before save
        
        if removed > 0 {
            info!("Cleaned up {} old embeddings", removed);
            self.save().await.ok();
        }
    }
}

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }
    
    dot_product / (magnitude_a * magnitude_b)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cosine_similarity() {
        // Identical vectors
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);
        
        // Orthogonal vectors
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!(cosine_similarity(&a, &b).abs() < 0.001);
        
        // Similar vectors
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.1, 2.1, 2.9];
        assert!(cosine_similarity(&a, &b) > 0.99);
    }
}
