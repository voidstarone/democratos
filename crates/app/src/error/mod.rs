//! The application's error vocabulary: one error type per use-case function, so a
//! function's `Result` names only the errors it can emit or propagate. The storage
//! vocabulary ([`StoreError`](store_error::StoreError)) is the shared base every
//! `*Store` port speaks and the default of the [`Result`](result::Result) alias;
//! richer use-cases wrap it (`#[from] StoreError`) alongside their own domain
//! variants.
//!
//! One definition per file: each error type and the `Result` alias live in their
//! own leaf module. The crate root re-exports the flat names.

pub mod accept_invite_error;
pub mod approve_invite_error;
pub mod authenticate_error;
pub mod can_post_error;
pub mod cast_jury_vote_error;
pub mod cast_vote_error;
pub mod close_proposal_error;
pub mod create_post_error;
pub mod enroll_public_key_error;
pub mod ensure_barred_account_error;
pub mod found_demos_error;
pub mod media_error;
pub mod member_action_error;
pub mod mint_account_error;
pub mod notify_error;
pub mod open_proposal_error;
pub mod open_trial_error;
pub mod register_account_error;
pub mod request_invite_error;
pub mod result;
pub mod sensitive_review_error;
pub mod set_feed_paging_error;
pub mod settle_trial_error;
pub mod sign_founding_error;
pub mod start_founding_error;
pub mod store_error;
pub mod verify_action_error;
pub mod vote_post_error;
