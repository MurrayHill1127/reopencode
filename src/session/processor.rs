//! Session stream event processor
//!
//! Handles streaming events from LLM responses, including text, reasoning,
//! tool calls, and step tracking. Mirrors TypeScript processor.ts functionality.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use tracing::{info, error};

use crate::bus::{Bus, EventDefinition};
use crate::session::parts::{
    PartId, SessionId, TextPart, TextTime,
    ReasoningPart, ToolPart, ToolState,
    ToolStateRunning, ToolStateCompleted, ToolStateError,
    ToolTime, RunningTime, TokenInfo,
};
use crate::session::message::{AssistantMessage, MessageError};
use crate::session::llm::{StreamEvent, StreamInput, stream};
use crate::session::status;
use crate::provider::Provider;

/// Doom loop detection threshold
const DOOM_LOOP_THRESHOLD: usize = 3;

/// Session error event definition
pub const SESSION_ERROR: EventDefinition<SessionErrorProperties> =
    EventDefinition::new("session.error");

/// Properties for session error events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionErrorProperties {
    pub session_id: String,
    pub error: MessageError,
}

/// Processor result after processing stream
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessResult {
    /// Continue with next iteration
    Continue,
    /// Stop processing (blocked or error)
    Stop,
    /// Need compaction due to context overflow
    Compact,
}

/// Processor info returned from create()
pub struct Processor {
    /// The assistant message being built
    message: AssistantMessage,
    /// Session ID
    session_id: SessionId,
    /// Abort signal receiver
    abort: Option<watch::Receiver<bool>>,
    /// Tool call tracking by call ID
    tool_calls: HashMap<String, ToolPart>,
    /// Current snapshot ID for step tracking
    snapshot: Option<String>,
    /// Whether processing was blocked by permission denial
    blocked: bool,
    /// Retry attempt count
    attempt: u32,
    /// Whether compaction is needed
    needs_compaction: bool,
    /// Bus for event publishing
    bus: Option<Arc<Bus>>,
}

impl Processor {
    /// Create a new processor for an assistant message
    pub fn new(
        message: AssistantMessage,
        abort: Option<watch::Receiver<bool>>,
    ) -> Self {
        Self {
            session_id: message.session_id.clone(),
            message,
            abort,
            tool_calls: HashMap::new(),
            snapshot: None,
            blocked: false,
            attempt: 0,
            needs_compaction: false,
            bus: None,
        }
    }

    /// Set the bus for event publishing
    pub fn with_bus(mut self, bus: Arc<Bus>) -> Self {
        self.bus = Some(bus);
        self
    }

    /// Get a reference to the assistant message
    pub fn message(&self) -> &AssistantMessage {
        &self.message
    }

    /// Get a mutable reference to the assistant message
    pub fn message_mut(&mut self) -> &mut AssistantMessage {
        &mut self.message
    }

    /// Get a tool part by call ID
    pub fn part_from_tool_call(&self, tool_call_id: &str) -> Option<&ToolPart> {
        self.tool_calls.get(tool_call_id)
    }

    /// Process a stream of events
    pub async fn process(
        &mut self,
        input: StreamInput,
        provider: Arc<dyn Provider>,
    ) -> ProcessResult {
        info!("processor: process started");
        self.needs_compaction = false;

        loop {
            // Check abort signal
            if let Some(ref abort_rx) = self.abort {
                if *abort_rx.borrow() {
                    info!("processor: aborted");
                    return ProcessResult::Stop;
                }
            }

            match self.process_stream_iteration(&input, provider.clone()).await {
                Ok(result) => return result,
                Err(e) => {
                    error!("processor: error during processing: {:?}", e);
                    
                    // Handle context overflow
                    if matches!(e, ProcessError::ContextOverflow) {
                        self.needs_compaction = true;
                        self.publish_error(MessageError::ContextOverflowError {
                            message: "Context window exceeded".to_string(),
                            response_body: None,
                        }).await;
                        return ProcessResult::Compact;
                    }

                    // Check for retryable errors
                    if self.attempt < 3 {
                        self.attempt += 1;
                        let delay = self.retry_delay(&e);
                        status::set(&self.session_id, status::SessionStatusInfo::Retry {
                            attempt: self.attempt as i32,
                            message: e.to_string(),
                            next: chrono::Utc::now().timestamp_millis() + delay as i64,
                        });
                        
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                        
                        // Check abort during retry delay
                        if let Some(ref abort_rx) = self.abort {
                            if *abort_rx.borrow() {
                                return ProcessResult::Stop;
                            }
                        }
                        continue;
                    }

                    // Non-retryable error or max retries exceeded
                    self.message.error = Some(self.error_to_message_error(&e));
                    self.publish_error(self.message.error.clone().unwrap()).await;
                    status::set(&self.session_id, status::SessionStatusInfo::Idle);
                    return ProcessResult::Stop;
                }
            }
        }
    }

