//! Utility functions for security and helpers

use subtle::ConstantTimeEq;

/// Constant-time string comparison to prevent timing attacks
///
/// Used for comparing authentication tokens
pub fn ct_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ct_eq_equal() {
        assert!(ct_eq("hello", "hello"));
        assert!(ct_eq("", ""));
    }

    #[test]
    fn test_ct_eq_not_equal() {
        assert!(!ct_eq("hello", "world"));
        assert!(!ct_eq("hello", "hell"));
        assert!(!ct_eq("hello", "hello!"));
    }

    #[test]
    fn test_ct_eq_different_lengths() {
        assert!(!ct_eq("short", "longer"));
        assert!(!ct_eq("", "not empty"));
    }
}
