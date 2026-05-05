//! Message types and conversion for the MessageV2 system
//!
//! Provides message structures and conversion to model-compatible format.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::session::parts::{
    is_media, FilePart, MessageId, ModelId, Part, ProviderId, SessionId, TokenInfo, ToolState,
};

// ============================================================================
// Error types
// ============================================================================

/// Named error types for messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "name", rename_all = "camelCase")]
pub enum MessageError {
    OutputLengthError {},
    AbortedError {
        message: String,
    },
    StructuredOutputError {
        message: String,
        retries: i64,
    },
    ProviderAuthError {
        provider_id: String,
        message: String,
    },
    APIError {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        status_code: Option<i64>,
        is_retryable: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        response_headers: Option<HashMap<String, String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        response_body: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, String>>,
    },
    ContextOverflowError {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        response_body: Option<String>,
    },
    UnknownError {
        message: String,
    },
}

// ============================================================================
// Output format types
// ============================================================================

/// Output format for text responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OutputFormat {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "json_schema")]
    JsonSchema {
        schema: serde_json::Value,
        #[serde(default = "default_retry_count")]
        retry_count: i64,
    },
}

fn default_retry_count() -> i64 {
    2
}

// ============================================================================
// Time tracking
// ============================================================================

/// Time info for user messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserTime {
    pub created: i64,
}

/// Time info for assistant messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantTime {
    pub created: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<i64>,
}

// ============================================================================
// Path info
// ============================================================================

/// Path information for assistant messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathInfo {
    pub cwd: String,
    pub root: String,
}

// ============================================================================
// Model reference
// ============================================================================

/// Model reference for messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRef {
    pub provider_id: ProviderId,
    pub model_id: ModelId,
}

// ============================================================================
// Summary types
// ============================================================================

/// File diff for summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub additions: u32,
    pub deletions: u32,
}

/// User message summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    pub diffs: Vec<FileDiff>,
}

// ============================================================================
// Message types
// ============================================================================

/// User message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub id: MessageId,
    #[serde(rename = "sessionID")]
    pub session_id: SessionId,
    pub time: UserTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<OutputFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<UserSummary>,
    pub agent: String,
    pub model: ModelRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<HashMap<String, bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
}

impl Default for UserMessage {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: String::new(),
            time: UserTime {
                created: chrono::Utc::now().timestamp_millis(),
            },
            format: None,
            summary: None,
            agent: String::new(),
            model: ModelRef {
                provider_id: String::new(),
                model_id: String::new(),
            },
            system: None,
            tools: None,
            variant: None,
        }
    }
}

/// Assistant message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub id: MessageId,
    #[serde(rename = "sessionID")]
    pub session_id: SessionId,
    pub time: AssistantTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<MessageError>,
    #[serde(rename = "parentID")]
    pub parent_id: MessageId,
    #[serde(rename = "modelID")]
    pub model_id: ModelId,
    #[serde(rename = "providerID")]
    pub provider_id: ProviderId,
    #[deprecated]
    pub mode: String,
    pub agent: String,
    pub path: PathInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<bool>,
    pub cost: f64,
    pub tokens: TokenInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish: Option<String>,
}

/// Message info (user or assistant)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum MessageInfo {
    User(UserMessage),
    Assistant(AssistantMessage),
}

impl MessageInfo {
    pub fn id(&self) -> &str {
        match self {
            MessageInfo::User(m) => &m.id,
            MessageInfo::Assistant(m) => &m.id,
        }
    }

    pub fn session_id(&self) -> &str {
        match self {
            MessageInfo::User(m) => &m.session_id,
            MessageInfo::Assistant(m) => &m.session_id,
        }
    }
}

// ============================================================================
// Message with parts
// ============================================================================

/// Message with its parts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithParts {
    pub info: MessageInfo,
    pub parts: Vec<Part>,
}

// ============================================================================
// Model message conversion
// ============================================================================

/// UI message part for conversion
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum UIPart {
    Text {
        text: String,
    },
    File {
        url: String,
        media_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        filename: Option<String>,
    },
    Reasoning {
        text: String,
    },
    StepStart,
    #[serde(rename = "tool-$name")]
    ToolResult {
        #[serde(rename = "tool-$name")]
        tool_name: String,
        state: ToolResultState,
        tool_call_id: String,
        input: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        output: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error_text: Option<String>,
    },
}

/// Tool result state
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolResultState {
    OutputAvailable,
    OutputError,
}

/// UI message for conversion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIMessage {
    pub id: String,
    pub role: String,
    pub parts: Vec<UIPart>,
}

