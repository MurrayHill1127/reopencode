//! Session prompt loop - orchestrates AI interactions in a session
//!
//! This module provides the main prompt loop that ties together LLM calls,
//! tool execution, and session updates. It's the core orchestration layer
//! that handles multi-step AI interactions.
//!
//! # Key Components
//!
//! - [`PromptInput`]: Input configuration for sending a prompt
//! - [`LoopInput`]: Configuration for the main loop
//! - [`prompt`]: Send a single prompt and optionally start the loop
//! - [`run_loop`]: Run the full agent loop (up to 100 steps)
//! - [`cancel`]: Abort a running prompt
//! - [`create_structured_output_tool`]: Create tool for JSON schema output
//!
//! # Architecture
//!
//! The prompt loop integrates:
//! - Wave 1's parts.rs and message.rs for message structure
//! - Wave 2's llm.rs stream function for LLM calls
//! - Wave 3's processor.rs for stream processing
//! - Wave 4's compaction.rs for context overflow handling
//! - Session status updates for UI feedback

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock, watch};
use tracing::{error, info, warn};

use super::parts::{
    Part, PartId, SessionId, MessageId, ProviderId, ModelId,
    SubtaskPart, CompactionPart, TokenInfo,
};
use super::message::{
    UserMessage, AssistantMessage, MessageInfo, WithParts,
    ModelRef, OutputFormat, UserTime, AssistantTime, PathInfo,
    to_model_messages, ProviderModel,
};
use super::status::{self, SessionStatusInfo};
use super::compaction::{self, TokenUsage, ModelLimits, is_overflow, ProcessResult as CompactionProcessResult};
use super::processor::{Processor, ProcessResult};
use super::llm::{
    StreamInput, ToolDef, ToolChoice,
};
use crate::agent::{AgentInfo, AgentRegistry};
use crate::provider::Provider;

// ============================================================================
// Cleanup Guard
// ============================================================================

/// RAII guard for cleaning up session state
struct CleanupGuard {
    session_id: String,
}

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        let session_id = self.session_id.clone();
        let state = PROMPT_STATE.get().unwrap();
        // Spawn a task to clear the session asynchronously
        tokio::spawn(async move {
            state.clear(&session_id).await;
        });
    }
}

// ============================================================================
// Constants
// ============================================================================

/// Maximum number of steps in the loop (matches TypeScript MAX_STEPS logic)
pub const MAX_STEPS: u32 = 100;

/// Structured output tool name
const STRUCTURED_OUTPUT_TOOL_NAME: &str = "StructuredOutput";

/// Description for the structured output tool
const STRUCTURED_OUTPUT_DESCRIPTION: &str = r#"Use this tool to return your final response in the requested structured format.

IMPORTANT:
- You MUST call this tool exactly once at the end of your response
- The input must be valid JSON matching the required schema
- Complete all necessary research and tool calls BEFORE calling this tool
- This tool provides your final answer - no further actions are taken after calling it"#;

/// System prompt for structured output mode
const STRUCTURED_OUTPUT_SYSTEM_PROMPT: &str = r#"IMPORTANT: The user has requested structured output. You MUST use the StructuredOutput tool to provide your final response. Do NOT respond with plain text - you MUST call the StructuredOutput tool with your answer formatted according to the schema."#;

// ============================================================================
// Error Types
// ============================================================================

/// Prompt error types
#[derive(Debug, Clone, thiserror::Error)]
pub enum PromptError {
    #[error("Session {0} is busy")]
    Busy(String),

    #[error("Session not found: {0}")]
    NotFound(String),

    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Model not found: {0}/{1}")]
    ModelNotFound(String, String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Context overflow")]
    ContextOverflow,

    #[error("Aborted")]
    Aborted,

    #[error("Max steps ({0}) exceeded")]
    MaxStepsExceeded(u32),

    #[error("No user message found")]
    NoUserMessage,

    #[error("{0}")]
    Other(String),
}

// ============================================================================
// Prompt Input Types
// ============================================================================

/// Part input for creating messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum PartInput {
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<PartId>,
    },
    File {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        filename: Option<String>,
        mime: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<PartId>,
        #[serde(skip_serializing_if = "Option::is_none")]
        source: Option<super::parts::FilePartSource>,
    },
    Agent {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<PartId>,
    },
    Subtask {
        prompt: String,
        description: String,
        agent: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        model: Option<ModelRef>,
        #[serde(skip_serializing_if = "Option::is_none")]
        command: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<PartId>,
    },
}

