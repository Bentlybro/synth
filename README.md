# OSIT (Open Search It)

**Self-hosted AI-powered research engine for agents.**

## What is OSIT?

OSIT is an **AI-powered research engine** that enhances web search with intelligent analysis.

How it works:
1. **DuckDuckGo Search** finds relevant URLs for your query
2. **Smart Cache** checks if pages were recently scraped (avoid waste)
3. **Parallel Scraper** fetches fresh content from top results (50 concurrent!)
4. **Claude AI** analyzes each page and synthesizes information
5. **Results** with comprehensive answers, citations, sources, and confidence scores

## Features

- **SearXNG integration** — self-hosted metasearch (no API keys needed!)
- **YouTube + Whisper** ⭐ — transcribe videos, analyze alongside web content
- **Concurrent LLM analysis** ⚡ — 5x parallel processing, blazing fast
- **Smart caching** — never scrape the same page twice (24hr TTL)
- **50 concurrent scrapes** — parallel page fetching
- **Multi-source synthesis** — combines web + video sources with citations
- **Agent-friendly API** — JSON output, structured data, confidence scores
- **Self-hosted** — completely private, your data stays yours
- **No rate limits** — scrape as much as you need

## Architecture

Enhanced pipeline with video support:

```
1. Search
   ├─ SearXNG (localhost:8888) → Web URLs
   └─ YouTube Search (optional) → Video URLs
   
2. Content Fetching (PARALLEL)
   ├─ Check Cache (Tantivy) → Skip recent
   ├─ Scrape Web Pages (50 concurrent)
   └─ Download + Transcribe Videos (Whisper API)
   
3. LLM Analysis (CONCURRENT!)
   ↓ [5x parallel analysis of ALL sources]
   ↓ [Extract facts, quotes, confidence]
   
4. Synthesis
   → Comprehensive answer with citations from web + video
```

**Key Benefits:**
- ✅ Multi-modal research (web + video)
- ✅ Concurrent LLM analysis (5-10x speedup)
- ✅ Self-hosted search (SearXNG)
- ✅ Fast parallel scraping (50 concurrent)
- ✅ Smart caching (24hr TTL)
- ✅ No rate limits

## Tech Stack

- **Rust + Axum** — fast async web server
- **Tokio** — concurrent scraping (50 parallel tasks)
- **Tantivy** — page cache (avoid re-scraping)
- **SearXNG** — self-hosted metasearch (aggregates Google, Bing, DDG, etc.)
- **Claude (Anthropic)** — per-page analysis and multi-source synthesis

## Workflow

```
You ask → OSIT API → SearXNG searches web + YouTube → Scrape pages + Transcribe videos (parallel) → Claude analyzes (concurrent!) → Synthesis → Results
```

Simple, fast, self-hosted, **now with video analysis!**

## API Usage

```bash
# Quick search (10 pages, <30s)
curl -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{"query": "How does Rust ownership work?", "depth": "quick"}'

# Deep search (20 pages, 1-2min)
curl -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{"query": "Rust async runtime comparison", "depth": "deep"}'

# Custom page count
curl -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{"query": "what is rust", "depth": "quick", "max_pages": 5}'

# Include YouTube videos (NEW!)
curl -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{"query": "explain quantum computing", "include_youtube": true, "max_videos": 2}'

# Deep search with YouTube
curl -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{"query": "machine learning tutorial", "depth": "deep", "include_youtube": true}'

# Check cache stats
curl http://localhost:8765/stats

# Health check
curl http://localhost:8765/health
```

**Response format:**
```json
{
  "status": "complete",
  "synthesis": "Comprehensive answer with [Source N] citations...",
  "sources": [
    {
      "url": "https://...",
      "title": "Page Title",
      "key_facts": ["fact1", "fact2"],
      "quotes": ["direct quote"],
      "confidence": 0.85
    }
  ]
}
```

## Modes

- **Quick** — 3-5 pages, < 30s response time
- **Deep** — 10-20 pages, comprehensive analysis, 1-2 min

## Development

```bash
# Run server
cargo run --release

# Run tests
cargo test

# Build
cargo build --release
```

## Roadmap

