//! Transcript formatting types for converting chat messages to Markdown.
//!
//! This module provides type definitions for formatting OpenCode session
//! transcripts into Markdown format.

use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};

/// Options for transcript formatting.
///
/// Controls which parts of messages are included in the output.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct TranscriptOptions {
    /// Include thinking/reasoning content in the transcript.
    pub thinking: bool,
    /// Include detailed tool call information (input/output/state).
    pub tool_details: bool,
    /// Include assistant metadata (model, agent, duration).
    pub assistant_metadata: bool,
}

/// Session metadata for transcript header.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranscriptSessionInfo {
    /// Unique session identifier.
    pub id: String,
    /// Human-readable session title.
    pub title: String,
    /// Unix timestamp of session creation (seconds since epoch).
    pub created_at: i64,
    /// Unix timestamp of last update (seconds since epoch).
    pub updated_at: i64,
}

/// Status of a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ToolStatus {
    /// Tool call has been made but not yet started.
    #[default]
    Pending,
    /// Tool is currently executing.
    Running,
    /// Tool completed successfully.
    Completed,
    /// Tool encountered an error.
    Error,
}

/// Individual parts that make up a message.
///
/// Each part represents a different type of content in a message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum PartType {
    /// Text content from the user or assistant.
    Text {
        /// The actual text content.
        text: String,
        /// Whether this text was synthetically generated.
        synthetic: bool,
    },
    /// Reasoning or thinking content from the model.
    Reasoning {
        /// The reasoning content.
        text: String,
    },
    /// A tool call with its execution state.
    Tool {
        /// Name of the tool being called.
        name: String,
        /// Current execution status of the tool.
        status: ToolStatus,
        /// Tool input (optional, depends on status).
        input: Option<String>,
        /// Tool output (optional, depends on status).
        output: Option<String>,
        /// Error message if status is Error.
        error: Option<String>,
    },
}

/// Role of the message sender.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageRole {
    /// Message from the user.
    User,
    /// Message from the assistant.
    Assistant {
        /// Name of the agent that generated this message.
        agent: String,
        /// Model identifier used to generate this message.
        model_id: String,
        /// Time taken to generate the response in milliseconds.
        duration_ms: Option<u64>,
    },
}

/// A single message in the transcript.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranscriptMessage {
    /// The role and metadata of the message sender.
    pub role: MessageRole,
    /// Individual parts that make up this message.
    pub parts: Vec<PartType>,
}

/// A message with its associated metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageWithParts {
    /// The message info including role and content parts.
    pub info: TranscriptMessage,
}

/// Formats a single part into a Markdown string.
///
/// Converts a `PartType` to its Markdown representation based on the provided
/// formatting options. Handles text, reasoning/thinking, and tool call content.
pub fn format_part(part: &PartType, options: &TranscriptOptions) -> String {
    match part {
        PartType::Text { text, synthetic } => {
            if !synthetic {
                format!("{text}\n\n")
            } else {
                String::new()
            }
        }
        PartType::Reasoning { text } => {
            if options.thinking {
                format!("_Thinking:_\n\n{text}\n\n")
            } else {
                String::new()
            }
        }
        PartType::Tool {
            name,
            status,
            input,
            output,
            error,
        } => {
            let mut result = format!("**Tool: {name}**\n");

            if options.tool_details {
                if let Some(input) = input {
                    result.push_str(&format!("\n**Input:**\n```json\n{}\n```\n", input));
                }

                if status == &ToolStatus::Completed
                    && let Some(output) = output
                {
                    result.push_str(&format!("\n**Output:**\n```\n{}\n```\n", output));
                }

                if status == &ToolStatus::Error
                    && let Some(error) = error
                {
                    result.push_str(&format!("\n**Error:**\n```\n{}\n```\n", error));
                }
            }

            result.push('\n');
            result
        }
    }
}

/// Formats the assistant message header with optional metadata.
///
/// If `include_metadata` is false, returns a simple "## Assistant" header.
/// Otherwise, includes the agent name (titlecased), model ID, and optional duration.
///
/// # Arguments
/// * `agent` - The name of the agent that generated the message
/// * `model_id` - The model identifier used
/// * `duration_ms` - Optional duration in milliseconds
/// * `include_metadata` - Whether to include full metadata in the header
///
/// # Returns
/// A formatted Markdown header string for an assistant message.
pub fn format_assistant_header(
    agent: &str,
    model_id: &str,
    duration_ms: Option<u64>,
    include_metadata: bool,
) -> String {
    if !include_metadata {
        return "## Assistant\n\n".to_string();
    }

    // Titlecase: first char uppercase, rest lowercase
    let mut chars = agent.chars();
    let titlecase_agent = match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
    };

    let duration_str = duration_ms.map(|ms| {
        let seconds = ms as f64 / 1000.0;
        format!("{:.1}s", seconds)
    });

    match duration_str {
        Some(d) => format!(
            "## Assistant ({} · {} · {})\n\n",
            titlecase_agent, model_id, d
        ),
        None => format!("## Assistant ({} · {})\n\n", titlecase_agent, model_id),
    }
}

/// Formats a complete message into a Markdown string.
///
/// Converts a `TranscriptMessage` to its Markdown representation, including
/// the appropriate header based on the message role and all parts formatted
/// according to the provided options.
pub fn format_message(message: &TranscriptMessage, options: &TranscriptOptions) -> String {
    let mut result = String::new();

    match &message.role {
        MessageRole::User => {
            result.push_str("## User\n\n");
        }
        MessageRole::Assistant {
            agent,
            model_id,
            duration_ms,
        } => {
            result.push_str(&format_assistant_header(
                agent,
                model_id,
                *duration_ms,
                options.assistant_metadata,
            ));
        }
    }

    // Format all parts
    for part in &message.parts {
        result.push_str(&format_part(part, options));
    }

    result
}

