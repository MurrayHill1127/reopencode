# Hook 模块规格文档

**版本:** 0.1.0-draft  
**创建日期:** 2026-03-17  
**状态:** 待实现  
**优先级:** MVP (核心模块)

---

## 1. 概述 (Overview)

### 模块目的

Hook 模块是 ROC (reopencode) 的生命周期钩子系统核心，负责：

- **事件驱动回调** - 在关键事件点执行自定义逻辑
- **优先级执行** - 支持 5 个优先级层级 (onBefore, onInstead, onAfter, onSuccess, onError)
- **同步/异步钩子** - 支持 sync 和 async 两种执行模式
- **钩子注册与管理** - 动态注册、禁用、注销钩子
- **与 Agent 系统集成** - 钩子可访问和修改 Agent 上下文

### 设计目标

| 目标 | 说明 | 优先级 |
|------|------|--------|
| 类型安全 | 所有钩子强类型化，编译期检查 | MVP |
| 事件驱动 | 支持 8 种核心事件类型 | MVP |
| 优先级执行 | 5 级优先级：onBefore → onInstead → onAfter → onSuccess/onError | MVP |
| 异步支持 | 所有钩子支持 async/await | MVP |
| 错误隔离 | 单个钩子失败不影响其他钩子 | MVP |
| 动态注册 | 运行时注册和注销钩子 | MVP |
| 禁用机制 | 通过配置禁用特定钩子 | MVP |

### MVP 范围 (v0.1.0)

**必须实现:**
- ✅ Hook trait 定义
- ✅ HookRegistry 钩子注册中心
- ✅ HookContext 执行上下文
- ✅ 5 级优先级执行
- ✅ 8 种核心事件类型
- ✅ 错误隔离机制
- ✅ 同步/异步执行

**迭代实现:**
- ⏳ 内置钩子实现 (model-fallback, session-recovery 等)
- ⏳ 钩子配置文件支持
- ⏳ 钩子性能监控
- ⏳ 钩子调试工具
- ⏳ 第三方钩子加载

---

## 2. 文件索引 (File Index)

| 文件路径 | 职责 | 关键导出 | TypeScript 参考 |
|----------|------|----------|-----------------|
| `src/hook/mod.rs` | 模块根，公共 API 导出 | `Hook`, `HookRegistry`, `HookContext` | `plugin/types.ts` |
| `src/hook/types.rs` | 数据结构定义 | `HookId`, `HookPriority`, `HookEvent`, `HookResult` | `plugin/hooks/*.ts` |
| `src/hook/registry.rs` | 钩子注册中心 | `HookRegistry`, `register_hook`, `dispatch` | `create-hooks.ts` |
| `src/hook/context.rs` | 执行上下文 | `HookContext`, `SessionContext`, `ToolContext` | 各 hook 实现参数 |
| `src/hook/executor.rs` | 执行引擎 | `HookExecutor`, `execute_chain` | `plugin/chat-message.ts` 等处理器 |
| `src/hook/builtin.rs` | 内置钩子 | 内置钩子工厂函数 | `hooks/*/hook.ts` |
| `src/hook/error.rs` | 错误类型定义 | `HookError`, `ExecutionError` | 自定义 |

### 模块结构

```
src/hook/
├── mod.rs              # 公共 API 导出 (~150 行)
├── types.rs            # 数据结构 (~250 行)
├── registry.rs         # 注册中心 (~200 行)
├── context.rs          # 执行上下文 (~150 行)
├── executor.rs         # 执行引擎 (~200 行)
├── builtin.rs          # 内置钩子 (~500 行)
└── error.rs            # 错误类型 (~80 行)
```

---

## 3. 数据结构 (Data Structures)

### 3.1 钩子标识 (Hook Identity)

```rust
/// 钩子唯一标识符
/// 对应 TypeScript: HookName (config/schema/hooks.ts:3-54)
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct HookId(pub String);

impl HookId {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 内置钩子名称枚举
/// 对应 TypeScript: HookNameSchema (config/schema/hooks.ts)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BuiltinHookName {
    // Session 级别钩子 (23)
    ContextWindowMonitor,
    PreemptiveCompaction,
    SessionRecovery,
    SessionNotification,
    ThinkMode,
    ModelFallback,
    AnthropicContextWindowLimitRecovery,
    AutoUpdateChecker,
    AgentUsageReminder,
    NonInteractiveEnv,
    InteractiveBashSession,
    RalphLoop,
    EditErrorRecovery,
    DelegateTaskRetry,
    StartWork,
    PrometheusMdOnly,
    SisyphusJuniorNotepad,
    NoSisyphusGpt,
    NoHephaestusNonGpt,
    QuestionLabelTruncator,
    TaskResumeInfo,
    AnthropicEffort,
    RuntimeFallback,
    
    // Tool Guard 钩子 (10)
    CommentChecker,
    ToolOutputTruncator,
    DirectoryAgentsInjector,
    DirectoryReadmeInjector,
    EmptyTaskResponseDetector,
    RulesInjector,
    TasksTodowriteDisabler,
    WriteExistingFileGuard,
    HashlineReadEnhancer,
    JsonErrorRecovery,
    
    // Transform 钩子 (4)
    ClaudeCodeHooks,
    KeywordDetector,
    ContextInjectorMessagesTransform,
    ThinkingBlockValidator,
    
    // Continuation 钩子 (7)
    StopContinuationGuard,
    CompactionContextInjector,
    CompactionTodoPreserver,
    TodoContinuationEnforcer,
    UnstableAgentBabysitter,
    BackgroundNotification,
    Atlas,
    
    // Skill 钩子 (2)
    CategorySkillReminder,
    AutoSlashCommand,
}
```

