//! Command discovery module
//!
//! Discovers and loads commands from multiple sources: builtin, config files,
//! project directories, user directories, and plugins.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::builtin::BUILTIN_COMMANDS;
use super::types::{CommandInfo, CommandScope, CommandMetadata};

/// Discovery options for command loading
#[derive(Debug, Clone, Default)]
pub struct DiscoveryOptions {
    /// Whether plugin commands are enabled
    pub plugins_enabled: bool,

    /// Plugin enablement override map
    pub enabled_plugins_override: Option<HashMap<String, bool>>,
}

impl DiscoveryOptions {
    /// Create new discovery options with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set plugins enabled
    pub fn with_plugins_enabled(mut self, enabled: bool) -> Self {
        self.plugins_enabled = enabled;
        self
    }

    /// Set plugin override map
    pub fn with_plugin_override(mut self, plugin: impl Into<String>, enabled: bool) -> Self {
        let overrides = self.enabled_plugins_override.get_or_insert_with(HashMap::new);
        overrides.insert(plugin.into(), enabled);
        self
    }
}

/// Command discovery engine
///
/// Loads commands from multiple sources in priority order:
/// 1. Project commands (.roc/commands/)
/// 2. User commands (~/.config/roc/commands/)
/// 3. OpenCode project commands (.opencode/command/)
/// 4. OpenCode global commands
/// 5. Built-in commands
/// 6. Plugin commands
pub struct CommandDiscovery {
    working_dir: PathBuf,
    options: DiscoveryOptions,
}

impl CommandDiscovery {
    /// Create a new command discovery instance
    pub fn new() -> Self {
        Self {
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            options: DiscoveryOptions::default(),
        }
    }

    /// Set the working directory for discovery
    pub fn with_working_dir(mut self, dir: impl AsRef<Path>) -> Self {
        self.working_dir = dir.as_ref().to_path_buf();
        self
    }

    /// Set discovery options
    pub fn with_options(mut self, options: DiscoveryOptions) -> Self {
        self.options = options;
        self
    }

    /// Discover all available commands
    ///
    /// MVP: Currently only loads built-in commands.
    /// Future: Will scan project, user, and OpenCode directories.
    pub fn discover(&self) -> Vec<CommandInfo> {
        let mut commands = Vec::new();

        // Priority order from spec:
        // 1. Project commands (not yet implemented)
        // 2. User commands (not yet implemented)
        // 3. OpenCode global commands (not yet implemented)
        // 4. Built-in commands (MVP)
        // 5. Plugin commands (not yet implemented)

        // For MVP: just load builtin commands
        commands.extend(self.load_builtin_commands());

        commands
    }

    /// Discover commands from a specific directory
    ///
    /// This is a placeholder for future file system discovery.
    /// Will scan the given directory for .md command files.
    fn discover_from_dir(&self, _dir: &Path, _scope: CommandScope) -> Vec<CommandInfo> {
        // Future: scan directory for .md files with frontmatter
        // Parse each file and create CommandInfo
        Vec::new()
    }

    /// Load built-in commands from the BUILTIN_COMMANDS static
    fn load_builtin_commands(&self) -> Vec<CommandInfo> {
        let mut commands = Vec::new();

        for (name, def) in BUILTIN_COMMANDS.iter() {
            let metadata = CommandMetadata::from_definition(&super::types::CommandDefinition {
                name: def.name.clone(),
                description: def.description.clone(),
                template: def.template.clone(),
                agent: def.agent.clone(),
                model: def.model.clone(),
                subtask: def.subtask,
                argument_hint: def.argument_hint.clone(),
                handoffs: None,
            });

            let info = CommandInfo::new(name.clone(), metadata)
                .with_scope(CommandScope::Builtin)
                .with_content(def.template.clone());

            commands.push(info);
        }

        // Sort by name for consistent ordering
        commands.sort_by(|a, b| a.name.cmp(&b.name));
        commands
    }

    /// Load plugin commands
    ///
    /// Stub for future plugin command loading.
    fn load_plugin_commands(&self) -> Vec<CommandInfo> {
        if !self.options.plugins_enabled {
            return Vec::new();
        }

        // Future: load commands from enabled plugins
        // Check enabled_plugins_override for plugin-specific settings
        Vec::new()
    }
}