/// Formats a complete session transcript into a Markdown string.
///
/// Takes session metadata and a collection of messages, then formats them
/// into a complete transcript with header information and all messages
/// separated by horizontal rules.
///
/// # Arguments
/// * `session` - Session metadata including ID, title, and timestamps
/// * `messages` - Slice of messages to include in the transcript
/// * `options` - Formatting options controlling what to include
///
/// # Returns
/// A complete Markdown-formatted transcript string.
pub fn format_transcript(
    session: &TranscriptSessionInfo,
    messages: &[MessageWithParts],
    options: &TranscriptOptions,
) -> String {
    let mut transcript = format!("# {}\n\n", session.title);
    transcript.push_str(&format!("**Session ID:** {}\n", session.id));

    // Format timestamps using chrono
    if let Some(created_dt) = Utc.timestamp_opt(session.created_at, 0).single() {
        transcript.push_str(&format!(
            "**Created:** {}\n",
            created_dt.format("%Y-%m-%d %H:%M:%S UTC")
        ));
    }

    if let Some(updated_dt) = Utc.timestamp_opt(session.updated_at, 0).single() {
        transcript.push_str(&format!(
            "**Updated:** {}\n",
            updated_dt.format("%Y-%m-%d %H:%M:%S UTC")
        ));
    }

    transcript.push_str("---\n\n");

    // Format each message with separator
    for msg in messages {
        transcript.push_str(&format_message(&msg.info, options));
        transcript.push_str("---\n\n");
    }

    transcript
}

/// Create a user message from text input.
pub fn create_user_message(text: &str) -> TranscriptMessage {
    TranscriptMessage {
        role: MessageRole::User,
        parts: vec![PartType::Text {
            text: text.to_string(),
            synthetic: false,
        }],
    }
}

/// Create an assistant message from response text.
pub fn create_assistant_message(
    text: &str,
    agent: &str,
    model_id: &str,
    duration_ms: Option<u64>,
) -> TranscriptMessage {
    TranscriptMessage {
        role: MessageRole::Assistant {
            agent: agent.to_string(),
            model_id: model_id.to_string(),
            duration_ms,
        },
        parts: vec![PartType::Text {
            text: text.to_string(),
            synthetic: false,
        }],
    }
}

