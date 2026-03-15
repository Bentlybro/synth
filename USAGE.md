# Synth Usage Guide

Complete guide to using Synth's universal AI research engine.

## Quick Start

```bash
# Start the service
systemctl --user start synth

# Check status
systemctl --user status synth

# View logs
journalctl --user -u synth -f
```

## API Endpoints

### 1. Search (Multi-Modal)

**Endpoint:** `POST /search`

Search the web and extract content from ANY type of URL (web pages, PDFs, videos, images, audio).

**Request:**
```json
{
  "query": "rust memory safety",
  "max_pages": 5,
  "depth": "quick",
  "include_youtube": true,
  "max_videos": 2
}
```

**Fields:**
- `query` (required): Search query
- `max_pages` (optional, default 5): Max number of results to process
- `depth` (optional, default "quick"): "quick" (up to 10 results) or "deep" (up to 20)
- `include_youtube` (optional, default false): Include YouTube video transcriptions
- `max_videos` (optional, default 2): Max YouTube videos to transcribe

**Response:**
```json
{
  "status": "complete",
  "synthesis": "# Rust Memory Safety\n\n...",
  "sources": [
    {
      "url": "https://example.com/article",
      "title": "Memory Safety in Rust",
      "key_facts": ["Fact 1", "Fact 2"],
      "quotes": ["Quote 1", "Quote 2"],
      "confidence": 0.95
    }
  ]
}
```

**Example:**
```bash
curl -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "how does rust prevent memory leaks",
    "max_pages": 3,
    "include_youtube": true
  }' | jq .
```

---

### 2. Extract (Direct URL Analysis)

**Endpoint:** `POST /extract`

Extract and analyze content from a specific URL. Works with ANY content type.

**Request:**
```json
{
  "url": "https://example.com/document.pdf",
  "query": "What is this document about?"
}
```

**Fields:**
- `url` (required): URL to extract content from
- `query` (optional): If provided, analyzes content with Claude

**Response:**
```json
{
  "url": "https://example.com/document.pdf",
  "title": "Research Paper",
  "content_type": "PDF",
  "content": "Full extracted text...",
  "analysis": {
    "url": "https://example.com/document.pdf",
    "title": "Research Paper",
    "key_facts": ["Fact 1", "Fact 2"],
    "quotes": ["Quote 1", "Quote 2"],
    "confidence": 0.9
  },
  "metadata": {
    "file_size_bytes": 1024000,
    "format": "PDF"
  }
}
```

**Examples:**

Extract a web page:
```bash
curl -X POST http://localhost:8765/extract \
  -H "Content-Type: application/json" \
  -d '{"url": "https://www.rust-lang.org/"}' | jq .
```

Analyze a PDF:
```bash
curl -X POST http://localhost:8765/extract \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://arxiv.org/pdf/2101.00000.pdf",
    "query": "What are the key findings?"
  }' | jq .
```

Describe an image:
```bash
curl -X POST http://localhost:8765/extract \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://example.com/chart.png",
    "query": "What does this chart show?"
  }' | jq .
```

Transcribe audio:
```bash
curl -X POST http://localhost:8765/extract \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com/podcast.mp3"}' | jq .
```

---

### 3. Stats

**Endpoint:** `GET /stats`

Get cache statistics.

**Response:**
```json
{
  "cached_pages": 127
}
```

**Example:**
```bash
curl http://localhost:8765/stats | jq .
```

---

### 4. Health Check

**Endpoint:** `GET /health`

Check if service is running.

**Response:**
```
OK
```

**Example:**
```bash
curl http://localhost:8765/health
```

---

## Supported Content Types

| Type | Extensions/Domains | Processing |
|------|-------------------|------------|
| **Web Pages** | Any HTTP/HTTPS | HTML scraping → main content extraction |
| **PDF Documents** | `.pdf` | Text extraction → 100k char limit |
| **Videos** | YouTube, Vimeo, TikTok, Twitch, etc. | yt-dlp download → Whisper transcription |
| **Audio Files** | `.mp3`, `.wav`, `.m4a`, `.ogg`, `.flac` | Download → Whisper transcription |
| **Images** | `.jpg`, `.png`, `.gif`, `.webp` | Download → Claude Vision analysis |

