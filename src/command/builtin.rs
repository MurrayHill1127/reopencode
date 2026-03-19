//! 内置命令定义模块
//!
//! 包含所有 ROC 内置斜杠命令的定义和元数据。
//! 对应 TypeScript: features/builtin-commands/commands.ts

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

/// 内置命令名称枚举
/// 对应 TypeScript: BuiltinCommandName (config/schema/commands.ts:3-11)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[allow(dead_code)]
pub enum BuiltinCommandName {
    /// 初始化分层 AGENTS.md 知识库
    InitDeep,

    /// 启动自引用开发循环
    RalphLoop,

    /// 启动 Ultrawork 循环
    UlwLoop,

    /// 取消 Ralph Loop
    CancelRalph,

    /// 智能重构命令
    Refactor,

    /// 从 Prometheus 计划开始工作
    StartWork,

    /// 停止所有延续机制
    StopContinuation,

    /// 创建会话交接摘要
    Handoff,
}

impl BuiltinCommandName {
    /// 获取命令描述
    pub fn description(&self) -> &'static str {
        match self {
            Self::InitDeep => "(builtin) Initialize hierarchical AGENTS.md knowledge base",
            Self::RalphLoop => "(builtin) Start self-referential development loop until completion",
            Self::UlwLoop => {
                "(builtin) Start ultrawork loop - continues until completion with ultrawork mode"
            }
            Self::CancelRalph => "(builtin) Cancel active Ralph Loop",
            Self::Refactor => {
                "(builtin) Intelligent refactoring command with LSP, AST-grep, architecture analysis, codemap, and TDD verification."
            }
            Self::StartWork => "(builtin) Start Sisyphus work session from Prometheus plan",
            Self::StopContinuation => {
                "(builtin) Stop all continuation mechanisms (ralph loop, todo continuation, boulder) for this session"
            }
            Self::Handoff => {
                "(builtin) Create a detailed context summary for continuing work in a new session"
            }
        }
    }

    /// 获取参数提示
    pub fn argument_hint(&self) -> Option<&'static str> {
        match self {
            Self::InitDeep => Some("[--create-new] [--max-depth=N]"),
            Self::RalphLoop => Some(
                "\"task description\" [--completion-promise=TEXT] [--max-iterations=N] [--strategy=reset|continue]",
            ),
            Self::UlwLoop => {
                Some("\"task description\" [--completion-promise=TEXT] [--strategy=reset|continue]")
            }
            Self::Refactor => Some(
                "<refactoring-target> [--scope=<file|module|project>] [--strategy=<safe|aggressive>]",
            ),
            Self::StartWork => Some("[plan-name]"),
            Self::Handoff => Some("[goal]"),
            _ => None,
        }
    }

    /// 获取 kebab-case 字符串表示
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InitDeep => "init-deep",
            Self::RalphLoop => "ralph-loop",
            Self::UlwLoop => "ulw-loop",
            Self::CancelRalph => "cancel-ralph",
            Self::Refactor => "refactor",
            Self::StartWork => "start-work",
            Self::StopContinuation => "stop-continuation",
            Self::Handoff => "handoff",
        }
    }

    /// 遍历所有内置命令变体
    pub fn iter() -> impl Iterator<Item = Self> {
        [
            Self::InitDeep,
            Self::RalphLoop,
            Self::UlwLoop,
            Self::CancelRalph,
            Self::Refactor,
            Self::StartWork,
            Self::StopContinuation,
            Self::Handoff,
        ]
        .into_iter()
    }
}

impl fmt::Display for BuiltinCommandName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for BuiltinCommandName {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "init-deep" => Ok(Self::InitDeep),
            "ralph-loop" => Ok(Self::RalphLoop),
            "ulw-loop" => Ok(Self::UlwLoop),
            "cancel-ralph" => Ok(Self::CancelRalph),
            "refactor" => Ok(Self::Refactor),
            "start-work" => Ok(Self::StartWork),
            "stop-continuation" => Ok(Self::StopContinuation),
            "handoff" => Ok(Self::Handoff),
            _ => Err(format!("Unknown builtin command: {}", s)),
        }
    }
}

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
        }
    }

    /// 设置描述
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// 设置参数提示
    pub fn with_argument_hint(mut self, hint: impl Into<String>) -> Self {
        self.argument_hint = Some(hint.into());
        self
    }

    /// 设置执行 Agent
    pub fn with_agent(mut self, agent: impl Into<String>) -> Self {
        self.agent = Some(agent.into());
        self
    }
}

