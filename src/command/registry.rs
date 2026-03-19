use super::types::{CommandDefinition, CommandInfo, CommandScope};
use std::collections::HashMap;

pub type TemplateContext = HashMap<String, String>;

pub struct CommandRegistry {
    commands: HashMap<String, CommandInfo>,
    disabled: Vec<String>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
            disabled: Vec::new(),
        }
    }

    pub fn from_discovery(_discovery: ()) -> Self {
        Self::new()
    }

    pub fn register(&mut self, info: CommandInfo) {
        self.commands.insert(info.name.clone(), info);
    }

    pub fn get(&self, name: &str) -> Option<&CommandInfo> {
        if self.is_disabled(name) {
            return None;
        }
        self.commands.get(name)
    }

    pub fn get_definition(&self, name: &str) -> Option<&CommandDefinition> {
        let _ = name;
        None
    }

    pub fn list(&self) -> Vec<&CommandInfo> {
        self.commands
            .values()
            .filter(|cmd| !self.is_disabled(&cmd.name))
            .collect()
    }

    pub fn list_by_scope(&self, scope: CommandScope) -> Vec<&CommandInfo> {
        self.commands
            .values()
            .filter(|cmd| cmd.scope == scope && !self.is_disabled(&cmd.name))
            .collect()
    }

    pub fn disable(&mut self, name: impl Into<String>) {
        let name = name.into();
        if !self.disabled.contains(&name) {
            self.disabled.push(name);
        }
    }

    pub fn is_disabled(&self, name: &str) -> bool {
        self.disabled.contains(&name.to_string())
    }

    pub fn exists(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandInfo {
    pub fn render(&self, arguments: &Option<String>, context: TemplateContext) -> String {
        let template = self.content.as_deref().unwrap_or("");
        render_template(template, arguments, &context)
    }
}

fn render_template(
    template: &str,
    arguments: &Option<String>,
    context: &TemplateContext,
) -> String {
    let mut result = template.to_string();

    if let Some(args) = arguments {
        result = result.replace("{{arg}}", args);
        result = result.replace("{{args}}", args);
    } else {
        result = result.replace("{{arg}}", "");
        result = result.replace("{{args}}", "");
    }

    for (key, value) in context {
        let placeholder = format!("{{{{{}}}}}", key);
        result = result.replace(&placeholder, value);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::super::types::CommandMetadata;
    use super::*;

    fn create_test_command(name: &str) -> CommandInfo {
        CommandInfo {
            name: name.to_string(),
            path: None,
            metadata: CommandMetadata {
                name: name.to_string(),
                description: format!("Test command {}", name),
                ..Default::default()
            },
            content: Some("test content".to_string()),
            scope: CommandScope::Builtin,
        }
    }

    fn create_test_command_with_scope(name: &str, scope: CommandScope) -> CommandInfo {
        CommandInfo {
            name: name.to_string(),
            path: None,
            metadata: CommandMetadata {
                name: name.to_string(),
                description: format!("Test command {}", name),
                ..Default::default()
            },
            content: Some("test content".to_string()),
            scope,
        }
    }

    #[test]
    fn test_registry_new() {
        let registry = CommandRegistry::new();
        assert!(registry.list().is_empty());
    }

    #[test]
    fn test_registry_default() {
        let registry: CommandRegistry = Default::default();
        assert!(registry.list().is_empty());
    }

    #[test]
    fn test_registry_register_and_exists() {
        let mut registry = CommandRegistry::new();
        let cmd = create_test_command("test");

        assert!(!registry.exists("test"));

        registry.register(cmd);

        assert!(registry.exists("test"));
    }

    #[test]
    fn test_registry_get() {
        let mut registry = CommandRegistry::new();
        let cmd = create_test_command("test");

        registry.register(cmd);

        let retrieved = registry.get("test");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test");
    }

    #[test]
    fn test_registry_get_nonexistent() {
        let registry = CommandRegistry::new();
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_registry_disable() {
        let mut registry = CommandRegistry::new();
        let cmd = create_test_command("test");

        registry.register(cmd);
        assert!(!registry.is_disabled("test"));
        assert!(registry.get("test").is_some());

        registry.disable("test");

        assert!(registry.is_disabled("test"));
        assert!(registry.get("test").is_none());
    }

    #[test]
    fn test_registry_disable_string() {
        let mut registry = CommandRegistry::new();
        let cmd = create_test_command("test");

        registry.register(cmd);
        registry.disable("test".to_string());

        assert!(registry.is_disabled("test"));
    }

    #[test]
    fn test_registry_list() {
        let mut registry = CommandRegistry::new();
        registry.register(create_test_command("cmd1"));
        registry.register(create_test_command("cmd2"));
        registry.register(create_test_command("cmd3"));

        let list = registry.list();
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn test_registry_list_excludes_disabled() {
        let mut registry = CommandRegistry::new();
        registry.register(create_test_command("cmd1"));
        registry.register(create_test_command("cmd2"));
        registry.register(create_test_command("cmd3"));

        registry.disable("cmd2");

        let list = registry.list();
        assert_eq!(list.len(), 2);
        assert!(!list.iter().any(|cmd| cmd.name == "cmd2"));
    }

    #[test]
    fn test_registry_list_by_scope() {
        let mut registry = CommandRegistry::new();
        registry.register(create_test_command_with_scope(
            "builtin1",
            CommandScope::Builtin,
        ));
        registry.register(create_test_command_with_scope("user1", CommandScope::User));
        registry.register(create_test_command_with_scope(
            "builtin2",
            CommandScope::Builtin,
        ));
        registry.register(create_test_command_with_scope(
            "project1",
            CommandScope::Project,
        ));

        let builtin_cmds = registry.list_by_scope(CommandScope::Builtin);
        assert_eq!(builtin_cmds.len(), 2);
        assert!(
            builtin_cmds
                .iter()
                .all(|cmd| cmd.scope == CommandScope::Builtin)
        );

        let user_cmds = registry.list_by_scope(CommandScope::User);
        assert_eq!(user_cmds.len(), 1);
        assert_eq!(user_cmds[0].name, "user1");
    }

    #[test]
    fn test_registry_list_by_scope_excludes_disabled() {
        let mut registry = CommandRegistry::new();
        registry.register(create_test_command_with_scope(
            "builtin1",
            CommandScope::Builtin,
        ));
        registry.register(create_test_command_with_scope(
            "builtin2",
            CommandScope::Builtin,
        ));

        registry.disable("builtin1");

        let builtin_cmds = registry.list_by_scope(CommandScope::Builtin);
        assert_eq!(builtin_cmds.len(), 1);
        assert_eq!(builtin_cmds[0].name, "builtin2");
    }

    #[test]
    fn test_registry_exists_after_disable() {
        let mut registry = CommandRegistry::new();
        let cmd = create_test_command("test");
        registry.register(cmd);

        assert!(registry.exists("test"));
        registry.disable("test");
        assert!(registry.exists("test"));
    }

    #[test]
    fn test_render_template_with_args() {
        let template = "Hello {{arg}}!";
        let args = Some("World".to_string());
        let context: TemplateContext = HashMap::new();

        let result = render_template(template, &args, &context);
        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn test_render_template_with_context() {
        let template = "Session: {{session_id}}, Time: {{timestamp}}";
        let args: Option<String> = None;
        let mut context: TemplateContext = HashMap::new();
        context.insert("session_id".to_string(), "abc123".to_string());
        context.insert("timestamp".to_string(), "1234567890".to_string());

        let result = render_template(template, &args, &context);
        assert_eq!(result, "Session: abc123, Time: 1234567890");
    }

    #[test]
    fn test_render_template_with_both() {
        let template = "{{args}} in session {{session_id}}";
        let args = Some("Run test".to_string());
        let mut context: TemplateContext = HashMap::new();
        context.insert("session_id".to_string(), "xyz789".to_string());

        let result = render_template(template, &args, &context);
        assert_eq!(result, "Run test in session xyz789");
    }

    #[test]
    fn test_render_template_no_args() {
        let template = "Hello {{arg}}!";
        let args: Option<String> = None;
        let context: TemplateContext = HashMap::new();

        let result = render_template(template, &args, &context);
        assert_eq!(result, "Hello !");
    }

    #[test]
    fn test_command_info_render() {
        let cmd = CommandInfo {
            name: "test".to_string(),
            path: None,
            metadata: CommandMetadata::new("test"),
            content: Some("Execute: {{args}}".to_string()),
            scope: CommandScope::Builtin,
        };

        let args = Some("build --release".to_string());
        let context: TemplateContext = HashMap::new();

        let result = cmd.render(&args, context);
        assert_eq!(result, "Execute: build --release");
    }

    #[test]
    fn test_command_info_render_no_content() {
        let cmd = CommandInfo {
            name: "test".to_string(),
            path: None,
            metadata: CommandMetadata::new("test"),
            content: None,
            scope: CommandScope::Builtin,
        };

        let args: Option<String> = None;
        let context: TemplateContext = HashMap::new();

        let result = cmd.render(&args, context);
        assert_eq!(result, "");
    }
}
