//! Write forwarding: route a write **use-case** to the community's owner node.
//!
//! Reads are served from the local replica, but a write that changes a community
//! must be authoritative on that community's owner — most of all a **vote**, where
//! the anti-takeover rules (enfranchisement caps, no double-voting, weighting)
//! have to be evaluated against the single authoritative electorate. So a write is
//! forwarded as a **command** (the *intent*, not a row): the owner runs the real
//! use-case, re-validating every rule against its own state — a forwarding node's
//! claims are never trusted — and the resulting change replicates back over the
//! normal feed.
//!
//! Forwarding is at the use-case level, not the store level, because one use-case
//! (`cast_vote`) spans several store operations that must all happen on the owner.
//!
//! # Fail-closed
//!
//! If a community has no reachable owner, a write **fails** rather than being
//! applied locally and reconciled later. For a governance system, integrity wins
//! over availability: a vote that can't be recorded authoritatively is refused.

#[allow(clippy::module_inception)]
pub mod command;
pub mod command_outcome;
pub mod command_transport;
pub mod demos_of;
pub mod execute;
pub mod federated_writes;
pub mod forward_error;
pub mod http_command_transport;
pub mod in_memory_nonce_log;
pub mod max_command_skew_secs;
pub mod nonce_log;
pub mod replay_guard;
pub mod signed_command;
pub mod sync_vote_executor;
pub mod verify_signed;
pub mod write_router;

pub(crate) mod signing_payload;