    /// Process a single stream iteration
    async fn process_stream_iteration(
        &mut self,
        input: &StreamInput,
        provider: Arc<dyn Provider>,
    ) -> Result<ProcessResult, ProcessError> {
        let stream_result = stream(input.clone(), provider).await;
        let mut stream = stream_result.stream;

        // Track current text and reasoning parts
        let mut current_text: Option<TextPart> = None;
        let reasoning_map: HashMap<String, ReasoningPart> = HashMap::new();

        // Set status to busy
        status::set(&self.session_id, status::SessionStatusInfo::Busy);

        while let Some(event) = futures::StreamExt::next(&mut stream).await {
            // Check abort
            if let Some(ref abort_rx) = self.abort {
                if *abort_rx.borrow() {
                    return Ok(ProcessResult::Stop);
                }
            }

            match event {
                StreamEvent::TextDelta { delta } => {
                    self.handle_text_delta(&mut current_text, &delta).await;
                }
                StreamEvent::ReasoningDelta { delta } => {
                    // For now, we'll need to track reasoning similar to text
                    // This is a simplified version
                    info!("processor: reasoning delta received");
                }
                StreamEvent::ToolCall { id, name, arguments } => {
                    self.handle_tool_call(&id, &name, &arguments).await;
                }
                StreamEvent::ToolResult { id, result } => {
                    self.handle_tool_result(&id, &result).await;
                }
                StreamEvent::Usage { input, output, total } => {
                    self.message.tokens.input = input as i64;
                    self.message.tokens.output = output as i64;
                    self.message.tokens.total = Some(total as i64);
                }
                StreamEvent::Finish { reason } => {
                    self.message.finish = Some(reason);
                    self.handle_step_finish().await;
                }
                StreamEvent::Error { message } => {
                    return Err(ProcessError::ProviderError(message));
                }
            }

            if self.needs_compaction {
                break;
            }
        }

        // Finalize any incomplete tool calls
        self.finalize_tool_calls().await;

        // Mark message as completed
        self.message.time.completed = Some(chrono::Utc::now().timestamp_millis());

        if self.needs_compaction {
            return Ok(ProcessResult::Compact);
        }
        if self.blocked {
            return Ok(ProcessResult::Stop);
        }
        if self.message.error.is_some() {
            return Ok(ProcessResult::Stop);
        }

        Ok(ProcessResult::Continue)
    }

    /// Handle text delta event
    async fn handle_text_delta(&mut self, current_text: &mut Option<TextPart>, delta: &str) {
        if let Some(text_part) = current_text {
            text_part.text.push_str(delta);
        } else {
            // Create new text part
            let now = chrono::Utc::now().timestamp_millis();
            let text_part = TextPart {
                id: generate_part_id(),
                session_id: self.session_id.clone(),
                message_id: self.message.id.clone(),
                text: delta.to_string(),
                time: Some(TextTime {
                    start: now,
                    end: None,
                }),
                ..Default::default()
            };
            *current_text = Some(text_part);
        }
        // In a full implementation, we would persist the delta to the database
        info!("processor: text delta: {} bytes", delta.len());
    }

    /// Handle tool call event
    async fn handle_tool_call(&mut self, id: &str, name: &str, arguments: &str) {
        info!("processor: tool call: {} ({})", name, id);

        let now = chrono::Utc::now().timestamp_millis();
        
        // Create or update tool part
        let part = ToolPart {
            id: self.tool_calls.get(id).map(|p| p.id.clone()).unwrap_or_else(generate_part_id),
            session_id: self.session_id.clone(),
            message_id: self.message.id.clone(),
            call_id: id.to_string(),
            tool: name.to_string(),
            state: ToolState::Running(ToolStateRunning {
                input: parse_json_or_default(arguments),
                title: None,
                metadata: None,
                time: RunningTime { start: now },
            }),
            metadata: None,
        };

        self.tool_calls.insert(id.to_string(), part);

        // Check for doom loop (same tool called with same args repeatedly)
        self.check_doom_loop(id, name, arguments);
    }

