use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use futures::stream::{self, StreamExt};
use std::sync::Arc;
use tracing::info;

use crate::{
    cache::{PageCache, CacheManager},
    extractors::ExtractorRouter,
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
    pub cache_manager: CacheManager,
    pub extractor: ExtractorRouter,
    pub embedding_store: crate::embeddings::EmbeddingStore,
}

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/search", post(search_handler))
        .route("/extract", post(extract_handler))
        .route("/stats", get(stats_handler))
        .route("/health", get(health_handler))
        // MCP server runs on separate port via rust-mcp-sdk
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

    // Step 0: Check semantic search for similar cached queries (SMART CACHE!)
    info!("Checking semantic cache for similar queries...");
    if let Ok(similar_matches) = state.embedding_store.find_similar(&request.query, 0.7, 5).await {
        if !similar_matches.is_empty() {
            info!("Found {} semantically similar cached results!", similar_matches.len());
            info!("Top match: '{}' (similarity: {:.2})", 
                  similar_matches[0].text, similar_matches[0].similarity);
            
            // If we have a very high similarity match (>0.85), we could return cached results
            // For now, just log it - we'll still do the search but this is FAST PATH ready
        }
    }

    // Step 1: Search SearXNG for URLs (with query expansion for deep mode!)
    let mut all_search_results = Vec::new();
    
    let queries_to_search = if matches!(request.depth, SearchDepth::Deep) {
        // Deep mode: use query expansion for MAXIMUM coverage
        let expanded = crate::query_expansion::expand_query(&request.query);
        info!("Deep mode: Searching with {} expanded queries", expanded.len());
        expanded
    } else {
        // Quick mode: just the original query
        vec![request.query.clone()]
    };
    
    for (i, query) in queries_to_search.iter().enumerate() {
        info!("Searching SearXNG [{}/{}]: {}", i + 1, queries_to_search.len(), query);
        match state.search.search(query, max_pages / queries_to_search.len()).await {
            Ok(mut results) => {
                info!("Found {} results for: {}", results.len(), query);
                all_search_results.append(&mut results);
            }
            Err(e) => {
                info!("Search failed for '{}': {}", query, e);
            }
        }
    }
    
    // Deduplicate by URL
    all_search_results.sort_by(|a, b| a.url.cmp(&b.url));
    all_search_results.dedup_by(|a, b| a.url == b.url);
    
    // Sort by relevance score (SMART RANKING!)
    info!("Ranking {} results by relevance...", all_search_results.len());
    let mut scored_results: Vec<_> = all_search_results
        .into_iter()
        .map(|result| {
            let score = crate::query_expansion::score_relevance(
                &request.query,
                &result.title,
                &result.snippet,
            );
            (score, result)
        })
        .collect();
    
    scored_results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    
    let search_results: Vec<_> = scored_results
        .into_iter()
        .take(max_pages) // Take top N most relevant
        .map(|(score, result)| {
            info!("  {} - {} (score: {:.1})", result.url, result.title, score);
            result
        })
        .collect();

    if search_results.is_empty() {
        return Ok(Json(SearchResponse {
            status: SearchStatus::Complete,
            synthesis: Some("No results found.".to_string()),
            sources: Some(vec![]),
            progress: None,
        }));
    }

    info!("Found {} results from SearXNG", search_results.len());

    // Step 2: Extract content from all URLs using ExtractorRouter (with caching)
    info!("Extracting content from {} URLs...", search_results.len());
    
    let extracted_content: Vec<crate::extractors::ExtractedContent> = stream::iter(search_results)
        .map(|result| {
            let extractor = &state.extractor;
            let cache = &state.cache_manager;
            async move {
                extractor.extract_cached(&result.url, cache).await
            }
        })
        .buffer_unordered(15) // Extract up to 15 URLs concurrently (FAST!)
        .filter_map(|result| async move {
            match result {
                Ok(content) => {
                    info!("Extracted: {} ({})", content.title, format!("{:?}", content.content_type));
                    Some(content)
                }
                Err(e) => {
                    info!("Extraction failed: {}", e);
                    None
                }
            }
        })
        .collect()
        .await;

    // Step 2.5: Add YouTube videos if requested
    let youtube_content = if request.include_youtube {
        info!("Searching YouTube for videos...");
        match state.youtube.search_and_transcribe(&request.query, request.max_videos, &state.cache_manager).await {
            Ok(videos) => {
                info!("Transcribed {} YouTube videos", videos.len());
                videos.into_iter()
                    .map(|video| crate::extractors::ExtractedContent {
                        url: video.url,
                        title: video.title,
                        content: video.transcript,
                        content_type: crate::extractors::ContentType::Video,
                        metadata: None,
                    })
                    .collect()
            }
            Err(e) => {
                info!("YouTube search failed: {}", e);
                vec![]
            }
        }
    } else {
        vec![]
    };

    // Combine all extracted content
    let mut all_content = extracted_content;
    all_content.extend(youtube_content);

    if all_content.is_empty() {
        return Ok(Json(SearchResponse {
            status: SearchStatus::Complete,
            synthesis: Some("Could not extract any content.".to_string()),
            sources: Some(vec![]),
            progress: None,
        }));
    }

    // Step 2.7: Store embeddings for semantic search (async, don't wait)
    info!("Storing embeddings for {} extracted items...", all_content.len());
    for content in &all_content {
        let embedding_store = state.embedding_store.clone();
        let key = format!("{}::{}", content.url, request.query);
        let text = content.content.clone();
        let url = content.url.clone();
        let content_type = format!("{:?}", content.content_type);
        
        // Spawn async task to store embedding (don't block)
        tokio::spawn(async move {
            if let Err(e) = embedding_store.store_embedding(&key, &text, &url, &content_type).await {
                info!("Failed to store embedding: {}", e);
            }
        });
    }

    // Count content types
    let web_count = all_content.iter().filter(|c| matches!(c.content_type, crate::extractors::ContentType::Web)).count();
    let pdf_count = all_content.iter().filter(|c| matches!(c.content_type, crate::extractors::ContentType::PDF)).count();
    let video_count = all_content.iter().filter(|c| matches!(c.content_type, crate::extractors::ContentType::Video)).count();
    let image_count = all_content.iter().filter(|c| matches!(c.content_type, crate::extractors::ContentType::Image)).count();
    let audio_count = all_content.iter().filter(|c| matches!(c.content_type, crate::extractors::ContentType::Audio)).count();
    
    info!("Total content: {} web, {} PDF, {} video, {} image, {} audio", 
          web_count, pdf_count, video_count, image_count, audio_count);

    // Convert ExtractedContent to ScrapedPage format for LLM analysis
    let all_pages: Vec<crate::models::ScrapedPage> = all_content
        .iter()
        .map(|content| crate::models::ScrapedPage {
            url: content.url.clone(),
            title: content.title.clone(),
            content: content.content.clone(),
            word_count: content.content.split_whitespace().count(),
        })
        .collect();

    // Step 3: Analyze all content with LLM (PARALLEL + CACHED!)
    info!("Analyzing {} sources with LLM (concurrent)...", all_pages.len());
    
    let sources: Vec<Source> = stream::iter(all_pages)
        .map(|page| {
            let llm = &state.llm;
            let cache = &state.cache_manager;
            let query = request.query.clone();
            async move {
                llm.analyze_page(&page, &query, cache).await
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

async fn extract_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ExtractRequest>,
) -> Result<Json<ExtractResponse>, AppError> {
    info!("Extract request: url='{}'", request.url);

    // Step 1: Extract content using ExtractorRouter (with caching)
    let extracted = state.extractor
        .extract_cached(&request.url, &state.cache_manager)
        .await
        .map_err(|e| AppError::SearchFailed(format!("Extraction failed: {}", e)))?;

    info!("Extracted {} content: {}", 
          format!("{:?}", extracted.content_type), extracted.title);

    // Step 2: Analyze with LLM if query provided
    let analysis = if let Some(query) = &request.query {
        info!("Analyzing content with query: {}", query);
        
        // Convert ExtractedContent to ScrapedPage format for LLM analysis
        let page = crate::models::ScrapedPage {
            url: extracted.url.clone(),
            title: extracted.title.clone(),
            content: extracted.content.clone(),
            word_count: extracted.content.split_whitespace().count(),
        };
        
        match state.llm.analyze_page(&page, query, &state.cache_manager).await {
            Ok(source) => Some(source),
            Err(e) => {
                info!("LLM analysis failed: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Step 3: Convert metadata from JSON Value
    let metadata = extracted.metadata.as_ref().map(|m| ContentMetadata {
        duration_seconds: m.get("duration_seconds").and_then(|v| v.as_f64()),
        file_size_bytes: m.get("file_size_bytes").and_then(|v| v.as_u64()),
        format: m.get("format").and_then(|v| v.as_str()).map(|s| s.to_string()),
        dimensions: m.get("dimensions").and_then(|v| {
            if let Some(arr) = v.as_array() {
                if arr.len() == 2 {
                    let w = arr[0].as_u64()? as u32;
                    let h = arr[1].as_u64()? as u32;
                    return Some((w, h));
                }
            }
            None
        }),
    });

    Ok(Json(ExtractResponse {
        url: extracted.url,
        title: extracted.title,
        content_type: format!("{:?}", extracted.content_type),
        content: extracted.content,
        analysis,
        metadata,
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
