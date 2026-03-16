//! Encoding utilities for the util module.
//!
//! This module provides functions for common encoding operations including
//! Base64 encoding/decoding, FNV-1a hashing, and hexadecimal encoding/decoding.
//!
//! # Examples
//!
//! ```
//! use reopencode::util::encoding::{base64_encode, base64_decode, fnv1a_hash, hex_encode, hex_decode};
//!
//! // Base64 encoding
//! let encoded = base64_encode(b"Hello, World!");
//! assert_eq!(encoded, "SGVsbG8sIFdvcmxkIQ==");
//!
//! // FNV-1a hashing
//! let hash = fnv1a_hash(b"test");
//!
//! // Hex encoding
//! let hex = hex_encode(b"Hello");
//! assert_eq!(hex, "48656c6c6f");
//! ```

use base64::{engine::general_purpose, Engine as _};
use fnv::FnvHasher;
use std::hash::Hasher;

use super::error::UtilError;

/// Base64 encode data.
///
/// Encodes a byte slice into a Base64 string using the standard Base64 alphabet
/// with padding.
///
/// # Arguments
///
/// * `data` - The byte slice to encode
///
/// # Returns
///
/// A Base64-encoded string.
///
/// # Examples
///
/// ```
/// use reopencode::util::encoding::base64_encode;
///
/// let encoded = base64_encode(b"Hello, World!");
/// assert_eq!(encoded, "SGVsbG8sIFdvcmxkIQ==");
///
/// let empty = base64_encode(b"");
/// assert_eq!(empty, "");
/// ```
pub fn base64_encode(data: &[u8]) -> String {
    general_purpose::STANDARD.encode(data)
}

/// Base64 decode data.
///
/// Decodes a Base64 string into a byte vector using the standard Base64 alphabet
/// with padding.
///
/// # Arguments
///
/// * `data` - The Base64 string to decode
///
/// # Returns
///
/// A `Result` containing either the decoded bytes or a `UtilError`.
///
/// # Errors
///
/// Returns `UtilError::Base64` if the input is not valid Base64.
///
/// # Examples
///
/// ```
/// use reopencode::util::encoding::base64_decode;
///
/// let decoded = base64_decode("SGVsbG8sIFdvcmxkIQ==").unwrap();
/// assert_eq!(decoded, b"Hello, World!");
///
/// let empty = base64_decode("").unwrap();
/// assert_eq!(empty, b"");
/// ```
pub fn base64_decode(data: &str) -> Result<Vec<u8>, UtilError> {
    general_purpose::STANDARD
        .decode(data)
        .map_err(UtilError::Base64)
}

/// Compute FNV-1a 64-bit hash.
///
/// Computes the FNV-1a (Fowler-Noll-Vo) 64-bit hash of the input data.
/// This is a non-cryptographic hash function that is fast and provides
/// good distribution for hash tables.
///
/// # Arguments
///
/// * `data` - The byte slice to hash
///
/// # Returns
///
/// The 64-bit FNV-1a hash value.
///
/// # Examples
///
/// ```
/// use reopencode::util::encoding::fnv1a_hash;
///
/// let hash1 = fnv1a_hash(b"test");
/// let hash2 = fnv1a_hash(b"test");
/// assert_eq!(hash1, hash2);
///
/// let hash = fnv1a_hash(b"Hello, World!");
/// assert_ne!(hash, 0);
/// ```
pub fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hasher = FnvHasher::default();
    hasher.write(data);
    hasher.finish()
}

