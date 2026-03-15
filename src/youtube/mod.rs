use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct YouTubeVideo {
    pub url: String,
    pub title: String,
    pub transcript: String,
}

pub struct YouTubeSearcher {
    openai_api_key: Option<String>,
    download_dir: PathBuf,
}

impl YouTubeSearcher {
    pub fn new(openai_api_key: Option<String>) -> Self {
        let download_dir = std::env::temp_dir().join("osit_youtube");
        std::fs::create_dir_all(&download_dir).ok();
        
        Self {
            openai_api_key,
            download_dir,
        }
    }

    /// Search YouTube and transcribe top results
    pub async fn search_and_transcribe(&self, query: &str, max_results: usize) -> Result<Vec<YouTubeVideo>> {
        info!("Searching YouTube for: {}", query);

        // Check if yt-dlp is available
        if !self.check_ytdlp().await {
            warn!("yt-dlp not found, skipping YouTube search");
            return Ok(vec![]);
        }

        // Search YouTube for videos
        let video_urls = self.search_youtube(query, max_results).await?;
        
        if video_urls.is_empty() {
            return Ok(vec![]);
        }

        info!("Found {} YouTube videos, downloading and transcribing...", video_urls.len());

        // Download and transcribe each video
        let mut transcribed_videos = Vec::new();
        
        for url in video_urls.iter().take(max_results) {
            match self.download_and_transcribe(url).await {
                Ok(video) => {
                    info!("Transcribed: {}", video.title);
                    transcribed_videos.push(video);
                }
                Err(e) => {
                    warn!("Failed to process {}: {}", url, e);
                }
            }
        }

        Ok(transcribed_videos)
    }

    async fn check_ytdlp(&self) -> bool {
        Command::new("yt-dlp")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .is_ok()
    }

    async fn search_youtube(&self, query: &str, max_results: usize) -> Result<Vec<String>> {
        let output = Command::new("yt-dlp")
            .arg("--js-runtimes")
            .arg("node")
            .arg("--get-id")
            .arg("--max-downloads")
            .arg(max_results.to_string())
            .arg(format!("ytsearch{}:{}", max_results, query))
            .stderr(Stdio::null()) // Suppress warnings
            .output()
            .await
            .context("Failed to search YouTube")?;

        // Parse stdout even if command exited with warnings (non-zero exit code)
        let video_ids: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| !line.is_empty() && line.len() == 11) // YouTube IDs are 11 chars
            .map(|id| format!("https://www.youtube.com/watch?v={}", id.trim()))
            .collect();

        if video_ids.is_empty() {
            anyhow::bail!("No video IDs found");
        }

        Ok(video_ids)
    }

    async fn download_and_transcribe(&self, url: &str) -> Result<YouTubeVideo> {
        // Get video title
        let title = self.get_video_title(url).await?;
        
        // Download audio
        let audio_path = self.download_audio(url).await?;
        
        // Transcribe with Whisper
        let transcript = if let Some(ref api_key) = self.openai_api_key {
            self.transcribe_with_api(&audio_path, api_key).await?
        } else {
            self.transcribe_with_local(&audio_path).await?
        };

        // Clean up audio file
        tokio::fs::remove_file(&audio_path).await.ok();

        Ok(YouTubeVideo {
            url: url.to_string(),
            title,
            transcript,
        })
    }

    async fn get_video_title(&self, url: &str) -> Result<String> {
        let output = Command::new("yt-dlp")
            .arg("--get-title")
            .arg(url)
            .output()
            .await
            .context("Failed to get video title")?;

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    async fn download_audio(&self, url: &str) -> Result<PathBuf> {
        let filename = format!("{}.mp3", uuid::Uuid::new_v4());
        let output_path = self.download_dir.join(&filename);

        let status = Command::new("yt-dlp")
            .arg("--js-runtimes")
            .arg("node")
            .arg("-x")
            .arg("--audio-format")
            .arg("mp3")
            .arg("--postprocessor-args")
            .arg("-ar 16000 -ac 1") // Compress: 16kHz mono (reduces file size)
            .arg("-o")
            .arg(&output_path)
            .arg(url)
            .stderr(Stdio::null()) // Suppress warnings
            .status()
            .await
            .context("Failed to download audio")?;

        if !status.success() {
            anyhow::bail!("yt-dlp download failed");
        }

        // Check file size (OpenAI Whisper limit is 25MB)
        let metadata = tokio::fs::metadata(&output_path).await?;
        if metadata.len() > 25 * 1024 * 1024 {
            warn!("Audio file too large ({} bytes), skipping transcription", metadata.len());
            tokio::fs::remove_file(&output_path).await.ok();
            anyhow::bail!("Audio file exceeds 25MB limit");
        }

        Ok(output_path)
    }

    async fn transcribe_with_api(&self, audio_path: &PathBuf, api_key: &str) -> Result<String> {
        use reqwest::multipart;
        
        let client = reqwest::Client::new();
        let audio_file = tokio::fs::read(audio_path).await?;
        
        let part = multipart::Part::bytes(audio_file)
            .file_name("audio.mp3")
            .mime_str("audio/mpeg")?;
        
        let form = multipart::Form::new()
            .part("file", part)
            .text("model", "whisper-1");

        let response = client
            .post("https://api.openai.com/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", api_key))
            .multipart(form)
            .send()
            .await
            .context("Failed to call Whisper API")?;

        #[derive(Deserialize)]
        struct WhisperResponse {
            text: String,
        }

        let whisper_response: WhisperResponse = response
            .json()
            .await
            .context("Failed to parse Whisper response")?;

        Ok(whisper_response.text)
    }

    async fn transcribe_with_local(&self, audio_path: &PathBuf) -> Result<String> {
        // Try using whisper command-line tool if available
        let output = Command::new("whisper")
            .arg(audio_path)
            .arg("--model")
            .arg("base")
            .arg("--output_format")
            .arg("txt")
            .output()
            .await;

        match output {
            Ok(out) if out.status.success() => {
                Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
            }
            _ => {
                warn!("Local whisper not available, skipping transcription");
                Ok(String::from("(Transcription unavailable - no OpenAI API key or local Whisper)"))
            }
        }
    }
}
