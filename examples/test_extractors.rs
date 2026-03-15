use synth::extractors::{ExtractorRouter, ContentType};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize router
    let router = ExtractorRouter::new();
    
    // Test URL detection
    let test_urls = vec![
        "https://example.com/page.html",
        "https://example.com/document.pdf",
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
        "https://example.com/audio.mp3",
        "https://example.com/image.jpg",
    ];
    
    println!("=== Content Type Detection ===\n");
    for url in test_urls {
        if let Some(content_type) = router.detect_type(url) {
            println!("{:?}: {}", content_type, url);
        }
    }
    
    Ok(())
}
