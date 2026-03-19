//! 命令模块核心数据结构
//!
//! 定义命令相关的所有类型，包括命令定义、元数据、作用域等。

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

/// 命令定义
/// 对应 TypeScript: CommandDefinition (features/claude-code-command-loader/types.ts:19-29)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[allow(dead_code)]
pub struct CommandDefinition {
    /// 命令名称 (不含 /)
    pub name: String,

    /// 命令描述 (显示在命令列表中)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// 命令模板内容
    pub template: String,

    /// 指定执行的 Agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,

    /// 指定使用的模型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// 是否作为子任务执行
    #[serde(default)]
    pub subtask: bool,

    /// 参数提示 (显示在自动补全中)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub argument_hint: Option<String>,

    /// Handoff 工作流定义
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handoffs: Option<Vec<HandoffDefinition>>,
}

impl CommandDefinition {
    /// 创建一个新的命令定义
    pub fn new(name: impl Into<String>, template: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            template: template.into(),
            agent: None,
            model: None,
            subtask: false,
            argument_hint: None,
            handoffs: None,
        }
    }

    /// 添加描述
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// 添加 Agent
    pub fn with_agent(mut self, agent: impl Into<String>) -> Self {
        self.agent = Some(agent.into());
        self
    }

    /// 添加模型
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// 设置为子任务
    pub fn with_subtask(mut self, subtask: bool) -> Self {
        self.subtask = subtask;
        self
    }

    /// 添加参数提示
    pub fn with_argument_hint(mut self, hint: impl Into<String>) -> Self {
        self.argument_hint = Some(hint.into());
        self
    }

    /// 添加 Handoff 定义
    pub fn with_handoffs(mut self, handoffs: Vec<HandoffDefinition>) -> Self {
        self.handoffs = Some(handoffs);
        self
    }
}

/// Handoff 定义
/// 对应 TypeScript: HandoffDefinition (features/claude-code-command-loader/types.ts:8-17)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[allow(dead_code)]
pub struct HandoffDefinition {
    /// 标签
    pub label: String,

    /// 目标 Agent
    pub agent: String,

    /// 预填充提示词
    pub prompt: String,

    /// 是否自动发送
    #[serde(default)]
    pub send: bool,
}

impl HandoffDefinition {
    /// 创建一个新的 Handoff 定义
    pub fn new(
        label: impl Into<String>,
        agent: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            agent: agent.into(),
            prompt: prompt.into(),
            send: false,
        }
    }

    /// 设置自动发送
    pub fn with_send(mut self, send: bool) -> Self {
        self.send = send;
        self
    }
}

/// 命令元数据
/// 对应 TypeScript: CommandMetadata (tools/slashcommand/types.ts:5-12)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[allow(dead_code)]
pub struct CommandMetadata {
    /// 命令名称
    pub name: String,

    /// 命令描述
    #[serde(default)]
    pub description: String,

    /// 参数提示
    #[serde(skip_serializing_if = "Option::is_none")]
    pub argument_hint: Option<String>,

    /// 指定模型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// 指定 Agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,

    /// 是否为子任务
    #[serde(default)]
    pub subtask: bool,
}

impl CommandMetadata {
    /// 创建一个新的命令元数据
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            argument_hint: None,
            model: None,
            agent: None,
            subtask: false,
        }
    }

    /// 添加描述
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// 添加参数提示
    pub fn with_argument_hint(mut self, hint: impl Into<String>) -> Self {
        self.argument_hint = Some(hint.into());
        self
    }

    /// 添加模型
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// 添加 Agent
    pub fn with_agent(mut self, agent: impl Into<String>) -> Self {
        self.agent = Some(agent.into());
        self
    }

    /// 设置为子任务
    pub fn with_subtask(mut self, subtask: bool) -> Self {
        self.subtask = subtask;
        self
    }

    /// 从 CommandDefinition 创建元数据
    pub fn from_definition(def: &CommandDefinition) -> Self {
        Self {
            name: def.name.clone(),
            description: def.description.clone().unwrap_or_default(),
            argument_hint: def.argument_hint.clone(),
            model: def.model.clone(),
            agent: def.agent.clone(),
            subtask: def.subtask,
        }
    }
}

/// 命令来源作用域
/// 对应 TypeScript: CommandScope (tools/slashcommand/types.ts:3)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
#[allow(dead_code)]
pub enum CommandScope {
    /// 内置命令
    #[default]
    Builtin,
    /// 配置文件定义
    Config,
    /// 用户全局命令 (~/.config/roc/commands/)
    User,
    /// 项目命令 (.roc/commands/)
    Project,
    /// OpenCode 全局命令
    Opencode,
    /// OpenCode 项目命令
    OpencodeProject,
    /// 插件命令
    Plugin,
}

