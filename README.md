# Synth

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)

**Self-hosted AI research engine that synthesizes information from web + video sources.**

## What is Synth?

Synth combines web search, parallel scraping, and AI analysis to give you comprehensive answers with citations. Unlike simple search engines, Synth:

- **Searches multiple sources** via SearXNG (aggregates Google, Bing, DuckDuckGo, etc.)
- **Scrapes in parallel** (50 concurrent requests — blazing fast!)
- **Transcribes YouTube videos** with OpenAI Whisper (optional)
- **Analyzes with Claude AI** (concurrent analysis of all sources)
- **Synthesizes multi-source answers** with direct quotes and citations

**Perfect for:** Research, fact-checking, technical questions, current events, tutorials

---

## Features

- **Self-hosted** — Your data stays private, no external dependencies
- **Multi-source synthesis** — Combines web pages + YouTube transcripts
- **Concurrent everything** — Parallel scraping (50x) + parallel LLM analysis (5x)
- **Smart caching** — Never scrape the same page twice (24hr TTL via Tantivy)
- **Agent-friendly API** — Clean JSON endpoints for AI assistants
- **No rate limits** — Run as many queries as you need
- **YouTube support** — Transcribe and analyze video content (optional)

---

## Quick Start

### Option 1: Docker Compose (Recommended)

The easiest way to get started — includes SearXNG + Synth in one command:

```bash
# Clone the repo
git clone https://github.com/Bentlybro/synth.git
cd synth

# Copy environment file and add your API keys
cp .env.example .env
# Edit .env and add:
#   ANTHROPIC_API_KEY=sk-ant-xxx  (required)
#   OPENAI_API_KEY=sk-proj-xxx    (optional, for YouTube)

# Start everything
docker-compose up -d

# Synth is now running at http://localhost:8765
# SearXNG is at http://localhost:8888
```

### Option 2: Manual Setup