lazy_static! {
    /// 内置命令集合
    /// 包含所有 ROC 内置斜杠命令的定义
    pub static ref BUILTIN_COMMANDS: HashMap<String, CommandDefinition> = {
        let mut m = HashMap::new();

        m.insert("init-deep".to_string(), CommandDefinition::new(
            "init-deep",
            "Initialize hierarchical AGENTS.md knowledge base structure in the project."
        )
        .with_description("Initialize hierarchical AGENTS.md knowledge base")
        .with_argument_hint("[--create-new] [--max-depth=N]")
        .with_agent("atlas"));

        m.insert("ralph-loop".to_string(), CommandDefinition::new(
            "ralph-loop",
            "Start self-referential development loop that continues until task completion."
        )
        .with_description("Start self-referential development loop until completion")
        .with_argument_hint("\"task description\" [--completion-promise=TEXT] [--max-iterations=N] [--strategy=reset|continue]")
        .with_agent("sisyphus"));

        m.insert("ulw-loop".to_string(), CommandDefinition::new(
            "ulw-loop",
            "Start ultrawork loop mode - continues until completion with maximum effort."
        )
        .with_description("Start ultrawork loop - continues until completion with ultrawork mode")
        .with_argument_hint("\"task description\" [--completion-promise=TEXT] [--strategy=reset|continue]")
        .with_agent("sisyphus"));

        m.insert("cancel-ralph".to_string(), CommandDefinition::new(
            "cancel-ralph",
            "Cancel any active Ralph Loop in the current session."
        )
        .with_description("Cancel active Ralph Loop"));

        m.insert("refactor".to_string(), CommandDefinition::new(
            "refactor",
            "Perform intelligent refactoring with LSP, AST-grep, and architecture analysis."
        )
        .with_description("Intelligent refactoring command with LSP, AST-grep, architecture analysis, codemap, and TDD verification.")
        .with_argument_hint("<refactoring-target> [--scope=<file|module|project>] [--strategy=<safe|aggressive>]")
        .with_agent("atlas"));

        m.insert("start-work".to_string(), CommandDefinition::new(
            "start-work",
            "Start a Sisyphus work session from a Prometheus plan."
        )
        .with_description("Start Sisyphus work session from Prometheus plan")
        .with_argument_hint("[plan-name]")
        .with_agent("atlas"));

        m.insert("stop-continuation".to_string(), CommandDefinition::new(
            "stop-continuation",
            "Stop all continuation mechanisms including ralph loop, todo continuation, and boulder."
        )
        .with_description("Stop all continuation mechanisms (ralph loop, todo continuation, boulder) for this session"));

        m.insert("handoff".to_string(), CommandDefinition::new(
            "handoff",
            "Create a detailed context summary for continuing work in a new session."
        )
        .with_description("Create a detailed context summary for continuing work in a new session")
        .with_argument_hint("[goal]")
        .with_agent("atlas"));

        m
    };
}

/// 获取内置命令定义
pub fn get_builtin_command(name: &str) -> Option<&'static CommandDefinition> {
    BUILTIN_COMMANDS.get(name)
}

/// 检查命令是否为内置命令
pub fn is_builtin_command(name: &str) -> bool {
    BUILTIN_COMMANDS.contains_key(name)
}

