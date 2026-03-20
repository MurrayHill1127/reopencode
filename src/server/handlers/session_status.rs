use std::collections::HashMap;

use axum::{extract::State, Json};

use crate::server::AppState;
use crate::session::SessionStatusInfo;

pub async fn list(
    State(state): State<AppState>,
) -> Json<HashMap<String, SessionStatusInfo>> {
    Json(state.session_manager.list_session_statuses())
}