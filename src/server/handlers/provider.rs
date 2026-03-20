//! Provider handlers

use axum::{
    extract::{Json, Path},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Response Types
// =============================================================================

/// Provider info for list endpoint
#[derive(Debug, Serialize)]
pub struct ProviderInfo {
    id: String,
    name: String,
    models: Vec<String>,
}

/// Auth method type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthMethodType {
    Oauth,
    Api,
}

/// Auth method returned by /provider/auth
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthMethod {
    #[serde(rename = "type")]
    auth_type: AuthMethodType,
    label: String,
}

/// Authorization method (auto or code)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthorizationMethod {
    Auto,
    Code,
}

/// Authorization response from /provider/:provider_id/oauth/authorize
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Authorization {
    url: String,
    method: AuthorizationMethod,
    instructions: String,
}

/// Request body for OAuth authorize endpoint
#[derive(Debug, Deserialize)]
pub struct OAuthAuthorizeRequest {
    method: usize,
}

/// Request body for OAuth callback endpoint
#[derive(Debug, Deserialize)]
pub struct OAuthCallbackRequest {
    method: usize,
    code: Option<String>,
}

// =============================================================================
// Handlers
// =============================================================================

/// GET /provider - List providers
pub async fn list() -> Json<Vec<ProviderInfo>> {
    Json(vec![
        ProviderInfo {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            models: vec!["gpt-4".to_string(), "gpt-3.5-turbo".to_string()],
        },
        ProviderInfo {
            id: "anthropic".to_string(),
            name: "Anthropic".to_string(),
            models: vec!["claude-3-opus".to_string(), "claude-3-sonnet".to_string()],
        },
    ])
}

/// GET /provider/auth - Get auth methods for all providers
///
/// Returns a map of provider ID to available auth methods.
/// Currently returns empty map as plugin system is not implemented.
pub async fn auth_methods() -> Json<HashMap<String, Vec<AuthMethod>>> {
    // TODO: Implement when plugin system is available
    // For now, return empty map (no providers with auth methods configured)
    Json(HashMap::new())
}

/// POST /provider/:provider_id/oauth/authorize - Start OAuth authorization
///
/// Initiates OAuth flow for a provider. Returns authorization URL and instructions.
/// Currently returns 404 as plugin system is not implemented.
pub async fn oauth_authorize(
    Path(provider_id): Path<String>,
    Json(_body): Json<OAuthAuthorizeRequest>,
) -> Result<Json<Option<Authorization>>, StatusCode> {
    // TODO: Implement when plugin system is available
    tracing::debug!(
        "OAuth authorize requested for provider '{}' with method index {}",
        provider_id,
        _body.method
    );
    // Return None (404 equivalent) as no providers support OAuth yet
    Err(StatusCode::NOT_FOUND)
}

/// POST /provider/:provider_id/oauth/callback - Handle OAuth callback
///
/// Processes OAuth callback after user authorization.
/// Currently returns false as plugin system is not implemented.
pub async fn oauth_callback(
    Path(provider_id): Path<String>,
    Json(_body): Json<OAuthCallbackRequest>,
) -> Result<Json<bool>, StatusCode> {
    // TODO: Implement when plugin system is available
    tracing::debug!(
        "OAuth callback for provider '{}' with method index {}, code: {:?}",
        provider_id,
        _body.method,
        _body.code
    );
    // Return false as no OAuth flow was initiated
    Err(StatusCode::NOT_FOUND)
}
