//! Format utilities for common data formatting operations.
//!
//! This module provides utilities for formatting durations, relative times,
//! byte sizes, and numbers with proper localization support.

use std::time::Duration;

/// Style for formatting durations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DurationStyle {
    /// Full format: "1 小时 2 分 3 秒"
    #[default]
    Full,
    /// Short format: "1h 2m 3s"
    Short,
    /// Compact format: "1:02:03"
    Compact,
    /// Seconds only: "3723s"
    Seconds,
}

impl DurationStyle {
    /// Returns the unit names for this style.
    fn units(&self) -> (&'static str, &'static str, &'static str) {
        match self {
            DurationStyle::Full => ("小时", "分", "秒"),
            DurationStyle::Short => ("h", "m", "s"),
            DurationStyle::Compact => ("", "", ""),
            DurationStyle::Seconds => ("", "", "s"),
        }
    }
}

/// Format a duration into a human-readable string.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use reopencode::util::format::{format_duration, DurationStyle};
///
/// // Full format
/// assert_eq!(
///     format_duration(Duration::from_secs(3723), DurationStyle::Full),
///     "1 小时 2 分 3 秒"
/// );
///
/// // Short format
/// assert_eq!(
///     format_duration(Duration::from_secs(3723), DurationStyle::Short),
///     "1h 2m 3s"
/// );
///
/// // Compact format
/// assert_eq!(
///     format_duration(Duration::from_secs(3723), DurationStyle::Compact),
///     "1:02:03"
/// );
///
/// // Seconds only
/// assert_eq!(
///     format_duration(Duration::from_secs(3723), DurationStyle::Seconds),
///     "3723s"
/// );
/// ```
pub fn format_duration(duration: Duration, style: DurationStyle) -> String {
    let total_secs = duration.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    match style {
        DurationStyle::Compact => {
            if hours > 0 {
                format!("{}:{:02}:{:02}", hours, minutes, seconds)
            } else {
                format!("{:02}:{:02}", minutes, seconds)
            }
        }
        DurationStyle::Seconds => {
            format!("{}s", total_secs)
        }
        _ => {
            let (hour_unit, min_unit, sec_unit) = style.units();
            let mut parts = Vec::new();

            if hours > 0 {
                parts.push(format!("{}{}", hours, hour_unit));
            }
            if minutes > 0 {
                parts.push(format!("{}{}", minutes, min_unit));
            }
            if seconds > 0 || parts.is_empty() {
                parts.push(format!("{}{}", seconds, sec_unit));
            }

            parts.join(" ")
        }
    }
}

/// Style for formatting relative time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RelativeTimeStyle {
    /// Chinese format: "3 小时前"
    #[default]
    Chinese,
    /// English format: "3 hours ago"
    English,
    /// ISO8601 format: "PT3H"
    Iso8601,
}

/// Format a datetime as relative time from now.
///
/// # Examples
///
/// ```
/// use chrono::{Utc, Duration};
/// use reopencode::util::format::{format_relative_time, RelativeTimeStyle};
///
/// let three_hours_ago = Utc::now() - Duration::hours(3);
///
/// assert_eq!(
///     format_relative_time(three_hours_ago, RelativeTimeStyle::Chinese),
///     "3 小时前"
/// );
///
/// assert_eq!(
///     format_relative_time(three_hours_ago, RelativeTimeStyle::English),
///     "3 hours ago"
/// );
/// ```
pub fn format_relative_time(datetime: chrono::DateTime<chrono::Utc>, style: RelativeTimeStyle) -> String {
    let now = chrono::Utc::now();
    let diff = now.signed_duration_since(datetime);

    if diff.num_seconds() < 0 {
        return match style {
            RelativeTimeStyle::Chinese => "未来".to_string(),
            RelativeTimeStyle::English => "in the future".to_string(),
            RelativeTimeStyle::Iso8601 => "P0S".to_string(),
        };
    }

    let abs_diff = abs_duration(diff);

    match style {
        RelativeTimeStyle::Chinese => format_chinese_relative(abs_diff),
        RelativeTimeStyle::English => format_english_relative(abs_diff),
        RelativeTimeStyle::Iso8601 => format_iso8601_duration(abs_diff),
    }
}

fn abs_duration(diff: chrono::Duration) -> Duration {
    Duration::from_secs(diff.num_seconds().unsigned_abs())
}

fn format_chinese_relative(diff: Duration) -> String {
    let total_secs = diff.as_secs();
    let minutes = total_secs / 60;
    let hours = minutes / 60;
    let days = hours / 24;

    if days > 0 {
        format!("{} 天前", days)
    } else if hours > 0 {
        format!("{} 小时前", hours)
    } else if minutes > 0 {
        format!("{} 分钟前", minutes)
    } else {
        "刚刚".to_string()
    }
}

fn format_english_relative(diff: Duration) -> String {
    let total_secs = diff.as_secs();
    let minutes = total_secs / 60;
    let hours = minutes / 60;
    let days = hours / 24;

    if days > 0 {
        let s = if days == 1 { "" } else { "s" };
        format!("{} day{} ago", days, s)
    } else if hours > 0 {
        let s = if hours == 1 { "" } else { "s" };
        format!("{} hour{} ago", hours, s)
    } else if minutes > 0 {
        let s = if minutes == 1 { "" } else { "s" };
        format!("{} minute{} ago", minutes, s)
    } else {
        "just now".to_string()
    }
}

