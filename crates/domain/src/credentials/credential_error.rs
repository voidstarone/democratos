//! Why a submitted credential was rejected.

use std::fmt;

use crate::{MAX_PASSWORD_LEN, MIN_PASSWORD_LEN};

/// Why a submitted credential was rejected. The delivery layer renders these to
/// the user; keep the messages self-contained.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CredentialError {
    EmailEmpty,
    EmailMalformed,
    PasswordTooShort,
    PasswordTooLong,
}

impl CredentialError {
    /// A human-readable, already-translated-into-English reason.
    pub fn message(&self) -> String {
        match self {
            CredentialError::EmailEmpty => "email is required".into(),
            CredentialError::EmailMalformed => "that doesn't look like an email address".into(),
            CredentialError::PasswordTooShort => {
                format!("password must be at least {MIN_PASSWORD_LEN} characters")
            }
            CredentialError::PasswordTooLong => {
                format!("password must be at most {MAX_PASSWORD_LEN} characters")
            }
        }
    }
}

impl fmt::Display for CredentialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message())
    }
}
