//! Ports — the trait boundary between the application and the outside world.
//!
//! Every adapter (in-memory, text-file, SQLite, …) implements these. Because the
//! application depends only on the traits, swapping one implementation for
//! another is a single line in the composition root. The `Clock` is a port too,
//! so tests can drive time deterministically.
//!
//! One definition per file: each port trait (and the `MediaVerdict` enum) lives
//! in its own leaf module. The crate root re-exports the flat names.

pub mod account_authenticator;
pub mod account_minter;
pub mod age_verifier;
pub mod clock;
pub mod comment_store;
pub mod comment_vote_store;
pub mod demos_store;
pub mod founding_store;
pub mod governance_writes;
pub mod invite_request_store;
pub mod media_quarantine;
pub mod media_safety_scanner;
pub mod media_sanitizer;
pub mod media_store;
pub mod media_verdict;
pub mod membership_store;
pub mod safety_verdict;
pub mod sanitized_media;
pub mod notifier;
pub mod nsfw_scanner;
pub mod post_store;
pub mod post_vote_store;
pub mod proposal_store;
pub mod report_store;
pub mod rule_store;
pub mod sensitive_case_store;
pub mod settings_store;
pub mod similarity_index;
pub mod trial_store;
pub mod user_store;
pub mod vote_store;
