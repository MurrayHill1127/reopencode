//! Message processor for handling streaming responses from AI providers
//!
//! This module provides the core processor infrastructure for handling streaming
//! responses, including:
//! - Stream event processing and accumulation
//! - Reasoning (thinking) content handling
//! - Tool call state management
//! - Error detection and retry mechanisms
//! - Doom loop detection
//!
//! ## Stream Processing
//!
//! The processor handles various stream events from providers:
//! - Text content (start/delta/end)
//! - Reasoning content (start/delta/end)
//! - Tool calls (start/delta/end/result/error)
//! - Errors and finish events
//!
//! ## Retry Mechanism
//!
//! The processor implements exponential backoff for retryable errors:
//! - Rate limits
//! - Network timeouts
//! - Temporary API errors
//!
//! ## Doom Loop Detection
//!
//! Detects when the agent is stuck in a loop calling the same tool
//! with identical inputs (threshold: 3 consecutive identical calls).

use std::collections::HashMap;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use super::loop_::FinishReason;

/// Threshold for doom loop detection (consecutive identical tool calls)
pub const DOOM_LOOP_THRESHOLD: usize = 3;

/// Default retry configuration
pub const DEFAULT_MAX_RETRY_ATTEMPTS: u32 = 5;
pub const DEFAULT_BASE_DELAY_MS: u64 = 1000;
pub const DEFAULT_MAX_DELAY_MS: u64 = 60_000;
pub const DEFAULT_EXPONENTIAL_BASE: f64 = 2.0;

// ============================================================================
// Stream Event Types
// ============================================================================

/// Stream event types from provider
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    /// Stream started
    Start,
    /// Text content started
    TextStart {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        metadata: Option<serde_json::Value>,
    },
    /// Text content delta
    TextDelta {
        id: String,
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        metadata: Option<serde_json::Value>,
    },
    /// Text content ended
    TextEnd {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        metadata: Option<serde_json::Value>,
    },
    /// Reasoning (thinking) started
    ReasoningStart {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        metadata: Option<serde_json::Value>,
    },
    /// Reasoning content delta
    ReasoningDelta {
        id: String,
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        metadata: Option<serde_json::Value>,
    },
    /// Reasoning ended
    ReasoningEnd {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        metadata: Option<serde_json::Value>,
    },
    /// Tool call started
    ToolCallStart { id: String, tool_name: String },
    /// Tool call input delta
    ToolCallDelta { id: String, delta: String },
    /// Tool call ended with complete input
    ToolCallEnd {
        id: String,
        input: serde_json::Value,
    },
    /// Tool execution result
    ToolResult {
        id: String,
        output: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attachments: Option<Vec<serde_json::Value>>,
    },
    /// Tool execution error
    ToolError { id: String, error: String },
    /// Stream error
    Error { error: ProcessorError },
    /// Stream finished
    Finish {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        usage: Option<Usage>,
    },
}

// ============================================================================
// Error Types
// ============================================================================

/// Processor error types
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProcessorError {
    #[error("Rate limit exceeded")]
    RateLimit {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        retry_after_ms: Option<u64>,
    },
    #[error("Authentication failed")]
    Authentication,
    #[error("Context overflow: {message}")]
    ContextOverflow { message: String },
    #[error("Network error: {message}")]
    Network { message: String },
    #[error("API error: {message}")]
    Api {
        message: String,
        code: Option<String>,
    },
    #[error("Request timeout")]
    Timeout,
    #[error("Stream error: {message}")]
    Stream { message: String },
    #[error("Tool error: {message}")]
    Tool { message: String },
    #[error("Permission denied: {message}")]
    PermissionDenied { message: String },
    #[error("Unknown error: {message}")]
    Unknown { message: String },
}

impl ProcessorError {
    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            ProcessorError::RateLimit { .. } => true,
            ProcessorError::Network { .. } => true,
            ProcessorError::Timeout => true,
            ProcessorError::Api {
                code: Some(code), ..
            } => Self::is_retryable_api_code(code),
            _ => false,
        }
    }

    /// Check if this is a context overflow error
    pub fn is_context_overflow(&self) -> bool {
        matches!(self, ProcessorError::ContextOverflow { .. })
    }

    /// Check if this is an authentication error
    pub fn is_authentication(&self) -> bool {
        matches!(self, ProcessorError::Authentication)
    }

    /// Check if this is a rate limit error
    pub fn is_rate_limit(&self) -> bool {
        matches!(self, ProcessorError::RateLimit { .. })
    }

    /// Check if API code is retryable
    fn is_retryable_api_code(code: &str) -> bool {
        matches!(
            code,
            "overloaded" | "server_error" | "internal_error" | "529"
        )
    }

    /// Create from error string (for parsing provider errors)
    pub fn from_error_string(s: &str) -> Self {
        let s_lower = s.to_lowercase();

        if s_lower.contains("rate limit") || s_lower.contains("429") {
            return ProcessorError::RateLimit {
                retry_after_ms: None,
            };
        }
        if s_lower.contains("unauthorized")
            || s_lower.contains("401")
            || s_lower.contains("authentication")
        {
            return ProcessorError::Authentication;
        }
        if s_lower.contains("context")
            && (s_lower.contains("overflow")
                || s_lower.contains("length")
                || s_lower.contains("too long"))
        {
            return ProcessorError::ContextOverflow {
                message: s.to_string(),
            };
        }
        if s_lower.contains("timeout") {
            return ProcessorError::Timeout;
        }
        if s_lower.contains("network") || s_lower.contains("connection") {
            return ProcessorError::Network {
                message: s.to_string(),
            };
        }

        ProcessorError::Api {
            message: s.to_string(),
            code: None,
        }
    }

    /// Get retry delay hint (for rate limits)
    pub fn retry_after_ms(&self) -> Option<u64> {
        match self {
            ProcessorError::RateLimit { retry_after_ms } => *retry_after_ms,
            _ => None,
        }
    }
}

