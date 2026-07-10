//! A lightweight, dependency-free per-IP rate limiter (fixed-window counter).
//!
//! It exists to blunt two abuse vectors on the auth endpoints:
//!   * online password brute-forcing against `POST /session`, and
//!   * CPU exhaustion — every login/registration runs Argon2, which is
//!     deliberately slow, so an unthrottled flood is a cheap denial of service.
//!
//! State-changing POSTs are bucketed: `POST /session` and `POST /register` get a
//! strict allowance, every other POST a looser one. Reads (GET) are never
//! limited here. The counter is a `Mutex<HashMap>` keyed by `(ip, bucket)` — good
//! enough for a single self-hosted box; a federated deployment would front this
//! with its own edge limiting.
//!
//! ## Client identity
//! The key is the **connection peer address** ([`ConnectInfo`]), never the
//! `X-Forwarded-For` header. `X-Forwarded-For` is attacker-controlled unless a
//! trusted reverse proxy is known to overwrite it, and trusting it blindly would
//! let a single client mint unlimited synthetic IPs and sail past the limit. If
//! Democratos is later run behind the bundled proxy, resolve the real client IP
//! there (or teach this module the proxy's address) — until then the direct peer
//! is the only value we can trust.

pub mod auth_max_requests;
pub mod auth_window;
pub mod bucket;
#[allow(clippy::module_inception)]
pub mod rate_limit;
pub mod rate_limiter;
pub mod write_max_requests;
pub mod write_window;
