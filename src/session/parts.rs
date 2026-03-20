//! Message part types for the MessageV2 system
//!
//! These types match the TypeScript Zod schemas in message-v2.ts for compatibility.

use serde::{Deserialize, Serialize};

/// Part ID type alias
pub type PartId = String;

/// Session ID type alias
pub type SessionId = String;

/// Message ID type alias  
pub type MessageId = String;

/// Provider ID type alias
pub type ProviderId = String;

/// Model ID type alias
pub type ModelId = String;

// ============================================================================
// Time tracking structures
// ============================================================================

/// Time tracking for text parts
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TextTime {
    pub start: i64,
    pub end: Option<i64>,
}

/// Time tracking for reasoning parts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningTime {
    pub start: i64,
    pub end: Option<i64>,
}

/// Time tracking for tool parts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolTime {
    pub start: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compacted: Option<i64>,
}

/// Time tracking for running tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningTime {
    pub start: i64,
}

/// Time tracking for retry parts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryTime {
    pub created: i64,
}

// ============================================================================
// File part source types
// ============================================================================

/// Text range in a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePartSourceText {
    pub value: String,
    pub start: i64,
    pub end: i64,
}

/// File source type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum FilePartSource {
    /// Regular file source
    File {
        path: String,
        text: FilePartSourceText,
    },
    /// Symbol source (LSP)
    Symbol {
        path: String,
        range: LspRange,
        name: String,
        kind: i64,
        text: FilePartSourceText,
    },
    /// Resource source (MCP)
    Resource {
        client_name: String,
        uri: String,
        text: FilePartSourceText,
    },
}

/// LSP Range type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

/// LSP Position type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspPosition {
    pub line: i64,
    pub character: i64,
}

// ============================================================================
// Tool state types
// ============================================================================

/// Tool state for pending tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatePending {
    pub input: serde_json::Value,
    pub raw: String,
}

impl Default for ToolStatePending {
    fn default() -> Self {
        Self {
            input: serde_json::json!({}),
            raw: String::new(),
        }
    }
}

/// Tool state for running tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStateRunning {
    pub input: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    pub time: RunningTime,
}

/// Tool state for completed tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStateCompleted {
    pub input: serde_json::Value,
    pub output: String,
    pub title: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
    pub time: ToolTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<FilePart>>,
}

/// Tool state for errored tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStateError {
    pub input: serde_json::Value,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    pub time: ToolTime,
}

/// Discriminated union of tool states
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum ToolState {
    Pending(ToolStatePending),
    Running(ToolStateRunning),
    Completed(ToolStateCompleted),
    Error(ToolStateError),
}

impl Default for ToolState {
    fn default() -> Self {
        Self::Pending(ToolStatePending::default())
    }
}

// ============================================================================
// API Error type
// ============================================================================

/// API error information for retry parts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<i64>,
    pub is_retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_headers: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

// ============================================================================
// Token tracking
// ============================================================================

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<i64>,
    pub input: i64,
    pub output: i64,
    pub reasoning: i64,
    pub cache: CacheInfo,
}

/// Cache token information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheInfo {
    pub read: i64,
    pub write: i64,
}

// ============================================================================
// Message Part Types (without type field - handled by Part enum tag)
// ============================================================================

/// Text message part
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextPart {
    pub id: PartId,
    #[serde(rename = "sessionID")]
    pub session_id: SessionId,
    #[serde(rename = "messageID")]
    pub message_id: MessageId,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synthetic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignored: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<TextTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl Default for TextPart {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: String::new(),
            message_id: String::new(),
            text: String::new(),
            synthetic: None,
            ignored: None,
            time: None,
            metadata: None,
        }
    }
}

/// Reasoning message part (for extended thinking)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningPart {
    pub id: PartId,
    #[serde(rename = "sessionID")]
    pub session_id: SessionId,
    #[serde(rename = "messageID")]
    pub message_id: MessageId,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    pub time: ReasoningTime,
}

/// Tool call message part
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPart {
    pub id: PartId,
    #[serde(rename = "sessionID")]
    pub session_id: SessionId,
    #[serde(rename = "messageID")]
    pub message_id: MessageId,
    #[serde(rename = "callID")]
    pub call_id: String,
    pub tool: String,
    pub state: ToolState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// File message part
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePart {
    pub id: PartId,
    #[serde(rename = "sessionID")]
    pub session_id: SessionId,
    #[serde(rename = "messageID")]
    pub message_id: MessageId,
    pub mime: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<FilePartSource>,
}

/// Snapshot message part
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotPart {
    pub id: PartId,
    #[serde(rename = "sessionID")]
    pub session_id: SessionId,
    #[serde(rename = "messageID")]
    pub message_id: MessageId,
    pub snapshot: String,
}

/// Patch message part
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchPart {
    pub id: PartId,
    #[serde(rename = "sessionID")]
    pub session_id: SessionId,
    #[serde(rename = "messageID")]
    pub message_id: MessageId,
    pub hash: String,
    pub files: Vec<String>,
}

/// Compaction message part
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionPart {
    pub id: PartId,
    #[serde(rename = "sessionID")]
    pub session_id: SessionId,
    #[serde(rename = "messageID")]
    pub message_id: MessageId,
    pub auto: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overflow: Option<bool>,
}

/// Model reference for subtask parts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtaskModel {
    #[serde(rename = "providerID")]
    pub provider_id: ProviderId,
    #[serde(rename = "modelID")]
    pub model_id: ModelId,
}

