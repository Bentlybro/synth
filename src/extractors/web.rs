use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::info;

use super::{ContentExtractor, ContentType, ExtractedContent};

/// Web page HTML extractor
pub struct WebExtractor {
    client: Client,
}

impl WebExtractor {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (compatible; SynthBot/1.0)")
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }
    
    /// Extract main content from HTML
    fn extract_content(&self, html: &str) -> String {
        let document = Html::parse_document(html);
        
        // Try main content selectors first
        let content_selectors = vec![
            "article",
            "main",
            "[role='main']",
            ".content",
            ".post-content",
            "#content",
        ];
        
        for selector_str in content_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Some(element) = document.select(&selector).next() {
                    let text = element.text().collect::<Vec<_>>().join(" ");
                    if text.len() > 200 {
                        return Self::clean_text(&text);
                    }
                }
            }
        }
        
        // Fallback: body text
        if let Ok(selector) = Selector::parse("body") {
            if let Some(body) = document.select(&selector).next() {
                let text = body.text().collect::<Vec<_>>().join(" ");
                return Self::clean_text(&text);
            }
        }
        
        String::new()
    }
    
    /// Extract title from HTML
    fn extract_title(&self, html: &str, url: &str) -> String {
        let document = Html::parse_document(html);
        
        // Try title tag
        if let Ok(selector) = Selector::parse("title") {
            if let Some(title) = document.select(&selector).next() {
                let text = title.text().collect::<String>();
                if !text.is_empty() {
                    return text.trim().to_string();
                }
            }
        }
        
        // Try og:title
        if let Ok(selector) = Selector::parse("meta[property='og:title']") {
            if let Some(meta) = document.select(&selector).next() {
                if let Some(content) = meta.value().attr("content") {
                    return content.to_string();
                }
            }
        }
        
        // Fallback: URL
        url.to_string()
    }
    
    /// Clean and normalize text
    fn clean_text(text: &str) -> String {
        text.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(50_000) // Limit to ~50k chars
            .collect()
    }
}

impl Default for WebExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContentExtractor for WebExtractor {
    fn can_handle(&self, url: &str) -> bool {
        // Web extractor is the fallback - handles http/https if no other extractor matches
        url.starts_with("http://") || url.starts_with("https://")
    }
    
    async fn extract(&self, url: &str) -> Result<ExtractedContent> {
        info!("Extracting web page: {}", url);
        
        let response = self.client
            .get(url)
            .send()
            .await
            .context("Failed to fetch URL")?;
        
        let html = response.text().await.context("Failed to read response")?;
        
        let title = self.extract_title(&html, url);
        let content = self.extract_content(&html);
        
        Ok(ExtractedContent {
            url: url.to_string(),
            title,
            content,
            content_type: ContentType::Web,
            metadata: None,
        })
    }
    
    fn content_type(&self) -> ContentType {
        ContentType::Web
    }
}
