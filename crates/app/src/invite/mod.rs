//! Invite-token helpers: minting the raw single-use token and hashing it for
//! storage. Kept out of the service body so both live one-per-file and are unit-
//! testable in isolation.

pub mod hash_token;
pub mod new_invite_token;
