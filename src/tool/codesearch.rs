//! Code search tool - search for code context using Exa AI MCP API

use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

use crate::tool::error::{Result, ToolError};
use crate::tool::traits::{Tool, ToolResult};

const API_URL: &str = "https://mcp.exa.ai/mcp";
const DEFAULT_TOKENS_NUM: usize = 5000;
const MIN_TOKENS: usize = 1000;
const MAX_TOKENS: usize = 50000;
const TIMEOUT_SECS: u64 = 30;

/// Code search tool - search for code examples and documentation
pub struct CodeSearchTool {
    client: Client,
}

impl CodeSearchTool {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }
}

impl Default for CodeSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for CodeSearchTool {
    fn name(&self) -> &str {
        "grep_app_searchGitHub"
    }

    fn description(&self) -> &str {
        "Find real-world code examples from over a million public GitHub repositories to help answer programming questions.\n\n**IMPORTANT: This tool searches for literal code patterns (like grep), not keywords. Search for actual code that would appear in files:**\n- ✅ Good: 'useState(', 'import React from', 'async function', '(?s)try {.*await'\n- ❌ Bad: 'react tutorial', 'best practices', 'how to use'\n\n**Perfect for questions like:**\n- \"How do developers handle authentication in Next.js apps?\" → Search: 'getServerSession' with language=['TypeScript', 'TSX']\n- \"What are common React error boundary patterns?\" → Search: 'ErrorBoundary' with language=['TSX']\n- \"Show me real useEffect cleanup examples\" → Search: '(?s)useEffect\\(\\(\\) => {.*removeEventListener' with useRegexp=true\n\nUse regular expressions with useRegexp=true for flexible patterns like '(?s)useState\\(.*loading' to find useState hooks with loading-related variables. Prefix the pattern with '(?s)' to match across multiple lines.\n\nFilter by language, repository, or file path to narrow results."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query for APIs, Libraries, SDKs"
                },
                "tokensNum": {
                    "type": "number",
                    "description": "Number of tokens to return (1000-50000, default: 5000)"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| ToolError::Parse("Missing 'query' argument".to_string()))?;

        let tokens_num = args["tokensNum"]
            .as_u64()
            .unwrap_or(DEFAULT_TOKENS_NUM as u64) as usize;

        // Clamp tokens to valid range
        let tokens_num = tokens_num.clamp(MIN_TOKENS, MAX_TOKENS);

        // Build MCP JSON-RPC request
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "get_code_context_exa",
                "arguments": {
                    "query": query,
                    "tokensNum": tokens_num
                }
            }
        });

        let response = self
            .client
            .post(API_URL)
            .header("Accept", "application/json, text/event-stream")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ToolError::Execution(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ToolError::Execution(format!(
                "Code search error ({}): {}",
                status, error_text
            )));
        }

        let response_text = response
            .text()
            .await
            .map_err(|e| ToolError::Execution(format!("Failed to read response: {}", e)))?;

        // Parse SSE response - lines starting with "data: " contain JSON
        for line in response_text.lines() {
            if let Some(data_str) = line.strip_prefix("data: ") {
                if let Ok(data) = serde_json::from_str::<Value>(data_str) {
                    if let Some(result) = data.get("result") {
                        if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
                            if let Some(first_item) = content.first() {
                                if let Some(text) = first_item.get("text").and_then(|t| t.as_str()) {
                                    return Ok(ToolResult::new(text.to_string()));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(ToolResult::new(
            "No code snippets or documentation found. Please try a different query, be more specific about the library or programming concept, or check the spelling of framework names.".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codesearch_tool_name() {
        let tool = CodeSearchTool::new();
        assert_eq!(tool.name(), "grep_app_searchGitHub");
    }

    #[test]
    fn test_codesearch_tool_default() {
        let tool: CodeSearchTool = Default::default();
        assert_eq!(tool.name(), "grep_app_searchGitHub");
    }

    #[test]
    fn test_codesearch_parameters() {
        let tool = CodeSearchTool::new();
        let params = tool.parameters();

        assert_eq!(params["type"], "object");
        assert!(params["required"].as_array().unwrap().contains(&serde_json::json!("query")));
    }

    #[tokio::test]
    async fn test_codesearch_missing_query() {
        let tool = CodeSearchTool::new();
        let args = serde_json::json!({});

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ToolError::Parse(_)));
    }
}