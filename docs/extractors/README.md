# Content Extractors

Synth's extractor system provides universal content extraction from any URL type.

## Overview

```
URL → ExtractorRouter → Detect Type → Route to Extractor → Extract Content
```

All extractors implement the same trait:

```rust
#[async_trait]
pub trait ContentExtractor: Send + Sync {
    fn can_handle(&self, url: &str) -> bool;
    async fn extract(&self, url: &str) -> Result<ExtractedContent>;
    fn content_type(&self) -> ContentType;
}
```

## Unified Output

All extractors return the same structure:

```rust
pub struct ExtractedContent {
    pub url: String,
    pub title: String,
    pub content: String,  // Extracted text/transcript/description
    pub content_type: ContentType,
    pub metadata: Option<ContentMetadata>,
}
```

This allows downstream code (LLM analysis, caching) to be content-type agnostic.

## Extractors

### 1. Web Extractor (`web.rs`)

**Handles:** Any HTTP/HTTPS URL (fallback extractor)

**Detection:**
```rust
fn can_handle(&self, url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}
```

**Process:**
1. HTTP GET with reqwest
2. Parse HTML with `scraper` crate
3. Try content selectors (`article`, `main`, `.content`)
4. Extract title (title tag, og:title)
5. Clean text (normalize whitespace, limit 50k chars)

**Key code:**
```rust
fn extract_content(&self, html: &str) -> String {
    let document = Html::parse_document(html);
    
    // Try main content selectors
    for selector_str in ["article", "main", "[role='main']", ".content"] {
        if let Ok(selector) = Selector::parse(selector_str) {
            if let Some(element) = document.select(&selector).next() {
                let text = element.text().collect::<Vec<_>>().join(" ");
                if text.len() > 200 {
                    return Self::clean_text(&text);
                }
            }
        }
    }
    
    // Fallback: body
    // ...
}
```

**Limits:**
- 50k characters
- 30 second timeout
- No JavaScript execution (use browser tool for that)

---

### 2. PDF Extractor (`pdf.rs`)

**Handles:** `.pdf` file extension

**Detection:**
```rust
fn can_handle(&self, url: &str) -> bool {
    url.to_lowercase().ends_with(".pdf")
}
```

**Process:**
1. Download PDF with reqwest
2. Extract text with `pdf-extract` crate
3. Run extraction in `spawn_blocking` (CPU-intensive)
4. Extract title from filename
5. Limit to 100k chars

**Key code:**
```rust
async fn extract_text(&self, pdf_bytes: &[u8]) -> Result<String> {
    let text = tokio::task::spawn_blocking({
        let bytes = pdf_bytes.to_vec();
        move || -> Result<String> {
            let cursor = Cursor::new(bytes);
            let extracted = pdf_extract::extract_text_from_mem(&cursor.into_inner())
                .context("Failed to extract text from PDF")?;
            Ok(extracted)
        }
    })
    .await
    .context("PDF extraction task failed")??;
    
    Ok(text)
}
```

**Limits:**
- 100k characters
- 60 second timeout
- Stores file size in metadata

**Gotchas:**
- Encrypted PDFs fail
- Scanned PDFs (no text layer) return empty
- Complex layouts may have scrambled text order

---

### 3. Video Extractor (`video.rs`)

**Handles:** YouTube, Vimeo, TikTok, Twitch, Dailymotion

**Detection:**
```rust
fn can_handle(&self, url: &str) -> bool {
    let video_domains = [
        "youtube.com", "youtu.be",
        "vimeo.com", "dailymotion.com",
        "twitch.tv", "tiktok.com",
    ];
    video_domains.iter().any(|domain| url.contains(domain))
}
```

**Process:**
1. Check `yt-dlp` is installed
2. Get metadata (`yt-dlp --dump-json`)
3. Download audio only (`--extract-audio --audio-format mp3 --audio-quality 16K`)
4. Save to temp file (`/tmp/synth_video/{uuid}.mp3`)
5. Transcribe with OpenAI Whisper API
6. Delete temp file (auto-cleanup via `Drop` trait)

**Key code:**
```rust
async fn download_and_transcribe(&self, url: &str) -> Result<(String, String, f64)> {
    let temp_id = uuid::Uuid::new_v4();
    let audio_path = self.temp_dir.join(format!("{}.mp3", temp_id));
    
    // Download
    let output = Command::new("yt-dlp")
        .args([
            "--extract-audio",
            "--audio-format", "mp3",
            "--audio-quality", "16K",
            "--output", audio_path.to_str().unwrap(),
            "--no-playlist",
            url,
        ])
        .output()
        .await?;
    
    // Get metadata
    let (title, duration) = self.get_metadata(url).await?;
    
    // Transcribe
    let transcript = self.transcribe_with_whisper(&audio_path, api_key).await?;
    
    // Cleanup (automatic via Drop)
    tokio::fs::remove_file(&audio_path).await.ok();
    
    Ok((title, transcript, duration))
}
```