impl Default for CommandDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to discover all commands
///
/// Uses the current directory and default options.
/// For more control, use CommandDiscovery directly.
pub fn discover_commands(
    directory: Option<&Path>,
    options: Option<DiscoveryOptions>,
) -> Vec<CommandInfo> {
    let mut discovery = CommandDiscovery::new();

    if let Some(dir) = directory {
        discovery = discovery.with_working_dir(dir);
    }

    if let Some(opts) = options {
        discovery = discovery.with_options(opts);
    }

    discovery.discover()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_options_default() {
        let opts = DiscoveryOptions::default();
        assert!(!opts.plugins_enabled);
        assert!(opts.enabled_plugins_override.is_none());
    }

    #[test]
    fn test_discovery_options_builder() {
        let opts = DiscoveryOptions::new()
            .with_plugins_enabled(true)
            .with_plugin_override("test-plugin", true);

        assert!(opts.plugins_enabled);
        assert!(opts.enabled_plugins_override.is_some());
        assert_eq!(
            opts.enabled_plugins_override.as_ref().unwrap().get("test-plugin"),
            Some(&true)
        );
    }

    #[test]
    fn test_command_discovery_new() {
        let discovery = CommandDiscovery::new();
        assert!(!discovery.options.plugins_enabled);
    }

    #[test]
    fn test_command_discovery_default() {
        let discovery: CommandDiscovery = Default::default();
        assert!(!discovery.options.plugins_enabled);
    }

    #[test]
    fn test_command_discovery_builder() {
        let opts = DiscoveryOptions::new().with_plugins_enabled(true);
        let discovery = CommandDiscovery::new()
            .with_working_dir("/tmp")
            .with_options(opts);

        assert!(discovery.options.plugins_enabled);
        assert_eq!(discovery.working_dir, PathBuf::from("/tmp"));
    }

    #[test]
    fn test_load_builtin_commands() {
        let discovery = CommandDiscovery::new();
        let commands = discovery.load_builtin_commands();

        // Should have all 8 builtin commands
        assert_eq!(commands.len(), 8);

        // Check that commands are sorted
        let names: Vec<_> = commands.iter().map(|c| c.name.as_str()).collect();
        let mut sorted_names = names.clone();
        sorted_names.sort();
        assert_eq!(names, sorted_names);

        // Check that all commands have Builtin scope
        for cmd in &commands {
            assert_eq!(cmd.scope, CommandScope::Builtin);
        }
    }

    #[test]
    fn test_discover_returns_builtin_commands() {
        let discovery = CommandDiscovery::new();
        let commands = discovery.discover();

        // MVP: should return only builtin commands
        assert_eq!(commands.len(), 8);

        // All should be builtin scope
        for cmd in &commands {
            assert_eq!(cmd.scope, CommandScope::Builtin);
        }
    }

    #[test]
    fn test_discover_commands_with_none() {
        let commands = discover_commands(None, None);
        assert_eq!(commands.len(), 8);
    }

    #[test]
    fn test_discover_commands_with_options() {
        let opts = DiscoveryOptions::new().with_plugins_enabled(true);
        let commands = discover_commands(None, Some(opts));
        // MVP: still only builtin commands even with plugins enabled
        assert_eq!(commands.len(), 8);
    }

    #[test]
    fn test_load_plugin_commands_disabled() {
        let discovery = CommandDiscovery::new();
        let commands = discovery.load_plugin_commands();
        assert!(commands.is_empty());
    }

    #[test]
    fn test_load_plugin_commands_enabled_but_stub() {
        let opts = DiscoveryOptions::new().with_plugins_enabled(true);
        let discovery = CommandDiscovery::new().with_options(opts);
        // MVP: plugin loading is stubbed
        let commands = discovery.load_plugin_commands();
        assert!(commands.is_empty());
    }

    #[test]
    fn test_discover_from_dir_returns_empty() {
        let discovery = CommandDiscovery::new();
        let commands = discovery.discover_from_dir(Path::new("/tmp"), CommandScope::User);
        // MVP: directory discovery not implemented
        assert!(commands.is_empty());
    }

    #[test]
    fn test_builtin_command_content_loaded() {
        let discovery = CommandDiscovery::new();
        let commands = discovery.discover();

        // Check that content is populated
        let init_deep = commands.iter().find(|c| c.name == "init-deep");
        assert!(init_deep.is_some());
        assert!(init_deep.unwrap().content.is_some());
    }

    #[test]
    fn test_builtin_command_metadata() {
        let discovery = CommandDiscovery::new();
        let commands = discovery.discover();

        let init_deep = commands.iter().find(|c| c.name == "init-deep").unwrap();
        assert_eq!(init_deep.metadata.name, "init-deep");
        assert!(!init_deep.metadata.description.is_empty());
    }
}
