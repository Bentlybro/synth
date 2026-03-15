# Architecture Overview

Synth's architecture is designed for modularity, performance, and cacheability.

## System Design

```
┌─────────────────────────────────────────────────────────────────┐
│                        HTTP API (Axum)                          │
│                    localhost:8765                                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │  /search     │  │  /extract    │  │  /health     │          │
│  │  /stats      │  │              │  │              │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                     Core Components                              │
│                                                                  │
│  ┌──────────────────┐                 ┌──────────────────┐     │
│  │  SearXNG Client  │                 │  ExtractorRouter │     │
│  │  (search.rs)     │                 │  (extractors/)   │     │
│  │                  │                 │                  │     │
│  │ • Query SearXNG  │                 │ • Auto-routing   │     │
│  │ • Parse results  │                 │ • 5 extractors   │     │
│  │ • Return URLs    │                 │ • Type detection │     │
│  └──────────────────┘                 └──────────────────┘     │
│           │                                     │               │
│           │                                     ↓               │
│           │                    ┌────────────────────────┐      │
│           │                    │  Content Extractors    │      │
│           │                    │                        │      │
│           │                    │ • Web (HTML scraping)  │      │
│           │                    │ • PDF (text extract)   │      │
│           │                    │ • Video (yt-dlp+Whisper)│     │
│           │                    │ • Audio (Whisper)      │      │
│           │                    │ • Image (Claude Vision)│      │
│           │                    └────────────────────────┘      │
│           │                                     │               │
│           └─────────────┬───────────────────────┘               │
│                         ↓                                       │
│                ┌──────────────────┐                             │
│                │  CacheManager    │                             │
│                │  (cache/)        │                             │
│                │                  │                             │
│                │ • Per-type TTLs  │                             │
│                │ • Tantivy-based  │                             │
│                │ • Auto-cleanup   │                             │
│                └──────────────────┘                             │
│                         ↓                                       │
│                ┌──────────────────┐                             │
│                │  LLM Analyzer    │                             │
│                │  (llm.rs)        │                             │
│                │                  │                             │
│                │ • Claude API     │                             │
│                │ • Parallel       │                             │
│                │ • Synthesis      │                             │
│                └──────────────────┘                             │
└─────────────────────────────────────────────────────────────────┘
```

## Request Flow

### /search Endpoint

1. **Receive Request**
   - Query, max_pages, depth, include_youtube
   - Validate parameters

2. **Search Phase**
   - Query SearXNG
   - Parse results (URLs, titles, snippets)
   - Return N URLs

3. **Extraction Phase** (parallel)
   - For each URL:
     - Detect content type
     - Check cache (key: `hash(url)`)
     - If hit: return cached content
     - If miss: extract → cache → return
   - Up to 10 concurrent extractions

4. **YouTube Phase** (if enabled)
   - Search YouTube via yt-dlp
   - Download audio (16K compression)
   - Transcribe with Whisper API
   - Cache transcript (7 day TTL)

5. **Analysis Phase** (parallel)
   - For each extracted content:
     - Check cache (key: `hash(url + query)`)
     - If hit: return cached analysis
     - If miss: analyze with Claude → cache
   - Up to 5 concurrent analyses
   - Extract: key facts, quotes, confidence

6. **Synthesis Phase**
   - Combine all sources
   - Claude generates markdown synthesis
   - Include citations
   - Return response

### /extract Endpoint

1. **Receive Request**
   - URL, optional query

2. **Extract Content**
   - Detect content type from URL
   - Route to appropriate extractor
   - Check cache → extract → cache

3. **Analyze** (if query provided)
   - Send content + query to Claude
   - Extract key facts, quotes
   - Cache analysis

4. **Return Response**
   - URL, title, content_type, content
   - Optional analysis
   - Optional metadata

## Component Details

### API Layer (src/api/)

**Framework:** Axum (Tokio-based async HTTP)

**Endpoints:**
- `POST /search` - Multi-modal search
- `POST /extract` - Direct URL extraction
- `GET /health` - Health check
- `GET /stats` - Cache statistics

