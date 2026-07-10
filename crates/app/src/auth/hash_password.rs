//! Hash a raw password into a storable PHC string.

use argon2::password_hash::{PasswordHasher, SaltString};
use argon2::Argon2;
use rand_core::OsRng;

use crate::{Result, StoreError};

/// Hash a raw password into a self-describing PHC string (algorithm, params, and
/// salt are all encoded in the output), suitable for storage.
pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| StoreError::Store(format!("password hash: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::verify_password::verify_password;

    #[test]
    fn same_password_hashes_differ_by_salt() {
        let a = hash_password("hunter2").unwrap();
        let b = hash_password("hunter2").unwrap();
        assert_ne!(a, b, "each hash must use a fresh salt");
        assert!(verify_password("hunter2", &a));
        assert!(verify_password("hunter2", &b));
    }
}
