mod api;
mod cache;
mod index;
mod llm;
mod models;
mod scraper;
mod search;

use anyhow::{Context, Result};
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use api::AppState;
use cache::PageCache;
use index::SearchIndex;
use llm::LLMAnalyzer;
use scraper::Scraper;
use search::DuckDuckGoSearch;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "osit=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment variables
    dotenvy::dotenv().ok();
    
    let anthropic_api_key = std::env::var("ANTHROPIC_API_KEY")
        .context("ANTHROPIC_API_KEY environment variable not set")?;

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8765".to_string())
        .parse::<u16>()
        .context("Invalid PORT")?;

    let index_path = std::env::var("INDEX_PATH")
        .unwrap_or_else(|_| "./index".to_string());

    // Initialize components
    info!("Initializing page cache at {}", index_path);
    let index = Arc::new(SearchIndex::new(&index_path)?);
    
    let stats = index.stats()?;
    info!("Cache loaded: {} pages cached", stats.num_docs);

    let cache_ttl = std::env::var("CACHE_TTL_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(86400); // 24 hours default

    let cache = Arc::new(PageCache::new(Arc::clone(&index), cache_ttl));
    let search = DuckDuckGoSearch::new();
    let scraper = Scraper::new(50); // Max 50 concurrent scrapes
    let llm = LLMAnalyzer::new(anthropic_api_key);

    let state = Arc::new(AppState {
        search,
        cache,
        scraper,
        llm,
    });

    // Create router
    let app = api::create_router(state);

    // Start server
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    info!("🚀 OSIT server listening on http://{}", addr);
    info!("📡 API endpoints:");
    info!("  POST /search - Search DuckDuckGo, scrape, and analyze with AI");
    info!("  GET  /stats  - Cache statistics");
    info!("  GET  /health - Health check");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("Failed to bind to address")?;

    axum::serve(listener, app)
        .await
        .context("Server error")?;

    Ok(())
}