**State Management:**
```rust
struct AppState {
    search: SearXNGSearch,
    youtube: YouTubeSearcher,
    cache: Arc<PageCache>,        // Legacy
    scraper: Scraper,
    llm: LLMAnalyzer,
    cache_manager: CacheManager,  // New unified cache
    extractor: ExtractorRouter,   // Universal extraction
}
```

### ExtractorRouter (src/extractors/mod.rs)

**Purpose:** Route URLs to appropriate content extractors

**How it works:**
1. Iterate through extractors in priority order
2. Call `can_handle(url)` on each
3. First match wins
4. Call `extract(url)` on matched extractor

**Priority order:**
1. PDF (`.pdf` extension)
2. Video (YouTube, Vimeo, etc. domains)
3. Image (`.jpg`, `.png`, etc.)
4. Audio (`.mp3`, `.wav`, etc.)
5. Web (fallback for any HTTP/HTTPS)

**Caching:**
```rust
pub async fn extract_cached(&self, url: &str, cache: &CacheManager) -> Result<ExtractedContent> {
    let key = cache_key(url);
    let (category, ttl) = match self.detect_type(url) {
        Some(ContentType::PDF) => ("extractors_pdf", 24),
        Some(ContentType::Video) => ("extractors_video", 168),
        Some(ContentType::Audio) => ("extractors_audio", 168),
        Some(ContentType::Image) => ("extractors_image", 24),
        Some(ContentType::Web) | None => ("extractors_web", 24),
    };
    
    if let Some(cached) = cache.get(category, &key, ttl).await {
        return Ok(cached);
    }
    
    let content = self.extract(url).await?;
    cache.put(category, &key, &content).await.ok();
    Ok(content)
}
```

### CacheManager (src/cache/manager.rs)

**Purpose:** Unified caching for all content types

**Storage:** Tantivy-based (originally for search, repurposed for JSON storage)

**Structure:**
```
index/cache/
├── extractors_web/     # Web pages (24h TTL)
├── extractors_pdf/     # PDFs (24h TTL)
├── extractors_video/   # Videos (168h TTL)
├── extractors_audio/   # Audio (168h TTL)
├── extractors_image/   # Images (24h TTL)
├── llm/                # LLM analyses (24h TTL)
├── pages/              # Legacy web cache
└── youtube/            # Legacy YouTube cache
```

**API:**
```rust
pub async fn get<T: DeserializeOwned>(&self, category: &str, key: &str, ttl_hours: u64) -> Option<T>
pub async fn put<T: Serialize>(&self, category: &str, key: &str, data: T) -> Result<()>
pub async fn cleanup(&self, category: &str, ttl_hours: u64)
```

**Cache keys:**
- Web/PDF/Video/Audio/Image: `hash(url)`
- LLM analysis: `hash(url + query)`

### LLM Analyzer (src/llm/mod.rs)

**Purpose:** Claude API integration for analysis and synthesis

**Model:** `claude-sonnet-4-20250514`

**Two phases:**

1. **Analysis** (per source)
   - Input: Content + query
   - Prompt: Extract key facts, quotes, confidence
   - Output: JSON with structured data
   - Cache: 24 hours (key: `hash(url + query)`)

2. **Synthesis** (all sources)
   - Input: All analyzed sources + query
   - Prompt: Combine into comprehensive markdown
   - Output: Markdown with citations
   - No cache (always fresh synthesis)

### SearXNG Client (src/search/)

**Purpose:** Query SearXNG metasearch engine

**Endpoint:** `{SEARXNG_URL}/search?q={query}&format=json`

**Processing:**
- Parse JSON response
- Extract URLs, titles, snippets
- Return as `Vec<SearchResult>`

## Data Flow Diagram