/// Hex encode data.
///
/// Encodes a byte slice into a lowercase hexadecimal string.
/// Each byte is represented as two hex characters (e.g., 255 -> "ff").
///
/// # Arguments
///
/// * `data` - The byte slice to encode
///
/// # Returns
///
/// A lowercase hexadecimal-encoded string.
///
/// # Examples
///
/// ```
/// use reopencode::util::encoding::hex_encode;
///
/// let hex = hex_encode(b"Hello");
/// assert_eq!(hex, "48656c6c6f");
///
/// let empty = hex_encode(b"");
/// assert_eq!(empty, "");
///
/// let bytes = hex_encode(&[0x00, 0xFF, 0xAB]);
/// assert_eq!(bytes, "00ffab");
/// ```
pub fn hex_encode(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

/// Hex decode data.
///
/// Decodes a hexadecimal string into a byte vector.
/// The input must contain an even number of hex characters.
/// Both uppercase and lowercase hex characters are accepted.
///
/// # Arguments
///
/// * `data` - The hexadecimal string to decode
///
/// # Returns
///
/// A `Result` containing either the decoded bytes or a `UtilError`.
///
/// # Errors
///
/// Returns `UtilError::Encoding` if:
/// - The input has an odd number of characters
/// - The input contains non-hex characters
///
/// # Examples
///
/// ```
/// use reopencode::util::encoding::hex_decode;
///
/// let decoded = hex_decode("48656c6c6f").unwrap();
/// assert_eq!(decoded, b"Hello");
///
/// let empty = hex_decode("").unwrap();
/// assert_eq!(empty, b"");
///
/// // Uppercase also works
/// let decoded = hex_decode("48656C6C6F").unwrap();
/// assert_eq!(decoded, b"Hello");
/// ```
pub fn hex_decode(data: &str) -> Result<Vec<u8>, UtilError> {
    if !data.len().is_multiple_of(2) {
        return Err(UtilError::Encoding(
            "Hex string must have an even number of characters".to_string(),
        ));
    }

    (0..data.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&data[i..i + 2], 16)
                .map_err(|e| UtilError::Encoding(format!("Invalid hex character: {}", e)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    mod base64_tests {
        use super::*;

        #[test]
        fn test_base64_encode_basic() {
            let encoded = base64_encode(b"Hello, World!");
            assert_eq!(encoded, "SGVsbG8sIFdvcmxkIQ==");
        }

        #[test]
        fn test_base64_encode_empty() {
            let encoded = base64_encode(b"");
            assert_eq!(encoded, "");
        }

        #[test]
        fn test_base64_decode_basic() {
            let decoded = base64_decode("SGVsbG8sIFdvcmxkIQ==").unwrap();
            assert_eq!(decoded, b"Hello, World!");
        }

        #[test]
        fn test_base64_decode_empty() {
            let decoded = base64_decode("").unwrap();
            assert_eq!(decoded, b"");
        }

        #[test]
        fn test_base64_roundtrip() {
            let original = b"The quick brown fox jumps over the lazy dog. 1234567890!@#$%^&*()";
            let encoded = base64_encode(original);
            let decoded = base64_decode(&encoded).unwrap();
            assert_eq!(decoded, original);
        }

        #[test]
        fn test_base64_decode_invalid() {
            let result = base64_decode("Invalid!!!");
            assert!(result.is_err());
        }
    }

    mod fnv1a_tests {
        use super::*;

        #[test]
        fn test_fnv1a_hash_consistency() {
            let hash1 = fnv1a_hash(b"test");
            let hash2 = fnv1a_hash(b"test");
            assert_eq!(hash1, hash2);
        }

        #[test]
        fn test_fnv1a_hash_different_inputs() {
            let hash1 = fnv1a_hash(b"foo");
            let hash2 = fnv1a_hash(b"bar");
            assert_ne!(hash1, hash2);
        }

        #[test]
        fn test_fnv1a_hash_empty() {
            let hash = fnv1a_hash(b"");
            // FNV-1a offset basis for 64-bit
            assert_eq!(hash, 0xcbf29ce484222325);
        }

        #[test]
        fn test_fnv1a_hash_known_value() {
            // Known FNV-1a hash for "hello"
            let hash = fnv1a_hash(b"hello");
            assert_ne!(hash, 0);
            // Verify consistency
            assert_eq!(hash, fnv1a_hash(b"hello"));
        }
    }

    mod hex_tests {
        use super::*;

        #[test]
        fn test_hex_encode_basic() {
            let hex = hex_encode(b"Hello");
            assert_eq!(hex, "48656c6c6f");
        }

        #[test]
        fn test_hex_encode_empty() {
            let hex = hex_encode(b"");
            assert_eq!(hex, "");
        }

        #[test]
        fn test_hex_encode_bytes() {
            let hex = hex_encode(&[0x00, 0xFF, 0xAB, 0xCD]);
            assert_eq!(hex, "00ffabcd");
        }

        #[test]
        fn test_hex_decode_basic() {
            let decoded = hex_decode("48656c6c6f").unwrap();
            assert_eq!(decoded, b"Hello");
        }

        #[test]
        fn test_hex_decode_empty() {
            let decoded = hex_decode("").unwrap();
            assert_eq!(decoded, b"");
        }

        #[test]
        fn test_hex_decode_uppercase() {
            let decoded = hex_decode("48656C6C6F").unwrap();
            assert_eq!(decoded, b"Hello");
        }

        #[test]
        fn test_hex_decode_mixed_case() {
            let decoded = hex_decode("48FfAbCd").unwrap();
            assert_eq!(decoded, vec![0x48, 0xFF, 0xAB, 0xCD]);
        }

        #[test]
        fn test_hex_roundtrip() {
            let original = b"The quick brown fox jumps over the lazy dog. 1234567890!@#$%^&*()";
            let hex = hex_encode(original);
            let decoded = hex_decode(&hex).unwrap();
            assert_eq!(decoded, original);
        }

        #[test]
        fn test_hex_decode_odd_length() {
            let result = hex_decode("123");
            assert!(result.is_err());
        }

        #[test]
        fn test_hex_decode_invalid_chars() {
            let result = hex_decode("gg");
            assert!(result.is_err());
        }
    }
}
