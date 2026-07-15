//! Strongly-typed identifiers. Newtypes prevent mixing a `UserId` with a
//! `DemosId` at compile time. Stores assign the underlying values.

pub mod comment_id;
pub mod demos_id;
pub mod founding_id;
pub mod invite_id;
pub mod notification_id;
pub mod post_id;
pub mod proposal_id;
pub mod report_id;
pub mod rule_id;
pub mod sensitive_case_id;
pub mod trial_comment_id;
pub mod trial_id;
pub mod user_id;
