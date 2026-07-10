//! Vote weighting — a demos may value some citizens' votes more than others.
//!
//! Weighting is a *governable policy*: every demos picks a [`crate::VoteWeighting`]
//! scheme and a [`crate::WeightingScope`] (which decisions it touches), both amendable
//! by vote. The platform default is one-citizen-one-vote everywhere, so a demos
//! that never opts in behaves exactly as before.
//!
//! Two invariants keep weighting from breaking the democracy:
//! * **No citizen is silenced** — every weight is at least `1`.
//! * **No citizen dominates** — every weight is capped at [`crate::MAX_VOTE_WEIGHT`].

pub mod max_vote_weight;
pub mod vote_weighting;
pub mod weighting_scope;
