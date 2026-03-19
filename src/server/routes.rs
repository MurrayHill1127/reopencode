use axum::{Router, routing::get};

use super::AppState;
use super::handlers;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::health))
        .route("/global/health", get(handlers::global::health))
        .route("/global/event", get(handlers::global::event))
        .route(
            "/global/config",
            get(handlers::global::config_get).patch(handlers::global::config_patch),
        )
        .route(
            "/global/dispose",
            axum::routing::post(handlers::global::dispose),
        )
        .route(
            "/session",
            get(handlers::session::list).post(handlers::session::create),
        )
        .route(
            "/session/{id}",
            get(handlers::session::get).delete(handlers::session::delete),
        )
        .route(
            "/session/{id}/message",
            axum::routing::post(handlers::session::send_message),
        )
        .route(
            "/session/{id}/stream",
            axum::routing::post(handlers::session::stream_message),
        )
        .route("/provider", get(handlers::provider::list))
        .route("/config", get(handlers::config::get))
        .route("/project", get(handlers::project::list))
        .route("/project/current", get(handlers::project::current))
        .route(
            "/project/git/init",
            axum::routing::post(handlers::project::git_init),
        )
        .route("/path", get(handlers::path::get))
        .route("/vcs", get(handlers::vcs::get))
        .route("/command", get(handlers::command::list))
        .route("/permission", get(handlers::permission::list))
        .route(
            "/permission/{id}/reply",
            axum::routing::post(handlers::permission::reply),
        )
        .route("/find", get(handlers::file::find))
        .route("/find/file", get(handlers::file::find_file))
        .route("/pty", get(handlers::pty::list).post(handlers::pty::create))
        .route(
            "/pty/{id}",
            get(handlers::pty::get)
                .put(handlers::pty::update)
                .delete(handlers::pty::remove),
        )
        .route("/pty/{id}/connect", get(handlers::pty::connect))
        .route("/question", get(handlers::question::list))
        .route(
            "/question/{id}/reply",
            axum::routing::post(handlers::question::reply),
        )
        .route(
            "/question/{id}/reject",
            axum::routing::post(handlers::question::reject),
        )
        .route("/mcp", get(handlers::mcp::status).post(handlers::mcp::add))
        .route(
            "/mcp/{name}/auth",
            axum::routing::post(handlers::mcp::auth_start),
        )
        .route(
            "/mcp/{name}/auth/callback",
            axum::routing::post(handlers::mcp::auth_callback),
        )
        .route(
            "/mcp/{name}/auth/authenticate",
            axum::routing::post(handlers::mcp::auth_authenticate),
        )
        .route(
            "/mcp/{name}/auth",
            axum::routing::delete(handlers::mcp::auth_remove),
        )
        .route(
            "/mcp/{name}/connect",
            axum::routing::post(handlers::mcp::connect),
        )
        .route(
            "/mcp/{name}/disconnect",
            axum::routing::post(handlers::mcp::disconnect),
        )
        .route("/tui/control/next", get(handlers::tui::control_next))
        .route(
            "/tui/control/response",
            axum::routing::post(handlers::tui::control_response),
        )
        .route(
            "/tui/append-prompt",
            axum::routing::post(handlers::tui::append_prompt),
        )
        .route(
            "/tui/open-help",
            axum::routing::post(handlers::tui::open_help),
        )
        .route(
            "/tui/open-sessions",
            axum::routing::post(handlers::tui::open_sessions),
        )
        .route(
            "/tui/open-themes",
            axum::routing::post(handlers::tui::open_themes),
        )
        .route(
            "/tui/open-models",
            axum::routing::post(handlers::tui::open_models),
        )
        .route(
            "/tui/submit-prompt",
            axum::routing::post(handlers::tui::submit_prompt),
        )
        .route(
            "/tui/clear-prompt",
            axum::routing::post(handlers::tui::clear_prompt),
        )
        .route(
            "/tui/execute-command",
            axum::routing::post(handlers::tui::execute_command),
        )
        .route(
            "/tui/show-toast",
            axum::routing::post(handlers::tui::show_toast),
        )
        .route("/tui/publish", axum::routing::post(handlers::tui::publish))
        .route(
            "/tui/select-session",
            axum::routing::post(handlers::tui::select_session),
        )
}
