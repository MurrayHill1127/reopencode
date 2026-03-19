//! Deep merge for configuration layers
//!
//! Implements merge strategy similar to remeda.mergeDeep:
//! - Scalar values: override wins
//! - Arrays: concatenate (union)
//! - HashMaps/BTreeMaps: deep merge
//! - Options: Some wins over None

use std::collections::{BTreeMap, HashMap};

use crate::config::types::*;

/// Deep merge strategy
pub struct MergeStrategy;

impl MergeStrategy {
    /// 合并两个配置
    ///
    /// 合并规则:
    /// - 标量值：覆盖 (override 优先)
    /// - 数组：合并 (union)
    /// - 对象：深度合并
    /// - Option: Some 覆盖 None
    pub fn merge(base: &Config, override_: &Config) -> Config {
        Config {
            schema: override_.schema.clone().or_else(|| base.schema.clone()),
            server: Self::merge_server(&base.server, &override_.server),
            agent: Self::merge_agent_configs(&base.agent, &override_.agent),
            provider: Self::merge_provider_map(&base.provider, &override_.provider),
            mcp: Self::merge_mcp_map(&base.mcp, &override_.mcp),
            permission: Self::merge_permission(&base.permission, &override_.permission),
            command: Self::merge_command_map(&base.command, &override_.command),
            skills: Self::merge_skills(&base.skills, &override_.skills),
            storage: override_.storage.clone(),
            hook: override_.hook.clone(),
            category: override_.category.clone(),
            extra: Self::merge_extra(&base.extra, &override_.extra),
        }
    }

    fn merge_server(base: &ServerConfig, override_: &ServerConfig) -> ServerConfig {
        ServerConfig {
            port: override_.port,
            host: if override_.host != "127.0.0.1" || base.host == "127.0.0.1" {
                override_.host.clone()
            } else {
                base.host.clone()
            },
            mdns: override_.mdns || base.mdns,
            cors_origin: Self::merge_vec(&base.cors_origin, &override_.cors_origin),
        }
    }

    fn merge_agent_configs(base: &AgentConfigs, override_: &AgentConfigs) -> AgentConfigs {
        let mut agents = base.agents.clone();
        for (name, config) in &override_.agents {
            if let Some(existing) = agents.get(name) {
                agents.insert(name.clone(), Self::merge_agent(existing, config));
            } else {
                agents.insert(name.clone(), config.clone());
            }
        }
        AgentConfigs { agents }
    }

    fn merge_agent(base: &AgentConfig, override_: &AgentConfig) -> AgentConfig {
        AgentConfig {
            model: override_.model.clone().or_else(|| base.model.clone()),
            temperature: override_.temperature,
            top_p: override_.top_p.or(base.top_p),
            prompt: override_.prompt.clone().or_else(|| base.prompt.clone()),
            description: override_
                .description
                .clone()
                .or_else(|| base.description.clone()),
            category: override_.category.clone().or_else(|| base.category.clone()),
            skills: override_.skills.clone().or_else(|| base.skills.clone()),
            disable: override_.disable.clone().or_else(|| base.disable.clone()),
            mode: override_.mode.clone().or(base.mode.clone()),
            color: override_.color.clone().or_else(|| base.color.clone()),
            permission: override_
                .permission
                .clone()
                .or_else(|| base.permission.clone()),
        }
    }

    fn merge_provider_map(
        base: &HashMap<String, ProviderConfig>,
        override_: &HashMap<String, ProviderConfig>,
    ) -> HashMap<String, ProviderConfig> {
        let mut result = base.clone();
        for (name, config) in override_ {
            if let Some(existing) = result.get(name) {
                result.insert(name.clone(), Self::merge_provider(existing, config));
            } else {
                result.insert(name.clone(), config.clone());
            }
        }
        result
    }

