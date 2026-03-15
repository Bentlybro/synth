<p align="center">
  <strong>Synth</strong>
  <br>
  <strong>Self-hosted AI research engine with multi-modal content extraction</strong>
  <br>
  Extract, analyze, and synthesize information from any content type on the web
</p>

<p align="center">
  <a href="https://github.com/Bentlybro/synth/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License"></a>
  <a href="https://github.com/Bentlybro/synth"><img src="https://img.shields.io/badge/rust-1.88+-orange.svg" alt="Rust Version"></a>
  <a href="https://github.com/Bentlybro/synth/issues"><img src="https://img.shields.io/github/issues/Bentlybro/synth" alt="Issues"></a>
</p>

---

## What is Synth?

**Synth** is a self-hosted AI research engine that goes beyond simple web search. It automatically extracts content from **any URL** (web pages, PDFs, videos, audio, images), analyzes it with Claude AI, and synthesizes comprehensive answers with citations.

Think of it as your personal research assistant that:
- Searches the web via SearXNG (aggregates multiple search engines)
- Extracts text from PDFs, transcribes videos/audio, analyzes images
- Analyzes each source with Claude for key facts and insights
- Synthesizes everything into a comprehensive markdown answer
- Caches aggressively (95% cost savings on repeat queries)

### Key Features

- **Semantic Search**: Finds similar cached queries using AI embeddings (0.7 similarity threshold)
- **Query Expansion**: Automatically expands queries for maximum coverage (Deep mode)
- **Smart Ranking**: Relevance scoring prioritizes best results first
- **Code Repository Analysis**: Analyzes entire GitHub repos with commit-aware caching
- **Universal Content Extraction**: Web, PDF, video, audio, image, code - ALL formats
- **Multi-Modal Analysis**: Automatic content type detection and intelligent routing
- **AI-Powered Synthesis**: Claude analyzes sources and generates comprehensive answers
- **Intelligent Caching**: 2-layer cache (extraction + LLM) with semantic matching
- **Self-Hosted**: No external dependencies except Claude/Whisper/OpenAI APIs
- **Privacy-First**: All data stays on your server
- **RESTful API**: Simple HTTP endpoints for integration

---