### Requirements by Type

**Web Pages:** No requirements (always works)

**PDF Documents:** No requirements (always works)

**Videos:**
- `yt-dlp` installed (`pip install yt-dlp`)
- `OPENAI_API_KEY` environment variable

**Audio:**
- `OPENAI_API_KEY` environment variable

**Images:**
- `ANTHROPIC_API_KEY` environment variable (already configured)

---

## Caching

All content is cached to save time and API costs.

### Cache Categories

| Category | TTL | Location |
|----------|-----|----------|
| Web pages | 24 hours | `index/cache/extractors_web/` |
| PDFs | 24 hours | `index/cache/extractors_pdf/` |
| Videos | 7 days | `index/cache/extractors_video/` |
| Audio | 7 days | `index/cache/extractors_audio/` |
| Images | 24 hours | `index/cache/extractors_image/` |
| LLM analyses | 24 hours | `index/cache/llm/` |

### Cache Behavior

**First request:**
- Extracts content from URL
- Stores in cache
- Time: Full extraction time (varies by type)

**Subsequent requests (within TTL):**
- Loads from cache instantly
- Time: <100ms (instant!)
- Cost: $0 (no API calls)

### Cache Management

View cache contents:
```bash
ls -lh ~/clawd/projects/synth/index/cache/extractors_web/
ls -lh ~/clawd/projects/synth/index/cache/extractors_video/
```

Clear specific cache:
```bash
rm -rf ~/clawd/projects/synth/index/cache/extractors_web/*
```

Clear all caches:
```bash
rm -rf ~/clawd/projects/synth/index/cache/extractors_*/*
```

Cache auto-cleanup runs on service startup (removes expired files).

---

## Performance

### Extraction Times

| Content Type | First Request | Cached Request |
|--------------|---------------|----------------|
| Web page | 2-5 seconds | <100ms |
| PDF | 3-10 seconds | <100ms |
| Video | 30-90 seconds | <100ms |
| Audio | 10-30 seconds | <100ms |
| Image | 5-10 seconds | <100ms |

### Cost Savings

**Without caching:**
- 100 identical queries = 100 × extraction cost

**With caching:**
- 100 identical queries = 1 × extraction cost + 99 × $0
- **Savings:** ~99% on repeated queries

**Example:**
- YouTube transcription: $0.006/minute
- 10-minute video first time: $0.06
- Same video 100 more times: $0.00
- **Total saved:** $6.00

---

## Common Use Cases

### 1. Research a Topic

```bash
curl -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "quantum computing applications",
    "max_pages": 10,
    "depth": "deep",
    "include_youtube": true,
    "max_videos": 3
  }' | jq '.synthesis' -r > research.md
```

Combines web pages, PDFs, and videos into a single comprehensive analysis.

### 2. Analyze a Research Paper

```bash
curl -X POST http://localhost:8765/extract \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://arxiv.org/pdf/1706.03762.pdf",
    "query": "Summarize the key innovations in the Transformer architecture"
  }' | jq '.analysis'
```

### 3. Transcribe a Podcast

```bash
curl -X POST http://localhost:8765/extract \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://example.com/podcast-episode.mp3"
  }' | jq '.content' -r > transcript.txt
```

### 4. Describe Technical Diagrams

```bash
curl -X POST http://localhost:8765/extract \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://example.com/architecture-diagram.png",
    "query": "Explain this system architecture"
  }' | jq '.analysis.key_facts'
```

### 5. Learn from Video Tutorials

```bash
curl -X POST http://localhost:8765/extract \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
    "query": "What are the main points covered?"
  }' | jq '.analysis'
```

---

## Integration with OpenClaw

Synth is designed to be called by Orion (your AI assistant) when you ask research questions.