### 3.2 钩子优先级 (Hook Priority)

```rust
/// 钩子优先级层级
/// 对应 TypeScript: hook 执行顺序 (plugin/chat-message.ts 等处理器)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HookPriority {
    /// 前置钩子 - 在目标操作前执行
    /// 可用于：验证、预处理、阻止操作
    OnBefore = 1,
    
    /// 替代钩子 - 替代目标操作
    /// 如果返回 Some，跳过原始操作
    OnInstead = 2,
    
    /// 后置钩子 - 在目标操作后执行
    /// 可用于：日志、清理、转换结果
    OnAfter = 3,
    
    /// 成功钩子 - 操作成功后执行
    /// 可用于：通知、更新状态
    OnSuccess = 4,
    
    /// 错误钩子 - 操作失败后执行
    /// 可用于：错误恢复、日志
    OnError = 5,
}

impl Default for HookPriority {
    fn default() -> Self {
        Self::OnAfter
    }
}
```

### 3.3 钩子事件 (Hook Event)

```rust
/// 钩子事件类型
/// 对应 TypeScript: PluginInterface 的 8 个 hook handler
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type", content = "properties")]
pub enum HookEvent {
    // ===== Chat 事件 =====
    /// 聊天消息事件
    /// 对应 TypeScript: "chat.message" handler
    ChatMessage {
        session_id: String,
        agent: Option<String>,
        model: Option<ResolvedModel>,
    },
    
    /// 聊天参数修改事件
    /// 对应 TypeScript: "chat.params" handler
    ChatParams {
        session_id: String,
        provider_id: String,
        model_id: String,
    },
    
    /// 聊天头修改事件
    /// 对应 TypeScript: "chat.headers" handler
    ChatHeaders {
        session_id: String,
        headers: std::collections::HashMap<String, String>,
    },
    
    // ===== Tool 事件 =====
    /// 工具注册事件
    /// 对应 TypeScript: "tool" handler
    ToolRegister {
        tools: Vec<String>,
    },
    
    /// 工具执行前事件
    /// 对应 TypeScript: "tool.execute.before" handler
    ToolExecuteBefore {
        session_id: String,
        tool_name: String,
        tool_input: serde_json::Value,
    },
    
    /// 工具执行后事件
    /// 对应 TypeScript: "tool.execute.after" handler
    ToolExecuteAfter {
        session_id: String,
        tool_name: String,
        tool_input: serde_json::Value,
        tool_output: serde_json::Value,
    },
    
    // ===== Session 事件 =====
    /// 会话生命周期事件
    /// 对应 TypeScript: "event" handler
    SessionEvent {
        event_type: SessionEventType,
        session_id: String,
        properties: Option<serde_json::Value>,
    },
    
    // ===== Transform 事件 =====
    /// 消息转换事件
    /// 对应 TypeScript: "experimental.chat.messages.transform" handler
    MessagesTransform {
        session_id: String,
        messages: Vec<ChatMessage>,
    },
}

/// 会话事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEventType {
    Created,
    Deleted,
    Idle,
    Compacted,
    Error,
}

/// 解析后的模型
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResolvedModel {
    pub provider_id: String,
    pub model_id: String,
    pub variant: Option<String>,
}

/// 聊天消息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: Vec<MessageContent>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum MessageContent {
    Text { text: String },
    Image { source: ImageSource },
    ToolUse { id: String, name: String, input: serde_json::Value },
    ToolResult { tool_use_id: String, content: String, is_error: bool },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImageSource {
    pub r#type: String,
    pub media_type: String,
    pub data: String,
}
```

### 3.4 钩子上下文 (Hook Context)

```rust
/// 钩子执行上下文
/// 对应 TypeScript: 各 hook 函数的 input/output 参数
#[derive(Debug, Clone)]
pub struct HookContext {
    /// 钩子 ID
    pub hook_id: HookId,
    
    /// 触发的事件
    pub event: HookEvent,
    
    /// 会话上下文 (可选)
    pub session: Option<SessionContext>,
    
    /// 工具上下文 (可选)
    pub tool: Option<ToolContext>,
    
    /// 执行元数据
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

/// 会话上下文
#[derive(Debug, Clone)]
pub struct SessionContext {
    pub id: String,
    pub agent: Option<String>,
    pub model: Option<ResolvedModel>,
    pub directory: std::path::PathBuf,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub message_count: usize,
    pub token_usage: Option<TokenUsage>,
}

/// 工具上下文
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub name: String,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub execution_time_ms: Option<u64>,
}

/// Token 使用统计
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenUsage {
    pub total: u64,
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}
```

