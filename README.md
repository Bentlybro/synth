# Synth

**Self-hosted AI-powered research engine for agents.**

## What is Synth?

Synth is an **AI-powered research engine** that synthesizes information from multiple sources with intelligent analysis.

How it works:
1. **DuckDuckGo Search** finds relevant URLs for your query
2. **Smart Cache** checks if pages were recently scraped (avoid waste)
3. **Parallel Scraper** fetches fresh content from top results (50 concurrent!)
4. **Claude AI** analyzes each page and synthesizes information
5. **Results** with comprehensive answers, citations, sources, and confidence scores

## Features

- **SearXNG integration** — self-hosted metasearch (no API keys needed!)
- **YouTube + Whisper** — transcribe videos, analyze alongside web content
- **Concurrent LLM analysis** — 5x parallel processing, blazing fast
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
You ask → Synth API → SearXNG searches web + YouTube → Scrape pages + Transcribe videos (parallel) → Claude analyzes (concurrent!) → Synthesis → Results
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

### Phase 1: MVP - Complete
- [x] Basic search + scrape + analyze pipeline
- [x] Claude integration
- [x] REST API
- [x] SearXNG integration (self-hosted search)
- [x] Test with real queries (stock prices, general knowledge)
- [x] Systemd service for auto-start
- [x] Helper CLI tool (synth.sh)
- [x] Skill documentation

**Status:** Fully working. Production-ready for personal use.

---

### Phase 2: Optimization
- [x] ~~Vector~~ Tantivy cache for page content (24hr TTL)
- [x] Retry logic for failed scrapes (graceful degradation)
- [x] **Concurrent LLM analysis** (5x parallel analysis, major speedup)
- [ ] **Streaming responses** (SSE or WebSocket for progress updates)
- [ ] **Rate limiting** and quotas per API key
- [ ] **Performance benchmarks** (measure latency, cache hit rate)

**Status:** Concurrent LLM complete. 5-10x faster for multi-page queries.

---

### Phase 3: Enhancement
- [x] CLI tool for direct usage (synth.sh)
- [x] **YouTube + Whisper support** (transcribe videos, analyze transcripts)
- [ ] **PDF scraping** (extract text from PDFs in search results)
- [ ] **Multi-LLM support** (fallback providers: OpenAI, local Ollama)
- [ ] **Custom scrapers** for common sites (Wikipedia, Stack Overflow, docs)
- [ ] **Result ranking/filtering** (by confidence, recency, domain authority)
- [ ] **Export to markdown/PDF** (save research reports)
- [ ] **Timing metrics** (show search/scrape/analysis/synthesis durations)

**Status:** YouTube transcription complete. PDF scraping and timing metrics next.

---

### Phase 4: Production (Future)
- [ ] **Docker deployment** (containerized for easy hosting)
- [ ] **Metrics and monitoring** (Prometheus, Grafana)
- [ ] **Admin dashboard** (web UI for stats, cache management)
- [ ] **API authentication** (API keys, usage tracking)
- [ ] **Multi-user support** (per-user quotas, history)
- [ ] **Horizontal scaling** (multiple Synth instances behind load balancer)

**Not needed yet** — works great for personal use!

---

## Prerequisites

**SearXNG** (self-hosted metasearch):
- Install SearXNG: https://docs.searxng.org/admin/installation.html
- Or use Docker: `docker pull searxng/searxng`
- Default URL: `http://localhost:8888`
- Synth will use this for search (no API keys needed!)

## Installation

1. Clone the repo
2. Copy `.env.example` to `.env` and add your Anthropic API key
3. Build: `cargo build --release`
4. Run: `./target/release/synth`

Or use systemd:
```bash
sudo cp synth.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now synth
```

## License

Private - Not for public distribution