**Example conversation:**

**You:** "What's the current state of AI safety research?"

**Orion internally calls:**
```bash
synth.sh "current state of AI safety research" --deep --youtube
```

**Orion responds:** With a synthesized answer combining multiple sources.

---

## Environment Variables

```bash
# Required
ANTHROPIC_API_KEY=sk-ant-...  # For Claude analysis (always used)

# Optional
OPENAI_API_KEY=sk-...          # For Whisper transcription (video/audio)
SEARXNG_URL=http://localhost:8888  # SearXNG instance (default: localhost)
CACHE_TTL_SECONDS=86400        # Legacy cache TTL (default: 24h)
```

---

## Troubleshooting

### Service won't start

```bash
# Check logs
journalctl --user -u synth --no-pager | tail -50

# Check if port is in use
lsof -i :8765

# Restart service
systemctl --user restart synth
```

### Extraction fails

**PDF extraction error:**
- Check if URL is accessible
- Verify PDF is valid (not encrypted)

**Video extraction error:**
- Ensure `yt-dlp` is installed: `pip install yt-dlp`
- Check `OPENAI_API_KEY` is set
- Verify video URL is supported

**Image analysis error:**
- Check `ANTHROPIC_API_KEY` is set
- Verify image URL is accessible

### Slow performance

**First time is slow:**
- Normal! First extraction takes time
- Subsequent requests use cache (fast)

**Always slow:**
- Check cache directory exists: `ls ~/clawd/projects/synth/index/cache/`
- Verify cache cleanup isn't deleting everything
- Check disk space

### No results from search

**SearXNG not found:**
- Ensure SearXNG is running on port 8888
- Check `SEARXNG_URL` environment variable

**All extractions fail:**
- Check network connectivity
- Verify URLs are accessible
- Check service logs for errors

---

## Testing

Run integration tests:
```bash
cd ~/clawd/projects/synth
./test_integration.sh
```

Expected output:
```
=== Synth Integration Tests ===

1. Health check... ✓
2. Extracting web page... ✓
3. Extracting with LLM analysis... ✓
4. Search query... ✓
5. Checking cache directories... ✓

=== All Tests Passed! ===
```

---

## Advanced Usage

### Batch Processing

Process multiple URLs:
```bash
for url in url1 url2 url3; do
  curl -X POST http://localhost:8765/extract \
    -H "Content-Type: application/json" \
    -d "{\"url\": \"$url\"}" | jq .
done
```

### Custom Analysis

Extract content, then analyze with custom query:
```bash
# Extract
CONTENT=$(curl -X POST http://localhost:8765/extract \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com/article"}' | jq -r '.content')

# Analyze with custom query
curl -X POST http://localhost:8765/extract \
  -H "Content-Type: application/json" \
  -d "{\"url\": \"https://example.com/article\", \"query\": \"What are the main arguments?\"}"
```

### Pipeline with jq

Extract specific fields:
```bash
curl -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{"query": "rust vs go", "max_pages": 3}' | \
  jq -r '.sources[] | "\(.title)\n\(.key_facts | join("\n"))\n"'
```

---

## Future Enhancements

Planned features:
- [ ] Word documents (.docx)
- [ ] Spreadsheets (.xlsx, .csv)
- [ ] Presentations (.pptx)
- [ ] Code repositories (GitHub analysis)
- [ ] RSS/Atom feeds
- [ ] Email parsing (.eml)
- [ ] Archive extraction (.zip contents)
- [ ] Semantic search (vector embeddings)
- [ ] Multi-language support
- [ ] Real-time streaming results

---

## Support

- **Documentation:** `~/clawd/projects/synth/README.md`
- **Extractors Guide:** `~/clawd/projects/synth/src/extractors/README.md`
- **Cache Proof:** `~/clawd/projects/synth/CACHE_PROOF.md`
- **GitHub:** https://github.com/Bentlybro/synth
- **Issues:** Open an issue on GitHub

---

**Status:** Production-ready, fully tested, documented ✅
