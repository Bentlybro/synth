use anyhow::{Context, Result};
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::info;
use std::time::Duration;

use crate::models::SearchResult;

pub struct DuckDuckGoSearch {
    client: Client,
    user_agents: Vec<&'static str>,
}

impl DuckDuckGoSearch {
    pub fn new() -> Self {
        // Multiple realistic user agents to rotate through
        let user_agents = vec![
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:122.0) Gecko/20100101 Firefox/122.0",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15",
        ];

        let client = Client::builder()
            .cookie_store(true)
            .timeout(Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .expect("Failed to build HTTP client");

        Self { client, user_agents }
    }

    pub async fn search(&self, query: &str, max_results: usize) -> Result<Vec<SearchResult>> {
        info!("Searching DuckDuckGo for: {}", query);

        // Use DuckDuckGo Lite (simpler HTML, less bot detection)
        let url = format!(
            "https://lite.duckduckgo.com/lite/?q={}",
            urlencoding::encode(query)
        );

        // Random user agent
        let ua = self.user_agents[fastrand::usize(..self.user_agents.len())];

        let response = self.client
            .get(&url)
            .header("User-Agent", ua)
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header("Accept-Language", "en-US,en;q=0.5")
            .header("Accept-Encoding", "gzip, deflate, br")
            .header("DNT", "1")
            .header("Connection", "keep-alive")
            .header("Upgrade-Insecure-Requests", "1")
            .send()
            .await
            .context("Failed to fetch DuckDuckGo Lite results")?;

        if !response.status().is_success() {
            anyhow::bail!("DuckDuckGo returned error: {}", response.status());
        }

        let html = response.text().await.context("Failed to read response")?;
        
        // Check for bot detection
        if html.contains("anomaly") || html.contains("challenge") || html.contains("captcha") {
            anyhow::bail!("DuckDuckGo bot detection triggered - try again later or use Brave API");
        }

        let document = Html::parse_document(&html);

        // DuckDuckGo Lite uses simpler HTML structure
        let results = self.parse_lite_results(&document, max_results)?;

        info!("Found {} results from DuckDuckGo", results.len());
        Ok(results)
    }

    fn parse_lite_results(&self, document: &Html, max_results: usize) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();

        // Lite version uses tables for results
        let row_selector = Selector::parse("tr").unwrap();
        let link_selector = Selector::parse("a").unwrap();
        
        for row in document.select(&row_selector).skip(4).take(max_results * 3) {
            // Each result has a link in a table row
            if let Some(link) = row.select(&link_selector).next() {
                if let Some(href) = link.value().attr("href") {
                    // Skip navigation links
                    if href.starts_with("http") && !href.contains("duckduckgo.com") {
                        let title = link.text().collect::<String>().trim().to_string();
                        
                        // Get snippet from next rows if available
                        let snippet = row.text()
                            .collect::<Vec<_>>()
                            .join(" ")
                            .trim()
                            .to_string();

                        if !title.is_empty() && title.len() > 3 {
                            results.push(SearchResult {
                                url: href.to_string(),
                                title,
                                snippet,
                            });

                            if results.len() >= max_results {
                                break;
                            }
                        }
                    }
                }
            }
        }

        Ok(results)
    }
}
