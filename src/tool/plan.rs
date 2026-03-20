//! Plan tools - plan mode switching
//!
//! This is a placeholder implementation. Full implementation requires
//! session module integration for agent switching.

use async_trait::async_trait;
use serde_json::Value;

use crate::tool::error::Result;
use crate::tool::traits::{Tool, ToolResult};

/// Plan exit tool - exit plan mode and switch to build agent
pub struct PlanExitTool;

impl PlanExitTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for PlanExitTool {
    fn name(&self) -> &str {
        "plan_exit"
    }

    fn description(&self) -> &str {
        r#"Use this tool when you have completed the planning phase and are ready to exit plan agent.

This tool will ask the user if they want to switch to build agent to start implementing the plan.

Call this tool:
- After you have written a complete plan to the plan file
- After you have clarified any questions with the user
- When you are confident the plan is ready for implementation

Do NOT call this tool:
- Before you have created or finalized the plan
- If you still have unanswered questions about the implementation
- If the user has indicated they want to continue planning"#
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _args: Value) -> Result<ToolResult> {
        // Placeholder implementation - session module integration required
        let output = r#"Plan exit requires session module integration.

This tool would:
1. Ask the user if they want to switch to build agent
2. Create a new user message to switch agent mode
3. Transition from plan agent to build agent

The plan file location would be determined by the session configuration."#;

        Ok(ToolResult {
            output: output.to_string(),
            metadata: Some(serde_json::json!({
                "placeholder": true,
                "action": "plan_exit"
            })),
        })
    }
}

impl Default for PlanExitTool {
    fn default() -> Self {
        Self::new()
    }
}

/// Plan enter tool - enter plan mode from build agent
///
/// Note: This tool is currently disabled in the TypeScript implementation.
/// Keeping the struct for future use.
pub struct PlanEnterTool;

impl PlanEnterTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for PlanEnterTool {
    fn name(&self) -> &str {
        "plan_enter"
    }

    fn description(&self) -> &str {
        r#"Use this tool to switch from build agent to plan agent for research and planning.

This tool will ask the user if they want to enter plan mode.

Call this tool:
- When you need to do research before making changes
- When you want to create a detailed implementation plan
- When the user requests planning mode

Do NOT call this tool:
- If you are already in plan mode
- If you are actively implementing changes"#
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _args: Value) -> Result<ToolResult> {
        // Placeholder implementation - session module integration required
        let output = r#"Plan enter requires session module integration.

This tool would:
1. Ask the user if they want to enter plan mode
2. Create a new user message to switch agent mode
3. Transition from build agent to plan agent

The plan file would be created at a location determined by session configuration."#;

        Ok(ToolResult {
            output: output.to_string(),
            metadata: Some(serde_json::json!({
                "placeholder": true,
                "action": "plan_enter"
            })),
        })
    }
}

impl Default for PlanEnterTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // PlanExitTool tests
    #[test]
    fn test_plan_exit_tool_new() {
        let tool = PlanExitTool::new();
        assert_eq!(tool.name(), "plan_exit");
    }

    #[test]
    fn test_plan_exit_tool_default() {
        let tool: PlanExitTool = Default::default();
        assert_eq!(tool.name(), "plan_exit");
    }

    #[test]
    fn test_plan_exit_tool_parameters() {
        let tool = PlanExitTool::new();
        let params = tool.parameters();

        assert_eq!(params["type"], "object");
        assert!(params["properties"].as_object().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_plan_exit_execute() {
        let tool = PlanExitTool::new();
        let args = serde_json::json!({});

        let result = tool.execute(args).await.unwrap();
        assert!(result.output.contains("session module integration"));
        assert!(result.metadata.is_some());
        let metadata = result.metadata.unwrap();
        assert_eq!(metadata["action"], "plan_exit");
    }

    #[test]
    fn test_plan_exit_description() {
        let tool = PlanExitTool::new();
        let desc = tool.description();
        assert!(desc.contains("planning phase"));
        assert!(desc.contains("build agent"));
    }

    // PlanEnterTool tests
    #[test]
    fn test_plan_enter_tool_new() {
        let tool = PlanEnterTool::new();
        assert_eq!(tool.name(), "plan_enter");
    }

    #[test]
    fn test_plan_enter_tool_default() {
        let tool: PlanEnterTool = Default::default();
        assert_eq!(tool.name(), "plan_enter");
    }

    #[test]
    fn test_plan_enter_tool_parameters() {
        let tool = PlanEnterTool::new();
        let params = tool.parameters();

        assert_eq!(params["type"], "object");
        assert!(params["properties"].as_object().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_plan_enter_execute() {
        let tool = PlanEnterTool::new();
        let args = serde_json::json!({});

        let result = tool.execute(args).await.unwrap();
        assert!(result.output.contains("session module integration"));
        assert!(result.metadata.is_some());
        let metadata = result.metadata.unwrap();
        assert_eq!(metadata["action"], "plan_enter");
    }

    #[test]
    fn test_plan_enter_description() {
        let tool = PlanEnterTool::new();
        let desc = tool.description();
        assert!(desc.contains("plan mode"));
        assert!(desc.contains("research and planning"));
    }
}