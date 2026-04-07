use axum::{
    extract::State,
    response::IntoResponse,
    Json,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::info;

use crate::api::AppState;
use crate::models::*;

/// POST /mcp — JSON-RPC 2.0 endpoint for MCP (Model Context Protocol)
pub async fn mcp_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<Value>,
) -> impl IntoResponse {
    // Handle batch requests (array of JSON-RPC)
    if let Some(arr) = request.as_array() {
        let mut responses = Vec::new();
        for req in arr {
            responses.push(handle_single_request(&state, req).await);
        }
        return Json(json!(responses));
    }

    Json(handle_single_request(&state, &request).await)
}

async fn handle_single_request(state: &Arc<AppState>, request: &Value) -> Value {
    let id = request.get("id").cloned();
    let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = request.get("params").cloned().unwrap_or(json!({}));

    // Notifications have no id and expect no response — but we handle them gracefully
    match method {
        "initialize" => jsonrpc_result(id, handle_initialize()),
        "notifications/initialized" => {
            // Notification — no response needed, but if id present, acknowledge
            if id.is_some() {
                jsonrpc_result(id, json!({}))
            } else {
                json!(null)
            }
        }
        "tools/list" => jsonrpc_result(id, handle_tools_list()),
        "tools/call" => {
            match handle_tools_call(state, &params).await {
                Ok(result) => jsonrpc_result(id, result),
                Err(e) => jsonrpc_error(id, -32000, &e),
            }
        }
        _ => jsonrpc_error(id, -32601, &format!("Method not found: {}", method)),
    }
}

fn jsonrpc_result(id: Option<Value>, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

fn jsonrpc_error(id: Option<Value>, code: i32, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}

fn handle_initialize() -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "synth",
            "version": env!("CARGO_PKG_VERSION")
        }
    })
}

fn handle_tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": "synth_search",
                "description": "Search the web and get an AI-synthesized answer with sources. Uses SearXNG for privacy-respecting search and Claude for synthesis.",
                "inputSchema": {
                    "type": "object",
                    "required": ["query"],
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query"
                        },
                        "depth": {
                            "type": "string",
                            "enum": ["quick", "deep"],
                            "description": "Search depth - quick (default, 5 pages) or deep (20 pages with query expansion)"
                        },
                        "include_youtube": {
                            "type": "boolean",
                            "description": "Include YouTube video transcripts"
                        }
                    }
                }
            },
            {
                "name": "synth_extract",
                "description": "Extract and analyze content from any URL. Supports web pages, PDFs, GitHub repos, images, audio, and video.",
                "inputSchema": {
                    "type": "object",
                    "required": ["url"],
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "URL to extract content from"
                        },
                        "query": {
                            "type": "string",
                            "description": "Optional context query to focus the analysis"
                        }
                    }
                }
            },
            {
                "name": "synth_stats",
                "description": "Get Synth cache statistics",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            }
        ]
    })
}

async fn handle_tools_call(state: &Arc<AppState>, params: &Value) -> Result<Value, String> {
    let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    match tool_name {
        "synth_search" => call_synth_search(state, &arguments).await,
        "synth_extract" => call_synth_extract(state, &arguments).await,
        "synth_stats" => call_synth_stats(state).await,
        _ => Err(format!("Unknown tool: {}", tool_name)),
    }
}

async fn call_synth_search(state: &Arc<AppState>, args: &Value) -> Result<Value, String> {
    let query = args.get("query")
        .and_then(|q| q.as_str())
        .ok_or("Missing required parameter: query")?
        .to_string();

    let depth = match args.get("depth").and_then(|d| d.as_str()) {
        Some("deep") => SearchDepth::Deep,
        _ => SearchDepth::Quick,
    };

    let include_youtube = args.get("include_youtube")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let max_pages = match depth {
        SearchDepth::Quick => 5,
        SearchDepth::Deep => 20,
    };

    info!("MCP synth_search: query='{}', depth={:?}", query, depth);

    let request = SearchRequest {
        query,
        depth,
        max_pages,
        include_youtube,
        max_videos: 2,
    };

    // Reuse the search logic — build the search pipeline inline
    // This mirrors the search_handler logic from api/mod.rs
    let result = execute_search(state, &request).await?;

    // Format nicely for MCP
    let mut text = String::new();

    if let Some(synthesis) = &result.synthesis {
        text.push_str(synthesis);
        text.push_str("\n\n");
    }

    if let Some(sources) = &result.sources {
        if !sources.is_empty() {
            text.push_str("## Sources\n\n");
            for (i, source) in sources.iter().enumerate() {
                text.push_str(&format!("{}. **{}**\n", i + 1, source.title));
                text.push_str(&format!("   URL: {}\n", source.url));
                if !source.key_facts.is_empty() {
                    text.push_str("   Key facts:\n");
                    for fact in &source.key_facts {
                        text.push_str(&format!("   - {}\n", fact));
                    }
                }
                text.push('\n');
            }
        }
    }

    Ok(json!({
        "content": [
            {"type": "text", "text": text.trim()}
        ]
    }))
}