**Limits:**
- 3 concurrent downloads (semaphore)
- No video length limit (Whisper handles)
- 16K audio compression (saves bandwidth)

**Requirements:**
- `yt-dlp` installed (`pip install yt-dlp`)
- `OPENAI_API_KEY` for Whisper API

**Temp files:**
- Location: `/tmp/synth_video/`
- Naming: `{uuid}.mp3`
- Cleanup: On success + auto-cleanup of 24h+ old files on startup

---

### 4. Audio Extractor (`audio.rs`)

**Handles:** `.mp3`, `.wav`, `.m4a`, `.ogg`, `.flac`, `.aac`

**Detection:**
```rust
fn can_handle(&self, url: &str) -> bool {
    let audio_extensions = [".mp3", ".wav", ".m4a", ".ogg", ".flac", ".aac"];
    let url_lower = url.to_lowercase();
    audio_extensions.iter().any(|ext| url_lower.ends_with(ext))
}
```

**Process:**
1. Download audio file
2. Save to temp file (`/tmp/synth_audio/{uuid}.{ext}`)
3. Transcribe with OpenAI Whisper API
4. Extract title from filename
5. Delete temp file

**Key code:**
```rust
async fn transcribe(&self, audio_path: &PathBuf, api_key: &str) -> Result<String> {
    let file_bytes = tokio::fs::read(audio_path).await?;
    let filename = audio_path.file_name()...to_string();
    
    let form = reqwest::multipart::Form::new()
        .text("model", "whisper-1")
        .part("file", reqwest::multipart::Part::bytes(file_bytes)
            .file_name(filename)
            .mime_str("audio/mpeg")?);
    
    let response = client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await?;
    
    let whisper_response: WhisperResponse = response.json().await?;
    Ok(whisper_response.text)
}
```

**Limits:**
- Whisper API file size limits (25 MB)
- 120 second timeout

**Requirements:**
- `OPENAI_API_KEY` for Whisper API

---

### 5. Image Extractor (`image.rs`)

**Handles:** `.jpg`, `.jpeg`, `.png`, `.gif`, `.webp`, `.bmp`

**Detection:**
```rust
fn can_handle(&self, url: &str) -> bool {
    let image_extensions = [".jpg", ".jpeg", ".png", ".gif", ".webp", ".bmp"];
    let url_lower = url.to_lowercase();
    image_extensions.iter().any(|ext| url_lower.ends_with(ext))
}
```

**Process:**
1. Download image
2. Detect media type from Content-Type header or URL
3. Encode to base64
4. Send to Claude Vision API with analysis prompt
5. Extract dimensions using `image` crate

**Key code:**
```rust
async fn analyze_image(&self, image_bytes: &[u8], media_type: &str, api_key: &str) -> Result<String> {
    let base64_image = general_purpose::STANDARD.encode(image_bytes);
    
    let request = ClaudeVisionRequest {
        model: "claude-sonnet-4-20250514".to_string(),
        max_tokens: 2048,
        messages: vec![ClaudeMessage {
            role: "user".to_string(),
            content: vec![
                ContentPart::Image {
                    source: ImageSource {
                        source_type: "base64".to_string(),
                        media_type: media_type.to_string(),
                        data: base64_image,
                    },
                },
                ContentPart::Text {
                    text: "Describe this image in detail. Include: what you see, any text present, key objects, colors, composition, and any notable features. If it's a chart/graph/diagram, explain what it shows.".to_string(),
                },
            ],
        }],
    };
    
    let response = self.client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&request)
        .send()
        .await?;
    
    let claude_response: ClaudeResponse = response.json().await?;
    Ok(claude_response.content.first()?.text.clone())
}
```

**Limits:**
- Image size depends on Claude API limits
- 60 second timeout

**Requirements:**
- `ANTHROPIC_API_KEY` (same as main analysis)

**Metadata:**
- Dimensions (width × height) via `image` crate
- File size
- Media type (image/jpeg, image/png, etc.)

---

## Router Implementation

**File:** `src/extractors/mod.rs`

**Priority order** (first match wins):

```rust
pub fn new() -> Self {
    let extractors: Vec<Box<dyn ContentExtractor>> = vec![
        Box::new(pdf::PdfExtractor::new()),      // 1. PDF
        Box::new(video::VideoExtractor::new()),   // 2. Video
        Box::new(image::ImageExtractor::new()),   // 3. Image
        Box::new(audio::AudioExtractor::new()),   // 4. Audio
        Box::new(web::WebExtractor::new()),       // 5. Web (fallback)
    ];
    Self { extractors }
}
```

