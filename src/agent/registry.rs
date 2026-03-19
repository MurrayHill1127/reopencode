//! Agent registry and builtin agent definitions

#![allow(dead_code)]

use std::collections::HashMap;

use crate::agent::config::{AgentInfo, AgentMode};
use crate::agent::permission::{Action, PermissionEngine, Rule};
use crate::agent::prompts;

/// Agent registry for managing available agents
#[derive(Debug, Default)]
pub struct AgentRegistry {
    agents: HashMap<String, AgentInfo>,
}

impl AgentRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        let mut registry = Self {
            agents: HashMap::new(),
        };
        registry.register_builtin_agents();
        registry
    }

    /// Register all builtin agents
    fn register_builtin_agents(&mut self) {
        self.register(build_agent());
        self.register(plan_agent());
        self.register(explore_agent());
        self.register(compaction_agent());
        self.register(title_agent());
        self.register(summary_agent());
    }

    /// Register an agent
    pub fn register(&mut self, info: AgentInfo) {
        self.agents.insert(info.name.clone(), info);
    }

    /// Get an agent by name
    pub fn get(&self, name: &str) -> Option<&AgentInfo> {
        self.agents.get(name)
    }

    /// List all agent names
    pub fn list(&self) -> Vec<&str> {
        self.agents.keys().map(|s| s.as_str()).collect()
    }

    /// List only visible (non-hidden) agents
    pub fn list_visible(&self) -> Vec<&AgentInfo> {
        self.agents.values().filter(|a| !a.hidden).collect()
    }

    /// Get the default agent name
    pub fn default_agent(&self) -> Option<&str> {
        self.agents
            .values()
            .find(|a| a.mode == AgentMode::Primary && !a.hidden)
            .map(|a| a.name.as_str())
    }
}

/// Create the build agent (primary, native, default executor)
fn build_agent() -> AgentInfo {
    let mut perm = PermissionEngine::new();
    perm.add_rules(vec![
        Rule::new("*", "*", Action::Ask),
        Rule::new("bash", "git *", Action::Allow),
        Rule::new("read", "*", Action::Allow),
    ]);

    AgentInfo {
        name: "build".to_string(),
        description: Some(
            "The default agent. Executes tools based on configured permissions.".to_string(),
        ),
        mode: AgentMode::Primary,
        native: true,
        hidden: false,
        permission: perm.rules().to_vec(),
        ..Default::default()
    }
}

/// Create the plan agent (primary, native, disallows edit tools)
fn plan_agent() -> AgentInfo {
    let mut perm = PermissionEngine::new();
    perm.add_rules(vec![
        Rule::new("*", "*", Action::Ask),
        Rule::new("edit", "*", Action::Deny),
        Rule::new("write", "*", Action::Deny),
        Rule::new("bash", "git *", Action::Allow),
        Rule::new("read", "*", Action::Allow),
    ]);

    AgentInfo {
        name: "plan".to_string(),
        description: Some("Plan mode. Disallows all edit tools.".to_string()),
        mode: AgentMode::Primary,
        native: true,
        hidden: false,
        permission: perm.rules().to_vec(),
        ..Default::default()
    }
}

/// Create the explore agent (subagent, native, read-only tools)
fn explore_agent() -> AgentInfo {
    let mut perm = PermissionEngine::new();
    perm.add_rules(vec![
        Rule::new("*", "*", Action::Deny),
        Rule::new("grep", "*", Action::Allow),
        Rule::new("glob", "*", Action::Allow),
        Rule::new("read", "*", Action::Allow),
        Rule::new("bash", "git *", Action::Allow),
        Rule::new("webfetch", "*", Action::Allow),
        Rule::new("websearch", "*", Action::Allow),
    ]);

    AgentInfo {
        name: "explore".to_string(),
        description: Some("Fast agent specialized for exploring codebases. Use this when you need to quickly find files by patterns, search code for keywords, or answer questions about the codebase.".to_string()),
        mode: AgentMode::Subagent,
        native: true,
        hidden: false,
        prompt: prompts::get_template("explore").map(|s| s.to_string()),
        permission: perm.rules().to_vec(),
        ..Default::default()
    }
}

