use axum::{
    routing::get,
    Router,
};

use super::handlers;

pub fn create_router() -> Router {
    Router::new()
        .route("/", get(handlers::health))
        .route("/session", get(handlers::session::list).post(handlers::session::create))
        .route("/session/{id}", get(handlers::session::get))
        .route("/session/{id}/message", axum::routing::post(handlers::session::send_message))
        .route("/provider", get(handlers::provider::list))
        .route("/config", get(handlers::config::get))
}