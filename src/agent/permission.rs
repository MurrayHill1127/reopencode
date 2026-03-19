//! Permission evaluation system for agent actions.
//!
//! This module provides a rules-based permission system that supports:
//! - Three action types: Allow, Ask (default), Deny
//! - Pattern matching for permission strings
//! - Multiple rulesets that can be merged
//! - Last-match-wins semantics for rule evaluation

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use wildmatch::WildMatch;

/// Represents the possible outcomes of a permission evaluation.
///
/// The default action is `Ask`, which means the user should be prompted
/// for confirmation when no explicit rule matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    /// Permission is granted without user interaction
    Allow,
    /// User should be prompted for confirmation (default)
    #[default]
    Ask,
    /// Permission is denied
    Deny,
}

/// A single permission rule that matches permission strings against patterns.
///
/// Rules are evaluated in order, with the last matching rule determining
/// the final action (last-match-wins semantics).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// The permission category this rule applies to (e.g., "file", "network")
    /// Use "*" to match all permission categories.
    pub permission: String,
    /// A glob pattern to match against the input string
    /// Supports `*` (matches any sequence within a segment) and `**` (matches across segments)
    pub pattern: String,
    /// The action to take when this rule matches
    pub action: Action,
}

impl Rule {
    /// Creates a new permission rule.
    ///
    /// # Arguments
    ///
    /// * `permission` - The permission category (e.g., "file", "network", or "*" for all)
    /// * `pattern` - A glob pattern to match input strings
    /// * `action` - The action to take when this rule matches
    ///
    /// # Example
    ///
    /// ```
    /// use reopencode::agent::permission::{Action, Rule};
    ///
    /// let rule = Rule::new("file", "/home/user/**", Action::Allow);
    /// ```
    pub fn new(permission: impl Into<String>, pattern: impl Into<String>, action: Action) -> Self {
        Self {
            permission: permission.into(),
            pattern: pattern.into(),
            action,
        }
    }

    /// Checks if this rule matches the given permission and input.
    ///
    /// A rule matches when:
    /// - The permission category matches (or is "*")
    /// - The input matches the pattern (or pattern is "*")
    ///
    /// # Arguments
    ///
    /// * `permission` - The permission category to check
    /// * `input` - The input string to match against the pattern
    ///
    /// # Returns
    ///
    /// `true` if the rule matches, `false` otherwise
    pub fn matches(&self, permission: &str, input: &str) -> bool {
        let perm_match = self.permission == "*" || self.permission == permission;
        let pattern_match = self.matches_pattern(input);
        perm_match && pattern_match
    }

    /// Performs glob pattern matching against an input string.
    ///
    /// Uses the `wildmatch` crate for pattern matching, supporting:
    /// - `*` - matches any sequence of characters within a segment
    /// - `**` - matches across path segments
    /// - `?` - matches any single character
    /// - Character classes like `[abc]` or `[a-z]`
    ///
    /// # Arguments
    ///
    /// * `input` - The string to match against the pattern
    ///
    /// # Returns
    ///
    /// `true` if the input matches the pattern, `false` otherwise
    fn matches_pattern(&self, input: &str) -> bool {
        if self.pattern == "*" {
            return true;
        }
        WildMatch::new(&self.pattern).matches(input)
    }
}

/// A collection of permission rules.
///
/// Rules are evaluated in order, and the last matching rule determines
/// the final action. If no rules match, the default action is `Ask`.
pub type Ruleset = Vec<Rule>;

/// Evaluates a permission against a set of rules.
///
/// Uses last-match-wins semantics: rules are evaluated in order, and
/// the last rule that matches determines the final action. If no rules
/// match, returns `Action::Ask`.
///
/// # Arguments
///
/// * `rules` - The set of rules to evaluate against
/// * `permission` - The permission category being checked
/// * `input` - The input string to match against rule patterns
///
/// # Returns
///
/// The `Action` determined by the last matching rule, or `Action::Ask`
/// if no rules match.
///
/// # Example
///
/// ```
/// use reopencode::agent::permission::{Action, evaluate, Rule};
///
/// let rules = vec![
///     Rule::new("file", "/tmp/**", Action::Deny),
///     Rule::new("file", "/home/user/**", Action::Allow),
/// ];
///
/// assert_eq!(evaluate(&rules, "file", "/home/user/doc.txt"), Action::Allow);
/// assert_eq!(evaluate(&rules, "file", "/tmp/test.txt"), Action::Deny);
/// assert_eq!(evaluate(&rules, "network", "example.com"), Action::Ask);
/// ```
pub fn evaluate(rules: &[Rule], permission: &str, input: &str) -> Action {
    rules
        .iter()
        .filter(|r| r.matches(permission, input))
        .map(|r| r.action)
        .next_back()
        .unwrap_or(Action::Ask)
}