// ============================================================================
// Configuration Types
// ============================================================================

/// Retry configuration with exponential backoff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Base delay in milliseconds
    pub base_delay_ms: u64,
    /// Maximum delay in milliseconds
    pub max_delay_ms: u64,
    /// Exponential base for backoff
    pub exponential_base: f64,
    /// Add jitter to delays (default: true)
    #[serde(default = "default_jitter")]
    pub jitter: bool,
}

fn default_jitter() -> bool {
    true
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: DEFAULT_MAX_RETRY_ATTEMPTS,
            base_delay_ms: DEFAULT_BASE_DELAY_MS,
            max_delay_ms: DEFAULT_MAX_DELAY_MS,
            exponential_base: DEFAULT_EXPONENTIAL_BASE,
            jitter: true,
        }
    }
}

impl RetryConfig {
    /// Create a new retry config with custom values
    pub fn new(max_attempts: u32, base_delay_ms: u64, max_delay_ms: u64) -> Self {
        Self {
            max_attempts,
            base_delay_ms,
            max_delay_ms,
            exponential_base: DEFAULT_EXPONENTIAL_BASE,
            jitter: true,
        }
    }

    /// Create a fast retry config (for non-critical operations)
    pub fn fast() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 500,
            max_delay_ms: 10_000,
            exponential_base: 1.5,
            jitter: true,
        }
    }

    /// Create an aggressive retry config (for important operations)
    pub fn aggressive() -> Self {
        Self {
            max_attempts: 10,
            base_delay_ms: 1000,
            max_delay_ms: 120_000,
            exponential_base: 2.0,
            jitter: true,
        }
    }
}

/// Processor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessorConfig {
    /// Retry configuration
    pub retry: RetryConfig,
    /// Enable doom loop detection
    pub doom_loop_detection: bool,
    /// Maximum tool calls to track for doom loop
    pub doom_loop_history: usize,
    /// Enable reasoning accumulation
    pub track_reasoning: bool,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            retry: RetryConfig::default(),
            doom_loop_detection: true,
            doom_loop_history: DOOM_LOOP_THRESHOLD,
            track_reasoning: true,
        }
    }
}

// ============================================================================
// Result Types
// ============================================================================

/// Result of processing a stream event
#[derive(Debug, Clone)]
pub enum ProcessResult {
    /// Continue processing
    Continue,
    /// Stop processing with reason
    Stop { reason: FinishReason },
    /// Context overflow detected, need compaction
    Compact,
    /// Retry after delay
    Retry { attempt: u32, delay_ms: u64 },
    /// Doom loop detected
    DoomLoopDetected { tool_name: String },
}

// ============================================================================
// Tool Call State
// ============================================================================

/// Tool call status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    /// Tool call is pending (input still being received)
    Pending,
    /// Tool is running
    Running,
    /// Tool completed successfully
    Completed,
    /// Tool failed with error
    Error,
}

impl Default for ToolCallStatus {
    fn default() -> Self {
        ToolCallStatus::Pending
    }
}

/// Tool call state tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallState {
    /// Unique identifier for this tool call
    pub id: String,
    /// Name of the tool
    pub tool_name: String,
    /// Tool input arguments
    pub input: serde_json::Value,
    /// Raw input string (for accumulation)
    pub raw_input: String,
    /// Current status
    pub status: ToolCallStatus,
    /// Output (if completed)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// Error message (if failed)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Timing information
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<TimeInfo>,
    /// Metadata from provider
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl ToolCallState {
    /// Create a new pending tool call
    pub fn new(id: String, tool_name: String) -> Self {
        Self {
            id,
            tool_name,
            input: serde_json::Value::Object(serde_json::Map::new()),
            raw_input: String::new(),
            status: ToolCallStatus::Pending,
            output: None,
            error: None,
            time: None,
            metadata: None,
        }
    }

    /// Append delta to raw input
    pub fn append_delta(&mut self, delta: &str) {
        self.raw_input.push_str(delta);
    }

    /// Finalize the input from raw string
    pub fn finalize_input(&mut self) {
        self.input = serde_json::from_str(&self.raw_input)
            .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new()));
        self.status = ToolCallStatus::Running;
    }

    /// Mark as running
    pub fn set_running(&mut self, input: serde_json::Value) {
        self.input = input;
        self.status = ToolCallStatus::Running;
        self.time = Some(TimeInfo {
            start: Some(Instant::now()),
            end: None,
        });
    }

    /// Mark as completed with output
    pub fn complete(&mut self, output: String) {
        self.output = Some(output);
        self.status = ToolCallStatus::Completed;
        if let Some(ref mut time) = self.time {
            time.end = Some(Instant::now());
        }
    }

    /// Mark as failed with error
    pub fn fail(&mut self, error: String) {
        self.error = Some(error);
        self.status = ToolCallStatus::Error;
        if let Some(ref mut time) = self.time {
            time.end = Some(Instant::now());
        }
    }

    /// Check if tool call is finished (completed or error)
    pub fn is_finished(&self) -> bool {
        matches!(
            self.status,
            ToolCallStatus::Completed | ToolCallStatus::Error
        )
    }
}

// ============================================================================
// Content Parts
// ============================================================================

/// Timing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeInfo {
    #[serde(
        with = "instant_option",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub start: Option<Instant>,
    #[serde(
        with = "instant_option",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub end: Option<Instant>,
}

// Custom serialization for Instant (store as Duration from epoch)
mod instant_option {
    use std::time::Instant;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(instant: &Option<Instant>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        instant.map(|_| 0u64).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Instant>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt: Option<u64> = Option::deserialize(deserializer)?;
        Ok(opt.map(|_| Instant::now()))
    }
}

