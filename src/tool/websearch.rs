//! Web search tool - search the web using Exa AI MCP API

use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

use crate::tool::error::{Result, ToolError};
use crate::tool::traits::{Tool, ToolResult};

const API_URL: &str = "https://mcp.exa.ai/mcp";
const DEFAULT_NUM_RESULTS: usize = 8;
const TIMEOUT_SECS: u64 = 25;

/// Web search tool - search the web using Exa AI
pub struct WebSearchTool {
    client: Client,
}

impl WebSearchTool {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "websearch_web_search_exa"
    }

    fn description(&self) -> &str {
        "Search the web for any topic and get clean, ready-to-use content.\n\nBest for: Finding current information, news, facts, or answering questions about any topic.\nReturns: Clean text content from top search results, ready for LLM use."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Websearch query"
                },
                "numResults": {
                    "type": "number",
                    "description": "Number of search results to return (default: 8)"
                },
                "livecrawl": {
                    "type": "string",
                    "enum": ["fallback", "preferred"],
                    "description": "Live crawl mode - 'fallback': use live crawling as backup if cached content unavailable, 'preferred': prioritize live crawling (default: 'fallback')"
                },
                "type": {
                    "type": "string",
                    "enum": ["auto", "fast", "deep"],
                    "description": "Search type - 'auto': balanced search (default), 'fast': quick results"
                },
                "contextMaxCharacters": {
                    "type": "number",
                    "description": "Maximum characters for context string optimized for LLMs (must be a number, default: 10000)"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| ToolError::Parse("Missing 'query' argument".to_string()))?;

        let num_results = args["numResults"]
            .as_u64()
            .unwrap_or(DEFAULT_NUM_RESULTS as u64) as usize;

        let livecrawl = args["livecrawl"]
            .as_str()
            .unwrap_or("fallback");

        let search_type = args["type"]
            .as_str()
            .unwrap_or("auto");

        let context_max_chars = args["contextMaxCharacters"]
            .as_u64()
            .unwrap_or(10000) as usize;

        // Build MCP JSON-RPC request
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "web_search_exa",
                "arguments": {
                    "query": query,
                    "numResults": num_results,
                    "livecrawl": livecrawl,
                    "type": search_type,
                    "contextMaxCharacters": context_max_chars
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
                "Search error ({}): {}",
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
            "No search results found. Please try a different query.".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websearch_tool_name() {
        let tool = WebSearchTool::new();
        assert_eq!(tool.name(), "websearch_web_search_exa");
    }

    #[test]
    fn test_websearch_tool_default() {
        let tool: WebSearchTool = Default::default();
        assert_eq!(tool.name(), "websearch_web_search_exa");
    }

    #[test]
    fn test_websearch_parameters() {
        let tool = WebSearchTool::new();
        let params = tool.parameters();

        assert_eq!(params["type"], "object");
        assert!(params["required"].as_array().unwrap().contains(&serde_json::json!("query")));
    }

    #[tokio::test]
    async fn test_websearch_missing_query() {
        let tool = WebSearchTool::new();
        let args = serde_json::json!({});

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ToolError::Parse(_)));
    }
}