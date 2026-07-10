//! The password policy ceiling.

/// The most a password may be, so a login can never be turned into a hashing
/// denial-of-service by submitting megabytes to Argon2.
pub const MAX_PASSWORD_LEN: usize = 256;