### 3.5 钩子结果 (Hook Result)

```rust
/// 钩子执行结果
/// 用于控制后续执行流程
#[derive(Debug, Clone)]
pub enum HookResult {
    /// 继续执行下一个钩子
    Continue,
    
    /// 继续执行，但携带修改后的数据
    /// 对应 TypeScript: output 参数修改
    Modified(HookOutput),
    
    /// 跳过剩余钩子和原始操作
    /// 仅 OnInstead 优先级有效
    Skip(HookOutput),
    
    /// 停止执行链，返回错误
    Stop(HookError),
}

/// 钩子输出
#[derive(Debug, Clone, Default)]
pub struct HookOutput {
    /// 修改后的消息
    pub message: Option<serde_json::Value>,
    
    /// 修改后的参数
    pub params: Option<serde_json::Value>,
    
    /// 修改后的头
    pub headers: Option<std::collections::HashMap<String, String>>,
    
    /// Toast 通知
    pub toast: Option<ToastRequest>,
    
    /// 注入的内容
    pub injected_content: Option<String>,
    
    /// 自定义数据
    pub custom: std::collections::HashMap<String, serde_json::Value>,
}

/// Toast 通知请求
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToastRequest {
    pub title: String,
    pub message: String,
    pub variant: ToastVariant,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToastVariant {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}
```

### 3.6 钩子配置 (Hook Config)

```rust
/// 钩子配置
/// 对应 TypeScript: hooks.ts 部分配置
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct HookConfig {
    /// 禁用的钩子列表
    /// 对应 TypeScript: disabled_hooks
    #[serde(default)]
    pub disabled_hooks: Vec<String>,
    
    /// 钩子覆盖配置
    #[serde(default)]
    pub overrides: std::collections::HashMap<String, HookOverride>,
}

/// 钩子覆盖配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct HookOverride {
    /// 是否禁用
    #[serde(default)]
    pub disabled: bool,
    
    /// 优先级覆盖
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<HookPriority>,
    
    /// 自定义配置
    #[serde(flatten)]
    pub config: serde_json::Value,
}
```

---

## 4. API 设计 (API Design)

### 4.1 公共 API (mod.rs 导出)

```rust
// src/hook/mod.rs

mod error;
mod types;
mod registry;
mod context;
mod executor;
mod builtin;

pub use error::{HookError, ExecutionError};
pub use types::{
    HookId, BuiltinHookName, HookPriority, HookEvent,
    HookResult, HookOutput, ToastRequest, ToastVariant,
    HookConfig, HookOverride,
    SessionEventType, ResolvedModel, ChatMessage, MessageContent,
    TokenUsage,
};
pub use context::{HookContext, SessionContext, ToolContext};
pub use registry::{HookRegistry, HookEntry, register_hook, dispatch};
pub use executor::{HookExecutor, execute_chain};
pub use builtin::{
    create_model_fallback_hook,
    create_session_recovery_hook,
    create_context_window_monitor_hook,
    // ... 其他内置钩子工厂
};

/// 钩子 Trait
/// 所有钩子必须实现此 trait
pub trait Hook: Send + Sync {
    /// 钩子 ID
    fn id(&self) -> &HookId;
    
    /// 钩子优先级
    fn priority(&self) -> HookPriority {
        HookPriority::default()
    }
    
    /// 订阅的事件类型
    fn events(&self) -> Vec<HookEventType>;
    
    /// 执行钩子
    fn execute(&self, ctx: &mut HookContext) -> impl std::future::Future<Output = HookResult> + Send;
    
    /// 是否启用
    fn is_enabled(&self) -> bool {
        true
    }
    
    /// 清理资源 (可选)
    fn dispose(&self) {}
}

/// 钩子事件类型 (用于订阅过滤)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HookEventType {
    ChatMessage,
    ChatParams,
    ChatHeaders,
    ToolRegister,
    ToolExecuteBefore,
    ToolExecuteAfter,
    SessionEvent,
    MessagesTransform,
}

/// 快捷方法：创建钩子注册中心
pub fn create_registry() -> HookRegistry {
    HookRegistry::new()
}

/// 快捷方法：注册内置钩子
pub fn register_builtin_hooks(registry: &mut HookRegistry, config: &HookConfig) {
    builtin::register_all(registry, config);
}
```

### 4.2 HookRegistry

