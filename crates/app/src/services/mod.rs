//! Use-cases — the application's public API. Both the web and CLI adapters call
//! these exact methods; neither contains business logic of its own.
//!
//! One definition per file: the [`Services`](services::Services) container and
//! each supporting type live in their own leaf module. The crate root re-exports
//! the flat names.

pub mod enfranchise_outcome;
pub mod feed_item;
pub mod member_metrics;
pub mod search_results;
pub mod search_scope;
#[allow(clippy::module_inception)]
pub mod services;

// Cohesive per-area services, each owning only the ports it needs.
pub mod account_service;
pub mod blocking_service;
pub mod founding_service;
pub mod invite_service;
pub mod membership_service;
pub mod metrics_service;
pub mod moderation_service;
pub mod notification_service;
pub mod profile_service;
pub mod search_service;
pub mod sensitive_review_service;

// Free helper functions, one per file.
pub mod escalate_to_operator;
pub mod post_matches;
pub mod posting_allowed;
pub mod vote_value;
pub mod voting_window_days;

// Use-case areas: each is one `impl Services` block.
pub mod accounts;
pub mod blocking;
pub mod content;
pub mod feed;
pub mod founding;
pub mod governance;
pub mod invites;
pub mod membership;
pub mod metrics;
pub mod moderation;
pub mod notifications_ops;
pub mod profile;
pub mod search_ops;
pub mod sensitive;
