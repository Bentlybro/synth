# Extractors - Universal Content Ingestion

Modular content extraction system that automatically routes URLs to the appropriate handler.

## Architecture

```
URL → ExtractorRouter → Detect Type → Route to Extractor → Extract Content → Cache → LLM Analysis
```

## Supported Content Types

| Type | Extensions | Extractors Used | Output |
|------|-----------|----------------|--------|
| **PDF** | `.pdf` | pdf-extract | Text content |
| **Video** | YouTube, Vimeo, TikTok, etc. | yt-dlp + Whisper API | Transcript |
| **Audio** | `.mp3`, `.wav`, `.m4a`, `.ogg`, `.flac` | Whisper API | Transcript |
| **Image** | `.jpg`, `.png`, `.gif`, `.webp` | Claude Vision API | Description |
| **Web** | Any HTTP/HTTPS | HTML scraper | Main content |

## How It Works

### 1. Automatic Routing

```rust
let router = ExtractorRouter::new();
let content = router.extract("https://example.com/document.pdf").await?;
```

The router automatically:
1. Detects content type from URL/extension
2. Routes to appropriate extractor
3. Returns unified `ExtractedContent` struct

### 2. Unified Output

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

### 3. Caching Integration

All extractors work with the CacheManager:
- PDFs cached by URL (24h TTL)
- Videos cached by URL (7 day TTL)
- Audio cached by URL (7 day TTL)
- Images cached by URL (24h TTL)
- Web pages cached by URL (24h TTL)

## Individual Extractors

### Web Extractor (`web.rs`)

Scrapes HTML pages and extracts main content.

**Features:**
- Intelligent content detection (article, main, .content selectors)
- Title extraction (title tag, og:title)
- Text cleaning and normalization
- 50k character limit

**Example:**
```
Input:  https://example.com/blog/post
Output: {
  title: "Blog Post Title",
  content: "Main article content...",
  content_type: Web
}
```

### PDF Extractor (`pdf.rs`)

Extracts text from PDF documents.

**Features:**
- Uses `pdf-extract` crate
- Handles multi-page documents
- Title extraction from filename
- File size metadata
- 100k character limit

**Example:**
```
Input:  https://example.com/paper.pdf
Output: {
  title: "paper",
  content: "Extracted PDF text...",
  content_type: PDF,
  metadata: { file_size_bytes: 1024000 }
}
```

### Video Extractor (`video.rs`)

Downloads and transcribes videos from any supported site.

**Features:**
- Uses `yt-dlp` for universal video download
- Supports: YouTube, Vimeo, TikTok, Twitch, Dailymotion, etc.
- Audio-only download (16K compression)
- OpenAI Whisper API transcription
- Duration metadata

**Example:**
```
Input:  https://www.youtube.com/watch?v=dQw4w9WgXcQ
Output: {
  title: "[VIDEO] Never Gonna Give You Up",
  content: "Full video transcript...",
  content_type: Video,
  metadata: { duration_seconds: 213.0 }
}
```

**Requirements:**
- `yt-dlp` installed (`pip install yt-dlp`)
- `OPENAI_API_KEY` environment variable

### Audio Extractor (`audio.rs`)

Downloads and transcribes audio files.

**Features:**
- Supports: MP3, WAV, M4A, OGG, FLAC, AAC
- Downloads to temp directory
- OpenAI Whisper API transcription
- Auto-cleanup after transcription
- File size metadata

**Example:**
```
Input:  https://example.com/podcast.mp3
Output: {
  title: "[AUDIO] podcast",
  content: "Transcribed audio...",
  content_type: Audio,
  metadata: { file_size_bytes: 5242880, format: "MP3" }
}
```

**Requirements:**
- `OPENAI_API_KEY` environment variable

### Image Extractor (`image.rs`)

Analyzes images using Claude's vision capabilities.

**Features:**
- Supports: JPG, PNG, GIF, WebP, BMP
- Downloads image
- Claude Vision API analysis
- Describes content, text, objects, colors
- Dimension metadata (width × height)

**Example:**
```
Input:  https://example.com/chart.png
Output: {
  title: "[IMAGE] chart",
  content: "This image shows a bar chart comparing...",
  content_type: Image,
  metadata: { 
    file_size_bytes: 102400,
    format: "image/png",
    dimensions: (800, 600)
  }
}
```

**Requirements:**
- `ANTHROPIC_API_KEY` environment variable

## Adding New Extractors

To add a new content type:

1. Create `src/extractors/my_type.rs`:

```rust
use async_trait::async_trait;
use super::{ContentExtractor, ContentType, ExtractedContent};

pub struct MyExtractor;

#[async_trait]
impl ContentExtractor for MyExtractor {
    fn can_handle(&self, url: &str) -> bool {
        // Return true if this extractor can handle the URL
        url.ends_with(".myext")
    }
    
    async fn extract(&self, url: &str) -> Result<ExtractedContent> {
        // Extract content
        Ok(ExtractedContent {
            url: url.to_string(),
            title: "Title".to_string(),
            content: "Extracted content".to_string(),
            content_type: ContentType::MyType,
            metadata: None,
        })
    }
    
    fn content_type(&self) -> ContentType {
        ContentType::MyType
    }
}
```

2. Add to `ContentType` enum in `mod.rs`
3. Register in `ExtractorRouter::new()`

## Error Handling

All extractors return `Result<ExtractedContent>`:
- Network errors → propagated
- Extraction failures → logged, fallback content returned
- Missing API keys → logged, placeholder content returned

## Performance

**Extraction times:**
- Web pages: ~2-5 seconds
- PDFs: ~3-10 seconds (depends on size)
- Videos: ~30-90 seconds (download + transcription)
- Audio: ~10-30 seconds (download + transcription)
- Images: ~5-10 seconds (download + vision analysis)

**Caching impact:**
- First extraction: Full time
- Cached extraction: ~instant (< 100ms)

## Future Enhancements

- [ ] Word documents (.docx)
- [ ] Spreadsheets (.xlsx, .csv)
- [ ] Presentations (.pptx)
- [ ] Archives (.zip content indexing)
- [ ] Code repositories (GitHub/GitLab analysis)
- [ ] Email parsing (.eml, .msg)
- [ ] Markdown files
- [ ] RSS/Atom feeds
- [ ] JSON/XML data parsing
