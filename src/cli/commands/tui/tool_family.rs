//! Tool family glyph system — maps tool names to distinctive Unicode icons.
//!
//! Inspired by DeepSeek-TUI's tool card vocabulary.

/// Tool family for visual classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolFamily {
    /// File editing / patching: ◆
    Patch,
    /// Command execution: ▶
    Run,
    /// File reading: ▷
    Read,
    /// Search / find: ⌕
    Find,
    /// Web operations: ⬡
    Web,
    /// Agent delegation: ◐
    Delegate,
    /// Thinking / reasoning: …
    Think,
    /// Task management: ☐
    Task,
    /// Question / interaction: ?
    Question,
    /// Generic fallback: •
    Generic,
}

impl ToolFamily {
    /// Primary glyph for this tool family.
    pub fn glyph(&self) -> &'static str {
        match self {
            ToolFamily::Patch => "◆",
            ToolFamily::Run => "▶",
            ToolFamily::Read => "▷",
            ToolFamily::Find => "⌕",
            ToolFamily::Web => "⬡",
            ToolFamily::Delegate => "◐",
            ToolFamily::Think => "…",
            ToolFamily::Task => "☐",
            ToolFamily::Question => "?",
            ToolFamily::Generic => "•",
        }
    }

    /// Human-readable label for this family.
    pub fn label(&self) -> &'static str {
        match self {
            ToolFamily::Patch => "edit",
            ToolFamily::Run => "run",
            ToolFamily::Read => "read",
            ToolFamily::Find => "find",
            ToolFamily::Web => "web",
            ToolFamily::Delegate => "delegate",
            ToolFamily::Think => "think",
            ToolFamily::Task => "task",
            ToolFamily::Question => "ask",
            ToolFamily::Generic => "tool",
        }
    }
}

/// Classify a tool name into its visual family.
pub fn classify_tool(name: &str) -> ToolFamily {
    match name {
        "edit" | "write" | "apply_patch" | "patch" => ToolFamily::Patch,
        "bash" | "shell" | "exec" | "exec_shell" | "exec_shell_wait" => ToolFamily::Run,
        "read" | "read_file" | "list_dir" | "glob" => ToolFamily::Read,
        "grep" | "grep_files" | "file_search" | "search" => ToolFamily::Find,
        "web_search" | "web_fetch" | "websearch" | "webfetch" => ToolFamily::Web,
        "task" | "todo_write" | "todo" | "task_tool" => ToolFamily::Task,
        "question" | "ask" | "ask_user" | "prompt" => ToolFamily::Question,
        "agent" | "agent_spawn" | "delegate" | "subagent" => ToolFamily::Delegate,
        _ => ToolFamily::Generic,
    }
}
