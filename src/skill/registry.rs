//! Skill registry with priority-based override
//!
//! Provides registration, lookup, and management capabilities for skills
//! with scope-based priority handling.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::{DiscoveryOptions, SkillError, SkillInfo, SkillScope};

/// Skill registry for managing registered skills
pub struct SkillRegistry {
    skills: Arc<RwLock<HashMap<String, SkillInfo>>>,
    directories: Arc<RwLock<Vec<PathBuf>>>,
    initialized: Arc<RwLock<bool>>,
}

impl SkillRegistry {
    /// Get global singleton instance
    pub fn global() -> &'static Self {
        static INSTANCE: std::sync::OnceLock<SkillRegistry> = std::sync::OnceLock::new();
        INSTANCE.get_or_init(Self::new)
    }

    /// Create new instance
    pub fn new() -> Self {
        Self {
            skills: Arc::new(RwLock::new(HashMap::new())),
            directories: Arc::new(RwLock::new(Vec::new())),
            initialized: Arc::new(RwLock::new(false)),
        }
    }

    /// Register single skill with priority override logic
    ///
    /// If a skill with the same name exists, compares scopes:
    /// - Project < Opencode < User < Builtin (lower variant = higher priority)
    /// - Only overrides if new skill has higher priority
    pub async fn register(&self, skill: SkillInfo) -> Result<(), SkillError> {
        let mut skills = self.skills.write().await;
        let skill_name = skill.name.clone();

        if let Some(existing) = skills.get(&skill_name) {
            if skill.scope >= existing.scope {
                tracing::debug!(
                    "Skill '{}' already registered with higher priority, skipping",
                    skill_name
                );
                return Ok(());
            }

            tracing::info!("Overriding skill '{}' scope", skill_name);
        }

        skills.insert(skill_name, skill);
        Ok(())
    }

    /// Register multiple skills at once
    pub async fn register_all(&self, skills: HashMap<String, SkillInfo>) {
        let mut registry = self.skills.write().await;

        for (name, skill) in skills {
            if let Some(existing) = registry.get(&name) {
                if skill.scope >= existing.scope {
                    tracing::debug!(
                        "Skill '{}' already registered with higher priority, skipping",
                        name
                    );
                    continue;
                }
                tracing::info!("Overriding skill '{}' during batch registration", name);
            }
            registry.insert(name, skill);
        }
    }

    /// Get skill by name
    pub async fn get(&self, name: &str) -> Option<SkillInfo> {
        let skills = self.skills.read().await;
        skills.get(name).cloned()
    }

    /// Get all registered skills
    pub async fn all(&self) -> Vec<SkillInfo> {
        let skills = self.skills.read().await;
        skills.values().cloned().collect()
    }

    /// Get all registered directories
    pub async fn directories(&self) -> Vec<PathBuf> {
        let dirs = self.directories.read().await;
        dirs.clone()
    }

    /// Add a directory to search list
    pub async fn add_directory(&self, dir: PathBuf) {
        let mut dirs = self.directories.write().await;
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }

    /// Clear all registered skills
    pub async fn clear(&self) {
        let mut skills = self.skills.write().await;
        skills.clear();
        let mut dirs = self.directories.write().await;
        dirs.clear();
        let mut initialized = self.initialized.write().await;
        *initialized = false;
    }

    /// Initialize registry from discovery options
    pub async fn initialize(&self, _options: DiscoveryOptions) -> Result<(), SkillError> {
        let mut initialized = self.initialized.write().await;
        *initialized = true;
        tracing::info!("Skill registry initialized");
        Ok(())
    }

    /// Check if registry has been initialized
    pub async fn is_initialized(&self) -> bool {
        let initialized = self.initialized.read().await;
        *initialized
    }

    /// Returns the number of registered skills
    pub async fn len(&self) -> usize {
        let skills = self.skills.read().await;
        skills.len()
    }

    /// Returns true if no skills are registered
    pub async fn is_empty(&self) -> bool {
        let skills = self.skills.read().await;
        skills.is_empty()
    }

    /// Get skills filtered by scope
    pub async fn by_scope(&self, scope: SkillScope) -> Vec<SkillInfo> {
        let skills = self.skills.read().await;
        skills.values().filter(|s| s.scope == scope).cloned().collect()
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_skill(name: &str, scope: SkillScope) -> SkillInfo {
        SkillInfo::new(name.to_string(), format!("Test skill: {}", name), scope)
    }

    #[tokio::test]
    async fn test_registry_new() {
        let registry = SkillRegistry::new();
        assert!(registry.is_empty().await);
        assert_eq!(registry.len().await, 0);
        assert!(!registry.is_initialized().await);
    }

    #[tokio::test]
    async fn test_register_and_get_skill() {
        let registry = SkillRegistry::new();
        let skill = create_test_skill("test-skill", SkillScope::Project);

        registry.register(skill.clone()).await.unwrap();

        assert_eq!(registry.len().await, 1);
        assert!(!registry.is_empty().await);

        let retrieved = registry.get("test-skill").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test-skill");
    }

    #[tokio::test]
    async fn test_priority_override_project_over_user() {
        let registry = SkillRegistry::new();

        let user_skill = create_test_skill("my-skill", SkillScope::User);
        registry.register(user_skill).await.unwrap();

        let project_skill = create_test_skill("my-skill", SkillScope::Project);
        registry.register(project_skill).await.unwrap();

        assert_eq!(registry.len().await, 1);

        let skill = registry.get("my-skill").await.unwrap();
        assert_eq!(skill.scope, SkillScope::Project);
    }

    #[tokio::test]
    async fn test_priority_no_override_user_over_project() {
        let registry = SkillRegistry::new();

        let project_skill = create_test_skill("priority-skill", SkillScope::Project);
        registry.register(project_skill).await.unwrap();

        let user_skill = create_test_skill("priority-skill", SkillScope::User);
        registry.register(user_skill).await.unwrap();

        let skill = registry.get("priority-skill").await.unwrap();
        assert_eq!(skill.scope, SkillScope::Project);
    }

    #[tokio::test]
    async fn test_register_all_multiple_skills() {
        let registry = SkillRegistry::new();

        let mut skills = HashMap::new();
        skills.insert("skill1".to_string(), create_test_skill("skill1", SkillScope::Project));
        skills.insert("skill2".to_string(), create_test_skill("skill2", SkillScope::User));
        skills.insert("skill3".to_string(), create_test_skill("skill3", SkillScope::Opencode));

        registry.register_all(skills).await;

        assert_eq!(registry.len().await, 3);
    }

    #[tokio::test]
    async fn test_all_skills() {
        let registry = SkillRegistry::new();

        registry
            .register(create_test_skill("alpha", SkillScope::Project))
            .await
            .unwrap();
        registry
            .register(create_test_skill("beta", SkillScope::User))
            .await
            .unwrap();

        let all = registry.all().await;
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_clear_registry() {
        let registry = SkillRegistry::new();

        registry
            .register(create_test_skill("to-clear", SkillScope::Project))
            .await
            .unwrap();
        registry.add_directory(PathBuf::from("/test/dir")).await;

        assert!(!registry.is_empty().await);
        assert!(!registry.directories().await.is_empty());

        registry.clear().await;

        assert!(registry.is_empty().await);
        assert!(registry.directories().await.is_empty());
        assert!(!registry.is_initialized().await);
    }

    #[tokio::test]
    async fn test_directories() {
        let registry = SkillRegistry::new();

        registry.add_directory(PathBuf::from("/dir1")).await;
        registry.add_directory(PathBuf::from("/dir2")).await;
        registry.add_directory(PathBuf::from("/dir1")).await;

        let dirs = registry.directories().await;
        assert_eq!(dirs.len(), 2);
    }

    #[tokio::test]
    async fn test_global_singleton() {
        let global1 = SkillRegistry::global();
        let global2 = SkillRegistry::global();

        assert!(std::ptr::eq(global1, global2));

        global1
            .register(create_test_skill("singleton-test", SkillScope::Project))
            .await
            .unwrap();

        let skill = global2.get("singleton-test").await;
        assert!(skill.is_some());

        global1.clear().await;
    }

    #[tokio::test]
    async fn test_by_scope() {
        let registry = SkillRegistry::new();

        registry
            .register(create_test_skill("proj1", SkillScope::Project))
            .await
            .unwrap();
        registry
            .register(create_test_skill("proj2", SkillScope::Project))
            .await
            .unwrap();
        registry
            .register(create_test_skill("user1", SkillScope::User))
            .await
            .unwrap();

        let project_skills = registry.by_scope(SkillScope::Project).await;
        assert_eq!(project_skills.len(), 2);

        let user_skills = registry.by_scope(SkillScope::User).await;
        assert_eq!(user_skills.len(), 1);
    }

    #[tokio::test]
    async fn test_initialize() {
        let registry = SkillRegistry::new();

        assert!(!registry.is_initialized().await);

        let options = DiscoveryOptions {
            directory: Some(PathBuf::from("/test")),
            include_claude_paths: true,
            disable_external: false,
            max_depth: 2,
        };

        registry.initialize(options).await.unwrap();

        assert!(registry.is_initialized().await);
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let registry = SkillRegistry::new();

        let result = registry.get("nonexistent").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_scope_priority_ordering() {
        assert!(SkillScope::Project < SkillScope::Opencode);
        assert!(SkillScope::Opencode < SkillScope::User);
        assert!(SkillScope::Project < SkillScope::User);
    }

    #[tokio::test]
    async fn test_register_all_with_priority_override() {
        let registry = SkillRegistry::new();

        let mut batch1 = HashMap::new();
        batch1.insert("skill-a".to_string(), create_test_skill("skill-a", SkillScope::User));
        registry.register_all(batch1).await;

        let mut batch2 = HashMap::new();
        batch2.insert("skill-a".to_string(), create_test_skill("skill-a", SkillScope::Project));
        registry.register_all(batch2).await;

        let mut batch3 = HashMap::new();
        batch3.insert("skill-a".to_string(), create_test_skill("skill-a", SkillScope::Opencode));
        registry.register_all(batch3).await;

        let skill = registry.get("skill-a").await.unwrap();
        assert_eq!(skill.scope, SkillScope::Project);
    }
}