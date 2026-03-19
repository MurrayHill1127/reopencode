//! AI Provider 模块
//!
//! 该模块负责与各种 AI 模型提供商（OpenAI、Anthropic、智谱等）进行交互，
//! 提供统一的 API 接口。

pub mod anthropic;
pub mod config;
pub mod error;
pub mod message;
pub mod openai;
pub mod provider_trait;
pub mod registry;
pub mod zhipu;

pub use config::ProviderConfig;
pub use message::{Message, MessageRole};
pub use openai::OpenAiProvider;
pub use provider_trait::{
    Provider, ToolDefinition,
};
