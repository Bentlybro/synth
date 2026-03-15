use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tracing::info;

use crate::{
    cache::PageCache,
    llm::LLMAnalyzer,
    models::*,
    scraper::Scraper,
    search::SearXNGSearch,
    youtube::YouTubeSearcher,
};

pub struct AppState {
    pub search: SearXNGSearch,
    pub youtube: YouTubeSearcher,
    pub cache: Arc<PageCache>,
    pub scraper: Scraper,
    pub llm: LLMAnalyzer,
}

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/search", post(search_handler))
        .route("/stats", get(stats_handler))
        .route("/health", get(health_handler))
        .with_state(state)
}

async fn health_handler() -> &'static str {
    "OK"
}

async fn search_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, AppError> {
    info!("Search request: query='{}', depth={:?}, max_pages={}", 
          request.query, request.depth, request.max_pages);

    // Determine page count based on depth
    let max_pages = match request.depth {
        SearchDepth::Quick => request.max_pages.min(10),
        SearchDepth::Deep => request.max_pages.min(20),
    };

    // Step 1: Search SearXNG for URLs
    info!("Searching SearXNG for: {}", request.query);
    let search_results = state.search
        .search(&request.query, max_pages)
        .await
        .map_err(|e| AppError::SearchFailed(e.to_string()))?;

    if search_results.is_empty() {
        return Ok(Json(SearchResponse {
            status: SearchStatus::Complete,
            synthesis: Some("No results found.".to_string()),
            sources: Some(vec![]),
            progress: None,
        }));
    }

    info!("Found {} results from SearXNG", search_results.len());

    // Step 2: Scrape pages in parallel
    info!("Scraping {} pages...", search_results.len());
    let scraped_pages = state.scraper
        .scrape_parallel(search_results)
        .await;

    info!("Successfully scraped {} pages", scraped_pages.len());

    // Step 2.5: Search and transcribe YouTube videos (if enabled)
    let youtube_transcripts = if request.include_youtube {
        info!("Searching YouTube for videos...");
        match state.youtube.search_and_transcribe(&request.query, request.max_videos).await {
            Ok(videos) => {
                info!("Transcribed {} YouTube videos", videos.len());
                videos
            }
            Err(e) => {
                info!("YouTube search failed: {}", e);
                vec![]
            }
        }
    } else {
        vec![]
    };

    // Convert YouTube transcripts to ScrapedPage format
    let youtube_pages: Vec<crate::models::ScrapedPage> = youtube_transcripts
        .iter()
        .map(|video| crate::models::ScrapedPage {
            url: video.url.clone(),
            title: format!("[VIDEO] {}", video.title),
            content: video.transcript.clone(),
            word_count: video.transcript.split_whitespace().count(),
        })
        .collect();

    // Combine web pages and YouTube transcripts
    let mut all_pages = scraped_pages;
    all_pages.extend(youtube_pages);

    if all_pages.is_empty() {
        return Ok(Json(SearchResponse {
            status: SearchStatus::Complete,
            synthesis: Some("Could not scrape any pages or videos.".to_string()),
            sources: Some(vec![]),
            progress: None,
        }));
    }

    let web_count = all_pages.iter().filter(|p| !p.title.starts_with("[VIDEO]")).count();
    let video_count = all_pages.len() - web_count;
    
    info!("Total content sources: {} (web) + {} (video) = {}", 
          web_count, video_count, all_pages.len());

    // Step 3: Analyze all content with LLM (PARALLEL!)
    info!("Analyzing {} sources with LLM (concurrent)...", all_pages.len());
    
    use futures::stream::{self, StreamExt};
    
    let sources: Vec<Source> = stream::iter(all_pages)
        .map(|page| {
            let llm = &state.llm;
            let query = request.query.clone();
            async move {
                llm.analyze_page(&page, &query).await
            }
        })
        .buffer_unordered(5) // Analyze up to 5 sources concurrently
        .filter_map(|result| async move {
            match result {
                Ok(source) => {
                    info!("Analyzed: {} (confidence: {:?})", source.title, source.confidence);
                    Some(source)
                }
                Err(e) => {
                    info!("Failed to analyze source: {}", e);
                    None
                }
            }
        })
        .collect()
        .await;

    if sources.is_empty() {
        return Ok(Json(SearchResponse {
            status: SearchStatus::Complete,
            synthesis: Some("Could not analyze any pages.".to_string()),
            sources: Some(vec![]),
            progress: None,
        }));
    }

    // Step 4: Synthesize final answer
    info!("Synthesizing final answer from {} sources...", sources.len());
    let synthesis = state.llm
        .synthesize(&request.query, &sources)
        .await
        .map_err(|e| AppError::SynthesisFailed(e.to_string()))?;

    info!("Search complete!");

    Ok(Json(SearchResponse {
        status: SearchStatus::Complete,
        synthesis: Some(synthesis),
        sources: Some(sources),
        progress: None,
    }))
}

async fn stats_handler(State(state): State<Arc<AppState>>) -> Result<Json<StatsResponse>, AppError> {
    let stats = state.cache.stats()
        .map_err(|e: anyhow::Error| AppError::SearchFailed(e.to_string()))?;

    Ok(Json(StatsResponse {
        cached_pages: stats.num_docs,
    }))
}

// Error handling
#[derive(Debug)]
enum AppError {
    SearchFailed(String),
    SynthesisFailed(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::SearchFailed(msg) => (StatusCode::BAD_GATEWAY, msg),
            AppError::SynthesisFailed(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        (status, Json(serde_json::json!({
            "error": message
        }))).into_response()
    }
}
