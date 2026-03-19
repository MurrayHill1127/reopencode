//! Skill module data structures
//!
//! Defines all core types for the skill system including scopes,
//! metadata, configuration, and discovery types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// ==================== Skill Scope ====================

/// Skill scope defining where a skill can be used
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum SkillScope {
    /// Project-level skill (local to current project)
    Project = 1,

    /// Opencode-level skill (available in opencode workspace)
    Opencode = 2,

    /// User-level skill (available to current user)
    User = 3,

    /// Built-in skill (shipped with the application)
    #[default]
    Builtin = 4,
}

// ==================== Skill Metadata ====================

/// Metadata associated with a skill
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SkillMetadata {
    /// Model hint for which model this skill works best with
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Argument hint for skill invocation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub argument_hint: Option<String>,

    /// Agent name this skill is associated with
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,

    /// Whether this skill is a subtask skill
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtask: Option<bool>,

    /// License information for the skill
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// Compatibility version information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility: Option<String>,

    /// Additional custom metadata key-value pairs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// List of allowed tools for this skill
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<Vec<String>>,
}

// ==================== Skill Info ====================

/// Complete skill information including content and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    /// Skill name
    pub name: String,

    /// Skill description
    pub description: String,

    /// File location of the skill
    pub location: PathBuf,

    /// Skill content/script
    pub content: String,

    /// Skill scope
    pub scope: SkillScope,

    /// Additional metadata (flattened in serialization)
    #[serde(flatten)]
    pub metadata: SkillMetadata,
}

impl SkillInfo {
    /// Create a new SkillInfo with default metadata
    pub fn new(name: String, description: String, scope: SkillScope) -> Self {
        Self {
            name,
            description,
            location: PathBuf::new(),
            content: String::new(),
            scope,
            metadata: SkillMetadata::default(),
        }
    }

    /// Create a SkillInfo with a location
    pub fn with_location(mut self, location: PathBuf) -> Self {
        self.location = location;
        self
    }

    /// Create a SkillInfo with content
    pub fn with_content(mut self, content: String) -> Self {
        self.content = content;
        self
    }
}

// ==================== Skill Config ====================

/// Configuration for skill discovery and loading
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SkillConfig {
    /// Additional custom paths to search for skills
    #[serde(default)]
    pub paths: Vec<String>,

    /// URLs to fetch remote skills from
    #[serde(default)]
    pub urls: Vec<String>,
}

// ==================== Discovery Options ====================

/// Options for skill discovery
#[derive(Debug, Clone)]
pub struct DiscoveryOptions {
    /// Base directory to search from
    pub directory: Option<PathBuf>,

    /// Include Claude Code skill paths
    pub include_claude_paths: bool,

    /// Disable fetching external/remote skills
    pub disable_external: bool,

    /// Maximum directory depth to search
    pub max_depth: usize,
}

impl Default for DiscoveryOptions {
    fn default() -> Self {
        Self {
            directory: None,
            include_claude_paths: true,
            disable_external: false,
            max_depth: 2,
        }
    }
}

// ==================== Discovery Result ====================

/// Result of skill discovery operation
#[derive(Debug, Clone, Default)]
pub struct DiscoveryResult {
    /// Discovered skills mapped by name
    pub skills: HashMap<String, SkillInfo>,

    /// Directories that were searched
    pub directories: Vec<PathBuf>,

    /// Warnings encountered during discovery
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== SkillScope Tests ====================

    #[test]
    fn test_skill_scope_ordering() {
        // Verify ordering: Project < Opencode < User < Builtin
        assert!(SkillScope::Project < SkillScope::Opencode);
        assert!(SkillScope::Opencode < SkillScope::User);
        assert!(SkillScope::User < SkillScope::Builtin);
    }

    #[test]
    fn test_skill_scope_default() {
        assert_eq!(SkillScope::default(), SkillScope::Builtin);
    }

