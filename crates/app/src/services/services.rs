//! The application service container. Every use-case method on it lives in a
//! sibling area module (`accounts`, `governance`, `content`, …), each an
//! `impl Services` block; this file holds only the struct itself.

use std::sync::Arc;




use crate::{
    AgeVerifier, Clock, CommentStore, CommentVoteStore, DemosStore, FoundingStore,
    InviteRequestStore, MediaStore, MembershipStore, NotificationStore, Notifier,
    NsfwScanner, PostStore, PostVoteStore, ProposalStore,
    ReportStore, RuleStore, SensitiveCaseStore, SettingsStore, SimilarityIndex, TrialCommentStore,
    TrialStore, UserStore, VoteStore,
};


/// The application service container. Holds the chosen port implementations;
/// the composition root decides which concrete adapters go in here.
#[derive(Clone)]
pub struct Services {
    pub users: Arc<dyn UserStore>,
    pub demoi: Arc<dyn DemosStore>,
    /// Pending founding petitions, before they become real communities.
    pub foundings: Arc<dyn FoundingStore>,
    pub memberships: Arc<dyn MembershipStore>,
    pub proposals: Arc<dyn ProposalStore>,
    pub votes: Arc<dyn VoteStore>,
    pub rules: Arc<dyn RuleStore>,
    pub posts: Arc<dyn PostStore>,
    pub comments: Arc<dyn CommentStore>,
    pub reports: Arc<dyn ReportStore>,
    /// The node-local access waitlist (invite requests), used only while the node
    /// runs invitation-only.
    pub invites: Arc<dyn InviteRequestStore>,
    /// Operator settings that survive a restart — today just the invitation-only
    /// toggle.
    pub settings: Arc<dyn SettingsStore>,
    /// Delivers the invite-approval email (SMTP in prod, a log sink in dev).
    pub notifier: Arc<dyn Notifier>,
    /// Platform-wide sensitive-content review cases (the extra-demos review queue).
    pub sensitive_cases: Arc<dyn SensitiveCaseStore>,
    pub trials: Arc<dyn TrialStore>,
    pub trial_comments: Arc<dyn TrialCommentStore>,
    /// Node-local in-app notifications (mentions + jury summons). Never federated.
    pub notifications: Arc<dyn NotificationStore>,
    pub post_votes: Arc<dyn PostVoteStore>,
    pub comment_votes: Arc<dyn CommentVoteStore>,
    pub media: Arc<dyn MediaStore>,
    pub recommender: Arc<dyn SimilarityIndex>,
    /// Classifies uploaded media for NSFW content (text is handled in-domain).
    pub nsfw_scanner: Arc<dyn NsfwScanner>,
    /// Performs real age verification (stub in dev).
    pub age_verifier: Arc<dyn AgeVerifier>,
    /// Deployment toggle: when true (e.g. a UK deployment), NSFW content is
    /// age-gated and unverified viewers cannot reveal it. Off in most countries.
    pub requires_age_verification: bool,
    /// Deployment toggle: when true, every governance action must carry a valid
    /// signature — an account with no enrolled signing key cannot vote, jury, or
    /// vote on posts until it enrols one. This closes the open-federation forgery
    /// gap where a malicious node can act for any *key-less* member (the unsigned
    /// "rollout fallback"). Leave off only during initial key rollout on a trusted
    /// single box; turn on before opening the federation to untrusted nodes.
    pub require_signatures: bool,
    /// Public origin (`scheme://host[:port]`) this node is reached at, used to
    /// build absolute invite-accept links for the approval email. No trailing
    /// slash.
    pub public_base_url: String,
    /// How many days an issued invite token stays valid before it must be
    /// re-approved.
    pub invite_token_ttl_days: i64,
    pub clock: Arc<dyn Clock>,
}
