# Synth Documentation

Complete documentation for the Synth AI research engine.

## Quick Links

- **[Architecture](architecture/)** - System design and data flow
- **[Extractors](extractors/)** - Content extraction internals
- **[Caching](caching/)** - Multi-layer caching system
- **[API](api/)** - HTTP endpoint reference
- **[Deployment](deployment/)** - Production setup
- **[Development](development/)** - Contributing guide

---

## What is Synth?

Synth is a self-hosted AI research engine that extracts content from **any URL** (web pages, PDFs, videos, audio, images), analyzes it with Claude AI, and synthesizes comprehensive answers with citations.

---

## Documentation Structure

### 📐 [Architecture](architecture/)

**Read this first** to understand how Synth works.

Covers:
- System design and components
- Request flow (search vs extract)
- Data flow diagrams
- Performance characteristics
- Error handling strategy

### 🔧 [Extractors](extractors/)

Deep dive into the content extraction system.

Covers:
- How extractors work
- Each extractor type (Web, PDF, Video, Audio, Image)
- Router implementation
- Caching integration
- Adding new extractors

### 💾 [Caching](caching/)

How Synth's multi-layer caching works.

Covers:
- Cache architecture (extraction + LLM)
- Cache categories and TTLs
- CacheManager implementation
- Performance metrics
- Cache management

### 🌐 [API](api/)

Complete HTTP API reference.

Covers:
- All endpoints (`/search`, `/extract`, etc.)
- Request/response formats
- Error handling
- Client examples
- Rate limits

### 🚀 [Deployment](deployment/)

Production deployment guide.

Covers:
- Systemd service setup
- Docker Compose deployment
- Nginx reverse proxy
- Monitoring and backups
- Troubleshooting

### 💻 [Development](development/)

Contributing to Synth.

Covers:
- Development setup
- Code style and conventions
- Adding features (extractors, endpoints)
- Testing and debugging
- PR guidelines

---

## Quick Start

### Installation

```bash
# Clone
git clone https://github.com/Bentlybro/synth.git
cd synth

# Configure
echo "ANTHROPIC_API_KEY=sk-ant-..." > .env
echo "OPENAI_API_KEY=sk-..." >> .env

# Build and run
cargo build --release
./target/release/synth
```

Service runs on http://localhost:8765

### Example Usage

```bash
# Deep research
curl -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{"query": "rust async performance", "depth": "deep"}'

# Extract PDF
curl -X POST http://localhost:8765/extract \
  -H "Content-Type: application/json" \
  -d '{"url": "https://arxiv.org/pdf/1706.03762.pdf"}'
```

---

## Key Concepts

### Multi-Modal Content

Synth automatically handles:
- **Web pages** - HTML scraping
- **PDFs** - Text extraction
- **Videos** - Transcription (yt-dlp + Whisper)
- **Audio** - Transcription (Whisper)
- **Images** - Visual analysis (Claude Vision)

### Two-Layer Caching

1. **Extraction cache** (by URL)
   - Avoids re-downloading content
   - 24h-7d TTL depending on type

2. **LLM cache** (by URL + query)
   - Avoids re-analyzing content
   - Query-specific
   - 24h TTL

Result: **~10-900x speedup** on cache hits, **95% cost savings** on repeated queries.

### Concurrent Processing

- **10 URLs** extracted in parallel
- **5 sources** analyzed concurrently
- **3 videos** downloaded simultaneously

### Self-Hosted Privacy

- All data stays on your server
- No external dependencies except AI APIs
- SearXNG aggregates search engines locally

---

## Architecture at a Glance

```
┌─────────────────────────────────────────────────────────┐
│                     HTTP API (Axum)                     │
│              POST /search, POST /extract                │
└─────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────┐
│                  Core Components                        │
│                                                         │
│  SearXNG → ExtractorRouter → CacheManager → LLM        │
│            (5 extractors)      (multi-layer) (Claude)   │
└─────────────────────────────────────────────────────────┘
```

See [Architecture](architecture/) for detailed diagrams.

---

## Performance

| Metric | First Request | Cached | Speedup |
|--------|---------------|--------|---------|
| Web page | 2-5s | <100ms | ~20-50x |
| PDF | 3-10s | <100ms | ~30-100x |
| Video | 30-90s | <100ms | ~300-900x |
| Audio | 10-30s | <100ms | ~100-300x |
| Image | 5-10s | <100ms | ~50-100x |

**Cost savings:** 95% on repeated queries (cache hits = $0)

---

## Common Use Cases

1. **Research Assistant**
   - Query: "latest advances in quantum computing"
   - Returns: Synthesis from papers, videos, discussions

2. **PDF Analysis**
   - URL: https://arxiv.org/pdf/1706.03762.pdf
   - Query: "Summarize the Transformer architecture"
   - Returns: Key innovations, quotes, confidence

3. **Video Summaries**
   - URL: YouTube link
   - Returns: Full transcript + main points

4. **Image Analysis**
   - URL: Chart/diagram
   - Returns: Claude Vision description

---

## FAQ

**Q: Why self-hosted?**  
A: Privacy, cost control, customization

**Q: Why not just use ChatGPT?**  
A: Synth extracts content from ANY type (PDFs, videos, images), caches aggressively, and provides citations

**Q: How much does it cost?**  
A: Claude API costs (~$0.003/source analysis). First query costs, cache hits = $0

**Q: Can I add new content types?**  
A: Yes! See [Extractors Documentation](extractors/README.md#adding-new-extractors)

**Q: Production-ready?**  
A: Yes, with caveats: single-user, local deployment, no auth yet

---

## Roadmap

**Current (v1.0):**
- ✅ Multi-modal extraction
- ✅ Claude synthesis
- ✅ Comprehensive caching
- ✅ RESTful API

**Planned:**
- [ ] Word/Excel/PowerPoint support
- [ ] Code repository analysis
- [ ] Semantic search (embeddings)
- [ ] Streaming results (SSE)
- [ ] Web UI
- [ ] Multi-user support
- [ ] API authentication

---

## Contributing

See [Development Guide](development/) for:
- Setup instructions
- Code style
- Adding features
- Testing
- PR process

---

## Support

- **GitHub Issues**: [github.com/Bentlybro/synth/issues](https://github.com/Bentlybro/synth/issues)
- **Discussions**: [github.com/Bentlybro/synth/discussions](https://github.com/Bentlybro/synth/discussions)
- **Documentation**: You're reading it!

---

## License

MIT License - see [LICENSE](../LICENSE)

---

<p align="center">
  Built with Rust 🦀 by <a href="https://github.com/Bentlybro">Bently</a>
</p>
