use anyhow::{Context, Result};
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::info;

use crate::models::SearchResult;

pub struct DuckDuckGoSearch {
    client: Client,
}

impl DuckDuckGoSearch {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to build HTTP client");

        Self { client }
    }

    pub async fn search(&self, query: &str, max_results: usize) -> Result<Vec<SearchResult>> {
        info!("Searching DuckDuckGo for: {}", query);

        let url = format!("https://html.duckduckgo.com/html/?q={}", 
            urlencoding::encode(query));

        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch DuckDuckGo results")?;

        if !response.status().is_success() {
            anyhow::bail!("DuckDuckGo returned error: {}", response.status());
        }

        let html = response.text().await.context("Failed to read response")?;
        let document = Html::parse_document(&html);

        // DuckDuckGo HTML result selectors
        let result_selector = Selector::parse(".result").unwrap();
        let title_selector = Selector::parse(".result__a").unwrap();
        let snippet_selector = Selector::parse(".result__snippet").unwrap();
        let url_selector = Selector::parse(".result__url").unwrap();

        let mut results = Vec::new();

        for result in document.select(&result_selector).take(max_results) {
            let title = result
                .select(&title_selector)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();

            let snippet = result
                .select(&snippet_selector)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();

            let url_text = result
                .select(&url_selector)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();

            // Clean up the URL (DuckDuckGo shows it without protocol)
            let url = if !url_text.is_empty() {
                if url_text.starts_with("http") {
                    url_text
                } else {
                    format!("https://{}", url_text.trim())
                }
            } else {
                continue;
            };

            if !title.is_empty() && !url.is_empty() {
                results.push(SearchResult {
                    url,
                    title: title.trim().to_string(),
                    snippet: snippet.trim().to_string(),
                });
            }
        }

        info!("Found {} results from DuckDuckGo", results.len());
        Ok(results)
    }
}
