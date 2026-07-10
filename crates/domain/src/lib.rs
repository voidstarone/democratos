//! Democratos domain core — pure governance logic.
//!
//! This crate has no knowledge of databases, HTTP, the filesystem, or any async
//! runtime. Every rule is a pure function of its inputs (entities + a `now`
//! timestamp), which makes the governance engine exhaustively unit-testable and
//! keeps the four defensive layers in one auditable place:
//!
//! * Layer 1 — earned franchise: [`franchise::evaluate_eligibility`]
//! * Layer 2 — enfranchisement rate cap: [`franchise::enfranchisement_slots`]
//! * Layer 3 — tiered thresholds: [`governance::threshold_for`] + [`governance::decide`]
//! * Layer 4 — timelock + recall: [`governance::Proposal::close`]

pub mod bots;
pub mod content;
pub mod credentials;
pub mod demos;
pub mod franchise;
pub mod governance;
pub mod ids;
pub mod jury;
pub mod membership;
pub mod node;
pub mod nsfw;
pub mod recommend;
pub mod report;
pub mod rules;
pub mod time;
pub mod user;
pub mod weighting;

pub use bots::bot_report_threshold::BOT_REPORT_THRESHOLD;
pub use bots::bot_score::bot_score;
pub use bots::bot_signals::BotSignals;
pub use bots::is_likely_bot::is_likely_bot;
pub use content::build_comment_tree::build_comment_tree;
pub use content::comment::Comment;
pub use content::comment_node::CommentNode;
pub use content::feed_threshold::feed_threshold;
pub use content::max_tags::MAX_TAGS;
pub use content::media::Media;
pub use content::normalize_tags::normalize_tags;
pub use content::post::Post;
pub use credentials::credential_error::CredentialError;
pub use credentials::max_password_len::MAX_PASSWORD_LEN;
pub use credentials::min_password_len::MIN_PASSWORD_LEN;
pub use credentials::normalize_email::normalize_email;
pub use credentials::validate_email::validate_email;
pub use credentials::validate_password::validate_password;
pub use demos::demos::Demos;
pub use demos::founding_petition::FoundingPetition;
pub use demos::phase::Phase;
pub use demos::posting_policy::PostingPolicy;
pub use demos::sign_offs_required::SIGN_OFFS_REQUIRED;
pub use demos::slugify::slugify;
pub use franchise::eligibility::Eligibility;
pub use franchise::enfranchisement_slots::enfranchisement_slots;
pub use franchise::evaluate_eligibility::evaluate_eligibility;
pub use franchise::franchise_criteria::FranchiseCriteria;
pub use franchise::unmet::Unmet;
pub use governance::decide::decide;
pub use governance::decision::Decision;
pub use governance::decision_class::DecisionClass;
pub use governance::proposal::Proposal;
pub use governance::proposal_kind::ProposalKind;
pub use governance::proposal_status::ProposalStatus;
pub use governance::recall_window_days::RECALL_WINDOW_DAYS;
pub use governance::tally::Tally;
pub use governance::threshold::Threshold;
pub use governance::threshold_for::threshold_for;
pub use ids::comment_id::CommentId;
pub use ids::demos_id::DemosId;
pub use ids::founding_id::FoundingId;
pub use ids::post_id::PostId;
pub use ids::proposal_id::ProposalId;
pub use ids::report_id::ReportId;
pub use ids::rule_id::RuleId;
pub use ids::trial_id::TrialId;
pub use ids::user_id::UserId;
pub use jury::content_scale::ContentScale;
pub use jury::default_jury_size::DEFAULT_JURY_SIZE;
pub use jury::jury_ballot::JuryBallot;
pub use jury::jury_sizing::JurySizing;
pub use jury::reach_verdict::reach_verdict;
pub use jury::select_jury::select_jury;
pub use jury::trial::Trial;
pub use jury::verdict::Verdict;
pub use membership::membership::Membership;
pub use membership::tier::Tier;
pub use node::compose_id::compose_id;
pub use node::local_sequence::local_sequence;
pub use node::max_sequence::MAX_SEQUENCE;
pub use node::node_id::NodeId;
pub use node::origin_node::origin_node;
pub use node::sequence_bits::SEQUENCE_BITS;
pub use node::sequence_mask::SEQUENCE_MASK;
pub use nsfw::is_nsfw_text::is_nsfw_text;
pub use nsfw::nsfw_flag_threshold::NSFW_FLAG_THRESHOLD;
pub use nsfw::nsfw_score::nsfw_score;
pub use nsfw::visibility::visibility;
pub use nsfw::visibility::Visibility;
pub use recommend::blend::blend;
pub use recommend::item_index::ItemIndex;
pub use recommend::rank_and_diversify::rank_and_diversify;
pub use recommend::rating::Rating;
pub use recommend::tag_affinity::tag_affinity;
pub use recommend::tag_profile::tag_profile;
pub use report::flag::Flag;
pub use report::report::Report;
pub use report::report_reason::ReportReason;
pub use report::report_status::ReportStatus;
pub use report::report_target::ReportTarget;
pub use rules::Rule;
pub use time::Timestamp;
pub use user::feed_paging::FeedPaging;
pub use user::user::User;
pub use weighting::max_vote_weight::MAX_VOTE_WEIGHT;
pub use weighting::vote_weighting::VoteWeighting;
pub use weighting::weighting_scope::WeightingScope;