fn format_iso8601_duration(diff: Duration) -> String {
    let total_secs = diff.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    let mut parts = Vec::new();
    if hours > 0 {
        parts.push(format!("{}H", hours));
    }
    if minutes > 0 {
        parts.push(format!("{}M", minutes));
    }
    if seconds > 0 || parts.is_empty() {
        parts.push(format!("{}S", seconds));
    }

    format!("PT{}", parts.join(""))
}

/// Format bytes into a human-readable string with appropriate units.
///
/// # Examples
///
/// ```
/// use reopencode::util::format::format_bytes;
///
/// assert_eq!(format_bytes(512), "512 B");
/// assert_eq!(format_bytes(1536), "1.5 KB");
/// assert_eq!(format_bytes(1048576), "1.0 MB");
/// assert_eq!(format_bytes(1073741824), "1.0 GB");
/// ```
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format a number with thousands separators.
///
/// # Examples
///
/// ```
/// use reopencode::util::format::format_number;
///
/// assert_eq!(format_number(0), "0");
/// assert_eq!(format_number(999), "999");
/// assert_eq!(format_number(1000), "1,000");
/// assert_eq!(format_number(1234567), "1,234,567");
/// assert_eq!(format_number(1234567890), "1,234,567,890");
/// ```
pub fn format_number(num: u64) -> String {
    let s = num.to_string();
    let len = s.len();

    if len <= 3 {
        return s;
    }

    let mut result = String::new();
    let mut count = 0;

    for ch in s.chars().rev() {
        if count == 3 {
            result.push(',');
            count = 0;
        }
        result.push(ch);
        count += 1;
    }

    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_full() {
        assert_eq!(
            format_duration(Duration::from_secs(0), DurationStyle::Full),
            "0秒"
        );
        assert_eq!(
            format_duration(Duration::from_secs(60), DurationStyle::Full),
            "1分"
        );
        assert_eq!(
            format_duration(Duration::from_secs(3600), DurationStyle::Full),
            "1小时"
        );
        assert_eq!(
            format_duration(Duration::from_secs(3723), DurationStyle::Full),
            "1小时 2分 3秒"
        );
    }

    #[test]
    fn test_format_duration_short() {
        assert_eq!(
            format_duration(Duration::from_secs(0), DurationStyle::Short),
            "0s"
        );
        assert_eq!(
            format_duration(Duration::from_secs(3723), DurationStyle::Short),
            "1h 2m 3s"
        );
    }

    #[test]
    fn test_format_duration_compact() {
        assert_eq!(
            format_duration(Duration::from_secs(0), DurationStyle::Compact),
            "00:00"
        );
        assert_eq!(
            format_duration(Duration::from_secs(59), DurationStyle::Compact),
            "00:59"
        );
        assert_eq!(
            format_duration(Duration::from_secs(60), DurationStyle::Compact),
            "01:00"
        );
        assert_eq!(
            format_duration(Duration::from_secs(3723), DurationStyle::Compact),
            "1:02:03"
        );
    }

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(
            format_duration(Duration::from_secs(3723), DurationStyle::Seconds),
            "3723s"
        );
        assert_eq!(
            format_duration(Duration::from_secs(0), DurationStyle::Seconds),
            "0s"
        );
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
        assert_eq!(format_bytes(1073741824), "1.0 GB");
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1234567), "1,234,567");
        assert_eq!(format_number(1234567890), "1,234,567,890");
    }

    #[test]
    fn test_format_relative_time_chinese() {
        use chrono::Duration;

        let now = chrono::Utc::now();
        let ago_30s = now - Duration::seconds(30);
        let ago_5m = now - Duration::minutes(5);
        let ago_3h = now - Duration::hours(3);
        let ago_2d = now - Duration::days(2);

        assert_eq!(format_relative_time(ago_30s, RelativeTimeStyle::Chinese), "刚刚");
        assert_eq!(format_relative_time(ago_5m, RelativeTimeStyle::Chinese), "5 分钟前");
        assert_eq!(format_relative_time(ago_3h, RelativeTimeStyle::Chinese), "3 小时前");
        assert_eq!(format_relative_time(ago_2d, RelativeTimeStyle::Chinese), "2 天前");
    }

    #[test]
    fn test_format_relative_time_english() {
        use chrono::Duration;

        let now = chrono::Utc::now();
        let ago_30s = now - Duration::seconds(30);
        let ago_5m = now - Duration::minutes(5);
        let ago_3h = now - Duration::hours(3);
        let ago_2d = now - Duration::days(2);

        assert_eq!(format_relative_time(ago_30s, RelativeTimeStyle::English), "just now");
        assert_eq!(format_relative_time(ago_5m, RelativeTimeStyle::English), "5 minutes ago");
        assert_eq!(format_relative_time(ago_3h, RelativeTimeStyle::English), "3 hours ago");
        assert_eq!(format_relative_time(ago_2d, RelativeTimeStyle::English), "2 days ago");
    }

    #[test]
    fn test_format_relative_time_iso8601() {
        use chrono::Duration;

        let now = chrono::Utc::now();
        let ago_30s = now - Duration::seconds(30);
        let ago_5m = now - Duration::minutes(5);
        let ago_3h = now - Duration::hours(3);

        assert_eq!(format_relative_time(ago_30s, RelativeTimeStyle::Iso8601), "PT30S");
        assert_eq!(format_relative_time(ago_5m, RelativeTimeStyle::Iso8601), "PT5M");
        assert_eq!(format_relative_time(ago_3h, RelativeTimeStyle::Iso8601), "PT3H");
    }
}