    /// Handle tool result event
    async fn handle_tool_result(&mut self, id: &str, result: &str) {
        info!("processor: tool result for {}: {} bytes", id, result.len());

        if let Some(part) = self.tool_calls.get_mut(id) {
            let now = chrono::Utc::now().timestamp_millis();
            
            if let ToolState::Running(running) = &part.state {
                part.state = ToolState::Completed(ToolStateCompleted {
                    input: running.input.clone(),
                    output: result.to_string(),
                    title: "Tool execution".to_string(),
                    metadata: running.metadata.clone().unwrap_or(serde_json::json!({})),
                    time: ToolTime {
                        start: running.time.start,
                        end: Some(now),
                        compacted: None,
                    },
                    attachments: None,
                });
            }
        }
    }

    /// Check for doom loop pattern
    fn check_doom_loop(&self, _id: &str, name: &str, _arguments: &str) {
        // In a full implementation, we would check the last N tool calls
        // for repeated patterns and trigger permission request
        // For now, just log
        info!("processor: checking doom loop for tool {}", name);
    }

    /// Handle step finish
    async fn handle_step_finish(&mut self) {
        info!("processor: step finish");
        
        // In a full implementation, we would:
        // 1. Track snapshot changes
        // 2. Create StepFinishPart
        // 3. Update message cost and tokens
        // 4. Trigger summarization if needed
    }

    /// Finalize any incomplete tool calls
    async fn finalize_tool_calls(&mut self) {
        let now = chrono::Utc::now().timestamp_millis();

        for (_id, part) in self.tool_calls.iter_mut() {
            if matches!(part.state, ToolState::Pending(_)) || matches!(part.state, ToolState::Running(_)) {
                part.state = ToolState::Error(ToolStateError {
                    input: match &part.state {
                        ToolState::Pending(p) => p.input.clone(),
                        ToolState::Running(r) => r.input.clone(),
                        _ => serde_json::json!({}),
                    },
                    error: "Tool execution aborted".to_string(),
                    metadata: None,
                    time: ToolTime {
                        start: now,
                        end: Some(now),
                        compacted: None,
                    },
                });
            }
        }
    }

    /// Publish an error event to the bus
    async fn publish_error(&self, error: MessageError) {
        if let Some(ref bus) = self.bus {
            bus.publish(&SESSION_ERROR, SessionErrorProperties {
                session_id: self.session_id.clone(),
                error,
            }).await;
        }
    }

    /// Calculate retry delay based on error type and attempt
    fn retry_delay(&self, error: &ProcessError) -> u64 {
        // Exponential backoff with jitter
        let base_delay = 1000u64;
        let max_delay = 30000u64;
        
        let delay = base_delay * (2u64.pow(self.attempt));
        let delay = delay.min(max_delay);
        
        // Add some jitter
        let jitter = (delay as f64 * 0.1) as u64;
        delay + jitter
    }

    /// Convert process error to message error
    fn error_to_message_error(&self, error: &ProcessError) -> MessageError {
        match error {
            ProcessError::ProviderError(msg) => MessageError::APIError {
                message: msg.clone(),
                status_code: None,
                is_retryable: false,
                response_headers: None,
                response_body: None,
                metadata: None,
            },
            ProcessError::ContextOverflow => MessageError::ContextOverflowError {
                message: "Context window exceeded".to_string(),
                response_body: None,
            },
            ProcessError::Aborted(msg) => MessageError::AbortedError {
                message: msg.clone(),
            },
            ProcessError::Other(msg) => MessageError::UnknownError {
                message: msg.clone(),
            },
        }
    }
}

/// Processing errors
#[derive(Debug, Clone)]
pub enum ProcessError {
    /// Provider error
    ProviderError(String),
    /// Context window overflow
    ContextOverflow,
    /// Aborted by user
    Aborted(String),
    /// Other error
    Other(String),
}

impl std::fmt::Display for ProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessError::ProviderError(msg) => write!(f, "Provider error: {}", msg),
            ProcessError::ContextOverflow => write!(f, "Context overflow"),
            ProcessError::Aborted(msg) => write!(f, "Aborted: {}", msg),
            ProcessError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for ProcessError {}

/// Generate a new part ID
fn generate_part_id() -> PartId {
    format!("part_{}", uuid::Uuid::new_v4())
}

