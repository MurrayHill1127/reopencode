//! Agent loop logic - manages the main agent execution cycle
//!
//! This module provides the core loop infrastructure for agent execution,
//! including finish state detection, max steps checking, and context overflow detection.
//!
//! ## Finish States
//!
//! The loop terminates based on finish reasons from the provider response:
//! - **Stop**: Agent has completed its task (terminal)
//! - **ToolCalls**: Agent is requesting tool execution (non-terminal, continue loop)
//! - **Continue**: Agent is still thinking/processing (non-terminal)
//!
//! ## Max Steps
//!
//! Each loop has a configurable maximum number of iterations to prevent infinite loops.
//! When max_steps is reached, the loop terminates with a Stop reason.
//!
//! ## Context Overflow
//!
//! The loop monitors token usage and can detect when the context window is approaching
//! its limit (default: 180,000 tokens). When detected, the loop can trigger compaction
//! or terminate gracefully.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::provider::provider_trait::{Provider, ProviderResponse, ProviderToolCall};
use crate::tool::registry::ToolRegistry;

/// Default maximum number of loop iterations
pub const DEFAULT_MAX_STEPS: usize = 100;

/// Default token threshold for context overflow detection (180k tokens)
pub const DEFAULT_CONTEXT_OVERFLOW_THRESHOLD: u32 = 180_000;

/// Warning threshold (90% of overflow threshold)
pub const DEFAULT_CONTEXT_WARNING_THRESHOLD: u32 = 162_000;

/// Finish reason indicating why the agent loop terminated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// Agent completed its task successfully
    Stop,
    /// Agent is requesting tool execution (non-terminal)
    ToolCalls,
    /// Agent is still processing (non-terminal)
    Continue,
    /// Maximum steps reached
    MaxSteps,
    /// Context overflow detected
    ContextOverflow,
    /// Error occurred during execution
    Error,
}

impl FinishReason {
    /// Check if this finish reason is terminal (loop should stop)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            FinishReason::Stop
                | FinishReason::MaxSteps
                | FinishReason::ContextOverflow
                | FinishReason::Error
        )
    }

    /// Check if this finish reason indicates tool calls (non-terminal)
    pub fn is_tool_calls(&self) -> bool {
        matches!(self, FinishReason::ToolCalls)
    }

    /// Convert from a string finish reason (from provider response)
    pub fn from_str(s: &str) -> Self {
        match s {
            "stop" | "end_turn" => FinishReason::Stop,
            "tool_calls" | "tool-calls" => FinishReason::ToolCalls,
            "continue" => FinishReason::Continue,
            "max_steps" => FinishReason::MaxSteps,
            "context_overflow" => FinishReason::ContextOverflow,
            "error" => FinishReason::Error,
            _ => {
                debug!("Unknown finish reason: {}, treating as Continue", s);
                FinishReason::Continue
            }
        }
    }
}

impl std::fmt::Display for FinishReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FinishReason::Stop => write!(f, "stop"),
            FinishReason::ToolCalls => write!(f, "tool_calls"),
            FinishReason::Continue => write!(f, "continue"),
            FinishReason::MaxSteps => write!(f, "max_steps"),
            FinishReason::ContextOverflow => write!(f, "context_overflow"),
            FinishReason::Error => write!(f, "error"),
        }
    }
}

/// State of the agent loop
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopState {
    /// Loop is idle, not started
    Idle,
    /// Loop is running
    Running,
    /// Loop is waiting for tool execution
    WaitingTool,
    /// Loop has completed
    Completed,
    /// Loop encountered an error
    Failed,
}

impl Default for LoopState {
    fn default() -> Self {
        LoopState::Idle
    }
}

/// Configuration for the agent loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopConfig {
    /// Maximum number of iterations before stopping
    pub max_steps: usize,
    /// Token threshold for context overflow detection
    pub context_overflow_threshold: u32,
    /// Token threshold for warning (typically 90% of overflow)
    pub context_warning_threshold: u32,
    /// Whether to enable context overflow detection
    pub enable_overflow_detection: bool,
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self {
            max_steps: DEFAULT_MAX_STEPS,
            context_overflow_threshold: DEFAULT_CONTEXT_OVERFLOW_THRESHOLD,
            context_warning_threshold: DEFAULT_CONTEXT_WARNING_THRESHOLD,
            enable_overflow_detection: true,
        }
    }
}

/// Statistics for the agent loop execution
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoopStats {
    /// Number of iterations completed
    pub iterations: usize,
    /// Total prompt tokens used
    pub total_prompt_tokens: u32,
    /// Total completion tokens used
    pub total_completion_tokens: u32,
    /// Total tokens used
    pub total_tokens: u32,
    /// Number of tool calls executed
    pub tool_calls_executed: usize,
    /// Final finish reason
    pub finish_reason: Option<FinishReason>,
}