/// Model configuration for prompts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub provider_id: ProviderId,
    pub model_id: ModelId,
}

/// Input for the prompt function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptInput {
    /// Session ID to send the prompt to
    #[serde(rename = "sessionID")]
    pub session_id: SessionId,

    /// Optional message ID (generated if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "messageID")]
    pub message_id: Option<MessageId>,

    /// Model override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<ModelConfig>,

    /// Agent to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,

    /// Don't wait for assistant response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_reply: Option<bool>,

    /// Tool enable/disable overrides (deprecated - use session permissions)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<HashMap<String, bool>>,

    /// Output format (text or json_schema)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<OutputFormat>,

    /// Custom system prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,

    /// Model variant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,

    /// Message parts
    #[serde(default)]
    pub parts: Vec<PartInput>,
}

impl Default for PromptInput {
    fn default() -> Self {
        Self {
            session_id: String::new(),
            message_id: None,
            model: None,
            agent: None,
            no_reply: None,
            tools: None,
            format: None,
            system: None,
            variant: None,
            parts: Vec::new(),
        }
    }
}

/// Input for the loop function
#[derive(Debug, Clone)]
pub struct LoopInput {
    /// Session ID
    pub session_id: SessionId,

    /// Resume existing loop if session is busy
    pub resume_existing: bool,
}

impl Default for LoopInput {
    fn default() -> Self {
        Self {
            session_id: String::new(),
            resume_existing: false,
        }
    }
}

// ============================================================================
// Abort Controller
// ============================================================================

/// Abort controller using tokio watch channel for cancellation signaling
#[derive(Clone)]
pub struct AbortController {
    cancelled: watch::Sender<bool>,
    is_cancelled: watch::Receiver<bool>,
}

impl AbortController {
    /// Create a new abort controller
    pub fn new() -> Self {
        let (cancelled, is_cancelled) = watch::channel(false);
        Self { cancelled, is_cancelled }
    }

    /// Signal abort to all listeners
    pub fn abort(&self) {
        let _ = self.cancelled.send(true);
    }

    /// Check if already cancelled
    pub fn is_cancelled(&self) -> bool {
        *self.is_cancelled.borrow()
    }

    /// Get a receiver for cancellation signal
    pub fn cancelled_signal(&self) -> watch::Receiver<bool> {
        self.is_cancelled.clone()
    }
}