/// Parse JSON or return empty object
fn parse_json_or_default(s: &str) -> serde_json::Value {
    serde_json::from_str(s).unwrap_or(serde_json::json!({}))
}

// ============================================================================
// Step tracking types
// ============================================================================

/// Step state for tracking multi-step responses
#[derive(Debug, Clone, Default)]
pub struct StepState {
    /// Current step number
    pub step: u32,
    /// Total tokens used in this step
    pub tokens: TokenInfo,
    /// Cost for this step
    pub cost: f64,
    /// Snapshot ID at step start
    pub snapshot: Option<String>,
    /// Reason for step finish
    pub finish_reason: Option<String>,
}

impl StepState {
    /// Create a new step state
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a new step
    pub fn start(&mut self, snapshot: Option<String>) {
        self.step += 1;
        self.tokens = TokenInfo::default();
        self.cost = 0.0;
        self.snapshot = snapshot;
        self.finish_reason = None;
    }

    /// Finish the current step
    pub fn finish(&mut self, reason: String, tokens: TokenInfo, cost: f64) {
        self.finish_reason = Some(reason);
        self.tokens = tokens;
        self.cost = cost;
    }
}

// ============================================================================
// Retry logic
// ============================================================================

/// Check if an error is retryable
pub fn is_retryable(error: &MessageError) -> Option<String> {
    match error {
        MessageError::APIError { is_retryable, message, .. } if *is_retryable => {
            Some(message.clone())
        }
        MessageError::OutputLengthError {} => {
            Some("Output length exceeded".to_string())
        }
        _ => None,
    }
}