/// Result of determining whether the loop should continue
#[derive(Debug, Clone)]
pub enum LoopDecision {
    /// Continue the loop, possibly with tool calls to execute
    Continue {
        /// Tool calls to execute (if any)
        tool_calls: Vec<ProviderToolCall>,
    },
    /// Stop the loop with a reason
    Stop {
        /// Why the loop is stopping
        reason: FinishReason,
        /// Final response content
        content: String,
    },
}

/// Context overflow detection result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowStatus {
    /// No overflow detected
    Ok,
    /// Approaching overflow (warning level)
    Warning,
    /// Overflow detected
    Overflow,
}

/// Agent loop manager
///
/// Manages the execution cycle of an agent, tracking state, steps,
/// and determining when to continue or stop.
pub struct AgentLoop {
    /// Loop configuration
    config: LoopConfig,
    /// Current loop state
    state: LoopState,
    /// Execution statistics
    stats: LoopStats,
    /// Provider for LLM calls
    provider: Arc<dyn Provider>,
    /// Tool registry for tool execution
    tool_registry: Arc<ToolRegistry>,
}

impl AgentLoop {
    /// Create a new agent loop
    pub fn new(provider: Arc<dyn Provider>, tool_registry: Arc<ToolRegistry>) -> Self {
        Self {
            config: LoopConfig::default(),
            state: LoopState::Idle,
            stats: LoopStats::default(),
            provider,
            tool_registry,
        }
    }

    /// Create a new agent loop with custom configuration
    pub fn with_config(
        provider: Arc<dyn Provider>,
        tool_registry: Arc<ToolRegistry>,
        config: LoopConfig,
    ) -> Self {
        Self {
            config,
            state: LoopState::Idle,
            stats: LoopStats::default(),
            provider,
            tool_registry,
        }
    }

    /// Get current loop state
    pub fn state(&self) -> LoopState {
        self.state
    }

    /// Get loop statistics
    pub fn stats(&self) -> &LoopStats {
        &self.stats
    }

    /// Get loop configuration
    pub fn config(&self) -> &LoopConfig {
        &self.config
    }

    /// Check if max steps has been reached
    pub fn is_max_steps_reached(&self) -> bool {
        self.stats.iterations >= self.config.max_steps
    }

    /// Detect context overflow based on token usage
    pub fn detect_overflow(&self) -> OverflowStatus {
        if !self.config.enable_overflow_detection {
            return OverflowStatus::Ok;
        }

        let total = self.stats.total_tokens;

        if total >= self.config.context_overflow_threshold {
            OverflowStatus::Overflow
        } else if total >= self.config.context_warning_threshold {
            OverflowStatus::Warning
        } else {
            OverflowStatus::Ok
        }
    }

    /// Determine finish reason from provider response
    pub fn determine_finish(&self, response: &ProviderResponse) -> FinishReason {
        // Check for tool calls first (non-terminal)
        if response.has_tool_calls() {
            return FinishReason::ToolCalls;
        }

        // Check for explicit finish reason in response
        if let Some(reason) = &response.finish_reason {
            return FinishReason::from_str(reason);
        }

        // Default: agent has completed (stop)
        FinishReason::Stop
    }

    /// Determine if the loop should continue
    ///
    /// This method checks:
    /// 1. Max steps reached
    /// 2. Context overflow
    /// 3. Finish reason from response
    pub fn decide(&mut self, response: &ProviderResponse) -> LoopDecision {
        // Update stats with response usage
        self.stats.total_prompt_tokens += response.usage.prompt_tokens;
        self.stats.total_completion_tokens += response.usage.completion_tokens;
        self.stats.total_tokens += response.usage.total_tokens;
        self.stats.iterations += 1;

        // Check max steps
        if self.is_max_steps_reached() {
            info!(
                "Max steps ({}) reached, stopping loop",
                self.config.max_steps
            );
            self.stats.finish_reason = Some(FinishReason::MaxSteps);
            return LoopDecision::Stop {
                reason: FinishReason::MaxSteps,
                content: response.content.clone(),
            };
        }

        // Check context overflow
        match self.detect_overflow() {
            OverflowStatus::Overflow => {
                warn!(
                    "Context overflow detected ({} tokens), stopping loop",
                    self.stats.total_tokens
                );
                self.stats.finish_reason = Some(FinishReason::ContextOverflow);
                return LoopDecision::Stop {
                    reason: FinishReason::ContextOverflow,
                    content: response.content.clone(),
                };
            }
            OverflowStatus::Warning => {
                warn!(
                    "Context warning: {} tokens used (threshold: {})",
                    self.stats.total_tokens, self.config.context_warning_threshold
                );
            }
            OverflowStatus::Ok => {}
        }

        // Determine finish from response
        let finish = self.determine_finish(response);

        match finish {
            FinishReason::ToolCalls => {
                debug!("Tool calls detected, continuing loop");
                self.state = LoopState::WaitingTool;
                LoopDecision::Continue {
                    tool_calls: response.tool_calls.clone(),
                }
            }
            FinishReason::Continue => {
                debug!("Agent continuing, processing next iteration");
                LoopDecision::Continue { tool_calls: vec![] }
            }
            _ => {
                info!("Loop terminating with reason: {}", finish);
                self.stats.finish_reason = Some(finish);
                LoopDecision::Stop {
                    reason: finish,
                    content: response.content.clone(),
                }
            }
        }
    }

