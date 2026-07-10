//! Password length-policy validation.

use crate::{CredentialError, MAX_PASSWORD_LEN, MIN_PASSWORD_LEN};

/// Validate a raw (un-hashed) password against the length policy. The password
/// is never trimmed — leading/trailing spaces are legitimate characters.
pub fn validate_password(password: &str) -> Result<(), CredentialError> {
    if password.len() < MIN_PASSWORD_LEN {
        Err(CredentialError::PasswordTooShort)
    } else if password.len() > MAX_PASSWORD_LEN {
        Err(CredentialError::PasswordTooLong)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_password_length() {
        assert_eq!(
            validate_password("1234567"),
            Err(CredentialError::PasswordTooShort)
        );
        assert_eq!(validate_password("12345678"), Ok(()));
        let long = "x".repeat(MAX_PASSWORD_LEN + 1);
        assert_eq!(
            validate_password(&long),
            Err(CredentialError::PasswordTooLong)
        );
    }
}
