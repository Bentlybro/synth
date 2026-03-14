use anyhow::{Context, Result};
use futures::stream::{self, StreamExt};
use governor::{Quota, RateLimiter};
use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tracing::{info, warn};
use url::Url;

use crate::index::{IndexedPage, SearchIndex};

type DirectRateLimiter = RateLimiter<
    governor::state::direct::NotKeyed,
    governor::state::InMemoryState,
    governor::clock::DefaultClock,
>;

pub struct Crawler {
    client: Client,
    index: Arc<SearchIndex>,
    visited: Arc<RwLock<HashSet<String>>>,
    queue: Arc<RwLock<VecDeque<String>>>,
    rate_limiter: Arc<DirectRateLimiter>,
    max_depth: usize,
    max_concurrent: usize,
}

impl Crawler {
    pub fn new(index: Arc<SearchIndex>) -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (compatible; OSIT/0.1; +https://github.com/bentlybro/osit)")
            .timeout(Duration::from_secs(15))
            .build()
            .expect("Failed to build HTTP client");

        // Rate limit: 5 requests per second
        let rate_limiter = Arc::new(RateLimiter::direct(
            Quota::per_second(std::num::NonZeroU32::new(5).unwrap())
        ));

        Self {
            client,
            index,
            visited: Arc::new(RwLock::new(HashSet::new())),
            queue: Arc::new(RwLock::new(VecDeque::new())),
            rate_limiter,
            max_depth: 3,
            max_concurrent: 5,
        }
    }

    /// Seed the crawler with initial URLs
    pub fn seed(&self, urls: Vec<String>) {
        let mut queue = self.queue.write().unwrap();
        for url in urls {
            queue.push_back(url);
        }
    }

    /// Start crawling (blocking)
    pub async fn crawl(&self, max_pages: usize) -> Result<()> {
        let mut pages_crawled = 0;

        while pages_crawled < max_pages {
            // Get next batch of URLs
            let urls: Vec<String> = {
                let mut queue = self.queue.write().unwrap();
                (0..self.max_concurrent)
                    .filter_map(|_| queue.pop_front())
                    .collect()
            };

            if urls.is_empty() {
                info!("Queue empty, crawl complete");
                break;
            }

            // Crawl batch in parallel
            let results: Vec<_> = stream::iter(urls)
                .map(|url| {
                    let crawler = self.clone_refs();
                    async move { crawler.crawl_page(&url).await }
                })
                .buffer_unordered(self.max_concurrent)
                .collect()
                .await;

            // Process results
            for result in results {
                if let Ok((indexed_page, links)) = result {
                    // Add to index
                    if let Err(e) = self.index.add_page(indexed_page.clone()) {
                        warn!("Failed to index {}: {}", indexed_page.url, e);
                    } else {
                        pages_crawled += 1;
                        info!("Indexed: {} ({}/{})", indexed_page.url, pages_crawled, max_pages);
                    }

                    // Add new links to queue
                    let mut queue = self.queue.write().unwrap();
                    for link in links {
                        if !self.is_visited(&link) {
                            queue.push_back(link);
                        }
                    }
                }
            }

            // Commit batch
            if let Err(e) = self.index.commit() {
                warn!("Failed to commit index: {}", e);
            }
        }

        Ok(())
    }

    /// Crawl a single page
    async fn crawl_page(&self, url: &str) -> Result<(IndexedPage, Vec<String>)> {
        // Mark as visited
        self.mark_visited(url);

        // Rate limit
        self.rate_limiter.until_ready().await;

        // Fetch page
        let response = self.client
            .get(url)
            .send()
            .await
            .context("Failed to fetch page")?;

        if !response.status().is_success() {
            anyhow::bail!("HTTP {}", response.status());
        }

        let html = response.text().await.context("Failed to read body")?;
        let document = Html::parse_document(&html);

        // Extract title
        let title = document
            .select(&Selector::parse("title").unwrap())
            .next()
            .map(|el| el.text().collect::<String>())
            .unwrap_or_else(|| "Untitled".to_string());

        // Extract content
        let content = self.extract_content(&document);

        // Extract links
        let links = self.extract_links(&document, url);

        // Get domain
        let parsed_url = Url::parse(url)?;
        let domain = parsed_url.host_str().unwrap_or("").to_string();

        Ok((
            IndexedPage {
                url: url.to_string(),
                title,
                content,
                domain,
            },
            links,
        ))
    }

    fn extract_content(&self, document: &Html) -> String {
        // Try to find main content
        let selectors = vec!["article", "main", "[role='main']", ".content", "#content"];

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

        // Fallback to body
        let body_text = document.root_element().text().collect::<Vec<_>>().join(" ");
        self.clean_text(&body_text)
    }

    fn extract_links(&self, document: &Html, base_url: &str) -> Vec<String> {
        let selector = Selector::parse("a[href]").unwrap();
        let base = Url::parse(base_url).ok();

        document
            .select(&selector)
            .filter_map(|el| el.value().attr("href"))
            .filter_map(|href| {
                if let Some(base) = &base {
                    base.join(href).ok()
                } else {
                    Url::parse(href).ok()
                }
            })
            .filter(|url| url.scheme() == "http" || url.scheme() == "https")
            .map(|url| url.to_string())
            .collect()
    }

    fn clean_text(&self, text: &str) -> String {
        text.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(50000)
            .collect()
    }

    fn mark_visited(&self, url: &str) {
        self.visited.write().unwrap().insert(url.to_string());
    }

    fn is_visited(&self, url: &str) -> bool {
        self.visited.read().unwrap().contains(url)
    }

    fn clone_refs(&self) -> Self {
        Self {
            client: self.client.clone(),
            index: Arc::clone(&self.index),
            visited: Arc::clone(&self.visited),
            queue: Arc::clone(&self.queue),
            rate_limiter: Arc::clone(&self.rate_limiter),
            max_depth: self.max_depth,
            max_concurrent: self.max_concurrent,
        }
    }
}