impl Default for AbortController {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Prompt State
// ============================================================================

/// Callback for when a message is completed
type MessageCallback = Box<dyn Fn(WithParts) + Send + Sync>;

/// Session state for tracking active prompts
struct SessionPromptState {
    /// Abort controller for the session
    abort: AbortController,
    /// Callbacks waiting for message completion
    callbacks: Vec<MessageCallback>,
}

/// Global prompt state tracking active sessions
pub struct PromptState {
    controllers: Arc<RwLock<HashMap<String, SessionPromptState>>>,
}

impl PromptState {
    /// Create a new prompt state
    pub fn new() -> Self {
        Self {
            controllers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if a session has an active prompt
    pub async fn is_active(&self, session_id: &str) -> bool {
        self.controllers.read().await.contains_key(session_id)
    }

    /// Start tracking a session, returning its abort signal if newly started
    pub async fn start(&self, session_id: &str) -> Option<watch::Receiver<bool>> {
        let mut controllers = self.controllers.write().await;
        if controllers.contains_key(session_id) {
            return None;
        }

        let controller = AbortController::new();
        let signal = controller.cancelled_signal();
        controllers.insert(
            session_id.to_string(),
            SessionPromptState {
                abort: controller,
                callbacks: Vec::new(),
            },
        );
        Some(signal)
    }

    /// Resume an existing session, returning its abort signal
    pub async fn resume(&self, session_id: &str) -> Option<watch::Receiver<bool>> {
        let controllers = self.controllers.read().await;
        controllers.get(session_id).map(|s| s.abort.cancelled_signal())
    }

    /// Cancel a session's active operation
    pub async fn cancel(&self, session_id: &str) {
        if let Some(state) = self.controllers.write().await.remove(session_id) {
            state.abort.abort();
        }
    }

    /// Get the abort controller for a session
    pub async fn get_abort(&self, session_id: &str) -> Option<AbortController> {
        self.controllers
            .read()
            .await
            .get(session_id)
            .map(|s| s.abort.clone())
    }

    /// Clear a session from tracking
    pub async fn clear(&self, session_id: &str) {
        self.controllers.write().await.remove(session_id);
    }
}

impl Default for PromptState {
    fn default() -> Self {
        Self::new()
    }
}

// Global singleton
static PROMPT_STATE: std::sync::OnceLock<PromptState> = std::sync::OnceLock::new();

/// Get the global prompt state singleton
pub fn global() -> &'static PromptState {
    PROMPT_STATE.get_or_init(PromptState::new)
}

// ============================================================================
// Core Functions
// ============================================================================

/// Assert that a session is not busy
///
/// # Errors
/// Returns `PromptError::Busy` if the session has an active prompt
pub async fn assert_not_busy(session_id: &str) -> Result<(), PromptError> {
    if global().is_active(session_id).await {
        return Err(PromptError::Busy(session_id.to_string()));
    }
    Ok(())
}

/// Cancel a session's active operation
///
/// Sets the session status to idle after cancellation.
/// This is safe to call even if the session is not active.
pub async fn cancel(session_id: &str) {
    info!("Cancelling session: {}", session_id);
    global().cancel(session_id).await;
    status::set(session_id, SessionStatusInfo::Idle);
}

/// Send a prompt to a session
///
/// This creates a user message and optionally starts the agent loop.
///
/// # Arguments
/// * `input` - Prompt input configuration
/// * `messages` - Existing messages in the session (for context)
/// * `provider` - Provider to use for LLM calls
/// * `agent_registry` - Agent registry for getting agent configuration
///
/// # Returns
/// * The assistant message with parts, or the user message if `no_reply` is set
pub async fn prompt(
    input: PromptInput,
    messages: Vec<WithParts>,
    provider: Arc<dyn Provider>,
    agent_registry: Arc<AgentRegistry>,
) -> Result<WithParts, PromptError> {
    let session_id = &input.session_id;

    // Check if session is busy
    assert_not_busy(session_id).await?;

    // Get agent configuration
    let agent_name = input.agent.clone().unwrap_or_else(|| "build".to_string());
    let agent = agent_registry
        .get(&agent_name)
        .cloned()
        .ok_or_else(|| PromptError::AgentNotFound(agent_name.clone()))?;

    // Create user message
    let user_message = create_user_message(&input, &agent)?;
    
    // If no_reply is set, return the user message without starting the loop
    if input.no_reply.unwrap_or(false) {
        return Ok(WithParts {
            info: MessageInfo::User(user_message),
            parts: vec![],
        });
    }

    // Start the loop
    let loop_input = LoopInput {
        session_id: session_id.clone(),
        resume_existing: false,
    };

    run_loop(loop_input, messages, provider, agent_registry).await
}

/// Run the main agent loop
///
/// This is the core orchestration that:
/// 1. Streams responses from the LLM
/// 2. Executes tool calls
/// 3. Handles compaction when context overflows
/// 4. Loops until the agent finishes or max steps reached
///
/// # Arguments
/// * `input` - Loop input configuration
/// * `messages` - Existing messages in the session
/// * `provider` - Provider to use for LLM calls
/// * `agent_registry` - Agent registry
///
/// # Returns
/// * The final assistant message with parts
pub async fn run_loop(
    input: LoopInput,
    mut messages: Vec<WithParts>,
    provider: Arc<dyn Provider>,
    agent_registry: Arc<AgentRegistry>,
) -> Result<WithParts, PromptError> {
    let session_id = &input.session_id;

    // Start or resume the session
    let abort_signal = if input.resume_existing {
        global().resume(session_id).await
    } else {
        global().start(session_id).await
    };

    // If no abort signal, another loop is running - wait for it
    let abort_rx = match abort_signal {
        Some(rx) => rx,
        None => {
            // Wait for existing loop to complete
            // In a full implementation, we would register a callback
            return Err(PromptError::Busy(session_id.clone()));
        }
    };

    // Ensure cleanup on exit
    let session_id_for_cleanup = session_id.clone();
    let _cleanup_guard = CleanupGuard {
        session_id: session_id_for_cleanup,
    };

    let mut step = 0;
    let structured_output: Arc<Mutex<Option<serde_json::Value>>> = Arc::new(Mutex::new(None));
    let structured_output_clone = structured_output.clone();

    loop {
        // Set status to busy
        status::set(session_id, SessionStatusInfo::Busy);

        // Check for cancellation
        if *abort_rx.borrow() {
            info!("Loop aborted for session: {}", session_id);
            return Err(PromptError::Aborted);
        }

        // Filter compacted messages
        messages = filter_compacted(messages);

        // Find the last user message and last finished assistant message
        let (last_user, last_assistant, last_finished, tasks) = find_last_messages(&messages);

        let last_user = match last_user {
            Some(u) => u,
            None => {
                error!("No user message found in session: {}", session_id);
                return Err(PromptError::NoUserMessage);
            }
        };

        // Check if we should exit the loop
        if let Some(ref assistant) = last_assistant {
            if let Some(ref finish) = assistant.finish {
                if !["tool-calls", "unknown"].contains(&finish.as_str()) {
                    if last_user.id < assistant.id {
                        info!("Loop finished for session: {}", session_id);
                        break;
                    }
                }
            }
        }

        step += 1;

        // Get agent configuration early for max_steps check
        let current_agent = agent_registry
            .get(&last_user.agent)
            .cloned()
            .unwrap_or_else(AgentInfo::default);

        // Check max steps
        let max_steps = current_agent.steps.unwrap_or(MAX_STEPS as usize) as u32;
        if step > max_steps {
            warn!("Max steps ({}) exceeded for session: {}", max_steps, session_id);
            return Err(PromptError::MaxStepsExceeded(max_steps));
        }

        // Get the model
        let model_id = &last_user.model.model_id;
        let provider_id = &last_user.model.provider_id;

        // Handle pending subtask
        if let Some(task) = tasks.iter().find(|t| matches!(t, Part::Subtask(_))) {
            if let Part::Subtask(subtask) = task {
                let result = handle_subtask(
                    session_id,
                    subtask,
                    &last_user,
                    &messages,
                    provider.clone(),
                    agent_registry.clone(),
                    &abort_rx,
                ).await;

                if let Err(e) = result {
                    error!("Subtask failed: {:?}", e);
                }
                continue;
            }
        }

        // Handle pending compaction
        if let Some(task) = tasks.iter().find(|t| matches!(t, Part::Compaction(_))) {
            if let Part::Compaction(compaction) = task {
                let result = handle_compaction(
                    session_id,
                    compaction,
                    &messages,
                    &abort_rx,
                ).await;

                match result {
                    Ok(CompactionProcessResult::Stop) => break,
                    Ok(CompactionProcessResult::Continue) => continue,
                    Err(e) => {
                        error!("Compaction failed: {:?}", e);
                    }
                }
            }
        }

        // Check for context overflow
        if let Some(ref finished) = last_finished {
            if finished.summary.unwrap_or(false) {
                let tokens = TokenUsage {
                    total: finished.tokens.total,
                    input: finished.tokens.input,
                    output: finished.tokens.output,
                    cache_read: finished.tokens.cache.read,
                    cache_write: finished.tokens.cache.write,
                };

                let model_limits = ModelLimits::new(100_000, 4_096); // TODO: Get from provider
                
                if is_overflow(&tokens, &model_limits, None) {
                    // Create compaction
                    let agent_name = &last_user.agent;
                    let model_ref = ModelRef {
                        provider_id: provider_id.clone(),
                        model_id: model_id.clone(),
                    };
                    
                    compaction::create(
                        session_id.clone(),
                        agent_name.clone(),
                        super::compaction::ModelRef {
                            provider_id: model_ref.provider_id.clone(),
                            model_id: model_ref.model_id.clone(),
                        },
                        true,
                        Some(true),
                    );
                    continue;
                }
            }
        }

        // Build tools
        let tools = resolve_tools_for_session(
            &current_agent,
            &last_user,
            &messages,
            provider.clone(),
        );

        // Add structured output tool if needed
        if let Some(OutputFormat::JsonSchema { schema, .. }) = &last_user.format {
            let output_store = structured_output_clone.clone();
            let _structured_tool = create_structured_output_tool(schema.clone(), move |output| {
                // Note: This is synchronous but we use try_lock to avoid blocking
                if let Ok(mut guard) = output_store.try_lock() {
                    *guard = Some(output);
                }
            });
            // tools.insert(STRUCTURED_OUTPUT_TOOL_NAME.to_string(), structured_tool);
        }

        // Create assistant message
        let assistant_id = generate_message_id();
        let assistant_message = AssistantMessage {
            id: assistant_id.clone(),
            session_id: session_id.clone(),
            time: AssistantTime {
                created: chrono::Utc::now().timestamp_millis(),
                completed: None,
            },
            error: None,
            parent_id: last_user.id.clone(),
            model_id: model_id.clone(),
            provider_id: provider_id.clone(),
            #[allow(deprecated)]
            mode: current_agent.name.clone(),
            agent: current_agent.name.clone(),
            path: PathInfo {
                cwd: std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "/".to_string()),
                root: std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "/".to_string()),
            },
            summary: None,
            cost: 0.0,
            tokens: TokenInfo::default(),
            structured: None,
            variant: last_user.variant.clone(),
            finish: None,
        };