## How It Works

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. Search Query                                                 │
│    "What's new in Rust async runtime performance?"              │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ 2. Semantic Cache Check                                         │
│    → Check for similar queries using AI embeddings              │
│    → 0.7+ similarity = instant cache hit!                       │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ 3. Query Expansion (Deep mode)                                  │
│    → "rust async performance" → 5 related queries:              │
│      • "rust async explained"                                   │
│      • "rust async tutorial"                                    │
│      • "latest rust async"                                      │
│      • "rust async best practices"                              │
│    → Search all variants in parallel!                           │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ 4. Smart Ranking                                                │
│    → Score each result by relevance                             │
│    → Exact matches, recent content, official docs prioritized   │
│    → Best results extracted first                               │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ 5. Universal Content Extraction (15 concurrent!)                │
│    ┌──────────┬──────────┬──────────┬──────────┬──────────┐     │
│    │   Web    │   PDF    │  Video   │  Audio   │  Image   │     │
│    │ Scraper  │ Extract  │ yt-dlp + │ Whisper  │  Claude  │     │
│    │          │          │ Whisper  │          │  Vision  │     │
│    └──────────┴──────────┴──────────┴──────────┴──────────┘     │
│    + Code Repos (GitHub) with deep mode (100 files!)            │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ 6. Store Embeddings (async, non-blocking)                       │
│    → Generate AI embeddings for future semantic matching        │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ 7. Claude Analysis (5 concurrent, cached)                       │
│    Each source → Extract key facts, quotes, confidence score    │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ 8. Synthesis                                                    │
│    Claude combines all sources into comprehensive markdown      │
│    with citations, organized sections, and confidence levels    │
└─────────────────────────────────────────────────────────────────┘
```

---

## Performance

### Speed

| Operation | First Request | Cached Request | Speedup |
|-----------|---------------|----------------|---------|
| **Web Page** | 2-5 seconds | <100ms | ~20-50x |
| **PDF** | 3-10 seconds | <100ms | ~30-100x |
| **Video** | 30-90 seconds | <100ms | ~300-900x |
| **Audio** | 10-30 seconds | <100ms | ~100-300x |
| **Image** | 5-10 seconds | <100ms | ~50-100x |

### Cost Savings

**Without caching:**
- 100 identical queries = 100 × full extraction cost
- Example: 10-min YouTube video = $0.06 × 100 = $6.00

**With caching:**
- 100 identical queries = 1 × extraction + 99 × $0
- Example: Same 100 queries = $0.06 total
- **Savings: 99% ($5.94)**

### Concurrency & Intelligence

- **Semantic search**: Instant cache hits for similar queries (0.7+ similarity)
- **Query expansion**: Up to 5 related queries searched in parallel (Deep mode)
- **Smart ranking**: Relevance scoring prioritizes best sources
- **Web extraction**: 15 URLs in parallel (increased from 10!)
- **LLM analysis**: 5 sources analyzed concurrently
- **YouTube downloads**: 3 concurrent (prevents rate limiting)
- **Embedding generation**: Async/non-blocking (doesn't slow search)

---

## Example Use Cases

### Research Question (Semantic + Expansion)

**Query:** "How does Rust async runtime work?"

**What Synth does:**
1. Checks semantic cache → finds "tokio async overview" (0.87 similarity)
2. Expands to: ["rust async explained", "rust async tutorial", "latest rust async"]
3. Searches 3 queries in parallel
4. Ranks by relevance (Tokio docs, official guides first)
5. Extracts 15 URLs concurrently
6. Stores embeddings for future semantic matching
7. Synthesizes comprehensive answer with citations

**Result:** Multi-source answer covering runtime internals, async/await, Tokio vs async-std, latest performance improvements

### Code Repository Analysis

**Query:** `https://github.com/tokio-rs/tokio?deep`

**What Synth does:**
1. Clones repo (shallow, commit-aware)
2. Detects language: Rust
3. Deep mode: analyzes 100 files (vs 20 basic)
4. Generates 4-level directory tree
5. Extracts: README, Cargo.toml, main files, core modules
6. Claude analyzes architecture
7. Caches by commit hash (same commit = instant)

**Result:** Complete understanding of Tokio's architecture, runtime design, and key components

### Multi-Modal Research

**Query:** "Latest GPU architecture improvements 2026"

**What Synth finds:**
- Web pages: NVIDIA/AMD announcements
- PDFs: Research papers from arXiv
- Videos: Tech conference talks (transcribed)
- Images: Architecture diagrams (Claude Vision)
- Code: GitHub repos with benchmarks

**Result:** Comprehensive synthesis across ALL content types with proper citations

---

## Quick Start

### One-Script Install (Recommended)

**Easiest way to get started:**

```bash
curl -fsSL https://raw.githubusercontent.com/Bentlybro/synth/main/install.sh | bash
```

**What it does:**
- Installs Rust (if needed)
- Installs yt-dlp (for video support)
- Clones repository
- Builds Synth
- Sets up systemd service (auto-start)
- Prompts for API keys
- Tests installation

**Post-install:**
```bash
# Check status
systemctl --user status synth

# Test
curl -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{"query": "rust programming", "max_pages": 2}'
```

**With OpenClaw:**

Just say: *"Install Synth from https://github.com/Bentlybro/synth"*

OpenClaw will run the installer automatically!

---

### Manual Installation

**Prerequisites:**
- Rust 1.88+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- SearXNG instance (local or remote)
- Anthropic API key (Claude)
- OpenAI API key (optional, for video/audio transcription)
- `yt-dlp` installed (`pip install yt-dlp` - for video support)