impl CommandScope {
    /// 获取字符串表示
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::Config => "config",
            Self::User => "user",
            Self::Project => "project",
            Self::Opencode => "opencode",
            Self::OpencodeProject => "opencode-project",
            Self::Plugin => "plugin",
        }
    }

    /// 获取显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Builtin => "Built-in",
            Self::Config => "Config",
            Self::User => "User",
            Self::Project => "Project",
            Self::Opencode => "OpenCode",
            Self::OpencodeProject => "OpenCode Project",
            Self::Plugin => "Plugin",
        }
    }

    /// 是否为用户定义作用域
    pub fn is_user_defined(&self) -> bool {
        matches!(
            self,
            Self::User | Self::Project | Self::Opencode | Self::OpencodeProject
        )
    }

    /// 是否为项目级作用域
    pub fn is_project_scope(&self) -> bool {
        matches!(self, Self::Project | Self::OpencodeProject)
    }

    /// 遍历所有作用域
    pub fn iter() -> impl Iterator<Item = Self> {
        [
            Self::Builtin,
            Self::Config,
            Self::User,
            Self::Project,
            Self::Opencode,
            Self::OpencodeProject,
            Self::Plugin,
        ]
        .into_iter()
    }
}

impl fmt::Display for CommandScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for CommandScope {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "builtin" => Ok(Self::Builtin),
            "config" => Ok(Self::Config),
            "user" => Ok(Self::User),
            "project" => Ok(Self::Project),
            "opencode" => Ok(Self::Opencode),
            "opencode-project" => Ok(Self::OpencodeProject),
            "plugin" => Ok(Self::Plugin),
            _ => Err(format!("Unknown command scope: {}", s)),
        }
    }
}

/// 命令完整信息
/// 对应 TypeScript: CommandInfo (tools/slashcommand/types.ts:14-21)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CommandInfo {
    /// 命令名称
    pub name: String,

    /// 命令文件路径 (如果是文件定义)
    pub path: Option<PathBuf>,

    /// 命令元数据
    pub metadata: CommandMetadata,

    /// 命令内容/模板
    pub content: Option<String>,

    /// 命令作用域
    pub scope: CommandScope,
}

impl CommandInfo {
    /// 创建一个新的命令信息
    pub fn new(name: impl Into<String>, metadata: CommandMetadata) -> Self {
        Self {
            name: name.into(),
            path: None,
            metadata,
            content: None,
            scope: CommandScope::Builtin,
        }
    }

    /// 添加路径
    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// 添加内容
    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// 设置作用域
    pub fn with_scope(mut self, scope: CommandScope) -> Self {
        self.scope = scope;
        self
    }

    /// 获取命令描述
    pub fn description(&self) -> &str {
        &self.metadata.description
    }

    /// 检查是否有参数提示
    pub fn has_argument_hint(&self) -> bool {
        self.metadata.argument_hint.is_some()
    }

    /// 获取参数提示
    pub fn argument_hint(&self) -> Option<&str> {
        self.metadata.argument_hint.as_deref()
    }

    /// 检查是否为子任务命令
    pub fn is_subtask(&self) -> bool {
        self.metadata.subtask
    }
}

/// 斜杠命令解析结果
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CommandParseResult {
    /// 完整匹配字符串 (如 "/init-deep --create-new")
    pub full_match: String,

    /// 命令名称 (不含 /)
    pub command_name: String,

    /// 命令参数
    pub arguments: Option<String>,

    /// 匹配起始位置
    pub start: usize,

    /// 匹配结束位置
    pub end: usize,
}

impl CommandParseResult {
    /// 创建一个新的解析结果
    pub fn new(
        full_match: impl Into<String>,
        command_name: impl Into<String>,
        start: usize,
        end: usize,
    ) -> Self {
        Self {
            full_match: full_match.into(),
            command_name: command_name.into(),
            arguments: None,
            start,
            end,
        }
    }

    /// 添加参数
    pub fn with_arguments(mut self, args: impl Into<String>) -> Self {
        self.arguments = Some(args.into());
        self
    }

    /// 获取参数（如果存在）
    pub fn arguments(&self) -> Option<&str> {
        self.arguments.as_deref()
    }

    /// 检查是否有参数
    pub fn has_arguments(&self) -> bool {
        self.arguments.is_some()
    }

    /// 获取匹配长度
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// 检查是否为空匹配
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_definition_basic() {
        let cmd = CommandDefinition::new("test", "This is a {{arg}} test")
            .with_description("A test command")
            .with_agent("test-agent")
            .with_subtask(true);

        assert_eq!(cmd.name, "test");
        assert_eq!(cmd.template, "This is a {{arg}} test");
        assert_eq!(cmd.description, Some("A test command".to_string()));
        assert_eq!(cmd.agent, Some("test-agent".to_string()));
        assert!(cmd.subtask);
        assert!(!cmd.handoffs.is_some());
    }

