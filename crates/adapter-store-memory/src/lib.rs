//! In-memory implementation of the store ports, plus a controllable clock.
//!
//! One struct implements every store trait; the composition root clones a single
//! `Arc<MemoryStore>` into each `Arc<dyn …Store>` slot. Locking is coarse (one
//! mutex) which is perfectly adequate for tests and small dev runs.

mod comment_vote_rec;
mod fixed_clock;
mod inner;
mod jury_ballot_rec;
mod memory_store;
mod post_vote_rec;
mod vote_rec;

pub use fixed_clock::FixedClock;
pub use memory_store::MemoryStore;
