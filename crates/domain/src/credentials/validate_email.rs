//! Loose email-shape validation.

use crate::CredentialError;

/// Validate an already-[`crate::normalize_email`]d address with a deliberately loose
/// shape check: exactly one `@`, non-empty local part, and a dotted domain. Full
/// RFC 5322 validation is a fool's errand; real deliverability is proven by a
/// verification email, not a regex.
pub fn validate_email(email: &str) -> Result<(), CredentialError> {
    if email.is_empty() {
        return Err(CredentialError::EmailEmpty);
    }
    let (local, domain) = email
        .split_once('@')
        .ok_or(CredentialError::EmailMalformed)?;
    let looks_valid = !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !domain.contains('@')
        && !email.contains(char::is_whitespace);
    if looks_valid {
        Ok(())
    } else {
        Err(CredentialError::EmailMalformed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize_email;

    #[test]
    fn accepts_a_plain_address() {
        assert_eq!(validate_email("alice@example.com"), Ok(()));
    }

    #[test]
    fn rejects_malformed_addresses() {
        for bad in [
            "",
            "alice",
            "alice@",
            "@example.com",
            "a@b",
            "a b@x.com",
            "a@@x.com",
        ] {
            assert!(
                validate_email(&normalize_email(bad)).is_err(),
                "expected {bad:?} to be rejected"
            );
        }
    }
}