    #[test]
    fn test_skill_scope_serialization() {
        // Test kebab-case serialization
        let project = serde_json::to_string(&SkillScope::Project).unwrap();
        assert_eq!(project, "\"project\"");

        let opencode = serde_json::to_string(&SkillScope::Opencode).unwrap();
        assert_eq!(opencode, "\"opencode\"");

        let user = serde_json::to_string(&SkillScope::User).unwrap();
        assert_eq!(user, "\"user\"");

        let builtin = serde_json::to_string(&SkillScope::Builtin).unwrap();
        assert_eq!(builtin, "\"builtin\"");
    }

    #[test]
    fn test_skill_scope_deserialization() {
        // Test kebab-case deserialization
        let project: SkillScope = serde_json::from_str("\"project\"").unwrap();
        assert_eq!(project, SkillScope::Project);

        let opencode: SkillScope = serde_json::from_str("\"opencode\"").unwrap();
        assert_eq!(opencode, SkillScope::Opencode);

        let user: SkillScope = serde_json::from_str("\"user\"").unwrap();
        assert_eq!(user, SkillScope::User);

        let builtin: SkillScope = serde_json::from_str("\"builtin\"").unwrap();
        assert_eq!(builtin, SkillScope::Builtin);
    }

    #[test]
    fn test_skill_scope_equality() {
        assert_eq!(SkillScope::Project, SkillScope::Project);
        assert_ne!(SkillScope::Project, SkillScope::Opencode);
    }

    #[test]
    fn test_skill_scope_copy() {
        let scope = SkillScope::User;
        let _copied = scope; // Should compile without error (Copy)
        let _cloned = scope; // Should compile without error (Clone)
    }

    // ==================== SkillMetadata Tests ====================

    #[test]
    fn test_skill_metadata_default() {
        let metadata = SkillMetadata::default();

        assert!(metadata.model.is_none());
        assert!(metadata.argument_hint.is_none());
        assert!(metadata.agent.is_none());
        assert!(metadata.subtask.is_none());
        assert!(metadata.license.is_none());
        assert!(metadata.compatibility.is_none());
        assert!(metadata.metadata.is_none());
        assert!(metadata.allowed_tools.is_none());
    }

    #[test]
    fn test_skill_metadata_serialization() {
        let metadata = SkillMetadata {
            model: Some("claude-sonnet-4-20250514".to_string()),
            argument_hint: Some("skill name".to_string()),
            agent: Some("build".to_string()),
            subtask: Some(true),
            license: Some("MIT".to_string()),
            compatibility: Some(">=1.0.0".to_string()),
            metadata: Some(HashMap::from([("version".to_string(), "1.0".to_string())])),
            allowed_tools: Some(vec!["read".to_string(), "write".to_string()]),
        };

        let json = serde_json::to_string(&metadata).unwrap();

        assert!(json.contains("\"model\""));
        assert!(json.contains("\"argument-hint\""));
        assert!(json.contains("\"agent\""));
        assert!(json.contains("\"subtask\":true"));
        assert!(json.contains("\"license\""));
        assert!(json.contains("\"compatibility\""));
    }

    #[test]
    fn test_skill_metadata_skip_none() {
        let metadata = SkillMetadata::default();
        let json = serde_json::to_string(&metadata).unwrap();

        // All optional fields should be skipped
        assert_eq!(json, "{}");
    }

    // ==================== SkillInfo Tests ====================

    #[test]
    fn test_skill_info_serialization() {
        let info = SkillInfo {
            name: "test-skill".to_string(),
            description: "A test skill".to_string(),
            location: PathBuf::from("/path/to/skill.md"),
            content: "# Test Skill\n\nThis is a test.".to_string(),
            scope: SkillScope::Project,
            metadata: SkillMetadata {
                model: Some("claude-sonnet-4-20250514".to_string()),
                ..Default::default()
            },
        };

        let json = serde_json::to_string(&info).unwrap();

        assert!(json.contains("\"name\":\"test-skill\""));
        assert!(json.contains("\"description\":\"A test skill\""));
        assert!(json.contains("\"scope\":\"project\""));
        assert!(json.contains("\"model\""));
    }

