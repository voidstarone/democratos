//! The password policy floor.

/// The password policy floor. Argon2 defends the hash; this only rules out
/// trivially short secrets. Deliberately modest — usability over theatre.
pub const MIN_PASSWORD_LEN: usize = 8;
