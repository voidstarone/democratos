//! Recommendation **use cases** (interactors).
//!
//! Each use case is its own object that declares *only* the ports it needs —
//! not the whole [`Services`](crate::Services) container — so its dependencies
//! are honest and it is unit-testable in isolation. [`Services`](crate::Services)
//! keeps the shared `Arc<dyn Port>` handles and offers thin factory methods
//! ([`Services::recommend_feed`](crate::Services::recommend_feed),
//! [`Services::refresh_recommendations`](crate::Services::refresh_recommendations))
//! that wire a use case up, but the logic lives here.
//!
//! Two use cases, deliberately split by responsibility:
//! * [`RefreshRecommendations`](refresh_recommendations::RefreshRecommendations)
//!   — the *write* side: rebuild the similarity model when votes change. Driven
//!   off the request path (a background task / a CLI pre-step).
//! * [`RecommendFeed`](recommend_feed::RecommendFeed) — the *read* side: a pure
//!   read of the current model.

pub mod cold_start_min_likes;
pub mod recommend_feed;
pub mod recommend_limit;
pub mod recommend_per_community_cap;
pub mod recommendation;
pub mod refresh_recommendations;
