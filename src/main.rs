mod api;
mod llm;
mod models;
mod scraper;
mod search;

use anyhow::{Context, Result};
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use api::AppState;
use llm::LLMAnalyzer;
use scraper::Scraper;
use search::SearchEngine;

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

    let brave_api_key = std::env::var("BRAVE_API_KEY")
        .context("BRAVE_API_KEY environment variable not set")?;
    
    let anthropic_api_key = std::env::var("ANTHROPIC_API_KEY")
        .context("ANTHROPIC_API_KEY environment variable not set")?;

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8765".to_string())
        .parse::<u16>()
        .context("Invalid PORT")?;

    // Initialize components
    let search_engine = SearchEngine::new(brave_api_key);
    let scraper = Scraper::new(10); // Max 10 concurrent scrapes
    let llm = LLMAnalyzer::new(anthropic_api_key);

    let state = Arc::new(AppState {
        search_engine,
        scraper,
        llm,
    });

    // Create router
    let app = api::create_router(state);

    // Start server
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    info!("🚀 OSIT server listening on http://{}", addr);
    info!("📡 API endpoints:");
    info!("  POST /search - Search and analyze");
    info!("  GET  /health - Health check");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("Failed to bind to address")?;

    axum::serve(listener, app)
        .await
        .context("Server error")?;

    Ok(())
}
