//! Session management with SQLite persistence

pub mod compaction;
pub mod error;
pub mod llm;
pub mod manager;
pub mod message;
pub mod parts;
pub mod prompt;
pub mod query;
pub mod status;
pub mod store;
pub mod todo;
pub mod types;

pub use compaction::{
    is_overflow, prune, process, create,
    CompactionConfig, CompactionEvent, Event, ModelLimits, ProcessResult, TokenUsage,
    COMPACTION_BUFFER, PRUNE_MINIMUM, PRUNE_PROTECT,
};
pub use llm::{
    has_tool_calls, resolve_tools, stream, build_provider_messages,
    PermissionRuleset, StreamEvent, StreamInput, StreamOutput, StreamResult,
    ToolChoice, ToolDef, OUTPUT_TOKEN_MAX,
};
pub use manager::SessionManager;
pub use message::{MessageInfo, ModelMessage, WithParts, to_model_messages};
pub use parts::Part;
pub use prompt::{AbortController, PromptState};
pub use status::{SessionStatusInfo, SessionStatusState, StatusEvent};
pub use todo::TodoInfo;
pub use types::{Session, SessionStatus};
