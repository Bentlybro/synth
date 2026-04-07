use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use std::io::Cursor;
use tracing::{info, warn};

use super::{ContentExtractor, ContentMetadata, ContentType, ExtractedContent};

/// PDF document extractor
pub struct PdfExtractor {
    client: Client,
}

impl PdfExtractor {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (compatible; SynthBot/1.0)")
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap(),
        }
    }
    
    /// Extract text from PDF bytes
    async fn extract_text(&self, pdf_bytes: &[u8]) -> Result<String> {
        // Use pdf-extract crate for text extraction
        let text = tokio::task::spawn_blocking({
            let bytes = pdf_bytes.to_vec();
            move || -> Result<String> {
                let cursor = Cursor::new(bytes);
                let extracted = pdf_extract::extract_text_from_mem(&cursor.into_inner())
                    .context("Failed to extract text from PDF")?;
                Ok(extracted)
            }
        })
        .await
        .context("PDF extraction task failed")??;
        
        Ok(text)
    }
    
    /// Extract title from PDF metadata or filename
    fn extract_title(&self, url: &str) -> String {
        // Try to get filename from URL
        if let Some(filename) = url.split('/').last() {
            let title = filename
                .trim_end_matches(".pdf")
                .replace('-', " ")
                .replace('_', " ");
            if !title.is_empty() {
                return title;
            }
        }
        
        url.to_string()
    }
}

impl Default for PdfExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContentExtractor for PdfExtractor {
    fn can_handle(&self, url: &str) -> bool {
        let lower = url.to_lowercase();
        // Direct .pdf extension (with optional query string/fragment)
        let path = lower.split('?').next().unwrap_or(&lower);
        let path = path.split('#').next().unwrap_or(path);
        if path.ends_with(".pdf") {
            return true;
        }
        // Common PDF URL patterns (e.g. arxiv.org/pdf/XXXX)
        if let Ok(parsed) = url::Url::parse(&lower) {
            let segments: Vec<&str> = parsed.path_segments()
                .map(|s| s.collect())
                .unwrap_or_default();
            // URL has a path segment that is exactly "pdf" (e.g. /pdf/2106.09685)
            if segments.iter().any(|s| *s == "pdf") {
                return true;
            }
        }
        false
    }
    
    async fn extract(&self, url: &str) -> Result<ExtractedContent> {
        info!("Extracting PDF: {}", url);
        
        // Download PDF
        let response = self.client
            .get(url)
            .send()
            .await
            .context("Failed to download PDF")?;
        
        let pdf_bytes = response.bytes().await.context("Failed to read PDF bytes")?;
        let file_size = pdf_bytes.len() as u64;
        
        // Extract text
        let content = match self.extract_text(&pdf_bytes).await {
            Ok(text) => text,
            Err(e) => {
                warn!("Failed to extract PDF text: {}", e);
                format!("[PDF extraction failed: {}]", e)
            }
        };
        
        let title = self.extract_title(url);
        
        Ok(ExtractedContent {
            url: url.to_string(),
            title,
            content: content.chars().take(100_000).collect(), // Limit to 100k chars
            content_type: ContentType::PDF,
            metadata: Some(serde_json::json!({
                "file_size_bytes": file_size,
                "format": "PDF"
            })),
        })
    }
    
    fn content_type(&self) -> ContentType {
        ContentType::PDF
    }
}
