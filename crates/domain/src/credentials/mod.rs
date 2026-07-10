//! Sign-up credential policy — pure validation of the email and password a new
//! account offers, before either is hashed or stored. Kept in the domain so the
//! rules live in one auditable place and stay independent of the web layer; the
//! actual password *hashing* is an application concern (it needs a crypto
//! dependency), so it lives in `app`, not here.

pub mod credential_error;
pub mod max_password_len;
pub mod min_password_len;
pub mod normalize_email;
pub mod validate_email;
pub mod validate_password;
