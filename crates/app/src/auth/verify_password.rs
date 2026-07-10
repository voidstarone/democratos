//! Verify a raw password against a stored PHC hash.

use argon2::password_hash::{PasswordHash, PasswordVerifier};
use argon2::Argon2;

/// Verify a raw password against a stored PHC hash. Returns `false` for a
/// mismatch *or* an unparseable hash — callers translate either into the same
/// opaque "invalid credentials" so nothing about the stored value leaks.
pub fn verify_password(password: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::hash_password::hash_password;

    #[test]
    fn round_trips_and_rejects_wrong_password() {
        let hash = hash_password("correct horse battery").unwrap();
        assert!(verify_password("correct horse battery", &hash));
        assert!(!verify_password("wrong", &hash));
    }

    #[test]
    fn garbage_hash_is_a_rejection_not_a_panic() {
        assert!(!verify_password("anything", "not-a-phc-string"));
    }
}
