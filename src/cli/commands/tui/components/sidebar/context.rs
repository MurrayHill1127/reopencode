//! Sidebar Context Information
//!
//! This module provides data structures for displaying context information
//! in the sidebar, including token usage, cost, and percentage.

use serde::{Deserialize, Serialize};

/// Context information displayed in the sidebar
///
/// Contains token usage statistics, percentage of context window used,
/// and estimated cost in USD.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextInfo {
    /// Total tokens used in the current session
    pub tokens: u64,
    /// Percentage of context window used (if available)
    pub percentage: Option<u8>,
    /// Estimated cost in USD
    pub cost: f64,
}

impl ContextInfo {
    /// Create new context info
    ///
    /// # Arguments
    ///
    /// * `tokens` - Total tokens used
    /// * `percentage` - Percentage of context window used (0-100)
    /// * `cost` - Estimated cost in USD
    ///
    /// # Returns
    ///
    /// A new `ContextInfo` instance.
    pub fn new(tokens: u64, percentage: Option<u8>, cost: f64) -> Self {
        Self {
            tokens,
            percentage,
            cost,
        }
    }

    /// Create context info with tokens only
    ///
    /// # Arguments
    ///
    /// * `tokens` - Total tokens used
    ///
    /// # Returns
    ///
    /// A new `ContextInfo` with tokens and zero cost.
    pub fn with_tokens(tokens: u64) -> Self {
        Self {
            tokens,
            percentage: None,
            cost: 0.0,
        }
    }

    /// Format tokens for display with K/M suffixes
    ///
    /// # Returns
    ///
    /// A formatted string like "1.2K" or "1.5M" for large numbers.
    pub fn format_tokens(&self) -> String {
        if self.tokens >= 1_000_000 {
            format!("{:.1}M", self.tokens as f64 / 1_000_000.0)
        } else if self.tokens >= 1_000 {
            format!("{:.1}K", self.tokens as f64 / 1_000.0)
        } else {
            self.tokens.to_string()
        }
    }

    /// Format percentage for display
    ///
    /// # Returns
    ///
    /// A formatted percentage string like "45%" or "-" if not available.
    pub fn format_percentage(&self) -> String {
        self.percentage
            .map(|p| format!("{}%", p))
            .unwrap_or_else(|| "-".to_string())
    }

    /// Format cost for display
    ///
    /// # Returns
    ///
    /// A formatted cost string like "$0.002" or "$0.00" if zero.
    pub fn format_cost(&self) -> String {
        if self.cost == 0.0 {
            "$0.00".to_string()
        } else {
            format!("${:.4}", self.cost)
        }
    }

    /// Get color based on percentage usage
    ///
    /// # Returns
    ///
    /// A color indicator string: "green" for low usage,
    /// "yellow" for medium, "red" for high.
    pub fn usage_color(&self) -> &'static str {
        match self.percentage {
            Some(p) if p >= 80 => "red",
            Some(p) if p >= 50 => "yellow",
            _ => "green",
        }
    }
}

impl Default for ContextInfo {
    fn default() -> Self {
        Self {
            tokens: 0,
            percentage: None,
            cost: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_info_new() {
        let info = ContextInfo::new(1500, Some(25), 0.0015);
        assert_eq!(info.tokens, 1500);
        assert_eq!(info.percentage, Some(25));
        assert_eq!(info.cost, 0.0015);
    }

    #[test]
    fn test_context_info_with_tokens() {
        let info = ContextInfo::with_tokens(2000);
        assert_eq!(info.tokens, 2000);
        assert_eq!(info.percentage, None);
        assert_eq!(info.cost, 0.0);
    }

    #[test]
    fn test_format_tokens() {
        let info = ContextInfo::with_tokens(1500);
        assert_eq!(info.format_tokens(), "1.5K");

        let info = ContextInfo::with_tokens(1_500_000);
        assert_eq!(info.format_tokens(), "1.5M");

        let info = ContextInfo::with_tokens(500);
        assert_eq!(info.format_tokens(), "500");
    }

    #[test]
    fn test_format_percentage() {
        let info = ContextInfo::new(1000, Some(45), 0.0);
        assert_eq!(info.format_percentage(), "45%");

        let info = ContextInfo::with_tokens(1000);
        assert_eq!(info.format_percentage(), "-");
    }

    #[test]
    fn test_format_cost() {
        let info = ContextInfo::new(1000, None, 0.0);
        assert_eq!(info.format_cost(), "$0.00");

        let info = ContextInfo::new(1000, None, 0.0015);
        assert_eq!(info.format_cost(), "$0.0015");
    }

    #[test]
    fn test_usage_color() {
        let low = ContextInfo::new(1000, Some(30), 0.0);
        assert_eq!(low.usage_color(), "green");

        let medium = ContextInfo::new(1000, Some(65), 0.0);
        assert_eq!(medium.usage_color(), "yellow");

        let high = ContextInfo::new(1000, Some(85), 0.0);
        assert_eq!(high.usage_color(), "red");
    }

    #[test]
    fn test_context_info_default() {
        let info = ContextInfo::default();
        assert_eq!(info.tokens, 0);
        assert_eq!(info.percentage, None);
        assert_eq!(info.cost, 0.0);
    }
}
