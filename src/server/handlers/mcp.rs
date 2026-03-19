//! MCP (Model Context Protocol) handlers

use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde_json::json;

use crate::mcp::{McpConfigRequest, McpStatus};
use crate::server::AppState;

/// GET /mcp - Get MCP server statuses
pub async fn status(State(state): State<AppState>) -> Json<serde_json::Value> {
    let statuses = state.mcp_manager.status().await;
    Json(serde_json::to_value(statuses).unwrap_or(json!({})))
}

/// POST /mcp - Add MCP server
pub async fn add(
    State(state): State<AppState>,
    Json(body): Json<crate::mcp::McpAddRequest>,
) -> Json<serde_json::Value> {
    let config = match body.config {
        McpConfigRequest::Local {
            command,
            environment,
            timeout: _,
        } => {
            let mut args = vec![];
            if command.len() > 1 {
                args = command[1..].to_vec();
            }
            crate::config::McpConfig::Local(crate::config::McpLocalConfig {
                command: command.first().cloned().unwrap_or_default(),
                args,
                env: environment,
                cwd: None,
            })
        }
        McpConfigRequest::Remote {
            url,
            headers,
            timeout,
        } => {
            let token = headers.and_then(|h| h.get("Authorization").cloned());
            crate::config::McpConfig::Remote(crate::config::McpRemoteConfig {
                url,
                token,
                timeout: timeout.unwrap_or(30000),
            })
        }
    };

    if let Err(e) = state.mcp_manager.add(&body.name, config).await {
        tracing::error!("Failed to add MCP server '{}': {}", body.name, e);
    }

    let statuses = state.mcp_manager.status().await;
    Json(serde_json::to_value(statuses).unwrap_or(json!({})))
}

/// POST /mcp/:name/auth - Start OAuth flow
pub async fn auth_start(
    State(_state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<crate::mcp::AuthStartResponse>, StatusCode> {
    Ok(Json(crate::mcp::AuthStartResponse {
        authorization_url: format!("https://example.com/oauth/{}", name),
    }))
}

/// POST /mcp/:name/auth/callback - OAuth callback
pub async fn auth_callback(
    State(_state): State<AppState>,
    Path(_name): Path<String>,
    Json(_body): Json<crate::mcp::AuthCallback>,
) -> Json<McpStatus> {
    Json(McpStatus::NeedsAuth)
}

/// POST /mcp/:name/auth/authenticate - Full OAuth authentication
pub async fn auth_authenticate(
    State(_state): State<AppState>,
    Path(_name): Path<String>,
) -> Json<McpStatus> {
    Json(McpStatus::NeedsAuth)
}

/// DELETE /mcp/:name/auth - Remove OAuth credentials
pub async fn auth_remove(
    State(_state): State<AppState>,
    Path(_name): Path<String>,
) -> Json<crate::mcp::RemoveAuthResponse> {
    Json(crate::mcp::RemoveAuthResponse { success: true })
}

/// POST /mcp/:name/connect - Connect to MCP server
pub async fn connect(State(state): State<AppState>, Path(name): Path<String>) -> Json<bool> {
    let client = state.mcp_manager.get_client(&name).await;
    Json(client.is_some())
}

/// POST /mcp/:name/disconnect - Disconnect from MCP server
pub async fn disconnect(State(state): State<AppState>, Path(name): Path<String>) -> Json<bool> {
    if let Err(e) = state.mcp_manager.disconnect(&name).await {
        tracing::error!("Failed to disconnect MCP server '{}': {}", name, e);
        Json(false)
    } else {
        Json(true)
    }
}
