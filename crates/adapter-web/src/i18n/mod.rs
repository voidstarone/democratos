//! Translatability.
//!
//! All user-facing chrome lives in a [`Strings`] catalog — one `&'static str`
//! field per message — with a value per [`Lang`]. Because it is a struct, a
//! missing translation is a *compile error*, not a silent fallback. Dynamic
//! domain values (phases, tiers, statuses, unmet requirements) are translated by
//! the functions at the bottom, so the domain never has to know about language.
//!
//! Adding a language = add a `Lang` variant and one more `Strings` constant.

pub mod class;
pub mod lang;
pub mod phase;
pub mod posting_policy_label;
pub mod proposal_title;
pub mod queued_note;
pub mod status;
pub mod strings;
pub mod tier;
pub mod unmet;
pub mod verdict;