```rust
// src/hook/registry.rs

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::hook::*;

/// 钩子注册中心
/// 
/// 管理所有注册的钩子，提供按事件类型和优先级执行的能力
/// 
/// # Example
/// 
/// ```rust
/// use reopencode::hook::{HookRegistry, Hook, HookId, HookContext, HookResult};
/// 
/// struct MyHook;
/// 
/// impl Hook for MyHook {
///     fn id(&self) -> &HookId {
///         static ID: HookId = HookId::new("my-hook");
///         &ID
///     }
///     
///     fn events(&self) -> Vec<HookEventType> {
///         vec![HookEventType::ChatMessage]
///     }
///     
///     async fn execute(&self, ctx: &mut HookContext) -> HookResult {
///         println!("Hook executed!");
///         HookResult::Continue
///     }
/// }
/// 
/// let mut registry = HookRegistry::new();
/// registry.register(Box::new(MyHook));
/// ```
pub struct HookRegistry {
    /// 按事件类型索引的钩子
    hooks_by_event: HashMap<HookEventType, Vec<HookEntry>>,
    
    /// 所有钩子的映射 (ID -> Entry)
    hooks_by_id: HashMap<HookId, HookEntry>,
    
    /// 禁用的钩子 ID 集合
    disabled_hooks: std::collections::HashSet<String>,
    
    /// 执行统计
    stats: Arc<RwLock<HookStats>>,
}

/// 钩子条目
pub struct HookEntry {
    /// 钩子实现
    pub hook: Box<dyn Hook>,
    
    /// 注册时间
    pub registered_at: chrono::DateTime<chrono::Utc>,
    
    /// 执行次数
    pub execution_count: u64,
    
    /// 最后执行时间
    pub last_executed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// 执行统计
#[derive(Debug, Default)]
pub struct HookStats {
    pub total_executions: u64,
    pub total_errors: u64,
    pub total_time_ms: u64,
    pub by_hook: HashMap<String, HookExecutionStats>,
}

#[derive(Debug, Default)]
pub struct HookExecutionStats {
    pub count: u64,
    pub errors: u64,
    pub total_time_ms: u64,
    pub avg_time_ms: f64,
}

impl HookRegistry {
    /// 创建新的注册中心
    pub fn new() -> Self {
        Self {
            hooks_by_event: HashMap::new(),
            hooks_by_id: HashMap::new(),
            disabled_hooks: std::collections::HashSet::new(),
            stats: Arc::new(RwLock::new(HookStats::default())),
        }
    }
    
    /// 注册钩子
    pub fn register(&mut self, hook: Box<dyn Hook>) -> Result<(), HookError> {
        let id = hook.id().clone();
        let events = hook.events();
        
        // 检查是否已存在
        if self.hooks_by_id.contains_key(&id) {
            return Err(HookError::AlreadyRegistered(id));
        }
        
        let entry = HookEntry {
            hook,
            registered_at: chrono::Utc::now(),
            execution_count: 0,
            last_executed_at: None,
        };
        
        // 添加到 ID 索引
        self.hooks_by_id.insert(id.clone(), entry);
        
        // 添加到事件索引
        for event_type in events {
            self.hooks_by_event
                .entry(event_type)
                .or_insert_with(Vec::new)
                .push(self.hooks_by_id.get(&id).unwrap().clone());
        }
        
        // 按优先级排序
        for hooks in self.hooks_by_event.values_mut() {
            hooks.sort_by_key(|e| e.hook.priority());
        }
        
        Ok(())
    }
    
    /// 注销钩子
    pub fn unregister(&mut self, id: &HookId) -> Result<(), HookError> {
        let entry = self.hooks_by_id.remove(id)
            .ok_or_else(|| HookError::NotFound(id.clone()))?;
        
        // 从事件索引中移除
        for event_type in entry.hook.events() {
            if let Some(hooks) = self.hooks_by_event.get_mut(&event_type) {
                hooks.retain(|e| e.hook.id() != id);
            }
        }
        
        Ok(())
    }
    
    /// 禁用钩子
    pub fn disable(&mut self, name: &str) {
        self.disabled_hooks.insert(name.to_string());
    }
    
    /// 启用钩子
    pub fn enable(&mut self, name: &str) {
        self.disabled_hooks.remove(name);
    }
    
    /// 检查钩子是否启用
    pub fn is_enabled(&self, id: &HookId) -> bool {
        !self.disabled_hooks.contains(id.as_str())
    }
    
    /// 获取指定事件类型的钩子
    pub fn get_hooks_for_event(&self, event_type: HookEventType) -> Vec<&HookEntry> {
        self.hooks_by_event
            .get(&event_type)
            .map(|hooks| hooks.iter().collect())
            .unwrap_or_default()
    }
    
