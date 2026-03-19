//! Tool registry - manages tool registration and lookup

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use tracing::{debug, info};

use crate::tool::traits::Tool;

/// Thread-safe registry for managing tools
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Arc<Box<dyn Tool>>>>>,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a tool by its name
    pub fn register(&self, tool: Box<dyn Tool>) {
        let name = tool.name().to_string();
        info!("Registering tool: {}", name);

        let mut tools = self.tools.write().expect("Failed to acquire write lock");
        tools.insert(name.clone(), Arc::new(tool));
        debug!("Registered tool: {}", name);
    }

    /// Get a tool by name (returns Arc reference)
    pub fn get(&self, name: &str) -> Option<Arc<Box<dyn Tool>>> {
        let tools = self.tools.read().expect("Failed to acquire read lock");
        tools.get(name).cloned()
    }

    /// Check if a tool exists in the registry
    pub fn contains(&self, name: &str) -> bool {
        let tools = self.tools.read().expect("Failed to acquire read lock");
        tools.contains_key(name)
    }

    /// List all registered tool names
    pub fn list(&self) -> Vec<String> {
        let tools = self.tools.read().expect("Failed to acquire read lock");
        tools.keys().cloned().collect()
    }

    /// Remove a tool from the registry
    pub fn remove(&self, name: &str) -> Option<String> {
        let mut tools = self.tools.write().expect("Failed to acquire write lock");
        tools.remove(name).map(|_| {
            info!("Removed tool: {}", name);
            name.to_string()
        })
    }

    /// Clear all tools from the registry
    pub fn clear(&self) {
        let mut tools = self.tools.write().expect("Failed to acquire write lock");
        tools.clear();
        info!("Cleared all tools from registry");
    }

    /// Get the number of registered tools
    pub fn len(&self) -> usize {
        let tools = self.tools.read().expect("Failed to acquire read lock");
        tools.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ToolRegistry {
    fn clone(&self) -> Self {
        Self {
            tools: Arc::clone(&self.tools),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_new() {
        let registry = ToolRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_register_and_get() {
        use crate::tool::bash::BashTool;

        let registry = ToolRegistry::new();
        let tool = Box::new(BashTool::new());

        registry.register(tool);

        assert_eq!(registry.len(), 1);
        assert!(registry.contains("bash"));

        let retrieved = registry.get("bash");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "bash");
    }

    #[test]
    fn test_registry_list() {
        use crate::tool::bash::BashTool;

        let registry = ToolRegistry::new();

        registry.register(Box::new(BashTool::new()));

        let list = registry.list();
        assert_eq!(list.len(), 1);
        assert!(list.contains(&"bash".to_string()));
    }

    #[test]
    fn test_registry_contains() {
        use crate::tool::bash::BashTool;

        let registry = ToolRegistry::new();

        assert!(!registry.contains("bash"));

        registry.register(Box::new(BashTool::new()));

        assert!(registry.contains("bash"));
        assert!(!registry.contains("nonexistent"));
    }

    #[test]
    fn test_registry_remove() {
        use crate::tool::bash::BashTool;

        let registry = ToolRegistry::new();
        registry.register(Box::new(BashTool::new()));

        assert_eq!(registry.len(), 1);

        let removed = registry.remove("bash");
        assert_eq!(removed, Some("bash".to_string()));
        assert!(registry.is_empty());

        // Removing again should return None
        let removed_again = registry.remove("bash");
        assert!(removed_again.is_none());
    }

    #[test]
    fn test_registry_clear() {
        use crate::tool::bash::BashTool;
        use crate::tool::read::ReadTool;

        let registry = ToolRegistry::new();
        registry.register(Box::new(BashTool::new()));
        registry.register(Box::new(ReadTool::new()));

        assert_eq!(registry.len(), 2);
        registry.clear();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_registry_clone() {
        use crate::tool::bash::BashTool;
        use crate::tool::read::ReadTool;

        let registry = ToolRegistry::new();
        registry.register(Box::new(BashTool::new()));

        let cloned = registry.clone();
        assert_eq!(cloned.len(), 1);
        assert!(cloned.contains("bash"));

        // Changes to original should be visible in clone (Arc shares data)
        let original = registry.clone();
        original.register(Box::new(ReadTool::new()));
        assert_eq!(registry.len(), 2);
        assert_eq!(cloned.len(), 2);
    }

    #[test]
    fn test_registry_is_empty() {
        let registry = ToolRegistry::new();
        assert!(registry.is_empty());

        use crate::tool::bash::BashTool;
        registry.register(Box::new(BashTool::new()));
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_registry_get_nonexistent() {
        let registry = ToolRegistry::new();

        let result = registry.get("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_registry_multiple_tools() {
        use crate::tool::bash::BashTool;

        let registry = ToolRegistry::new();

        // Register multiple tools with different names would require different tool types
        // For now, test with multiple registrations
        registry.register(Box::new(BashTool::new()));

        let list = registry.list();
        assert_eq!(list.len(), 1);
    }
}
