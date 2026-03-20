//! Context overflow handling and auto-compaction system
//!
//! This module provides functionality for managing context window overflow
//! by detecting when token usage exceeds limits, pruning old tool outputs,
//! and generating compaction summaries.
//!
//! # Constants
//!
//! - `COMPACTION_BUFFER`: Reserved tokens for compaction operations (20,000)
//! - `PRUNE_MINIMUM`: Minimum tokens to prune (20,000)
//! - `PRUNE_PROTECT`: Protected tokens before pruning starts (40,000)
//!
//! # Functions
//!
//! - [`is_overflow`]: Check if token usage exceeds context limits
//! - [`prune`]: Remove old tool outputs to save tokens
//! - [`process`]: Handle the compaction process
//! - [`create`]: Create a compaction message

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::session::parts::{CompactionPart, Part, PartId, SessionId, ToolState};
use crate::session::store::SessionStore;
use crate::session::message::{MessageInfo, WithParts};

/// Buffer tokens reserved for compaction operations
pub const COMPACTION_BUFFER: i64 = 20_000;

/// Minimum tokens to prune before taking action
pub const PRUNE_MINIMUM: i64 = 20_000;

/// Protected tokens that won't be pruned (most recent tool calls)
pub const PRUNE_PROTECT: i64 = 40_000;

/// Tools that are protected from pruning
pub const PRUNE_PROTECTED_TOOLS: &[&str] = &["skill"];

// ============================================================================
// Event Types
// ============================================================================

/// Compaction event for bus notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionEvent {
    /// Session that was compacted
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,
}

/// Event types emitted by the compaction system
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Event {
    /// Emitted when a session is compacted
    Compacted(CompactionEvent),
}

// ============================================================================
// Config Types
// ============================================================================

/// Compaction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionConfig {
    /// Whether auto-compaction is enabled (default: true)
    #[serde(default = "default_auto")]
    pub auto: bool,

    /// Whether pruning is enabled (default: true)
    #[serde(default = "default_prune")]
    pub prune: bool,

    /// Reserved tokens for compaction buffer (default: COMPACTION_BUFFER)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserved: Option<i64>,
}

fn default_auto() -> bool {
    true
}

fn default_prune() -> bool {
    true
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            auto: default_auto(),
            prune: default_prune(),
            reserved: None,
        }
    }
}

// ============================================================================
// Model Limits Types
// ============================================================================

/// Model limits for overflow detection
#[derive(Debug, Clone)]
pub struct ModelLimits {
    /// Total context window size
    pub context: i64,

    /// Maximum input tokens (if different from context - max_output)
    pub input: Option<i64>,

    /// Maximum output tokens
    pub max_output: i64,
}

impl ModelLimits {
    /// Create new model limits
    pub fn new(context: i64, max_output: i64) -> Self {
        Self {
            context,
            input: None,
            max_output,
        }
    }

    /// Create with explicit input limit
    pub fn with_input(context: i64, input: i64, max_output: i64) -> Self {
        Self {
            context,
            input: Some(input),
            max_output,
        }
    }
}

// ============================================================================
// Token Info for Overflow Check
// ============================================================================

/// Token usage information for overflow detection
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    /// Total tokens used
    pub total: Option<i64>,

    /// Input tokens
    pub input: i64,

    /// Output tokens
    pub output: i64,

    /// Cache read tokens
    pub cache_read: i64,

    /// Cache write tokens
    pub cache_write: i64,
}

impl TokenUsage {
    /// Create new token usage
    pub fn new(input: i64, output: i64) -> Self {
        Self {
            total: None,
            input,
            output,
            cache_read: 0,
            cache_write: 0,
        }
    }

    /// Get total token count
    pub fn count(&self) -> i64 {
        self.total.unwrap_or_else(|| {
            self.input + self.output + self.cache_read + self.cache_write
        })
    }
}

// ============================================================================
// Overflow Detection
// ============================================================================

