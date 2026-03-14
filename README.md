# OSIT (Open Search It)

**Self-hosted AI-powered research engine for agents.**

## What is OSIT?

Instead of manually browsing through search results, OSIT does the heavy lifting:
1. Takes your query
2. Searches the web
3. Scrapes and analyzes multiple pages in parallel
4. Synthesizes all relevant information into a coherent answer
5. Provides citations and sources

## Features

- **Fast parallel scraping** — analyze 10+ pages simultaneously
- **LLM-powered synthesis** — Claude analyzes and combines information
- **Vector caching** — avoid re-fetching the same content
- **Streaming responses** — see progress in real-time
- **Agent-friendly API** — JSON output, structured data, citations
- **Self-hosted** — no API rate limits, full control

## Architecture

```
Query → Search API → Parallel Scraping → LLM Analysis → Synthesis
                                              ↓
                                        Vector Cache
```

## Tech Stack

- **Rust + Axum** — fast async web server
- **Tokio** — concurrent scraping
- **Brave Search API** — web search backend
- **Claude (Anthropic)** — content analysis and synthesis
- **Vector DB** — content caching (TBD: qdrant or chroma)

## API Usage

```bash
# Quick search
curl -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{"query": "How does Rust ownership work?", "depth": "quick"}'

# Deep search
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
