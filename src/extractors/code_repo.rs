use super::{ContentExtractor, ContentType, ExtractedContent};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use git2::Repository;
use ignore::WalkBuilder;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Maximum total size to analyze (50 MB)
const MAX_TOTAL_SIZE: usize = 50 * 1024 * 1024;

/// Maximum single file size (1 MB)
const MAX_FILE_SIZE: usize = 1024 * 1024;

/// Maximum files to analyze in basic mode
const MAX_FILES_BASIC: usize = 20;

/// Maximum files to analyze in deep mode
const MAX_FILES_DEEP: usize = 100;

pub struct CodeRepoExtractor;

impl CodeRepoExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Parse GitHub URL to extract owner/repo
    fn parse_github_url(&self, url: &str) -> Option<(String, String)> {
        // Support formats:
        // - https://github.com/owner/repo
        // - https://github.com/owner/repo.git
        // - https://github.com/owner/repo/tree/branch
        
        let url = url.trim_end_matches('/');
        
        if let Some(path) = url.strip_prefix("https://github.com/") {
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() >= 2 {
                let owner = parts[0].to_string();
                let repo = parts[1].trim_end_matches(".git").to_string();
                return Some((owner, repo));
            }
        }
        
        None
    }

    /// Clone repository to temp directory
    async fn clone_repo(&self, url: &str) -> Result<(PathBuf, String)> {
        let temp_dir = std::env::temp_dir().join(format!("synth-repo-{}", Uuid::new_v4()));
        
        info!("Cloning repository: {} to {:?}", url, temp_dir);
        
        // Clone with depth=1 for speed
        let repo = Repository::clone(url, &temp_dir)
            .map_err(|e| anyhow!("Failed to clone repository: {}", e))?;
        
        // Get HEAD commit hash
        let head = repo.head()?;
        let commit = head.peel_to_commit()?;
        let commit_hash = commit.id().to_string();
        
        debug!("Cloned at commit: {}", commit_hash);
        
        Ok((temp_dir, commit_hash))
    }

    /// Detect primary language from file extensions
    fn detect_language(&self, repo_path: &Path) -> String {
        let mut lang_counts: HashMap<&str, usize> = HashMap::new();
        
        let extensions = [
            ("rs", "Rust"),
            ("ts", "TypeScript"),
            ("tsx", "TypeScript"),
            ("js", "JavaScript"),
            ("jsx", "JavaScript"),
            ("py", "Python"),
            ("go", "Go"),
            ("java", "Java"),
            ("c", "C"),
            ("cpp", "C++"),
            ("cc", "C++"),
            ("h", "C/C++"),
            ("hpp", "C++"),
            ("cs", "C#"),
            ("rb", "Ruby"),
            ("php", "PHP"),
            ("swift", "Swift"),
            ("kt", "Kotlin"),
            ("scala", "Scala"),
        ];
        
        for entry in WalkBuilder::new(repo_path)
            .max_depth(Some(3))
            .build()
            .flatten()
        {
            if let Some(ext) = entry.path().extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                for (pattern, lang) in &extensions {
                    if ext_str == *pattern {
                        *lang_counts.entry(lang).or_insert(0) += 1;
                        break;
                    }
                }
            }
        }
        
        lang_counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(lang, _)| lang.to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    }

    /// Check if file is important (README, docs, etc.)
    fn is_important_file(&self, path: &Path) -> bool {
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        // Important files
        file_name.starts_with("readme")
            || file_name.starts_with("contributing")
            || file_name.starts_with("license")
            || file_name.starts_with("changelog")
            || file_name == "cargo.toml"
            || file_name == "package.json"
            || file_name == "go.mod"
            || file_name == "requirements.txt"
            || file_name == "pyproject.toml"
            || file_name == "pom.xml"
            || file_name == "build.gradle"
    }

    /// Check if file is likely an entry point
    fn is_entry_point(&self, path: &Path) -> bool {
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        // Entry point files
        file_name == "main.rs"
            || file_name == "lib.rs"
            || file_name == "index.ts"
            || file_name == "index.js"
            || file_name == "main.py"
            || file_name == "__init__.py"
            || file_name == "main.go"
            || file_name == "app.py"
            || file_name == "server.ts"
            || file_name == "server.js"
    }

    /// Select files to analyze
    fn select_files(&self, repo_path: &Path, deep: bool) -> Result<Vec<PathBuf>> {
        let max_files = if deep { MAX_FILES_DEEP } else { MAX_FILES_BASIC };
        let mut selected = Vec::new();
        let mut total_size = 0usize;
        
        // First pass: important files and entry points
        let mut important = Vec::new();
        let mut entry_points = Vec::new();
        let mut source_files = Vec::new();
        
        for entry in WalkBuilder::new(repo_path).build().flatten() {
            let path = entry.path();
            
            // Skip if not a file
            if !path.is_file() {
                continue;
            }
            
            // Skip large files
            if let Ok(metadata) = std::fs::metadata(path) {
                if metadata.len() > MAX_FILE_SIZE as u64 {
                    continue;
                }
            }
            
            // Skip binary files (simple heuristic)
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if matches!(
                    ext_str.as_str(),
                    "png" | "jpg" | "jpeg" | "gif" | "pdf" | "zip" | "tar" | "gz" | "bin" | "so" | "dylib" | "dll"
                ) {
                    continue;
                }
            }
            
            let relative = path.strip_prefix(repo_path).unwrap_or(path);
            
            if self.is_important_file(path) {
                important.push(relative.to_path_buf());
            } else if self.is_entry_point(path) {
                entry_points.push(relative.to_path_buf());
            } else if path.extension().is_some() {
                source_files.push(relative.to_path_buf());
            }
        }
        
        // Add important files first
        for path in important {
            let full_path = repo_path.join(&path);
            if let Ok(metadata) = std::fs::metadata(&full_path) {
                let size = metadata.len() as usize;
                if total_size + size > MAX_TOTAL_SIZE {
                    break;
                }
                selected.push(path);
                total_size += size;
            }
        }
        
        // Add entry points
        for path in entry_points {
            if selected.len() >= max_files {
                break;
            }
            let full_path = repo_path.join(&path);
            if let Ok(metadata) = std::fs::metadata(&full_path) {
                let size = metadata.len() as usize;
                if total_size + size > MAX_TOTAL_SIZE {
                    break;
                }
                selected.push(path);
                total_size += size;
            }
        }
        
        // Add source files (sample)
        let sample_size = (max_files - selected.len()).min(source_files.len());
        for path in source_files.iter().take(sample_size) {
            let full_path = repo_path.join(path);
            if let Ok(metadata) = std::fs::metadata(&full_path) {
                let size = metadata.len() as usize;
                if total_size + size > MAX_TOTAL_SIZE {
                    break;
                }
                selected.push(path.clone());
                total_size += size;
            }
        }
        
        info!("Selected {} files ({} bytes)", selected.len(), total_size);
        
        Ok(selected)
    }

    /// Generate tree structure
    fn generate_tree(&self, repo_path: &Path, max_depth: usize) -> String {
        let mut lines = Vec::new();
        
        fn walk_tree(
            path: &Path,
            prefix: &str,
            depth: usize,
            max_depth: usize,
            lines: &mut Vec<String>,
        ) {
            if depth > max_depth {
                return;
            }
            
            let mut entries: Vec<_> = std::fs::read_dir(path)
                .map(|entries| entries.flatten().collect())
                .unwrap_or_default();
            
            entries.sort_by_key(|e| e.path());
            
            for (i, entry) in entries.iter().enumerate() {
                let is_last = i == entries.len() - 1;
                let name = entry.file_name().to_string_lossy().to_string();
                
                // Skip hidden files and common directories
                if name.starts_with('.')
                    || name == "node_modules"
                    || name == "target"
                    || name == "build"
                    || name == "dist"
                {
                    continue;
                }
                
                let connector = if is_last { "└── " } else { "├── " };
                let line = format!("{}{}{}", prefix, connector, name);
                lines.push(line);
                
                if entry.path().is_dir() {
                    let new_prefix = format!(
                        "{}{}",
                        prefix,
                        if is_last { "    " } else { "│   " }
                    );
                    walk_tree(&entry.path(), &new_prefix, depth + 1, max_depth, lines);
                }
            }
        }
        
        walk_tree(repo_path, "", 0, max_depth, &mut lines);
        lines.join("\n")
    }

    /// Extract repository content
    async fn extract_repo_content(
        &self,
        repo_path: &Path,
        files: &[PathBuf],
        deep: bool,
    ) -> Result<String> {
        let mut content = String::new();
        
        // Add tree structure
        content.push_str("## Repository Structure\n\n");
        content.push_str("```\n");
        content.push_str(&self.generate_tree(repo_path, if deep { 4 } else { 2 }));
        content.push_str("\n```\n\n");
        
        // Add file contents
        content.push_str("## Files\n\n");
        
        for file_path in files {
            let full_path = repo_path.join(file_path);
            
            if let Ok(file_content) = tokio::fs::read_to_string(&full_path).await {
                content.push_str(&format!("### {}\n\n", file_path.display()));
                
                // Detect file type for syntax highlighting
                let ext = file_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                
                content.push_str(&format!("```{}\n", ext));
                content.push_str(&file_content);
                content.push_str("\n```\n\n");
            }
        }
        
        Ok(content)
    }
}

