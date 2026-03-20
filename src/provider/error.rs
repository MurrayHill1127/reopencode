use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("API 错误：{0}")]
    Api(String),

    #[error("网络错误：{0}")]
    Network(#[from] reqwest::Error),

    #[error("JSON 解析错误：{0}")]
    Json(#[from] serde_json::Error),

    #[error("配置错误：{0}")]
    Config(String),

    #[error("不支持的模型：{0}")]
    UnsupportedModel(String),

    #[error("流式响应错误：{0}")]
    Stream(String),

    #[error("速率限制：请稍后重试")]
    RateLimit,

    #[error("认证失败：请检查 API Key")]
    Authentication,

    #[error("请求超时：{0}")]
    Timeout(String),

    #[error("上下文溢出：token 数超出模型限制")]
    ContextOverflow,

    #[error("未实现：{0}")]
    NotImplemented(String),
}

impl ProviderError {
    pub fn is_rate_limit(&self) -> bool {
        matches!(self, ProviderError::RateLimit)
    }

    pub fn is_authentication(&self) -> bool {
        matches!(self, ProviderError::Authentication)
    }

    pub fn is_context_overflow(&self) -> bool {
        matches!(self, ProviderError::ContextOverflow)
    }

    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ProviderError::Network(_)
                | ProviderError::RateLimit
                | ProviderError::Timeout(_)
                | ProviderError::Api(_)
        )
    }
}

pub type Result<T> = std::result::Result<T, ProviderError>;
