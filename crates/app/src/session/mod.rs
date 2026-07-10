//! Session-cookie signing — the piece that makes "who is this browser?" an
//! *authenticated* question rather than a self-asserted one.
//!
//! The delivery layer records the acting user in a cookie. On its own a bare
//! user id is forgeable: anyone could send `uid=1` and be treated as user 1.
//! [`SessionSigner`](session_signer::SessionSigner) fixes that by attaching an
//! HMAC-SHA256 tag over the id, keyed by a server secret the client never sees.
//! A cookie is accepted only if its tag verifies (constant-time), so a browser
//! can hold a session **only for an id the server itself signed** — i.e. one it
//! actually authenticated.
//!
//! This lives in `app` (beside [`crate::auth`]) because, like password hashing,
//! it is a cryptographic concern the pure domain shouldn't carry, and it is
//! independent of any particular delivery mechanism.

pub mod constant_time_eq;
pub mod session_signer;
