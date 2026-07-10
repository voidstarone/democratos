//! In-memory similarity index — the default [`SimilarityIndex`] backend.
//!
//! Holds the precomputed [`domain::ItemIndex`] behind an `RwLock`: rebuilds take
//! the write lock briefly to swap in a fresh model, while request-time neighbour
//! lookups take the read lock and never block one another. The model is built by
//! the pure [`domain::ItemIndex::build`]; this adapter only owns caching and
//! concurrency.
//!
//! Because the application depends only on the [`SimilarityIndex`] trait, this
//! exact-cosine backend can later be swapped for an approximate-nearest-neighbour
//! or matrix-factorisation service with a one-line change in the composition
//! root — no change to `domain`, the use-cases, or any delivery adapter.
//!
//! One definition per file: each public item lives in its own leaf module and is
//! re-exported flat here.

pub mod default_neighbours;
pub mod memory_recommender;

pub use default_neighbours::DEFAULT_NEIGHBOURS;
pub use memory_recommender::MemoryRecommender;