/// Check if token usage exceeds context limits
///
/// # Arguments
/// * `tokens` - Token usage information
/// * `model` - Model limits
/// * `config` - Compaction configuration
///
/// # Returns
/// * `true` if overflow detected, `false` otherwise
pub fn is_overflow(tokens: &TokenUsage, model: &ModelLimits, config: Option<&CompactionConfig>) -> bool {
    // Check if auto-compaction is disabled
    if let Some(cfg) = config {
        if !cfg.auto {
            return false;
        }
    }

    // Check if context is unlimited (0 means unlimited)
    if model.context == 0 {
        return false;
    }

    let count = tokens.count();

    // Calculate reserved tokens
    let reserved = config
        .and_then(|c| c.reserved)
        .unwrap_or_else(|| COMPACTION_BUFFER.min(model.max_output));

    // Calculate usable tokens
    let usable = if let Some(input_limit) = model.input {
        input_limit - reserved
    } else {
        model.context - model.max_output
    };

    count >= usable
}

// ============================================================================
// Pruning
// ============================================================================

/// Result of pruning operation
#[derive(Debug, Clone)]
pub struct PruneResult {
    /// Number of parts pruned
    pub parts_pruned: usize,

    /// Total tokens saved
    pub tokens_saved: i64,

    /// Total tokens examined
    pub tokens_examined: i64,
}

/// Estimate token count for a string
///
/// Simple estimation: ~4 characters per token on average
fn estimate_tokens(text: &str) -> i64 {
    (text.len() as i64 / 4).max(1)
}

/// Prune old tool outputs from messages
///
/// Goes backwards through parts until there are `PRUNE_PROTECT` tokens worth of
/// tool calls, then erases output of previous tool calls. The idea is to throw
/// away old tool calls that are no longer relevant.
///
/// # Arguments
/// * `store` - Session store for updating parts
/// * `session_id` - Session to prune
/// * `messages` - Messages to examine for pruning
/// * `config` - Compaction configuration
///
/// # Returns
/// * `Result<PruneResult, SessionError>` - Pruning result
pub async fn prune(
    store: &SessionStore,
    session_id: &SessionId,
    messages: &[WithParts],
    config: Option<&CompactionConfig>,
) -> crate::session::error::Result<PruneResult> {
    // Check if pruning is disabled
    if let Some(cfg) = config {
        if !cfg.prune {
            return Ok(PruneResult {
                parts_pruned: 0,
                tokens_saved: 0,
                tokens_examined: 0,
            });
        }
    }

    info!("Pruning session: {}", session_id);

    let mut total: i64 = 0;
    let mut pruned: i64 = 0;
    let mut to_prune: Vec<Part> = Vec::new();
    let mut turns = 0;

    // Iterate backwards through messages
    'outer: for msg in messages.iter().rev() {
        // Count turns (user messages)
        if matches!(&msg.info, MessageInfo::User(_)) {
            turns += 1;
        }

        // Skip the last 2 turns
        if turns < 2 {
            continue;
        }

        // Stop at messages that already have a summary
        if let MessageInfo::Assistant(assistant) = &msg.info {
            if assistant.summary.unwrap_or(false) {
                break 'outer;
            }
        }

        // Iterate backwards through parts
        for part in msg.parts.iter().rev() {
            if let Part::Tool(tool_part) = part {
                if let ToolState::Completed(state) = &tool_part.state {
                    // Skip protected tools
                    if PRUNE_PROTECTED_TOOLS.contains(&tool_part.tool.as_str()) {
                        continue;
                    }

                    // Skip already compacted tools
                    if state.time.compacted.is_some() {
                        break 'outer;
                    }

                    // Estimate tokens
                    let estimate = estimate_tokens(&state.output);
                    total += estimate;

                    // Start pruning after PRUNE_PROTECT tokens
                    if total > PRUNE_PROTECT {
                        pruned += estimate;
                        to_prune.push(part.clone());
                    }
                }
            }
        }
    }

    info!(
        "Pruning complete: {} tokens to prune, {} total examined",
        pruned, total
    );

    // Only prune if we have enough tokens to save
    if pruned > PRUNE_MINIMUM {
        let mut parts_pruned = 0;

        for mut part in to_prune {
            if let Part::Tool(tool_part) = &mut part {
                if let ToolState::Completed(ref mut state) = tool_part.state {
                    // Mark as compacted
                    state.time.compacted = Some(Utc::now().timestamp_millis());
                    store.update_part(&part).await?;
                    parts_pruned += 1;
                }
            }
        }

        info!("Pruned {} parts", parts_pruned);

        Ok(PruneResult {
            parts_pruned,
            tokens_saved: pruned,
            tokens_examined: total,
        })
    } else {
        debug!("Not enough tokens to prune ({} < {})", pruned, PRUNE_MINIMUM);
        Ok(PruneResult {
            parts_pruned: 0,
            tokens_saved: 0,
            tokens_examined: total,
        })
    }
}

