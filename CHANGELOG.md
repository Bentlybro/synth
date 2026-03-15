# Changelog

All notable changes to Synth will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added - March 15, 2026 (🔥 BEAST MODE Update)

#### Semantic Search
- **Semantic cache matching** using OpenAI embeddings (text-embedding-3-small)
- **0.7 similarity threshold** for intelligent cache hits
- **Vector storage** with cosine similarity calculation
- **Async embedding generation** (non-blocking, doesn't slow searches)
- **Persistent embedding cache** (index/cache/embeddings.json)
- **7-day TTL** for embedding cleanup

#### Query Intelligence
- **Query expansion** for Deep mode (up to 5 related queries)
- Automatic variations:
  - "how does X work" → "X explained", "X tutorial", "understanding X"
  - Adds "latest", "guide", "best practices" variants
  - Language-aware expansions (Rust, Python, JS, etc.)
- **Smart relevance ranking** with scoring system:
  - Title matches: 3x weight
  - Exact phrase matches: 10x weight (title), 5x (snippet)
  - Recent content bonus (2026, 2025, latest, new)
  - Official docs/guides bonus
- **Key term extraction** (removes stop words)

#### Code Repository Analysis
- **GitHub repository extraction** with git2-rs
- **Language detection** (19 languages supported)
- **Smart file selection**:
  - Important files: README, LICENSE, dependencies
  - Entry points: main.rs, index.ts, __init__.py, etc.
  - Core source files (sampled)
- **Deep mode** support:
  - Basic: 20 files, 2-level tree
  - Deep: 100 files, 4-level tree
- **Commit-aware caching** (URL + commit hash)
- **7-day cache TTL** for repos
- **Auto-cleanup** of temp clone directories

#### Performance Improvements
- **Increased concurrent extraction**: 10 → 15 URLs
- **Parallel query search** in Deep mode
- **Result deduplication** by URL
- **Relevance-based sorting** before extraction
- **Async embedding storage** (non-blocking)

#### Documentation
- **One-script installer** (`install.sh`)
- **Comprehensive docs/** structure:
  - architecture/ - System design
  - extractors/ - Content extraction internals
  - caching/ - Cache system details
  - api/ - Endpoint reference
  - deployment/ - Production setup
  - development/ - Contributing guide
- **INSTALL.md** - Detailed installation guide
- **Updated README** with new features showcase
- **Example use cases** demonstrating capabilities

### Changed

#### Metadata Flexibility
- Changed `ExtractedContent.metadata` from struct to `serde_json::Value`
- Allows different extractors to provide different metadata
- Updated all extractors to use `json!` macro

#### Thread Safety
- Switched from `std::sync::RwLock` to `tokio::sync::RwLock`
- Enables async operations in spawned tasks
- Required for semantic search integration

#### Cache Structure
- Added `extractors_code` category (7-day TTL)
- Updated cleanup to include new cache categories
- Centralized cache management across all extractors

### Fixed
- **Metadata extraction** in API handler (JSON Value conversion)
- **Type annotations** in embedding similarity search
- **Send trait** compatibility for tokio::spawn

---

## [0.1.0] - 2026-03-15 (Initial Public Release)

### Added
- **Multi-modal content extraction**:
  - Web pages (HTML scraping)
  - PDF documents (text extraction)
  - Videos (yt-dlp + Whisper transcription)
  - Audio files (Whisper transcription)
  - Images (Claude Vision analysis)
- **SearXNG integration** for web search
- **Claude AI analysis** with parallel processing
- **Two-layer caching**:
  - Extraction cache (by URL)
  - LLM analysis cache (by URL + query)
- **Tantivy-based full-text search index**
- **RESTful API**:
  - POST /search - Multi-modal search
  - POST /extract - Direct URL extraction
  - GET /health - Health check
  - GET /stats - Cache statistics
- **Systemd service** support
- **Docker Compose** deployment option
- **MIT License** (public release)

### Performance
- **10 concurrent URL extractions**
- **5 concurrent LLM analyses**
- **3 concurrent YouTube downloads**
- **~10-900x speedup** on cache hits
- **95% cost savings** on repeated queries

---

## Version History

- **[Unreleased]** - BEAST mode features (semantic search, query expansion, code repos)
- **[0.1.0]** - Initial public release with multi-modal extraction

---

## Upgrade Guide

### Upgrading to BEAST Mode (March 15, 2026)

**New dependencies:**
- git2 (0.19) - Git operations for code repos
- walkdir (2.5) - File tree traversal
- ignore (0.4) - Gitignore support

**New environment variables (optional):**
- `OPENAI_API_KEY` - Enables semantic search + video/audio transcription
  - Without it: Semantic search disabled, videos/audio not transcribed
  - With it: Full semantic matching + Whisper support

**Breaking changes:**
- None - all changes are additive and backward compatible

**To upgrade:**
```bash
cd ~/synth
git pull
cargo build --release
systemctl --user restart synth
```

---

## Future Roadmap

### Planned Features
- [ ] Streaming results via Server-Sent Events (SSE)
- [ ] Multi-user support with API authentication
- [ ] Web UI for testing and debugging
- [ ] Additional repository platforms (GitLab, Bitbucket)
- [ ] Local repository path support
- [ ] Word/Excel/PowerPoint extraction
- [ ] Spreadsheet data analysis
- [ ] Presentation slide analysis
- [ ] Code repository diff analysis
- [ ] Multi-engine search aggregation (beyond SearXNG)
- [ ] Configurable similarity thresholds
- [ ] Query optimization suggestions
- [ ] Result ranking models
- [ ] Custom extractor plugins
- [ ] Embeddings model selection

### Under Consideration
- Self-hosted embeddings (local model instead of OpenAI)
- RAG (Retrieval-Augmented Generation) capabilities
- Knowledge graph generation
- Automatic source reliability scoring
- Multi-language support
- Result filtering and faceting
- Export to various formats (JSON, XML, CSV)

---

**Contributions welcome!** See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.
