use anyhow::Result;
use tracing::info;

/// Expand a query into related queries for better coverage
pub fn expand_query(query: &str) -> Vec<String> {
    let mut queries = vec![query.to_string()];
    
    // Add variations
    let lowercase = query.to_lowercase();
    
    // If query is a question, add statement version
    if lowercase.contains("how") || lowercase.contains("what") || lowercase.contains("why") {
        // "how does X work" → "X explained", "X tutorial", "understanding X"
        let cleaned = query
            .replace("how does ", "")
            .replace("how do ", "")
            .replace("how to ", "")
            .replace("what is ", "")
            .replace("what are ", "")
            .replace("why is ", "")
            .replace("why are ", "")
            .replace(" work?", "")
            .replace(" work", "")
            .replace("?", "");
        
        queries.push(format!("{} explained", cleaned));
        queries.push(format!("{} tutorial", cleaned));
        queries.push(format!("understanding {}", cleaned));
    }
    
    // Add "latest" variant for current info
    if !lowercase.contains("latest") && !lowercase.contains("new") && !lowercase.contains("recent") {
        queries.push(format!("latest {}", query));
    }
    
    // Add "guide" variant
    if !lowercase.contains("guide") && !lowercase.contains("tutorial") {
        queries.push(format!("{} guide", query));
    }
    
    // Add "best practices" for technical queries
    if lowercase.contains("rust") || lowercase.contains("python") || 
       lowercase.contains("javascript") || lowercase.contains("programming") ||
       lowercase.contains("code") || lowercase.contains("api") {
        queries.push(format!("{} best practices", query));
    }
    
    // Add "vs" comparisons if mentioning specific tech
    let techs = ["rust", "python", "javascript", "tokio", "async", "go"];
    for tech in techs {
        if lowercase.contains(tech) {
            queries.push(format!("{} compared", query));
            break;
        }
    }
    
    // Limit to 5 queries max
    queries.truncate(5);
    
    info!("Expanded query into {} variants: {:?}", queries.len(), queries);
    
    queries
}

/// Score a title/snippet based on query relevance
pub fn score_relevance(query: &str, title: &str, snippet: &str) -> f32 {
    let query_lower = query.to_lowercase();
    let title_lower = title.to_lowercase();
    let snippet_lower = snippet.to_lowercase();
    
    let mut score = 0.0;
    
    // Extract key terms from query
    let key_terms = extract_key_terms(query);
    
    for term in &key_terms {
        let term_lower = term.to_lowercase();
        
        // Title matches are worth more
        if title_lower.contains(&term_lower) {
            score += 3.0;
        }
        
        // Snippet matches
        if snippet_lower.contains(&term_lower) {
            score += 1.0;
        }
    }
    
    // Exact phrase match in title is GOLD
    if title_lower.contains(&query_lower) {
        score += 10.0;
    }
    
    // Exact phrase match in snippet
    if snippet_lower.contains(&query_lower) {
        score += 5.0;
    }
    
    // Bonus for recent/new/latest (if query implies wanting fresh info)
    if query_lower.contains("latest") || query_lower.contains("new") || query_lower.contains("recent") {
        if title_lower.contains("2026") || title_lower.contains("2025") || 
           title_lower.contains("latest") || title_lower.contains("new") {
            score += 2.0;
        }
    }
    
    // Bonus for official documentation/guides
    if title_lower.contains("documentation") || title_lower.contains("official") || 
       title_lower.contains("guide") || title_lower.contains("docs") {
        score += 1.5;
    }
    
    score
}

/// Generate search terms from query (extract key terms)
pub fn extract_key_terms(query: &str) -> Vec<String> {
    // Remove common words
    let stop_words = [
        "the", "a", "an", "and", "or", "but", "is", "are", "was", "were",
        "how", "what", "why", "when", "where", "which", "who", "whom",
        "does", "do", "did", "has", "have", "had", "will", "would", "should",
        "can", "could", "may", "might", "must", "to", "of", "in", "on", "at",
        "for", "with", "about", "as", "by", "from", "this", "that", "these",
        "those", "it", "its", "i", "you", "we", "they", "them", "be", "being",
    ];
    
    query
        .to_lowercase()
        .split_whitespace()
        .filter(|word| !stop_words.contains(&word.trim_matches(|c: char| !c.is_alphanumeric())))
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_expand_query() {
        let expanded = expand_query("how does rust async work");
        assert!(expanded.len() > 1);
        assert!(expanded.iter().any(|q| q.contains("explained")));
    }
    
    #[test]
    fn test_extract_key_terms() {
        let terms = extract_key_terms("how does rust async work");
        assert!(terms.contains(&"rust".to_string()));
        assert!(terms.contains(&"async".to_string()));
        assert!(!terms.contains(&"does".to_string()));
    }
}