    #[test]
    fn test_skill_info_deserialization() {
        let json = r#"{
            "name": "my-skill",
            "description": "A sample skill",
            "location": "/skills/my-skill.md",
            "content": "skill content here",
            "scope": "user",
            "model": "claude-opus"
        }"#;

        let info: SkillInfo = serde_json::from_str(json).unwrap();

        assert_eq!(info.name, "my-skill");
        assert_eq!(info.description, "A sample skill");
        assert_eq!(info.scope, SkillScope::User);
        assert_eq!(info.metadata.model, Some("claude-opus".to_string()));
    }

    #[test]
    fn test_skill_info_flatten_metadata() {
        // Test that metadata fields are flattened in JSON
        let info = SkillInfo {
            name: "flatten-test".to_string(),
            description: "Testing flatten".to_string(),
            location: PathBuf::from("/test.md"),
            content: "content".to_string(),
            scope: SkillScope::Builtin,
            metadata: SkillMetadata {
                agent: Some("explore".to_string()),
                ..Default::default()
            },
        };

        let json = serde_json::to_string(&info).unwrap();

        // agent should appear at top level (flattened)
        assert!(json.contains("\"agent\":\"explore\""));
        assert!(!json.contains("\"metadata\":"));
    }

    // ==================== SkillConfig Tests ====================

    #[test]
    fn test_skill_config_default() {
        let config = SkillConfig::default();

        assert!(config.paths.is_empty());
        assert!(config.urls.is_empty());
    }

    #[test]
    fn test_skill_config_serialization() {
        let config = SkillConfig {
            paths: vec!["/custom/skills".to_string()],
            urls: vec!["https://example.com/skills".to_string()],
        };

        let json = serde_json::to_string(&config).unwrap();

        assert!(json.contains("\"paths\""));
        assert!(json.contains("\"urls\""));
    }

    #[test]
    fn test_skill_config_deserialization() {
        let json = r#"{
            "paths": ["/home/user/skills", "/project/skills"],
            "urls": ["https://api.example.com/skills"]
        }"#;

        let config: SkillConfig = serde_json::from_str(json).unwrap();

        assert_eq!(config.paths.len(), 2);
        assert_eq!(config.urls.len(), 1);
    }

    // ==================== DiscoveryOptions Tests ====================

    #[test]
    fn test_discovery_options_default() {
        let options = DiscoveryOptions::default();

        assert!(options.directory.is_none());
        assert!(options.include_claude_paths);
        assert!(!options.disable_external);
        assert_eq!(options.max_depth, 2);
    }

    #[test]
    fn test_discovery_options_builder() {
        let options = DiscoveryOptions {
            directory: Some(PathBuf::from("/custom/path")),
            include_claude_paths: false,
            disable_external: true,
            max_depth: 5,
        };

        assert_eq!(options.directory, Some(PathBuf::from("/custom/path")));
        assert!(!options.include_claude_paths);
        assert!(options.disable_external);
        assert_eq!(options.max_depth, 5);
    }

    // ==================== DiscoveryResult Tests ====================

    #[test]
    fn test_discovery_result_default() {
        let result = DiscoveryResult::default();

        assert!(result.skills.is_empty());
        assert!(result.directories.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_discovery_result_with_data() {
        let mut skills = HashMap::new();
        skills.insert(
            "test-skill".to_string(),
            SkillInfo {
                name: "test-skill".to_string(),
                description: "Test".to_string(),
                location: PathBuf::from("/test.md"),
                content: "content".to_string(),
                scope: SkillScope::Project,
                metadata: Default::default(),
            },
        );

        let result = DiscoveryResult {
            skills,
            directories: vec![PathBuf::from("/search/path")],
            warnings: vec!["Warning: skill not found".to_string()],
        };

        assert_eq!(result.skills.len(), 1);
        assert!(result.skills.contains_key("test-skill"));
        assert_eq!(result.directories.len(), 1);
        assert_eq!(result.warnings.len(), 1);
    }
}
