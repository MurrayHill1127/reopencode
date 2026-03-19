//! Session management with SQLite persistence

pub mod error;
pub mod manager;
pub mod query;
pub mod store;
pub mod types;

pub use manager::SessionManager;
pub use types::{Session, SessionStatus};