/// Subtask message part
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtaskPart {
    pub id: PartId,
    #[serde(rename = "sessionID")]
    pub session_id: SessionId,
    #[serde(rename = "messageID")]
    pub message_id: MessageId,
    pub prompt: String,
    pub description: String,
    pub agent: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<SubtaskModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

/// Step start message part
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepStartPart {
    pub id: PartId,
    #[serde(rename = "sessionID")]
    pub session_id: SessionId,
    #[serde(rename = "messageID")]
    pub message_id: MessageId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,
}

/// Step finish message part
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepFinishPart {
    pub id: PartId,
    #[serde(rename = "sessionID")]
    pub session_id: SessionId,
    #[serde(rename = "messageID")]
    pub message_id: MessageId,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,
    pub cost: f64,
    pub tokens: TokenInfo,
}

/// Agent reference for agent parts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSource {
    pub value: String,
    pub start: i64,
    pub end: i64,
}

/// Agent mention message part
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPart {
    pub id: PartId,
    #[serde(rename = "sessionID")]
    pub session_id: SessionId,
    #[serde(rename = "messageID")]
    pub message_id: MessageId,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<AgentSource>,
}

/// Retry message part
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPart {
    pub id: PartId,
    #[serde(rename = "sessionID")]
    pub session_id: SessionId,
    #[serde(rename = "messageID")]
    pub message_id: MessageId,
    pub attempt: i64,
    pub error: ApiError,
    pub time: RetryTime,
}

// ============================================================================
// Part union type
// ============================================================================

/// All message part types as a discriminated union
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Part {
    Text(TextPart),
    Reasoning(ReasoningPart),
    Tool(ToolPart),
    File(FilePart),
    Snapshot(SnapshotPart),
    Patch(PatchPart),
    Compaction(CompactionPart),
    Subtask(SubtaskPart),
    #[serde(rename = "step-start")]
    StepStart(StepStartPart),
    #[serde(rename = "step-finish")]
    StepFinish(StepFinishPart),
    Agent(AgentPart),
    Retry(RetryPart),
}

impl Part {
    pub fn id(&self) -> &str {
        match self {
            Part::Text(p) => &p.id,
            Part::Reasoning(p) => &p.id,
            Part::Tool(p) => &p.id,
            Part::File(p) => &p.id,
            Part::Snapshot(p) => &p.id,
            Part::Patch(p) => &p.id,
            Part::Compaction(p) => &p.id,
            Part::Subtask(p) => &p.id,
            Part::StepStart(p) => &p.id,
            Part::StepFinish(p) => &p.id,
            Part::Agent(p) => &p.id,
            Part::Retry(p) => &p.id,
        }
    }

    pub fn session_id(&self) -> &str {
        match self {
            Part::Text(p) => &p.session_id,
            Part::Reasoning(p) => &p.session_id,
            Part::Tool(p) => &p.session_id,
            Part::File(p) => &p.session_id,
            Part::Snapshot(p) => &p.session_id,
            Part::Patch(p) => &p.session_id,
            Part::Compaction(p) => &p.session_id,
            Part::Subtask(p) => &p.session_id,
            Part::StepStart(p) => &p.session_id,
            Part::StepFinish(p) => &p.session_id,
            Part::Agent(p) => &p.session_id,
            Part::Retry(p) => &p.session_id,
        }
    }

