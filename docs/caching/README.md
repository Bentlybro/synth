# Caching System

Synth uses a multi-layer caching system to minimize costs and improve response times.

## Overview

```
Request → Check Cache → Cache Hit? → Return Cached
                ↓
             Cache Miss
                ↓
        Extract/Analyze → Store in Cache → Return Fresh
```

## Cache Categories

| Category | Content | TTL | Key | Location |
|----------|---------|-----|-----|----------|
| `extractors_web` | Web pages | 24h | `hash(url)` | `index/cache/extractors_web/` |
| `extractors_pdf` | PDF documents | 24h | `hash(url)` | `index/cache/extractors_pdf/` |
| `extractors_video` | Video transcripts | 7d | `hash(url)` | `index/cache/extractors_video/` |
| `extractors_audio` | Audio transcripts | 7d | `hash(url)` | `index/cache/extractors_audio/` |
| `extractors_image` | Image descriptions | 24h | `hash(url)` | `index/cache/extractors_image/` |
| `llm` | LLM analyses | 24h | `hash(url+query)` | `index/cache/llm/` |

## Why Two Layers?

### 1. Extraction Cache (by URL)

**Purpose:** Avoid re-downloading and processing content

**Benefit:**
- Saves bandwidth
- Respects source websites
- Faster extraction (no network I/O)

**Example:**
```
First request: https://example.com/article.html
→ Download HTML (2s)
→ Parse and extract (0.1s)
→ Cache for 24h
→ Total: 2.1s

Second request (same URL):
→ Load from cache (0.05s)
→ Total: 0.05s
→ 42x faster!
```

### 2. LLM Analysis Cache (by URL + Query)

**Purpose:** Avoid re-analyzing same content with Claude

**Benefit:**
- Saves API costs (most expensive operation)
- Faster analysis (no API call)
- Query-specific caching

**Example:**
```
First request: "Summarize this article"
→ Extract: https://example.com/article.html (cached from above)
→ Analyze with Claude (5s, $0.003)
→ Cache for 24h
→ Total: 5.05s

Second request (same URL, same query):
→ Extract: cached (0.05s)
→ Analyze: cached (0.05s, $0)
→ Total: 0.1s
→ 50x faster, 100% cost savings!
```

**Query-specific:**
```
Query 1: "Summarize this article"
→ Cached as: hash(url + "Summarize this article")

Query 2: "What are the main arguments?"
→ Cached as: hash(url + "What are the main arguments?")
→ Different cache entry!
```

## Implementation

### CacheManager (`src/cache/manager.rs`)

**Core API:**
```rust
pub struct CacheManager {
    root: PathBuf,
}

impl CacheManager {
    pub async fn get<T: DeserializeOwned>(
        &self, 
        category: &str, 
        key: &str, 
        ttl_hours: u64
    ) -> Option<T>
    
    pub async fn put<T: Serialize>(
        &self, 
        category: &str, 
        key: &str, 
        data: T
    ) -> Result<()>
    
    pub async fn cleanup(&self, category: &str, ttl_hours: u64)
}
```

**File structure:**
```
index/cache/
├── extractors_web/
│   ├── a1b2c3d4e5f6.json    # hash(url1)
│   ├── f6e5d4c3b2a1.json    # hash(url2)
│   └── ...
├── llm/
│   ├── 123abc456def.json    # hash(url1 + query1)
│   ├── 789ghi012jkl.json    # hash(url1 + query2)
│   └── ...
└── ...
```

**File format:**
```json
{
  "data": {
    "url": "https://example.com",
    "title": "Article Title",
    "content": "Extracted text...",
    "content_type": "Web",
    "metadata": null
  },
  "cached_at": 1773577314
}
```

### Cache Key Generation

**File:** `src/shared/hash.rs`

```rust
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub fn cache_key<T: Hash + ?Sized>(value: &T) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

pub fn cache_key_multi(values: &[&str]) -> String {
    let combined = values.join("::");
    cache_key(&combined)
}
```

**Usage:**
```rust
// Extraction cache
let key = cache_key(&url);

// LLM cache
let key = cache_key_multi(&[&url, &query]);
```

### TTL Checking

```rust
pub async fn get<T: DeserializeOwned>(&self, category: &str, key: &str, ttl_hours: u64) -> Option<T> {
    let path = self.cache_path(category, key);
    
    // Read file
    let content = tokio::fs::read_to_string(&path).await.ok()?;
    
    // Parse
    let entry: CacheEntry<T> = serde_json::from_str(&content).ok()?;
    
    // Check TTL
    let age_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs() - entry.cached_at;
    
    if age_seconds > ttl_hours * 3600 {
        return None;  // Expired
    }
    
    Some(entry.data)
}
```

### Auto-Cleanup

**On startup** (`src/main.rs`):
```rust
tokio::spawn({
    let cache = cache_manager.clone();
    async move {
        cache.cleanup("pages", 24).await;
        cache.cleanup("youtube", 168).await;
        cache.cleanup("llm", 24).await;
        cache.cleanup("extractors_web", 24).await;
        cache.cleanup("extractors_pdf", 24).await;
        cache.cleanup("extractors_video", 168).await;
        cache.cleanup("extractors_audio", 168).await;
        cache.cleanup("extractors_image", 24).await;
    }
});
```

