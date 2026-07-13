//! Sensitive-content review — how flagged content is classified by a platform-wide
//! pool of reviewers, outside per-demos governance.
//!
//! Flagged content is hidden pending review; opted-in reviewers each apply a
//! [`SensitiveTag`]; once [`REVIEW_QUORUM`] reviewers have classified it, the
//! plurality tag ([`tally_tags`]) decides the [`ReviewOutcome`]. This is
//! deliberately *not* a demos jury: illegal/sensitive content is not a matter of
//! community opinion.
//!
//! One definition per file; the crate root re-exports the flat names.

pub mod review_outcome;
pub mod review_quorum;
pub mod review_vote;
#[allow(clippy::module_inception)]
pub mod sensitive_case;
pub mod sensitive_case_status;
pub mod sensitive_tag;
pub mod tally_tags;
