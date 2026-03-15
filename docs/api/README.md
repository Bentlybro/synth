# API Reference

Complete reference for Synth's HTTP API.

## Base URL

```
http://localhost:8765
```

## Authentication

Currently: **None** (single-user local deployment)

Future: API key support planned

## Endpoints

### 1. POST /search

Multi-modal web search with synthesis.

**Request:**
```http
POST /search HTTP/1.1
Content-Type: application/json

{
  "query": "string (required)",
  "max_pages": number (optional, default: 5),
  "depth": "quick" | "deep" (optional, default: "quick"),
  "include_youtube": boolean (optional, default: false),
  "max_videos": number (optional, default: 2)
}
```

**Parameters:**

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `query` | string | ✅ | - | Search query |
| `max_pages` | number | ❌ | 5 | Max sources to process |
| `depth` | "quick"\|"deep" | ❌ | "quick" | Quick: up to 10, Deep: up to 20 |
| `include_youtube` | boolean | ❌ | false | Include video transcriptions |
| `max_videos` | number | ❌ | 2 | Max videos to transcribe |

**Response:**
```http
HTTP/1.1 200 OK
Content-Type: application/json

{
  "status": "complete",
  "synthesis": "# Markdown Answer\n\n...",
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

**Status values:**
- `"complete"` - Success
- `"error"` - Failed

**Example:**
```bash
curl -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "rust memory safety explained",
    "max_pages": 10,
    "depth": "deep"
  }' | jq .
```

---

### 2. POST /extract

Extract and analyze content from a URL.

**Request:**
```http
POST /extract HTTP/1.1
Content-Type: application/json

{
  "url": "string (required)",
  "query": "string (optional)"
}
```

**Parameters:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | string | ✅ | URL to extract content from |
| `query` | string | ❌ | Optional analysis query (triggers LLM analysis) |

**Response:**
```http
HTTP/1.1 200 OK
Content-Type: application/json

{
  "url": "https://example.com/document.pdf",
  "title": "Document Title",
  "content_type": "PDF",
  "content": "Extracted text content...",
  "analysis": {
    "url": "https://example.com/document.pdf",
    "title": "Document Title",
    "key_facts": ["Fact 1", "Fact 2"],
    "quotes": ["Quote 1"],
    "confidence": 0.9
  },
  "metadata": {
    "file_size_bytes": 1024000,
    "format": "PDF"
  }
}
```

**Content types:**
- `"Web"` - Web page
- `"PDF"` - PDF document
- `"Video"` - Video (YouTube, etc.)
- `"Audio"` - Audio file
- `"Image"` - Image file

**Metadata fields:**

| Field | Type | Present | Description |
|-------|------|---------|-------------|
| `duration_seconds` | number | Video/Audio | Length in seconds |
| `file_size_bytes` | number | PDF/Audio/Image | File size |
| `format` | string | PDF/Audio/Image | MIME type or format |
| `dimensions` | [number, number] | Image | Width × height |

**Examples:**

```bash
# Extract PDF
curl -X POST http://localhost:8765/extract \
  -H "Content-Type: application/json" \
  -d '{"url": "https://arxiv.org/pdf/1706.03762.pdf"}' | jq .

# Analyze PDF with query
curl -X POST http://localhost:8765/extract \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://arxiv.org/pdf/1706.03762.pdf",
    "query": "Summarize the Transformer architecture"
  }' | jq .

# Transcribe YouTube video
curl -X POST http://localhost:8765/extract \
  -H "Content-Type: application/json" \
  -d '{"url": "https://youtube.com/watch?v=dQw4w9WgXcQ"}' | jq .

# Analyze image
curl -X POST http://localhost:8765/extract \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://example.com/chart.png",
    "query": "What does this chart show?"
  }' | jq .
```

---

### 3. GET /health

Health check endpoint.

**Request:**
```http
GET /health HTTP/1.1
```

**Response:**
```http
HTTP/1.1 200 OK
Content-Type: text/plain

OK
```

**Example:**
```bash
curl http://localhost:8765/health
# Returns: OK
```

---

### 4. GET /stats

Cache statistics.

**Request:**
```http
GET /stats HTTP/1.1
```

**Response:**
```http
HTTP/1.1 200 OK
Content-Type: application/json

{
  "cached_pages": 127
}
```

**Example:**
```bash
curl http://localhost:8765/stats | jq .
```

---

## Error Responses

**Format:**
```json
{
  "error": "error_type",
  "message": "Human-readable description"
}
```

**Common errors:**

| Error | Status | Cause | Solution |
|-------|--------|-------|----------|
| `missing_api_key` | 400 | ANTHROPIC_API_KEY not set | Set environment variable |
| `extraction_failed` | 400 | URL inaccessible or invalid | Check URL, verify content type |
| `invalid_url` | 400 | Malformed URL | Provide valid HTTP/HTTPS URL |
| `timeout` | 504 | Request took too long | Retry or reduce complexity |

**Example error:**
```json
{
  "error": "extraction_failed",
  "message": "Failed to download PDF: 404 Not Found"
}
```

---

## Rate Limits

**Current:** None (local deployment)

**Effective limits:**
- Concurrency: 10 URL extractions, 5 LLM analyses
- Claude API: Subject to Anthropic's rate limits
- Whisper API: Subject to OpenAI's rate limits

---

## Client Examples

### JavaScript (fetch)

```javascript
async function search(query) {
  const response = await fetch('http://localhost:8765/search', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      query,
      max_pages: 10,
      depth: 'deep'
    })
  });
  
  const data = await response.json();
  return data.synthesis;
}

// Usage
const answer = await search('rust async performance');
console.log(answer);
```

### Python (requests)

```python
import requests

def search(query, max_pages=10):
    response = requests.post('http://localhost:8765/search', json={
        'query': query,
        'max_pages': max_pages,
        'depth': 'deep'
    })
    return response.json()['synthesis']

# Usage
answer = search('rust async performance')
print(answer)
```

### Rust (reqwest)

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct SearchRequest {
    query: String,
    max_pages: usize,
    depth: String,
}

#[derive(Deserialize)]
struct SearchResponse {
    synthesis: String,
}

async fn search(query: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8765/search")
        .json(&SearchRequest {
            query: query.to_string(),
            max_pages: 10,
            depth: "deep".to_string(),
        })
        .send()
        .await?
        .json::<SearchResponse>()
        .await?;
    
    Ok(response.synthesis)
}
```

---

## Streaming (Future)

**Planned:** Server-Sent Events (SSE) for progress updates

```
POST /search?stream=true

data: {"status": "searching", "progress": 10}
data: {"status": "extracting", "progress": 40}
data: {"status": "analyzing", "progress": 70}
data: {"status": "complete", "synthesis": "..."}
```

Not yet implemented.

---

## Next Steps

- Read [Architecture](../architecture/) for request flow details
- Read [Deployment](../deployment/) for production setup
- Read [Development](../development/) for extending the API