// ============================================================================
// Compaction Processing
// ============================================================================

/// Result of compaction process
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessResult {
    /// Continue with conversation
    Continue,
    /// Stop (error occurred or session needs attention)
    Stop,
}

/// Input for compaction process
#[derive(Debug, Clone)]
pub struct ProcessInput {
    /// Parent message ID
    pub parent_id: String,

    /// Messages to compact
    pub messages: Vec<WithParts>,

    /// Session ID
    pub session_id: SessionId,

    /// Whether this is an auto-compaction
    pub auto: bool,

    /// Whether this is an overflow compaction
    pub overflow: Option<bool>,
}

/// Default compaction prompt
const DEFAULT_COMPACTION_PROMPT: &str = r#"Provide a detailed prompt for continuing our conversation above.
Focus on information that would be helpful for continuing the conversation, including what we did, what we're doing, which files we're working on, and what we're going to do next.
The summary that you construct will be used so that another agent can read it and continue the work.

When constructing the summary, try to stick to this template:
---
## Goal

[What goal(s) is the user trying to accomplish?]

## Instructions

- [What important instructions did the user give you that are relevant]
- [If there is a plan or spec, include information about it so next agent can continue using it]

## Discoveries

[What notable things were learned during this conversation that would be useful for the next agent to know when continuing the work]

## Accomplished

[What work has been completed, what work is still in progress, and what work is left?]

## Relevant files / directories

[Construct a structured list of relevant files that have been read, edited, or created that pertain to the task at hand. If all the files in a directory are relevant, include the path to the directory.]
---"#;

/// Handle compaction process
///
/// This function orchestrates the compaction process:
/// 1. Finds the replay point if overflow
/// 2. Creates a compaction assistant message
/// 3. Generates a summary
/// 4. Creates continue/replay messages
///
/// Note: This is a simplified version that handles the core logic.
/// Full implementation would integrate with the session processor.
pub fn process(input: &ProcessInput) -> ProcessResult {
    let ProcessInput {
        parent_id,
        messages,
        session_id,
        auto,
        overflow,
    } = input;

    debug!(
        "Processing compaction for session: {}, auto: {}, overflow: {:?}",
        session_id, auto, overflow
    );

    // Find the parent message
    let user_message = messages.iter().rev().find(|m| m.info.id() == *parent_id);

    if user_message.is_none() {
        warn!("Parent message not found: {}", parent_id);
        return ProcessResult::Stop;
    }

    // Find replay point if overflow
    let (replay_msg, truncated_messages): (Option<WithParts>, Vec<WithParts>) = if overflow.unwrap_or(false) {
        find_replay_point(messages, parent_id)
    } else {
        (None, messages.clone())
    };

    // Check if we have content to summarize
    let has_content: bool = truncated_messages.iter().any(|m| {
        matches!(&m.info, MessageInfo::User(_)) 
            && !m.parts.iter().any(|p| matches!(p, Part::Compaction(_)))
    });

    if replay_msg.is_some() && !has_content {
        debug!("No content to summarize after replay point");
        return ProcessResult::Stop;
    }

    // In a full implementation, we would:
    // 1. Create an assistant message with mode="compaction"
    // 2. Use the session processor to generate a summary
    // 3. Create replay or continue messages
    // 4. Publish Compacted event

    info!(
        "Compaction processed: {} messages to summarize",
        truncated_messages.len()
    );

    ProcessResult::Continue
}

