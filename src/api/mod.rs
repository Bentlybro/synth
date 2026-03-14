use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use std::sync::Arc;
use tracing::info;

use crate::{
    llm::LLMAnalyzer,
    models::*,
    scraper::Scraper,
    search::SearchEngine,
};

pub struct AppState {
    pub search_engine: SearchEngine,
    pub scraper: Scraper,
    pub llm: LLMAnalyzer,
}

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/search", post(search_handler))
        .route("/health", axum::routing::get(health_handler))
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
        SearchDepth::Quick => request.max_pages.min(5),
        SearchDepth::Deep => request.max_pages.min(20),
    };

    // Step 1: Search
    info!("Searching for: {}", request.query);
    let search_results = state.search_engine
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

    info!("Found {} results", search_results.len());

    // Step 2: Scrape pages in parallel
    info!("Scraping {} pages...", search_results.len());
    let scraped_pages = state.scraper
        .scrape_parallel(search_results)
        .await;

    info!("Successfully scraped {} pages", scraped_pages.len());

    if scraped_pages.is_empty() {
        return Ok(Json(SearchResponse {
            status: SearchStatus::Complete,
            synthesis: Some("Could not scrape any pages.".to_string()),
            sources: Some(vec![]),
            progress: None,
        }));
    }

    // Step 3: Analyze each page with LLM
    info!("Analyzing pages with LLM...");
    let mut sources = Vec::new();
    
    for page in &scraped_pages {
        match state.llm.analyze_page(page, &request.query).await {
            Ok(source) => {
                info!("Analyzed: {} (confidence: {:?})", source.title, source.confidence);
                sources.push(source);
            }
            Err(e) => {
                info!("Failed to analyze {}: {}", page.url, e);
            }
        }
    }

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
