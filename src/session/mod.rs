//! Session management with SQLite persistence

pub mod error;
pub mod manager;
pub mod prompt;
pub mod query;
pub mod status;
pub mod store;
pub mod todo;
pub mod types;

pub use manager::SessionManager;
pub use prompt::{AbortController, PromptState};
pub use status::{SessionStatusInfo, SessionStatusState, StatusEvent};
pub use todo::TodoInfo;
pub use types::{Session, SessionStatus};
