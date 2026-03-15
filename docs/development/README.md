# Development Guide

Contributing to Synth.

## Getting Started

### Prerequisites

- Rust 1.88+ (`rustup install stable`)
- Git
- SearXNG instance (Docker recommended)
- API keys (Anthropic, optionally OpenAI)

### Clone and Setup

```bash
# Fork repository on GitHub
# Then clone your fork
git clone https://github.com/YOUR_USERNAME/synth.git
cd synth

# Add upstream remote
git remote add upstream https://github.com/Bentlybro/synth.git

# Create .env file
cat > .env << EOF
ANTHROPIC_API_KEY=sk-ant-...
OPENAI_API_KEY=sk-...
SEARXNG_URL=http://localhost:8888
RUST_LOG=debug
EOF

# Build
cargo build

# Run
cargo run
```

---

## Project Structure

```
synth/
├── src/
│   ├── main.rs              # Entry point, initialization
│   ├── api/                 # HTTP API
│   │   └── mod.rs           # Routes, handlers
│   ├── cache/               # Caching system
│   │   ├── mod.rs           # Legacy PageCache
│   │   └── manager.rs       # CacheManager
│   ├── extractors/          # Content extraction
│   │   ├── mod.rs           # Router, traits
│   │   ├── web.rs           # Web scraper
│   │   ├── pdf.rs           # PDF extractor
│   │   ├── video.rs         # Video transcriber
│   │   ├── audio.rs         # Audio transcriber
│   │   ├── image.rs         # Image analyzer
│   │   └── README.md        # Extractor docs
│   ├── llm/                 # Claude integration
│   │   └── mod.rs           # Analysis, synthesis
│   ├── models/              # Data structures
│   │   └── mod.rs           # Request/response types
│   ├── scraper/             # Legacy scraper (being phased out)
│   │   └── mod.rs
│   ├── search/              # SearXNG client
│   │   ├── mod.rs
│   │   └── searxng.rs
│   ├── shared/              # Utilities
│   │   ├── mod.rs
│   │   └── hash.rs          # Cache key generation
│   └── youtube/             # Legacy YouTube (being phased out)
│       └── mod.rs
├── docs/                    # Documentation
│   ├── architecture/
│   ├── extractors/
│   ├── caching/
│   ├── api/
│   ├── deployment/
│   └── development/
├── examples/                # Usage examples
├── Cargo.toml               # Dependencies
├── Dockerfile
├── docker-compose.yml
├── synth.service            # Systemd service
└── README.md
```

---

## Development Workflow

### 1. Create Feature Branch

```bash
git checkout -b feature/amazing-feature
```

### 2. Make Changes

```bash
# Edit code
nano src/extractors/new_type.rs

# Format
cargo fmt

# Check
cargo clippy

# Build
cargo build

# Test
cargo test

# Run
cargo run
```

### 3. Test Changes

```bash
# Start service
cargo run

# In another terminal, test API
curl -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{"query": "test query", "max_pages": 2}'
```

### 4. Commit and Push

```bash
git add .
git commit -m "feat: Add amazing feature"
git push origin feature/amazing-feature
```

### 5. Create PR

Open Pull Request on GitHub with:
- Clear description
- Test results
- Breaking changes (if any)

---

## Code Style

### Rust Conventions

```rust
// Use descriptive names
pub async fn extract_content(&self, url: &str) -> Result<String>

// Document public APIs
/// Extract main content from HTML
/// 
/// # Arguments
/// * `html` - Raw HTML string
/// 
/// # Returns
/// Cleaned text content (up to 50k chars)
pub fn extract_content(&self, html: &str) -> String

// Use ?Sized for generic parameters accepting &str
pub fn cache_key<T: Hash + ?Sized>(value: &T) -> String

// Prefer async/await over .then() chains
let content = self.extract(url).await?;

// Use tracing for logs
info!("Extracted content from: {}", url);
```

### Format and Lint

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Lint
cargo clippy

# Fix clippy warnings
cargo clippy --fix
```

---

## Adding Features

### New Content Extractor

See [Extractors Documentation](../extractors/README.md#adding-new-extractors) for detailed guide.

**Quick example:**

```rust
// 1. Create src/extractors/word.rs
use async_trait::async_trait;
use super::{ContentExtractor, ContentType, ExtractedContent};

pub struct WordExtractor;

#[async_trait]
impl ContentExtractor for WordExtractor {
    fn can_handle(&self, url: &str) -> bool {
        url.to_lowercase().ends_with(".docx")
    }
    
    async fn extract(&self, url: &str) -> Result<ExtractedContent> {
        // Extraction logic
        Ok(ExtractedContent { /* ... */ })
    }
    
    fn content_type(&self) -> ContentType {
        ContentType::Word
    }
}

// 2. Add ContentType::Word to enum
// 3. Register in ExtractorRouter
// 4. Add cache category
// 5. Add cleanup
```

### New API Endpoint

```rust
// In src/api/mod.rs

// 1. Add route
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/search", post(search_handler))
        .route("/extract", post(extract_handler))
        .route("/my_endpoint", post(my_handler))  // Add here
        .with_state(state)
}