    /// 获取统计信息
    pub async fn stats(&self) -> HookStats {
        self.stats.read().await.clone()
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

### 4.3 HookExecutor

```rust
// src/hook/executor.rs

use std::sync::Arc;
use tokio::sync::RwLock;
use crate::hook::*;

/// 钩子执行器
/// 
/// 负责执行钩子链，处理错误隔离和结果合并
pub struct HookExecutor {
    /// 钩子注册中心
    registry: Arc<RwLock<HookRegistry>>,
    
    /// 是否启用错误隔离
    error_isolation: bool,
    
    /// 超时时间 (毫秒)
    timeout_ms: u64,
}

impl HookExecutor {
    /// 创建新的执行器
    pub fn new(registry: Arc<RwLock<HookRegistry>>) -> Self {
        Self {
            registry,
            error_isolation: true,
            timeout_ms: 30000,
        }
    }
    
    /// 设置错误隔离
    pub fn with_error_isolation(mut self, enabled: bool) -> Self {
        self.error_isolation = enabled;
        self
    }
    
    /// 设置超时
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
    
    /// 执行钩子链
    /// 
    /// 按优先级顺序执行所有匹配的钩子
    /// 
    /// # Arguments
    /// 
    /// * `event` - 触发的事件
    /// * `initial_context` - 初始上下文
    /// 
    /// # Returns
    /// 
    /// 返回最终的结果和输出
    pub async fn execute(
        &self,
        event: HookEvent,
        mut context: HookContext,
    ) -> Result<HookOutput, HookError> {
        let event_type = event_to_type(&event);
        let registry = self.registry.read().await;
        
        let hooks = registry.get_hooks_for_event(event_type);
        let mut output = HookOutput::default();
        let mut skip_remaining = false;
        
        for entry in hooks {
            // 检查是否启用
            if !entry.hook.is_enabled() || !registry.is_enabled(entry.hook.id()) {
                continue;
            }
            
            // 检查是否应该跳过
            if skip_remaining {
                break;
            }
            
            // 执行钩子 (带超时)
            let result = self.execute_with_timeout(&*entry.hook, &mut context).await;
            
            match result {
                Ok(HookResult::Continue) => {
                    // 继续执行下一个钩子
                }
                Ok(HookResult::Modified(modified)) => {
                    // 合并修改
                    output = self.merge_output(output, modified);
                }
                Ok(HookResult::Skip(skipped_output)) => {
                    // 跳过剩余钩子 (仅 OnInstead 有效)
                    if entry.hook.priority() == HookPriority::OnInstead {
                        output = skipped_output;
                        skip_remaining = true;
                    }
                }
                Ok(HookResult::Stop(error)) => {
                    // 停止执行链
                    return Err(error);
                }
                Err(error) => {
                    // 错误处理
                    if !self.error_isolation {
                        return Err(error);
                    }
                    // 记录错误但继续执行
                    tracing::warn!(
                        hook_id = %entry.hook.id().as_str(),
                        error = %error,
                        "Hook execution failed, but continuing due to error isolation"
                    );
                }
            }
        }
        
        Ok(output)
    }
    
    /// 带超时执行钩子
    async fn execute_with_timeout(
        &self,
        hook: &dyn Hook,
        ctx: &mut HookContext,
    ) -> Result<HookResult, HookError> {
        let timeout = tokio::time::Duration::from_millis(self.timeout_ms);
        
        tokio::time::timeout(timeout, hook.execute(ctx))
            .await
            .map_err(|_| HookError::Timeout(hook.id().clone()))?
    }
    
    /// 合并输出
    fn merge_output(&self, base: HookOutput, overlay: HookOutput) -> HookOutput {
        HookOutput {
            message: overlay.message.or(base.message),
            params: overlay.params.or(base.params),
            headers: overlay.headers.or(base.headers),
            toast: overlay.toast.or(base.toast),
            injected_content: overlay.injected_content.or(base.injected_content),
            custom: {
                let mut merged = base.custom;
                merged.extend(overlay.custom);
                merged
            },
        }
    }
}

/// 将事件转换为事件类型
fn event_to_type(event: &HookEvent) -> HookEventType {
    match event {
        HookEvent::ChatMessage { .. } => HookEventType::ChatMessage,
        HookEvent::ChatParams { .. } => HookEventType::ChatParams,
        HookEvent::ChatHeaders { .. } => HookEventType::ChatHeaders,
        HookEvent::ToolRegister { .. } => HookEventType::ToolRegister,
        HookEvent::ToolExecuteBefore { .. } => HookEventType::ToolExecuteBefore,
        HookEvent::ToolExecuteAfter { .. } => HookEventType::ToolExecuteAfter,
        HookEvent::SessionEvent { .. } => HookEventType::SessionEvent,
        HookEvent::MessagesTransform { .. } => HookEventType::MessagesTransform,
    }
}

/// 快捷方法：执行钩子链
pub async fn execute_chain(
    registry: Arc<RwLock<HookRegistry>>,
    event: HookEvent,
    context: HookContext,
) -> Result<HookOutput, HookError> {
    let executor = HookExecutor::new(registry);
    executor.execute(event, context).await
}
```

### 4.4 内置钩子工厂示例

```rust
// src/hook/builtin.rs

use super::*;

/// 创建模型回退钩子
/// 
/// 对应 TypeScript: createModelFallbackHook (hooks/model-fallback/hook.ts)
/// 
/// 当模型错误发生时，自动切换到备选模型
pub fn create_model_fallback_hook(
    config: ModelFallbackConfig,
) -> impl Hook {
    ModelFallbackHook {
        id: HookId::new("model-fallback"),
        config,
    }
}

struct ModelFallbackHook {
    id: HookId,
    config: ModelFallbackConfig,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelFallbackConfig {
    pub enabled: bool,
    pub toast_on_fallback: bool,
}

impl Hook for ModelFallbackHook {
    fn id(&self) -> &HookId {
        &self.id
    }
    
    fn priority(&self) -> HookPriority {
        HookPriority::OnAfter
    }
    
    fn events(&self) -> Vec<HookEventType> {
        vec![HookEventType::ChatMessage, HookEventType::SessionEvent]
    }
    
    async fn execute(&self, ctx: &mut HookContext) -> HookResult {
        match &ctx.event {
            HookEvent::ChatMessage { session_id, .. } => {
                // 检查是否有待处理的回退
                // 如果有，修改 output 中的 model 字段
                HookResult::Continue
            }
            HookEvent::SessionEvent { event_type, .. } => {
                if *event_type == SessionEventType::Error {
                    // 处理会话错误，设置回退状态
                }
                HookResult::Continue
            }
            _ => HookResult::Continue,
        }
    }
}

/// 创建会话恢复钩子
/// 
/// 对应 TypeScript: createSessionRecoveryHook (hooks/session-recovery/hook.ts)
/// 
/// 自动从可恢复错误中恢复
pub fn create_session_recovery_hook() -> impl Hook {
    SessionRecoveryHook {
        id: HookId::new("session-recovery"),
    }
}

struct SessionRecoveryHook {
    id: HookId,
}

impl Hook for SessionRecoveryHook {
    fn id(&self) -> &HookId {
        &self.id
    }
    
    fn priority(&self) -> HookPriority {
        HookPriority::OnError
    }
    
    fn events(&self) -> Vec<HookEventType> {
        vec![HookEventType::SessionEvent]
    }
    
    async fn execute(&self, ctx: &mut HookContext) -> HookResult {
        // 检测错误类型
        // 执行恢复逻辑
        HookResult::Continue
    }
}

/// 注册所有内置钩子
pub fn register_all(registry: &mut HookRegistry, config: &HookConfig) {
    // 注册内置钩子
    let hooks: Vec<Box<dyn Hook>> = vec![
        Box::new(ModelFallbackHook { id: HookId::new("model-fallback"), config: ModelFallbackConfig::default() }),
        Box::new(SessionRecoveryHook { id: HookId::new("session-recovery") }),
        // ... 其他内置钩子
    ];
    
    for hook in hooks {
        // 检查是否被禁用
        if !config.disabled_hooks.contains(&hook.id().0) {
            if let Err(e) = registry.register(hook) {
                tracing::warn!("Failed to register hook: {}", e);
            }
        }
    }
}
```

---

## 5. 内置钩子列表 (Built-in Hooks)

### 5.1 按层级分类

| 层级 | 钩子数量 | 用途 | TypeScript 参考 |
|------|---------|------|-----------------|
| Session | 23 | 会话生命周期管理 | `create-session-hooks.ts` |
| Tool Guard | 10 | 工具执行拦截和修改 | `create-tool-guard-hooks.ts` |
| Transform | 4 | 消息内容转换 | `create-transform-hooks.ts` |
| Continuation | 7 | 任务续行和状态恢复 | `create-continuation-hooks.ts` |
| Skill | 2 | Skill 系统集成 | `create-skill-hooks.ts` |

### 5.2 核心 MVP 钩子

| 钩子名称 | 事件类型 | 优先级 | 功能 |
|----------|---------|--------|------|
| `model-fallback` | ChatMessage, SessionEvent | OnAfter | 模型自动回退 |
| `session-recovery` | SessionEvent | OnError | 会话错误恢复 |
| `context-window-monitor` | SessionEvent | OnAfter | 上下文窗口监控 |
| `think-mode` | ChatParams | OnBefore | 思考模式切换 |
| `comment-checker` | ToolExecuteAfter | OnAfter | AI 注释检测 |
| `rules-injector` | ToolExecuteBefore | OnBefore | 规则注入 |
| `compaction-context-injector` | SessionEvent | OnAfter | 压缩上下文注入 |

### 5.3 钩子执行顺序

```
事件触发
    │
    ├─→ [OnBefore] 钩子 (验证、预处理)
    │       │
    │       └─→ 可阻止执行 (返回 Skip)
    │
    ├─→ [OnInstead] 钩子 (替代操作)
    │       │
    │       └─→ 可替代原始操作 (返回 Skip with output)
    │
    ├─→ [原始操作]
    │
    ├─→ [OnAfter] 钩子 (后处理)
    │
    ├─→ 成功 ─→ [OnSuccess] 钩子
    │
    └─→ 失败 ─→ [OnError] 钩子
```

---

## 6. TypeScript 版本对照 (TypeScript Migration Mapping)

### 6.1 核心功能映射

| 功能 | TypeScript 位置 | Rust 目标文件 | 优先级 |
|------|-----------------|---------------|--------|
| 钩子定义 | `hooks/*/hook.ts` | `builtin.rs` | MVP |
| 钩子注册 | `create-hooks.ts` | `registry.rs` | MVP |
| 钩子执行 | `plugin/chat-message.ts` 等 | `executor.rs` | MVP |
| 钩子配置 | `config/schema/hooks.ts` | `types.rs` | MVP |
| 安全执行 | `shared/safe-create-hook.ts` | `executor.rs` | MVP |
| 钩子类型 | `plugin/types.ts` | `types.rs` | MVP |

### 6.2 数据结构映射

| TypeScript Schema | Rust Struct | 字段数 | 优先级 |
|-------------------|-------------|--------|--------|
| `HookName` | `HookId` / `BuiltinHookName` | 46 枚举值 | MVP |
| `PluginInterface` | `Hook` trait | 8 handler 方法 | MVP |
| `ChatMessageInput` | `HookContext` | 3 | MVP |
| `ChatMessageHandlerOutput` | `HookOutput` | 6 | MVP |
| `HookConfig` | `HookConfig` | 2 | MVP |

### 6.3 事件类型映射

| TypeScript Handler | Rust Event | 说明 |
|-------------------|------------|------|
| `config` | ToolRegister | 配置加载 |
| `tool` | ToolRegister | 工具注册 |
| `chat.message` | ChatMessage | 消息处理 |
| `chat.params` | ChatParams | 参数修改 |
| `chat.headers` | ChatHeaders | 请求头修改 |
| `event` | SessionEvent | 会话事件 |
| `tool.execute.before` | ToolExecuteBefore | 工具执行前 |
| `tool.execute.after` | ToolExecuteAfter | 工具执行后 |
| `experimental.chat.messages.transform` | MessagesTransform | 消息转换 |

---

## 7. 测试策略 (Test Strategy)

### 7.1 单元测试

**测试文件:** `src/hook/*.rs` (内联 `#[cfg(test)]` 模块)

#### 7.1.1 钩子注册测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hook_registration() {
        let mut registry = HookRegistry::new();
        let hook = TestHook::new("test-hook");
        
        assert!(registry.register(Box::new(hook)).is_ok());
        assert!(registry.get_hooks_for_event(HookEventType::ChatMessage).len() == 1);
    }
    
