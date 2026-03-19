//! Command 模块 - 斜杠命令系统核心
//!
//! 提供命令注册、发现、解析和模板渲染功能：
//! - 内置命令 (`init-deep`, `ralph-loop`, `ulw-loop`, 等)
//! - 多源命令发现 (用户、项目、插件)
//! - 模板变量替换 (`$ARGUMENTS`, `$SESSION_ID`, `$TIMESTAMP`)
//! - 命令优先级管理 (项目 > 用户 > OpenCode > 内置 > 插件)

mod builtin;
mod discovery;
mod error;
mod parser;
mod registry;
mod template;
mod types;

pub use error::{CommandError, ParseError, RenderError};

pub use types::{
    CommandDefinition, CommandInfo, CommandMetadata, CommandParseResult, CommandScope,
    HandoffDefinition,
};

pub use registry::CommandRegistry;

pub use discovery::{CommandDiscovery, DiscoveryOptions, discover_commands};

pub use parser::{find_commands_in_text, parse_slash_command};

pub use template::{TemplateContext, render_template};

pub use builtin::{
    BUILTIN_COMMANDS, BuiltinCommandName, builtin_command_names, get_builtin_command,
    is_builtin_command,
};

/// 快捷方法：解析并执行斜杠命令
///
/// # Example
/// ```no_run
/// use reopencode::command::{parse_slash_command, CommandRegistry};
///
/// let registry = CommandRegistry::new();
/// if let Some(result) = parse_slash_command("/init-deep --create-new") {
///     if let Some(cmd) = registry.get(&result.command_name) {
///         let rendered = cmd.render(&result.arguments, Default::default());
///         println!("{}", rendered);
///     }
/// }
/// ```
pub fn parse_and_execute(input: &str, registry: &CommandRegistry) -> Option<String> {
    let result = parse_slash_command(input)?;
    let cmd = registry.get(&result.command_name)?;
    Some(cmd.render(&result.arguments, std::collections::HashMap::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_exports_accessible() {
        let _cmd_err = CommandError::not_found("test");
        let _parse_err = ParseError::EmptyName;
        let _render_err = RenderError::missing_variable("var");

        let _def = CommandDefinition::new("test", "template");
        let _meta = CommandMetadata::new("test");
        let _info = CommandInfo::new("test", CommandMetadata::new("test"));
        let _scope = CommandScope::Builtin;
        let _builtin = BuiltinCommandName::InitDeep;
        let _handoff = HandoffDefinition::new("label", "agent", "prompt");
        let _parse_result = CommandParseResult::new("/test", "test", 0, 5);

        let _registry = CommandRegistry::new();

        let _ = parse_slash_command("/test");
        let _ = find_commands_in_text("text");

        let _ctx = TemplateContext::new();
        let _ = render_template("$ARGUMENTS", Some("test"), &_ctx);

        let _ = get_builtin_command("init-deep");
        let _ = is_builtin_command("init-deep");
        let _ = builtin_command_names();

        let registry = CommandRegistry::new();
        let _ = parse_and_execute("/test", &registry);
    }
}
