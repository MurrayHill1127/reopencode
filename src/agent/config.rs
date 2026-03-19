//! Agent configuration types
//!
//! This module defines the configuration structures for AI agents,
//! including agent metadata, mode settings, and model configuration.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Agent operating mode
///
/// Determines how the agent can be used within the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentMode {
    /// Primary agent - can be used as the main agent
    #[default]
    Primary,
    /// Subagent - can only be invoked as a sub-agent
    Subagent,
    /// Can be used in any mode
    All,
}

/// Model configuration
///
/// Specifies which model and provider to use for an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// The model identifier (e.g., "claude-3-opus-20240229")
    pub model_id: String,
    /// The provider identifier (e.g., "anthropic", "openai")
    pub provider_id: String,
}

/// Agent information and configuration
///
/// This struct contains all the metadata and settings for an agent,
/// including its name, description, permissions, and model configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// The unique name of the agent
    pub name: String,
    /// Optional description of the agent's purpose and capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The agent's operating mode (primary, subagent, or all)
    #[serde(default)]
    pub mode: AgentMode,
    /// Whether this is a native/system agent
    #[serde(default)]
    pub native: bool,
    /// Whether the agent should be hidden from listings
    #[serde(default)]
    pub hidden: bool,
    /// Top-p sampling parameter for the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Temperature parameter for the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Color identifier for UI display
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Permission rules for this agent
    #[serde(default)]
    pub permission: Vec<crate::agent::permission::Rule>,
    /// Model configuration for this agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<ModelConfig>,
    /// Variant identifier for model selection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    /// Custom system prompt for this agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// Additional options for the agent
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub options: serde_json::Map<String, serde_json::Value>,
    /// Maximum number of steps for this agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<usize>,
}

impl Default for AgentInfo {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: None,
            mode: AgentMode::default(),
            native: false,
            hidden: false,
            top_p: None,
            temperature: None,
            color: None,
            permission: Vec::new(),
            model: None,
            variant: None,
            prompt: None,
            options: serde_json::Map::new(),
            steps: None,
        }
    }
}