        // Create processor
        let mut processor = Processor::new(assistant_message, Some(abort_rx.clone()));

        // Build stream input
        let provider_model = ProviderModel {
            provider_id: provider_id.clone(),
            model_id: model_id.clone(),
            api_npm: "unknown".to_string(), // TODO: Get from provider
            api_id: model_id.clone(),
        };

        let model_messages = to_model_messages(&messages, &provider_model, None);

        let stream_input = StreamInput {
            user: last_user.clone(),
            session_id: session_id.clone(),
            model: format!("{}/{}", provider_id, model_id),
            agent: current_agent.clone(),
            permission: None,
            system: build_system_prompts(&current_agent, &last_user),
            abort: Some(abort_rx.clone()),
            messages: model_messages,
            small: false,
            tools,
            retries: Some(3),
            tool_choice: last_user.format.as_ref().and_then(|f| {
                matches!(f, OutputFormat::JsonSchema { .. }).then_some(ToolChoice::Required)
            }),
        };

        // Process the stream
        let result = processor.process(stream_input, provider.clone()).await;

        match result {
            ProcessResult::Continue => {}
            ProcessResult::Stop => {
                let mut guard = structured_output.lock().await;
                if let Some(output) = guard.take() {
                    processor.message_mut().structured = Some(output);
                }
                break;
            }
            ProcessResult::Compact => {
                let agent_name = &last_user.agent;
                compaction::create(
                    session_id.clone(),
                    agent_name.clone(),
                    super::compaction::ModelRef {
                        provider_id: provider_id.clone(),
                        model_id: model_id.clone(),
                    },
                    true,
                    Some(true),
                );
            }
        }
    }

