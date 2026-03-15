use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsResponse {
    pub cached_pages: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchDepth {
    #[serde(rename = "quick")]
    Quick,
    #[serde(rename = "deep")]
    Deep,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default = "default_depth")]
    pub depth: SearchDepth,
    #[serde(default = "default_max_pages")]
    pub max_pages: usize,
    #[serde(default)]
    pub include_youtube: bool,
    #[serde(default = "default_max_videos")]
    pub max_videos: usize,
}

fn default_depth() -> SearchDepth {
    SearchDepth::Quick
}

fn default_max_pages() -> usize {
    5
}

fn default_max_videos() -> usize {
    2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub status: SearchStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synthesis: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sources: Option<Vec<Source>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<ProgressInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchStatus {
    Searching,
    Analyzing,
    Synthesizing,
    Complete,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressInfo {
    pub results_found: usize,
    pub pages_scraped: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_page: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub url: String,
    pub title: String,
    pub key_facts: Vec<String>,
    pub quotes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub url: String,
    pub title: String,
    pub snippet: String,
}

#[derive(Debug, Clone)]
pub struct ScrapedPage {
    pub url: String,
    pub title: String,
    pub content: String,
    pub word_count: usize,
}
