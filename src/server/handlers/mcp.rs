//! MCP (Model Context Protocol) handler stubs
//!
//! MVP stub implementations for MCP server management routes.

use axum::{
    extract::{Json, Path},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum McpStatus {
    Connected,
    Disabled,
    Failed { error: String },
    NeedsAuth,
    NeedsClientRegistration { error: String },
}

#[derive(Debug, Deserialize)]
pub struct McpAddRequest {
    pub name: String,
    pub config: McpConfig,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum McpConfig {
    Local {
        command: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        environment: Option<HashMap<String, String>>,
    },
    Remote {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<HashMap<String, String>>,
    },
}

#[derive(Debug, Serialize)]
pub struct AuthStartResponse {
    pub authorization_url: String,
}

#[derive(Debug, Deserialize)]
pub struct AuthCallback {
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct RemoveAuthResponse {
    pub success: bool,
}

// ============================================================================
// Handlers (MVP Stubs)
// ============================================================================

/// GET /mcp - Get MCP server statuses
pub async fn status() -> Json<serde_json::Value> {
    Json(serde_json::json!({}))
}

/// POST /mcp - Add MCP server
pub async fn add(Json(body): Json<McpAddRequest>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        body.name: {"status": "disabled"}
    }))
}

/// POST /mcp/:name/auth - Start OAuth flow
pub async fn auth_start(Path(name): Path<String>) -> Result<Json<AuthStartResponse>, StatusCode> {
    Ok(Json(AuthStartResponse {
        authorization_url: format!("https://example.com/oauth/{}", name),
    }))
}

/// POST /mcp/:name/auth/callback - OAuth callback
pub async fn auth_callback(
    Path(_name): Path<String>,
    Json(_body): Json<AuthCallback>,
) -> Json<McpStatus> {
    Json(McpStatus::NeedsAuth)
}

/// POST /mcp/:name/auth/authenticate - Full OAuth authentication
pub async fn auth_authenticate(Path(_name): Path<String>) -> Json<McpStatus> {
    Json(McpStatus::NeedsAuth)
}

/// DELETE /mcp/:name/auth - Remove OAuth credentials
pub async fn auth_remove(Path(_name): Path<String>) -> Json<RemoveAuthResponse> {
    Json(RemoveAuthResponse { success: true })
}

/// POST /mcp/:name/connect - Connect to MCP server
pub async fn connect(Path(_name): Path<String>) -> Json<bool> {
    Json(true)
}

/// POST /mcp/:name/disconnect - Disconnect from MCP server
pub async fn disconnect(Path(_name): Path<String>) -> Json<bool> {
    Json(true)
}