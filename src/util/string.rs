//! String utilities module
//!
//! Provides common string manipulation functions including truncation,
//! case conversion, and capitalization.

/// Truncates a string to the specified maximum length.
/// 
/// Returns a string slice of at most `max_length` characters.
/// If the string is shorter than `max_length`, returns the original string.
/// 
/// # Arguments
/// 
/// * `s` - The input string to truncate
/// * `max_length` - Maximum number of characters to keep
/// 
/// # Examples
/// 
/// ```
/// use reopencode::util::string::truncate;
/// 
/// assert_eq!(truncate("Hello World", 5), "Hello");
/// assert_eq!(truncate("Hi", 10), "Hi");
/// assert_eq!(truncate("", 5), "");
/// ```
pub fn truncate(s: &str, max_length: usize) -> &str {
    if s.len() <= max_length {
        s
    } else {
        &s[..max_length]
    }
}

/// Truncates a string and appends "..." at the end if truncation occurred.
/// 
/// If the string is shorter than `max_length`, returns the original string.
/// When truncation occurs, the result length will be `max_length + 3` (including "...").
/// 
/// # Arguments
/// 
/// * `s` - The input string to truncate
/// * `max_length` - Maximum number of characters before the ellipsis
/// 
/// # Examples
/// 
/// ```
/// use reopencode::util::string::truncate_with_ellipsis;
/// 
/// assert_eq!(truncate_with_ellipsis("Hello World", 8), "Hello...");
/// assert_eq!(truncate_with_ellipsis("Hi", 10), "Hi");
/// assert_eq!(truncate_with_ellipsis("", 5), "");
/// ```
pub fn truncate_with_ellipsis(s: &str, max_length: usize) -> String {
    if s.len() <= max_length {
        s.to_string()
    } else if max_length <= 3 {
        ".".repeat(max_length)
    } else {
        let truncate_len = max_length - 3;
        format!("{}...", &s[..truncate_len])
    }
}

/// Converts a camelCase string to snake_case.
/// 
/// Inserts underscore before uppercase letters and converts the result to lowercase.
/// 
/// # Arguments
/// 
/// * `s` - The camelCase string to convert
/// 
/// # Examples
/// 
/// ```
/// use reopencode::util::string::camel_to_snake;
/// 
/// assert_eq!(camel_to_snake("camelCase"), "camel_case");
/// assert_eq!(camel_to_snake("CamelCase"), "camel_case");
/// assert_eq!(camel_to_snake("helloWorld"), "hello_world");
/// assert_eq!(camel_to_snake("helloWorldXML"), "hello_world_xml");
/// ```
pub fn camel_to_snake(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 2);
    let chars: Vec<char> = s.chars().collect();
    
    for (i, c) in chars.iter().enumerate() {
        if c.is_uppercase() {
            let prev_is_lower = i > 0 && chars[i - 1].is_lowercase();
            let next_is_lower = i + 1 < chars.len() && chars[i + 1].is_lowercase();
            
            if i > 0 && (prev_is_lower || next_is_lower) {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(*c);
        }
    }
    
    result
}

/// Converts a snake_case string to camelCase.
/// 
/// Removes underscores and capitalizes the character following each underscore.
/// 
/// # Arguments
/// 
/// * `s` - The snake_case string to convert
/// 
/// # Examples
/// 
/// ```
/// use reopencode::util::string::snake_to_camel;
/// 
/// assert_eq!(snake_to_camel("snake_case"), "snakeCase");
/// assert_eq!(snake_to_camel("snake_case_case"), "snakeCaseCase");
/// assert_eq!(snake_to_camel("hello_world"), "helloWorld");
/// ```
pub fn snake_to_camel(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut capitalize_next = false;
    
    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    
    result
}

/// Capitalizes the first character of the string.
/// 
/// Converts the first character to uppercase and leaves the rest unchanged.
/// If the string is empty, returns an empty string.
/// 
/// # Arguments
/// 
/// * `s` - The string to capitalize
/// 
/// # Examples
/// 
/// ```
/// use reopencode::util::string::capitalize;
/// 
/// assert_eq!(capitalize("hello"), "Hello");
/// assert_eq!(capitalize("Hello"), "Hello");
/// assert_eq!(capitalize(""), "");
/// assert_eq!(capitalize("h"), "H");
/// ```
pub fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_ascii_uppercase().to_string() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("Hello World", 5), "Hello");
        assert_eq!(truncate("Hi", 10), "Hi");
        assert_eq!(truncate("", 5), "");
        assert_eq!(truncate("Hello", 5), "Hello");
        assert_eq!(truncate("Hello World", 0), "");
    }

    #[test]
    fn test_truncate_with_ellipsis() {
        assert_eq!(truncate_with_ellipsis("Hello World", 8), "Hello...");
        assert_eq!(truncate_with_ellipsis("Hi", 10), "Hi");
        assert_eq!(truncate_with_ellipsis("", 5), "");
        assert_eq!(truncate_with_ellipsis("Hello", 5), "Hello");
        assert_eq!(truncate_with_ellipsis("Hello", 10), "Hello");
    }

    #[test]
    fn test_camel_to_snake() {
        assert_eq!(camel_to_snake("camelCase"), "camel_case");
        assert_eq!(camel_to_snake("CamelCase"), "camel_case");
        assert_eq!(camel_to_snake("helloWorld"), "hello_world");
        assert_eq!(camel_to_snake("helloWorldXML"), "hello_world_xml");
        assert_eq!(camel_to_snake("snake_case"), "snake_case");
        assert_eq!(camel_to_snake(""), "");
    }

    #[test]
    fn test_snake_to_camel() {
        assert_eq!(snake_to_camel("snake_case"), "snakeCase");
        assert_eq!(snake_to_camel("snake_case_case"), "snakeCaseCase");
        assert_eq!(snake_to_camel("hello_world"), "helloWorld");
        assert_eq!(snake_to_camel("camelCase"), "camelCase");
        assert_eq!(snake_to_camel(""), "");
    }

    #[test]
    fn test_capitalize() {
        assert_eq!(capitalize("hello"), "Hello");
        assert_eq!(capitalize("Hello"), "Hello");
        assert_eq!(capitalize(""), "");
        assert_eq!(capitalize("h"), "H");
        assert_eq!(capitalize("hello world"), "Hello world");
    }
}