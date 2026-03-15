mod manager;

pub use manager::CacheManager;

use anyhow::Result;
use std::sync::Arc;
use crate::index::{IndexedPage, SearchIndex, IndexStats};

/// Legacy PageCache wrapper for Tantivy index
/// (Kept for stats endpoint, will eventually migrate)
pub struct PageCache {
    pub index: Arc<SearchIndex>,
    ttl_seconds: i64,
}

impl PageCache {
    pub fn new(index: Arc<SearchIndex>, ttl_seconds: i64) -> Self {
        Self { index, ttl_seconds }
    }

    /// Store scraped page in index
    pub async fn put(&self, page: IndexedPage) -> Result<()> {
        self.index.add_page(page)?;
        self.index.commit()?;
        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> Result<IndexStats> {
        self.index.stats()
    }
}
