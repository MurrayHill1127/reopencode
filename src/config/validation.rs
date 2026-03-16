//! Schema validation for configuration
//!
//! Validates config fields: required fields, ranges, formats

use std::collections::HashMap;

use crate::config::error::ValidationError;
use crate::config::types::{AgentConfig, AgentConfigs, Config, ProviderConfig, ServerConfig};

/// Schema validator
pub struct Validator;

impl Validator {
    /// 验证配置
    ///
    /// 验证规则:
    /// - 必填字段检查
    /// - 类型验证
    /// - 范围验证 (如 temperature: 0.0-2.0)
    /// - 格式验证 (如 URL, email)
    pub fn validate(config: &Config) -> Result<(), ValidationError> {
        Self::validate_server(&config.server)?;
        Self::validate_agents(&config.agent)?;
        Self::validate_providers(&config.provider)?;
        Ok(())
    }

    fn validate_server(server: &ServerConfig) -> Result<(), ValidationError> {
        // Port range: 1-65535
        if server.port == 0 {
            return Err(ValidationError::OutOfRange {
                field: "server.port".to_string(),
                actual: "0".to_string(),
                expected: "1-65535".to_string(),
            });
        }

        // Host should not be empty
        if server.host.is_empty() {
            return Err(ValidationError::RequiredField {
                field: "server.host".to_string(),
            });
        }

        Ok(())
    }

    fn validate_agents(agents: &AgentConfigs) -> Result<(), ValidationError> {
        for (name, agent) in &agents.agents {
            Self::validate_agent(name, agent)?;
        }
        Ok(())
    }

    fn validate_agent(name: &str, agent: &AgentConfig) -> Result<(), ValidationError> {
        // Temperature range: 0.0-2.0
        if agent.temperature < 0.0 || agent.temperature > 2.0 {
            return Err(ValidationError::OutOfRange {
                field: format!("agent.{}.temperature", name),
                actual: agent.temperature.to_string(),
                expected: "0.0-2.0".to_string(),
            });
        }

        // Top-p range: 0.0-1.0 (if set)
        if let Some(top_p) = agent.top_p {
            if top_p < 0.0 || top_p > 1.0 {
                return Err(ValidationError::OutOfRange {
                    field: format!("agent.{}.top_p", name),
                    actual: top_p.to_string(),
                    expected: "0.0-1.0".to_string(),
                });
            }
        }

        Ok(())
    }

    fn validate_providers(
        providers: &HashMap<String, ProviderConfig>,
    ) -> Result<(), ValidationError> {
        for (name, provider) in providers {
            Self::validate_provider(name, provider)?;
        }
        Ok(())
    }

    fn validate_provider(name: &str, provider: &ProviderConfig) -> Result<(), ValidationError> {
        // API URL format check (if set)
        if let Some(url) = &provider.api_url {
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(ValidationError::InvalidFormat {
                    field: format!("provider.{}.api_url", name),
                    message: "must start with http:// or https://".to_string(),
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_default_config() {
        let config = Config::default();
        assert!(Validator::validate(&config).is_ok());
    }

    #[test]
    fn test_validate_temperature_out_of_range() {
        let mut config = Config::default();
        config.agent.agents.insert(
            "test".to_string(),
            AgentConfig {
                temperature: 3.0,
                ..Default::default()
            },
        );

        let result = Validator::validate(&config);
        assert!(result.is_err());
        if let Err(ValidationError::OutOfRange { field, .. }) = result {
            assert!(field.contains("temperature"));
        } else {
            panic!("Expected OutOfRange error");
        }
    }

    #[test]
    fn test_validate_invalid_api_url() {
        let mut config = Config::default();
        config.provider.insert(
            "test".to_string(),
            ProviderConfig {
                api_url: Some("invalid-url".to_string()),
                ..Default::default()
            },
        );

        let result = Validator::validate(&config);
        assert!(result.is_err());
        if let Err(ValidationError::InvalidFormat { field, .. }) = result {
            assert!(field.contains("api_url"));
        } else {
            panic!("Expected InvalidFormat error");
        }
    }

    #[test]
    fn test_validate_valid_config() {
        let mut config = Config::default();
        config.server.port = 8080;
        config.server.host = "0.0.0.0".to_string();
        config.agent.agents.insert(
            "build".to_string(),
            AgentConfig {
                model: Some("anthropic/claude-3".to_string()),
                temperature: 0.1,
                ..Default::default()
            },
        );

        assert!(Validator::validate(&config).is_ok());
    }
}