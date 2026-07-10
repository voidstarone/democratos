//! Per-user account signing identity — the cryptographic root of trust for an
//! **open** federation, where the node hosting an account is not trusted.
//!
//! # Why this exists
//!
//! Once anyone can run a node, "the node authenticated its user" is worthless: a
//! hostile operator would simply forge its members' votes. So a governance action
//! is signed by the **acting user**, whose Ed25519 *public* key the account
//! carries, and verified against that key wherever the action is applied — most
//! importantly on the community's **owner** node, which re-checks the signature
//! and never takes the forwarding node's word for who acted. A rogue node can
//! censor or lie to its own users, but it cannot forge a signed ballot.
//!
//! The server only ever holds *public* keys and *verifies* — the secret key stays
//! on the user's device. This module is verify-only by construction; it has no
//! way to sign. It lives in `app` beside [`crate::auth`] and [`crate::session`]
//! because, like those, it is a cryptographic concern the pure domain shouldn't
//! carry.
//!
//! # What is signed
//!
//! Each action has a single **canonical message string**
//! ([`vote_message`](vote_message::vote_message) et al.) carrying a version + a
//! domain-separation tag + the action's identifying fields. Domain separation
//! means a signature captured for one action (or protocol) can never be replayed
//! as another. Actions are idempotent per actor+target, so a replayed identical
//! signature is a harmless no-op; a *different* decision (toggle or clear) is a
//! different message and needs its own signature.

pub mod domain;
pub mod is_valid_public_key;
pub mod jury_vote_message;
pub mod post_vote_message;
pub mod user_public_key;
pub mod vote_message;
