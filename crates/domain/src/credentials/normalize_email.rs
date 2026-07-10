//! Email normalization for case-insensitive lookups.

/// Trim and lower-case an email so lookups and the uniqueness check are
/// case-insensitive (the local part technically is case-sensitive, but treating
/// it so surprises users far more often than it helps).
pub fn normalize_email(raw: &str) -> String {
    raw.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_case_and_whitespace() {
        assert_eq!(normalize_email("  Alice@Example.COM "), "alice@example.com");
    }
}
