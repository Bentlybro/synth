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

## License

Private - Not for public distribution
