//! Tool system - tools that agents can use

pub mod bash;
pub mod edit;
pub mod error;
pub mod read;
pub mod registry;
pub mod traits;
pub mod write;

pub use bash::BashTool;
pub use edit::EditTool;
pub use error::{Result, ToolError};
pub use read::ReadTool;
pub use registry::ToolRegistry;
pub use traits::{Tool, ToolResult};
pub use write::WriteTool;
