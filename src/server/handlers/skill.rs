//! Skill handler for HTTP API
//!
//! Implements skill listing endpoint:
//! - GET /skill - List available skills

use axum::extract::Json;
use crate::skill::registry::SkillRegistry;
use crate::skill::types::SkillInfo;

/// List all available skills
pub async fn list() -> Json<Vec<SkillInfo>> {
    let registry = SkillRegistry::global();
    let skills = registry.all().await;
    Json(skills)
}