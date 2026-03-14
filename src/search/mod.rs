use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::models::SearchResult;

pub struct SearchEngine {
    client: Client,
    api_key: String,
}

#[derive(Debug, Deserialize)]
struct BraveSearchResponse {
    web: Option<BraveWebResults>,
}

#[derive(Debug, Deserialize)]
struct BraveWebResults {
    results: Vec<BraveResult>,
}

#[derive(Debug, Deserialize)]
struct BraveResult {
    url: String,
    title: String,
    description: String,
}

impl SearchEngine {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    pub async fn search(&self, query: &str, count: usize) -> Result<Vec<SearchResult>> {
        let url = "https://api.search.brave.com/res/v1/web/search";
        
        let response = self.client
            .get(url)
            .header("X-Subscription-Token", &self.api_key)
            .query(&[
                ("q", query),
                ("count", &count.to_string()),
            ])
            .send()
            .await
            .context("Failed to send search request")?;

        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("Search API returned error: {}", status);
        }

        let brave_response: BraveSearchResponse = response
            .json()
            .await
            .context("Failed to parse search response")?;

        let results = brave_response
            .web
            .map(|web| {
                web.results
                    .into_iter()
                    .map(|r| SearchResult {
                        url: r.url,
                        title: r.title,
                        snippet: r.description,
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_basic() {
        // Test with mock API key (requires actual key for real test)
        let engine = SearchEngine::new("test_key".to_string());
        // Mock test - actual implementation would use wiremock or similar
    }
}