/// Merges multiple rulesets into a single ruleset.
///
/// Rules are concatenated in the order they appear in the input vector.
/// Later rules will take precedence during evaluation (last-match-wins).
///
/// # Arguments
///
/// * `rulesets` - A vector of rulesets to merge
///
/// # Returns
///
/// A single ruleset containing all rules from all input rulesets
///
/// # Example
///
/// ```
/// use reopencode::agent::permission::{Action, merge, Rule};
///
/// let ruleset1 = vec![Rule::new("file", "/tmp/**", Action::Deny)];
/// let ruleset2 = vec![Rule::new("file", "/home/**", Action::Allow)];
///
/// let merged = merge(vec![ruleset1, ruleset2]);
/// assert_eq!(merged.len(), 2);
/// ```
pub fn merge(rulesets: Vec<Ruleset>) -> Ruleset {
    rulesets.into_iter().flatten().collect()
}

/// A convenience wrapper for managing permission rules.
///
/// Provides a simple API for adding rules and evaluating permissions.
/// Maintains a single ruleset internally.
#[derive(Debug, Clone, Default)]
pub struct PermissionEngine {
    rules: Ruleset,
}

impl PermissionEngine {
    /// Creates a new, empty permission engine.
    ///
    /// # Example
    ///
    /// ```
    /// use reopencode::agent::permission::PermissionEngine;
    ///
    /// let engine = PermissionEngine::new();
    /// ```
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Adds a single rule to the engine.
    ///
    /// # Arguments
    ///
    /// * `rule` - The rule to add
    ///
    /// # Example
    ///
    /// ```
    /// use reopencode::agent::permission::{Action, PermissionEngine, Rule};
    ///
    /// let mut engine = PermissionEngine::new();
    /// engine.add_rule(Rule::new("file", "*.txt", Action::Allow));
    /// ```
    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
    }

    /// Adds multiple rules to the engine.
    ///
    /// # Arguments
    ///
    /// * `rules` - The ruleset to add
    ///
    /// # Example
    ///
    /// ```
    /// use reopencode::agent::permission::{Action, PermissionEngine, Rule};
    ///
    /// let mut engine = PermissionEngine::new();
    /// let rules = vec![
    ///     Rule::new("file", "*.txt", Action::Allow),
    ///     Rule::new("file", "*.exe", Action::Deny),
    /// ];
    /// engine.add_rules(rules);
    /// ```
    pub fn add_rules(&mut self, rules: Ruleset) {
        self.rules.extend(rules);
    }

    /// Evaluates a permission against all rules in the engine.
    ///
    /// Uses last-match-wins semantics. Returns `Action::Ask` if no
    /// rules match.
    ///
    /// # Arguments
    ///
    /// * `permission` - The permission category to check
    /// * `input` - The input string to match against patterns
    ///
    /// # Returns
    ///
    /// The determined action
    ///
    /// # Example
    ///
    /// ```
    /// use reopencode::agent::permission::{Action, PermissionEngine, Rule};
    ///
    /// let mut engine = PermissionEngine::new();
    /// engine.add_rule(Rule::new("file", "*.txt", Action::Allow));
    ///
    /// assert_eq!(engine.evaluate("file", "document.txt"), Action::Allow);
    /// assert_eq!(engine.evaluate("file", "script.sh"), Action::Ask);
    /// ```
    pub fn evaluate(&self, permission: &str, input: &str) -> Action {
        evaluate(&self.rules, permission, input)
    }

    /// Returns the number of rules in the engine.
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Returns true if the engine has no rules.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Returns a reference to the rules in the engine.
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// Clears all rules from the engine.
    pub fn clear(&mut self) {
        self.rules.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_default_is_ask() {
        let action: Action = Default::default();
        assert_eq!(action, Action::Ask);
    }

    #[test]
    fn test_rule_matches_exact_permission() {
        let rule = Rule::new("file", "*.txt", Action::Allow);
        assert!(rule.matches("file", "document.txt"));
        assert!(!rule.matches("network", "document.txt"));
    }

    #[test]
    fn test_rule_matches_wildcard_permission() {
        let rule = Rule::new("*", "*.txt", Action::Allow);
        assert!(rule.matches("file", "document.txt"));
        assert!(rule.matches("network", "document.txt"));
    }

    #[test]
    fn test_rule_matches_wildcard_pattern() {
        let rule = Rule::new("file", "*", Action::Allow);
        assert!(rule.matches("file", "anything"));
        assert!(!rule.matches("network", "anything"));
    }

    #[test]
    fn test_evaluate_last_match_wins() {
        let rules = vec![
            Rule::new("file", "*.txt", Action::Deny),
            Rule::new("file", "*.txt", Action::Allow),
        ];
        assert_eq!(evaluate(&rules, "file", "doc.txt"), Action::Allow);
    }

    #[test]
    fn test_evaluate_default_ask() {
        let rules: Vec<Rule> = vec![];
        assert_eq!(evaluate(&rules, "file", "doc.txt"), Action::Ask);
    }

    #[test]
    fn test_evaluate_no_match_ask() {
        let rules = vec![Rule::new("file", "*.txt", Action::Allow)];
        assert_eq!(evaluate(&rules, "file", "doc.pdf"), Action::Ask);
    }

    #[test]
    fn test_merge_rulesets() {
        let ruleset1 = vec![Rule::new("file", "*.txt", Action::Allow)];
        let ruleset2 = vec![Rule::new("network", "*", Action::Deny)];
        let merged = merge(vec![ruleset1, ruleset2]);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn test_permission_engine_basic() {
        let mut engine = PermissionEngine::new();
        engine.add_rule(Rule::new("file", "*.txt", Action::Allow));
        engine.add_rule(Rule::new("file", "*.exe", Action::Deny));

        assert_eq!(engine.evaluate("file", "doc.txt"), Action::Allow);
        assert_eq!(engine.evaluate("file", "app.exe"), Action::Deny);
        assert_eq!(engine.evaluate("file", "image.png"), Action::Ask);
    }

    #[test]
    fn test_permission_engine_add_rules() {
        let mut engine = PermissionEngine::new();
        let rules = vec![
            Rule::new("file", "*.txt", Action::Allow),
            Rule::new("file", "*.exe", Action::Deny),
        ];
        engine.add_rules(rules);

        assert_eq!(engine.len(), 2);
    }

    #[test]
    fn test_permission_engine_clear() {
        let mut engine = PermissionEngine::new();
        engine.add_rule(Rule::new("file", "*.txt", Action::Allow));
        engine.clear();
        assert!(engine.is_empty());
    }

    #[test]
    fn test_glob_pattern_matching() {
        let rule = Rule::new("file", "**/*.txt", Action::Allow);
        assert!(rule.matches("file", "/home/user/doc.txt"));
        assert!(rule.matches("file", "src/test.txt"));
        assert!(!rule.matches("file", "/home/user/doc.pdf"));
    }

    #[test]
    fn test_serialize_action() {
        let action = Action::Allow;
        let json = serde_json::to_string(&action).unwrap();
        assert_eq!(json, "\"allow\"");
    }

    #[test]
    fn test_deserialize_action() {
        let action: Action = serde_json::from_str("\"deny\"").unwrap();
        assert_eq!(action, Action::Deny);
    }

    #[test]
    fn test_serialize_rule() {
        let rule = Rule::new("file", "*.txt", Action::Allow);
        let json = serde_json::to_string(&rule).unwrap();
        assert!(json.contains("\"permission\":\"file\""));
        assert!(json.contains("\"pattern\":\"*.txt\""));
        assert!(json.contains("\"action\":\"allow\""));
    }

    #[test]
    fn test_deserialize_rule() {
        let json = r#"{"permission":"file","pattern":"*.txt","action":"allow"}"#;
        let rule: Rule = serde_json::from_str(json).unwrap();
        assert_eq!(rule.permission, "file");
        assert_eq!(rule.pattern, "*.txt");
        assert_eq!(rule.action, Action::Allow);
    }
}
