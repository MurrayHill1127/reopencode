use axum::{routing::get, Router};

use super::handlers;
use super::AppState;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::health))
        // Global endpoints
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
        // Auth endpoints
        .route(
            "/global/auth/{provider_id}",
            axum::routing::put(handlers::auth::set_auth).delete(handlers::auth::remove_auth),
        )
        // Agent and Skill endpoints
        .route("/global/agent", get(handlers::agent::list))
        .route("/global/skill", get(handlers::skill::list))
        // Session endpoints
        .route(
            "/session",
            get(handlers::session::list).post(handlers::session::create),
        )
        .route("/session/status", get(handlers::session_status::list))
        .route(
            "/session/{id}",
            get(handlers::session::get)
                .delete(handlers::session::delete)
                .patch(handlers::session::update),
        )
        .route(
            "/session/{id}/children",
            get(handlers::session_children::get_children),
        )
        .route(
            "/session/{id}/message",
            axum::routing::post(handlers::session::send_message),
        )
        .route(
            "/session/{id}/message/{message_id}",
            axum::routing::get(handlers::session_message::get_message)
                .delete(handlers::session_message::delete_message),
        )
        .route(
            "/session/{id}/message/{message_id}/part/{part_id}",
            axum::routing::delete(handlers::session_message::delete_part)
                .patch(handlers::session_message::update_part),
        )
        .route(
            "/session/{id}/stream",
            axum::routing::post(handlers::session::stream_message),
        )
        .route(
            "/session/{id}/abort",
            axum::routing::post(handlers::session_abort::abort),
        )
        .route(
            "/session/{id}/fork",
            axum::routing::post(handlers::session_fork::fork),
        )
        .route(
            "/session/{id}/revert",
            axum::routing::post(handlers::session_revert::revert),
        )
        .route(
            "/session/{id}/unrevert",
            axum::routing::post(handlers::session_unrevert::unrevert),
        )
        .route(
            "/session/{id}/summarize",
            axum::routing::post(handlers::session_summarize::summarize),
        )
        .route(
            "/session/{id}/init",
            axum::routing::post(handlers::session_init::init),
        )
        .route(
            "/session/{id}/share",
            axum::routing::post(handlers::session_share::share)
                .delete(handlers::session_share::unshare),
        )
        .route("/session/{id}/diff", get(handlers::session_diff::get_diff))
        .route("/session/{id}/todo", get(handlers::session_todo::get_todos))
        .route(
            "/session/{id}/prompt_async",
            axum::routing::post(handlers::session_prompt_async::prompt_async),
        )
        .route(
            "/session/{id}/command",
            axum::routing::post(handlers::session_command::command),
        )
        .route(
            "/session/{id}/shell",
            axum::routing::post(handlers::session_shell::shell),
        )
        .route(
            "/session/{id}/permissions/{permissionID}",
            axum::routing::post(handlers::session_permission_reply::permission_reply),
        )
        .route("/provider", get(handlers::provider::list))
        .route("/provider/auth", get(handlers::provider::auth_methods))
        .route(
            "/provider/{provider_id}/oauth/authorize",
            axum::routing::post(handlers::provider::oauth_authorize),
        )
        .route(
            "/provider/{provider_id}/oauth/callback",
            axum::routing::post(handlers::provider::oauth_callback),
        )
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
        .route("/lsp", get(handlers::lsp::list))
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