    // Return the last assistant message
    // In a full implementation, we would fetch from the store
    let last_assistant = messages
        .iter()
        .rev()
        .find_map(|m| match &m.info {
            MessageInfo::Assistant(a) => Some(WithParts {
                info: MessageInfo::Assistant(a.clone()),
                parts: m.parts.clone(),
            }),
            _ => None,
        });

    last_assistant.ok_or(PromptError::Other("No assistant message found".to_string()))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Generate a unique message ID
fn generate_message_id() -> MessageId {
    format!("msg_{}", uuid::Uuid::new_v4())
}

/// Generate a unique part ID
fn generate_part_id() -> PartId {
    format!("part_{}", uuid::Uuid::new_v4())
}

/// Create a user message from prompt input
fn create_user_message(input: &PromptInput, agent: &AgentInfo) -> Result<UserMessage, PromptError> {
    let message_id = input.message_id.clone().unwrap_or_else(generate_message_id);

    // Get model configuration
    let model = input.model.clone().map(|m| ModelRef {
        provider_id: m.provider_id,
        model_id: m.model_id,
    }).or_else(|| agent.model.as_ref().map(|m| ModelRef {
        provider_id: m.provider_id.clone(),
        model_id: m.model_id.clone(),
    })).unwrap_or_else(|| ModelRef {
        provider_id: "openai".to_string(),
        model_id: "gpt-4".to_string(),
    });

    Ok(UserMessage {
        id: message_id,
        session_id: input.session_id.clone(),
        time: UserTime {
            created: chrono::Utc::now().timestamp_millis(),
        },
        format: input.format.clone(),
        summary: None,
        agent: agent.name.clone(),
        model,
        system: input.system.clone(),
        tools: input.tools.clone(),
        variant: input.variant.clone(),
    })
}

/// Filter compacted messages
fn filter_compacted(messages: Vec<WithParts>) -> Vec<WithParts> {
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

/// Find the last user message, last assistant message, last finished message, and pending tasks
fn find_last_messages(messages: &[WithParts]) -> (
    Option<UserMessage>,
    Option<AssistantMessage>,
    Option<AssistantMessage>,
    Vec<Part>,
) {
    let mut last_user: Option<UserMessage> = None;
    let mut last_assistant: Option<AssistantMessage> = None;
    let mut last_finished: Option<AssistantMessage> = None;
    let mut tasks: Vec<Part> = Vec::new();

    for msg in messages.iter().rev() {
        if last_user.is_none() {
            if let MessageInfo::User(u) = &msg.info {
                last_user = Some(u.clone());
            }
        }

        if last_assistant.is_none() {
            if let MessageInfo::Assistant(a) = &msg.info {
                last_assistant = Some(a.clone());
            }
        }

        if last_finished.is_none() {
            if let MessageInfo::Assistant(a) = &msg.info {
                if a.finish.is_some() {
                    last_finished = Some(a.clone());
                }
            }
        }

        if last_user.is_some() && last_finished.is_some() {
            break;
        }

        // Collect pending tasks
        if last_finished.is_none() {
            for part in &msg.parts {
                if matches!(part, Part::Compaction(_)) || matches!(part, Part::Subtask(_)) {
                    tasks.push(part.clone());
                }
            }
        }
    }

    (last_user, last_assistant, last_finished, tasks)
}

/// Handle a pending subtask
async fn handle_subtask(
    _session_id: &str,
    subtask: &SubtaskPart,
    _last_user: &UserMessage,
    _messages: &[WithParts],
    _provider: Arc<dyn Provider>,
    agent_registry: Arc<AgentRegistry>,
    _abort_rx: &watch::Receiver<bool>,
) -> Result<(), PromptError> {
    info!("Handling subtask: {} ({})", subtask.description, subtask.agent);

    // Get the task agent
    let _task_agent = agent_registry
        .get(&subtask.agent)
        .cloned()
        .ok_or_else(|| PromptError::AgentNotFound(subtask.agent.clone()))?;

    // In a full implementation, we would:
    // 1. Create an assistant message with the tool part
    // 2. Execute the subtask with the task agent
    // 3. Update the tool part with the result
    // 4. Create a synthetic user message if needed

    Ok(())
}

/// Handle a pending compaction
async fn handle_compaction(
    session_id: &str,
    compaction: &CompactionPart,
    messages: &[WithParts],
    _abort_rx: &watch::Receiver<bool>,
) -> Result<CompactionProcessResult, PromptError> {
    info!("Handling compaction for session: {}", session_id);

    let input = compaction::ProcessInput {
        parent_id: messages.iter().rev()
            .find(|m| matches!(&m.info, crate::session::message::MessageInfo::User(_)))
            .map(|m| m.info.id().to_string())
            .unwrap_or_default(),
        messages: messages.to_vec(),
        session_id: session_id.to_string(),
        auto: compaction.auto,
        overflow: compaction.overflow,
    };

    Ok(compaction::process(&input))
}

/// Build system prompts for the agent
fn build_system_prompts(agent: &AgentInfo, user: &UserMessage) -> Vec<String> {
    let mut system = Vec::new();

    // Add environment info
    system.push(format!(
        "Current directory: {}",
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string())
    ));

    // Add agent prompt if available
    if let Some(ref prompt) = agent.prompt {
        system.push(prompt.clone());
    }

    // Add user's custom system prompt
    if let Some(ref user_system) = user.system {
        system.push(user_system.clone());
    }

    // Add structured output prompt if needed
    if matches!(user.format, Some(OutputFormat::JsonSchema { .. })) {
        system.push(STRUCTURED_OUTPUT_SYSTEM_PROMPT.to_string());
    }

    system
}

/// Resolve tools for a session
fn resolve_tools_for_session(
    _agent: &AgentInfo,
    user: &UserMessage,
    _messages: &[WithParts],
    _provider: Arc<dyn Provider>,
) -> HashMap<String, ToolDef> {
    // Start with agent tools
    let mut tools: HashMap<String, ToolDef> = HashMap::new();

    // Add basic tools
    // In a full implementation, we would:
    // 1. Get tools from the tool registry
    // 2. Filter by agent permissions
    // 3. Filter by user overrides (user.tools)

    // Apply user tool overrides
    if let Some(ref user_tools) = user.tools {
        // Remove disabled tools
        tools.retain(|name, _| user_tools.get(name).copied().unwrap_or(true));
    }

    tools
}

/// Create a structured output tool
///
/// This creates a tool that the model can call to return structured output
/// matching a JSON schema.
pub fn create_structured_output_tool<F>(
    schema: serde_json::Value,
    _on_success: F,
) -> ToolDef
where
    F: Fn(serde_json::Value) + Send + Sync + 'static,
{
    // Remove $schema property if present
    let schema = if let Some(obj) = schema.as_object() {
        let mut cleaned = obj.clone();
        cleaned.remove("$schema");
        serde_json::Value::Object(cleaned)
    } else {
        schema
    };

    ToolDef::new(
        STRUCTURED_OUTPUT_TOOL_NAME,
        STRUCTURED_OUTPUT_DESCRIPTION,
        schema,
    )
    // Note: In a full implementation, we would add the execute function
    // that captures structured_output and calls on_success
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abort_controller_new() {
        let ctrl = AbortController::new();
        assert!(!ctrl.is_cancelled());
    }

    #[test]
    fn test_abort_controller_abort() {
        let ctrl = AbortController::new();
        assert!(!ctrl.is_cancelled());
        ctrl.abort();
        assert!(ctrl.is_cancelled());
    }

    #[test]
    fn test_abort_controller_signal() {
        let ctrl = AbortController::new();
        let mut signal = ctrl.cancelled_signal();
        assert!(!*signal.borrow_and_update());
        ctrl.abort();
        assert!(signal.has_changed().unwrap());
        assert!(*signal.borrow_and_update());
    }

    #[tokio::test]
    async fn test_prompt_state_is_active() {
        let state = PromptState::new();
        assert!(!state.is_active("test-session").await);
    }

    #[tokio::test]
    async fn test_prompt_state_start() {
        let state = PromptState::new();
        let signal = state.start("test-session").await;
        assert!(signal.is_some());
        assert!(state.is_active("test-session").await);
    }

    #[tokio::test]
    async fn test_prompt_state_start_twice() {
        let state = PromptState::new();
        let signal1 = state.start("test-session").await;
        let signal2 = state.start("test-session").await;
        assert!(signal1.is_some());
        assert!(signal2.is_none()); // Already started
    }

    #[tokio::test]
    async fn test_prompt_state_cancel() {
        let state = PromptState::new();
        state.start("test-session").await;
        assert!(state.is_active("test-session").await);

        state.cancel("test-session").await;
        assert!(!state.is_active("test-session").await);
    }

    #[tokio::test]
    async fn test_assert_not_busy() {
        assert!(assert_not_busy("nonexistent-session").await.is_ok());

        let state = global();
        state.start("busy-session").await;
        let result = assert_not_busy("busy-session").await;
        assert!(matches!(result, Err(PromptError::Busy(_))));
        state.cancel("busy-session").await;
    }

    #[tokio::test]
    async fn test_cancel_function() {
        cancel("test-cancel-session").await;
        // Should not panic, and status should be idle
        let status = status::get("test-cancel-session");
        assert_eq!(status, SessionStatusInfo::Idle);
    }

    #[test]
    fn test_prompt_input_default() {
        let input = PromptInput::default();
        assert!(input.session_id.is_empty());
        assert!(input.message_id.is_none());
        assert!(input.model.is_none());
        assert!(input.agent.is_none());
    }

    #[test]
    fn test_prompt_input_serialization() {
        let input = PromptInput {
            session_id: "sess_123".to_string(),
            agent: Some("build".to_string()),
            parts: vec![PartInput::Text {
                text: "Hello".to_string(),
                id: None,
            }],
            ..Default::default()
        };

        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("\"sessionID\":\"sess_123\""));
        assert!(json.contains("\"agent\":\"build\""));

        let deserialized: PromptInput = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.session_id, "sess_123");
    }

    #[test]
    fn test_loop_input_default() {
        let input = LoopInput::default();
        assert!(input.session_id.is_empty());
        assert!(!input.resume_existing);
    }

    #[test]
    fn test_generate_message_id() {
        let id1 = generate_message_id();
        let id2 = generate_message_id();
        assert!(id1.starts_with("msg_"));
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_generate_part_id() {
        let id1 = generate_part_id();
        let id2 = generate_part_id();
        assert!(id1.starts_with("part_"));
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_filter_compacted_empty() {
        let messages: Vec<WithParts> = vec![];
        let filtered = filter_compacted(messages);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_find_last_messages_empty() {
        let messages: Vec<WithParts> = vec![];
        let (user, assistant, finished, tasks) = find_last_messages(&messages);
        assert!(user.is_none());
        assert!(assistant.is_none());
        assert!(finished.is_none());
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_find_last_messages_with_user() {
        let user_msg = WithParts {
            info: MessageInfo::User(UserMessage {
                id: "msg_user".to_string(),
                session_id: "sess".to_string(),
                agent: "build".to_string(),
                model: ModelRef {
                    provider_id: "openai".to_string(),
                    model_id: "gpt-4".to_string(),
                },
                ..Default::default()
            }),
            parts: vec![],
        };

        let messages = vec![user_msg];
        let (user, assistant, finished, tasks) = find_last_messages(&messages);
        assert!(user.is_some());
        assert!(assistant.is_none());
        assert!(finished.is_none());
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_build_system_prompts() {
        let agent = AgentInfo::default();
        let user = UserMessage::default();

        let prompts = build_system_prompts(&agent, &user);
        assert!(!prompts.is_empty());
        assert!(prompts[0].contains("Current directory:"));
    }

    #[test]
    fn test_build_system_prompts_with_custom() {
        let agent = AgentInfo {
            prompt: Some("You are a helpful assistant.".to_string()),
            ..Default::default()
        };
        let user = UserMessage {
            system: Some("Focus on Rust code.".to_string()),
            ..Default::default()
        };

        let prompts = build_system_prompts(&agent, &user);
        assert!(prompts.iter().any(|p| p.contains("helpful assistant")));
        assert!(prompts.iter().any(|p| p.contains("Rust code")));
    }

    #[test]
    fn test_create_structured_output_tool() {
        let schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            }
        });

        let tool = create_structured_output_tool(schema, |_| {});

        assert_eq!(tool.name, STRUCTURED_OUTPUT_TOOL_NAME);
        assert!(tool.description.contains("structured format"));
        // $schema should be removed
        let params = &tool.parameters;
        assert!(!params.as_object().unwrap().contains_key("$schema"));
    }

    #[test]
    fn test_prompt_error_display() {
        let err = PromptError::Busy("sess_123".to_string());
        assert_eq!(format!("{}", err), "Session sess_123 is busy");

        let err = PromptError::AgentNotFound("build".to_string());
        assert_eq!(format!("{}", err), "Agent not found: build");

        let err = PromptError::MaxStepsExceeded(100);
        assert_eq!(format!("{}", err), "Max steps (100) exceeded");
    }

    #[test]
    fn test_global_singleton() {
        let g1 = global();
        let g2 = global();
        // Same instance
        assert!(std::ptr::eq(g1, g2));
    }

    #[test]
    fn test_max_steps_constant() {
        assert_eq!(MAX_STEPS, 100);
    }

    #[test]
    fn test_structured_output_constants() {
        assert_eq!(STRUCTURED_OUTPUT_TOOL_NAME, "StructuredOutput");
        assert!(STRUCTURED_OUTPUT_DESCRIPTION.contains("MUST call this tool"));
        assert!(STRUCTURED_OUTPUT_SYSTEM_PROMPT.contains("MUST use the StructuredOutput tool"));
    }
}