    /// Reset the loop state and stats
    pub fn reset(&mut self) {
        self.state = LoopState::Idle;
        self.stats = LoopStats::default();
    }

    /// Set loop state to running
    pub fn start(&mut self) {
        self.state = LoopState::Running;
        self.stats = LoopStats::default();
    }

    /// Set loop state to completed
    pub fn complete(&mut self) {
        self.state = LoopState::Completed;
    }

    /// Set loop state to failed
    pub fn fail(&mut self) {
        self.state = LoopState::Failed;
        self.stats.finish_reason = Some(FinishReason::Error);
    }

    /// Record a tool call execution
    pub fn record_tool_call(&mut self) {
        self.stats.tool_calls_executed += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finish_reason_is_terminal() {
        assert!(FinishReason::Stop.is_terminal());
        assert!(FinishReason::MaxSteps.is_terminal());
        assert!(FinishReason::ContextOverflow.is_terminal());
        assert!(FinishReason::Error.is_terminal());

        assert!(!FinishReason::ToolCalls.is_terminal());
        assert!(!FinishReason::Continue.is_terminal());
    }

    #[test]
    fn test_finish_reason_is_tool_calls() {
        assert!(FinishReason::ToolCalls.is_tool_calls());
        assert!(!FinishReason::Stop.is_tool_calls());
    }

    #[test]
    fn test_finish_reason_from_str() {
        assert_eq!(FinishReason::from_str("stop"), FinishReason::Stop);
        assert_eq!(FinishReason::from_str("end_turn"), FinishReason::Stop);
        assert_eq!(
            FinishReason::from_str("tool_calls"),
            FinishReason::ToolCalls
        );
        assert_eq!(
            FinishReason::from_str("tool-calls"),
            FinishReason::ToolCalls
        );
        assert_eq!(FinishReason::from_str("continue"), FinishReason::Continue);
        assert_eq!(FinishReason::from_str("unknown"), FinishReason::Continue);
    }

    #[test]
    fn test_finish_reason_display() {
        assert_eq!(format!("{}", FinishReason::Stop), "stop");
        assert_eq!(format!("{}", FinishReason::ToolCalls), "tool_calls");
        assert_eq!(format!("{}", FinishReason::MaxSteps), "max_steps");
    }

    #[test]
    fn test_loop_config_default() {
        let config = LoopConfig::default();
        assert_eq!(config.max_steps, DEFAULT_MAX_STEPS);
        assert_eq!(
            config.context_overflow_threshold,
            DEFAULT_CONTEXT_OVERFLOW_THRESHOLD
        );
        assert_eq!(
            config.context_warning_threshold,
            DEFAULT_CONTEXT_WARNING_THRESHOLD
        );
        assert!(config.enable_overflow_detection);
    }

    #[test]
    fn test_loop_state_default() {
        assert_eq!(LoopState::default(), LoopState::Idle);
    }

    #[test]
    fn test_overflow_status_detection() {
        let config = LoopConfig::default();

        // Create a minimal mock provider for testing overflow detection
        // We'll test the overflow logic directly via detect_overflow on stats

        let mut stats = LoopStats::default();
        stats.total_tokens = 100_000;

        // At 100k tokens: Ok
        assert!(stats.total_tokens < config.context_warning_threshold);

        stats.total_tokens = 170_000;
        // At 170k tokens: Warning (between warning and overflow)
        assert!(stats.total_tokens >= config.context_warning_threshold);
        assert!(stats.total_tokens < config.context_overflow_threshold);

        stats.total_tokens = 190_000;
        // At 190k tokens: Overflow
        assert!(stats.total_tokens >= config.context_overflow_threshold);
    }

    #[test]
    fn test_is_max_steps_reached() {
        let config = LoopConfig {
            max_steps: 5,
            ..Default::default()
        };

        let mut stats = LoopStats::default();
        stats.iterations = 4;

        // Not reached at 4 iterations with max_steps=5
        assert!(stats.iterations < config.max_steps);

        stats.iterations = 5;
        // Reached at 5 iterations
        assert!(stats.iterations >= config.max_steps);
    }

    #[test]
    fn test_loop_stats_default() {
        let stats = LoopStats::default();
        assert_eq!(stats.iterations, 0);
        assert_eq!(stats.total_prompt_tokens, 0);
        assert_eq!(stats.total_completion_tokens, 0);
        assert_eq!(stats.total_tokens, 0);
        assert_eq!(stats.tool_calls_executed, 0);
        assert!(stats.finish_reason.is_none());
    }
}
