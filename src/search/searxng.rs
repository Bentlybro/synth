use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use tracing::info;

use crate::models::SearchResult;

#[derive(Debug, Deserialize)]
struct SearXNGResponse {
    results: Vec<SearXNGResult>,
}

#[derive(Debug, Deserialize)]
struct SearXNGResult {
    url: String,
    title: String,
    #[serde(default)]
    content: String,
}

pub struct SearXNGSearch {
    client: Client,
    base_url: String,
}

impl SearXNGSearch {
    pub fn new(base_url: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to build HTTP client");

        let base_url = base_url.unwrap_or_else(|| "http://localhost:8888".to_string());

        Self { client, base_url }
    }

    pub async fn search(&self, query: &str, max_results: usize) -> Result<Vec<SearchResult>> {
        info!("Searching SearXNG for: {}", query);

        let url = format!("{}/search", self.base_url);

        let response = self.client
            .get(&url)
            .query(&[
                ("q", query),
                ("format", "json"),
                ("pageno", "1"),
            ])
            .send()
            .await
            .context("Failed to fetch SearXNG results")?;

        if !response.status().is_success() {
            anyhow::bail!("SearXNG returned error: {}", response.status());
        }

        // Get response text for debugging
        let response_text = response.text().await?;
        
        // Try to parse JSON
        let searxng_response: SearXNGResponse = serde_json::from_str(&response_text)
            .context(format!("Failed to parse SearXNG response. First 500 chars: {}", 
                &response_text.chars().take(500).collect::<String>()))?;

        let results: Vec<SearchResult> = searxng_response.results
            .into_iter()
            .take(max_results)
            .map(|r| SearchResult {
                url: r.url,
                title: r.title,
                snippet: r.content,
            })
            .collect();

        info!("Found {} results from SearXNG", results.len());
        Ok(results)
    }
}
