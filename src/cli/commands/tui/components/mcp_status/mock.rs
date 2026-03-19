//! Mock MCP Manager for Testing
//!
//! Provides a mock implementation of MCP manager functionality
//! for unit testing the McpStatusPanel component without requiring
//! actual MCP server connections.

use crate::mcp::types::{McpStatus, McpTool};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Mock MCP manager for testing
///
/// Provides controllable status and tool data for testing
/// the McpStatusPanel component's rendering and behavior.
///
/// # Examples
///
/// ```rust,ignore
/// let mut mock = MockMcpManager::new();
/// mock.add_server("server1", McpStatus::Connected);
/// mock.set_tool_count("server1", 5);
///
/// let statuses = mock.statuses();
/// assert_eq!(statuses.get("server1"), Some(&McpStatus::Connected));
/// ```
#[derive(Debug, Default)]
pub struct MockMcpManager {
    /// Server statuses (name -> status)
    statuses: Arc<Mutex<HashMap<String, McpStatus>>>,
    /// Tool counts (name -> count)
    tool_counts: Arc<Mutex<HashMap<String, usize>>>,
    /// Tools per server (name -> tools)
    tools: Arc<Mutex<HashMap<String, Vec<McpTool>>>>,
}

impl MockMcpManager {
    /// Create a new mock manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a server with the given status
    ///
    /// # Arguments
    ///
    /// * `name` - Server name
    /// * `status` - Server status
    pub fn add_server(&mut self, name: impl Into<String>, status: McpStatus) {
        let name = name.into();
        self.statuses.lock().unwrap().insert(name.clone(), status);
        self.tool_counts.lock().unwrap().insert(name, 0);
    }

    /// Remove a server
    ///
    /// # Arguments
    ///
    /// * `name` - Server name to remove
    pub fn remove_server(&mut self, name: &str) {
        self.statuses.lock().unwrap().remove(name);
        self.tool_counts.lock().unwrap().remove(name);
        self.tools.lock().unwrap().remove(name);
    }

    /// Set the status of a server
    ///
    /// # Arguments
    ///
    /// * `name` - Server name
    /// * `status` - New status
    pub fn set_status(&mut self, name: &str, status: McpStatus) {
        self.statuses
            .lock()
            .unwrap()
            .insert(name.to_string(), status);
    }

    /// Set the tool count for a server
    ///
    /// # Arguments
    ///
    /// * `name` - Server name
    /// * `count` - Number of tools
    pub fn set_tool_count(&mut self, name: &str, count: usize) {
        self.tool_counts
            .lock()
            .unwrap()
            .insert(name.to_string(), count);
    }

    /// Set the tools for a server
    ///
    /// # Arguments
    ///
    /// * `name` - Server name
    /// * `tools` - List of tools
    pub fn set_tools(&mut self, name: &str, tools: Vec<McpTool>) {
        let count = tools.len();
        self.tools.lock().unwrap().insert(name.to_string(), tools);
        self.tool_counts
            .lock()
            .unwrap()
            .insert(name.to_string(), count);
    }

    /// Get all server statuses
    ///
    /// # Returns
    ///
    /// A cloned HashMap of server names to statuses.
    pub fn statuses(&self) -> HashMap<String, McpStatus> {
        self.statuses.lock().unwrap().clone()
    }

    /// Get tool counts for all servers
    ///
    /// # Returns
    ///
    /// A cloned HashMap of server names to tool counts.
    pub fn tool_counts(&self) -> HashMap<String, usize> {
        self.tool_counts.lock().unwrap().clone()
    }

    /// Get tools for a specific server
    ///
    /// # Arguments
    ///
    /// * `name` - Server name
    ///
    /// # Returns
    ///
    /// A cloned Vec of tools, or empty Vec if server not found.
    pub fn tools(&self, name: &str) -> Vec<McpTool> {
        self.tools
            .lock()
            .unwrap()
            .get(name)
            .cloned()
            .unwrap_or_default()
    }

    /// Clear all servers
    pub fn clear(&mut self) {
        self.statuses.lock().unwrap().clear();
        self.tool_counts.lock().unwrap().clear();
        self.tools.lock().unwrap().clear();
    }

    /// Simulate a status change
    ///
    /// Changes a server's status to simulate real-world events
    /// like connection loss or authentication requirements.
    ///
    /// # Arguments
    ///
    /// * `name` - Server name
    /// * `new_status` - New status to set
    pub fn simulate_status_change(&mut self, name: &str, new_status: McpStatus) {
        self.set_status(name, new_status);
    }

    /// Create a mock tool
    ///
    /// Helper function to create a mock McpTool for testing.
    ///
    /// # Arguments
    ///
    /// * `name` - Tool name
    /// * `description` - Optional tool description
    ///
    /// # Returns
    ///
    /// A new `McpTool` instance.
    pub fn create_mock_tool(name: &str, description: Option<&str>) -> McpTool {
        McpTool {
            name: name.to_string(),
            description: description.map(|d| d.to_string()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_manager_new() {
        let mock = MockMcpManager::new();
        assert!(mock.statuses().is_empty());
        assert!(mock.tool_counts().is_empty());
    }

    #[test]
    fn test_mock_manager_add_server() {
        let mut mock = MockMcpManager::new();
        mock.add_server("test-server", McpStatus::Connected);

        let statuses = mock.statuses();
        assert_eq!(statuses.len(), 1);
        assert!(matches!(
            statuses.get("test-server"),
            Some(McpStatus::Connected)
        ));
    }

    #[test]
    fn test_mock_manager_set_status() {
        let mut mock = MockMcpManager::new();
        mock.add_server("test", McpStatus::Connected);
        mock.set_status(
            "test",
            McpStatus::Failed {
                error: "boom".into(),
            },
        );

        let statuses = mock.statuses();
        assert!(matches!(
            statuses.get("test"),
            Some(McpStatus::Failed { .. })
        ));
    }

    #[test]
    fn test_mock_manager_set_tool_count() {
        let mut mock = MockMcpManager::new();
        mock.add_server("test", McpStatus::Connected);
        mock.set_tool_count("test", 5);

        assert_eq!(mock.tool_counts().get("test"), Some(&5));
    }

    #[test]
    fn test_mock_manager_remove_server() {
        let mut mock = MockMcpManager::new();
        mock.add_server("test", McpStatus::Connected);
        mock.remove_server("test");

        assert!(mock.statuses().is_empty());
    }

    #[test]
    fn test_mock_manager_clear() {
        let mut mock = MockMcpManager::new();
        mock.add_server("s1", McpStatus::Connected);
        mock.add_server(
            "s2",
            McpStatus::Failed {
                error: "err".into(),
            },
        );
        mock.clear();

        assert!(mock.statuses().is_empty());
        assert!(mock.tool_counts().is_empty());
    }

    #[test]
    fn test_mock_manager_simulate_status_change() {
        let mut mock = MockMcpManager::new();
        mock.add_server("test", McpStatus::Connected);
        mock.simulate_status_change("test", McpStatus::NeedsAuth);

        assert!(matches!(
            mock.statuses().get("test"),
            Some(McpStatus::NeedsAuth)
        ));
    }

    #[test]
    fn test_mock_manager_create_mock_tool() {
        let tool = MockMcpManager::create_mock_tool("test_tool", Some("A test tool"));

        assert_eq!(tool.name, "test_tool");
        assert_eq!(tool.description, Some("A test tool".to_string()));
    }
}