/// Model message for AI SDK compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum ModelMessage {
    User {
        id: String,
        content: Vec<ModelContent>,
    },
    Assistant {
        id: String,
        content: Vec<AssistantContent>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_calls: Option<Vec<ToolCall>>,
    },
    Tool {
        tool_call_id: String,
        content: String,
    },
}

/// Content for user messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ModelContent {
    Text { text: String },
    Image { image: ImageContent },
    File { data: String, media_type: String },
}

/// Image content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageContent {
    pub url: String,
}

/// Content for assistant messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum AssistantContent {
    Text {
        text: String,
    },
    Reasoning {
        text: String,
    },
    ToolCall {
        tool_call_id: String,
        tool_name: String,
        args: serde_json::Value,
    },
}

/// Tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool_call_id: String,
    pub tool_name: String,
    pub args: serde_json::Value,
}

/// Provider model info for conversion
#[derive(Debug, Clone)]
pub struct ProviderModel {
    pub provider_id: String,
    pub model_id: String,
    pub api_npm: String,
    pub api_id: String,
}

/// Options for to_model_messages
#[derive(Debug, Clone, Default)]
pub struct ConversionOptions {
    pub strip_media: bool,
}

/// Convert messages with parts to model messages
pub fn to_model_messages(
    input: &[WithParts],
    model: &ProviderModel,
    options: Option<ConversionOptions>,
) -> Vec<ModelMessage> {
    let options = options.unwrap_or_default();
    let mut result: Vec<ModelMessage> = Vec::new();
    let mut tool_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    let supports_media_in_tool_results = check_media_support(model);

    for msg in input {
        if msg.parts.is_empty() {
            continue;
        }

        match &msg.info {
            MessageInfo::User(user_msg) => {
                let mut user_parts: Vec<ModelContent> = Vec::new();

                for part in &msg.parts {
                    match part {
                        Part::Text(p) if !p.ignored.unwrap_or(false) => {
                            user_parts.push(ModelContent::Text {
                                text: p.text.clone(),
                            });
                        }
                        Part::File(p)
                            if p.mime != "text/plain" && p.mime != "application/x-directory" =>
                        {
                            if options.strip_media && is_media(&p.mime) {
                                user_parts.push(ModelContent::Text {
                                    text: format!(
                                        "[Attached {}: {}]",
                                        p.mime,
                                        p.filename.as_deref().unwrap_or("file")
                                    ),
                                });
                            } else {
                                if is_media(&p.mime) {
                                    user_parts.push(ModelContent::Image {
                                        image: ImageContent { url: p.url.clone() },
                                    });
                                } else {
                                    user_parts.push(ModelContent::File {
                                        data: p.url.clone(),
                                        media_type: p.mime.clone(),
                                    });
                                }
                            }
                        }
                        Part::Compaction(_) => {
                            user_parts.push(ModelContent::Text {
                                text: "What did we do so far?".to_string(),
                            });
                        }
                        Part::Subtask(_) => {
                            user_parts.push(ModelContent::Text {
                                text: "The following tool was executed by the user".to_string(),
                            });
                        }
                        _ => {}
                    }
                }

                if !user_parts.is_empty() {
                    result.push(ModelMessage::User {
                        id: user_msg.id.clone(),
                        content: user_parts,
                    });
                }
            }
            MessageInfo::Assistant(assistant_msg) => {
                let _different_model = format!("{}/{}", model.provider_id, model.model_id)
                    != format!("{}/{}", assistant_msg.provider_id, assistant_msg.model_id);
                let mut media: Vec<(String, String)> = Vec::new();

                // Skip error messages (unless aborted with content)
                if assistant_msg.error.is_some() {
                    let should_skip = !matches!(
                        assistant_msg.error,
                        Some(MessageError::AbortedError { .. }) if msg.parts.iter().any(|p| !matches!(p, Part::StepStart(_) | Part::Reasoning(_)))
                    );
                    if should_skip {
                        continue;
                    }
                }

                let mut assistant_parts: Vec<AssistantContent> = Vec::new();
                let mut tool_calls: Vec<ToolCall> = Vec::new();

                for part in &msg.parts {
                    match part {
                        Part::Text(p) => {
                            assistant_parts.push(AssistantContent::Text {
                                text: p.text.clone(),
                            });
                        }
                        Part::StepStart(_) => {
                            // Step start parts are handled separately in the conversion
                        }
                        Part::Tool(p) => {
                            tool_names.insert(p.tool.clone());

                            match &p.state {
                                ToolState::Completed(state) => {
                                    let output_text = if state.time.compacted.is_some() {
                                        "[Old tool result content cleared]".to_string()
                                    } else {
                                        state.output.clone()
                                    };

                                    let attachments: Vec<&FilePart> =
                                        if state.time.compacted.is_some() || options.strip_media {
                                            Vec::new()
                                        } else {
                                            state
                                                .attachments
                                                .as_ref()
                                                .map(|a| a.iter().collect())
                                                .unwrap_or_default()
                                        };

                                    let media_attachments: Vec<_> = attachments
                                        .iter()
                                        .copied()
                                        .filter(|a| is_media(&a.mime))
                                        .collect();
                                    let non_media_attachments: Vec<_> = attachments
                                        .iter()
                                        .copied()
                                        .filter(|a| !is_media(&a.mime))
                                        .collect();

                                    if !supports_media_in_tool_results
                                        && !media_attachments.is_empty()
                                    {
                                        for attachment in media_attachments {
                                            media.push((
                                                attachment.mime.clone(),
                                                attachment.url.clone(),
                                            ));
                                        }
                                    }

                                    let final_attachments = if supports_media_in_tool_results {
                                        attachments.into_iter().cloned().collect::<Vec<_>>()
                                    } else {
                                        non_media_attachments.into_iter().cloned().collect()
                                    };

                                    let tool_call = ToolCall {
                                        tool_call_id: p.call_id.clone(),
                                        tool_name: p.tool.clone(),
                                        args: state.input.clone(),
                                    };
                                    tool_calls.push(tool_call);

                                    // Add tool result as a separate tool message
                                    let output = if !final_attachments.is_empty() {
                                        serde_json::json!({
                                            "text": output_text,
                                            "attachments": final_attachments
                                        })
                                    } else {
                                        serde_json::json!(output_text)
                                    };

                                    result.push(ModelMessage::Tool {
                                        tool_call_id: p.call_id.clone(),
                                        content: output.to_string(),
                                    });
                                }
                                ToolState::Error(state) => {
                                    let tool_call = ToolCall {
                                        tool_call_id: p.call_id.clone(),
                                        tool_name: p.tool.clone(),
                                        args: state.input.clone(),
                                    };
                                    tool_calls.push(tool_call);

                                    result.push(ModelMessage::Tool {
                                        tool_call_id: p.call_id.clone(),
                                        content: state.error.clone(),
                                    });
                                }
                                ToolState::Pending(state) => {
                                    let tool_call = ToolCall {
                                        tool_call_id: p.call_id.clone(),
                                        tool_name: p.tool.clone(),
                                        args: state.input.clone(),
                                    };
                                    tool_calls.push(tool_call);

                                    result.push(ModelMessage::Tool {
                                        tool_call_id: p.call_id.clone(),
                                        content: "[Tool execution was interrupted]".to_string(),
                                    });
                                }
                                ToolState::Running(state) => {
                                    let tool_call = ToolCall {
                                        tool_call_id: p.call_id.clone(),
                                        tool_name: p.tool.clone(),
                                        args: state.input.clone(),
                                    };
                                    tool_calls.push(tool_call);

                                    result.push(ModelMessage::Tool {
                                        tool_call_id: p.call_id.clone(),
                                        content: "[Tool execution was interrupted]".to_string(),
                                    });
                                }
                            }
                        }
                        Part::Reasoning(p) => {
                            assistant_parts.push(AssistantContent::Reasoning {
                                text: p.text.clone(),
                            });
                        }
                        _ => {}
                    }
                }

                if !assistant_parts.is_empty() || !tool_calls.is_empty() {
                    result.push(ModelMessage::Assistant {
                        id: assistant_msg.id.clone(),
                        content: assistant_parts,
                        tool_calls: if tool_calls.is_empty() {
                            None
                        } else {
                            Some(tool_calls)
                        },
                    });

                    // Inject media as user message for providers that don't support media in tool results
                    if !media.is_empty() {
                        let mut media_parts: Vec<ModelContent> = vec![ModelContent::Text {
                            text: "Attached image(s) from tool result:".to_string(),
                        }];
                        for (_mime, url) in media {
                            media_parts.push(ModelContent::Image {
                                image: ImageContent { url },
                            });
                        }
                        result.push(ModelMessage::User {
                            id: uuid::Uuid::new_v4().to_string(),
                            content: media_parts,
                        });
                    }
                }
            }
        }
    }

    result
}

