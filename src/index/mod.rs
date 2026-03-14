use anyhow::{Context, Result};
use chrono::Utc;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{doc, Index, IndexWriter, ReloadPolicy};
use std::path::Path;
use std::sync::{Arc, RwLock};

pub struct SearchIndex {
    index: Index,
    writer: Arc<RwLock<IndexWriter>>,
    schema: Schema,
    url_field: Field,
    title_field: Field,
    content_field: Field,
    domain_field: Field,
    timestamp_field: Field,
}

#[derive(Debug, Clone)]
pub struct IndexedPage {
    pub url: String,
    pub title: String,
    pub content: String,
    pub domain: String,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub url: String,
    pub title: String,
    pub snippet: String,
    pub score: f32,
}

impl SearchIndex {
    pub fn new<P: AsRef<Path>>(index_path: P) -> Result<Self> {
        // Create schema
        let mut schema_builder = Schema::builder();
        
        let url_field = schema_builder.add_text_field("url", STRING | STORED);
        let title_field = schema_builder.add_text_field("title", TEXT | STORED);
        let content_field = schema_builder.add_text_field("content", TEXT);
        let domain_field = schema_builder.add_text_field("domain", STRING | STORED);
        let timestamp_field = schema_builder.add_i64_field("timestamp", INDEXED | STORED);
        
        let schema = schema_builder.build();

        // Create or open index
        let index = if index_path.as_ref().exists() {
            Index::open_in_dir(index_path)?
        } else {
            std::fs::create_dir_all(&index_path)?;
            Index::create_in_dir(index_path, schema.clone())?
        };

        // Create writer with 50MB heap
        let writer = index.writer(50_000_000)?;

        Ok(Self {
            index,
            writer: Arc::new(RwLock::new(writer)),
            schema,
            url_field,
            title_field,
            content_field,
            domain_field,
            timestamp_field,
        })
    }

    /// Add a page to the index
    pub fn add_page(&self, page: IndexedPage) -> Result<()> {
        let mut writer = self.writer.write().unwrap();
        
        let doc = doc!(
            self.url_field => page.url,
            self.title_field => page.title,
            self.content_field => page.content,
            self.domain_field => page.domain,
            self.timestamp_field => Utc::now().timestamp(),
        );

        writer.add_document(doc)?;
        Ok(())
    }

    /// Commit all pending changes
    pub fn commit(&self) -> Result<()> {
        let mut writer = self.writer.write().unwrap();
        writer.commit()?;
        Ok(())
    }

    /// Search the index
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let reader = self.index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        let searcher = reader.searcher();

        // Parse query across title and content
        let query_parser = QueryParser::for_index(
            &self.index,
            vec![self.title_field, self.content_field],
        );

        let query = query_parser.parse_query(query)?;

        // Search
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        // Convert results
        let results: Result<Vec<_>> = top_docs
            .iter()
            .map(|(score, doc_address)| {
                let retrieved_doc = searcher.doc(*doc_address)?;
                
                let url = retrieved_doc
                    .get_first(self.url_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let title = retrieved_doc
                    .get_first(self.title_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                // Create snippet from title (we'll enhance this later)
                let snippet = title.clone();

                Ok(SearchResult {
                    url,
                    title,
                    snippet,
                    score: *score,
                })
            })
            .collect();

        results
    }

    /// Get index statistics
    pub fn stats(&self) -> Result<IndexStats> {
        let reader = self.index.reader()?;
        let searcher = reader.searcher();
        
        Ok(IndexStats {
            num_docs: searcher.num_docs() as usize,
        })
    }
}

#[derive(Debug, Clone)]
pub struct IndexStats {
    pub num_docs: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_index_basic() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let index = SearchIndex::new(temp_dir.path())?;

        // Add a test page
        index.add_page(IndexedPage {
            url: "https://example.com".to_string(),
            title: "Example Page".to_string(),
            content: "This is example content about Rust programming.".to_string(),
            domain: "example.com".to_string(),
        })?;

        index.commit()?;

        // Search
        let results = index.search("Rust", 10)?;
        assert!(!results.is_empty());
        assert_eq!(results[0].url, "https://example.com");

        Ok(())
    }
}