    fn merge_provider(base: &ProviderConfig, override_: &ProviderConfig) -> ProviderConfig {
        ProviderConfig {
            api_key: override_.api_key.clone().or_else(|| base.api_key.clone()),
            api_url: override_.api_url.clone().or_else(|| base.api_url.clone()),
            models: override_.models.clone().or_else(|| base.models.clone()),
            whitelist: override_
                .whitelist
                .clone()
                .or_else(|| base.whitelist.clone()),
            blacklist: override_
                .blacklist
                .clone()
                .or_else(|| base.blacklist.clone()),
        }
    }

    fn merge_mcp_map(
        base: &HashMap<String, McpConfig>,
        override_: &HashMap<String, McpConfig>,
    ) -> HashMap<String, McpConfig> {
        let mut result = base.clone();
        for (name, config) in override_ {
            result.insert(name.clone(), config.clone());
        }
        result
    }

    fn merge_permission(base: &PermissionConfig, override_: &PermissionConfig) -> PermissionConfig {
        let mut rules = base.rules.clone();
        for (tool, rule) in &override_.rules {
            rules.insert(tool.clone(), rule.clone());
        }
        PermissionConfig {
            default: override_.default.clone(),
            rules,
        }
    }

    fn merge_command_map(
        base: &HashMap<String, CommandConfig>,
        override_: &HashMap<String, CommandConfig>,
    ) -> HashMap<String, CommandConfig> {
        let mut result = base.clone();
        for (name, config) in override_ {
            result.insert(name.clone(), config.clone());
        }
        result
    }

    fn merge_skills(base: &SkillsConfig, override_: &SkillsConfig) -> SkillsConfig {
        SkillsConfig {
            sources: Self::merge_vec(&base.sources, &override_.sources),
        }
    }

    fn merge_extra(
        base: &BTreeMap<String, toml::Value>,
        override_: &BTreeMap<String, toml::Value>,
    ) -> BTreeMap<String, toml::Value> {
        let mut result = base.clone();
        for (key, value) in override_ {
            result.insert(key.clone(), value.clone());
        }
        result
    }

    fn merge_vec(base: &[String], override_: &[String]) -> Vec<String> {
        let mut result = base.to_vec();
        for item in override_ {
            if !result.contains(item) {
                result.push(item.clone());
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_scalar_override() {
        let base = Config::default();
        let mut override_ = Config::default();
        override_.server.port = 8080;

        let merged = MergeStrategy::merge(&base, &override_);
        assert_eq!(merged.server.port, 8080);
    }

    #[test]
    fn test_merge_server_config() {
        let mut base = Config::default();
        base.server.cors_origin = vec!["http://localhost:3000".to_string()];

        let mut override_ = Config::default();
        override_.server.port = 8080;
        override_.server.cors_origin = vec!["http://example.com".to_string()];

        let merged = MergeStrategy::merge(&base, &override_);
        assert_eq!(merged.server.port, 8080);
        assert!(
            merged
                .server
                .cors_origin
                .contains(&"http://localhost:3000".to_string())
        );
        assert!(
            merged
                .server
                .cors_origin
                .contains(&"http://example.com".to_string())
        );
    }

    #[test]
    fn test_merge_agent_configs() {
        let mut base = Config::default();
        base.agent.agents.insert(
            "build".to_string(),
            AgentConfig {
                model: Some("anthropic/claude-3".to_string()),
                temperature: 0.1,
                ..Default::default()
            },
        );

        let mut override_ = Config::default();
        override_.agent.agents.insert(
            "build".to_string(),
            AgentConfig {
                temperature: 0.5,
                ..Default::default()
            },
        );
        override_.agent.agents.insert(
            "plan".to_string(),
            AgentConfig {
                model: Some("openai/gpt-4".to_string()),
                temperature: 0.2,
                ..Default::default()
            },
        );

        let merged = MergeStrategy::merge(&base, &override_);
        assert_eq!(merged.agent.agents.get("build").unwrap().temperature, 0.5);
        assert!(merged.agent.agents.contains_key("plan"));
    }
}
