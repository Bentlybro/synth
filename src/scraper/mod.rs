use anyhow::{Context, Result};
use futures::stream::{self, StreamExt};
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use tracing::info;
use crate::cache::CacheManager;
use crate::models::{ScrapedPage, SearchResult};
use crate::shared::cache_key;

#[derive(Serialize, Deserialize)]
struct CachedPageData {
    title: String,
    content: String,
}

pub struct Scraper {
    client: Client,
    max_concurrent: usize,
}

impl Scraper {
    pub fn new(max_concurrent: usize) -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (compatible; OSIT/0.1; +https://github.com/bentlybro)")
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            max_concurrent,
        }
    }

    /// Scrape multiple pages in parallel with caching
    pub async fn scrape_parallel(&self, results: Vec<SearchResult>, cache: &CacheManager) -> Vec<ScrapedPage> {
        stream::iter(results)
            .map(|result| {
                let cache_ref = cache;
                async move {
                    self.scrape_page_cached(&result.url, cache_ref).await.ok()
                }
            })
            .buffer_unordered(self.max_concurrent)
            .filter_map(|page| async { page })
            .collect()
            .await
    }

    /// Scrape with cache check
    async fn scrape_page_cached(&self, url: &str, cache: &CacheManager) -> Result<ScrapedPage> {
        let key = cache_key(&url);
        
        // Check cache first
        if let Some(cached) = cache.get::<CachedPageData>("pages", &key, 24).await {
            info!("Page cache HIT: {}", url);
            return Ok(ScrapedPage {
                url: url.to_string(),
                title: cached.title,
                content: cached.content.clone(),
                word_count: cached.content.split_whitespace().count(),
            });
        }

        info!("Page cache MISS, scraping: {}", url);
        
        // Scrape page
        let page = self.scrape_page(url).await?;
        
        // Store in cache
        let cached_data = CachedPageData {
            title: page.title.clone(),
            content: page.content.clone(),
        };
        cache.put("pages", &key, cached_data).await.ok();
        
        Ok(page)
    }

    /// Scrape a single page
    async fn scrape_page(&self, url: &str) -> Result<ScrapedPage> {
        let response = self.client
            .get(url)
            .send()
            .await
            .context("Failed to fetch page")?;

        if !response.status().is_success() {
            anyhow::bail!("HTTP {}", response.status());
        }

        let html = response.text().await.context("Failed to read response body")?;
        
        let document = Html::parse_document(&html);
        
        // Extract title
        let title = document
            .select(&Selector::parse("title").unwrap())
            .next()
            .map(|el| el.text().collect::<String>())
            .unwrap_or_else(|| "Untitled".to_string());

        // Extract main content (prioritize article, main, body)
        let content = self.extract_content(&document);
        let word_count = content.split_whitespace().count();

        Ok(ScrapedPage {
            url: url.to_string(),
            title,
            content,
            word_count,
        })
    }

    fn extract_content(&self, document: &Html) -> String {
        // Try to find main content areas first
        let selectors = vec![
            "article",
            "main",
            "[role='main']",
            ".content",
            "#content",
            "body",
        ];

        for selector_str in selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Some(element) = document.select(&selector).next() {
                    let text = element.text().collect::<Vec<_>>().join(" ");
                    if !text.trim().is_empty() {
                        return self.clean_text(&text);
                    }
                }
            }
        }

        // Fallback to full body
        let body_text = document.root_element().text().collect::<Vec<_>>().join(" ");
        self.clean_text(&body_text)
    }

    fn clean_text(&self, text: &str) -> String {
        text.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(50000) // Limit to ~50k chars to avoid huge pages
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scraper_basic() {
        let scraper = Scraper::new(5);
        // Mock test - would need test server or wiremock
    }
}