/// Calculate retry delay with exponential backoff
pub fn retry_delay(attempt: u32, is_api_error: bool) -> u64 {
    let base = if is_api_error { 2000u64 } else { 1000u64 };
    let max = 30000u64;
    
    let delay = base * (2u64.pow(attempt));
    delay.min(max)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_result_variants() {
        assert_eq!(ProcessResult::Continue, ProcessResult::Continue);
        assert_ne!(ProcessResult::Continue, ProcessResult::Stop);
        assert_ne!(ProcessResult::Stop, ProcessResult::Compact);
    }

    #[test]
    fn test_processor_new() {
        let msg = AssistantMessage {
            id: "msg_123".to_string(),
            session_id: "session_abc".to_string(),
            time: crate::session::message::AssistantTime {
                created: 0,
                completed: None,
            },
            error: None,
            parent_id: "parent".to_string(),
            model_id: "model".to_string(),
            provider_id: "provider".to_string(),
            mode: "default".to_string(),
            agent: "build".to_string(),
            path: crate::session::message::PathInfo {
                cwd: "/".to_string(),
                root: "/".to_string(),
            },
            summary: None,
            cost: 0.0,
            tokens: TokenInfo::default(),
            structured: None,
            variant: None,
            finish: None,
        };

        let processor = Processor::new(msg.clone(), None);
        assert_eq!(processor.message().id, "msg_123");
        assert_eq!(processor.session_id, "session_abc");
        assert!(processor.tool_calls.is_empty());
        assert!(!processor.blocked);
        assert!(!processor.needs_compaction);
    }

    #[test]
    fn test_processor_with_bus() {
        let msg = AssistantMessage {
            id: "msg_456".to_string(),
            session_id: "session_def".to_string(),
            time: crate::session::message::AssistantTime {
                created: 0,
                completed: None,
            },
            error: None,
            parent_id: "parent".to_string(),
            model_id: "model".to_string(),
            provider_id: "provider".to_string(),
            mode: "default".to_string(),
            agent: "build".to_string(),
            path: crate::session::message::PathInfo {
                cwd: "/".to_string(),
                root: "/".to_string(),
            },
            summary: None,
            cost: 0.0,
            tokens: TokenInfo::default(),
            structured: None,
            variant: None,
            finish: None,
        };

        let bus = Arc::new(Bus::new("/test"));
        let processor = Processor::new(msg, None).with_bus(bus);
        assert!(processor.bus.is_some());
    }

    #[test]
    fn test_step_state() {
        let mut state = StepState::new();
        assert_eq!(state.step, 0);

        state.start(Some("snap_123".to_string()));
        assert_eq!(state.step, 1);
        assert_eq!(state.snapshot, Some("snap_123".to_string()));

        let tokens = TokenInfo {
            total: Some(100),
            input: 80,
            output: 20,
            reasoning: 0,
            cache: Default::default(),
        };
        state.finish("stop".to_string(), tokens.clone(), 0.001);
        assert_eq!(state.finish_reason, Some("stop".to_string()));
        assert_eq!(state.tokens.input, 80);
        assert_eq!(state.cost, 0.001);
    }

    #[test]
    fn test_process_error_display() {
        let err = ProcessError::ProviderError("Rate limit".to_string());
        assert_eq!(format!("{}", err), "Provider error: Rate limit");

        let err = ProcessError::ContextOverflow;
        assert_eq!(format!("{}", err), "Context overflow");

        let err = ProcessError::Aborted("User cancelled".to_string());
        assert_eq!(format!("{}", err), "Aborted: User cancelled");
    }

    #[test]
    fn test_is_retryable() {
        let retryable = MessageError::APIError {
            message: "Rate limit".to_string(),
            status_code: Some(429),
            is_retryable: true,
            response_headers: None,
            response_body: None,
            metadata: None,
        };
        assert!(is_retryable(&retryable).is_some());

        let non_retryable = MessageError::APIError {
            message: "Invalid request".to_string(),
            status_code: Some(400),
            is_retryable: false,
            response_headers: None,
            response_body: None,
            metadata: None,
        };
        assert!(is_retryable(&non_retryable).is_none());

        let output_error = MessageError::OutputLengthError {};
        assert!(is_retryable(&output_error).is_some());
    }

    #[test]
    fn test_retry_delay() {
        assert_eq!(retry_delay(0, false), 1000);
        assert_eq!(retry_delay(1, false), 2000);
        assert_eq!(retry_delay(2, false), 4000);
        assert_eq!(retry_delay(3, false), 8000);
        
        // API errors have higher base
        assert_eq!(retry_delay(0, true), 2000);
        assert_eq!(retry_delay(1, true), 4000);
        
        // Cap at max
        assert_eq!(retry_delay(10, false), 30000);
    }

    #[test]
    fn test_generate_part_id() {
        let id1 = generate_part_id();
        let id2 = generate_part_id();
        assert!(id1.starts_with("part_"));
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_parse_json_or_default() {
        let valid = parse_json_or_default(r#"{"key": "value"}"#);
        assert_eq!(valid["key"], "value");

        let invalid = parse_json_or_default("not json");
        assert!(invalid.is_object());
    }

    #[test]
    fn test_error_to_message_error() {
        let processor = Processor::new(AssistantMessage {
            id: "msg".to_string(),
            session_id: "sess".to_string(),
            time: crate::session::message::AssistantTime {
                created: 0,
                completed: None,
            },
            error: None,
            parent_id: "p".to_string(),
            model_id: "m".to_string(),
            provider_id: "prov".to_string(),
            mode: "default".to_string(),
            agent: "build".to_string(),
            path: crate::session::message::PathInfo {
                cwd: "/".to_string(),
                root: "/".to_string(),
            },
            summary: None,
            cost: 0.0,
            tokens: TokenInfo::default(),
            structured: None,
            variant: None,
            finish: None,
        }, None);

        let err = ProcessError::ProviderError("test error".to_string());
        let msg_err = processor.error_to_message_error(&err);
        assert!(matches!(msg_err, MessageError::APIError { .. }));

        let err = ProcessError::ContextOverflow;
        let msg_err = processor.error_to_message_error(&err);
        assert!(matches!(msg_err, MessageError::ContextOverflowError { .. }));

        let err = ProcessError::Aborted("user".to_string());
        let msg_err = processor.error_to_message_error(&err);
        assert!(matches!(msg_err, MessageError::AbortedError { .. }));
    }

    #[test]
    fn test_session_error_event() {
        assert_eq!(SESSION_ERROR.event_type, "session.error");
    }

    #[test]
    fn test_session_error_properties() {
        let props = SessionErrorProperties {
            session_id: "sess_123".to_string(),
            error: MessageError::AbortedError {
                message: "User cancelled".to_string(),
            },
        };

        let json = serde_json::to_string(&props).unwrap();
        assert!(json.contains("\"session_id\":\"sess_123\""));
        // MessageError uses #[serde(tag = "name", rename_all = "camelCase")]
        // so AbortedError is serialized as "abortedError"
        assert!(json.contains("\"name\":\"abortedError\""));
    }
}