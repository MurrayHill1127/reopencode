use axum::{
    routing::get,
    Router,
};

use super::handlers;
use super::AppState;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::health))
        .route("/session", get(handlers::session::list).post(handlers::session::create))
        .route("/session/{id}", get(handlers::session::get))
        .route("/session/{id}/message", axum::routing::post(handlers::session::send_message))
        .route("/session/{id}/stream", axum::routing::post(handlers::session::stream_message))
        .route("/provider", get(handlers::provider::list))
        .route("/config", get(handlers::config::get))
        .route("/project", get(handlers::project::list))
        .route("/project/current", get(handlers::project::current))
        .route("/project/git/init", axum::routing::post(handlers::project::git_init))
        .route("/path", get(handlers::path::get))
        .route("/vcs", get(handlers::vcs::get))
        .route("/command", get(handlers::command::list))
        .route("/find", get(handlers::file::find))
        .route("/find/file", get(handlers::file::find_file))
}