use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::path::PathBuf;
use tracing::{info, warn};

use super::{ContentExtractor, ContentMetadata, ContentType, ExtractedContent};

/// Audio file extractor and transcriber
pub struct AudioExtractor {
    client: Client,
    openai_api_key: Option<String>,
    temp_dir: PathBuf,
}

#[derive(Deserialize)]
struct WhisperResponse {
    text: String,
}

impl AudioExtractor {
    pub fn new() -> Self {
        let temp_dir = std::env::temp_dir().join("synth_audio");
        std::fs::create_dir_all(&temp_dir).ok();
        
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (compatible; SynthBot/1.0)")
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap(),
            openai_api_key: std::env::var("OPENAI_API_KEY").ok(),
            temp_dir,
        }
    }
    
    /// Download audio file
    async fn download_audio(&self, url: &str) -> Result<(PathBuf, u64)> {
        info!("Downloading audio: {}", url);
        
        let response = self.client
            .get(url)
            .send()
            .await
            .context("Failed to download audio")?;
        
        let file_bytes = response.bytes().await.context("Failed to read audio bytes")?;
        let file_size = file_bytes.len() as u64;
        
        // Save to temp file
        let temp_id = uuid::Uuid::new_v4();
        let extension = self.detect_extension(url);
        let temp_path = self.temp_dir.join(format!("{}.{}", temp_id, extension));
        
        tokio::fs::write(&temp_path, &file_bytes)
            .await
            .context("Failed to write audio file")?;
        
        Ok((temp_path, file_size))
    }
    
    /// Detect audio file extension from URL
    fn detect_extension(&self, url: &str) -> &str {
        let url_lower = url.to_lowercase();
        if url_lower.ends_with(".mp3") {
            "mp3"
        } else if url_lower.ends_with(".wav") {
            "wav"
        } else if url_lower.ends_with(".m4a") {
            "m4a"
        } else if url_lower.ends_with(".ogg") {
            "ogg"
        } else if url_lower.ends_with(".flac") {
            "flac"
        } else {
            "audio" // Generic fallback
        }
    }
    
    /// Extract title from URL
    fn extract_title(&self, url: &str) -> String {
        if let Some(filename) = url.split('/').last() {
            let title = filename
                .split('.')
                .next()
                .unwrap_or("Audio")
                .replace('-', " ")
                .replace('_', " ");
            return title;
        }
        url.to_string()
    }
    
    /// Transcribe audio using OpenAI Whisper API
    async fn transcribe(&self, audio_path: &PathBuf, api_key: &str) -> Result<String> {
        info!("Transcribing audio with Whisper...");
        
        let file_bytes = tokio::fs::read(audio_path)
            .await
            .context("Failed to read audio file")?;
        
        let filename = audio_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "audio.mp3".to_string());
        
        let form = reqwest::multipart::Form::new()
            .text("model", "whisper-1")
            .part(
                "file",
                reqwest::multipart::Part::bytes(file_bytes)
                    .file_name(filename.clone())
                    .mime_str("audio/mpeg")?,
            );
        
        let client = reqwest::Client::new();
        let response = client
            .post("https://api.openai.com/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", api_key))
            .multipart(form)
            .send()
            .await
            .context("Whisper API request failed")?;
        
        let whisper_response: WhisperResponse = response
            .json()
            .await
            .context("Failed to parse Whisper response")?;
        
        Ok(whisper_response.text)
    }
}

impl Default for AudioExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContentExtractor for AudioExtractor {
    fn can_handle(&self, url: &str) -> bool {
        let audio_extensions = [".mp3", ".wav", ".m4a", ".ogg", ".flac", ".aac"];
        let url_lower = url.to_lowercase();
        audio_extensions.iter().any(|ext| url_lower.ends_with(ext))
    }
    
    async fn extract(&self, url: &str) -> Result<ExtractedContent> {
        let (audio_path, file_size) = self.download_audio(url).await?;
        
        let transcript = if let Some(api_key) = &self.openai_api_key {
            match self.transcribe(&audio_path, api_key).await {
                Ok(text) => text,
                Err(e) => {
                    warn!("Transcription failed: {}", e);
                    format!("[Transcription failed: {}]", e)
                }
            }
        } else {
            warn!("No OPENAI_API_KEY, skipping transcription");
            "[Transcription unavailable - no API key]".to_string()
        };
        
        // Cleanup
        tokio::fs::remove_file(&audio_path).await.ok();
        
        let title = self.extract_title(url);
        
        Ok(ExtractedContent {
            url: url.to_string(),
            title: format!("[AUDIO] {}", title),
            content: transcript,
            content_type: ContentType::Audio,
            metadata: Some(ContentMetadata {
                duration_seconds: None,
                file_size_bytes: Some(file_size),
                format: Some(self.detect_extension(url).to_uppercase()),
                dimensions: None,
            }),
        })
    }
    
    fn content_type(&self) -> ContentType {
        ContentType::Audio
    }
}
