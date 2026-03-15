use anyhow::{Context, Result};
use async_trait::async_trait;
use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::info;

use super::{ContentExtractor, ContentMetadata, ContentType, ExtractedContent};

/// Image analyzer using Claude vision API
pub struct ImageExtractor {
    client: Client,
    api_key: Option<String>,
}

#[derive(Serialize)]
struct ClaudeVisionRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ClaudeMessage>,
}

#[derive(Serialize)]
struct ClaudeMessage {
    role: String,
    content: Vec<ContentPart>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum ContentPart {
    #[serde(rename = "image")]
    Image {
        source: ImageSource,
    },
    #[serde(rename = "text")]
    Text {
        text: String,
    },
}

#[derive(Serialize)]
struct ImageSource {
    #[serde(rename = "type")]
    source_type: String,
    media_type: String,
    data: String,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
}

#[derive(Deserialize)]
struct ClaudeContent {
    text: String,
}

impl ImageExtractor {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (compatible; SynthBot/1.0)")
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap(),
            api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
        }
    }
    
    /// Download image
    async fn download_image(&self, url: &str) -> Result<(Vec<u8>, String, u64)> {
        info!("Downloading image: {}", url);
        
        let response = self.client
            .get(url)
            .send()
            .await
            .context("Failed to download image")?;
        
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/jpeg")
            .to_string();
        
        let image_bytes = response.bytes().await.context("Failed to read image bytes")?;
        let file_size = image_bytes.len() as u64;
        
        Ok((image_bytes.to_vec(), content_type, file_size))
    }
    
    /// Detect media type from content-type header or URL
    fn detect_media_type(&self, content_type: &str, url: &str) -> String {
        if content_type.contains("png") || url.to_lowercase().ends_with(".png") {
            "image/png".to_string()
        } else if content_type.contains("jpeg") || content_type.contains("jpg") || url.to_lowercase().ends_with(".jpg") || url.to_lowercase().ends_with(".jpeg") {
            "image/jpeg".to_string()
        } else if content_type.contains("gif") || url.to_lowercase().ends_with(".gif") {
            "image/gif".to_string()
        } else if content_type.contains("webp") || url.to_lowercase().ends_with(".webp") {
            "image/webp".to_string()
        } else {
            "image/jpeg".to_string() // Default fallback
        }
    }
    
    /// Analyze image using Claude vision
    async fn analyze_image(&self, image_bytes: &[u8], media_type: &str, api_key: &str) -> Result<String> {
        info!("Analyzing image with Claude vision...");
        
        let base64_image = general_purpose::STANDARD.encode(image_bytes);
        
        let request = ClaudeVisionRequest {
            model: "claude-sonnet-4-20250514".to_string(),
            max_tokens: 2048,
            messages: vec![ClaudeMessage {
                role: "user".to_string(),
                content: vec![
                    ContentPart::Image {
                        source: ImageSource {
                            source_type: "base64".to_string(),
                            media_type: media_type.to_string(),
                            data: base64_image,
                        },
                    },
                    ContentPart::Text {
                        text: "Describe this image in detail. Include: what you see, any text present, key objects, colors, composition, and any notable features. If it's a chart/graph/diagram, explain what it shows.".to_string(),
                    },
                ],
            }],
        };
        
        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request)
            .send()
            .await
            .context("Claude API request failed")?;
        
        let claude_response: ClaudeResponse = response
            .json()
            .await
            .context("Failed to parse Claude response")?;
        
        let description = claude_response
            .content
            .first()
            .map(|c| c.text.clone())
            .unwrap_or_else(|| "[No description available]".to_string());
        
        Ok(description)
    }
    
    /// Extract title from URL
    fn extract_title(&self, url: &str) -> String {
        if let Some(filename) = url.split('/').last() {
            let title = filename
                .split('.')
                .next()
                .unwrap_or("Image")
                .replace('-', " ")
                .replace('_', " ");
            return title;
        }
        url.to_string()
    }
}

impl Default for ImageExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContentExtractor for ImageExtractor {
    fn can_handle(&self, url: &str) -> bool {
        let image_extensions = [".jpg", ".jpeg", ".png", ".gif", ".webp", ".bmp"];
        let url_lower = url.to_lowercase();
        image_extensions.iter().any(|ext| url_lower.ends_with(ext))
    }
    
    async fn extract(&self, url: &str) -> Result<ExtractedContent> {
        let (image_bytes, content_type, file_size) = self.download_image(url).await?;
        let media_type = self.detect_media_type(&content_type, url);
        
        let description = if let Some(api_key) = &self.api_key {
            match self.analyze_image(&image_bytes, &media_type, api_key).await {
                Ok(desc) => desc,
                Err(e) => {
                    info!("Image analysis failed: {}", e);
                    format!("[Image analysis unavailable: {}]", e)
                }
            }
        } else {
            "[Image analysis unavailable - no ANTHROPIC_API_KEY]".to_string()
        };
        
        let title = self.extract_title(url);
        
        // Try to get dimensions (basic check)
        let dimensions = image::load_from_memory(&image_bytes)
            .ok()
            .map(|img| (img.width(), img.height()));
        
        Ok(ExtractedContent {
            url: url.to_string(),
            title: format!("[IMAGE] {}", title),
            content: description,
            content_type: ContentType::Image,
            metadata: Some(ContentMetadata {
                duration_seconds: None,
                file_size_bytes: Some(file_size),
                format: Some(media_type),
                dimensions,
            }),
        })
    }
    
    fn content_type(&self) -> ContentType {
        ContentType::Image
    }
}
