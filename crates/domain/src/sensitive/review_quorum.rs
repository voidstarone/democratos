//! How many reviewers must weigh in before a case can be decided.

/// The minimum number of distinct reviewers that must classify a flagged item
/// before its case resolves: **at least five**. No single reviewer (nor a handful)
/// can decide the fate of content on their own — the platform-wide review is a
/// small jury, and the plurality of at least this many tags decides. Until the
/// quorum is met the content stays hidden pending review.
pub const REVIEW_QUORUM: usize = 5;