```bash
# Clone repository
git clone https://github.com/Bentlybro/synth.git
cd synth

# Create .env file
cat > .env << EOF
ANTHROPIC_API_KEY=sk-ant-...
OPENAI_API_KEY=sk-...        # Optional (for video/audio)
SEARXNG_URL=http://localhost:8888
EOF

# Build release binary
cargo build --release

# Run
./target/release/synth
```

Service starts on `http://localhost:8765`

### Docker

```bash
# Start SearXNG + Synth
docker-compose up -d

# Check status
docker-compose ps

# View logs
docker-compose logs -f synth
```

See [`docker-compose.yml`](docker-compose.yml) for configuration.

---

## API Reference

### 1. Search (Multi-Modal)

Search the web and synthesize comprehensive answers.

**Endpoint:** `POST /search`

**Request:**
```json
{
  "query": "rust async runtime performance",
  "max_pages": 10,
  "depth": "deep",
  "include_youtube": true,
  "max_videos": 2
}
```

**Response:**
```json
{
  "status": "complete",
  "synthesis": "# Rust Async Runtime Performance\n\n...",
  "sources": [
    {
      "url": "https://example.com",
      "title": "Article Title",
      "key_facts": ["Fact 1", "Fact 2"],
      "quotes": ["Quote 1", "Quote 2"],
      "confidence": 0.95
    }
  ]
}
```

### 2. Extract (Direct URL)

Extract and analyze content from a specific URL.

**Endpoint:** `POST /extract`

**Request:**
```json
{
  "url": "https://arxiv.org/pdf/paper.pdf",
  "query": "Summarize the key findings"
}
```

**Response:**
```json
{
  "url": "https://arxiv.org/pdf/paper.pdf",
  "title": "Paper Title",
  "content_type": "PDF",
  "content": "Extracted text...",
  "analysis": {
    "key_facts": ["Finding 1", "Finding 2"],
    "quotes": ["Quote 1"],
    "confidence": 0.9
  },
  "metadata": {
    "file_size_bytes": 1024000
  }
}
```

### 3. Health Check

**Endpoint:** `GET /health`

**Response:** `OK` (200)

### 4. Stats

**Endpoint:** `GET /stats`

**Response:**
```json
{
  "cached_pages": 127
}
```

See [API Documentation](docs/api/) for full reference.

---

## Supported Content Types

| Type | Detection | Extraction | Analysis |
|------|-----------|------------|----------|
| **Web Pages** | HTTP/HTTPS URLs | HTML → main content | Claude text analysis |
| **PDFs** | `.pdf` extension | `pdf-extract` → text | Claude text analysis |
| **Videos** | YouTube, Vimeo, TikTok, etc. | `yt-dlp` + Whisper → transcript | Claude transcript analysis |
| **Audio** | `.mp3`, `.wav`, `.m4a`, etc. | Whisper → transcript | Claude transcript analysis |
| **Images** | `.jpg`, `.png`, `.gif`, etc. | Download | Claude Vision analysis |

### Automatic Routing

Synth automatically detects content type and routes to the appropriate extractor:

```rust
URL → ExtractorRouter
        ↓
    Detect Type
        ↓
┌───────┴───────┐
│               │
PDF?          Video?
│               │
pdf-extract   yt-dlp
│               │
└───────┬───────┘
        ↓
    Extract Content
        ↓
    Cache + Analyze
```

See [Extractors Documentation](docs/extractors/) for details.

---

## Architecture

### Core Components

```
synth/
├── src/
│   ├── api/          # HTTP API (Axum)
│   ├── cache/        # CacheManager (Tantivy-based)
│   ├── extractors/   # Universal content extraction
│   │   ├── web.rs    # HTML scraping
│   │   ├── pdf.rs    # PDF text extraction
│   │   ├── video.rs  # Video transcription
│   │   ├── audio.rs  # Audio transcription
│   │   └── image.rs  # Image analysis
│   ├── llm/          # Claude integration
│   ├── search/       # SearXNG client
│   └── shared/       # Common utilities
├── docs/             # Comprehensive documentation
└── docker-compose.yml
```