**Prerequisites:**
- Rust 1.75+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- [SearXNG](https://docs.searxng.org/admin/installation.html) running on `localhost:8888`
- (Optional) `yt-dlp` for YouTube transcription: `pip install yt-dlp`

**Installation:**

```bash
# Clone and build
git clone https://github.com/Bentlybro/synth.git
cd synth
cargo build --release

# Configure
cp .env.example .env
# Edit .env with your API keys

# Run
./target/release/synth
```

**Systemd service (auto-start on boot):**

```bash
# Copy service file
cp synth.service ~/.config/systemd/user/

# Edit paths in synth.service to match your setup

# Enable and start
systemctl --user enable synth
systemctl --user start synth
```

---

## Usage

### Basic Query

```bash
curl -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{"query": "how does Rust async work?"}'
```

**Response:**
```json
{
  "query": "how does Rust async work?",
  "status": "complete",
  "synthesis": "# How Rust Async Works\n\nRust's async system is based on...",
  "sources": [
    {
      "url": "https://example.com/rust-async",
      "title": "Understanding Rust Async",
      "key_facts": ["Async is zero-cost", "Uses futures..."],
      "quotes": ["async fn returns a Future"],
      "confidence": 0.95
    }
  ]
}
```

### Query Modes

**Quick search** (10 pages, ~30 seconds):
```bash
curl -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{"query": "your question", "depth": "quick"}'
```

**Deep search** (20 pages, 1-2 minutes):
```bash
curl -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{"query": "your question", "depth": "deep"}'
```

**Custom page count:**
```bash
curl -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{"query": "your question", "max_pages": 15}'
```

### YouTube Transcription

**Note:** Requires `OPENAI_API_KEY` in `.env` and `yt-dlp` installed.

```bash
# Include YouTube videos in research
curl -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{"query": "explain quantum computing", "include_youtube": true, "max_videos": 2}'
```

**Perfect for:** Tutorials, how-to guides, educational content, explanations

### Real-World Examples

**Stock prices:**
```bash
curl -s -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{"query": "current Tesla stock price"}' | jq -r '.synthesis'
```

**Technical research with video:**
```bash
curl -s -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{"query": "Rust ownership explained", "include_youtube": true, "max_videos": 1}' | jq -r '.synthesis'
```

**Current events:**
```bash
curl -s -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{"query": "latest SpaceX launch news", "depth": "deep"}' | jq -r '.synthesis'
```

---

## API Reference

### Endpoints

**POST /search** — Main search endpoint
```json
{
  "query": "your search query",
  "depth": "quick" | "deep",        // optional, default: "quick"
  "max_pages": 15,                  // optional, overrides depth
  "include_youtube": true,          // optional, default: false
  "max_videos": 2                   // optional, default: 2
}
```

**GET /health** — Health check (returns "OK")

**GET /stats** — Cache statistics
```json
{
  "cached_pages": 1234,
  "cache_size_mb": 45.6
}
```

---

## Architecture

```
┌─────────────┐
│   Request   │
└──────┬──────┘
       │
       ▼
┌─────────────────────────────────┐
│  1. Search (SearXNG + YouTube)  │
│     • Web: Multi-engine search  │
│     • Video: YouTube query      │
└──────┬──────────────────────────┘
       │
       ▼
┌─────────────────────────────────┐
│  2. Content Fetching (Parallel) │
│     • Check cache (Tantivy)     │
│     • Scrape 50 pages at once   │
│     • Download + transcribe vids│
└──────┬──────────────────────────┘
       │
       ▼
┌─────────────────────────────────┐
│  3. Analysis (Concurrent!)      │
│     • Claude analyzes 5 sources │
│       simultaneously            │
│     • Extract facts, quotes     │
└──────┬──────────────────────────┘
       │
       ▼
┌─────────────────────────────────┐
│  4. Synthesis                   │
│     • Combine all analyses      │
│     • Add citations             │
│     • Return comprehensive      │
│       answer                    │
└─────────────────────────────────┘
```

**Key Technologies:**
- **Rust + Axum** — Fast async web server
- **Tokio** — Concurrent scraping (50 parallel tasks)
- **Tantivy** — Page cache (24hr TTL)
- **SearXNG** — Self-hosted metasearch (no API keys!)
- **Claude (Anthropic)** — AI analysis and synthesis
- **OpenAI Whisper** — Video transcription (optional)

---

## Configuration

All configuration via `.env` file:

```bash
# Required: Anthropic API key for Claude
ANTHROPIC_API_KEY=sk-ant-your_key_here

# Optional: OpenAI key for YouTube transcription
OPENAI_API_KEY=sk-proj-your_key_here

# Optional: Server configuration
PORT=8765
INDEX_PATH=./index
SEARXNG_URL=http://localhost:8888
```

**Getting API Keys:**
- **Anthropic (Claude):** https://console.anthropic.com/
- **OpenAI (Whisper):** https://platform.openai.com/api-keys

---

## Roadmap

### Completed
- [x] Multi-source web search via SearXNG
- [x] 50 concurrent parallel scraping
- [x] Claude AI analysis with robust JSON parsing
- [x] Smart caching (Tantivy, 24hr TTL)
- [x] YouTube + Whisper transcription
- [x] Concurrent LLM analysis (5x speedup)
- [x] Systemd service + auto-restart
- [x] Docker Compose deployment

### In Progress
- [ ] Timing metrics in API responses
- [ ] PDF scraping support
- [ ] Streaming responses (SSE/WebSocket)

### Future
- [ ] Multi-LLM support (OpenAI, local Ollama fallback)
- [ ] Custom scrapers for Wikipedia, Stack Overflow, docs
- [ ] Result filtering by confidence/recency
- [ ] Export to markdown/PDF
- [ ] Admin dashboard
- [ ] Horizontal scaling

---

## Performance

**Query times** (on decent hardware):
- **Quick search (10 pages):** ~20-30 seconds
- **Deep search (20 pages):** ~40-60 seconds
- **With YouTube (2 videos):** +30-60 seconds (transcription time)

**Concurrent analysis:** 5x faster than sequential (analyzes 5 sources simultaneously)

**Cache hit rate:** ~60-80% for repeated queries (24hr TTL)

---

## Troubleshooting

**SearXNG returns 0 results:**
- Check SearXNG is running: `curl http://localhost:8888`
- Verify SEARXNG_URL in .env is correct
- Check SearXNG logs for errors

**YouTube transcription fails:**
- Ensure yt-dlp is installed: `yt-dlp --version`
- Check OPENAI_API_KEY is set in .env
- Verify video isn't region-locked or private

**Slow queries:**
- Reduce max_pages: `{"max_pages": 5}`
- Disable YouTube: `{"include_youtube": false}`
- Check internet connection speed

**Out of memory:**
- Reduce concurrent scraping limit in code
- Close browser tabs / other memory-heavy apps
- Increase swap space

---

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

**Priority areas:**
- PDF scraping support
- Streaming responses
- Performance optimizations
- Multi-LLM support
- Better error handling
- Tests and benchmarks

---

## License

MIT License — see [LICENSE](LICENSE) for details.

---

## Acknowledgments

- Built with [Rust](https://www.rust-lang.org/) and [Axum](https://github.com/tokio-rs/axum)
- Powered by [Claude AI](https://www.anthropic.com/) (Anthropic)
- Search via [SearXNG](https://searxng.org/)
- Transcription via [OpenAI Whisper](https://platform.openai.com/docs/guides/speech-to-text)

---

## Support

- **Issues:** https://github.com/Bentlybro/synth/issues
- **Discussions:** https://github.com/Bentlybro/synth/discussions
- **Reagent Systems:** https://github.com/reagent-systems

---

Made with ❤️ by [Bently](https://github.com/Bentlybro)
