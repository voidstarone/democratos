//! Local, dependency-free stubs for the moderation ports — enough to run the
//! NSFW + age-verification features on a single box (a Raspberry Pi) today, and
//! swappable for real services later with no change to the domain or use-cases.
//!
//! * [`HeuristicNsfwScanner`] applies the pure text lexicon
//!   ([`domain::nsfw_score`]) to a media item's caption and URL. It can't see
//!   pixels, so it only ever returns [`MediaVerdict::Nsfw`] (caption/URL look
//!   explicit) or [`MediaVerdict::Unknown`] (defer to text/tags) — never a
//!   confident `Sfw`. Swap in a real vision model or an external API here.
//! * [`AutoApproveAgeVerifier`] stands in for a real age-verification provider
//!   (ID check, AV vendor) by approving every request. A production deployment
//!   replaces it with an adapter that drives the provider's flow.
//!
//! One definition per file: each public type lives in its own leaf module and is
//! re-exported flat here.

pub mod auto_approve_age_verifier;
pub mod heuristic_nsfw_scanner;

pub use auto_approve_age_verifier::AutoApproveAgeVerifier;
pub use heuristic_nsfw_scanner::HeuristicNsfwScanner;
