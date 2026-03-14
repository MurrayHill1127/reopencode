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
}

impl ProviderError {
    pub fn is_rate_limit(&self) -> bool {
        matches!(self, ProviderError::RateLimit)
    }

    pub fn is_authentication(&self) -> bool {
        matches!(self, ProviderError::Authentication)
    }
}

pub type Result<T> = std::result::Result<T, ProviderError>;