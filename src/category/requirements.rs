//! Model requirements with fallback chains
//!
//! Defines fallback chains for categories and agents.

use lazy_static::lazy_static;
use std::collections::HashMap;

use super::types::{FallbackEntry, ModelRequirement, ModelVariant};

lazy_static! {
    /// Category model requirements with fallback chains
    pub static ref CATEGORY_MODEL_REQUIREMENTS: HashMap<String, ModelRequirement> = {
        let mut m = HashMap::new();

        // visual-engineering
        m.insert("visual-engineering".to_string(), ModelRequirement {
            fallback_chain: vec![
                FallbackEntry {
                    providers: vec!["google".to_string(), "github-copilot".to_string(), "opencode".to_string()],
                    model: "gemini-3.1-pro".to_string(),
                    variant: Some(ModelVariant::High),
                },
                FallbackEntry {
                    providers: vec!["zai-coding-plan".to_string(), "opencode".to_string()],
                    model: "glm-5".to_string(),
                    variant: None,
                },
                FallbackEntry {
                    providers: vec!["anthropic".to_string(), "github-copilot".to_string(), "opencode".to_string()],
                    model: "claude-opus-4-6".to_string(),
                    variant: Some(ModelVariant::Max),
                },
            ],
            variant: None,
            requires_model: None,
            requires_any_model: false,
            requires_provider: None,
        });

        // ultrabrain
        m.insert("ultrabrain".to_string(), ModelRequirement {
            fallback_chain: vec![
                FallbackEntry {
                    providers: vec!["openai".to_string(), "opencode".to_string()],
                    model: "gpt-5.4".to_string(),
                    variant: Some(ModelVariant::Xhigh),
                },
                FallbackEntry {
                    providers: vec!["google".to_string(), "github-copilot".to_string(), "opencode".to_string()],
                    model: "gemini-3.1-pro".to_string(),
                    variant: Some(ModelVariant::High),
                },
                FallbackEntry {
                    providers: vec!["anthropic".to_string(), "github-copilot".to_string(), "opencode".to_string()],
                    model: "claude-opus-4-6".to_string(),
                    variant: Some(ModelVariant::Max),
                },
            ],
            variant: None,
            requires_model: None,
            requires_any_model: false,
            requires_provider: None,
        });

        // deep - requires GPT-5.3 Codex
        m.insert("deep".to_string(), ModelRequirement {
            fallback_chain: vec![
                FallbackEntry {
                    providers: vec!["openai".to_string(), "opencode".to_string()],
                    model: "gpt-5.3-codex".to_string(),
                    variant: Some(ModelVariant::Medium),
                },
                FallbackEntry {
                    providers: vec!["anthropic".to_string(), "github-copilot".to_string(), "opencode".to_string()],
                    model: "claude-opus-4-6".to_string(),
                    variant: Some(ModelVariant::Max),
                },
            ],
            variant: None,
            requires_model: Some("gpt-5.3-codex".to_string()),
            requires_any_model: false,
            requires_provider: None,
        });

        // artistry
        m.insert("artistry".to_string(), ModelRequirement {
            fallback_chain: vec![
                FallbackEntry {
                    providers: vec!["google".to_string(), "github-copilot".to_string(), "opencode".to_string()],
                    model: "gemini-3.1-pro".to_string(),
                    variant: Some(ModelVariant::High),
                },
                FallbackEntry {
                    providers: vec!["anthropic".to_string(), "github-copilot".to_string(), "opencode".to_string()],
                    model: "claude-opus-4-6".to_string(),
                    variant: Some(ModelVariant::Max),
                },
            ],
            variant: None,
            requires_model: None,
            requires_any_model: false,
            requires_provider: None,
        });

        // quick
        m.insert("quick".to_string(), ModelRequirement {
            fallback_chain: vec![
                FallbackEntry {
                    providers: vec!["anthropic".to_string(), "github-copilot".to_string(), "opencode".to_string()],
                    model: "claude-haiku-4-5".to_string(),
                    variant: None,
                },
                FallbackEntry {
                    providers: vec!["google".to_string(), "github-copilot".to_string(), "opencode".to_string()],
                    model: "gemini-3-flash".to_string(),
                    variant: None,
                },
                FallbackEntry {
                    providers: vec!["opencode".to_string()],
                    model: "gpt-5-nano".to_string(),
                    variant: None,
                },
            ],
            variant: None,
            requires_model: None,
            requires_any_model: false,
            requires_provider: None,
        });

        // unspecified-low
        m.insert("unspecified-low".to_string(), ModelRequirement {
            fallback_chain: vec![
                FallbackEntry {
                    providers: vec!["anthropic".to_string(), "github-copilot".to_string(), "opencode".to_string()],
                    model: "claude-sonnet-4-6".to_string(),
                    variant: None,
                },
                FallbackEntry {
                    providers: vec!["openai".to_string(), "github-copilot".to_string(), "opencode".to_string()],
                    model: "gpt-5.4".to_string(),
                    variant: Some(ModelVariant::Medium),
                },
                FallbackEntry {
                    providers: vec!["google".to_string(), "opencode".to_string()],
                    model: "gemini-3-flash".to_string(),
                    variant: None,
                },
            ],
            variant: None,
            requires_model: None,
            requires_any_model: false,
            requires_provider: None,
        });

        // unspecified-high
        m.insert("unspecified-high".to_string(), ModelRequirement {
            fallback_chain: vec![
                FallbackEntry {
                    providers: vec!["anthropic".to_string(), "github-copilot".to_string(), "opencode".to_string()],
                    model: "claude-opus-4-6".to_string(),
                    variant: Some(ModelVariant::Max),
                },
                FallbackEntry {
                    providers: vec!["openai".to_string(), "opencode".to_string()],
                    model: "gpt-5.4".to_string(),
                    variant: Some(ModelVariant::High),
                },
                FallbackEntry {
                    providers: vec!["google".to_string(), "github-copilot".to_string(), "opencode".to_string()],
                    model: "gemini-3.1-pro".to_string(),
                    variant: Some(ModelVariant::High),
                },
            ],
            variant: None,
            requires_model: None,
            requires_any_model: false,
            requires_provider: None,
        });

        // writing
        m.insert("writing".to_string(), ModelRequirement {
            fallback_chain: vec![
                FallbackEntry {
                    providers: vec!["kimi-for-coding".to_string()],
                    model: "k2p5".to_string(),
                    variant: None,
                },
                FallbackEntry {
                    providers: vec!["opencode-go".to_string()],
                    model: "kimi-k2.5".to_string(),
                    variant: None,
                },
                FallbackEntry {
                    providers: vec!["anthropic".to_string(), "opencode".to_string()],
                    model: "claude-sonnet-4-6".to_string(),
                    variant: None,
                },
            ],
            variant: None,
            requires_model: None,
            requires_any_model: false,
            requires_provider: None,
        });

        m
    };

    /// Agent model requirements with fallback chains
    pub static ref AGENT_MODEL_REQUIREMENTS: HashMap<String, ModelRequirement> = {
        let mut m = HashMap::new();

        // Sisyphus - main orchestrator
        m.insert("sisyphus".to_string(), ModelRequirement {
            fallback_chain: vec![
                FallbackEntry {
                    providers: vec!["anthropic".to_string(), "github-copilot".to_string(), "opencode".to_string()],
                    model: "claude-opus-4-6".to_string(),
                    variant: Some(ModelVariant::Max),
                },
                FallbackEntry {
                    providers: vec!["opencode-go".to_string()],
                    model: "kimi-k2.5".to_string(),
                    variant: None,
                },
                FallbackEntry {
                    providers: vec!["kimi-for-coding".to_string()],
                    model: "k2p5".to_string(),
                    variant: None,
                },
                FallbackEntry {
                    providers: vec!["openai".to_string(), "github-copilot".to_string(), "opencode".to_string()],
                    model: "gpt-5.4".to_string(),
                    variant: Some(ModelVariant::Medium),
                },
                FallbackEntry {
                    providers: vec!["zai-coding-plan".to_string(), "opencode".to_string()],
                    model: "glm-5".to_string(),
                    variant: None,
                },
            ],
            variant: None,
            requires_model: None,
            requires_any_model: true,
            requires_provider: None,
        });

        // Hephaestus - deep coding specialist (requires GPT)
        m.insert("hephaestus".to_string(), ModelRequirement {
            fallback_chain: vec![
                FallbackEntry {
                    providers: vec!["openai".to_string(), "venice".to_string(), "opencode".to_string()],
                    model: "gpt-5.3-codex".to_string(),
                    variant: Some(ModelVariant::Medium),
                },
                FallbackEntry {
                    providers: vec!["github-copilot".to_string()],
                    model: "gpt-5.4".to_string(),
                    variant: Some(ModelVariant::Medium),
                },
            ],
            variant: None,
            requires_model: None,
            requires_any_model: false,
            requires_provider: Some(vec!["openai".to_string(), "github-copilot".to_string(), "venice".to_string(), "opencode".to_string()]),
        });

        // Oracle - architecture consultation
        m.insert("oracle".to_string(), ModelRequirement {
            fallback_chain: vec![
                FallbackEntry {
                    providers: vec!["openai".to_string(), "github-copilot".to_string(), "opencode".to_string()],
                    model: "gpt-5.4".to_string(),
                    variant: Some(ModelVariant::High),
                },
                FallbackEntry {
                    providers: vec!["google".to_string(), "github-copilot".to_string(), "opencode".to_string()],
                    model: "gemini-3.1-pro".to_string(),
                    variant: Some(ModelVariant::High),
                },
                FallbackEntry {
                    providers: vec!["anthropic".to_string(), "github-copilot".to_string(), "opencode".to_string()],
                    model: "claude-opus-4-6".to_string(),
                    variant: Some(ModelVariant::Max),
                },
            ],
            variant: None,
            requires_model: None,
            requires_any_model: false,
            requires_provider: None,
        });

        // Explore - fast search (speed priority)
        m.insert("explore".to_string(), ModelRequirement {
            fallback_chain: vec![
                FallbackEntry {
                    providers: vec!["github-copilot".to_string()],
                    model: "grok-code-fast-1".to_string(),
                    variant: None,
                },
                FallbackEntry {
                    providers: vec!["opencode-go".to_string()],
                    model: "minimax-m2.5".to_string(),
                    variant: None,
                },
                FallbackEntry {
                    providers: vec!["anthropic".to_string(), "opencode".to_string()],
                    model: "claude-haiku-4-5".to_string(),
                    variant: None,
                },
            ],
            variant: None,
            requires_model: None,
            requires_any_model: false,
            requires_provider: None,
        });

        // Librarian - documentation lookup
        m.insert("librarian".to_string(), ModelRequirement {
            fallback_chain: vec![
                FallbackEntry {
                    providers: vec!["anthropic".to_string(), "opencode".to_string()],
                    model: "claude-haiku-4-5".to_string(),
                    variant: None,
                },
                FallbackEntry {
                    providers: vec!["google".to_string(), "opencode".to_string()],
                    model: "gemini-3-flash".to_string(),
                    variant: None,
                },
            ],
            variant: None,
            requires_model: None,
            requires_any_model: false,
            requires_provider: None,
        });

        // Metis - pre-planning consultant
        m.insert("metis".to_string(), ModelRequirement {
            fallback_chain: vec![
                FallbackEntry {
                    providers: vec!["anthropic".to_string(), "github-copilot".to_string(), "opencode".to_string()],
                    model: "claude-sonnet-4-6".to_string(),
                    variant: None,
                },
                FallbackEntry {
                    providers: vec!["openai".to_string(), "opencode".to_string()],
                    model: "gpt-5.4".to_string(),
                    variant: Some(ModelVariant::Medium),
                },
            ],
            variant: None,
            requires_model: None,
            requires_any_model: false,
            requires_provider: None,
        });

        // Momus - plan reviewer
        m.insert("momus".to_string(), ModelRequirement {
            fallback_chain: vec![
                FallbackEntry {
                    providers: vec!["anthropic".to_string(), "opencode".to_string()],
                    model: "claude-sonnet-4-6".to_string(),
                    variant: None,
                },
                FallbackEntry {
                    providers: vec!["google".to_string(), "opencode".to_string()],
                    model: "gemini-3-flash".to_string(),
                    variant: None,
                },
            ],
            variant: None,
            requires_model: None,
            requires_any_model: false,
            requires_provider: None,
        });

        // Plan - task planning
        m.insert("plan".to_string(), ModelRequirement {
            fallback_chain: vec![
                FallbackEntry {
                    providers: vec!["anthropic".to_string(), "github-copilot".to_string(), "opencode".to_string()],
                    model: "claude-sonnet-4-6".to_string(),
                    variant: None,
                },
                FallbackEntry {
                    providers: vec!["openai".to_string(), "opencode".to_string()],
                    model: "gpt-5.4".to_string(),
                    variant: Some(ModelVariant::Medium),
                },
                FallbackEntry {
                    providers: vec!["google".to_string(), "opencode".to_string()],
                    model: "gemini-3-flash".to_string(),
                    variant: None,
                },
            ],
            variant: None,
            requires_model: None,
            requires_any_model: false,
            requires_provider: None,
        });

        m
    };
}