/// 获取所有内置命令名称
pub fn builtin_command_names() -> Vec<&'static str> {
    vec![
        "init-deep",
        "ralph-loop",
        "ulw-loop",
        "cancel-ralph",
        "refactor",
        "start-work",
        "stop-continuation",
        "handoff",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_command_name_as_str() {
        assert_eq!(BuiltinCommandName::InitDeep.as_str(), "init-deep");
        assert_eq!(BuiltinCommandName::RalphLoop.as_str(), "ralph-loop");
        assert_eq!(BuiltinCommandName::UlwLoop.as_str(), "ulw-loop");
        assert_eq!(BuiltinCommandName::CancelRalph.as_str(), "cancel-ralph");
        assert_eq!(BuiltinCommandName::Refactor.as_str(), "refactor");
        assert_eq!(BuiltinCommandName::StartWork.as_str(), "start-work");
        assert_eq!(
            BuiltinCommandName::StopContinuation.as_str(),
            "stop-continuation"
        );
        assert_eq!(BuiltinCommandName::Handoff.as_str(), "handoff");
    }

    #[test]
    fn test_builtin_command_name_from_str() {
        assert_eq!(
            BuiltinCommandName::from_str("init-deep").unwrap(),
            BuiltinCommandName::InitDeep
        );
        assert_eq!(
            BuiltinCommandName::from_str("ralph-loop").unwrap(),
            BuiltinCommandName::RalphLoop
        );
        assert!(BuiltinCommandName::from_str("unknown").is_err());
    }

    #[test]
    fn test_builtin_command_name_display() {
        assert_eq!(format!("{}", BuiltinCommandName::InitDeep), "init-deep");
        assert_eq!(format!("{}", BuiltinCommandName::RalphLoop), "ralph-loop");
    }

    #[test]
    fn test_builtin_command_name_iter() {
        let commands: Vec<_> = BuiltinCommandName::iter().collect();
        assert_eq!(commands.len(), 8);
        assert!(commands.contains(&BuiltinCommandName::InitDeep));
        assert!(commands.contains(&BuiltinCommandName::Handoff));
    }

    #[test]
    fn test_builtin_command_name_description() {
        assert!(BuiltinCommandName::InitDeep
            .description()
            .contains("Initialize"));
        assert!(BuiltinCommandName::RalphLoop
            .description()
            .contains("self-referential"));
        assert!(BuiltinCommandName::Handoff
            .description()
            .contains("context summary"));
    }

    #[test]
    fn test_builtin_command_name_argument_hint() {
        assert!(BuiltinCommandName::InitDeep.argument_hint().is_some());
        assert!(BuiltinCommandName::CancelRalph.argument_hint().is_none());
        assert!(BuiltinCommandName::StopContinuation
            .argument_hint()
            .is_none());
    }

    #[test]
    fn test_builtin_commands_exist() {
        assert!(BUILTIN_COMMANDS.contains_key("init-deep"));
        assert!(BUILTIN_COMMANDS.contains_key("ralph-loop"));
        assert!(BUILTIN_COMMANDS.contains_key("ulw-loop"));
        assert!(BUILTIN_COMMANDS.contains_key("cancel-ralph"));
        assert!(BUILTIN_COMMANDS.contains_key("refactor"));
        assert!(BUILTIN_COMMANDS.contains_key("start-work"));
        assert!(BUILTIN_COMMANDS.contains_key("stop-continuation"));
        assert!(BUILTIN_COMMANDS.contains_key("handoff"));
    }

    #[test]
    fn test_builtin_commands_count() {
        assert_eq!(BUILTIN_COMMANDS.len(), 8);
    }

    #[test]
    fn test_get_builtin_command() {
        let cmd = get_builtin_command("init-deep").unwrap();
        assert_eq!(cmd.name, "init-deep");
        assert!(cmd.description.as_ref().unwrap().contains("Initialize"));

        assert!(get_builtin_command("unknown").is_none());
    }

    #[test]
    fn test_is_builtin_command() {
        assert!(is_builtin_command("init-deep"));
        assert!(is_builtin_command("ralph-loop"));
        assert!(!is_builtin_command("unknown"));
        assert!(!is_builtin_command("custom-command"));
    }

    #[test]
    fn test_builtin_command_names() {
        let names = builtin_command_names();
        assert_eq!(names.len(), 8);
        assert!(names.contains(&"init-deep"));
        assert!(names.contains(&"handoff"));
    }

    #[test]
    fn test_builtin_commands_have_agents_where_expected() {
        // init-deep, ralph-loop, ulw-loop, refactor, start-work, handoff have agents
        assert!(get_builtin_command("init-deep").unwrap().agent.is_some());
        assert!(get_builtin_command("ralph-loop").unwrap().agent.is_some());
        assert!(get_builtin_command("ulw-loop").unwrap().agent.is_some());
        assert!(get_builtin_command("refactor").unwrap().agent.is_some());
        assert!(get_builtin_command("start-work").unwrap().agent.is_some());
        assert!(get_builtin_command("handoff").unwrap().agent.is_some());

        // cancel-ralph and stop-continuation don't have agents
        assert!(get_builtin_command("cancel-ralph").unwrap().agent.is_none());
        assert!(get_builtin_command("stop-continuation")
            .unwrap()
            .agent
            .is_none());
    }
}
