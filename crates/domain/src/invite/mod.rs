//! Invite requests — the node-local access waitlist that gates account creation
//! while a node runs invitation-only. Not federated: a waitlist entry lives and
//! dies on the node that hosts it.

#[allow(clippy::module_inception)]
pub mod invite_request;
pub mod invite_status;