    #[test]
    fn test_command_definition_serde() {
        let cmd = CommandDefinition::new("init", "Initialize {{name}}")
            .with_description("Initialize a new project")
            .with_argument_hint("project-name");

        let json = serde_json::to_string(&cmd).unwrap();
        let parsed: CommandDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, cmd.name);
        assert_eq!(parsed.template, cmd.template);
        assert_eq!(parsed.description, cmd.description);
        assert_eq!(parsed.argument_hint, cmd.argument_hint);
    }

    #[test]
    fn test_handoff_definition() {
        let handoff = HandoffDefinition::new("Review", "reviewer-agent", "Please review this code")
            .with_send(true);

        assert_eq!(handoff.label, "Review");
        assert_eq!(handoff.agent, "reviewer-agent");
        assert_eq!(handoff.prompt, "Please review this code");
        assert!(handoff.send);
    }

    #[test]
    fn test_command_metadata() {
        let meta = CommandMetadata::new("build")
            .with_description("Build the project")
            .with_model("anthropic/claude-sonnet-4")
            .with_subtask(true);

        assert_eq!(meta.name, "build");
        assert_eq!(meta.description, "Build the project");
        assert_eq!(meta.model, Some("anthropic/claude-sonnet-4".to_string()));
        assert!(meta.subtask);
    }

    #[test]
    fn test_command_metadata_from_definition() {
        let def = CommandDefinition::new("deploy", "Deploy to {{env}}")
            .with_description("Deploy application")
            .with_agent("deploy-agent");

        let meta = CommandMetadata::from_definition(&def);

        assert_eq!(meta.name, "deploy");
        assert_eq!(meta.description, "Deploy application");
        assert_eq!(meta.agent, Some("deploy-agent".to_string()));
        assert!(!meta.subtask);
    }

    #[test]
    fn test_command_scope_as_str() {
        assert_eq!(CommandScope::Builtin.as_str(), "builtin");
        assert_eq!(CommandScope::User.as_str(), "user");
        assert_eq!(CommandScope::Project.as_str(), "project");
        assert_eq!(CommandScope::OpencodeProject.as_str(), "opencode-project");
    }

    #[test]
    fn test_command_scope_from_str() {
        assert_eq!("builtin".parse::<CommandScope>(), Ok(CommandScope::Builtin));
        assert_eq!("config".parse::<CommandScope>(), Ok(CommandScope::Config));
        assert_eq!(
            "opencode-project".parse::<CommandScope>(),
            Ok(CommandScope::OpencodeProject)
        );
        assert!("invalid".parse::<CommandScope>().is_err());
    }

    #[test]
    fn test_command_scope_properties() {
        assert!(CommandScope::User.is_user_defined());
        assert!(CommandScope::Project.is_user_defined());
        assert!(!CommandScope::Builtin.is_user_defined());

        assert!(CommandScope::Project.is_project_scope());
        assert!(CommandScope::OpencodeProject.is_project_scope());
        assert!(!CommandScope::User.is_project_scope());
    }

    #[test]
    fn test_command_scope_display() {
        assert_eq!(CommandScope::Builtin.to_string(), "builtin");
        assert_eq!(CommandScope::Plugin.to_string(), "plugin");
    }

    #[test]
    fn test_command_scope_iter() {
        let scopes: Vec<_> = CommandScope::iter().collect();
        assert_eq!(scopes.len(), 7);
        assert!(scopes.contains(&CommandScope::Builtin));
        assert!(scopes.contains(&CommandScope::Plugin));
    }

    #[test]
    fn test_command_info() {
        let meta = CommandMetadata::new("test-cmd");
        let info = CommandInfo::new("test-cmd", meta)
            .with_path("/path/to/cmd")
            .with_scope(CommandScope::User)
            .with_content("Test content");

        assert_eq!(info.name, "test-cmd");
        assert_eq!(info.scope, CommandScope::User);
        assert!(info.path.is_some());
        assert!(info.content.is_some());
    }

    #[test]
    fn test_command_parse_result() {
        let result =
            CommandParseResult::new("/init --force", "init", 0, 13).with_arguments("--force");

        assert_eq!(result.full_match, "/init --force");
        assert_eq!(result.command_name, "init");
        assert_eq!(result.arguments(), Some("--force"));
        assert!(result.has_arguments());
        assert_eq!(result.start, 0);
        assert_eq!(result.end, 13);
        assert_eq!(result.len(), 13);
    }

    #[test]
    fn test_serde_skip_serializing_if() {
        let cmd = CommandDefinition::new("simple", "Simple command");
        let json = serde_json::to_string(&cmd).unwrap();

        // Optional fields with None should not appear
        assert!(!json.contains("description"));
        assert!(!json.contains("agent"));
        assert!(!json.contains("model"));
        assert!(!json.contains("argument_hint"));
        assert!(!json.contains("handoffs"));

        // But name and template should be present
        assert!(json.contains("\"name\":\"simple\""));
        assert!(json.contains("\"template\":\"Simple command\""));
    }

    #[test]
    fn test_kebab_case_serialization() {
        let cmd = CommandDefinition::new("my-cmd", "Test").with_argument_hint("my-arg");

        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("argument-hint"));
        assert!(!json.contains("argument_hint"));
    }
}
