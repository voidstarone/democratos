//! Content recommendation — item-based collaborative filtering, pure and
//! testable.
//!
//! Like the governance layers, the *maths* lives here as pure functions of
//! their inputs; the application owns when to call them and where the data
//! comes from. The signal is the same up/down vote that drives the home feed
//! (see [`crate::content::feed_threshold`]): users who upvote the same posts
//! are "similar", so posts upvoted together are recommended together.
//!
//! The pipeline is three composable steps:
//!
//! 1. [`crate::ItemIndex::build`] turns a snapshot of ratings into a precomputed
//!    post→post similarity model (built off the request path, then cached).
//! 2. [`crate::blend`] aggregates the neighbours of everything a user liked into one
//!    score per candidate post.
//! 3. [`crate::rank_and_diversify`] orders the candidates and caps how many may come
//!    from any single community, so one large demos cannot flood the feed.
//!
//! [`crate::tag_affinity`] is the cold-start fallback for users with too little voting
//! history to have meaningful neighbours.

pub mod blend;
pub mod item_index;
pub mod rank_and_diversify;
pub mod rating;
pub mod tag_affinity;
pub mod tag_profile;
