# OSIT (Open Search It)

**Self-hosted AI-powered research engine for agents.**

## What is OSIT?

OSIT is a **real search engine** with its own crawler and index, not just an API wrapper.

How it works:
1. **Crawler** indexes the web (you control what gets crawled)
2. **Search** queries your local index (fast, private, no API limits)
3. **Scraper** fetches fresh content from top results
4. **LLM** analyzes and synthesizes information
5. **Results** with citations, sources, and confidence scores

## Features

- **Own search engine** — your index, your control, no external dependencies
- **Background crawler** — continuously builds and updates index
- **Fast full-text search** — Tantivy BM25 ranking, millisecond queries
- **Parallel scraping** — analyze 10+ pages simultaneously
- **LLM-powered synthesis** — Claude analyzes and combines information
- **Agent-friendly API** — JSON output, structured data, citations
- **Self-hosted** — completely private, no API costs, no rate limits
- **Respectful crawling** — rate limiting, robots.txt compliance

## Architecture

```
┌──────────────┐
│   Crawler    │ ─► Indexes web pages
│ (Background) │    (respects robots.txt, rate limits)
└──────┬───────┘
       │
       ▼
┌──────────────┐
│Tantivy Index │ ─► Full-text search (BM25)
│  (Local DB)  │    Fast, private, no API calls
└──────┬───────┘
       │
Query ─┘
       │
       ▼
┌──────────────┐
│ Top N URLs   │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│Fresh Scrape  │ ─► Parallel page fetching
│ (10 threads) │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ LLM Analysis │ ─► Claude analyzes each page
│ & Synthesis  │    Extracts facts, quotes, synthesizes
└──────────────┘
```

## Tech Stack

- **Rust + Axum** — fast async web server
- **Tokio** — concurrent crawling and scraping
- **Tantivy** — full-text search engine (Rust, BM25 ranking)
- **Claude (Anthropic)** — content analysis and synthesis
- **Governor** — rate limiting for respectful crawling

## API Usage

```bash
# 1. Start a crawl to build your index
curl -X POST http://localhost:8765/crawl \
  -H "Content-Type: application/json" \
  -d '{
    "max_pages": 1000,
    "seed_urls": [
      "https://doc.rust-lang.org/book/",
      "https://stackoverflow.com/questions/tagged/rust",
      "https://github.com/rust-lang/rust"
    ]
  }'

# 2. Check index stats
curl http://localhost:8765/stats

# 3. Search your index
curl -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{"query": "How does Rust ownership work?", "depth": "quick"}'

# 4. Deep search with more pages
curl -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{"query": "Rust async runtime comparison", "depth": "deep", "max_pages": 15}'
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

## Installation

1. Clone the repo
2. Copy `.env.example` to `.env` and fill in API keys
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
