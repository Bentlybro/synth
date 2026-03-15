use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::fs;
use tracing::{info, warn};

use crate::shared::cache_key_multi;

/// Centralized cache manager for all content types
#[derive(Clone)]
pub struct CacheManager {
    root: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct CachedItem<T> {
    data: T,
    cached_at: u64,
}

impl CacheManager {
    pub fn new(root: PathBuf) -> Self {
        std::fs::create_dir_all(&root).ok();
        Self { root }
    }

    /// Get cached item if fresh
    pub async fn get<T>(&self, category: &str, key: &str, ttl_hours: u64) -> Option<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let path = self.cache_path(category, key);
        
        if !path.exists() {
            return None;
        }

        let content = fs::read_to_string(&path).await.ok()?;
        let cached: CachedItem<T> = serde_json::from_str(&content).ok()?;

        // Check freshness
        let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
        let age_hours = (now - cached.cached_at) / 3600;

        if age_hours > ttl_hours {
            // Expired
            fs::remove_file(&path).await.ok();
            return None;
        }

        info!("Cache HIT: {}/{}", category, key);
        Some(cached.data)
    }

    /// Store item in cache
    pub async fn put<T>(&self, category: &str, key: &str, data: T) -> Result<()>
    where
        T: Serialize,
    {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        
        let cached = CachedItem {
            data,
            cached_at: now,
        };

        let path = self.cache_path(category, key);
        std::fs::create_dir_all(path.parent().unwrap()).ok();
        
        let json = serde_json::to_string_pretty(&cached)?;
        fs::write(&path, json).await?;

        info!("Cache STORED: {}/{}", category, key);
        Ok(())
    }

    fn cache_path(&self, category: &str, key: &str) -> PathBuf {
        self.root.join(category).join(format!("{}.json", key))
    }

    /// Clean up expired items
    pub async fn cleanup(&self, category: &str, ttl_hours: u64) {
        let category_dir = self.root.join(category);
        
        if !category_dir.exists() {
            return;
        }

        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let mut removed = 0;

        if let Ok(entries) = std::fs::read_dir(&category_dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        let modified_secs = modified.duration_since(UNIX_EPOCH).unwrap().as_secs();
                        let age_hours = (now - modified_secs) / 3600;
                        
                        if age_hours > ttl_hours {
                            fs::remove_file(entry.path()).await.ok();
                            removed += 1;
                        }
                    }
                }
            }
        }

        if removed > 0 {
            info!("Cleaned up {} expired cache items from {}", removed, category);
        }
    }
}
