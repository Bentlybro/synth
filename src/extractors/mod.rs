use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub mod web;
pub mod pdf;
pub mod video;
pub mod audio;
pub mod image;

/// Unified content that can be analyzed by LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedContent {
    pub url: String,
    pub title: String,
    pub content: String,
    pub content_type: ContentType,
    pub metadata: Option<ContentMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentType {
    Web,
    PDF,
    Video,
    Audio,
    Image,
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
            Box::new(pdf::PdfExtractor::new()),
            Box::new(video::VideoExtractor::new()),
            Box::new(image::ImageExtractor::new()),
            Box::new(audio::AudioExtractor::new()),
            Box::new(web::WebExtractor::new()), // Fallback - must be last
        ];
        
        Self { extractors }
    }
    
    /// Route URL to the appropriate extractor
    pub async fn extract(&self, url: &str) -> Result<ExtractedContent> {
        for extractor in &self.extractors {
            if extractor.can_handle(url) {
                return extractor.extract(url).await;
            }
        }
        
        anyhow::bail!("No extractor found for URL: {}", url)
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
