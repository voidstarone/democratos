//! Password hashing — the one piece of the credential flow that needs a crypto
//! dependency, so it lives in the application layer rather than the pure domain.
//!
//! Argon2id with the library defaults (a memory-hard KDF, per OWASP's current
//! guidance) and a fresh random salt per password. Verification is constant-time
//! within the Argon2 comparison. The rest of the policy — email shape, password
//! length — is validated in [`domain::credentials`] before we ever hash.

pub mod hash_password;
pub mod spend_verify_time;
pub mod verify_password;