/// Find the replay point for overflow compaction
fn find_replay_point(messages: &[WithParts], parent_id: &str) -> (Option<WithParts>, Vec<WithParts>) {
    // Find the index of the parent message
    let parent_idx = messages.iter().position(|m| m.info.id() == parent_id);

    if let Some(idx) = parent_idx {
        // Look for a user message before the parent
        for i in (0..idx).rev() {
            let msg = &messages[i];
            if matches!(&msg.info, MessageInfo::User(_)) {
                // Check if it has any non-compaction parts
                let has_non_compaction = msg.parts.iter().any(|p| !matches!(p, Part::Compaction(_)));
                if has_non_compaction {
                    // Return the replay message and truncated list
                    let truncated = messages[..=i].to_vec();
                    return (Some(msg.clone()), truncated);
                }
            }
        }
    }

    (None, messages.to_vec())
}

// ============================================================================
// Compaction Message Creation
// ============================================================================

/// Input for creating a compaction message
#[derive(Debug, Clone)]
pub struct CreateInput {
    /// Session ID
    pub session_id: SessionId,

    /// Agent name
    pub agent: String,

    /// Model reference
    pub model: ModelRef,

    /// Whether this is an auto-compaction
    pub auto: bool,

    /// Whether this is an overflow compaction
    pub overflow: Option<bool>,
}

/// Model reference
#[derive(Debug, Clone)]
pub struct ModelRef {
    /// Provider ID
    pub provider_id: String,

    /// Model ID
    pub model_id: String,
}