**Cleanup logic:**
```rust
pub async fn cleanup(&self, category: &str, ttl_hours: u64) {
    let category_path = self.root.join(category);
    let Ok(entries) = tokio::fs::read_dir(&category_path).await else { return };
    
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let mut entries_stream = entries;
    while let Ok(Some(entry)) = entries_stream.next_entry().await {
        if let Ok(metadata) = entry.metadata().await {
            if let Ok(modified) = metadata.modified() {
                let age = now - modified.duration_since(UNIX_EPOCH).unwrap().as_secs();
                if age > ttl_hours * 3600 {
                    tokio::fs::remove_file(entry.path()).await.ok();
                }
            }
        }
    }
}
```

## Performance Metrics

### Cache Hit Rates

**Typical usage patterns:**

| Pattern | Hit Rate | Explanation |
|---------|----------|-------------|
| Research same topic | 60-80% | Common sources across queries |
| Repeated queries | 95%+ | Exact same query + sources |
| Random queries | 10-20% | No overlap between queries |

### Speedup Examples

**Web page (2s extraction):**
- First: 2s
- Cached: 0.05s
- Speedup: 40x

**PDF (5s extraction):**
- First: 5s
- Cached: 0.05s
- Speedup: 100x

**Video (60s download + transcription):**
- First: 60s
- Cached: 0.05s
- Speedup: 1200x

**LLM analysis (3s API call):**
- First: 3s
- Cached: 0.05s
- Speedup: 60x

### Cost Savings

**Example: 100 queries about "Rust async"**

**Without caching:**
- 100 queries × 10 sources × $0.003 = $3.00

**With caching (60% hit rate):**
- First 40 queries: 40 × 10 × $0.003 = $1.20
- Next 60 queries: 60 × 0 (cache hits) = $0.00
- Total: $1.20
- **Savings: 60% ($1.80)**

**With caching (repeated exact query):**
- First query: 10 × $0.003 = $0.03
- Next 99 queries: $0.00
- **Savings: 99.7% ($2.97)**

## Cache Management

### View Cache Contents

```bash
# List categories
ls index/cache/

# Count files per category
find index/cache/extractors_web -name "*.json" | wc -l
find index/cache/llm -name "*.json" | wc -l

# View cache file
cat index/cache/extractors_web/a1b2c3d4.json | jq .

# Check total cache size
du -sh index/cache/
```

### Clear Cache

```bash
# Clear specific category
rm -rf index/cache/extractors_web/*

# Clear all extraction caches
rm -rf index/cache/extractors_*/*

# Clear LLM cache
rm -rf index/cache/llm/*

# Clear everything
rm -rf index/cache/*
```

### Monitor Cache

```bash
# Watch cache growth
watch -n 5 'find index/cache -name "*.json" | wc -l'

# Check cache age
find index/cache -name "*.json" -mtime +1  # Older than 1 day
find index/cache -name "*.json" -mtime -1  # Newer than 1 day
```

## TTL Tuning

### Current TTLs

| Content | TTL | Reasoning |
|---------|-----|-----------|
| Web pages | 24h | Balances freshness vs cache utility |
| PDFs | 24h | Rarely change but kept conservative |
| Videos | 7d | Transcripts static, expensive to regenerate |
| Audio | 7d | Same as video |
| Images | 24h | May be updated, vision API moderately expensive |
| LLM | 24h | Balances query freshness vs cost |

### Customization

**To change TTLs**, edit two places:

1. **`src/extractors/mod.rs`** - Extraction cache:
```rust
let (category, ttl_hours) = match self.detect_type(url) {
    Some(ContentType::Video) => ("extractors_video", 168),  // Change 168 (7 days)
    // ...
};
```

2. **`src/main.rs`** - Cleanup:
```rust
cache.cleanup("extractors_video", 168).await;  // Must match above
```

### Recommendations

**Increase TTL if:**
- Content rarely changes (academic papers, archived videos)
- API costs are high
- Network is slow

**Decrease TTL if:**
- Content changes frequently (news sites, live data)
- Disk space is limited
- Freshness is critical

## Debugging

### Cache Misses

**Enable verbose logging:**
```rust
// In src/extractors/mod.rs
info!("Extractor cache HIT ({}): {}", category, url);
info!("Extractor cache MISS ({}): {}", category, url);
```

**Check logs:**
```bash
journalctl --user -u synth -f | grep "cache"
```

### Cache Corruption

**Symptoms:**
- Deserialization errors
- Missing fields
- Unexpected None returns

**Fix:**
```bash
# Delete corrupted category
rm -rf index/cache/extractors_web/*

# Or delete specific file
rm index/cache/extractors_web/corrupt_file.json
```

### Disk Space

**Monitor:**
```bash
df -h .
du -sh index/cache/*
```

**If running out of space:**
```bash
# Reduce TTLs (rebuild + restart required)
# Or clear old caches manually
find index/cache -name "*.json" -mtime +7 -delete
```

## Next Steps

- Read [API Documentation](../api/) for how caching affects endpoints
- Read [Architecture](../architecture/) for caching's role in the system
- Read [Development Guide](../development/) for extending cache functionality
