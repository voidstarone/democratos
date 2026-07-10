//! Constant-time byte-string comparison for secret material.

use subtle::ConstantTimeEq;

/// Compare two byte strings in constant time (with respect to their contents).
///
/// Use this for any secret comparison — bearer tokens, API keys — where a plain
/// `==` would short-circuit on the first differing byte and leak, through
/// response timing, how long a matching prefix an attacker guessed. Lengths may
/// still differ observably; that's fine for fixed-length or high-entropy secrets.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.ct_eq(b).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_matches_semantics_of_eq() {
        assert!(constant_time_eq(b"Bearer secret", b"Bearer secret"));
        assert!(!constant_time_eq(b"Bearer secret", b"Bearer secreT"));
        assert!(!constant_time_eq(b"Bearer secret", b"Bearer"));
        assert!(!constant_time_eq(b"", b"x"));
        assert!(constant_time_eq(b"", b""));
    }
}
