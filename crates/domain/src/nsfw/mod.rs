//! NSFW detection (text) and the view-time age gate.
//!
//! Like [`crate::bots`], the text detector is a pure, auditable heuristic: it
//! scores 0–100 and at/above [`crate::NSFW_FLAG_THRESHOLD`] the content is flagged.
//! "The machine flags; the demos judges" — flagging never removes anything; in a
//! community that has *voted to forbid* NSFW it files a report for a jury, and a
//! flagged post is always blurred/age-gated rather than deleted.
//!
//! Image/video can't be classified by a pure function, so that lives behind an
//! application port (`NsfwScanner`) — swappable for a real model or an external
//! service. This module owns only what can be decided purely: a text lexicon
//! score and the visibility gate.

pub mod is_nsfw_text;
pub mod nsfw_flag_threshold;
pub mod nsfw_score;
pub mod visibility;
