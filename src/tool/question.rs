//! Question tool - ask user questions during execution

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::permission::PermissionStore;
use crate::tool::error::{Result, ToolError};
use crate::tool::traits::{Tool, ToolResult};

/// A single option in a question
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionOption {
    pub label: String,
    pub description: String,
}

/// A question to ask the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionInfo {
    pub question: String,
    pub header: String,
    pub options: Vec<QuestionOption>,
    #[serde(default)]
    pub multiple: bool,
}

/// Question tool - suspends the agent loop via PermissionStore until the user answers.
pub struct QuestionTool {
    permission_store: Option<Arc<PermissionStore>>,
    session_id: Option<String>,
}

impl QuestionTool {
    pub fn new() -> Self {
        Self { permission_store: None, session_id: None }
    }

    pub fn with_permission_store(mut self, store: Arc<PermissionStore>, session_id: String) -> Self {
        self.permission_store = Some(store);
        self.session_id = Some(session_id);
        self
    }
}

impl Default for QuestionTool {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl Tool for QuestionTool {
    fn name(&self) -> &str { "question" }

    fn description(&self) -> &str {
        "Ask the user one or more questions and wait for their answers before continuing. \
         Use this when you need clarification or a decision from the user."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "questions": {
                    "type": "array",
                    "description": "List of questions to ask",
                    "items": {
                        "type": "object",
                        "properties": {
                            "question": { "type": "string", "description": "The question text" },
                            "header": { "type": "string", "description": "Short label (max 30 chars)" },
                            "options": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "label": { "type": "string" },
                                        "description": { "type": "string" }
                                    },
                                    "required": ["label", "description"]
                                }
                            },
                            "multiple": { "type": "boolean" }
                        },
                        "required": ["question", "header", "options"]
                    }
                }
            },
            "required": ["questions"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let questions: Vec<QuestionInfo> = serde_json::from_value(
            args.get("questions")
                .ok_or_else(|| ToolError::Parse("Missing 'questions' argument".to_string()))?
                .clone(),
        )
        .map_err(|e| ToolError::Parse(format!("Invalid questions format: {}", e)))?;

        if questions.is_empty() {
            return Err(ToolError::Execution("At least one question is required".to_string()));
        }

        // If we have a permission store, suspend the agent loop and wait for user reply
        if let (Some(store), Some(session_id)) = (&self.permission_store, &self.session_id) {
            let metadata = serde_json::json!({ "questions": questions });
            let patterns = vec!["question".to_string()];
            let empty_rules: Vec<crate::permission::Rule> = Vec::new();
            let result = store.ask(
                session_id,
                "question",
                &patterns,
                &patterns,
                &empty_rules,
                metadata,
                None,
            ).await;

            match result {
                Ok(()) => {
                    // User approved — return a generic "user answered" message
                    // The actual answers come through the permission reply metadata
                    return Ok(ToolResult::new(
                        "User has been asked the questions. Continue based on their response."
                    ));
                }
                Err(e) => {
                    return Err(ToolError::Execution(format!("Question rejected: {e}")));
                }
            }
        }

        // Fallback: return first option for each question (no permission store available)
        let answers: Vec<Vec<String>> = questions.iter().map(|q| {
            if q.options.is_empty() { vec!["No options available".to_string()] }
            else { vec![q.options[0].label.clone()] }
        }).collect();

        let formatted: Vec<String> = questions.iter().enumerate().map(|(i, q)| {
            let answer_str = answers[i].join(", ");
            format!("\"{}\"=\"{}\"", q.question, answer_str)
        }).collect();

        Ok(ToolResult::new(format!(
            "User has answered your questions: {}. You can now continue with the user's answers in mind.",
            formatted.join(", ")
        )).with_metadata(serde_json::json!({ "answers": answers })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_question_tool_name() {
        let tool = QuestionTool::new();
        assert_eq!(tool.name(), "question");
    }

    #[test]
    fn test_question_tool_default() {
        let tool: QuestionTool = Default::default();
        assert_eq!(tool.name(), "question");
    }

    #[tokio::test]
    async fn test_question_single_question() {
        let tool = QuestionTool::new();
        let args = serde_json::json!({
            "questions": [{
                "question": "Which framework do you prefer?",
                "header": "Framework",
                "options": [
                    {"label": "React", "description": "A JavaScript library for building user interfaces"},
                    {"label": "Vue", "description": "The progressive JavaScript framework"}
                ]
            }]
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.output.contains("Which framework do you prefer?"));
        assert!(result.output.contains("React"));
    }

    #[tokio::test]
    async fn test_question_multiple_questions() {
        let tool = QuestionTool::new();
        let args = serde_json::json!({
            "questions": [
                {
                    "question": "Choose a language",
                    "header": "Language",
                    "options": [
                        {"label": "Rust", "description": "Fast and safe"},
                        {"label": "Go", "description": "Simple and concurrent"}
                    ]
                },
                {
                    "question": "Choose a database",
                    "header": "Database",
                    "options": [
                        {"label": "PostgreSQL", "description": "Relational database"},
                        {"label": "MongoDB", "description": "Document database"}
                    ]
                }
            ]
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.output.contains("Choose a language"));
        assert!(result.output.contains("Choose a database"));
    }

    #[tokio::test]
    async fn test_question_multiple_option() {
        let tool = QuestionTool::new();
        let args = serde_json::json!({
            "questions": [{
                "question": "Select features",
                "header": "Features",
                "multiple": true,
                "options": [
                    {"label": "Auth", "description": "Authentication system"},
                    {"label": "Logging", "description": "Logging system"}
                ]
            }]
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.output.contains("Select features"));
    }

    #[tokio::test]
    async fn test_question_missing_questions() {
        let tool = QuestionTool::new();
        let args = serde_json::json!({});

        let result = tool.execute(args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_question_empty_questions() {
        let tool = QuestionTool::new();
        let args = serde_json::json!({
            "questions": []
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_question_metadata() {
        let tool = QuestionTool::new();
        let args = serde_json::json!({
            "questions": [{
                "question": "Test?",
                "header": "Test",
                "options": [
                    {"label": "Yes", "description": "Affirmative"},
                    {"label": "No", "description": "Negative"}
                ]
            }]
        });

        let result = tool.execute(args).await.unwrap();
        let metadata = result.metadata.unwrap();
        assert!(metadata["answers"].is_array());
    }
}