fn check_media_support(model: &ProviderModel) -> bool {
    match model.api_npm.as_str() {
        "@ai-sdk/anthropic" => true,
        "@ai-sdk/openai" => true,
        "@ai-sdk/amazon-bedrock" => true,
        "@ai-sdk/google-vertex/anthropic" => true,
        "@ai-sdk/google" => {
            let id = model.api_id.to_lowercase();
            id.contains("gemini-3") && !id.contains("gemini-2")
        }
        _ => false,
    }
}

/// Filter compacted messages from a stream
pub fn filter_compacted(messages: Vec<WithParts>) -> Vec<WithParts> {
    let mut result = Vec::new();
    let mut completed = std::collections::HashSet::new();

    for msg in messages {
        result.push(msg.clone());

        if let MessageInfo::User(user) = &msg.info {
            if completed.contains(&user.id)
                && msg.parts.iter().any(|p| matches!(p, Part::Compaction(_)))
            {
                break;
            }
        }

        if let MessageInfo::Assistant(assistant) = &msg.info {
            if assistant.summary.unwrap_or(false)
                && assistant.finish.is_some()
                && assistant.error.is_none()
            {
                completed.insert(assistant.parent_id.clone());
            }
        }
    }

    result.reverse();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::parts::{Part, TextPart};

    #[test]
    fn test_user_message_serialization() {
        let msg = UserMessage {
            id: "msg_123".to_string(),
            session_id: "session_abc".to_string(),
            agent: "build".to_string(),
            model: ModelRef {
                provider_id: "openai".to_string(),
                model_id: "gpt-4".to_string(),
            },
            ..Default::default()
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"agent\":\"build\""));

        let deserialized: UserMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, msg.id);
    }

    #[test]
    fn test_message_info_discriminated_union() {
        let user = MessageInfo::User(UserMessage::default());
        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("\"role\":\"user\""));

        let deserialized: MessageInfo = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, MessageInfo::User(_)));
    }

    #[test]
    fn test_with_parts() {
        let with_parts = WithParts {
            info: MessageInfo::User(UserMessage::default()),
            parts: vec![Part::Text(TextPart {
                id: "p1".to_string(),
                session_id: "s1".to_string(),
                message_id: "m1".to_string(),
                text: "Hello".to_string(),
                ..Default::default()
            })],
        };

        let json = serde_json::to_string(&with_parts).unwrap();
        let deserialized: WithParts = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.parts.len(), 1);
    }

    #[test]
    fn test_to_model_messages_basic() {
        let model = ProviderModel {
            provider_id: "openai".to_string(),
            model_id: "gpt-4".to_string(),
            api_npm: "@ai-sdk/openai".to_string(),
            api_id: "gpt-4".to_string(),
        };

        let user_msg = WithParts {
            info: MessageInfo::User(UserMessage {
                id: "msg1".to_string(),
                session_id: "s1".to_string(),
                ..Default::default()
            }),
            parts: vec![Part::Text(TextPart {
                id: "p1".to_string(),
                session_id: "s1".to_string(),
                message_id: "msg1".to_string(),
                text: "Hello".to_string(),
                ..Default::default()
            })],
        };

        let messages = to_model_messages(&[user_msg], &model, None);
        assert_eq!(messages.len(), 1);

        match &messages[0] {
            ModelMessage::User { content, .. } => {
                assert_eq!(content.len(), 1);
            }
            _ => panic!("Expected user message"),
        }
    }

    #[test]
    fn test_output_format_serialization() {
        let text_format = OutputFormat::Text;
        let json = serde_json::to_string(&text_format).unwrap();
        assert!(json.contains("\"type\":\"text\""));

        let json_format = OutputFormat::JsonSchema {
            schema: serde_json::json!({"type": "object"}),
            retry_count: 3,
        };
        let json = serde_json::to_string(&json_format).unwrap();
        assert!(json.contains("\"type\":\"json_schema\""));
    }

    #[test]
    fn test_check_media_support() {
        let anthropic = ProviderModel {
            provider_id: "anthropic".to_string(),
            model_id: "claude-3".to_string(),
            api_npm: "@ai-sdk/anthropic".to_string(),
            api_id: "claude-3-opus".to_string(),
        };
        assert!(check_media_support(&anthropic));

        let unknown = ProviderModel {
            provider_id: "unknown".to_string(),
            model_id: "model".to_string(),
            api_npm: "@ai-sdk/unknown".to_string(),
            api_id: "model".to_string(),
        };
        assert!(!check_media_support(&unknown));
    }

    #[test]
    fn test_filter_compacted() {
        let messages = vec![WithParts {
            info: MessageInfo::User(UserMessage {
                id: "msg1".to_string(),
                ..Default::default()
            }),
            parts: vec![Part::Text(TextPart {
                id: "p1".to_string(),
                session_id: "s1".to_string(),
                message_id: "msg1".to_string(),
                text: "Hello".to_string(),
                ..Default::default()
            })],
        }];

        let filtered = filter_compacted(messages);
        assert_eq!(filtered.len(), 1);
    }
}