/// Text content part
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextPart {
    /// Unique identifier
    pub id: String,
    /// Text content
    pub text: String,
    /// Timing information
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<TimeInfo>,
    /// Metadata from provider
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl TextPart {
    pub fn new(id: String) -> Self {
        Self {
            id,
            text: String::new(),
            time: Some(TimeInfo {
                start: Some(Instant::now()),
                end: None,
            }),
            metadata: None,
        }
    }

    pub fn append(&mut self, text: &str) {
        self.text.push_str(text);
    }

    pub fn finalize(&mut self) {
        self.text = self.text.trim_end().to_string();
        if let Some(ref mut time) = self.time {
            time.end = Some(Instant::now());
        }
    }
}

/// Reasoning content part
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningPart {
    /// Unique identifier
    pub id: String,
    /// Reasoning text content
    pub text: String,
    /// Timing information
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<TimeInfo>,
    /// Metadata from provider
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl ReasoningPart {
    pub fn new(id: String) -> Self {
        Self {
            id,
            text: String::new(),
            time: Some(TimeInfo {
                start: Some(Instant::now()),
                end: None,
            }),
            metadata: None,
        }
    }

    pub fn append(&mut self, text: &str) {
        self.text.push_str(text);
    }

    pub fn finalize(&mut self) {
        self.text = self.text.trim_end().to_string();
        if let Some(ref mut time) = self.time {
            time.end = Some(Instant::now());
        }
    }
}

/// Usage statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write_tokens: Option<u32>,
}

// ============================================================================
// Tool Call History Entry (for doom loop detection)
// ============================================================================

#[derive(Debug, Clone)]
struct ToolCallHistoryEntry {
    tool_name: String,
    input: serde_json::Value,
}

// ============================================================================
// Main Processor
// ============================================================================

/// Main Processor struct for handling streaming responses
pub struct Processor {
    /// Processor configuration
    config: ProcessorConfig,
    /// Active tool calls by ID
    tool_calls: HashMap<String, ToolCallState>,
    /// Active reasoning parts by stream ID
    reasoning_parts: HashMap<String, ReasoningPart>,
    /// Completed text parts
    text_parts: Vec<TextPart>,
    /// Current text part being built
    current_text: Option<TextPart>,
    /// Retry attempt counter
    attempt: u32,
    /// Context overflow detected
    needs_compact: bool,
    /// Permission blocked (for stop detection)
    blocked: bool,
    /// Last finish reason
    finish_reason: Option<FinishReason>,
    /// Tool call history for doom loop detection
    tool_call_history: Vec<ToolCallHistoryEntry>,
    /// Accumulated usage
    total_usage: Usage,
}

impl Processor {
    /// Create a new processor with default configuration
    pub fn new() -> Self {
        Self::with_config(ProcessorConfig::default())
    }

    /// Create a new processor with custom configuration
    pub fn with_config(config: ProcessorConfig) -> Self {
        Self {
            config,
            tool_calls: HashMap::new(),
            reasoning_parts: HashMap::new(),
            text_parts: Vec::new(),
            current_text: None,
            attempt: 0,
            needs_compact: false,
            blocked: false,
            finish_reason: None,
            tool_call_history: Vec::new(),
            total_usage: Usage::default(),
        }
    }

    /// Get current attempt count
    pub fn attempt(&self) -> u32 {
        self.attempt
    }

    /// Check if compaction is needed
    pub fn needs_compact(&self) -> bool {
        self.needs_compact
    }

    /// Check if processing was blocked by permission
    pub fn is_blocked(&self) -> bool {
        self.blocked
    }

    /// Get finish reason
    pub fn finish_reason(&self) -> Option<FinishReason> {
        self.finish_reason
    }

    /// Get total usage
    pub fn usage(&self) -> &Usage {
        &self.total_usage
    }

    /// Process a stream event
    pub fn process_event(&mut self, event: StreamEvent) -> ProcessResult {
        match event {
            StreamEvent::Start => {
                debug!("Stream started");
                ProcessResult::Continue
            }

            StreamEvent::TextStart { id, metadata } => {
                let mut part = TextPart::new(id);
                part.metadata = metadata;
                self.current_text = Some(part);
                ProcessResult::Continue
            }

            StreamEvent::TextDelta { id, text, .. } => {
                if let Some(ref mut current) = self.current_text {
                    if current.id == id {
                        current.append(&text);
                    }
                }
                ProcessResult::Continue
            }

            StreamEvent::TextEnd { id, metadata } => {
                if let Some(mut current) = self.current_text.take() {
                    if current.id == id {
                        current.finalize();
                        if let Some(meta) = metadata {
                            current.metadata = Some(meta);
                        }
                        self.text_parts.push(current);
                    }
                }
                ProcessResult::Continue
            }

            StreamEvent::ReasoningStart { id, metadata } => {
                if self.config.track_reasoning && !self.reasoning_parts.contains_key(&id) {
                    let mut part = ReasoningPart::new(id.clone());
                    part.metadata = metadata;
                    self.reasoning_parts.insert(id, part);
                }
                ProcessResult::Continue
            }

            StreamEvent::ReasoningDelta { id, text, metadata } => {
                if let Some(part) = self.reasoning_parts.get_mut(&id) {
                    part.append(&text);
                    if let Some(meta) = metadata {
                        part.metadata = Some(meta);
                    }
                }
                ProcessResult::Continue
            }

            StreamEvent::ReasoningEnd { id, metadata } => {
                if let Some(part) = self.reasoning_parts.get_mut(&id) {
                    part.finalize();
                    if let Some(meta) = metadata {
                        part.metadata = Some(meta);
                    }
                }
                ProcessResult::Continue
            }

            StreamEvent::ToolCallStart { id, tool_name } => {
                let state = ToolCallState::new(id.clone(), tool_name);
                self.tool_calls.insert(id, state);
                ProcessResult::Continue
            }

            StreamEvent::ToolCallDelta { id, delta } => {
                if let Some(state) = self.tool_calls.get_mut(&id) {
                    state.append_delta(&delta);
                }
                ProcessResult::Continue
            }

            StreamEvent::ToolCallEnd { id, input } => {
                if let Some(state) = self.tool_calls.get_mut(&id) {
                    state.input = input.clone();
                    state.status = ToolCallStatus::Running;
                    state.time = Some(TimeInfo {
                        start: Some(Instant::now()),
                        end: None,
                    });

                    // Extract values before checking doom loop to avoid borrow issues
                    let tool_name = state.tool_name.clone();
                    let input_clone = input.clone();

                    // Check for doom loop
                    if self.config.doom_loop_detection {
                        self.tool_call_history.push(ToolCallHistoryEntry {
                            tool_name: tool_name.clone(),
                            input: input_clone.clone(),
                        });

                        if let Some(result) = self.check_doom_loop(&tool_name, &input_clone) {
                            return result;
                        }
                    }
                }
                ProcessResult::Continue
            }

            StreamEvent::ToolResult {
                id,
                output,
                title,
                attachments,
            } => {
                if let Some(state) = self.tool_calls.get_mut(&id) {
                    state.complete(output);
                    state.metadata = attachments.map(|a| serde_json::Value::Array(a));
                    // Note: title is stored but not used in current impl
                    let _ = title;
                }
                ProcessResult::Continue
            }

            StreamEvent::ToolError { id, error } => {
                if let Some(state) = self.tool_calls.get_mut(&id) {
                    state.fail(error.clone());

                    // Check for permission denied
                    if error.contains("permission") || error.contains("denied") {
                        self.blocked = true;
                    }
                }
                ProcessResult::Continue
            }

            StreamEvent::Error { error } => self.handle_error(&error),

            StreamEvent::Finish { reason, usage } => {
                if let Some(usage) = usage {
                    self.total_usage.prompt_tokens += usage.prompt_tokens;
                    self.total_usage.completion_tokens += usage.completion_tokens;
                    self.total_usage.total_tokens += usage.total_tokens;
                }

                if let Some(reason_str) = reason {
                    self.finish_reason = Some(FinishReason::from_str(&reason_str));
                }

                ProcessResult::Continue
            }
        }
    }

    /// Handle an error and determine if retry is needed
    fn handle_error(&mut self, error: &ProcessorError) -> ProcessResult {
        // Context overflow - need compaction
        if error.is_context_overflow() {
            warn!("Context overflow detected, need compaction");
            self.needs_compact = true;
            return ProcessResult::Compact;
        }

        // Check if retryable
        if error.is_retryable() && self.attempt < self.config.retry.max_attempts {
            self.attempt += 1;
            let delay = self.calculate_retry_delay(self.attempt);
            debug!(
                "Retryable error, attempt {}/{}, delay {}ms",
                self.attempt, self.config.retry.max_attempts, delay
            );
            return ProcessResult::Retry {
                attempt: self.attempt,
                delay_ms: delay,
            };
        }

        // Non-retryable error - stop
        warn!("Non-retryable error: {:?}", error);
        self.finish_reason = Some(FinishReason::Error);
        ProcessResult::Stop {
            reason: FinishReason::Error,
        }
    }

    /// Check for doom loop (same tool called with same input N times)
    fn check_doom_loop(
        &mut self,
        tool_name: &str,
        input: &serde_json::Value,
    ) -> Option<ProcessResult> {
        let history_size = self.config.doom_loop_history;

        if self.tool_call_history.len() >= history_size {
            let recent: Vec<_> = self
                .tool_call_history
                .iter()
                .rev()
                .take(history_size)
                .collect();

            // Check if all recent calls are the same tool with same input
            let all_same = recent
                .iter()
                .all(|entry| entry.tool_name == tool_name && entry.input == *input);

            if all_same {
                warn!(
                    "Doom loop detected: tool '{}' called {} times with identical input",
                    tool_name, history_size
                );
                return Some(ProcessResult::DoomLoopDetected {
                    tool_name: tool_name.to_string(),
                });
            }
        }

        None
    }

    /// Calculate retry delay with exponential backoff
    pub fn calculate_retry_delay(&self, attempt: u32) -> u64 {
        let config = &self.config.retry;

        // Base exponential backoff
        let base_delay = config.base_delay_ms as f64
            * config
                .exponential_base
                .powi(attempt.saturating_sub(1) as i32);

        // Cap at max delay
        let delay = base_delay.min(config.max_delay_ms as f64) as u64;

        // Add jitter (±25%)
        if config.jitter {
            let jitter = (rand_jitter() * 0.5 - 0.25) * delay as f64;
            (delay as f64 + jitter).max(0.0) as u64
        } else {
            delay
        }
    }

    /// Check if an error is retryable
    pub fn is_error_retryable(error: &ProcessorError) -> bool {
        error.is_retryable()
    }

    /// Get accumulated tool calls
    pub fn get_tool_calls(&self) -> Vec<&ToolCallState> {
        self.tool_calls.values().collect()
    }

    /// Get tool call by ID
    pub fn get_tool_call(&self, id: &str) -> Option<&ToolCallState> {
        self.tool_calls.get(id)
    }

    /// Get mutable tool call by ID
    pub fn get_tool_call_mut(&mut self, id: &str) -> Option<&mut ToolCallState> {
        self.tool_calls.get_mut(id)
    }

    /// Get accumulated text content
    pub fn get_text_content(&self) -> String {
        self.text_parts
            .iter()
            .map(|p| p.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get reasoning content
    pub fn get_reasoning(&self) -> String {
        self.reasoning_parts
            .values()
            .map(|p| p.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get text parts
    pub fn get_text_parts(&self) -> &[TextPart] {
        &self.text_parts
    }

    /// Get reasoning parts
    pub fn get_reasoning_parts(&self) -> impl Iterator<Item = &ReasoningPart> {
        self.reasoning_parts.values()
    }

    /// Reset processor state for new stream
    pub fn reset(&mut self) {
        self.tool_calls.clear();
        self.reasoning_parts.clear();
        self.text_parts.clear();
        self.current_text = None;
        self.attempt = 0;
        self.needs_compact = false;
        self.blocked = false;
        self.finish_reason = None;
        self.tool_call_history.clear();
        // Keep total_usage for aggregation across streams
    }

    /// Reset everything including usage stats
    pub fn full_reset(&mut self) {
        self.reset();
        self.total_usage = Usage::default();
    }

    /// Determine final process result after stream ends
    pub fn finalize(&mut self) -> ProcessResult {
        // Clean up any incomplete tool calls
        for state in self.tool_calls.values_mut() {
            if !state.is_finished() {
                state.fail("Stream ended before tool completion".to_string());
            }
        }

        if self.needs_compact {
            return ProcessResult::Compact;
        }

        if self.blocked {
            return ProcessResult::Stop {
                reason: FinishReason::Stop,
            };
        }

        ProcessResult::Continue
    }
}

impl Default for Processor {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple random jitter (0.0 - 1.0)
fn rand_jitter() -> f64 {
    // Use a simple deterministic "random" based on current time
    // In production, you'd use a proper random number generator
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    now.subsec_nanos() as f64 / 1_000_000_000.0
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // ProcessorError Tests
    // =========================================================================

    #[test]
    fn test_error_is_retryable() {
        // Retryable errors
        assert!(
            ProcessorError::RateLimit {
                retry_after_ms: None
            }
            .is_retryable()
        );
        assert!(
            ProcessorError::Network {
                message: "test".into()
            }
            .is_retryable()
        );
        assert!(ProcessorError::Timeout.is_retryable());
        assert!(
            ProcessorError::Api {
                message: "test".into(),
                code: Some("overloaded".into()),
            }
            .is_retryable()
        );
        assert!(
            ProcessorError::Api {
                message: "test".into(),
                code: Some("529".into()),
            }
            .is_retryable()
        );

        // Non-retryable errors
        assert!(!ProcessorError::Authentication.is_retryable());
        assert!(
            !ProcessorError::ContextOverflow {
                message: "test".into()
            }
            .is_retryable()
        );
        assert!(
            !ProcessorError::Api {
                message: "test".into(),
                code: Some("invalid_request".into()),
            }
            .is_retryable()
        );
        assert!(
            !ProcessorError::Api {
                message: "test".into(),
                code: None,
            }
            .is_retryable()
        );
    }

    #[test]
    fn test_error_is_context_overflow() {
        assert!(
            ProcessorError::ContextOverflow {
                message: "test".into()
            }
            .is_context_overflow()
        );
        assert!(
            !ProcessorError::RateLimit {
                retry_after_ms: None
            }
            .is_context_overflow()
        );
    }

    #[test]
    fn test_error_from_string() {
        let err = ProcessorError::from_error_string("Rate limit exceeded");
        assert!(err.is_rate_limit());

        let err = ProcessorError::from_error_string("Unauthorized access");
        assert!(err.is_authentication());

        let err = ProcessorError::from_error_string("Context length exceeded");
        assert!(err.is_context_overflow());

        let err = ProcessorError::from_error_string("Request timeout");
        assert!(matches!(err, ProcessorError::Timeout));

        let err = ProcessorError::from_error_string("Connection failed");
        assert!(matches!(err, ProcessorError::Network { .. }));
    }

    // =========================================================================
    // RetryConfig Tests
    // =========================================================================

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, DEFAULT_MAX_RETRY_ATTEMPTS);
        assert_eq!(config.base_delay_ms, DEFAULT_BASE_DELAY_MS);
        assert_eq!(config.max_delay_ms, DEFAULT_MAX_DELAY_MS);
        assert!(config.jitter);
    }

    #[test]
    fn test_retry_config_fast() {
        let config = RetryConfig::fast();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.base_delay_ms, 500);
        assert_eq!(config.max_delay_ms, 10_000);
    }

    #[test]
    fn test_retry_config_aggressive() {
        let config = RetryConfig::aggressive();
        assert_eq!(config.max_attempts, 10);
        assert_eq!(config.max_delay_ms, 120_000);
    }

    // =========================================================================
    // Processor Tests
    // =========================================================================

    #[test]
    fn test_processor_new() {
        let processor = Processor::new();
        assert_eq!(processor.attempt(), 0);
        assert!(!processor.needs_compact());
        assert!(!processor.is_blocked());
        assert!(processor.finish_reason().is_none());
    }

    #[test]
    fn test_processor_default() {
        let processor = Processor::default();
        assert_eq!(processor.attempt(), 0);
    }

    #[test]
    fn test_process_text_events() {
        let mut processor = Processor::new();

        // Process text start
        let result = processor.process_event(StreamEvent::TextStart {
            id: "text-1".into(),
            metadata: None,
        });
        assert!(matches!(result, ProcessResult::Continue));

        // Process text delta
        let result = processor.process_event(StreamEvent::TextDelta {
            id: "text-1".into(),
            text: "Hello ".into(),
            metadata: None,
        });
        assert!(matches!(result, ProcessResult::Continue));

        let result = processor.process_event(StreamEvent::TextDelta {
            id: "text-1".into(),
            text: "World".into(),
            metadata: None,
        });
        assert!(matches!(result, ProcessResult::Continue));

        // Process text end
        let result = processor.process_event(StreamEvent::TextEnd {
            id: "text-1".into(),
            metadata: None,
        });
        assert!(matches!(result, ProcessResult::Continue));

        // Check accumulated text
        assert_eq!(processor.get_text_content(), "Hello World");
    }

    #[test]
    fn test_process_reasoning_events() {
        let mut processor = Processor::new();

        // Process reasoning start
        let result = processor.process_event(StreamEvent::ReasoningStart {
            id: "reason-1".into(),
            metadata: None,
        });
        assert!(matches!(result, ProcessResult::Continue));

        // Process reasoning delta
        let result = processor.process_event(StreamEvent::ReasoningDelta {
            id: "reason-1".into(),
            text: "Thinking... ".into(),
            metadata: None,
        });
        assert!(matches!(result, ProcessResult::Continue));

        let result = processor.process_event(StreamEvent::ReasoningDelta {
            id: "reason-1".into(),
            text: "more thoughts".into(),
            metadata: None,
        });
        assert!(matches!(result, ProcessResult::Continue));

        // Process reasoning end
        let result = processor.process_event(StreamEvent::ReasoningEnd {
            id: "reason-1".into(),
            metadata: None,
        });
        assert!(matches!(result, ProcessResult::Continue));

        // Check accumulated reasoning
        assert_eq!(processor.get_reasoning(), "Thinking... more thoughts");
    }

    #[test]
    fn test_process_tool_call_events() {
        let mut processor = Processor::new();

        // Process tool call start
        let result = processor.process_event(StreamEvent::ToolCallStart {
            id: "call-1".into(),
            tool_name: "read_file".into(),
        });
        assert!(matches!(result, ProcessResult::Continue));

        // Process tool call delta
        let result = processor.process_event(StreamEvent::ToolCallDelta {
            id: "call-1".into(),
            delta: r#"{"path": ""#.into(),
        });
        assert!(matches!(result, ProcessResult::Continue));

        let result = processor.process_event(StreamEvent::ToolCallDelta {
            id: "call-1".into(),
            delta: r#"test.txt"}"#.into(),
        });
        assert!(matches!(result, ProcessResult::Continue));

        // Process tool call end
        let result = processor.process_event(StreamEvent::ToolCallEnd {
            id: "call-1".into(),
            input: serde_json::json!({"path": "test.txt"}),
        });
        assert!(matches!(result, ProcessResult::Continue));

        // Check tool call state
        let tool_calls = processor.get_tool_calls();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].tool_name, "read_file");
        assert_eq!(tool_calls[0].status, ToolCallStatus::Running);
    }

    #[test]
    fn test_tool_call_result() {
        let mut processor = Processor::new();

        // Setup tool call
        processor.process_event(StreamEvent::ToolCallStart {
            id: "call-1".into(),
            tool_name: "read_file".into(),
        });
        processor.process_event(StreamEvent::ToolCallEnd {
            id: "call-1".into(),
            input: serde_json::json!({"path": "test.txt"}),
        });

        // Process tool result
        let result = processor.process_event(StreamEvent::ToolResult {
            id: "call-1".into(),
            output: "file contents".into(),
            title: None,
            attachments: None,
        });
        assert!(matches!(result, ProcessResult::Continue));

        // Check tool call completed
        let state = processor.get_tool_call("call-1").unwrap();
        assert_eq!(state.status, ToolCallStatus::Completed);
        assert_eq!(state.output, Some("file contents".into()));
    }

    #[test]
    fn test_tool_call_error() {
        let mut processor = Processor::new();

        // Setup tool call
        processor.process_event(StreamEvent::ToolCallStart {
            id: "call-1".into(),
            tool_name: "read_file".into(),
        });
        processor.process_event(StreamEvent::ToolCallEnd {
            id: "call-1".into(),
            input: serde_json::json!({}),
        });

        // Process tool error
        let result = processor.process_event(StreamEvent::ToolError {
            id: "call-1".into(),
            error: "File not found".into(),
        });
        assert!(matches!(result, ProcessResult::Continue));

        // Check tool call error
        let state = processor.get_tool_call("call-1").unwrap();
        assert_eq!(state.status, ToolCallStatus::Error);
        assert_eq!(state.error, Some("File not found".into()));
    }

    #[test]
    fn test_error_retry() {
        let mut processor = Processor::new();

        // Process rate limit error
        let result = processor.process_event(StreamEvent::Error {
            error: ProcessorError::RateLimit {
                retry_after_ms: None,
            },
        });

        assert!(matches!(result, ProcessResult::Retry { attempt: 1, .. }));
        assert_eq!(processor.attempt(), 1);

        // Second retry
        let result = processor.process_event(StreamEvent::Error {
            error: ProcessorError::RateLimit {
                retry_after_ms: None,
            },
        });
        assert!(matches!(result, ProcessResult::Retry { attempt: 2, .. }));
    }

    #[test]
    fn test_error_max_retries() {
        let config = ProcessorConfig {
            retry: RetryConfig::new(2, 100, 1000),
            ..Default::default()
        };
        let mut processor = Processor::with_config(config);

        // First retry
        processor.process_event(StreamEvent::Error {
            error: ProcessorError::RateLimit {
                retry_after_ms: None,
            },
        });

        // Second retry
        processor.process_event(StreamEvent::Error {
            error: ProcessorError::RateLimit {
                retry_after_ms: None,
            },
        });

        // Should stop (max attempts reached)
        let result = processor.process_event(StreamEvent::Error {
            error: ProcessorError::RateLimit {
                retry_after_ms: None,
            },
        });
        assert!(matches!(result, ProcessResult::Stop { .. }));
    }

    #[test]
    fn test_context_overflow() {
        let mut processor = Processor::new();

        let result = processor.process_event(StreamEvent::Error {
            error: ProcessorError::ContextOverflow {
                message: "Context too long".into(),
            },
        });

        assert!(matches!(result, ProcessResult::Compact));
        assert!(processor.needs_compact());
    }

    #[test]
    fn test_non_retryable_error() {
        let mut processor = Processor::new();

        let result = processor.process_event(StreamEvent::Error {
            error: ProcessorError::Authentication,
        });

        assert!(matches!(
            result,
            ProcessResult::Stop {
                reason: FinishReason::Error
            }
        ));
    }

    #[test]
    fn test_retry_delay_calculation() {
        let processor = Processor::new();

        // First attempt
        let d1 = processor.calculate_retry_delay(1);
        assert!(d1 >= 750 && d1 <= 1250); // ~1000ms with jitter

        // Second attempt
        let d2 = processor.calculate_retry_delay(2);
        assert!(d2 >= 1500 && d2 <= 2500); // ~2000ms with jitter

        // Third attempt
        let d3 = processor.calculate_retry_delay(3);
        assert!(d3 >= 3000 && d3 <= 5000); // ~4000ms with jitter
    }

    #[test]
    fn test_retry_delay_max_cap() {
        let config = ProcessorConfig {
            retry: RetryConfig::new(10, 1000, 5000),
            ..Default::default()
        };
        let processor = Processor::with_config(config);

        // Even at high attempts, should be capped
        let delay = processor.calculate_retry_delay(10);
        assert!(delay <= 6500); // max 5000 + 25% jitter
    }

    #[test]
    fn test_doom_loop_detection() {
        let config = ProcessorConfig {
            doom_loop_detection: true,
            doom_loop_history: 3,
            ..Default::default()
        };
        let mut processor = Processor::with_config(config);

        let input = serde_json::json!({"path": "test.txt"});

        // First call - no doom loop
        processor.process_event(StreamEvent::ToolCallStart {
            id: "call-1".into(),
            tool_name: "read_file".into(),
        });
        let result = processor.process_event(StreamEvent::ToolCallEnd {
            id: "call-1".into(),
            input: input.clone(),
        });
        assert!(matches!(result, ProcessResult::Continue));

        // Second call - no doom loop yet
        processor.process_event(StreamEvent::ToolCallStart {
            id: "call-2".into(),
            tool_name: "read_file".into(),
        });
        let result = processor.process_event(StreamEvent::ToolCallEnd {
            id: "call-2".into(),
            input: input.clone(),
        });
        assert!(matches!(result, ProcessResult::Continue));

        // Third call - doom loop!
        processor.process_event(StreamEvent::ToolCallStart {
            id: "call-3".into(),
            tool_name: "read_file".into(),
        });
        let result = processor.process_event(StreamEvent::ToolCallEnd {
            id: "call-3".into(),
            input: input.clone(),
        });
        assert!(matches!(result, ProcessResult::DoomLoopDetected { .. }));
    }

    #[test]
    fn test_doom_loop_different_inputs() {
        let mut processor = Processor::new();

        // Different inputs should not trigger doom loop
        for i in 0..5 {
            processor.process_event(StreamEvent::ToolCallStart {
                id: format!("call-{}", i),
                tool_name: "read_file".into(),
            });
            let result = processor.process_event(StreamEvent::ToolCallEnd {
                id: format!("call-{}", i),
                input: serde_json::json!({"path": format!("file-{}.txt", i)}),
            });
            assert!(matches!(result, ProcessResult::Continue));
        }
    }

    #[test]
    fn test_processor_reset() {
        let mut processor = Processor::new();

        // Add some state
        processor.process_event(StreamEvent::TextStart {
            id: "text-1".into(),
            metadata: None,
        });
        processor.process_event(StreamEvent::TextDelta {
            id: "text-1".into(),
            text: "Hello".into(),
            metadata: None,
        });
        processor.process_event(StreamEvent::TextEnd {
            id: "text-1".into(),
            metadata: None,
        });
        processor.process_event(StreamEvent::Error {
            error: ProcessorError::RateLimit {
                retry_after_ms: None,
            },
        });

        assert!(!processor.get_text_content().is_empty());
        assert_eq!(processor.attempt(), 1);

        // Reset
        processor.reset();

        assert!(processor.get_text_content().is_empty());
        assert_eq!(processor.attempt(), 0);
        assert!(!processor.needs_compact());
        assert!(!processor.is_blocked());
    }

    #[test]
    fn test_processor_full_reset() {
        let mut processor = Processor::new();

        // Add usage
        processor.process_event(StreamEvent::Finish {
            reason: Some("stop".into()),
            usage: Some(Usage {
                prompt_tokens: 100,
                completion_tokens: 50,
                total_tokens: 150,
                ..Default::default()
            }),
        });

        assert_eq!(processor.usage().total_tokens, 150);

        // Full reset
        processor.full_reset();

        assert_eq!(processor.usage().total_tokens, 0);
    }

    #[test]
    fn test_finish_event() {
        let mut processor = Processor::new();

        let result = processor.process_event(StreamEvent::Finish {
            reason: Some("stop".into()),
            usage: Some(Usage {
                prompt_tokens: 100,
                completion_tokens: 50,
                total_tokens: 150,
                ..Default::default()
            }),
        });

        assert!(matches!(result, ProcessResult::Continue));
        assert_eq!(processor.finish_reason(), Some(FinishReason::Stop));
        assert_eq!(processor.usage().total_tokens, 150);
    }

    #[test]
    fn test_finalize_incomplete_tools() {
        let mut processor = Processor::new();

        // Start a tool call but don't finish it
        processor.process_event(StreamEvent::ToolCallStart {
            id: "call-1".into(),
            tool_name: "read_file".into(),
        });
        processor.process_event(StreamEvent::ToolCallEnd {
            id: "call-1".into(),
            input: serde_json::json!({}),
        });

        // Finalize should mark incomplete tools as error
        let result = processor.finalize();
        assert!(matches!(result, ProcessResult::Continue));

        let state = processor.get_tool_call("call-1").unwrap();
        assert_eq!(state.status, ToolCallStatus::Error);
    }

    #[test]
    fn test_finalize_with_compact() {
        let mut processor = Processor::new();

        // Trigger context overflow
        processor.process_event(StreamEvent::Error {
            error: ProcessorError::ContextOverflow {
                message: "test".into(),
            },
        });

        let result = processor.finalize();
        assert!(matches!(result, ProcessResult::Compact));
    }

    #[test]
    fn test_permission_blocked() {
        let mut processor = Processor::new();

        // Setup tool call
        processor.process_event(StreamEvent::ToolCallStart {
            id: "call-1".into(),
            tool_name: "dangerous_tool".into(),
        });
        processor.process_event(StreamEvent::ToolCallEnd {
            id: "call-1".into(),
            input: serde_json::json!({}),
        });

        // Permission denied
        processor.process_event(StreamEvent::ToolError {
            id: "call-1".into(),
            error: "Permission denied for this operation".into(),
        });

        assert!(processor.is_blocked());

        let result = processor.finalize();
        assert!(matches!(result, ProcessResult::Stop { .. }));
    }

    // =========================================================================
    // ToolCallState Tests
    // =========================================================================

    #[test]
    fn test_tool_call_state_new() {
        let state = ToolCallState::new("call-1".into(), "read_file".into());
        assert_eq!(state.id, "call-1");
        assert_eq!(state.tool_name, "read_file");
        assert_eq!(state.status, ToolCallStatus::Pending);
        assert!(state.raw_input.is_empty());
    }

    #[test]
    fn test_tool_call_state_append_delta() {
        let mut state = ToolCallState::new("call-1".into(), "read_file".into());
        state.append_delta(r#"{"path":"#);
        state.append_delta(r#""test.txt"}"#);
        assert_eq!(state.raw_input, r#"{"path":"test.txt"}"#);
    }

    #[test]
    fn test_tool_call_state_complete() {
        let mut state = ToolCallState::new("call-1".into(), "read_file".into());
        state.set_running(serde_json::json!({}));
        state.complete("file contents".into());

        assert_eq!(state.status, ToolCallStatus::Completed);
        assert_eq!(state.output, Some("file contents".into()));
        assert!(state.time.as_ref().unwrap().end.is_some());
    }

    #[test]
    fn test_tool_call_state_fail() {
        let mut state = ToolCallState::new("call-1".into(), "read_file".into());
        state.set_running(serde_json::json!({}));
        state.fail("File not found".into());

        assert_eq!(state.status, ToolCallStatus::Error);
        assert_eq!(state.error, Some("File not found".into()));
        assert!(state.time.as_ref().unwrap().end.is_some());
    }

    // =========================================================================
    // TextPart Tests
    // =========================================================================

    #[test]
    fn test_text_part() {
        let mut part = TextPart::new("text-1".into());
        part.append("Hello ");
        part.append("World");
        part.finalize();

        assert_eq!(part.text, "Hello World");
        assert!(part.time.as_ref().unwrap().end.is_some());
    }

    #[test]
    fn test_text_part_trims_end() {
        let mut part = TextPart::new("text-1".into());
        part.append("Hello World   ");
        part.finalize();

        assert_eq!(part.text, "Hello World");
    }

    // =========================================================================
    // ReasoningPart Tests
    // =========================================================================

    #[test]
    fn test_reasoning_part() {
        let mut part = ReasoningPart::new("reason-1".into());
        part.append("Let me think... ");
        part.append("I need to analyze this.");
        part.finalize();

        assert_eq!(part.text, "Let me think... I need to analyze this.");
    }

    // =========================================================================
    // ProcessResult Tests
    // =========================================================================

    #[test]
    fn test_process_result_variants() {
        // Test all variants can be created
        let _ = ProcessResult::Continue;
        let _ = ProcessResult::Stop {
            reason: FinishReason::Stop,
        };
        let _ = ProcessResult::Compact;
        let _ = ProcessResult::Retry {
            attempt: 1,
            delay_ms: 1000,
        };
        let _ = ProcessResult::DoomLoopDetected {
            tool_name: "test".into(),
        };
    }

    // =========================================================================
    // StreamEvent Serialization Tests
    // =========================================================================

    #[test]
    fn test_stream_event_serialization() {
        let event = StreamEvent::TextStart {
            id: "text-1".into(),
            metadata: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"text_start\""));
        assert!(json.contains("\"id\":\"text-1\""));
    }

    #[test]
    fn test_stream_event_deserialization() {
        let json = r#"{"type":"text_delta","id":"text-1","text":"Hello"}"#;
        let event: StreamEvent = serde_json::from_str(json).unwrap();
        match event {
            StreamEvent::TextDelta { id, text, .. } => {
                assert_eq!(id, "text-1");
                assert_eq!(text, "Hello");
            }
            _ => panic!("Expected TextDelta"),
        }
    }

    #[test]
    fn test_processor_error_serialization() {
        let error = ProcessorError::RateLimit {
            retry_after_ms: Some(5000),
        };
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("\"type\":\"rate_limit\""));
        assert!(json.contains("\"retry_after_ms\":5000"));
    }
}