async fn call_synth_extract(state: &Arc<AppState>, args: &Value) -> Result<Value, String> {
    let url = args.get("url")
        .and_then(|u| u.as_str())
        .ok_or("Missing required parameter: url")?
        .to_string();

    let query = args.get("query")
        .and_then(|q| q.as_str())
        .map(|s| s.to_string());

    info!("MCP synth_extract: url='{}'", url);

    // Step 1: Extract content
    let extracted = state.extractor
        .extract_cached(&url, &state.cache_manager)
        .await
        .map_err(|e| format!("Extraction failed: {}", e))?;

    // Step 2: Analyze with LLM if query provided
    let analysis = if let Some(ref q) = query {
        let page = ScrapedPage {
            url: extracted.url.clone(),
            title: extracted.title.clone(),
            content: extracted.content.clone(),
            word_count: extracted.content.split_whitespace().count(),
        };
        state.llm.analyze_page(&page, q, &state.cache_manager).await.ok()
    } else {
        None
    };

    // Format result
    let mut text = String::new();
    text.push_str(&format!("# {}\n\n", extracted.title));
    text.push_str(&format!("**URL:** {}\n", extracted.url));
    text.push_str(&format!("**Type:** {:?}\n\n", extracted.content_type));
    text.push_str(&extracted.content);

    if let Some(source) = analysis {
        text.push_str("\n\n## Analysis\n\n");
        if !source.key_facts.is_empty() {
            text.push_str("**Key facts:**\n");
            for fact in &source.key_facts {
                text.push_str(&format!("- {}\n", fact));
            }
        }
        if !source.quotes.is_empty() {
            text.push_str("\n**Notable quotes:**\n");
            for quote in &source.quotes {
                text.push_str(&format!("> {}\n", quote));
            }
        }
    }

    Ok(json!({
        "content": [
            {"type": "text", "text": text.trim()}
        ]
    }))
}

async fn call_synth_stats(state: &Arc<AppState>) -> Result<Value, String> {
    let stats = state.cache.stats()
        .map_err(|e| format!("Failed to get stats: {}", e))?;

    let result = json!({
        "cached_pages": stats.num_docs
    });

    Ok(json!({
        "content": [
            {"type": "text", "text": serde_json::to_string_pretty(&result).unwrap_or_default()}
        ]
    }))
}

/// Execute a search using the existing pipeline (mirrors search_handler logic)
async fn execute_search(state: &Arc<AppState>, request: &SearchRequest) -> Result<SearchResponse, String> {
    use futures::stream::{self, StreamExt};

    let max_pages = match request.depth {
        SearchDepth::Quick => request.max_pages.min(10),
        SearchDepth::Deep => request.max_pages.min(20),
    };

    // Step 1: Search SearXNG
    let mut all_search_results = Vec::new();
    let queries_to_search = if matches!(request.depth, SearchDepth::Deep) {
        crate::query_expansion::expand_query(&request.query)
    } else {
        vec![request.query.clone()]
    };

    for (i, query) in queries_to_search.iter().enumerate() {
        info!("MCP search [{}/{}]: {}", i + 1, queries_to_search.len(), query);
        match state.search.search(query, max_pages / queries_to_search.len()).await {
            Ok(mut results) => {
                all_search_results.append(&mut results);
            }
            Err(e) => {
                info!("Search failed for '{}': {}", query, e);
            }
        }
    }

    // Deduplicate
    all_search_results.sort_by(|a, b| a.url.cmp(&b.url));
    all_search_results.dedup_by(|a, b| a.url == b.url);

    // Rank by relevance
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
        .take(max_pages)
        .map(|(_, result)| result)
        .collect();

    if search_results.is_empty() {
        return Ok(SearchResponse {
            status: SearchStatus::Complete,
            synthesis: Some("No results found.".to_string()),
            sources: Some(vec![]),
            progress: None,
        });
    }

    // Step 2: Extract content
    let extracted_content: Vec<crate::extractors::ExtractedContent> = stream::iter(search_results)
        .map(|result| {
            let extractor = &state.extractor;
            let cache = &state.cache_manager;
            async move {
                extractor.extract_cached(&result.url, cache).await
            }
        })
        .buffer_unordered(15)
        .filter_map(|result| async move { result.ok() })
        .collect()
        .await;

    // YouTube
    let youtube_content = if request.include_youtube {
        match state.youtube.search_and_transcribe(&request.query, request.max_videos, &state.cache_manager).await {
            Ok(videos) => videos.into_iter()
                .map(|video| crate::extractors::ExtractedContent {
                    url: video.url,
                    title: video.title,
                    content: video.transcript,
                    content_type: crate::extractors::ContentType::Video,
                    metadata: None,
                })
                .collect(),
            Err(_) => vec![],
        }
    } else {
        vec![]
    };

    let mut all_content = extracted_content;
    all_content.extend(youtube_content);

    if all_content.is_empty() {
        return Ok(SearchResponse {
            status: SearchStatus::Complete,
            synthesis: Some("Could not extract any content.".to_string()),
            sources: Some(vec![]),
            progress: None,
        });
    }

    // Step 3: Analyze with LLM
    let all_pages: Vec<ScrapedPage> = all_content
        .iter()
        .map(|content| ScrapedPage {
            url: content.url.clone(),
            title: content.title.clone(),
            content: content.content.clone(),
            word_count: content.content.split_whitespace().count(),
        })
        .collect();

    let sources: Vec<Source> = stream::iter(all_pages)
        .map(|page| {
            let llm = &state.llm;
            let cache = &state.cache_manager;
            let query = request.query.clone();
            async move {
                llm.analyze_page(&page, &query, cache).await
            }
        })
        .buffer_unordered(5)
        .filter_map(|result| async move { result.ok() })
        .collect()
        .await;

    if sources.is_empty() {
        return Ok(SearchResponse {
            status: SearchStatus::Complete,
            synthesis: Some("Could not analyze any pages.".to_string()),
            sources: Some(vec![]),
            progress: None,
        });
    }

    // Step 4: Synthesize
    let synthesis = state.llm
        .synthesize(&request.query, &sources)
        .await
        .map_err(|e| format!("Synthesis failed: {}", e))?;

    Ok(SearchResponse {
        status: SearchStatus::Complete,
        synthesis: Some(synthesis),
        sources: Some(sources),
        progress: None,
    })
}