// 2. Add handler
async fn my_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<MyRequest>,
) -> Result<Json<MyResponse>, AppError> {
    // Handler logic
    Ok(Json(MyResponse { /* ... */ }))
}

// 3. Add request/response types in src/models/mod.rs
#[derive(Serialize, Deserialize)]
pub struct MyRequest {
    pub field: String,
}

#[derive(Serialize, Deserialize)]
pub struct MyResponse {
    pub result: String,
}
```

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key() {
        let key1 = cache_key("https://example.com");
        let key2 = cache_key("https://example.com");
        assert_eq!(key1, key2);
    }

    #[tokio::test]
    async fn test_extract_web() {
        let extractor = WebExtractor::new();
        assert!(extractor.can_handle("https://example.com"));
        assert!(!extractor.can_handle("https://example.com/file.pdf"));
    }
}
```

### Integration Tests

```bash
# Start service
cargo run &
sleep 2

# Test endpoints
curl http://localhost:8765/health
curl -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{"query": "test", "max_pages": 1}'

# Kill service
pkill synth
```

### Run Tests

```bash
# All tests
cargo test

# Specific test
cargo test test_cache_key

# With output
cargo test -- --nocapture

# Integration tests only
cargo test --test integration
```

---

## Debugging

### Enable Debug Logs

```bash
RUST_LOG=debug cargo run
```

### Add Logging

```rust
use tracing::{info, warn, error, debug};

info!("Processing URL: {}", url);
debug!("Cache key: {}", key);
warn!("Extraction took longer than expected: {}ms", elapsed);
error!("Failed to download: {}", e);
```

### Use Debugger

```bash
# Install lldb
sudo apt-get install lldb

# Build with debug symbols
cargo build

# Run with debugger
rust-lldb ./target/debug/synth
```

---

## Documentation

### Code Documentation

```bash
# Generate docs
cargo doc --no-deps --open

# Check doc coverage
cargo doc --no-deps 2>&1 | grep warning
```

### Adding Docs

```rust
/// Brief description
///
/// Longer explanation with details.
///
/// # Arguments
/// * `url` - URL to extract from
/// * `cache` - Cache manager instance
///
/// # Returns
/// Extracted content or error
///
/// # Example
/// ```
/// let content = extractor.extract_cached(&url, &cache).await?;
/// ```
pub async fn extract_cached(&self, url: &str, cache: &CacheManager) -> Result<ExtractedContent>
```

---

## Performance

### Profiling

```bash
# Install flamegraph
cargo install flamegraph

# Profile
cargo flamegraph --bin synth
```

### Benchmarking

```rust
// benches/cache_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn cache_key_benchmark(c: &mut Criterion) {
    c.bench_function("cache_key", |b| {
        b.iter(|| cache_key(black_box("https://example.com")))
    });
}

criterion_group!(benches, cache_key_benchmark);
criterion_main!(benches);
```

```bash
cargo bench
```

---

## Common Tasks

### Update Dependencies

```bash
# Check outdated
cargo outdated

# Update Cargo.lock
cargo update

# Update Cargo.toml
# Edit versions manually, then:
cargo build
```

### Fix Warnings

```bash
# Show all warnings
cargo clippy --all-targets

# Auto-fix
cargo clippy --fix
cargo fmt
```

### Clean Build

```bash
cargo clean
cargo build --release
```

---

## Contribution Guidelines

### Code Quality

- ✅ All tests pass
- ✅ No clippy warnings
- ✅ Formatted with `cargo fmt`
- ✅ Documented public APIs
- ✅ Updated docs if behavior changed

### Commit Messages

```
feat: Add Word document extractor
fix: Handle empty PDF files gracefully
docs: Update API reference with new endpoint
refactor: Simplify cache key generation
test: Add integration tests for extractors
```

### PR Checklist

- [ ] Code compiles without warnings
- [ ] Tests pass (`cargo test`)
- [ ] Formatted (`cargo fmt`)
- [ ] Linted (`cargo clippy`)
- [ ] Documentation updated
- [ ] CHANGELOG updated (if user-facing)
- [ ] Breaking changes noted

---

## Resources

### Learning Rust

- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Async Book](https://rust-lang.github.io/async-book/)

### Dependencies

- [Axum Docs](https://docs.rs/axum/)
- [Tokio Docs](https://docs.rs/tokio/)
- [Serde Docs](https://serde.rs/)
- [Reqwest Docs](https://docs.rs/reqwest/)

### Tools

- [Rust Analyzer](https://rust-analyzer.github.io/) - LSP for IDEs
- [Cargo Watch](https://github.com/watchexec/cargo-watch) - Auto-rebuild on changes
- [Bacon](https://github.com/Canop/bacon) - Background Rust task runner

---

## Getting Help

- **Issues**: [GitHub Issues](https://github.com/Bentlybro/synth/issues)
- **Discussions**: [GitHub Discussions](https://github.com/Bentlybro/synth/discussions)
- **Docs**: [docs/](../)

---

## Next Steps

- Read [Architecture Documentation](../architecture/) for system design
- Read [Extractors Documentation](../extractors/) for adding content types
- Read [API Documentation](../api/) for endpoint specs
