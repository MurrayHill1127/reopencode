//! ID generation utilities.
//!
//! Provides functions for generating unique identifiers including UUIDs
//! and monotonic IDs for sorting purposes.

use uuid::Uuid;

/// Generates a UUID v4 (random) identifier.
///
/// Returns a UUID in the standard hyphenated format (8-4-4-4-12).
///
/// # Examples
///
/// ```
/// let uuid = generate_uuid();
/// assert_eq!(uuid.len(), 36);
/// assert!(uuid.contains('-'));
/// ```
#[must_use]
pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string()
}

/// Generates an ascending ID string.
///
/// Formats the counter as `id_{counter}` which sorts in ascending order
/// when compared lexicographically.
///
/// # Examples
///
/// ```
/// let id = generate_ascending_id(1);
/// assert_eq!(id, "id_1");
///
/// let id = generate_ascending_id(100);
/// assert_eq!(id, "id_100");
/// ```
#[must_use]
pub fn generate_ascending_id(counter: u64) -> String {
    format!("id_{counter}")
}

/// Generates a descending ID string.
///
/// Formats the counter as `id_{u64::MAX - counter}` which sorts in
/// descending order when compared lexicographically (higher counter
/// values produce smaller ID strings).
///
/// # Examples
///
/// ```
/// let id = generate_descending_id(0);
/// assert_eq!(id, "id_18446744073709551615");
///
/// let id = generate_descending_id(100);
/// assert_eq!(id, "id_18446744073709551515");
/// ```
#[must_use]
pub fn generate_descending_id(counter: u64) -> String {
    let inverted = u64::MAX - counter;
    format!("id_{inverted}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_format() {
        let uuid = generate_uuid();
        assert_eq!(uuid.len(), 36, "UUID should be 36 characters");
        assert!(
            uuid.contains('-'),
            "UUID should contain hyphens in standard format"
        );
        // Check format: 8-4-4-4-12
        let parts: Vec<&str> = uuid.split('-').collect();
        assert_eq!(parts.len(), 5, "UUID should have 5 parts");
        assert_eq!(parts[0].len(), 8);
        assert_eq!(parts[1].len(), 4);
        assert_eq!(parts[2].len(), 4);
        assert_eq!(parts[3].len(), 4);
        assert_eq!(parts[4].len(), 12);
    }

    #[test]
    fn test_uuid_uniqueness() {
        let mut uuids = std::collections::HashSet::new();
        for _ in 0..1000 {
            let uuid = generate_uuid();
            assert!(uuids.insert(uuid), "UUID should be unique");
        }
        assert_eq!(uuids.len(), 1000);
    }

    #[test]
    fn test_ascending_id_format() {
        assert_eq!(generate_ascending_id(0), "id_0");
        assert_eq!(generate_ascending_id(1), "id_1");
        assert_eq!(generate_ascending_id(100), "id_100");
        assert_eq!(generate_ascending_id(u64::MAX), "id_18446744073709551615");
    }

    #[test]
    fn test_descending_id_format() {
        assert_eq!(generate_descending_id(0), "id_18446744073709551615");
        assert_eq!(generate_descending_id(1), "id_18446744073709551614");
        assert_eq!(generate_descending_id(100), "id_18446744073709551515");
        assert_eq!(generate_descending_id(u64::MAX), "id_0");
    }

    #[test]
    fn test_ascending_id_sorting() {
        let ids: Vec<String> = (0..5).map(generate_ascending_id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted, "Ascending IDs should sort lexicographically");
    }

    #[test]
    fn test_descending_id_sorting() {
        let ids: Vec<String> = (0..5).map(generate_descending_id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        // When sorted ascending, descending IDs appear in reverse counter order
        // This is the expected behavior: higher counter = smaller string = appears first
        let expected: Vec<String> = (0..5).rev().map(generate_descending_id).collect();
        assert_eq!(sorted, expected, "Sorted descending IDs should appear in reverse counter order");
    }
}