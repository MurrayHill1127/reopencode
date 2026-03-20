//! AI Provider 模块
//!
//! 该模块负责与各种 AI 模型提供商 (OpenAI、Anthropic、智谱等) 进行交互，
//! 提供统一的 API 接口。

pub mod anthropic;
pub mod auth;
pub mod capability;
pub mod config;
pub mod error;
pub mod id;
pub mod message;
pub mod openai;
pub mod provider_trait;
pub mod registry;
pub mod stream;
pub mod transform;
pub mod zhipu;

// Re-export public API
pub use auth::{AuthManager, CredentialSource};
pub use capability::{Cost, ModelCapabilities, ModelDefinition, ModelLimits};
pub use config::ProviderConfig;
pub use error::{ProviderError, Result};
pub use id::{parse_model_string, ModelId, ModelIdParseError, ProviderId};
pub use message::{Message, MessageRole};
pub use openai::OpenAiProvider;
pub use provider_trait::{Provider, ProviderResponse, ToolDefinition, Usage};
pub use stream::{wrap_sse_timeout, SseStream, StreamTimeout};
pub use transform::{
    extract_system_messages, filter_system_messages, MessageNormalizer, ProviderMessage,
    TransformPipeline,
};
