//! OpenAPI path definitions for all server routes

use std::collections::BTreeMap;

use utoipa::openapi::path::{HttpMethod, Operation, PathItem, Paths};
use utoipa::openapi::{RefOr, Response, ResponseBuilder};

fn json_response(desc: &str) -> RefOr<Response> {
    ResponseBuilder::new().description(desc).build().into()
}

fn make_op(id: &str, summary: &str, tag: &str) -> Operation {
    let mut op = Operation::default();
    op.operation_id = Some(id.to_string());
    op.summary = Some(summary.to_string());
    op.tags = Some(vec![tag.to_string()]);
    op.responses
        .responses
        .insert("200".to_string(), json_response("Success"));
    op
}

pub fn build_paths() -> Paths {
    let mut paths = Paths::default();
    paths.paths = BTreeMap::new();

    // Health check
    paths.paths.insert(
        "/".to_string(),
        PathItem::new(HttpMethod::Get, make_op("health", "Health check", "health")),
    );

    // Global routes
    paths.paths.insert(
        "/global/health".to_string(),
        PathItem::new(
            HttpMethod::Get,
            make_op("global.health", "Global health check", "global"),
        ),
    );

    paths.paths.insert(
        "/global/event".to_string(),
        PathItem::new(
            HttpMethod::Get,
            make_op("global.event", "Get global events", "global"),
        ),
    );

    let mut global_config_item = PathItem::new(
        HttpMethod::Get,
        make_op("global.config.get", "Get global config", "global"),
    );
    global_config_item.post = Some(make_op(
        "global.config.patch",
        "Update global config",
        "global",
    ));
    paths
        .paths
        .insert("/global/config".to_string(), global_config_item);

    paths.paths.insert(
        "/global/dispose".to_string(),
        PathItem::new(
            HttpMethod::Post,
            make_op("global.dispose", "Dispose global resources", "global"),
        ),
    );

    // Session routes
    let mut session_item = PathItem::new(
        HttpMethod::Get,
        make_op("session.list", "List all sessions", "session"),
    );
    session_item.post = Some(make_op("session.create", "Create a new session", "session"));
    paths.paths.insert("/session".to_string(), session_item);

    let mut session_id_item = PathItem::new(
        HttpMethod::Get,
        make_op("session.get", "Get session by ID", "session"),
    );
    session_id_item.delete = Some(make_op("session.delete", "Delete a session", "session"));
    paths
        .paths
        .insert("/session/{id}".to_string(), session_id_item);

    paths.paths.insert(
        "/session/{id}/message".to_string(),
        PathItem::new(
            HttpMethod::Post,
            make_op("session.message", "Send message to session", "session"),
        ),
    );

    paths.paths.insert(
        "/session/{id}/stream".to_string(),
        PathItem::new(
            HttpMethod::Post,
            make_op("session.stream", "Stream message to session", "session"),
        ),
    );

    // Provider routes
    paths.paths.insert(
        "/provider".to_string(),
        PathItem::new(
            HttpMethod::Get,
            make_op("provider.list", "List all providers", "provider"),
        ),
    );

    // Config routes
    paths.paths.insert(
        "/config".to_string(),
        PathItem::new(
            HttpMethod::Get,
            make_op("config.get", "Get configuration", "config"),
        ),
    );

    // Project routes
    paths.paths.insert(
        "/project".to_string(),
        PathItem::new(
            HttpMethod::Get,
            make_op("project.list", "List all projects", "project"),
        ),
    );

    paths.paths.insert(
        "/project/current".to_string(),
        PathItem::new(
            HttpMethod::Get,
            make_op("project.current", "Get current project", "project"),
        ),
    );

    paths.paths.insert(
        "/project/git/init".to_string(),
        PathItem::new(
            HttpMethod::Post,
            make_op("project.gitInit", "Initialize git repository", "project"),
        ),
    );

    // Path routes
    paths.paths.insert(
        "/path".to_string(),
        PathItem::new(
            HttpMethod::Get,
            make_op("path.get", "Get path information", "path"),
        ),
    );

    // VCS routes
    paths.paths.insert(
        "/vcs".to_string(),
        PathItem::new(HttpMethod::Get, make_op("vcs.get", "Get VCS status", "vcs")),
    );

    // Command routes
    paths.paths.insert(
        "/command".to_string(),
        PathItem::new(
            HttpMethod::Get,
            make_op("command.list", "List available commands", "command"),
        ),
    );

    // Permission routes
    paths.paths.insert(
        "/permission".to_string(),
        PathItem::new(
            HttpMethod::Get,
            make_op("permission.list", "List pending permissions", "permission"),
        ),
    );

    paths.paths.insert(
        "/permission/{id}/reply".to_string(),
        PathItem::new(
            HttpMethod::Post,
            make_op(
                "permission.reply",
                "Reply to permission request",
                "permission",
            ),
        ),
    );

    // File routes
    paths.paths.insert(
        "/find".to_string(),
        PathItem::new(
            HttpMethod::Get,
            make_op("file.find", "Find files matching pattern", "file"),
        ),
    );

    paths.paths.insert(
        "/find/file".to_string(),
        PathItem::new(
            HttpMethod::Get,
            make_op("file.findFile", "Find specific file", "file"),
        ),
    );

    // PTY routes
    let mut pty_item = PathItem::new(
        HttpMethod::Get,
        make_op("pty.list", "List PTY sessions", "pty"),
    );
    pty_item.post = Some(make_op("pty.create", "Create PTY session", "pty"));
    paths.paths.insert("/pty".to_string(), pty_item);

    let mut pty_id_item = PathItem::new(
        HttpMethod::Get,
        make_op("pty.get", "Get PTY session", "pty"),
    );
    pty_id_item.put = Some(make_op("pty.update", "Update PTY session", "pty"));
    pty_id_item.delete = Some(make_op("pty.delete", "Delete PTY session", "pty"));
    paths.paths.insert("/pty/{id}".to_string(), pty_id_item);

    paths.paths.insert(
        "/pty/{id}/connect".to_string(),
        PathItem::new(
            HttpMethod::Get,
            make_op("pty.connect", "Connect to PTY session", "pty"),
        ),
    );

    // Question routes
    paths.paths.insert(
        "/question".to_string(),
        PathItem::new(
            HttpMethod::Get,
            make_op("question.list", "List pending questions", "question"),
        ),
    );

    paths.paths.insert(
        "/question/{id}/reply".to_string(),
        PathItem::new(
            HttpMethod::Post,
            make_op("question.reply", "Reply to question", "question"),
        ),
    );

    paths.paths.insert(
        "/question/{id}/reject".to_string(),
        PathItem::new(
            HttpMethod::Post,
            make_op("question.reject", "Reject question", "question"),
        ),
    );

    // MCP routes
    let mut mcp_item = PathItem::new(
        HttpMethod::Get,
        make_op("mcp.status", "Get MCP status", "mcp"),
    );
    mcp_item.post = Some(make_op("mcp.add", "Add MCP server", "mcp"));
    paths.paths.insert("/mcp".to_string(), mcp_item);

    let mut mcp_auth_item = PathItem::new(
        HttpMethod::Post,
        make_op("mcp.authStart", "Start MCP authentication", "mcp"),
    );
    mcp_auth_item.delete = Some(make_op(
        "mcp.authRemove",
        "Remove MCP authentication",
        "mcp",
    ));
    paths
        .paths
        .insert("/mcp/{name}/auth".to_string(), mcp_auth_item);

    paths.paths.insert(
        "/mcp/{name}/auth/callback".to_string(),
        PathItem::new(
            HttpMethod::Post,
            make_op("mcp.authCallback", "MCP auth callback", "mcp"),
        ),
    );

    paths.paths.insert(
        "/mcp/{name}/auth/authenticate".to_string(),
        PathItem::new(
            HttpMethod::Post,
            make_op("mcp.authAuthenticate", "MCP authenticate", "mcp"),
        ),
    );

    paths.paths.insert(
        "/mcp/{name}/connect".to_string(),
        PathItem::new(
            HttpMethod::Post,
            make_op("mcp.connect", "Connect to MCP server", "mcp"),
        ),
    );

    paths.paths.insert(
        "/mcp/{name}/disconnect".to_string(),
        PathItem::new(
            HttpMethod::Post,
            make_op("mcp.disconnect", "Disconnect from MCP server", "mcp"),
        ),
    );

    // LSP routes
    paths.paths.insert(
        "/lsp".to_string(),
        PathItem::new(
            HttpMethod::Get,
            make_op("lsp.list", "List LSP servers", "lsp"),
        ),
    );

    // TUI routes
    paths.paths.insert(
        "/tui/control/next".to_string(),
        PathItem::new(
            HttpMethod::Get,
            make_op("tui.controlNext", "Get next TUI control", "tui"),
        ),
    );

    paths.paths.insert(
        "/tui/control/response".to_string(),
        PathItem::new(
            HttpMethod::Post,
            make_op("tui.controlResponse", "Send TUI control response", "tui"),
        ),
    );

    paths.paths.insert(
        "/tui/append-prompt".to_string(),
        PathItem::new(
            HttpMethod::Post,
            make_op("tui.appendPrompt", "Append to TUI prompt", "tui"),
        ),
    );

    paths.paths.insert(
        "/tui/open-help".to_string(),
        PathItem::new(
            HttpMethod::Post,
            make_op("tui.openHelp", "Open TUI help", "tui"),
        ),
    );

    paths.paths.insert(
        "/tui/open-sessions".to_string(),
        PathItem::new(
            HttpMethod::Post,
            make_op("tui.openSessions", "Open TUI sessions", "tui"),
        ),
    );

    paths.paths.insert(
        "/tui/open-themes".to_string(),
        PathItem::new(
            HttpMethod::Post,
            make_op("tui.openThemes", "Open TUI themes", "tui"),
        ),
    );

    paths.paths.insert(
        "/tui/open-models".to_string(),
        PathItem::new(
            HttpMethod::Post,
            make_op("tui.openModels", "Open TUI models", "tui"),
        ),
    );

    paths.paths.insert(
        "/tui/submit-prompt".to_string(),
        PathItem::new(
            HttpMethod::Post,
            make_op("tui.submitPrompt", "Submit TUI prompt", "tui"),
        ),
    );

    paths.paths.insert(
        "/tui/clear-prompt".to_string(),
        PathItem::new(
            HttpMethod::Post,
            make_op("tui.clearPrompt", "Clear TUI prompt", "tui"),
        ),
    );

    paths.paths.insert(
        "/tui/execute-command".to_string(),
        PathItem::new(
            HttpMethod::Post,
            make_op("tui.executeCommand", "Execute TUI command", "tui"),
        ),
    );

    paths.paths.insert(
        "/tui/show-toast".to_string(),
        PathItem::new(
            HttpMethod::Post,
            make_op("tui.showToast", "Show TUI toast", "tui"),
        ),
    );

    paths.paths.insert(
        "/tui/publish".to_string(),
        PathItem::new(
            HttpMethod::Post,
            make_op("tui.publish", "Publish to TUI", "tui"),
        ),
    );

    paths.paths.insert(
        "/tui/select-session".to_string(),
        PathItem::new(
            HttpMethod::Post,
            make_op("tui.selectSession", "Select TUI session", "tui"),
        ),
    );

    paths
}