/// Get model requirement for a category
pub fn get_category_requirement(category: &str) -> Option<&'static ModelRequirement> {
    CATEGORY_MODEL_REQUIREMENTS.get(category)
}

/// Get model requirement for an agent
pub fn get_agent_requirement(agent: &str) -> Option<&'static ModelRequirement> {
    AGENT_MODEL_REQUIREMENTS.get(agent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_requirements_exist() {
        assert!(CATEGORY_MODEL_REQUIREMENTS.contains_key("visual-engineering"));
        assert!(CATEGORY_MODEL_REQUIREMENTS.contains_key("ultrabrain"));
        assert!(CATEGORY_MODEL_REQUIREMENTS.contains_key("deep"));
        assert!(CATEGORY_MODEL_REQUIREMENTS.contains_key("artistry"));
        assert!(CATEGORY_MODEL_REQUIREMENTS.contains_key("quick"));
        assert!(CATEGORY_MODEL_REQUIREMENTS.contains_key("unspecified-low"));
        assert!(CATEGORY_MODEL_REQUIREMENTS.contains_key("unspecified-high"));
        assert!(CATEGORY_MODEL_REQUIREMENTS.contains_key("writing"));
    }

    #[test]
    fn test_category_requirements_count() {
        assert_eq!(CATEGORY_MODEL_REQUIREMENTS.len(), 8);
    }

    #[test]
    fn test_fallback_chain_length() {
        for (name, req) in CATEGORY_MODEL_REQUIREMENTS.iter() {
            assert!(
                req.fallback_chain.len() >= 2,
                "Category {} has insufficient fallback chain",
                name
            );
        }
    }

    #[test]
    fn test_agent_requirements_exist() {
        assert!(AGENT_MODEL_REQUIREMENTS.contains_key("sisyphus"));
        assert!(AGENT_MODEL_REQUIREMENTS.contains_key("hephaestus"));
        assert!(AGENT_MODEL_REQUIREMENTS.contains_key("oracle"));
        assert!(AGENT_MODEL_REQUIREMENTS.contains_key("explore"));
        assert!(AGENT_MODEL_REQUIREMENTS.contains_key("librarian"));
        assert!(AGENT_MODEL_REQUIREMENTS.contains_key("metis"));
        assert!(AGENT_MODEL_REQUIREMENTS.contains_key("momus"));
        assert!(AGENT_MODEL_REQUIREMENTS.contains_key("plan"));
    }

    #[test]
    fn test_deep_category_requires_model() {
        let req = CATEGORY_MODEL_REQUIREMENTS.get("deep").unwrap();
        assert!(req.requires_model.is_some());
        assert_eq!(req.requires_model.as_deref(), Some("gpt-5.3-codex"));
    }

    #[test]
    fn test_sisyphus_requires_any_model() {
        let req = AGENT_MODEL_REQUIREMENTS.get("sisyphus").unwrap();
        assert!(req.requires_any_model);
    }

    #[test]
    fn test_hephaestus_requires_provider() {
        let req = AGENT_MODEL_REQUIREMENTS.get("hephaestus").unwrap();
        assert!(req.requires_provider.is_some());
        let providers = req.requires_provider.as_ref().unwrap();
        assert!(providers.contains(&"openai".to_string()));
    }

    #[test]
    fn test_get_category_requirement() {
        assert!(get_category_requirement("quick").is_some());
        assert!(get_category_requirement("invalid").is_none());
    }

    #[test]
    fn test_get_agent_requirement() {
        assert!(get_agent_requirement("sisyphus").is_some());
        assert!(get_agent_requirement("invalid").is_none());
    }
}
