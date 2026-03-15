use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, SystemTime};
use tokio::process::Command;
use tokio::sync::Semaphore;
use tracing::{info, warn};
use crate::cache::CacheManager;
use crate::shared::cache_key;

#[derive(Serialize, Deserialize)]
struct CachedYouTubeData {
    title: String,
    transcript: String,
}

const MAX_QUERY_LENGTH: usize = 500;
const MAX_CONCURRENT_DOWNLOADS: usize = 3;
const TEMP_FILE_MAX_AGE_HOURS: u64 = 24;

#[derive(Debug, Clone)]
pub struct YouTubeVideo {
    pub url: String,
    pub title: String,
    pub transcript: String,
}

pub struct YouTubeSearcher {
    openai_api_key: Option<String>,
    download_dir: PathBuf,
    download_semaphore: Semaphore,
}

impl YouTubeSearcher {
    pub fn new(openai_api_key: Option<String>) -> Self {
        let download_dir = std::env::temp_dir().join("osit_youtube");
        std::fs::create_dir_all(&download_dir).ok();
        
        // Clean up old files on startup
        Self::cleanup_old_files(&download_dir);
        
        Self {
            openai_api_key,
            download_dir,
            download_semaphore: Semaphore::new(MAX_CONCURRENT_DOWNLOADS),
        }
    }

    /// Remove files older than TEMP_FILE_MAX_AGE_HOURS
    fn cleanup_old_files(dir: &PathBuf) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            let now = SystemTime::now();
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(age) = now.duration_since(modified) {
                            if age > Duration::from_secs(TEMP_FILE_MAX_AGE_HOURS * 3600) {
                                std::fs::remove_file(entry.path()).ok();
                            }
                        }
                    }
                }
            }
        }
    }

    /// Search YouTube and transcribe top results with caching
    pub async fn search_and_transcribe(&self, query: &str, max_results: usize, cache: &CacheManager) -> Result<Vec<YouTubeVideo>> {
        // Validate and sanitize query
        let sanitized_query = Self::sanitize_query(query)?;
        
        info!("Searching YouTube for: {}", sanitized_query);

        // Check if yt-dlp is available
        if !self.check_ytdlp().await {
            warn!("yt-dlp not found, skipping YouTube search");
            return Ok(vec![]);
        }

        // Search YouTube for videos
        let video_urls = self.search_youtube(&sanitized_query, max_results).await?;
        
        if video_urls.is_empty() {
            return Ok(vec![]);
        }

        info!("Found {} YouTube videos, downloading and transcribing...", video_urls.len());

        // Download and transcribe each video (with concurrency limit + caching)
        let mut transcribed_videos = Vec::new();
        
        for url in video_urls.iter().take(max_results) {
            let video_key = cache_key(&url);
            
            // Check cache first
            if let Some(cached) = cache.get::<CachedYouTubeData>("youtube", &video_key, 168).await { // 7 days TTL
                info!("YouTube cache HIT: {}", url);
                transcribed_videos.push(YouTubeVideo {
                    url: url.clone(),
                    title: cached.title,
                    transcript: cached.transcript,
                });
                continue;
            }
            
            info!("YouTube cache MISS, downloading: {}", url);
            
            // Acquire semaphore permit (limits concurrent downloads)
            let _permit = self.download_semaphore.acquire().await
                .context("Failed to acquire download permit")?;
            
            match self.download_and_transcribe(url).await {
                Ok(video) => {
                    info!("Transcribed: {}", video.title);
                    
                    // Store in cache
                    let cached_data = CachedYouTubeData {
                        title: video.title.clone(),
                        transcript: video.transcript.clone(),
                    };
                    cache.put("youtube", &video_key, cached_data).await.ok();
                    
                    transcribed_videos.push(video);
                }
                Err(e) => {
                    warn!("Failed to process {}: {}", url, e);
                }
            }
            // Permit automatically released when _permit drops
        }

        Ok(transcribed_videos)
    }

    /// Sanitize and validate query input
    fn sanitize_query(query: &str) -> Result<String> {
        // Enforce length limit
        if query.len() > MAX_QUERY_LENGTH {
            anyhow::bail!("Query too long (max {} chars)", MAX_QUERY_LENGTH);
        }

        // Remove potentially dangerous characters
        let sanitized: String = query
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace() || matches!(c, '-' | '_' | '.' | '?' | '!'))
            .collect();

        if sanitized.trim().is_empty() {
            anyhow::bail!("Query is empty after sanitization");
        }

        Ok(sanitized)
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
        
        // Ensure cleanup happens even on error
        let _cleanup_guard = CleanupGuard::new(audio_path.clone());
        
        // Transcribe with Whisper
        let transcript = if let Some(ref api_key) = self.openai_api_key {
            self.transcribe_with_api(&audio_path, api_key).await?
        } else {
            self.transcribe_with_local(&audio_path).await?
        };

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

/// Cleanup guard ensures file deletion even on error
struct CleanupGuard {
    path: PathBuf,
}

impl CleanupGuard {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        // Sync file deletion in destructor (best effort)
        std::fs::remove_file(&self.path).ok();
    }
}