    pub fn message_id(&self) -> &str {
        match self {
            Part::Text(p) => &p.message_id,
            Part::Reasoning(p) => &p.message_id,
            Part::Tool(p) => &p.message_id,
            Part::File(p) => &p.message_id,
            Part::Snapshot(p) => &p.message_id,
            Part::Patch(p) => &p.message_id,
            Part::Compaction(p) => &p.message_id,
            Part::Subtask(p) => &p.message_id,
            Part::StepStart(p) => &p.message_id,
            Part::StepFinish(p) => &p.message_id,
            Part::Agent(p) => &p.message_id,
            Part::Retry(p) => &p.message_id,
        }
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Check if a mime type is media (image or PDF)
pub fn is_media(mime: &str) -> bool {
    mime.starts_with("image/") || mime == "application/pdf"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_part_serialization() {
        let part = Part::Text(TextPart {
            id: "part_123".to_string(),
            session_id: "session_abc".to_string(),
            message_id: "msg_xyz".to_string(),
            text: "Hello, world!".to_string(),
            synthetic: Some(true),
            ..Default::default()
        });

        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"id\":\"part_123\""));
        assert!(json.contains("\"text\":\"Hello, world!\""));
        assert!(json.contains("\"synthetic\":true"));

        let deserialized: Part = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, Part::Text(_)));
        if let Part::Text(p) = deserialized {
            assert_eq!(p.id, "part_123");
            assert_eq!(p.text, "Hello, world!");
        }
    }

    #[test]
    fn test_tool_state_pending() {
        let state = ToolState::Pending(ToolStatePending {
            input: serde_json::json!({"path": "/test"}),
            raw: "test".to_string(),
        });

        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("\"status\":\"pending\""));

        let deserialized: ToolState = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, ToolState::Pending(_)));
    }

    #[test]
    fn test_tool_state_completed() {
        let state = ToolState::Completed(ToolStateCompleted {
            input: serde_json::json!({"query": "test"}),
            output: "result".to_string(),
            title: "File Read".to_string(),
            metadata: serde_json::json!({}),
            time: ToolTime {
                start: 1000,
                end: Some(2000),
                compacted: None,
            },
            attachments: None,
        });

        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("\"status\":\"completed\""));
        assert!(json.contains("\"output\":\"result\""));
    }

    #[test]
    fn test_part_union_serialization() {
        let part = Part::Text(TextPart {
            id: "p1".to_string(),
            session_id: "s1".to_string(),
            message_id: "m1".to_string(),
            text: "Hello".to_string(),
            ..Default::default()
        });

        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains("\"type\":\"text\""));

        let deserialized: Part = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, Part::Text(_)));
    }

    #[test]
    fn test_file_part_source_serialization() {
        let source = FilePartSource::File {
            path: "/src/main.rs".to_string(),
            text: FilePartSourceText {
                value: "fn main() {}".to_string(),
                start: 0,
                end: 13,
            },
        };

        let json = serde_json::to_string(&source).unwrap();
        assert!(json.contains("\"type\":\"file\""));
        assert!(json.contains("\"path\":\"/src/main.rs\""));

        let deserialized: FilePartSource = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, FilePartSource::File { .. }));
    }

    #[test]
    fn test_is_media() {
        assert!(is_media("image/png"));
        assert!(is_media("image/jpeg"));
        assert!(is_media("application/pdf"));
        assert!(!is_media("text/plain"));
        assert!(!is_media("application/json"));
    }

    #[test]
    fn test_step_parts_serialization() {
        let step_start = Part::StepStart(StepStartPart {
            id: "p1".to_string(),
            session_id: "s1".to_string(),
            message_id: "m1".to_string(),
            snapshot: Some("snap_123".to_string()),
        });

        let json = serde_json::to_string(&step_start).unwrap();
        assert!(json.contains("\"type\":\"step-start\""));

        let step_finish = Part::StepFinish(StepFinishPart {
            id: "p2".to_string(),
            session_id: "s1".to_string(),
            message_id: "m1".to_string(),
            reason: "completed".to_string(),
            snapshot: None,
            cost: 0.001,
            tokens: TokenInfo::default(),
        });

        let json = serde_json::to_string(&step_finish).unwrap();
        assert!(json.contains("\"type\":\"step-finish\""));
        assert!(json.contains("\"cost\":0.001"));
    }

    #[test]
    fn test_compaction_part() {
        let part = Part::Compaction(CompactionPart {
            id: "p1".to_string(),
            session_id: "s1".to_string(),
            message_id: "m1".to_string(),
            auto: true,
            overflow: Some(true),
        });

        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains("\"type\":\"compaction\""));
        assert!(json.contains("\"auto\":true"));
        assert!(json.contains("\"overflow\":true"));
    }

    #[test]
    fn test_subtask_part() {
        let part = Part::Subtask(SubtaskPart {
            id: "p1".to_string(),
            session_id: "s1".to_string(),
            message_id: "m1".to_string(),
            prompt: "Write a function".to_string(),
            description: "Code generation task".to_string(),
            agent: "build".to_string(),
            model: Some(SubtaskModel {
                provider_id: "openai".to_string(),
                model_id: "gpt-4".to_string(),
            }),
            command: None,
        });

        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains("\"type\":\"subtask\""));
        assert!(json.contains("\"prompt\":\"Write a function\""));
        assert!(json.contains("\"agent\":\"build\""));
    }

    #[test]
    fn test_retry_part() {
        let part = Part::Retry(RetryPart {
            id: "p1".to_string(),
            session_id: "s1".to_string(),
            message_id: "m1".to_string(),
            attempt: 2,
            error: ApiError {
                message: "Connection timeout".to_string(),
                status_code: Some(504),
                is_retryable: true,
                response_headers: None,
                response_body: None,
                metadata: None,
            },
            time: RetryTime {
                created: 1234567890,
            },
        });

        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains("\"type\":\"retry\""));
        assert!(json.contains("\"attempt\":2"));
        assert!(json.contains("\"is_retryable\":true"));
    }

    #[test]
    fn test_part_id_extraction() {
        let text_part = Part::Text(TextPart {
            id: "p123".to_string(),
            session_id: "s1".to_string(),
            message_id: "m1".to_string(),
            text: "test".to_string(),
            ..Default::default()
        });

        assert_eq!(text_part.id(), "p123");
        assert_eq!(text_part.session_id(), "s1");
        assert_eq!(text_part.message_id(), "m1");
    }
}