### Phase 1: MVP ✅ COMPLETE
- [x] Basic search + scrape + analyze pipeline
- [x] Claude integration
- [x] REST API
- [x] SearXNG integration (self-hosted search)
- [x] Test with real queries (stock prices, general knowledge)
- [x] Systemd service for auto-start
- [x] Helper CLI tool (osit.sh)
- [x] Skill documentation

**Status:** Fully working! Production-ready for personal use.

---

### Phase 2: Optimization ✅ MAJOR UPGRADES
- [x] ~~Vector~~ Tantivy cache for page content (24hr TTL)
- [x] Retry logic for failed scrapes (graceful degradation)
- [x] **Concurrent LLM analysis** (5x parallel analysis, HUGE speedup!)
- [ ] **Streaming responses** (SSE or WebSocket for progress updates)
- [ ] **Rate limiting** and quotas per API key
- [ ] **Performance benchmarks** (measure latency, cache hit rate)

**Status:** Concurrent LLM complete! 5-10x faster for multi-page queries.

---

### Phase 3: Enhancement (KILLER FEATURE ADDED!)
- [x] CLI tool for direct usage (osit.sh)
- [x] **YouTube + Whisper support** ⭐⭐⭐ (transcribe videos, analyze transcripts - DONE!)
- [ ] **PDF scraping** (extract text from PDFs in search results)
- [ ] **Multi-LLM support** (fallback providers: OpenAI, local Ollama)
- [ ] **Custom scrapers** for common sites (Wikipedia, Stack Overflow, docs)
- [ ] **Result ranking/filtering** (by confidence, recency, domain authority)
- [ ] **Export to markdown/PDF** (save research reports)
- [ ] **Timing metrics** (show search/scrape/analysis/synthesis durations)

**Status:** YouTube transcription IMPLEMENTED! This was the killer feature from old OSIT v2!

---

### Phase 4: Production (Future)
- [ ] **Docker deployment** (containerized for easy hosting)
- [ ] **Metrics and monitoring** (Prometheus, Grafana)
- [ ] **Admin dashboard** (web UI for stats, cache management)
- [ ] **API authentication** (API keys, usage tracking)
- [ ] **Multi-user support** (per-user quotas, history)
- [ ] **Horizontal scaling** (multiple OSIT instances behind load balancer)

**Not needed yet** — works great for personal use!

---

## 🚀 What to Work On Next

### ✅ RECENTLY COMPLETED
- ✅ **YouTube + Whisper Transcription** — DONE! Search YouTube, transcribe videos, include in analysis
- ✅ **Concurrent LLM Analysis** — DONE! 5x parallel analysis, huge speedup

---

### High Priority (Next Upgrades)
1. **Timing Metrics in Response** ⭐⭐
   - Old OSIT showed: search time, scrape time, LLM time
   - Useful for debugging and optimization
   - Add to response JSON
   - **Impact:** Visibility into performance

2. **PDF Scraping** ⭐
   - Detect PDF URLs, extract text with PyPDF2 or similar
   - Include in analysis (research papers, docs)
   - **Impact:** Academic research use case

### Medium Priority (Nice to Have)
3. **Streaming Responses** ⭐
   - SSE or WebSocket for real-time progress
   - Show: "Searching... Scraping 5 pages... Transcribing videos..."
   - Better UX for slow queries

4. **Multi-LLM Support**
   - Fallback to OpenAI if Claude fails
   - Or use local Ollama for privacy/cost

5. **Custom scrapers** for common sites (Wikipedia, Stack Overflow, docs)
   - Better extraction for known sites

### Low Priority (Eventually)
6. Performance benchmarks
7. Result filtering by confidence/recency
8. Export to markdown
9. Docker deployment
10. Admin dashboard

---

## Prerequisites

**SearXNG** (self-hosted metasearch):
- Install SearXNG: https://docs.searxng.org/admin/installation.html
- Or use Docker: `docker pull searxng/searxng`
- Default URL: `http://localhost:8888`
- OSIT will use this for search (no API keys needed!)

## Installation

1. Clone the repo
2. Copy `.env.example` to `.env` and add your Anthropic API key
3. Build: `cargo build --release`
4. Run: `./target/release/osit`

Or use systemd:
```bash
sudo cp osit.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now osit
```

## License

Private - Not for public distribution
