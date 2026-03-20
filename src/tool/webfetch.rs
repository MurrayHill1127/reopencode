//! Web fetch tool - fetch content from URLs

use async_trait::async_trait;
use base64::Engine;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

use crate::tool::error::{Result, ToolError};
use crate::tool::traits::{Tool, ToolResult};

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const MAX_TIMEOUT_SECS: u64 = 120;
const MAX_RESPONSE_SIZE: usize = 5 * 1024 * 1024; // 5MB

const USER_AGENT_BROWSER: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36";
const USER_AGENT_HONEST: &str = "reopencode";

/// Web fetch tool - fetch content from URLs
pub struct WebFetchTool {
    client: Client,
}

impl WebFetchTool {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(MAX_TIMEOUT_SECS))
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    fn get_accept_header(format: &str) -> &'static str {
        match format {
            "markdown" => "text/markdown;q=1.0, text/x-markdown;q=0.9, text/plain;q=0.8, text/html;q=0.7, */*;q=0.1",
            "text" => "text/plain;q=1.0, text/markdown;q=0.9, text/html;q=0.8, */*;q=0.1",
            "html" => "text/html;q=1.0, application/xhtml+xml;q=0.9, text/plain;q=0.8, text/markdown;q=0.7, */*;q=0.1",
            _ => "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8",
        }
    }

    fn convert_html_to_markdown(html: &str) -> String {
        html2md::parse_html(html)
    }

    fn extract_text_from_html(html: &str) -> String {
        // Simple text extraction: remove script, style, and other non-content tags
        let mut text = String::new();
        let mut in_script = false;
        let mut in_style = false;

        for line in html.lines() {
            let line_lower = line.to_lowercase();

            // Track script/style blocks
            if line_lower.contains("<script") {
                in_script = true;
            }
            if line_lower.contains("</script") {
                in_script = false;
                continue;
            }
            if line_lower.contains("<style") {
                in_style = true;
            }
            if line_lower.contains("</style") {
                in_style = false;
                continue;
            }

            // Skip content inside script/style
            if in_script || in_style {
                continue;
            }

            // Strip HTML tags (simple approach)
            let mut chars = line.chars().peekable();
            while let Some(c) = chars.next() {
                if c == '<' {
                    // Skip until >
                    while let Some(&next) = chars.peek() {
                        chars.next();
                        if next == '>' {
                            break;
                        }
                    }
                } else {
                    text.push(c);
                }
            }
            text.push('\n');
        }

        // Clean up whitespace
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "webfetch"
    }

    fn description(&self) -> &str {
        "- Fetches content from a specified URL\n- Takes a URL and optional format as input\n- Fetches the URL content, converts to requested format (markdown by default)\n- Returns the content in the specified format\n- Use this tool when you need to retrieve and analyze web content\n\nUsage notes:\n  - IMPORTANT: if another tool is present that offers better web fetching capabilities, is more targeted to the task, or has fewer restrictions, prefer using that tool instead of this one.\n  - The URL must be a fully-formed valid URL\n  - HTTP URLs will be automatically upgraded to HTTPS\n  - Format options: \"markdown\" (default), \"text\", or \"html\"\n  - This tool is read-only and does not modify any files\n  - Results may be summarized if the content is very large"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch content from"
                },
                "format": {
                    "type": "string",
                    "enum": ["text", "markdown", "html"],
                    "description": "Output format (default: markdown)"
                },
                "timeout": {
                    "type": "number",
                    "description": "Optional timeout in seconds (max 120)"
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let url = args["url"]
            .as_str()
            .ok_or_else(|| ToolError::Parse("Missing 'url' argument".to_string()))?;

        // Validate URL
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(ToolError::Parse(
                "URL must start with http:// or https://".to_string(),
            ));
        }

        let format = args["format"].as_str().unwrap_or("markdown");
        let timeout_secs = args["timeout"]
            .as_u64()
            .unwrap_or(DEFAULT_TIMEOUT_SECS)
            .min(MAX_TIMEOUT_SECS);

        let accept_header = Self::get_accept_header(format);
        let mut header_map = reqwest::header::HeaderMap::new();
        header_map.insert("User-Agent", USER_AGENT_BROWSER.parse().unwrap());
        header_map.insert("Accept", accept_header.parse().unwrap());
        header_map.insert("Accept-Language", "en-US,en;q=0.9".parse().unwrap());

        // Make request with timeout
        let response = self
            .client
            .get(url)
            .headers(header_map)
            .timeout(Duration::from_secs(timeout_secs))
            .send()
            .await
            .map_err(|e| ToolError::Execution(format!("HTTP request failed: {}", e)))?;

        // Check for Cloudflare challenge - retry with honest UA
        let response = if response.status() == 403
            && response
                .headers()
                .get("cf-mitigated")
                .and_then(|v| v.to_str().ok())
                == Some("challenge")
        {
            self.client
                .get(url)
                .header("User-Agent", USER_AGENT_HONEST)
                .header("Accept", accept_header)
                .timeout(Duration::from_secs(timeout_secs))
                .send()
                .await
                .map_err(|e| ToolError::Execution(format!("Retry request failed: {}", e)))?
        } else {
            response
        };

        if !response.status().is_success() {
            return Err(ToolError::Execution(format!(
                "Request failed with status code: {}",
                response.status()
            )));
        }

        // Check content length
        if let Some(content_length) = response.headers().get("content-length") {
            if let Ok(len_str) = content_length.to_str() {
                if let Ok(len) = len_str.parse::<usize>() {
                    if len > MAX_RESPONSE_SIZE {
                        return Err(ToolError::Execution(
                            "Response too large (exceeds 5MB limit)".to_string(),
                        ));
                    }
                }
            }
        }

        // Get content type (clone to owned string before response is moved)
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let mime = content_type.split(';').next().unwrap_or("").trim().to_lowercase();

        // Get response bytes
        let bytes = response
            .bytes()
            .await
            .map_err(|e| ToolError::Execution(format!("Failed to read response: {}", e)))?;

        if bytes.len() > MAX_RESPONSE_SIZE {
            return Err(ToolError::Execution(
                "Response too large (exceeds 5MB limit)".to_string(),
            ));
        }

        // Check if response is an image
        let is_image = mime.starts_with("image/")
            && mime != "image/svg+xml"
            && mime != "image/vnd.fastbidsheet";

        if is_image {
            let base64_content = base64::engine::general_purpose::STANDARD.encode(&bytes);
            let data_uri = format!("data:{};base64,{}", mime, base64_content);
            return Ok(ToolResult::new(format!("Image fetched successfully\nData URI: {}", data_uri)));
        }

        // Convert bytes to string
        let content = String::from_utf8_lossy(&bytes).to_string();

        // Process based on format
        let output = match format {
            "markdown" => {
                if content_type.contains("text/html") {
                    Self::convert_html_to_markdown(&content)
                } else {
                    content
                }
            }
            "text" => {
                if content_type.contains("text/html") {
                    Self::extract_text_from_html(&content)
                } else {
                    content
                }
            }
            "html" | _ => content,
        };

        Ok(ToolResult::new(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webfetch_tool_name() {
        let tool = WebFetchTool::new();
        assert_eq!(tool.name(), "webfetch");
    }

    #[test]
    fn test_webfetch_tool_default() {
        let tool: WebFetchTool = Default::default();
        assert_eq!(tool.name(), "webfetch");
    }

    #[test]
    fn test_webfetch_parameters() {
        let tool = WebFetchTool::new();
        let params = tool.parameters();

        assert_eq!(params["type"], "object");
        assert!(params["required"].as_array().unwrap().contains(&serde_json::json!("url")));
    }

    #[tokio::test]
    async fn test_webfetch_missing_url() {
        let tool = WebFetchTool::new();
        let args = serde_json::json!({});

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ToolError::Parse(_)));
    }

    #[tokio::test]
    async fn test_webfetch_invalid_url() {
        let tool = WebFetchTool::new();
        let args = serde_json::json!({
            "url": "not-a-valid-url"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ToolError::Parse(_)));
    }

    #[test]
    fn test_accept_header_markdown() {
        let header = WebFetchTool::get_accept_header("markdown");
        assert!(header.contains("text/markdown"));
    }

    #[test]
    fn test_accept_header_text() {
        let header = WebFetchTool::get_accept_header("text");
        assert!(header.contains("text/plain"));
    }

    #[test]
    fn test_accept_header_html() {
        let header = WebFetchTool::get_accept_header("html");
        assert!(header.contains("text/html"));
    }

    #[test]
    fn test_html_to_markdown() {
        let html = "<html><body><h1>Hello</h1><p>World</p></body></html>";
        let markdown = WebFetchTool::convert_html_to_markdown(html);
        assert!(markdown.contains("Hello") || markdown.contains("World"));
    }

    #[test]
    fn test_extract_text_from_html() {
        let html = "<html><body><h1>Hello</h1><p>World</p></body></html>";
        let text = WebFetchTool::extract_text_from_html(html);
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
    }
}