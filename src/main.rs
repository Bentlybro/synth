mod api;
mod cache;
pub mod mcp;
pub mod extractors;
mod embeddings;
mod index;
mod llm;
mod models;
mod query_expansion;
mod scraper;
mod search;
mod shared;
mod youtube;

use anyhow::{Context, Result};
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use api::AppState;
use cache::{PageCache, CacheManager};
use embeddings::EmbeddingStore;
use extractors::ExtractorRouter;
use index::SearchIndex;
use llm::LLMAnalyzer;
use scraper::Scraper;
use search::SearXNGSearch;
use youtube::YouTubeSearcher;

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
    
    let openai_api_key = std::env::var("OPENAI_API_KEY").ok();

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

    let searxng_url = std::env::var("SEARXNG_URL")
        .ok();

    info!("Using SearXNG at: {}", searxng_url.as_ref().unwrap_or(&"http://localhost:8888".to_string()));

    // Create centralized cache manager
    let cache_root = std::path::PathBuf::from(&index_path).join("cache");
    let cache_manager = CacheManager::new(cache_root.clone());
    
    // Create embedding store for semantic search
    let embedding_store = EmbeddingStore::new(cache_root, openai_api_key.clone());
    
    // Clean up old cache files and embeddings on startup
    tokio::spawn({
        let cache = cache_manager.clone();
        let embeddings = embedding_store.clone();
        async move {
            cache.cleanup("pages", 24).await;
            cache.cleanup("youtube", 168).await; // 7 days
            cache.cleanup("llm", 24).await;
            cache.cleanup("extractors_web", 24).await;
            cache.cleanup("extractors_pdf", 24).await;
            cache.cleanup("extractors_video", 168).await; // 7 days
            cache.cleanup("extractors_audio", 168).await; // 7 days
            cache.cleanup("extractors_image", 24).await;
            cache.cleanup("extractors_code", 168).await; // 7 days (commit-aware)
            embeddings.cleanup(168).await; // 7 days
        }
    });

    info!("✓ Centralized cache enabled (web/pdf/image: 24h, video/audio/code: 7d, llm: 24h)");
    
    if openai_api_key.is_some() {
        info!("✓ Semantic search enabled (OpenAI embeddings, 0.7 threshold)");
    } else {
        info!("  Semantic search disabled (no OPENAI_API_KEY)");
    }

    let cache = Arc::new(PageCache::new(Arc::clone(&index), cache_ttl));
    let search = SearXNGSearch::new(searxng_url);
    let youtube = YouTubeSearcher::new(openai_api_key.clone());
    let scraper = Scraper::new(50); // Max 50 concurrent scrapes
    let llm = LLMAnalyzer::new(anthropic_api_key);
    let extractor = ExtractorRouter::new();

    if openai_api_key.is_some() {
        info!("YouTube transcription enabled (OpenAI Whisper API)");
    } else {
        info!("YouTube transcription disabled (no OPENAI_API_KEY)");
    }

    info!("✓ Universal content extractors enabled (Web, PDF, Video, Audio, Image)");

    let state = Arc::new(AppState {
        search,
        youtube,
        cache,
        scraper,
        llm,
        cache_manager,
        extractor,
        embedding_store,
    });

    // Create REST API router
    let app = api::create_router(Arc::clone(&state));

    // Start MCP server on separate port
    let mcp_port = std::env::var("MCP_PORT")
        .unwrap_or_else(|_| "8766".to_string())
        .parse::<u16>()
        .context("Invalid MCP_PORT")?;

    let mcp_host = std::env::var("SYNTH_MCP_HOST")
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    {
        use rust_mcp_sdk::{
            mcp_server::{hyper_server, HyperServerOptions},
            schema::*,
            event_store::InMemoryEventStore,
            ToMcpServerHandler,
        };

        let mcp_handler = mcp::SynthMcpHandler { state: Arc::clone(&state) };

        let server_info = InitializeResult {
            server_info: Implementation {
                name: "synth".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                title: Some("Synth MCP Server".into()),
                description: Some("Web search, content extraction, and AI synthesis via MCP".into()),
                icons: vec![],
                website_url: None,
            },
            capabilities: ServerCapabilities {
                tools: Some(ServerCapabilitiesTools { list_changed: None }),
                ..Default::default()
            },
            protocol_version: ProtocolVersion::V2025_11_25.into(),
            instructions: Some("Synth provides web search with AI synthesis, URL content extraction (web, PDF, GitHub repos, images, audio, video), and cache statistics.".into()),
            meta: None,
        };

        let mcp_server = hyper_server::create_server(
            server_info,
            mcp_handler.to_mcp_server_handler(),
            HyperServerOptions {
                host: mcp_host.clone(),
                port: mcp_port,
                sse_support: true,
                event_store: Some(Arc::new(InMemoryEventStore::default())),
                health_endpoint: Some("/health".into()),
                ..Default::default()
            },
        );

        tokio::spawn(async move {
            info!("🔌 MCP server listening on http://{}:{}", mcp_host, mcp_port);
            info!("   Streamable HTTP: POST http://{}:{}/mcp", mcp_host, mcp_port);
            info!("   SSE (legacy):    GET  http://{}:{}/sse", mcp_host, mcp_port);
            if let Err(e) = mcp_server.start().await {
                tracing::error!("MCP server error: {}", e);
            }
        });
    }

    // Start REST API server
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    info!("🚀 REST API listening on http://{}", addr);
    info!("📡 API endpoints:");
    info!("  POST /search  - Search SearXNG, scrape, and analyze with AI");
    info!("  POST /extract - Extract content from URL");
    info!("  GET  /stats   - Cache statistics");
    info!("  GET  /health  - Health check");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("Failed to bind to address")?;

    axum::serve(listener, app)
        .await
        .context("Server error")?;

    Ok(())
}
