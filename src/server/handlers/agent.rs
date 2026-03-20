//! Agent handler for HTTP API
//!
//! Implements agent listing endpoint:
//! - GET /agent - List available agents

use axum::extract::Json;
use crate::agent::registry::AgentRegistry;
use crate::agent::config::AgentInfo;

/// List all available (visible) agents
pub async fn list() -> Json<Vec<AgentInfo>> {
    let registry = AgentRegistry::new();
    let agents: Vec<AgentInfo> = registry.list_visible()
        .into_iter()
        .cloned()
        .collect();
    Json(agents)
}