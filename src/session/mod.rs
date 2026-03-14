//! Session management with SQLite persistence

pub mod error;
pub mod manager;
pub mod query;
pub mod store;
pub mod types;

pub use error::{Result, SessionError};
pub use manager::SessionManager;
pub use query::SessionQuery;
pub use store::SessionStore;
pub use types::{MessageId, Session, SessionFilter, SessionId, SessionMessage, SessionStatus};