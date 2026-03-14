use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::models::{ScrapedPage, Source};

pub struct LLMAnalyzer {
    client: Client,
    api_key: String,
    model: String,
}

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ClaudeMessage>,
}

#[derive(Debug, Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
}

#[derive(Debug, Deserialize)]
struct ClaudeContent {
    text: String,
}

impl LLMAnalyzer {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: "claude-sonnet-4-20250514".to_string(),
        }
    }

    /// Analyze a single page and extract key information
    pub async fn analyze_page(&self, page: &ScrapedPage, query: &str) -> Result<Source> {
        let prompt = format!(
            r#"You are analyzing a web page to answer the query: "{}"

Page Title: {}
URL: {}
Content: {}

Extract the following in JSON format:
{{
  "key_facts": ["fact1", "fact2", ...],
  "quotes": ["direct quote 1", "direct quote 2", ...],
  "confidence": 0.0-1.0
}}

Only include information directly relevant to the query. Use direct quotes from the content.
Confidence should reflect how well this page answers the query."#,
            query, page.title, page.url, page.content
        );

        let response = self.call_claude(&prompt).await?;
        
        // Parse JSON from response
        let parsed: serde_json::Value = serde_json::from_str(&response)
            .context("Failed to parse LLM response as JSON")?;

        Ok(Source {
            url: page.url.clone(),
            title: page.title.clone(),
            key_facts: parsed["key_facts"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            quotes: parsed["quotes"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            confidence: parsed["confidence"].as_f64().map(|f| f as f32),
        })
    }

    /// Synthesize information from multiple sources
    pub async fn synthesize(&self, query: &str, sources: &[Source]) -> Result<String> {
        let sources_text = sources
            .iter()
            .enumerate()
            .map(|(i, source)| {
                format!(
                    "Source {}: {}\nKey facts:\n{}\n\nQuotes:\n{}\n",
                    i + 1,
                    source.url,
                    source.key_facts.join("\n- "),
                    source.quotes.iter().map(|q| format!("\"{}\"", q)).collect::<Vec<_>>().join("\n- ")
                )
            })
            .collect::<Vec<_>>()
            .join("\n---\n\n");

        let prompt = format!(
            r#"You are synthesizing information from multiple web sources to answer the query: "{}"

Sources:
{}

Provide a comprehensive answer that:
1. Directly answers the query
2. Combines information from multiple sources
3. Cites sources using [Source N] notation
4. Identifies agreements and disagreements between sources
5. Uses direct quotes where appropriate

Be concise but thorough. Focus on accuracy and clarity."#,
            query, sources_text
        );

        self.call_claude(&prompt).await
    }

    async fn call_claude(&self, prompt: &str) -> Result<String> {
        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: 4096,
            messages: vec![ClaudeMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to call Claude API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Claude API error {}: {}", status, body);
        }

        let claude_response: ClaudeResponse = response
            .json()
            .await
            .context("Failed to parse Claude response")?;

        claude_response
            .content
            .first()
            .map(|c| c.text.clone())
            .ok_or_else(|| anyhow::anyhow!("Empty response from Claude"))
    }
}
