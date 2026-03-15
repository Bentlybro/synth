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

### Phase 1: MVP (Current)
- [x] Basic search + scrape + analyze pipeline
- [x] Claude integration
- [x] REST API
- [ ] Test with real queries
- [ ] Performance benchmarks

### Phase 2: Optimization
- [ ] Vector cache for page content
- [ ] Streaming responses (SSE or WebSocket)
- [ ] Concurrent LLM calls for analysis
- [ ] Rate limiting and quotas
- [ ] Retry logic for failed scrapes

### Phase 3: Enhancement
- [ ] CLI tool for direct usage
- [ ] Multi-LLM support (fallback providers)
- [ ] Custom scrapers for common sites (Wikipedia, Stack Overflow, docs sites)
- [ ] Result ranking/filtering
- [ ] Export to markdown/PDF

### Phase 4: Production
- [ ] Docker deployment
- [ ] Metrics and monitoring
- [ ] Admin dashboard
- [ ] API authentication
- [ ] Multi-user support

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
