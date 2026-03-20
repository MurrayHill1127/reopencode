//! Question tool - ask user questions during execution

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::tool::error::{Result, ToolError};
use crate::tool::traits::{Tool, ToolResult};

/// A single option in a question
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionOption {
    /// Display text (1-5 words, concise)
    pub label: String,
    /// Explanation of choice
    pub description: String,
}

/// A question to ask the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionInfo {
    /// Complete question
    pub question: String,
    /// Very short label (max 30 chars)
    pub header: String,
    /// Available choices
    pub options: Vec<QuestionOption>,
    /// Allow selecting multiple choices (optional, default false)
    #[serde(default)]
    pub multiple: bool,
}

/// Question tool - ask user questions during execution
pub struct QuestionTool;

impl QuestionTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for QuestionTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for QuestionTool {
    fn name(&self) -> &str {
        "question"
    }

    fn description(&self) -> &str {
        r#"Use this tool when you need to ask the user questions during execution. This allows you to:
1. Gather user preferences or requirements
2. Clarify ambiguous instructions
3. Get decisions on implementation choices as you work
4. Offer choices to the user about what direction to take.

Usage notes:
- When `custom` is enabled (default), a "Type your own answer" option is added automatically; don't include "Other" or catch-all options
- Answers are returned as arrays of labels; set `multiple: true` to allow selecting more than one
- If you recommend a specific option, make that the first option in the list and add "(Recommended)" at the end of the label"#
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "questions": {
                    "type": "array",
                    "description": "Questions to ask",
                    "items": {
                        "type": "object",
                        "properties": {
                            "question": {
                                "type": "string",
                                "description": "Complete question"
                            },
                            "header": {
                                "type": "string",
                                "description": "Very short label (max 30 chars)"
                            },
                            "options": {
                                "type": "array",
                                "description": "Available choices",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "label": {
                                            "type": "string",
                                            "description": "Display text (1-5 words, concise)"
                                        },
                                        "description": {
                                            "type": "string",
                                            "description": "Explanation of choice"
                                        }
                                    },
                                    "required": ["label", "description"]
                                }
                            },
                            "multiple": {
                                "type": "boolean",
                                "description": "Allow selecting multiple choices"
                            }
                        },
                        "required": ["question", "header", "options"]
                    }
                }
            },
            "required": ["questions"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        // Parse questions array
        let questions: Vec<QuestionInfo> = serde_json::from_value(
            args.get("questions")
                .ok_or_else(|| ToolError::Parse("Missing 'questions' argument".to_string()))?
                .clone(),
        )
        .map_err(|e| ToolError::Parse(format!("Invalid questions format: {}", e)))?;

        if questions.is_empty() {
            return Err(ToolError::Execution("At least one question is required".to_string()));
        }

        // TODO: Integrate with session/permission system for actual question handling
        // For now, return mock answers (first option for each question)
        let answers: Vec<Vec<String>> = questions
            .iter()
            .map(|q| {
                if q.options.is_empty() {
                    vec!["No options available".to_string()]
                } else {
                    vec![q.options[0].label.clone()]
                }
            })
            .collect();

        // Format output
        let formatted: Vec<String> = questions
            .iter()
            .enumerate()
            .map(|(i, q)| {
                let answer = &answers[i];
                let answer_str = if answer.is_empty() {
                    "Unanswered".to_string()
                } else {
                    answer.join(", ")
                };
                format!("\"{}\"=\"{}\"", q.question, answer_str)
            })
            .collect();

        let output = format!(
            "User has answered your questions: {}. You can now continue with the user's answers in mind.",
            formatted.join(", ")
        );

        Ok(ToolResult::new(output).with_metadata(serde_json::json!({
            "answers": answers
        })))
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