    #[test]
    fn test_duplicate_registration() {
        let mut registry = HookRegistry::new();
        let hook1 = TestHook::new("same-id");
        let hook2 = TestHook::new("same-id");
        
        registry.register(Box::new(hook1)).unwrap();
        assert!(matches!(
            registry.register(Box::new(hook2)),
            Err(HookError::AlreadyRegistered(_))
        ));
    }
    
    #[test]
    fn test_hook_unregistration() {
        let mut registry = HookRegistry::new();
        let hook = TestHook::new("test-hook");
        
        registry.register(Box::new(hook)).unwrap();
        assert!(registry.unregister(&HookId::new("test-hook")).is_ok());
        assert!(registry.get_hooks_for_event(HookEventType::ChatMessage).is_empty());
    }
    
    #[test]
    fn test_hook_disable() {
        let mut registry = HookRegistry::new();
        registry.disable("test-hook");
        
        assert!(!registry.is_enabled(&HookId::new("test-hook")));
    }
}
```

#### 7.1.2 钩子执行测试

```rust
#[tokio::test]
async fn test_execute_chain_continue() {
    let registry = Arc::new(RwLock::new(HookRegistry::new()));
    registry.write().await.register(Box::new(ContinueHook::new())).unwrap();
    
    let executor = HookExecutor::new(registry.clone());
    let event = HookEvent::ChatMessage {
        session_id: "test".to_string(),
        agent: None,
        model: None,
    };
    let context = HookContext::new(event.clone());
    
    let result = executor.execute(event, context).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_execute_chain_skip() {
    let registry = Arc::new(RwLock::new(HookRegistry::new()));
    registry.write().await.register(Box::new(SkipHook::new())).unwrap();
    
    let executor = HookExecutor::new(registry.clone());
    let event = HookEvent::ChatMessage {
        session_id: "test".to_string(),
        agent: None,
        model: None,
    };
    let context = HookContext::new(event.clone());
    
    let result = executor.execute(event, context).await;
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.message.is_some());
}

#[tokio::test]
async fn test_error_isolation() {
    let registry = Arc::new(RwLock::new(HookRegistry::new()));
    registry.write().await.register(Box::new(ErrorHook::new())).unwrap();
    registry.write().await.register(Box::new(ContinueHook::new())).unwrap();
    
    let executor = HookExecutor::new(registry.clone()).with_error_isolation(true);
    let event = HookEvent::ChatMessage {
        session_id: "test".to_string(),
        agent: None,
        model: None,
    };
    let context = HookContext::new(event.clone());
    
    // 错误钩子失败，但继续执行
    let result = executor.execute(event, context).await;
    assert!(result.is_ok());
}
```

#### 7.1.3 优先级测试

```rust
#[test]
fn test_priority_ordering() {
    let mut registry = HookRegistry::new();
    
    // 按 3-1-2 顺序注册
    registry.register(Box::new(PriorityHook::new("h3", HookPriority::OnAfter))).unwrap();
    registry.register(Box::new(PriorityHook::new("h1", HookPriority::OnBefore))).unwrap();
    registry.register(Box::new(PriorityHook::new("h2", HookPriority::OnInstead))).unwrap();
    
    let hooks = registry.get_hooks_for_event(HookEventType::ChatMessage);
    
    // 应按优先级排序: OnBefore(1) < OnInstead(2) < OnAfter(3)
    assert_eq!(hooks[0].hook.id().as_str(), "h1");
    assert_eq!(hooks[1].hook.id().as_str(), "h2");
    assert_eq!(hooks[2].hook.id().as_str(), "h3");
}
```

### 7.2 集成测试

```rust
// tests/hook/test_integration.rs

#[tokio::test]
async fn test_full_hook_flow() {
    // 创建注册中心
    let registry = Arc::new(RwLock::new(HookRegistry::new()));
    
    // 注册多个钩子
    registry.write().await.register(Box::new(PreValidationHook::new())).unwrap();
    registry.write().await.register(Box::new(ModificationHook::new())).unwrap();
    registry.write().await.register(Box::new(PostProcessingHook::new())).unwrap();
    
    // 创建执行器
    let executor = HookExecutor::new(registry.clone());
    
    // 触发事件
    let event = HookEvent::ToolExecuteBefore {
        session_id: "test-session".to_string(),
        tool_name: "write".to_string(),
        tool_input: serde_json::json!({ "file_path": "/test.txt" }),
    };
    
    let context = HookContext::new(event.clone());
    let result = executor.execute(event, context).await;
    
    assert!(result.is_ok());
}
```

### 7.3 测试覆盖要求

| 模块 | 覆盖率要求 | 关键测试场景 |
|------|-----------|--------------|
| `types.rs` | 85% | 所有 struct 序列化/反序列化 |
| `registry.rs` | 90% | 注册、注销、禁用、查询 |
| `executor.rs` | 95% | 执行链、优先级、错误隔离、超时 |
| `builtin.rs` | 80% | 内置钩子基本功能 |

---

## 附录 A: 依赖项清单

### 必须依赖 (MVP)

```toml
[dependencies]
# 异步运行时
tokio = { version = "1", features = ["full"] }

# 序列化
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# 错误处理
thiserror = "2"

# 时间处理
chrono = { version = "0.4", features = ["serde"] }

# 日志
tracing = "0.1"
```

### 可选依赖 (Future)

```toml
[dependencies]
# 性能监控
metrics = "0.23"

# 异步 trait
async-trait = "0.1"
```

---

## 附录 B: 错误类型定义

```rust
// src/hook/error.rs

/// 钩子错误
#[derive(Debug, thiserror::Error)]
pub enum HookError {
    #[error("钩子已注册: {0}")]
    AlreadyRegistered(HookId),
    
