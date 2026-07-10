//! A user's standing *within a specific demos*. The same user is a lurker in one
//! demos and a voter in another, so tier lives on the membership, not the user.

#[allow(clippy::module_inception)]
pub mod membership;
pub mod tier;