/// Create a compaction message part
///
/// Creates a user message with a compaction part attached.
/// This signals that a compaction should occur.
///
/// # Arguments
/// * `session_id` - Session to create the message in
/// * `agent` - Agent that triggered compaction
/// * `model` - Model reference for the message
/// * `auto` - Whether this is an auto-compaction
/// * `overflow` - Whether this is an overflow compaction
///
/// # Returns
/// * `(MessageId, PartId)` - IDs of the created message and part
pub fn create(
    session_id: SessionId,
    _agent: String,
    _model: ModelRef,
    auto: bool,
    overflow: Option<bool>,
) -> (String, PartId) {
    // Generate IDs
    let message_id = uuid::Uuid::new_v4().to_string();
    let part_id = uuid::Uuid::new_v4().to_string();

    debug!(
        "Creating compaction message: session={}, auto={}, overflow={:?}",
        session_id, auto, overflow
    );

    // In a full implementation, we would:
    // 1. Create a user message in the database
    // 2. Create a compaction part attached to it

    let _part = CompactionPart {
        id: part_id.clone(),
        session_id: session_id.clone(),
        message_id: message_id.clone(),
        auto,
        overflow,
    };

    (message_id, part_id)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_overflow_no_overflow() {
        let tokens = TokenUsage::new(10_000, 5_000);
        let model = ModelLimits::new(100_000, 4_096);

        assert!(!is_overflow(&tokens, &model, None));
    }

    #[test]
    fn test_is_overflow_with_overflow() {
        let tokens = TokenUsage::new(90_000, 10_000);
        let model = ModelLimits::new(100_000, 4_096);

        assert!(is_overflow(&tokens, &model, None));
    }

    #[test]
    fn test_is_overflow_unlimited_context() {
        let tokens = TokenUsage::new(1_000_000, 500_000);
        let model = ModelLimits::new(0, 4_096); // Unlimited

        assert!(!is_overflow(&tokens, &model, None));
    }

    #[test]
    fn test_is_overflow_disabled_config() {
        let tokens = TokenUsage::new(80_000, 10_000);
        let model = ModelLimits::new(100_000, 4_096);
        let config = CompactionConfig {
            auto: false,
            ..Default::default()
        };

        assert!(!is_overflow(&tokens, &model, Some(&config)));
    }

    #[test]
    fn test_is_overflow_with_input_limit() {
        let tokens = TokenUsage::new(90_000, 10_000);
        let model = ModelLimits::with_input(200_000, 100_000, 4_096);

        assert!(is_overflow(&tokens, &model, None));
    }

    #[test]
    fn test_token_usage_count() {
        let tokens = TokenUsage {
            total: Some(100_000),
            input: 50_000,
            output: 30_000,
            cache_read: 10_000,
            cache_write: 10_000,
        };

        assert_eq!(tokens.count(), 100_000);
    }

    #[test]
    fn test_token_usage_count_without_total() {
        let tokens = TokenUsage {
            total: None,
            input: 50_000,
            output: 30_000,
            cache_read: 10_000,
            cache_write: 10_000,
        };

        assert_eq!(tokens.count(), 100_000);
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 1); // Minimum 1
        assert_eq!(estimate_tokens("abc"), 1); // 3/4 = 0, clamped to 1
        assert_eq!(estimate_tokens("abcdefgh"), 2); // 8/4 = 2
        assert_eq!(estimate_tokens("a very long string with many characters!"), 10); // 40 chars
    }

    #[test]
    fn test_event_serialization() {
        let event = Event::Compacted(CompactionEvent {
            session_id: "session_123".to_string(),
        });

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"compacted\""));
        assert!(json.contains("\"sessionId\":\"session_123\""));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, Event::Compacted(_)));
    }

    #[test]
    fn test_compaction_config_default() {
        let config = CompactionConfig::default();
        assert!(config.auto);
        assert!(config.prune);
        assert!(config.reserved.is_none());
    }

    #[test]
    fn test_model_limits_new() {
        let limits = ModelLimits::new(100_000, 4_096);
        assert_eq!(limits.context, 100_000);
        assert_eq!(limits.max_output, 4_096);
        assert!(limits.input.is_none());
    }

    #[test]
    fn test_model_limits_with_input() {
        let limits = ModelLimits::with_input(200_000, 100_000, 4_096);
        assert_eq!(limits.context, 200_000);
        assert_eq!(limits.input, Some(100_000));
        assert_eq!(limits.max_output, 4_096);
    }

    #[test]
    fn test_create_compaction_message() {
        let (msg_id, part_id) = create(
            "session_123".to_string(),
            "build".to_string(),
            ModelRef {
                provider_id: "openai".to_string(),
                model_id: "gpt-4".to_string(),
            },
            true,
            Some(false),
        );

        assert!(!msg_id.is_empty());
        assert!(!part_id.is_empty());
    }

    #[test]
    fn test_constants() {
        assert_eq!(COMPACTION_BUFFER, 20_000);
        assert_eq!(PRUNE_MINIMUM, 20_000);
        assert_eq!(PRUNE_PROTECT, 40_000);
        assert!(PRUNE_PROTECTED_TOOLS.contains(&"skill"));
    }

    #[test]
    fn test_process_result() {
        assert_eq!(ProcessResult::Continue, ProcessResult::Continue);
        assert_eq!(ProcessResult::Stop, ProcessResult::Stop);
        assert_ne!(ProcessResult::Continue, ProcessResult::Stop);
    }

    #[test]
    fn test_find_replay_point_no_match() {
        let messages: Vec<WithParts> = vec![];
        let (replay, truncated) = find_replay_point(&messages, "nonexistent");
        assert!(replay.is_none());
        assert!(truncated.is_empty());
    }
}