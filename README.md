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
- **Smart caching** — never scrape the same page twice (24hr TTL)
- **Blazing fast scraping** — 50 concurrent page fetches
- **LLM-powered analysis** — Claude analyzes each page individually
- **Multi-source synthesis** — combines information from many sources
- **Agent-friendly API** — JSON output, structured data, citations
- **Self-hosted** — completely private, your data stays yours
- **No rate limits** — scrape as much as you need

## Architecture

Simple 5-step pipeline:

```
1. SearXNG Search (localhost:8888)
   ↓ [Find URLs from Google, Bing, DDG, etc.]
   
2. Smart Cache Check (Tantivy)
   ↓ [Skip recently scraped pages]
   
3. Parallel Scraper (50 concurrent)
   ↓ [Fetch page content]
   
4. Claude Analysis (per-page)
   ↓ [Extract facts, quotes, confidence]
   
5. Synthesis (multi-source)
   → Comprehensive answer with citations
```

**Key Benefits:**
- ✅ No external API dependencies (except Claude for LLM)
- ✅ Self-hosted search (SearXNG)
- ✅ Fast parallel scraping
- ✅ Smart caching (24hr TTL)
- ✅ Multi-source synthesis with citations

## Tech Stack

- **Rust + Axum** — fast async web server
- **Tokio** — concurrent scraping (50 parallel tasks)
- **Tantivy** — page cache (avoid re-scraping)
- **SearXNG** — self-hosted metasearch (aggregates Google, Bing, DDG, etc.)
- **Claude (Anthropic)** — per-page analysis and multi-source synthesis

## Workflow

```
You ask → OSIT API → SearXNG searches web → Scrape pages → Claude analyzes → Synthesis → Results
```

Simple, fast, self-hosted.

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

### Phase 2: Optimization (PARTIAL)
- [x] ~~Vector~~ Tantivy cache for page content (24hr TTL)
- [x] Retry logic for failed scrapes (graceful degradation)
- [ ] **Streaming responses** (SSE or WebSocket for progress updates)
- [ ] **Concurrent LLM calls** for analysis (currently sequential)
- [ ] **Rate limiting** and quotas per API key
- [ ] **Performance benchmarks** (measure latency, cache hit rate)

**Next Priority:** Concurrent LLM analysis (speed up multi-page queries)

---

### Phase 3: Enhancement
- [x] CLI tool for direct usage (osit.sh)
- [ ] **YouTube + Whisper support** (transcribe videos, analyze transcripts)
- [ ] **PDF scraping** (extract text from PDFs in search results)
- [ ] **Multi-LLM support** (fallback providers: OpenAI, local Ollama)
- [ ] **Custom scrapers** for common sites (Wikipedia, Stack Overflow, docs)
- [ ] **Result ranking/filtering** (by confidence, recency, domain authority)
- [ ] **Export to markdown/PDF** (save research reports)
- [ ] **Timing metrics** (show search/scrape/analysis/synthesis durations)

**Wishlist from old OSIT:** YouTube transcription was killer feature!

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

Based on old OSIT's best features and current gaps:

### High Priority (Would Make OSIT Way Better)
1. **YouTube + Whisper Transcription** ⭐⭐⭐
   - Old OSIT v2 had this, it was AMAZING
   - Search YouTube for query, download audio, transcribe with Whisper
   - Include transcripts in synthesis
   - Use case: "explain quantum computing" → finds videos + analyzes transcripts

2. **Concurrent LLM Analysis** ⭐⭐
   - Currently: analyze pages sequentially (slow)
   - Fix: analyze all pages in parallel with Tokio
   - Speed improvement: 5-10x faster for deep searches

3. **Timing Metrics in Response** ⭐⭐
   - Old OSIT showed: search time, scrape time, LLM time
   - Useful for debugging and optimization
   - Add to response JSON

### Medium Priority (Nice to Have)
4. **PDF Scraping** ⭐
   - Detect PDF URLs, extract text with PyPDF2 or similar
   - Include in analysis (research papers, docs)

5. **Streaming Responses** ⭐
   - SSE or WebSocket for real-time progress
   - Show: "Searching... Scraping 5 pages... Analyzing..."
   - Better UX for slow queries

6. **Multi-LLM Support**
   - Fallback to OpenAI if Claude fails
   - Or use local Ollama for privacy/cost

### Low Priority (Eventually)
7. Performance benchmarks
8. Result filtering by confidence/recency
9. Export to markdown
10. Docker deployment

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