### Data Flow

1. **Request** → API endpoint (`/search` or `/extract`)
2. **Search** → SearXNG returns URLs
3. **Extract** → Parallel extraction (10 concurrent)
   - Check cache first (cache key = `hash(url)`)
   - If miss: extract content → store in cache
   - If hit: return cached content instantly
4. **Analyze** → Claude analyzes each source (5 concurrent)
   - Check cache first (cache key = `hash(url + query)`)
   - Extract key facts, quotes, confidence
5. **Synthesize** → Claude combines all sources
6. **Return** → Markdown with citations

See [Architecture Documentation](docs/architecture/) for deep dive.

---

## Configuration

### Environment Variables

```bash
# Required
ANTHROPIC_API_KEY=sk-ant-...   # Claude API key

# Optional
OPENAI_API_KEY=sk-...           # Whisper API (video/audio)
SEARXNG_URL=http://localhost:8888  # SearXNG instance
CACHE_TTL_SECONDS=86400        # Legacy cache TTL (24h)
```

### Cache Configuration

Cache TTLs by content type:

| Category | TTL | Location |
|----------|-----|----------|
| Web pages | 24 hours | `index/cache/extractors_web/` |
| PDFs | 24 hours | `index/cache/extractors_pdf/` |
| Videos | 7 days | `index/cache/extractors_video/` |
| Audio | 7 days | `index/cache/extractors_audio/` |
| Images | 24 hours | `index/cache/extractors_image/` |
| LLM analyses | 24 hours | `index/cache/llm/` |

Auto-cleanup runs on startup (removes expired files).

See [Cache Documentation](docs/caching/) for details.

---

## Use Cases

### 1. Research Assistant

```bash
curl -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "latest advances in quantum computing",
    "max_pages": 15,
    "depth": "deep",
    "include_youtube": true
  }'
```

Returns comprehensive synthesis from:
- Academic papers (PDFs)
- Blog posts and articles
- YouTube videos
- Technical documentation

### 2. PDF Analysis

```bash
curl -X POST http://localhost:8765/extract \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://arxiv.org/pdf/1706.03762.pdf",
    "query": "Summarize the Transformer architecture"
  }'
```

### 3. Video Summaries

```bash
curl -X POST http://localhost:8765/extract \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://youtube.com/watch?v=dQw4w9WgXcQ",
    "query": "What are the main topics covered?"
  }'
```

### 4. Technical Documentation

```bash
curl -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "rust ownership borrowing explained",
    "max_pages": 8
  }'
```

Returns synthesis from official docs, tutorials, Stack Overflow, Reddit discussions.

---

## Development

### Prerequisites

- Rust 1.88+ with `cargo`
- SearXNG instance (Docker recommended)
- API keys (Anthropic, optionally OpenAI)

### Setup

```bash
# Clone
git clone https://github.com/Bentlybro/synth.git
cd synth

# Install dependencies
cargo build

# Run in dev mode
cargo run

# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy
```

### Project Structure

```
synth/
├── src/
│   ├── main.rs           # Entry point
│   ├── api/mod.rs        # API routes
│   ├── extractors/       # Content extractors
│   │   ├── mod.rs        # Router + traits
│   │   ├── web.rs        # Web scraper
│   │   ├── pdf.rs        # PDF extractor
│   │   ├── video.rs      # Video transcriber
│   │   ├── audio.rs      # Audio transcriber
│   │   └── image.rs      # Image analyzer
│   ├── cache/            # Caching system
│   │   ├── mod.rs        # Legacy PageCache
│   │   └── manager.rs    # CacheManager
│   ├── llm/mod.rs        # Claude integration
│   ├── search/           # SearXNG client
│   ├── shared/           # Utilities
│   └── models/mod.rs     # Data structures
├── docs/                 # Documentation
├── examples/             # Usage examples
├── Dockerfile
├── docker-compose.yml
└── README.md
```

