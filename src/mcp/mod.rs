use async_trait::async_trait;
use rust_mcp_sdk::{
    macros::{mcp_tool, JsonSchema},
    mcp_server::ServerHandler,
    schema::{
        schema_utils::CallToolError, CallToolRequestParams, CallToolResult,
        ListToolsResult, PaginatedRequestParams, RpcError, TextContent,
    },
    McpServer,
};
use std::sync::Arc;
use tracing::info;

use crate::api::AppState;
use crate::models::*;

// ═══════════════════════════════════════════
//  Tool Definitions (schema only — execution is async via AppState)
// ═══════════════════════════════════════════

#[mcp_tool(
    name = "synth_search",
    description = "Search the web and get an AI-synthesized answer with sources. Uses SearXNG for privacy-respecting search and Claude for synthesis.",
    read_only_hint = true,
    open_world_hint = true
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct SynthSearchTool {
    /// Search query
    pub query: String,
    /// Search depth - "quick" (default, 5 pages) or "deep" (20 pages with query expansion)
    pub depth: Option<String>,
    /// Include YouTube video transcripts in results
    pub include_youtube: Option<bool>,
}

#[mcp_tool(
    name = "synth_extract",
    description = "Extract and analyze content from any URL. Supports web pages, PDFs, GitHub repos, images, audio, and video.",
    read_only_hint = true,
    open_world_hint = true
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct SynthExtractTool {
    /// URL to extract content from
    pub url: String,
    /// Optional context query to focus the analysis
    pub query: Option<String>,
}

#[mcp_tool(
    name = "synth_stats",
    description = "Get Synth cache statistics including number of cached pages",
    read_only_hint = true
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct SynthStatsTool {}

// ═══════════════════════════════════════════
//  MCP Server Handler
// ═══════════════════════════════════════════

pub struct SynthMcpHandler {
    pub state: Arc<AppState>,
}

#[async_trait]
impl ServerHandler for SynthMcpHandler {
    async fn handle_list_tools_request(
        &self,
        _params: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn McpServer>,
    ) -> Result<ListToolsResult, RpcError> {
        Ok(ListToolsResult {
            tools: vec![
                SynthSearchTool::tool(),
                SynthExtractTool::tool(),
                SynthStatsTool::tool(),
            ],
            meta: None,
            next_cursor: None,
        })
    }

    async fn handle_call_tool_request(
        &self,
        params: CallToolRequestParams,
        _runtime: Arc<dyn McpServer>,
    ) -> Result<CallToolResult, CallToolError> {
        let args = params.arguments.as_ref()
            .map(|m| serde_json::Value::Object(m.clone()))
            .unwrap_or(serde_json::json!({}));

        match params.name.as_str() {
            "synth_search" => self.call_search(&args).await,
            "synth_extract" => self.call_extract(&args).await,
            "synth_stats" => self.call_stats().await,
            _ => Err(CallToolError::unknown_tool(params.name)),
        }
    }
}

// ═══════════════════════════════════════════
//  Tool Implementations
// ═══════════════════════════════════════════

impl SynthMcpHandler {
    async fn call_search(&self, args: &serde_json::Value) -> Result<CallToolResult, CallToolError> {
        let query = args.get("query")
            .and_then(|q| q.as_str())
            .ok_or_else(|| CallToolError::from_message("Missing required parameter: query"))?
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

        let result = execute_search(&self.state, &request).await
            .map_err(|e| CallToolError::from_message(e))?;

        // Format response
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

        Ok(CallToolResult::text_content(vec![TextContent::from(text.trim().to_string())]))
    }

    async fn call_extract(&self, args: &serde_json::Value) -> Result<CallToolResult, CallToolError> {
        let url = args.get("url")
            .and_then(|u| u.as_str())
            .ok_or_else(|| CallToolError::from_message("Missing required parameter: url"))?
            .to_string();

        let query = args.get("query")
            .and_then(|q| q.as_str())
            .map(|s| s.to_string());

        info!("MCP synth_extract: url='{}'", url);

        let extracted = self.state.extractor
            .extract_cached(&url, &self.state.cache_manager)
            .await
            .map_err(|e| CallToolError::from_message(format!("Extraction failed: {}", e)))?;

        // Analyze with LLM if query provided
        let analysis = if let Some(ref q) = query {
            let page = ScrapedPage {
                url: extracted.url.clone(),
                title: extracted.title.clone(),
                content: extracted.content.clone(),
                word_count: extracted.content.split_whitespace().count(),
            };
            self.state.llm.analyze_page(&page, q, &self.state.cache_manager).await.ok()
        } else {
            None
        };

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

        Ok(CallToolResult::text_content(vec![TextContent::from(text.trim().to_string())]))
    }

    async fn call_stats(&self) -> Result<CallToolResult, CallToolError> {
        let stats = self.state.cache.stats()
            .map_err(|e| CallToolError::from_message(format!("Failed to get stats: {}", e)))?;

        let result = serde_json::json!({ "cached_pages": stats.num_docs });
        let text = serde_json::to_string_pretty(&result).unwrap_or_default();

        Ok(CallToolResult::text_content(vec![TextContent::from(text)]))
    }
}

// ═══════════════════════════════════════════
//  Shared Search Pipeline
// ═══════════════════════════════════════════

pub async fn execute_search(state: &Arc<AppState>, request: &SearchRequest) -> Result<SearchResponse, String> {
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
            Ok(mut results) => all_search_results.append(&mut results),
            Err(e) => info!("Search failed for '{}': {}", query, e),
        }
    }

    // Deduplicate
    all_search_results.sort_by(|a, b| a.url.cmp(&b.url));
    all_search_results.dedup_by(|a, b| a.url == b.url);

    // Rank by relevance
    let mut scored_results: Vec<_> = all_search_results
        .into_iter()
        .map(|result| {
            let score = crate::query_expansion::score_relevance(&request.query, &result.title, &result.snippet);
            (score, result)
        })
        .collect();

    scored_results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let search_results: Vec<_> = scored_results.into_iter().take(max_pages).map(|(_, r)| r).collect();

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
            async move { extractor.extract_cached(&result.url, cache).await }
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
            async move { llm.analyze_page(&page, &query, cache).await }
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
