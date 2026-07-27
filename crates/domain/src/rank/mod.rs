//! Feed ranking — how a pile of posts becomes an ordered front page.
//!
//! Pure functions over `(score, comments, timestamp)`, so ranking policy is
//! auditable and testable without a database, and a community can be shown
//! exactly what rule orders its feed.
//!
//! Two submission rankings are offered because they encode genuinely different
//! politics, and which one a demos picks is a governance question, not a
//! technical one:
//!
//! * [`postmill_ranking`](postmill_ranking::postmill_ranking) — Postmill/Raddle.
//!   Weights **comments above votes** (~2.8:1) and collapses the comment bonus
//!   10x once a post is disliked, so discussion is rewarded but a flamewar under
//!   a hated post is not. Linear and capped in both directions: nothing squats
//!   on the front page, nothing is buried forever.
//! * [`reddit_hot`](reddit_hot::reddit_hot) — Reddit. Logarithmic votes plus
//!   linear time decay, with **no comment term at all**. Ranks approval rather
//!   than conversation, and gives the earliest voters outsized power.
//!
//! Plus one comment ranking, [`wilson_lower_bound`](wilson_lower_bound::wilson_lower_bound),
//! which sorts replies by confidence rather than raw score so a good comment in
//! a small thread is not buried by a mediocre one in a busy thread.

pub mod postmill_ranking;
pub mod reddit_hot;
pub mod wilson_lower_bound;