    #[error("钩子未找到: {0}")]
    NotFound(HookId),
    
    #[error("钩子执行超时: {0}")]
    Timeout(HookId),
    
    #[error("钩子执行失败: {0}")]
    ExecutionFailed(String),
    
    #[error("无效的钩子配置: {0}")]
    InvalidConfig(String),
    
    #[error("钩子被禁用: {0}")]
    Disabled(HookId),
    
    #[error("无效的事件类型")]
    InvalidEventType,
}

/// 执行错误
#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("执行被中断: {0}")]
    Interrupted(String),
    
    #[error("执行结果类型不匹配")]
    TypeMismatch,
    
    #[error("上下文缺失: {0}")]
    MissingContext(String),
}
```

---

## 附录 C: 实现检查清单

### Phase 1: 基础架构

- [ ] 创建所有模块文件框架
- [ ] 定义错误类型 (`error.rs`)
- [ ] 实现基础类型 (`types.rs`)
- [ ] 实现 Hook trait
- [ ] 添加单元测试框架

### Phase 2: 核心功能

- [ ] 实现 HookRegistry (`registry.rs`)
- [ ] 实现 HookContext (`context.rs`)
- [ ] 实现 HookExecutor (`executor.rs`)
- [ ] 实现优先级排序
- [ ] 实现错误隔离

### Phase 3: 内置钩子

- [ ] 实现 model-fallback 钩子
- [ ] 实现 session-recovery 钩子
- [ ] 实现 context-window-monitor 钩子
- [ ] 实现 comment-checker 钩子
- [ ] 实现 rules-injector 钩子

### Phase 4: 集成与测试

- [ ] 与 Agent 系统集成
- [ ] 与 Tool 系统集成
- [ ] 与 Session 系统集成
- [ ] 完成所有单元测试
- [ ] 完成集成测试

---

**文档维护:** 此文档应随代码实现同步更新，所有 API 变更需在此文档中反映。