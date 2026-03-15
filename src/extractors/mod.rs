use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::info;
use crate::cache::CacheManager;
use crate::shared::cache_key;

pub mod web;
pub mod pdf;
pub mod video;
pub mod audio;
pub mod image;
pub mod code_repo;

/// Unified content that can be analyzed by LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedContent {
    pub url: String,
    pub title: String,
    pub content: String,
    pub content_type: ContentType,
    pub metadata: Option<serde_json::Value>, // Flexible metadata (was ContentMetadata)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentType {
    Web,
    PDF,
    Video,
    Audio,
    Image,
    CodeRepository,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentMetadata {
    pub duration_seconds: Option<f64>,
    pub file_size_bytes: Option<u64>,
    pub format: Option<String>,
    pub dimensions: Option<(u32, u32)>, // width, height for images/videos
}

/// Common trait for all extractors
#[async_trait]
pub trait ContentExtractor: Send + Sync {
    /// Check if this extractor can handle the given URL
    fn can_handle(&self, url: &str) -> bool;
    
    /// Extract content from the URL
    async fn extract(&self, url: &str) -> Result<ExtractedContent>;
    
    /// Get the content type this extractor handles
    fn content_type(&self) -> ContentType;
}

/// Router that dispatches URLs to appropriate extractors
pub struct ExtractorRouter {
    extractors: Vec<Box<dyn ContentExtractor>>,
}

impl ExtractorRouter {
    pub fn new() -> Self {
        let extractors: Vec<Box<dyn ContentExtractor>> = vec![
            Box::new(code_repo::CodeRepoExtractor::new()),
            Box::new(pdf::PdfExtractor::new()),
            Box::new(video::VideoExtractor::new()),
            Box::new(image::ImageExtractor::new()),
            Box::new(audio::AudioExtractor::new()),
            Box::new(web::WebExtractor::new()), // Fallback - must be last
        ];
        
        Self { extractors }
    }
    
    /// Route URL to the appropriate extractor (no caching)
    pub async fn extract(&self, url: &str) -> Result<ExtractedContent> {
        for extractor in &self.extractors {
            if extractor.can_handle(url) {
                return extractor.extract(url).await;
            }
        }
        
        anyhow::bail!("No extractor found for URL: {}", url)
    }
    
    /// Route URL to the appropriate extractor with caching
    pub async fn extract_cached(&self, url: &str, cache: &CacheManager) -> Result<ExtractedContent> {
        let key = cache_key(url);
        
        // Determine cache category and TTL based on content type
        let (category, ttl_hours) = match self.detect_type(url) {
            Some(ContentType::CodeRepository) => ("extractors_code", 168), // 7 days (commit-aware)
            Some(ContentType::PDF) => ("extractors_pdf", 24),
            Some(ContentType::Video) => ("extractors_video", 168), // 7 days
            Some(ContentType::Audio) => ("extractors_audio", 168), // 7 days
            Some(ContentType::Image) => ("extractors_image", 24),
            Some(ContentType::Web) | None => ("extractors_web", 24),
        };
        
        // Check cache first
        if let Some(cached) = cache.get::<ExtractedContent>(category, &key, ttl_hours).await {
            info!("Extractor cache HIT ({}): {}", category, url);
            return Ok(cached);
        }
        
        info!("Extractor cache MISS ({}): {}", category, url);
        
        // Extract content
        let content = self.extract(url).await?;
        
        // Store in cache
        cache.put(category, &key, &content).await.ok();
        
        Ok(content)
    }
    
    /// Detect content type without extracting
    pub fn detect_type(&self, url: &str) -> Option<ContentType> {
        for extractor in &self.extractors {
            if extractor.can_handle(url) {
                return Some(extractor.content_type());
            }
        }
        None
    }
}

impl Default for ExtractorRouter {
    fn default() -> Self {
        Self::new()
    }
}