#[async_trait]
impl ContentExtractor for CodeRepoExtractor {
    fn can_handle(&self, url: &str) -> bool {
        url.contains("github.com/") && !url.ends_with(".git")
            || (url.contains("github.com/") && url.contains("/tree/"))
    }

    async fn extract(&self, url: &str) -> Result<ExtractedContent> {
        // Parse GitHub URL
        let (owner, repo) = self
            .parse_github_url(url)
            .ok_or_else(|| anyhow!("Invalid GitHub URL"))?;
        
        info!("Extracting repository: {}/{}", owner, repo);
        
        // Clone repository
        let clone_url = format!("https://github.com/{}/{}.git", owner, repo);
        let (repo_path, commit_hash) = self.clone_repo(&clone_url).await?;
        
        // Detect language
        let language = self.detect_language(&repo_path);
        debug!("Detected language: {}", language);
        
        // Determine if deep analysis requested
        // Check URL for deep mode hint (e.g., URL?deep or URL#deep)
        let deep = url.contains("?deep") || url.contains("#deep") || url.contains("&deep");
        
        // Select files
        let files = self.select_files(&repo_path, deep)?;
        
        // Extract content
        let content = self.extract_repo_content(&repo_path, &files, deep).await?;
        
        // Clean up temp directory
        if let Err(e) = tokio::fs::remove_dir_all(&repo_path).await {
            warn!("Failed to clean up temp directory: {}", e);
        }
        
        Ok(ExtractedContent {
            url: url.to_string(),
            title: format!("{}/{}", owner, repo),
            content,
            content_type: ContentType::CodeRepository,
            metadata: Some(serde_json::json!({
                "repo": format!("{}/{}", owner, repo),
                "commit": commit_hash,
                "language": language,
                "files_analyzed": files.len(),
                "analysis_mode": if deep { "deep" } else { "basic" },
            })),
        })
    }

    fn content_type(&self) -> ContentType {
        ContentType::CodeRepository
    }
}