```
User Request
    ↓
┌────────────────────────────────────────┐
│ API Handler (search_handler)          │
│ • Validate input                       │
│ • Set max_pages based on depth        │
└────────────────────────────────────────┘
    ↓
┌────────────────────────────────────────┐
│ SearXNG Search                         │
│ • Query: "rust async performance"     │
│ • Returns: 10 URLs                     │
└────────────────────────────────────────┘
    ↓
┌────────────────────────────────────────┐
│ Parallel Extraction (10 concurrent)   │
│                                        │
│ URL 1 → ExtractorRouter                │
│     ↓                                  │
│   Detect: Web                          │
│     ↓                                  │
│   Cache check: MISS                    │
│     ↓                                  │
│   WebExtractor::extract()              │
│     ↓                                  │
│   HTML → clean text                    │
│     ↓                                  │
│   Cache store                          │
│     ↓                                  │
│   Return: ExtractedContent             │
│                                        │
│ URL 2 → ... (PDF)                      │
│ URL 3 → ... (Video - yt-dlp+Whisper)   │
│ ...                                    │
└────────────────────────────────────────┘
    ↓
┌────────────────────────────────────────┐
│ YouTube Search (if enabled)            │
│ • yt-dlp: search + download audio      │
│ • Whisper API: transcribe              │
│ • Cache: 7 days                        │
└────────────────────────────────────────┘
    ↓
┌────────────────────────────────────────┐
│ Parallel LLM Analysis (5 concurrent)   │
│                                        │
│ Source 1:                              │
│   Cache check: MISS                    │
│     ↓                                  │
│   Claude API: analyze content          │
│     ↓                                  │
│   Extract: facts, quotes, confidence   │
│     ↓                                  │
│   Cache store (24h)                    │
│                                        │
│ Source 2: ...                          │
│ Source 3: ...                          │
└────────────────────────────────────────┘
    ↓
┌────────────────────────────────────────┐
│ Synthesis (Claude)                     │
│ • Combine all sources                  │
│ • Generate markdown                    │
│ • Add citations                        │
│ • No cache (always fresh)              │
└────────────────────────────────────────┘
    ↓
Response to User
```

## Performance Characteristics

### Concurrency

| Operation | Concurrency | Why |
|-----------|-------------|-----|
| URL extraction | 10 | Balance between speed and politeness |
| LLM analysis | 5 | Anthropic rate limits |
| YouTube downloads | 3 | Prevent rate limiting |

### Caching Strategy

**Why cache at multiple levels?**

1. **Extraction cache** (by URL)
   - Avoid re-downloading/processing
   - Saves bandwidth
   - Faster response

2. **LLM cache** (by URL + query)
   - Avoid re-analyzing same content
   - Saves API costs (most expensive part)
   - Allows query-specific analysis

**TTL decisions:**

| Content | TTL | Reasoning |
|---------|-----|-----------|
| Web pages | 24h | Content changes frequently |
| PDFs | 24h | Static but may be updated |
| Videos | 7d | Transcripts rarely change |
| Audio | 7d | Transcripts rarely change |
| Images | 24h | May be updated |
| LLM analysis | 24h | Balance freshness vs cost |

### Memory Management

- **Streaming:** Large files streamed to disk
- **Limits:** 
  - Web content: 50k chars
  - PDF content: 100k chars
  - Video transcripts: unlimited (Whisper's limit)
- **Cleanup:** Temp files auto-deleted after processing

## Error Handling

### Strategy

1. **Graceful degradation**
   - If one URL fails, continue with others
   - Return partial results rather than fail entirely

2. **Informative errors**
   - Log failures with context
   - Return error details in API response

3. **Retry logic**
   - No automatic retries (avoid cost amplification)
   - Client can retry if needed

### Example

```
10 URLs to extract:
- 8 succeed → Include in analysis
- 2 fail → Log errors, continue
- Return synthesis from 8 sources + note about failures
```

## Scalability Considerations

### Current (Single Instance)

- Handles ~10 concurrent searches
- Limited by Claude API rate limits
- Suitable for personal/small team use

### Future (Distributed)

Potential improvements:
- **Horizontal scaling:** Multiple Synth instances + load balancer
- **Shared cache:** Redis/PostgreSQL instead of local Tantivy
- **Queue system:** RabbitMQ for async processing
- **Worker pools:** Dedicated extraction/analysis workers

## Security

### Input Validation

- Query length limits
- URL validation (http/https only)
- Shell command sanitization (yt-dlp args)

### Rate Limiting

- Built into concurrency limits
- No per-user rate limiting (single-user assumption)

### Data Privacy

- All data stored locally
- No telemetry
- API keys stored in environment (not config files)

## Next Steps

- Read [Extractors Documentation](../extractors/) for detailed extractor internals
- Read [Caching Documentation](../caching/) for cache implementation details
- Read [API Documentation](../api/) for endpoint specifications
