use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, warn};

use super::{ContentExtractor, ContentMetadata, ContentType, ExtractedContent};

/// Universal video extractor (uses yt-dlp for ANY video site)
pub struct VideoExtractor {
    openai_api_key: Option<String>,
    temp_dir: PathBuf,
}

#[derive(Deserialize)]
struct WhisperResponse {
    text: String,
}

impl VideoExtractor {
    pub fn new() -> Self {
        let temp_dir = std::env::temp_dir().join("synth_video");
        std::fs::create_dir_all(&temp_dir).ok();
        
        Self {
            openai_api_key: std::env::var("OPENAI_API_KEY").ok(),
            temp_dir,
        }
    }
    
    /// Check if yt-dlp is available
    async fn check_ytdlp(&self) -> bool {
        Command::new("yt-dlp")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .is_ok()
    }
    
    /// Get video metadata using yt-dlp
    async fn get_metadata(&self, url: &str) -> Result<(String, Option<f64>)> {
        let output = Command::new("yt-dlp")
            .args([
                "--dump-json",
                "--no-playlist",
                url,
            ])
            .output()
            .await
            .context("Failed to get video metadata")?;
        
        let json_str = String::from_utf8(output.stdout)
            .context("Invalid UTF-8 in metadata")?;
        
        let metadata: serde_json::Value = serde_json::from_str(&json_str)
            .context("Failed to parse metadata JSON")?;
        
        let title = metadata["title"]
            .as_str()
            .unwrap_or("Unknown Video")
            .to_string();
        
        let duration = metadata["duration"].as_f64();
        
        Ok((title, duration))
    }
    
    /// Download and transcribe video
    async fn download_and_transcribe(&self, url: &str) -> Result<(String, String, f64)> {
        let temp_id = uuid::Uuid::new_v4();
        let audio_path = self.temp_dir.join(format!("{}.mp3", temp_id));
        
        info!("Downloading video audio: {}", url);
        
        // Download audio only
        let output = Command::new("yt-dlp")
            .args([
                "--extract-audio",
                "--audio-format", "mp3",
                "--audio-quality", "16K", // Compress for Whisper
                "--output", audio_path.to_str().unwrap(),
                "--no-playlist",
                "--js-runtimes", "node",
                url,
            ])
            .output()
            .await
            .context("Failed to download video")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("yt-dlp failed: {}", stderr);
        }
        
        // Get metadata
        let (title, duration) = self.get_metadata(url).await?;
        
        // Transcribe
        let transcript = if let Some(api_key) = &self.openai_api_key {
            info!("Transcribing with OpenAI Whisper...");
            self.transcribe_with_whisper(&audio_path, api_key).await?
        } else {
            warn!("No OPENAI_API_KEY, skipping transcription");
            "[Transcription unavailable - no API key]".to_string()
        };
        
        // Cleanup
        tokio::fs::remove_file(&audio_path).await.ok();
        
        Ok((title, transcript, duration.unwrap_or(0.0)))
    }
    
    /// Transcribe audio using OpenAI Whisper API
    async fn transcribe_with_whisper(&self, audio_path: &PathBuf, api_key: &str) -> Result<String> {
        let file_bytes = tokio::fs::read(audio_path)
            .await
            .context("Failed to read audio file")?;
        
        let form = reqwest::multipart::Form::new()
            .text("model", "whisper-1")
            .part(
                "file",
                reqwest::multipart::Part::bytes(file_bytes)
                    .file_name("audio.mp3")
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

impl Default for VideoExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContentExtractor for VideoExtractor {
    fn can_handle(&self, url: &str) -> bool {
        // Handle common video platforms
        let video_domains = [
            "youtube.com",
            "youtu.be",
            "vimeo.com",
            "dailymotion.com",
            "twitch.tv",
            "tiktok.com",
        ];
        
        video_domains.iter().any(|domain| url.contains(domain))
    }
    
    async fn extract(&self, url: &str) -> Result<ExtractedContent> {
        if !self.check_ytdlp().await {
            anyhow::bail!("yt-dlp not found - install with: pip install yt-dlp");
        }
        
        let (title, transcript, duration) = self.download_and_transcribe(url).await?;
        
        Ok(ExtractedContent {
            url: url.to_string(),
            title: format!("[VIDEO] {}", title),
            content: transcript,
            content_type: ContentType::Video,
            metadata: Some(serde_json::json!({
                "duration_seconds": duration,
                "format": "Video"
            })),
        })
    }
    
    fn content_type(&self) -> ContentType {
        ContentType::Video
    }
}