**Why this order?**
- Specific types before generic (PDF before Web)
- Common types before rare (Video before Audio)
- Web last (catches everything else)

## Caching Integration

All extractors benefit from the same caching:

```rust
pub async fn extract_cached(&self, url: &str, cache: &CacheManager) -> Result<ExtractedContent> {
    let key = cache_key(url);  // hash(url)
    
    // Determine cache category and TTL
    let (category, ttl_hours) = match self.detect_type(url) {
        Some(ContentType::PDF) => ("extractors_pdf", 24),
        Some(ContentType::Video) => ("extractors_video", 168),  // 7 days
        Some(ContentType::Audio) => ("extractors_audio", 168),  // 7 days
        Some(ContentType::Image) => ("extractors_image", 24),
        Some(ContentType::Web) | None => ("extractors_web", 24),
    };
    
    // Check cache
    if let Some(cached) = cache.get::<ExtractedContent>(category, &key, ttl_hours).await {
        info!("Extractor cache HIT ({}): {}", category, url);
        return Ok(cached);
    }
    
    info!("Extractor cache MISS ({}): {}", category, url);
    
    // Extract
    let content = self.extract(url).await?;
    
    // Store
    cache.put(category, &key, &content).await.ok();
    
    Ok(content)
}
```

**Cache speedup:**
- First request: Full extraction (seconds to minutes)
- Cached request: <100ms (just disk read)

## Error Handling

### Graceful Degradation

```rust
// In search flow
let extracted_content: Vec<ExtractedContent> = stream::iter(search_results)
    .map(|result| async move {
        extractor.extract_cached(&result.url, cache).await
    })
    .buffer_unordered(10)
    .filter_map(|result| async move {
        match result {
            Ok(content) => Some(content),  // Success
            Err(e) => {
                info!("Extraction failed: {}", e);
                None  // Skip failed extractions
            }
        }
    })
    .collect()
    .await;
```

If 8 out of 10 URLs extract successfully, synthesis continues with 8 sources.

### Common Failures

| Extractor | Common Failures | Handling |
|-----------|----------------|----------|
| Web | Timeout, 404, JavaScript-required | Log + skip |
| PDF | Encrypted, corrupted, scanned | Log + skip |
| Video | yt-dlp not installed, geo-blocked | Log + skip |
| Audio | File too large, unsupported codec | Log + skip |
| Image | Unsupported format, too large | Log + skip |

## Adding New Extractors

1. **Create new file** in `src/extractors/your_type.rs`

```rust
use async_trait::async_trait;
use super::{ContentExtractor, ContentType, ExtractedContent};

pub struct YourExtractor;

#[async_trait]
impl ContentExtractor for YourExtractor {
    fn can_handle(&self, url: &str) -> bool {
        // Detection logic
        url.ends_with(".your_ext")
    }
    
    async fn extract(&self, url: &str) -> Result<ExtractedContent> {
        // Extraction logic
        Ok(ExtractedContent {
            url: url.to_string(),
            title: "Title".to_string(),
            content: "Extracted content".to_string(),
            content_type: ContentType::YourType,
            metadata: None,
        })
    }
    
    fn content_type(&self) -> ContentType {
        ContentType::YourType
    }
}
```

2. **Add ContentType variant**

```rust
pub enum ContentType {
    Web,
    PDF,
    Video,
    Audio,
    Image,
    YourType,  // Add this
}
```

3. **Register in router**

```rust
pub fn new() -> Self {
    let extractors: Vec<Box<dyn ContentExtractor>> = vec![
        Box::new(your_type::YourExtractor::new()),  // Add here
        Box::new(pdf::PdfExtractor::new()),
        // ...
    ];
    Self { extractors }
}
```

4. **Update cache logic** in `extract_cached()`

```rust
let (category, ttl_hours) = match self.detect_type(url) {
    Some(ContentType::YourType) => ("extractors_your_type", 24),
    // ...
};
```

5. **Add cleanup** in `main.rs`

```rust
cache.cleanup("extractors_your_type", 24).await;
```

## Performance Tips

### For Web Pages

- Use content selectors efficiently
- Limit text early (don't process 1MB, then truncate)
- Consider Readability.js for better extraction

### For PDFs

- Stream large PDFs instead of loading into memory
- Use `spawn_blocking` for CPU work
- Consider OCR for scanned PDFs (not implemented)

### For Videos

- Audio-only download (much faster)
- Compress audio (16K is sufficient for Whisper)
- Use semaphore to limit concurrent downloads

### For Audio/Images

- Stream uploads to Whisper/Claude APIs
- Don't load entire file into memory if possible
- Set reasonable timeouts

## Next Steps

- Read [Caching Documentation](../caching/) for cache implementation
- Read [API Documentation](../api/) for how extractors are used
- Read [Development Guide](../development/) for contributing new extractors