/// Create the compaction agent (primary, native, hidden)
fn compaction_agent() -> AgentInfo {
    AgentInfo {
        name: "compaction".to_string(),
        description: Some("Conversation summarizer for context compression".to_string()),
        mode: AgentMode::Primary,
        native: true,
        hidden: true,
        prompt: prompts::get_template("compaction").map(|s| s.to_string()),
        ..Default::default()
    }
}

/// Create the title agent (primary, native, hidden)
fn title_agent() -> AgentInfo {
    AgentInfo {
        name: "title".to_string(),
        description: Some("Session title generator".to_string()),
        mode: AgentMode::Primary,
        native: true,
        hidden: true,
        temperature: Some(0.5),
        prompt: prompts::get_template("title").map(|s| s.to_string()),
        ..Default::default()
    }
}

/// Create the summary agent (primary, native, hidden)
fn summary_agent() -> AgentInfo {
    AgentInfo {
        name: "summary".to_string(),
        description: Some("PR description style summarizer".to_string()),
        mode: AgentMode::Primary,
        native: true,
        hidden: true,
        prompt: prompts::get_template("summary").map(|s| s.to_string()),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_has_all_builtins() {
        let registry = AgentRegistry::new();
        let agents = registry.list();

        assert!(agents.contains(&"build"));
        assert!(agents.contains(&"plan"));
        assert!(agents.contains(&"explore"));
        assert!(agents.contains(&"compaction"));
        assert!(agents.contains(&"title"));
        assert!(agents.contains(&"summary"));
    }

    #[test]
    fn test_get_agent() {
        let registry = AgentRegistry::new();

        let build = registry.get("build").unwrap();
        assert_eq!(build.name, "build");
        assert_eq!(build.mode, AgentMode::Primary);
        assert!(!build.hidden);

        let explore = registry.get("explore").unwrap();
        assert_eq!(explore.mode, AgentMode::Subagent);
    }

    #[test]
    fn test_hidden_agents() {
        let registry = AgentRegistry::new();

        let compaction = registry.get("compaction").unwrap();
        assert!(compaction.hidden);

        let title = registry.get("title").unwrap();
        assert!(title.hidden);

        let summary = registry.get("summary").unwrap();
        assert!(summary.hidden);
    }

    #[test]
    fn test_list_visible() {
        let registry = AgentRegistry::new();
        let visible = registry.list_visible();

        // build, plan, explore are visible
        assert_eq!(visible.len(), 3);
        assert!(visible.iter().all(|a| !a.hidden));
    }

    #[test]
    fn test_default_agent() {
        let registry = AgentRegistry::new();
        let default = registry.default_agent();

        assert!(default.is_some());
        // Default should be a primary non-hidden agent (build or plan)
        let name = default.unwrap();
        let agent = registry.get(name).unwrap();
        assert_eq!(agent.mode, AgentMode::Primary);
        assert!(!agent.hidden);
    }

    #[test]
    fn test_explore_has_prompt() {
        let registry = AgentRegistry::new();
        let explore = registry.get("explore").unwrap();

        assert!(explore.prompt.is_some());
        assert!(explore.prompt.as_ref().unwrap().contains("file search"));
    }

    #[test]
    fn test_register_custom_agent() {
        let mut registry = AgentRegistry::new();
        let initial_count = registry.list().len();

        let custom = AgentInfo {
            name: "custom".to_string(),
            mode: AgentMode::Subagent,
            ..Default::default()
        };
        registry.register(custom);

        assert_eq!(registry.list().len(), initial_count + 1);
        assert!(registry.get("custom").is_some());
    }
}
