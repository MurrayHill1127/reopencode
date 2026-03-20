//! Session management with SQLite persistence

pub mod error;
pub mod manager;
pub mod message;
pub mod parts;
pub mod prompt;
pub mod query;
pub mod status;
pub mod store;
pub mod todo;
pub mod types;

pub use manager::SessionManager;
pub use message::{MessageInfo, ModelMessage, WithParts, to_model_messages};
pub use parts::Part;
pub use prompt::{AbortController, PromptState};
pub use status::{SessionStatusInfo, SessionStatusState, StatusEvent};
pub use todo::TodoInfo;
pub use types::{Session, SessionStatus};
