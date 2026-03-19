//! Command template renderer
//!
//! Provides variable substitution for command templates.

use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;

/// Context for template rendering
#[derive(Debug, Clone, Default)]
pub struct TemplateContext {
    /// Session identifier
    pub session_id: Option<String>,

    /// Timestamp string
    pub timestamp: Option<String>,

    /// Working directory path
    pub working_dir: Option<PathBuf>,

    /// Custom variables for substitution
    pub variables: HashMap<String, String>,
}

impl TemplateContext {
    /// Create a new context with current timestamp
    pub fn new() -> Self {
        Self {
            timestamp: Some(Utc::now().to_rfc3339()),
            ..Default::default()
        }
    }

    /// Set the session ID (builder pattern)
    pub fn with_session_id(mut self, id: impl Into<String>) -> Self {
        self.session_id = Some(id.into());
        self
    }

    /// Add a custom variable (builder pattern)
    pub fn with_variable(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.variables.insert(key.into(), value.into());
        self
    }
}

/// Render a template with variable substitution
///
/// Supported variables:
/// - `$ARGUMENTS` - replaced with provided arguments or empty string
/// - `$SESSION_ID` - replaced with context.session_id if present
/// - `$TIMESTAMP` - replaced with context.timestamp if present
/// - `${name}` - replaced with context.variables.get("name")
///
/// The result is trimmed of leading/trailing whitespace.
pub fn render_template(
    template: &str,
    arguments: Option<&str>,
    context: &TemplateContext,
) -> String {
    let mut result = template.to_string();

    result = result.replace("$ARGUMENTS", arguments.unwrap_or(""));

    if let Some(ref session_id) = context.session_id {
        result = result.replace("$SESSION_ID", session_id);
    }

    if let Some(ref timestamp) = context.timestamp {
        result = result.replace("$TIMESTAMP", timestamp);
    }

    for (key, value) in &context.variables {
        result = result.replace(&format!("${{{}}}", key), value);
    }

    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_template_with_arguments() {
        let template = "<user-request>$ARGUMENTS</user-request>";
        let rendered = render_template(template, Some("my args"), &TemplateContext::default());
        assert_eq!(rendered, "<user-request>my args</user-request>");
    }

    #[test]
    fn test_render_template_empty_arguments() {
        let template = "Args: [$ARGUMENTS]";
        let rendered = render_template(template, None, &TemplateContext::default());
        assert_eq!(rendered, "Args: []");
    }

    #[test]
    fn test_render_template_with_session_id() {
        let template = "Session: $SESSION_ID";
        let context = TemplateContext::new().with_session_id("ses_123");
        let rendered = render_template(template, None, &context);
        assert_eq!(rendered, "Session: ses_123");
    }

    #[test]
    fn test_render_template_with_custom_variables() {
        let template = "Hello ${name}!";
        let context = TemplateContext::new().with_variable("name", "World");
        let rendered = render_template(template, None, &context);
        assert_eq!(rendered, "Hello World!");
    }

    #[test]
    fn test_render_template_multiple_variables() {
        let template = "$SESSION_ID: ${action} $ARGUMENTS";
        let context = TemplateContext::new()
            .with_session_id("abc")
            .with_variable("action", "run");
        let rendered = render_template(template, Some("test"), &context);
        assert_eq!(rendered, "abc: run test");
    }

    #[test]
    fn test_render_template_trimming() {
        let template = "  $ARGUMENTS  ";
        let rendered = render_template(template, Some("x"), &TemplateContext::default());
        assert_eq!(rendered, "x");
    }

    #[test]
    fn test_template_context_default_timestamp() {
        let ctx = TemplateContext::new();
        assert!(ctx.timestamp.is_some());
        // Should be a valid RFC3339 timestamp
        assert!(ctx.timestamp.unwrap().contains('T'));
    }

    #[test]
    fn test_template_context_default_empty() {
        let ctx: TemplateContext = Default::default();
        assert!(ctx.session_id.is_none());
        assert!(ctx.timestamp.is_none());
        assert!(ctx.working_dir.is_none());
        assert!(ctx.variables.is_empty());
    }
}