/// Create a message with tool call information.
pub fn create_tool_message(
    name: &str,
    status: ToolStatus,
    input: Option<String>,
    output: Option<String>,
    error: Option<String>,
) -> TranscriptMessage {
    TranscriptMessage {
        role: MessageRole::Assistant {
            agent: "system".to_string(),
            model_id: "internal".to_string(),
            duration_ms: None,
        },
        parts: vec![PartType::Tool {
            name: name.to_string(),
            status,
            input,
            output,
            error,
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transcript_options_default() {
        let options = TranscriptOptions::default();
        assert!(!options.thinking);
        assert!(!options.tool_details);
        assert!(!options.assistant_metadata);
    }

    #[test]
    fn test_transcript_session_info() {
        let info = TranscriptSessionInfo {
            id: "ses_abc123".to_string(),
            title: "Test Session".to_string(),
            created_at: 1700000000,
            updated_at: 1700000100,
        };
        assert_eq!(info.id, "ses_abc123");
    }

    #[test]
    fn test_tool_status_default() {
        let status = ToolStatus::default();
        assert_eq!(status, ToolStatus::Pending);
    }

    #[test]
    fn test_part_type_text() {
        let part = PartType::Text {
            text: "Hello, world!".to_string(),
            synthetic: false,
        };
        if let PartType::Text { text, synthetic } = part {
            assert_eq!(text, "Hello, world!");
            assert!(!synthetic);
        } else {
            panic!("Expected Text variant");
        }
    }

    #[test]
    fn test_part_type_reasoning() {
        let part = PartType::Reasoning {
            text: "Let me think about this...".to_string(),
        };
        if let PartType::Reasoning { text } = part {
            assert_eq!(text, "Let me think about this...");
        } else {
            panic!("Expected Reasoning variant");
        }
    }

    #[test]
    fn test_part_type_tool() {
        let part = PartType::Tool {
            name: "bash".to_string(),
            status: ToolStatus::Running,
            input: Some("ls -la".to_string()),
            output: None,
            error: None,
        };
        if let PartType::Tool { name, status, .. } = part {
            assert_eq!(name, "bash");
            assert_eq!(status, ToolStatus::Running);
        } else {
            panic!("Expected Tool variant");
        }
    }

    #[test]
    fn test_message_role_user() {
        let role = MessageRole::User;
        assert!(matches!(role, MessageRole::User));
    }

    #[test]
    fn test_message_role_assistant() {
        let role = MessageRole::Assistant {
            agent: "oracle".to_string(),
            model_id: "claude-3-5-sonnet".to_string(),
            duration_ms: Some(1500),
        };
        if let MessageRole::Assistant {
            agent,
            model_id,
            duration_ms,
        } = role
        {
            assert_eq!(agent, "oracle");
            assert_eq!(model_id, "claude-3-5-sonnet");
            assert_eq!(duration_ms, Some(1500));
        } else {
            panic!("Expected Assistant variant");
        }
    }

    #[test]
    fn test_transcript_message() {
        let message = TranscriptMessage {
            role: MessageRole::User,
            parts: vec![PartType::Text {
                text: "Hello".to_string(),
                synthetic: false,
            }],
        };
        assert!(matches!(message.role, MessageRole::User));
        assert_eq!(message.parts.len(), 1);
    }

    #[test]
    fn test_message_with_parts() {
        let msg = MessageWithParts {
            info: TranscriptMessage {
                role: MessageRole::Assistant {
                    agent: "build".to_string(),
                    model_id: "claude-3-5-sonnet".to_string(),
                    duration_ms: None,
                },
                parts: vec![
                    PartType::Text {
                        text: "I'll help you build this.".to_string(),
                        synthetic: false,
                    },
                    PartType::Reasoning {
                        text: "Analyzing requirements...".to_string(),
                    },
                ],
            },
        };
        assert!(matches!(msg.info.role, MessageRole::Assistant { .. }));
        assert_eq!(msg.info.parts.len(), 2);
    }

    #[test]
    fn test_format_part_text_not_synthetic() {
        let part = PartType::Text {
            text: "Hello, world!".to_string(),
            synthetic: false,
        };
        let options = TranscriptOptions::default();
        let result = format_part(&part, &options);
        assert_eq!(result, "Hello, world!\n\n");
    }

    #[test]
    fn test_format_part_text_synthetic() {
        let part = PartType::Text {
            text: "Synthetic text".to_string(),
            synthetic: true,
        };
        let options = TranscriptOptions::default();
        let result = format_part(&part, &options);
        assert_eq!(result, "");
    }

    #[test]
    fn test_format_part_reasoning_with_thinking() {
        let part = PartType::Reasoning {
            text: "Let me think about this...".to_string(),
        };
        let options = TranscriptOptions {
            thinking: true,
            ..Default::default()
        };
        let result = format_part(&part, &options);
        assert_eq!(result, "_Thinking:_\n\nLet me think about this...\n\n");
    }

    #[test]
    fn test_format_part_reasoning_without_thinking() {
        let part = PartType::Reasoning {
            text: "Hidden thinking".to_string(),
        };
        let options = TranscriptOptions::default();
        let result = format_part(&part, &options);
        assert_eq!(result, "");
    }

    #[test]
    fn test_format_part_tool_basic() {
        let part = PartType::Tool {
            name: "bash".to_string(),
            status: ToolStatus::Running,
            input: None,
            output: None,
            error: None,
        };
        let options = TranscriptOptions::default();
        let result = format_part(&part, &options);
        assert_eq!(result, "**Tool: bash**\n\n");
    }

    #[test]
    fn test_format_part_tool_with_input() {
        let part = PartType::Tool {
            name: "bash".to_string(),
            status: ToolStatus::Running,
            input: Some(r#"{"command": "ls -la"}"#.to_string()),
            output: None,
            error: None,
        };
        let options = TranscriptOptions {
            tool_details: true,
            ..Default::default()
        };
        let result = format_part(&part, &options);
        assert!(result.contains("**Tool: bash**"));
        assert!(result.contains("**Input:**"));
        assert!(result.contains("```json"));
        assert!(result.contains(r#"{"command": "ls -la"}"#));
    }

    #[test]
    fn test_format_part_tool_with_output() {
        let part = PartType::Tool {
            name: "read".to_string(),
            status: ToolStatus::Completed,
            input: None,
            output: Some("file content here".to_string()),
            error: None,
        };
        let options = TranscriptOptions {
            tool_details: true,
            ..Default::default()
        };
        let result = format_part(&part, &options);
        assert!(result.contains("**Tool: read**"));
        assert!(result.contains("**Output:**"));
        assert!(result.contains("```\nfile content here\n```"));
    }

    #[test]
    fn test_format_part_tool_with_error() {
        let part = PartType::Tool {
            name: "bash".to_string(),
            status: ToolStatus::Error,
            input: None,
            output: None,
            error: Some("Command not found".to_string()),
        };
        let options = TranscriptOptions {
            tool_details: true,
            ..Default::default()
        };
        let result = format_part(&part, &options);
        assert!(result.contains("**Tool: bash**"));
        assert!(result.contains("**Error:**"));
        assert!(result.contains("```\nCommand not found\n```"));
    }

    #[test]
    fn test_format_part_tool_without_tool_details() {
        let part = PartType::Tool {
            name: "bash".to_string(),
            status: ToolStatus::Completed,
            input: Some("input data".to_string()),
            output: Some("output data".to_string()),
            error: None,
        };
        let options = TranscriptOptions::default();
        let result = format_part(&part, &options);
        assert_eq!(result, "**Tool: bash**\n\n");
    }

    // ============ Serialization Tests ============

    #[test]
    fn test_transcript_options_serialization_roundtrip() {
        // Test default values
        let options = TranscriptOptions::default();
        let json = serde_json::to_string(&options).unwrap();
        let decoded: TranscriptOptions = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.thinking, options.thinking);
        assert_eq!(decoded.tool_details, options.tool_details);
        assert_eq!(decoded.assistant_metadata, options.assistant_metadata);

        // Test with custom values
        let options = TranscriptOptions {
            thinking: true,
            tool_details: true,
            assistant_metadata: true,
        };
        let json = serde_json::to_string(&options).unwrap();
        let decoded: TranscriptOptions = serde_json::from_str(&json).unwrap();
        assert!(decoded.thinking);
        assert!(decoded.tool_details);
        assert!(decoded.assistant_metadata);
    }

    #[test]
    fn test_transcript_session_info_serialization_roundtrip() {
        let info = TranscriptSessionInfo {
            id: "ses_abc123".to_string(),
            title: "Test Session".to_string(),
            created_at: 1700000000,
            updated_at: 1700000100,
        };
        let json = serde_json::to_string(&info).unwrap();
        let decoded: TranscriptSessionInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, "ses_abc123");
        assert_eq!(decoded.title, "Test Session");
        assert_eq!(decoded.created_at, 1700000000);
        assert_eq!(decoded.updated_at, 1700000100);
    }

    #[test]
    fn test_tool_status_serialization_all_variants() {
        let variants = vec![
            ToolStatus::Pending,
            ToolStatus::Running,
            ToolStatus::Completed,
            ToolStatus::Error,
        ];

        for status in variants {
            let json = serde_json::to_string(&status).unwrap();
            let decoded: ToolStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, decoded);
        }
    }

    #[test]
    fn test_part_type_text_serialization() {
        // Without synthetic
        let part = PartType::Text {
            text: "Hello, world!".to_string(),
            synthetic: false,
        };
        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains(r#""type":"Text""#));
        assert!(json.contains(r#""text":"Hello, world!""#));
        assert!(json.contains(r#""synthetic":false"#));
        let decoded: PartType = serde_json::from_str(&json).unwrap();
        if let PartType::Text { text, synthetic } = decoded {
            assert_eq!(text, "Hello, world!");
            assert!(!synthetic);
        } else {
            panic!("Expected Text variant");
        }

        // With synthetic
        let part = PartType::Text {
            text: "Generated response".to_string(),
            synthetic: true,
        };
        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains(r#""synthetic":true"#));
        let decoded: PartType = serde_json::from_str(&json).unwrap();
        if let PartType::Text { text, synthetic } = decoded {
            assert_eq!(text, "Generated response");
            assert!(synthetic);
        } else {
            panic!("Expected Text variant");
        }
    }

    #[test]
    fn test_part_type_reasoning_serialization() {
        let part = PartType::Reasoning {
            text: "Let me think about this...".to_string(),
        };
        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains(r#""type":"Reasoning""#));
        assert!(json.contains(r#""text":"Let me think about this...""#));
        let decoded: PartType = serde_json::from_str(&json).unwrap();
        if let PartType::Reasoning { text } = decoded {
            assert_eq!(text, "Let me think about this...");
        } else {
            panic!("Expected Reasoning variant");
        }
    }

    #[test]
    fn test_part_type_tool_serialization_all_statuses() {
        // Test all status variants
        let statuses = vec![
            ToolStatus::Pending,
            ToolStatus::Running,
            ToolStatus::Completed,
            ToolStatus::Error,
        ];

        for status in statuses {
            let part = PartType::Tool {
                name: "bash".to_string(),
                status: status.clone(),
                input: Some("ls -la".to_string()),
                output: Some("total 4\ndrwxr-xr-x  2 user user 4096 Mar 17 10:00 .".to_string()),
                error: None,
            };
            let json = serde_json::to_string(&part).unwrap();
            assert!(json.contains(r#""type":"Tool""#));
            assert!(json.contains(r#""name":"bash""#));
            let decoded: PartType = serde_json::from_str(&json).unwrap();
            if let PartType::Tool {
                name,
                status: decoded_status,
                ..
            } = decoded
            {
                assert_eq!(name, "bash");
                assert_eq!(decoded_status, status);
            } else {
                panic!("Expected Tool variant");
            }
        }
    }

    #[test]
    fn test_part_type_tool_optional_fields() {
        // All optional fields present
        let part = PartType::Tool {
            name: "api_call".to_string(),
            status: ToolStatus::Completed,
            input: Some(r#"{"key": "value"}"#.to_string()),
            output: Some(r#"{"result": "success"}"#.to_string()),
            error: None,
        };
        let json = serde_json::to_string(&part).unwrap();
        let decoded: PartType = serde_json::from_str(&json).unwrap();
        if let PartType::Tool {
            name,
            status,
            input,
            output,
            error,
        } = decoded
        {
            assert_eq!(name, "api_call");
            assert_eq!(status, ToolStatus::Completed);
            assert!(input.is_some());
            assert!(output.is_some());
            assert!(error.is_none());
        } else {
            panic!("Expected Tool variant");
        }

        // Only required fields
        let part = PartType::Tool {
            name: "read".to_string(),
            status: ToolStatus::Running,
            input: None,
            output: None,
            error: None,
        };
        let json = serde_json::to_string(&part).unwrap();
        let decoded: PartType = serde_json::from_str(&json).unwrap();
        if let PartType::Tool {
            name,
            status,
            input,
            output,
            error,
        } = decoded
        {
            assert_eq!(name, "read");
            assert_eq!(status, ToolStatus::Running);
            assert!(input.is_none());
            assert!(output.is_none());
            assert!(error.is_none());
        } else {
            panic!("Expected Tool variant");
        }

        // With error
        let part = PartType::Tool {
            name: "failed_call".to_string(),
            status: ToolStatus::Error,
            input: None,
            output: None,
            error: Some("Connection refused".to_string()),
        };
        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains(r#""error":"Connection refused""#));
        let decoded: PartType = serde_json::from_str(&json).unwrap();
        if let PartType::Tool {
            name,
            status,
            error,
            ..
        } = decoded
        {
            assert_eq!(name, "failed_call");
            assert_eq!(status, ToolStatus::Error);
            assert!(error.is_some());
            assert_eq!(error.unwrap(), "Connection refused");
        } else {
            panic!("Expected Tool variant");
        }
    }

    #[test]
    fn test_message_role_user_serialization() {
        let role = MessageRole::User;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, r#""User""#);
        let decoded: MessageRole = serde_json::from_str(&json).unwrap();
        assert!(matches!(decoded, MessageRole::User));
    }

    #[test]
    fn test_message_role_assistant_serialization() {
        // With duration_ms
        let role = MessageRole::Assistant {
            agent: "oracle".to_string(),
            model_id: "claude-3-5-sonnet".to_string(),
            duration_ms: Some(1500),
        };
        let json = serde_json::to_string(&role).unwrap();
        assert!(json.contains(r#""agent":"oracle""#));
        assert!(json.contains(r#""model_id":"claude-3-5-sonnet""#));
        assert!(json.contains(r#""duration_ms":1500"#));
        let decoded: MessageRole = serde_json::from_str(&json).unwrap();
        if let MessageRole::Assistant {
            agent,
            model_id,
            duration_ms,
        } = decoded
        {
            assert_eq!(agent, "oracle");
            assert_eq!(model_id, "claude-3-5-sonnet");
            assert_eq!(duration_ms, Some(1500));
        } else {
            panic!("Expected Assistant variant");
        }

        // Without duration_ms - serde serializes None as null
        let role = MessageRole::Assistant {
            agent: "build".to_string(),
            model_id: "claude-3-5-sonnet".to_string(),
            duration_ms: None,
        };
        let json = serde_json::to_string(&role).unwrap();
        assert!(json.contains(r#""agent":"build""#));
        // None is serialized as null, so it IS contained but as null
        let decoded: MessageRole = serde_json::from_str(&json).unwrap();
        if let MessageRole::Assistant {
            agent,
            model_id,
            duration_ms,
        } = decoded
        {
            assert_eq!(agent, "build");
            assert_eq!(model_id, "claude-3-5-sonnet");
            assert!(duration_ms.is_none());
        } else {
            panic!("Expected Assistant variant");
        }
    }

    #[test]
    fn test_transcript_message_serialization_roundtrip() {
        let message = TranscriptMessage {
            role: MessageRole::User,
            parts: vec![PartType::Text {
                text: "Hello".to_string(),
                synthetic: false,
            }],
        };
        let json = serde_json::to_string(&message).unwrap();
        let decoded: TranscriptMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(decoded.role, MessageRole::User));
        assert_eq!(decoded.parts.len(), 1);
        if let PartType::Text { text, synthetic } = &decoded.parts[0] {
            assert_eq!(text, "Hello");
            assert!(!synthetic);
        } else {
            panic!("Expected Text part");
        }
    }

    #[test]
    fn test_message_with_parts_serialization_roundtrip() {
        let msg = MessageWithParts {
            info: TranscriptMessage {
                role: MessageRole::Assistant {
                    agent: "build".to_string(),
                    model_id: "claude-3-5-sonnet".to_string(),
                    duration_ms: Some(2000),
                },
                parts: vec![
                    PartType::Text {
                        text: "I'll help you build this.".to_string(),
                        synthetic: false,
                    },
                    PartType::Reasoning {
                        text: "Analyzing requirements...".to_string(),
                    },
                    PartType::Tool {
                        name: "read".to_string(),
                        status: ToolStatus::Completed,
                        input: Some("src/main.rs".to_string()),
                        output: Some("file content...".to_string()),
                        error: None,
                    },
                ],
            },
        };
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: MessageWithParts = serde_json::from_str(&json).unwrap();
        if let MessageRole::Assistant {
            agent,
            model_id,
            duration_ms,
        } = decoded.info.role
        {
            assert_eq!(agent, "build");
            assert_eq!(model_id, "claude-3-5-sonnet");
            assert_eq!(duration_ms, Some(2000));
        } else {
            panic!("Expected Assistant role");
        }
        assert_eq!(decoded.info.parts.len(), 3);
    }

    #[test]
    fn test_json_structure_with_serde_tag() {
        // Verify that PartType uses the correct JSON structure with #[serde(tag = "type")]
        let part = PartType::Text {
            text: "test".to_string(),
            synthetic: false,
        };
        let json = serde_json::to_string(&part).unwrap();
        // Should have "type" as the first field for the tag
        assert!(json.starts_with(r#"{"type":"Text""#));

        let part = PartType::Reasoning {
            text: "test".to_string(),
        };
        let json = serde_json::to_string(&part).unwrap();
        assert!(json.starts_with(r#"{"type":"Reasoning""#));

        let part = PartType::Tool {
            name: "test".to_string(),
            status: ToolStatus::Pending,
            input: None,
            output: None,
            error: None,
        };
        let json = serde_json::to_string(&part).unwrap();
        assert!(json.starts_with(r#"{"type":"Tool""#));
    }

    // ============ format_assistant_header Tests ============

    #[test]
    fn test_format_assistant_header_without_metadata() {
        let result = format_assistant_header("oracle", "claude-3-5-sonnet", Some(1500), false);
        assert_eq!(result, "## Assistant\n\n");
    }

    #[test]
    fn test_format_assistant_header_with_metadata_no_duration() {
        let result = format_assistant_header("build", "claude-3-5-sonnet", None, true);
        assert!(result.starts_with("## Assistant (Build ·"));
        assert!(result.contains("claude-3-5-sonnet"));
    }

    #[test]
    fn test_format_assistant_header_with_metadata_and_duration() {
        let result = format_assistant_header("oracle", "claude-3-5-sonnet", Some(2500), true);
        assert!(result.starts_with("## Assistant (Oracle ·"));
        assert!(result.contains("claude-3-5-sonnet"));
        assert!(result.contains("2.5s"));
    }

    #[test]
    fn test_format_assistant_header_titlecase_agent_name() {
        let agents = vec![
            ("oracle", "Oracle"),
            ("build", "Build"),
            ("explore", "Explore"),
            ("librarian", "Librarian"),
        ];

        for (input, expected) in agents {
            let result = format_assistant_header(input, "model", None, true);
            assert!(
                result.contains(&format!("({} ·", expected)),
                "Expected '{}' in result but got: {}",
                expected,
                result
            );
        }
    }

    #[test]
    fn test_format_assistant_header_user_role_returns_empty() {
        // User role is handled differently - returns empty and is handled in format_message
        let message = TranscriptMessage {
            role: MessageRole::User,
            parts: vec![],
        };
        let options = TranscriptOptions::default();
        let result = format_message(&message, &options);
        assert!(result.starts_with("## User\n\n"));
    }

    // ============ format_message Tests ============

    #[test]
    fn test_format_message_user_with_text() {
        let message = TranscriptMessage {
            role: MessageRole::User,
            parts: vec![PartType::Text {
                text: "Hello, help me please.".to_string(),
                synthetic: false,
            }],
        };
        let options = TranscriptOptions::default();
        let result = format_message(&message, &options);
        assert!(result.starts_with("## User\n\n"));
        assert!(result.contains("Hello, help me please."));
    }

    #[test]
    fn test_format_message_assistant_without_metadata() {
        let message = TranscriptMessage {
            role: MessageRole::Assistant {
                agent: "oracle".to_string(),
                model_id: "claude-3-5-sonnet".to_string(),
                duration_ms: Some(1500),
            },
            parts: vec![PartType::Text {
                text: "I'll help you with that.".to_string(),
                synthetic: false,
            }],
        };
        let options = TranscriptOptions::default();
        let result = format_message(&message, &options);
        assert!(result.starts_with("## Assistant\n\n"));
        assert!(result.contains("I'll help you with that."));
    }

    #[test]
    fn test_format_message_assistant_with_metadata() {
        let message = TranscriptMessage {
            role: MessageRole::Assistant {
                agent: "build".to_string(),
                model_id: "claude-3-5-sonnet".to_string(),
                duration_ms: Some(2000),
            },
            parts: vec![PartType::Text {
                text: "Building your project...".to_string(),
                synthetic: false,
            }],
        };
        let options = TranscriptOptions {
            assistant_metadata: true,
            ..Default::default()
        };
        let result = format_message(&message, &options);
        assert!(result.starts_with("## Assistant (Build ·"));
        assert!(result.contains("claude-3-5-sonnet"));
        assert!(result.contains("2.0s"));
    }

    #[test]
    fn test_format_message_multiple_parts() {
        let message = TranscriptMessage {
            role: MessageRole::Assistant {
                agent: "oracle".to_string(),
                model_id: "claude-3-5-sonnet".to_string(),
                duration_ms: None,
            },
            parts: vec![
                PartType::Text {
                    text: "Let me analyze this.".to_string(),
                    synthetic: false,
                },
                PartType::Reasoning {
                    text: "Looking at the requirements...".to_string(),
                },
                PartType::Tool {
                    name: "read".to_string(),
                    status: ToolStatus::Completed,
                    input: Some("file.rs".to_string()),
                    output: Some("content".to_string()),
                    error: None,
                },
            ],
        };
        let options = TranscriptOptions::default();
        let result = format_message(&message, &options);
        assert!(result.contains("Let me analyze this."));
        // Reasoning is not included without thinking option
        assert!(!result.contains("Thinking:"));
        assert!(result.contains("**Tool: read**"));
    }

    #[test]
    fn test_format_message_all_options_disabled() {
        let message = TranscriptMessage {
            role: MessageRole::Assistant {
                agent: "oracle".to_string(),
                model_id: "claude-3-5-sonnet".to_string(),
                duration_ms: Some(1000),
            },
            parts: vec![
                PartType::Text {
                    text: "Hello".to_string(),
                    synthetic: false,
                },
                PartType::Reasoning {
                    text: "thinking...".to_string(),
                },
                PartType::Tool {
                    name: "bash".to_string(),
                    status: ToolStatus::Completed,
                    input: Some("ls".to_string()),
                    output: Some("files".to_string()),
                    error: None,
                },
            ],
        };
        let options = TranscriptOptions::default();
        let result = format_message(&message, &options);

        // Only header and text should be present; tool name shows but details are hidden
        assert!(result.starts_with("## Assistant\n\n"));
        assert!(result.contains("Hello"));
        assert!(!result.contains("thinking"));
        assert!(result.contains("**Tool: bash**"));
        // Tool details are hidden
        assert!(!result.contains("**Input:**"));
        assert!(!result.contains("**Output:**"));
    }

    #[test]
    fn test_format_message_all_options_enabled() {
        let message = TranscriptMessage {
            role: MessageRole::Assistant {
                agent: "build".to_string(),
                model_id: "claude-3-5-sonnet".to_string(),
                duration_ms: Some(3500),
            },
            parts: vec![
                PartType::Text {
                    text: "Processing...".to_string(),
                    synthetic: false,
                },
                PartType::Reasoning {
                    text: "Analyzing the code...".to_string(),
                },
                PartType::Tool {
                    name: "read".to_string(),
                    status: ToolStatus::Completed,
                    input: Some(r#"{"path": "src/main.rs"}"#.to_string()),
                    output: Some("fn main() {}".to_string()),
                    error: None,
                },
            ],
        };
        let options = TranscriptOptions {
            thinking: true,
            tool_details: true,
            assistant_metadata: true,
        };
        let result = format_message(&message, &options);

        // All options should be present
        assert!(result.contains("## Assistant (Build ·"));
        assert!(result.contains("claude-3-5-sonnet"));
        assert!(result.contains("3.5s"));
        assert!(result.contains("_Thinking:_"));
        assert!(result.contains("Analyzing the code..."));
        assert!(result.contains("**Tool: read**"));
        assert!(result.contains("**Input:**"));
        assert!(result.contains("**Output:**"));
    }

    #[test]
    fn test_format_message_with_synthetic_text() {
        let message = TranscriptMessage {
            role: MessageRole::Assistant {
                agent: "oracle".to_string(),
                model_id: "test".to_string(),
                duration_ms: None,
            },
            parts: vec![
                PartType::Text {
                    text: "Real response".to_string(),
                    synthetic: false,
                },
                PartType::Text {
                    text: "Generated补充".to_string(),
                    synthetic: true,
                },
            ],
        };
        let options = TranscriptOptions::default();
        let result = format_message(&message, &options);

        assert!(result.contains("Real response"));
        assert!(!result.contains("Generated"));
    }

    #[test]
    fn test_format_message_with_special_characters() {
        let message = TranscriptMessage {
            role: MessageRole::User,
            parts: vec![PartType::Text {
                text: "Test with `code` and **bold** and _italic_".to_string(),
                synthetic: false,
            }],
        };
        let options = TranscriptOptions::default();
        let result = format_message(&message, &options);

        assert!(result.contains("`code`"));
        assert!(result.contains("**bold**"));
        assert!(result.contains("_italic_"));
    }

    #[test]
    fn test_format_message_empty_parts() {
        let message = TranscriptMessage {
            role: MessageRole::Assistant {
                agent: "oracle".to_string(),
                model_id: "test".to_string(),
                duration_ms: None,
            },
            parts: vec![],
        };
        let options = TranscriptOptions::default();
        let result = format_message(&message, &options);

        assert!(result.starts_with("## Assistant\n\n"));
        assert!(result.len() > 10); // Just header
    }

    #[test]
    fn test_format_message_tool_error_status() {
        let message = TranscriptMessage {
            role: MessageRole::Assistant {
                agent: "build".to_string(),
                model_id: "test".to_string(),
                duration_ms: None,
            },
            parts: vec![PartType::Tool {
                name: "bash".to_string(),
                status: ToolStatus::Error,
                input: Some("rm -rf /".to_string()),
                output: None,
                error: Some("Permission denied".to_string()),
            }],
        };
        let options = TranscriptOptions {
            tool_details: true,
            ..Default::default()
        };
        let result = format_message(&message, &options);

        assert!(result.contains("**Tool: bash**"));
        assert!(result.contains("**Error:**"));
        assert!(result.contains("Permission denied"));
    }

    // ============ format_transcript Tests ============

    #[test]
    fn test_format_transcript_single_user_message() {
        let session = TranscriptSessionInfo {
            id: "ses_test123".to_string(),
            title: "Test Session".to_string(),
            created_at: 1700000000,
            updated_at: 1700000100,
        };

        let messages = vec![MessageWithParts {
            info: TranscriptMessage {
                role: MessageRole::User,
                parts: vec![PartType::Text {
                    text: "Hello, help me please.".to_string(),
                    synthetic: false,
                }],
            },
        }];

        let options = TranscriptOptions::default();
        let result = format_transcript(&session, &messages, &options);

        assert!(result.starts_with("# Test Session\n\n"));
        assert!(result.contains("**Session ID:** ses_test123\n"));
        assert!(result.contains("**Created:**"));
        assert!(result.contains("**Updated:**"));
        assert!(result.contains("## User\n\n"));
        assert!(result.contains("Hello, help me please."));
        assert!(result.contains("---\n\n"));
    }

    #[test]
    fn test_format_transcript_multiple_messages() {
        let session = TranscriptSessionInfo {
            id: "ses_multi456".to_string(),
            title: "Multi-Message Session".to_string(),
            created_at: 1700000000,
            updated_at: 1700000200,
        };

        let messages = vec![
            MessageWithParts {
                info: TranscriptMessage {
                    role: MessageRole::User,
                    parts: vec![PartType::Text {
                        text: "Can you help me?".to_string(),
                        synthetic: false,
                    }],
                },
            },
            MessageWithParts {
                info: TranscriptMessage {
                    role: MessageRole::Assistant {
                        agent: "oracle".to_string(),
                        model_id: "claude-3-5-sonnet".to_string(),
                        duration_ms: Some(1500),
                    },
                    parts: vec![PartType::Text {
                        text: "Of course! I'll help you.".to_string(),
                        synthetic: false,
                    }],
                },
            },
        ];

        let options = TranscriptOptions::default();
        let result = format_transcript(&session, &messages, &options);

        assert!(result.contains("# Multi-Message Session"));
        assert!(result.contains("## User\n\n"));
        assert!(result.contains("Can you help me?"));
        assert!(result.contains("## Assistant\n\n"));
        assert!(result.contains("Of course! I'll help you."));
        let separator_count = result.matches("---").count();
        assert_eq!(separator_count, 3);
    }

    #[test]
    fn test_format_transcript_all_message_types() {
        let session = TranscriptSessionInfo {
            id: "ses_alltypes".to_string(),
            title: "All Types Session".to_string(),
            created_at: 1700000000,
            updated_at: 1700000300,
        };

        let messages = vec![MessageWithParts {
            info: TranscriptMessage {
                role: MessageRole::Assistant {
                    agent: "build".to_string(),
                    model_id: "claude-3-5-sonnet".to_string(),
                    duration_ms: Some(2000),
                },
                parts: vec![
                    PartType::Text {
                        text: "Processing your request.".to_string(),
                        synthetic: false,
                    },
                    PartType::Reasoning {
                        text: "Analyzing the requirements...".to_string(),
                    },
                    PartType::Tool {
                        name: "read".to_string(),
                        status: ToolStatus::Completed,
                        input: Some("src/main.rs".to_string()),
                        output: Some("fn main() {}".to_string()),
                        error: None,
                    },
                ],
            },
        }];

        let options = TranscriptOptions::default();
        let result = format_transcript(&session, &messages, &options);

        assert!(result.contains("Processing your request."));
        assert!(!result.contains("Thinking:"));
        assert!(result.contains("**Tool: read**"));
        assert!(!result.contains("**Input:**"));
        assert!(!result.contains("**Output:**"));
    }

    #[test]
    fn test_format_transcript_options_all_enabled() {
        let session = TranscriptSessionInfo {
            id: "ses_options".to_string(),
            title: "Options Test".to_string(),
            created_at: 1700000000,
            updated_at: 1700000400,
        };

        let messages = vec![MessageWithParts {
            info: TranscriptMessage {
                role: MessageRole::Assistant {
                    agent: "oracle".to_string(),
                    model_id: "claude-3-5-sonnet".to_string(),
                    duration_ms: Some(2500),
                },
                parts: vec![
                    PartType::Text {
                        text: "Here's my response.".to_string(),
                        synthetic: false,
                    },
                    PartType::Reasoning {
                        text: "Let me think about this...".to_string(),
                    },
                    PartType::Tool {
                        name: "bash".to_string(),
                        status: ToolStatus::Completed,
                        input: Some(r#"{"cmd": "ls"}"#.to_string()),
                        output: Some("file1.rs\nfile2.rs".to_string()),
                        error: None,
                    },
                ],
            },
        }];

        let options = TranscriptOptions {
            thinking: true,
            tool_details: true,
            assistant_metadata: true,
        };
        let result = format_transcript(&session, &messages, &options);

        assert!(result.contains("## Assistant (Oracle ·"));
        assert!(result.contains("2.5s"));
        assert!(result.contains("_Thinking:_"));
        assert!(result.contains("Let me think about this..."));
        assert!(result.contains("**Input:**"));
        assert!(result.contains("**Output:**"));
    }

    #[test]
    fn test_format_transcript_options_all_disabled() {
        let session = TranscriptSessionInfo {
            id: "ses_disabled".to_string(),
            title: "Disabled Options".to_string(),
            created_at: 1700000000,
            updated_at: 1700000500,
        };

        let messages = vec![MessageWithParts {
            info: TranscriptMessage {
                role: MessageRole::Assistant {
                    agent: "build".to_string(),
                    model_id: "claude-3-5-sonnet".to_string(),
                    duration_ms: Some(3000),
                },
                parts: vec![
                    PartType::Text {
                        text: "Response text.".to_string(),
                        synthetic: false,
                    },
                    PartType::Reasoning {
                        text: "Hidden thinking".to_string(),
                    },
                    PartType::Tool {
                        name: "read".to_string(),
                        status: ToolStatus::Completed,
                        input: Some("data".to_string()),
                        output: Some("result".to_string()),
                        error: None,
                    },
                ],
            },
        }];

        let options = TranscriptOptions::default();
        let result = format_transcript(&session, &messages, &options);

        assert!(result.contains("## Assistant\n\n"));
        assert!(result.contains("Response text."));
        assert!(!result.contains("Thinking:"));
        assert!(result.contains("**Tool: read**"));
        assert!(!result.contains("**Input:**"));
        assert!(!result.contains("**Output:**"));
    }

    #[test]
    fn test_format_transcript_empty_messages() {
        let session = TranscriptSessionInfo {
            id: "ses_empty".to_string(),
            title: "Empty Session".to_string(),
            created_at: 1700000000,
            updated_at: 1700000000,
        };

        let messages: Vec<MessageWithParts> = vec![];
        let options = TranscriptOptions::default();
        let result = format_transcript(&session, &messages, &options);

        assert!(result.starts_with("# Empty Session\n\n"));
        assert!(result.contains("**Session ID:** ses_empty"));
        assert!(result.contains("**Created:**"));
        assert!(result.contains("**Updated:**"));
        assert!(result.contains("---\n\n"));
        assert!(result.ends_with("---\n\n"));
    }

    #[test]
    fn test_format_transcript_session_metadata() {
        let session = TranscriptSessionInfo {
            id: "ses_metadata789".to_string(),
            title: "My Custom Session Title".to_string(),
            created_at: 1700000000,
            updated_at: 1700001000,
        };

        let messages = vec![MessageWithParts {
            info: TranscriptMessage {
                role: MessageRole::User,
                parts: vec![PartType::Text {
                    text: "Test".to_string(),
                    synthetic: false,
                }],
            },
        }];

        let options = TranscriptOptions::default();
        let result = format_transcript(&session, &messages, &options);

        assert!(result.starts_with("# My Custom Session Title\n\n"));
        assert!(result.contains("**Session ID:** ses_metadata789"));
        assert!(result.contains("**Created:**"));
        assert!(result.contains("**Updated:**"));
        assert!(result.contains("UTC"));
    }

    #[test]
    fn test_format_transcript_message_separators() {
        let session = TranscriptSessionInfo {
            id: "ses_sep".to_string(),
            title: "Separator Test".to_string(),
            created_at: 1700000000,
            updated_at: 1700000600,
        };

        let messages = vec![
            MessageWithParts {
                info: TranscriptMessage {
                    role: MessageRole::User,
                    parts: vec![PartType::Text {
                        text: "Message 1".to_string(),
                        synthetic: false,
                    }],
                },
            },
            MessageWithParts {
                info: TranscriptMessage {
                    role: MessageRole::Assistant {
                        agent: "oracle".to_string(),
                        model_id: "test".to_string(),
                        duration_ms: None,
                    },
                    parts: vec![PartType::Text {
                        text: "Message 2".to_string(),
                        synthetic: false,
                    }],
                },
            },
            MessageWithParts {
                info: TranscriptMessage {
                    role: MessageRole::User,
                    parts: vec![PartType::Text {
                        text: "Message 3".to_string(),
                        synthetic: false,
                    }],
                },
            },
        ];

        let options = TranscriptOptions::default();
        let result = format_transcript(&session, &messages, &options);

        assert!(result.contains("Message 1"));
        assert!(result.contains("Message 2"));
        assert!(result.contains("Message 3"));
        let separator_count = result.matches("---").count();
        assert_eq!(separator_count, 4);
    }

    #[test]
    fn test_format_transcript_with_special_characters_in_title() {
        let session = TranscriptSessionInfo {
            id: "ses_special".to_string(),
            title: "Test [Code] & 'Quotes' <tag>".to_string(),
            created_at: 1700000000,
            updated_at: 1700000000,
        };

        let messages = vec![];
        let options = TranscriptOptions::default();
        let result = format_transcript(&session, &messages, &options);

        assert!(result.starts_with("# Test [Code] & 'Quotes' <tag>\n\n"));
    }

    #[test]
    fn test_create_user_message() {
        let msg = create_user_message("Hello, world!");
        assert!(matches!(msg.role, MessageRole::User));
        assert_eq!(msg.parts.len(), 1);
        if let PartType::Text { text, synthetic } = &msg.parts[0] {
            assert_eq!(text, "Hello, world!");
            assert!(!synthetic);
        } else {
            panic!("Expected Text part");
        }
    }

    #[test]
    fn test_create_assistant_message() {
        let msg = create_assistant_message("Response", "oracle", "claude-3", Some(1500));
        if let MessageRole::Assistant {
            agent,
            model_id,
            duration_ms,
        } = msg.role
        {
            assert_eq!(agent, "oracle");
            assert_eq!(model_id, "claude-3");
            assert_eq!(duration_ms, Some(1500));
        } else {
            panic!("Expected Assistant role");
        }
    }

    #[test]
    fn test_create_tool_message() {
        let msg = create_tool_message(
            "bash",
            ToolStatus::Completed,
            Some("ls".into()),
            Some("files".into()),
            None,
        );
        assert_eq!(msg.parts.len(), 1);
        if let PartType::Tool { name, status, .. } = &msg.parts[0] {
            assert_eq!(name, "bash");
            assert_eq!(*status, ToolStatus::Completed);
        } else {
            panic!("Expected Tool part");
        }
    }
}