See [Development Guide](docs/development/) for contributing.

---

## Deployment

### Systemd Service

```bash
# Copy service file
sudo cp synth.service ~/.config/systemd/user/

# Edit paths and environment
nano ~/.config/systemd/user/synth.service

# Enable and start
systemctl --user enable synth
systemctl --user start synth

# Check status
systemctl --user status synth
```

### Docker Compose (Production)

```yaml
version: '3.8'
services:
  searxng:
    image: searxng/searxng:latest
    ports:
      - "8888:8080"
    volumes:
      - ./searxng:/etc/searxng

  synth:
    build: .
    ports:
      - "8765:8765"
    environment:
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
      - OPENAI_API_KEY=${OPENAI_API_KEY}
      - SEARXNG_URL=http://searxng:8080
    depends_on:
      - searxng
    volumes:
      - ./index:/app/index
```

See [Deployment Guide](docs/deployment/) for production setup.

---

## Troubleshooting

### Service won't start

```bash
# Check logs
journalctl --user -u synth --no-pager | tail -50

# Verify port availability
lsof -i :8765

# Test configuration
./target/release/synth --help
```

### Extraction fails

**PDF errors:**
- Verify URL is accessible
- Check if PDF is encrypted
- Ensure sufficient disk space

**Video errors:**
- Install `yt-dlp`: `pip install yt-dlp`
- Set `OPENAI_API_KEY` for transcription
- Check supported platforms

**Slow performance:**
- Normal on first extraction (downloads + processes)
- Check cache directory exists
- Verify cache isn't full/corrupted

See [Troubleshooting Guide](docs/troubleshooting.md) for more.

---

## Roadmap

### Current (v1.0)
- Multi-modal content extraction
- Claude analysis and synthesis
- Comprehensive caching
- Docker support
- RESTful API

### Planned
- [ ] Word documents (.docx)
- [ ] Spreadsheets (.xlsx, .csv)
- [ ] Presentations (.pptx)
- [ ] Code repositories (GitHub analysis)
- [ ] RSS/Atom feeds
- [ ] Email parsing (.eml)
- [ ] Semantic search (vector embeddings)
- [ ] Multi-language support
- [ ] Streaming results
- [ ] Web UI

See [GitHub Issues](https://github.com/Bentlybro/synth/issues) for tracking.

---

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Quick Start

```bash
# Fork and clone
git clone https://github.com/YOUR_USERNAME/synth.git

# Create feature branch
git checkout -b feature/amazing-feature

# Make changes and test
cargo test
cargo clippy

# Commit and push
git commit -m "Add amazing feature"
git push origin feature/amazing-feature

# Open PR
```

### Development Priorities

- New content extractors
- Performance improvements
- Test coverage
- Documentation
- Bug fixes

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## Acknowledgments

- [SearXNG](https://github.com/searxng/searxng) - Privacy-respecting metasearch engine
- [Anthropic Claude](https://anthropic.com) - AI analysis and synthesis
- [OpenAI Whisper](https://openai.com/research/whisper) - Audio transcription
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) - Universal video downloader
- [Tantivy](https://github.com/quickwit-oss/tantivy) - Full-text search engine
- [Axum](https://github.com/tokio-rs/axum) - Web framework

---

## Support

- **Documentation**: [docs/](docs/)
- **Issues**: [GitHub Issues](https://github.com/Bentlybro/synth/issues)
- **Discussions**: [GitHub Discussions](https://github.com/Bentlybro/synth/discussions)

---

<p align="center">
  Built with Rust by <a href="https://github.com/Bentlybro">Bently</a>
</p>
