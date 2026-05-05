# ReOpenCode

A Rust rewrite of [opencode](https://github.com/anomalyco/opencode) — same feature set, single binary, no runtime dependencies.

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)

---

## Why

The original opencode is TypeScript. That means Node.js, a fat `node_modules`, and the usual startup overhead. This project is a straight port to Rust: one binary you can drop anywhere and run.

Goals:
- Identical behaviour to opencode where it matters
- Fast startup, low memory
- No install step beyond copying the binary

Not a fork — the TypeScript source is used as a reference spec, not a dependency.

---

## Status

Core functionality is working. The server, session management, provider integrations, and most tools are implemented and tested. The TUI is functional but still being polished.

What's done:
- HTTP server with 29 endpoints (session CRUD, streaming, tool execution, provider auth)
- Session management with SQLite persistence
- Streaming LLM responses (SSE)
- Provider support: Anthropic, OpenAI, Azure, Google, OpenRouter, xAI, Vertex, Zhipu
- Tool implementations: edit, bash, read, write, glob, grep, web fetch, web search, apply\_patch, LSP stubs, and more
- TUI (ratatui): chat interface, session list, sidebar, file tree, command palette, theme system
- MCP client (stdio + HTTP transports)
- Config system with multi-level merge and environment variable substitution
- Category, command, hook, skill subsystems

What's still in progress:
- LSP integration (structure is there, language server communication not yet wired)
- Git operations module
- Snapshot and revert
- Permission confirmation flow
- Worktree management

---

## Tech stack

| Area | Crate |
|------|-------|
| Async runtime | tokio |
| HTTP server | axum + tower |
| HTTP client | reqwest |
| TUI | ratatui + crossterm |
| Database | sqlx (SQLite) |
| CLI | clap |
| MCP | rmcp |
| Serialization | serde + serde\_json |
| Streaming | async-stream + futures |

---

## Building

Requires Rust 1.75+.

```sh
# debug
cargo build

# release
cargo build --release

# run
./target/release/reopencode
```

Start the server:

```sh
./target/release/reopencode serve
```

The TUI connects to the server at `http://127.0.0.1:4096` by default.

---

## Development

```sh
# run tests
cargo test

# lint
cargo clippy -- -D warnings

# format
cargo fmt

# auto-recompile on save
cargo watch -x run
```

Run a specific test:

```sh
cargo test <test_name>
```

---

## Project layout

```
src/
├── main.rs          entry point
├── lib.rs           library root
├── agent/           agent loop and registry
├── provider/        LLM provider adapters
├── session/         session lifecycle, message store, streaming
├── server/          axum HTTP server and route handlers
├── tool/            tool implementations
├── mcp/             MCP client
├── pty/             pseudo-terminal sessions
├── config/          config loading and merging
├── storage/         storage backend (SQLite + JSON)
├── bus/             internal event bus
├── category/        category system
├── command/         slash command handling
├── hook/            lifecycle hooks
├── skill/           skill loader
└── cli/             CLI commands and TUI
    └── commands/tui/    ratatui interface and components
```

---

## Commit conventions

```
feat: add X
fix: handle Y edge case
refactor: simplify Z
test: cover W
docs: update README
```

---

## License

MIT — see [LICENSE](LICENSE).
