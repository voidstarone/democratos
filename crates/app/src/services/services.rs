//! The application service container and every use-case method on it.

use std::sync::Arc;

use std::collections::HashSet;

use domain::{
    bot_score, enfranchisement_slots, evaluate_eligibility, is_likely_bot, outcome_for,
    reach_verdict, select_jury, slugify, BotSignals, Comment, CommentId, ContentScale, Demos,
    DemosId, Eligibility, FeedPaging, FoundingId, FoundingPetition, InviteId, InviteRequest, Media,
    mentions, Membership, Notification, NotificationKind, normalize_tags, Phase, Post, MAX_SANCTION_DAYS,
    PostId, PostingPolicy, Proposal, ProposalId, ProposalKind, ProposalStatus, Report, ReportId,
    ReportReason, ReportStatus, ReportTarget, ReviewOutcome, Rule, RuleId, SensitiveCase, SensitiveCaseId,
    SensitiveTag, Tier, Timestamp, Trial, TrialComment, TrialId, User, UserId, Verdict,
    VoteWeighting,
};

use domain::feed_threshold;
use domain::{is_nsfw_text, visibility, Visibility};

use crate::auth::hash_password::hash_password;
use crate::auth::spend_verify_time::spend_verify_time;
use crate::auth::verify_password::verify_password;
use crate::identity::is_valid_public_key::is_valid_public_key;
use crate::identity::jury_vote_message::jury_vote_message;
use crate::identity::post_vote_message::post_vote_message;
use crate::identity::user_public_key::UserPublicKey;
use crate::identity::vote_message::vote_message;
use crate::invite::hash_token::hash_token;
use crate::invite::new_invite_token::new_invite_token;
use crate::{
    AgeVerifier, Clock, CommentStore, CommentVoteStore, DemosStore, FoundingStore,
    InviteRequestStore, MediaStore, MediaVerdict, MembershipStore, NotificationStore, Notifier,
    NsfwScanner, PostStore, PostVoteStore, ProposalStore, RecommendFeed, RefreshRecommendations,
    ReportStore, RuleStore, SensitiveCaseStore, SettingsStore, SimilarityIndex, TrialCommentStore,
    TrialStore, UserStore, VoteStore,
};
use crate::{
    AcceptInviteError, ApproveInviteError, AuthenticateError, CanPostError, CastJuryVoteError,
    CastVoteError, CloseProposalError, CommentOnTrialError, CreatePostError, EnrollPublicKeyError,
    EnsureBarredAccountError, FoundDemosError, MemberActionError, OpenProposalError, OpenTrialError,
    RegisterAccountError, RequestInviteError, Result, SensitiveReviewError, SetFeedPagingError,
    SettleTrialError, SignFoundingError, StartFoundingError, StoreError, VerifyActionError,
    VotePostError,
};

use super::enfranchise_outcome::EnfranchiseOutcome;
use super::feed_item::FeedItem;
use super::member_metrics::MemberMetrics;
use super::search_results::SearchResults;
use super::search_scope::SearchScope;

/// Trailing window for the enfranchisement rate cap.
const RATE_CAP_WINDOW_DAYS: i64 = 30;

/// How many posts the site-wide "top" feed shows.
const TOP_FEED_LIMIT: usize = 50;

fn voting_window_days(kind: &ProposalKind) -> i64 {
    use domain::DecisionClass::*;
    match kind.decision_class() {
        Moderation => 3,
        RuleChange => 5,
        BanOrRecall => 5,
        Constitutional => 7,
    }
}

/// Alert the operator that a review upheld a CSAM classification. There is a legal
/// duty to preserve the material and report it to the NCMEC CyberTipline (18 U.S.C.
/// §2258A); the content is already taken down, but a human must act on the report.
/// Logged at ERROR so it cannot pass unnoticed. (Byte-level preservation to the
/// media quarantine is a follow-up — see docs/sensitive-content-review-plan.md.)
fn escalate_to_operator(target: ReportTarget) {
    tracing::error!(
        ?target,
        "SENSITIVE REVIEW: content classified as CSAM and removed — PRESERVE the material \
         and file a NCMEC CyberTipline report (18 U.S.C. §2258A)"
    );
}

/// The signed integer value of an up/down/clear vote: `+1` up, `-1` down, `0`
/// none. Used to subtract a member's own ballot from their contribution metric.
fn vote_value(dir: Option<bool>) -> i64 {
    match dir {
        Some(true) => 1,
        Some(false) => -1,
        None => 0,
    }
}

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

impl Services {
    // --- accounts & communities -------------------------------------------

    /// Register a credential-less account from a handle alone. This is the
    /// dev-only path (the account switcher) — the resulting user has no email or
    /// password and so can never be reached through [`authenticate`]. Real
    /// sign-ups go through [`register_account`](Self::register_account).
    pub async fn register_user(&self, handle: &str) -> Result<User> {
        if self.users.by_handle(handle).await?.is_some() {
            return Err(StoreError::AlreadyExists);
        }
        self.users
            .create(handle, None, None, self.clock.now())
            .await
    }

    /// Register a fresh credential-less account that is permanently franchise-barred
    /// — a dev/content "puppet". Used by the dev switcher's create/login-by-handle
    /// paths so anything it mints is a non-voter by construction. Errors if taken.
    pub async fn register_barred_user(&self, handle: &str) -> Result<User> {
        let user = self.register_user(handle).await?;
        self.users.set_franchise_barred(user.id, true).await?;
        self.users.get(user.id).await?.ok_or(StoreError::NotFound)
    }

    /// Idempotently ensure a franchise-barred puppet account exists for `handle`
    /// and return it. Boot-time provisioning for the fixed set of content accounts
    /// the dev switcher toggles between: a new handle is created barred; an existing
    /// one is (re)marked barred. Callers pass ONLY the operator-configured puppet
    /// handles — this will bar an existing account, so never hand it a real user's
    /// handle.
    pub async fn ensure_barred_account(
        &self,
        handle: &str,
    ) -> Result<User, EnsureBarredAccountError> {
        let handle = handle.trim();
        if handle.is_empty() {
            return Err(EnsureBarredAccountError::Rejected("handle required".into()));
        }
        let user = match self.users.by_handle(handle).await? {
            Some(u) => u,
            None => {
                self.users
                    .create(handle, None, None, self.clock.now())
                    .await?
            }
        };
        if !user.is_franchise_barred {
            self.users.set_franchise_barred(user.id, true).await?;
        }
        Ok(self.users.get(user.id).await?.ok_or(StoreError::NotFound)?)
    }

    /// Register a real account with email + password. Validates the credential
    /// policy, enforces handle and email uniqueness, hashes the password, and
    /// stores the account. The raw password never leaves this function.
    pub async fn register_account(
        &self,
        handle: &str,
        email: &str,
        password: &str,
    ) -> Result<User, RegisterAccountError> {
        let handle = handle.trim();
        if handle.is_empty() {
            return Err(RegisterAccountError::Rejected("handle is required".into()));
        }
        let email = domain::normalize_email(email);
        domain::validate_email(&email).map_err(|e| RegisterAccountError::Rejected(e.message()))?;
        domain::validate_password(password)
            .map_err(|e| RegisterAccountError::Rejected(e.message()))?;

        if self.users.by_handle(handle).await?.is_some() {
            return Err(RegisterAccountError::Rejected("that handle is taken".into()));
        }
        if self.users.by_email(&email).await?.is_some() {
            return Err(RegisterAccountError::Rejected(
                "an account with that email already exists".into(),
            ));
        }

        let hash = hash_password(password)?;
        Ok(self
            .users
            .create(handle, Some(&email), Some(&hash), self.clock.now())
            .await?)
    }

    // --- profile page -----------------------------------------------------

    /// Look up an account by handle — backs the public profile page at `/u/:handle`.
    pub async fn user_by_handle(&self, handle: &str) -> Result<Option<User>> {
        self.users.by_handle(handle.trim()).await
    }

    /// Every non-removed post by `author`, newest first. Filters the site-wide
    /// list (the same source search uses) — fine at a profile's scale.
    pub async fn posts_by_author(&self, author: UserId) -> Result<Vec<Post>> {
        let mut posts: Vec<Post> = self
            .posts
            .list_all()
            .await?
            .into_iter()
            .filter(|p| p.author == author && !p.removed)
            .collect();
        posts.sort_by(|a, b| b.created_at.0.cmp(&a.created_at.0));
        Ok(posts)
    }

    /// Every non-removed comment by `author`, newest first.
    pub async fn comments_by_author(&self, author: UserId) -> Result<Vec<Comment>> {
        let mut comments: Vec<Comment> = self
            .comments
            .list_by_author(author)
            .await?
            .into_iter()
            .filter(|c| !c.removed)
            .collect();
        comments.sort_by(|a, b| b.created_at.0.cmp(&a.created_at.0));
        Ok(comments)
    }

    // --- personal blocking ------------------------------------------------
    // A block is a purely personal mute: it hides the blocked account's content
    // from the blocker's own feeds and threads and has no governance effect (a
    // sanction is the community-moderation counterpart). One-directional and
    // unbounded — you may block as many accounts as you like.

    /// Block `target` for `blocker`. Blocking yourself is a no-op. Idempotent.
    pub async fn block_user(&self, blocker: UserId, target: UserId) -> Result<()> {
        if blocker == target {
            return Ok(());
        }
        self.users.block_user(blocker, target).await
    }

    /// Lift `blocker`'s block on `target`. Idempotent.
    pub async fn unblock_user(&self, blocker: UserId, target: UserId) -> Result<()> {
        self.users.unblock_user(blocker, target).await
    }

    /// The accounts `viewer` has blocked. Empty if the account is gone. Feeds and
    /// threads filter their content against this set so a blocked author never
    /// reaches the viewer.
    pub async fn blocked_by(&self, viewer: UserId) -> Result<Vec<UserId>> {
        Ok(self
            .users
            .get(viewer)
            .await?
            .map(|u| u.blocked)
            .unwrap_or_default())
    }

    /// Whether `blocker` currently blocks `target`.
    pub async fn is_blocking(&self, blocker: UserId, target: UserId) -> Result<bool> {
        Ok(self
            .users
            .get(blocker)
            .await?
            .is_some_and(|u| u.blocks(target)))
    }

    // --- notifications ----------------------------------------------------
    // Node-local, opt-in-per-kind pings: a member is notified when they are named
    // (`@handle`) in content or summoned to a jury. Generated at write time on
    // this node; never federated.

    /// This member's notifications, newest first.
    pub async fn notifications(&self, user: UserId) -> Result<Vec<Notification>> {
        self.notifications.list_for(user).await
    }

    /// How many unseen notifications this member has — the toolbar badge count.
    pub async fn unread_notification_count(&self, user: UserId) -> Result<u64> {
        self.notifications.unread_count(user).await
    }

    /// Mark all this member's notifications seen (they opened their list).
    pub async fn mark_notifications_seen(&self, user: UserId) -> Result<()> {
        self.notifications.mark_all_seen(user).await
    }

    /// Save which notification kinds this member wants.
    pub async fn set_alert_prefs(
        &self,
        user: UserId,
        allows_mention_alerts: bool,
        allows_jury_alerts: bool,
        allows_trial_comment_alerts: bool,
    ) -> Result<()> {
        self.users
            .set_alert_prefs(
                user,
                allows_mention_alerts,
                allows_jury_alerts,
                allows_trial_comment_alerts,
            )
            .await
    }

    /// Notify every account named as `@handle` in `text` that `author` mentioned
    /// them — skipping the author themselves, unknown handles, and anyone who has
    /// opted out of mention alerts. Best-effort: a mention notification never
    /// blocks the post/comment it rode in on, so this is called after the write
    /// succeeds and its own errors propagate only as a failed notification.
    async fn notify_mentions(
        &self,
        author: UserId,
        text: &str,
        post: PostId,
        comment: Option<CommentId>,
    ) -> Result<()> {
        let now = self.clock.now();
        for handle in mentions(text) {
            if let Some(user) = self.users.by_handle(&handle).await? {
                if user.id != author && user.allows_mention_alerts {
                    self.notifications
                        .push(
                            user.id,
                            NotificationKind::Mention {
                                post,
                                comment,
                                by: author,
                            },
                            now,
                        )
                        .await?;
                }
            }
        }
        Ok(())
    }

    // --- invitation-only access -------------------------------------------

    /// Whether new sign-ups currently require an invite. Reads the persisted
    /// operator toggle, falling back to `default_when_unset` (the node's boot
    /// flag) when it has never been set. Cheap enough to call per request.
    pub async fn is_invite_only(&self, default_when_unset: bool) -> Result<bool> {
        Ok(self
            .settings
            .is_invite_only()
            .await?
            .unwrap_or(default_when_unset))
    }

    /// Turn invitation-only access on or off, persisting the choice so it survives
    /// a restart.
    pub async fn set_invite_only(&self, invite_only: bool) -> Result<()> {
        self.settings.set_invite_only(invite_only).await
    }

    /// Take a request for an account from the public waitlist form.
    ///
    /// Deliberately idempotent and enumeration-safe: a blank email is rejected,
    /// but an email that already has a live request — or already belongs to an
    /// account — quietly returns `Ok` without creating a second row and without
    /// revealing which case it was. So the public form can never be used to probe
    /// who is already registered or already waiting.
    pub async fn request_invite(
        &self,
        email: &str,
        note: Option<&str>,
    ) -> Result<(), RequestInviteError> {
        let email = domain::normalize_email(email);
        domain::validate_email(&email).map_err(|e| RequestInviteError::Rejected(e.message()))?;

        // Already an account, or already on the list → no-op, no leak.
        if self.users.by_email(&email).await?.is_some() {
            return Ok(());
        }
        if self.invites.by_email(&email).await?.is_some() {
            return Ok(());
        }

        let note = note.map(str::trim).filter(|n| !n.is_empty());
        self.invites
            .create(&email, note, self.clock.now())
            .await?;
        Ok(())
    }

    /// The review queue: every request still awaiting a decision, oldest first.
    pub async fn list_pending_invites(&self) -> Result<Vec<InviteRequest>> {
        self.invites.list_pending().await
    }

    /// Approve a pending request: mint a one-time token, email the requester the
    /// accept link, and — only if the email is accepted for delivery — record the
    /// approval. The email goes out *before* the store is marked so a delivery
    /// failure leaves the request pending and retryable rather than approved yet
    /// unreachable.
    pub async fn approve_invite(&self, id: InviteId) -> Result<(), ApproveInviteError> {
        let request = self
            .invites
            .get(id)
            .await?
            .ok_or(ApproveInviteError::NotPending)?;
        if request.status != domain::InviteStatus::Pending {
            return Err(ApproveInviteError::NotPending);
        }

        let token = new_invite_token();
        let accept_url = format!(
            "{}/invite/accept?token={}",
            self.public_base_url.trim_end_matches('/'),
            token
        );
        // Deliver first — if this fails, the request stays Pending.
        self.notifier
            .notify_invite_approved(&request.email, &accept_url)
            .await?;

        let now = self.clock.now();
        let expires_at = now.plus_days(self.invite_token_ttl_days);
        self.invites
            .approve(id, &hash_token(&token), expires_at, now)
            .await?;
        Ok(())
    }

    /// Reject a pending request. No email is sent.
    pub async fn reject_invite(&self, id: InviteId) -> Result<(), ApproveInviteError> {
        let request = self
            .invites
            .get(id)
            .await?
            .ok_or(ApproveInviteError::NotPending)?;
        if request.status != domain::InviteStatus::Pending {
            return Err(ApproveInviteError::NotPending);
        }
        self.invites.reject(id, self.clock.now()).await?;
        Ok(())
    }

    /// Resolve a raw invite token to its (still-redeemable) request, so the accept
    /// flow can bind the new account to the invited email. Returns the opaque
    /// [`AcceptInviteError::InvalidToken`] for an unknown, expired, or already-used
    /// token alike.
    pub async fn validate_invite_token(
        &self,
        token: &str,
    ) -> Result<InviteRequest, AcceptInviteError> {
        let request = self
            .invites
            .by_token_hash(&hash_token(token))
            .await?
            .ok_or(AcceptInviteError::InvalidToken)?;
        if !request.is_redeemable(self.clock.now()) {
            return Err(AcceptInviteError::InvalidToken);
        }
        Ok(request)
    }

    /// Consume an approved invite once its account has been created — makes the
    /// token single-use.
    pub async fn mark_invite_accepted(&self, id: InviteId) -> Result<(), AcceptInviteError> {
        self.invites.mark_accepted(id).await?;
        Ok(())
    }

    /// Verify an email + password login. Returns the account on success and the
    /// opaque [`AuthenticateError::InvalidCredentials`] on any failure — unknown
    /// email, a credential-less account, or a wrong password all look identical.
    pub async fn authenticate(
        &self,
        email: &str,
        password: &str,
    ) -> Result<User, AuthenticateError> {
        let email = domain::normalize_email(email);
        let user = match self.users.by_email(&email).await? {
            Some(u) => u,
            None => {
                // Spend the same hashing time a real verification would, so an
                // unknown email is indistinguishable from a wrong password by
                // response timing — otherwise this branch returns near-instantly
                // and leaks which emails have accounts.
                spend_verify_time(password);
                return Err(AuthenticateError::InvalidCredentials);
            }
        };
        let matches = match user.password_hash.as_deref() {
            Some(hash) => verify_password(password, hash),
            None => {
                // A credential-less account (dev/seed user) can't be logged into,
                // but must still cost the same to reject as any other account.
                spend_verify_time(password);
                false
            }
        };
        if matches {
            Ok(user)
        } else {
            Err(AuthenticateError::InvalidCredentials)
        }
    }

    /// Verify a **handle** + password login. The federated equivalent of
    /// [`authenticate`](Self::authenticate): handles replicate across nodes (emails
    /// do not — they are redacted from the feed), so cross-node login and the
    /// delegated-auth path a trusted issuer runs both key on the handle. Failure is
    /// the same opaque [`AuthenticateError::InvalidCredentials`], and an unknown or
    /// credential-less handle still spends the verification time so account existence
    /// never leaks by timing.
    pub async fn authenticate_by_handle(
        &self,
        handle: &str,
        password: &str,
    ) -> Result<User, AuthenticateError> {
        let user = match self.users.by_handle(handle.trim()).await? {
            Some(u) => u,
            None => {
                spend_verify_time(password);
                return Err(AuthenticateError::InvalidCredentials);
            }
        };
        let matches = match user.password_hash.as_deref() {
            Some(hash) => verify_password(password, hash),
            None => {
                spend_verify_time(password);
                false
            }
        };
        if matches {
            Ok(user)
        } else {
            Err(AuthenticateError::InvalidCredentials)
        }
    }

    /// Every registered account. Backs dev tooling that enumerates and switches
    /// between test users.
    pub async fn list_users(&self) -> Result<Vec<User>> {
        self.users.list().await
    }

    /// Enrol an account's Ed25519 public signing key (hex). Once set, this account's
    /// governance actions must carry a signature verifiable against this key (see
    /// [`cast_vote`](Self::cast_vote)). Enrolment is **first-key-only** here: a key
    /// cannot be silently replaced, since that would let whoever currently controls
    /// the account (e.g. a compromised node session) swap in their own key and take
    /// over signing. Key *rotation* — replacing a key with one authorised by the old
    /// key — is a deliberate follow-up, not a bare overwrite.
    pub async fn enroll_public_key(
        &self,
        user: UserId,
        public_key_hex: &str,
    ) -> Result<(), EnrollPublicKeyError> {
        if !is_valid_public_key(public_key_hex) {
            return Err(EnrollPublicKeyError::Rejected(
                "that is not a valid signing key".into(),
            ));
        }
        let u = self.users.get(user).await?.ok_or(StoreError::NotFound)?;
        if u.public_key.is_some() {
            return Err(EnrollPublicKeyError::Rejected(
                "this account already has a signing key".into(),
            ));
        }
        Ok(self.users.set_public_key(user, public_key_hex).await?)
    }

    /// Record how a member wants long feeds delivered (paged vs. lazy-loaded). A
    /// plain account preference with no policy attached — the store just persists it.
    pub async fn set_feed_paging(
        &self,
        user: UserId,
        paging: FeedPaging,
    ) -> Result<(), SetFeedPagingError> {
        Ok(self.users.set_feed_paging(user, paging).await?)
    }

    /// Verify that `user` authorised `message` with a signature over it. The policy:
    ///
    /// * The account **has** an enrolled key → a signature is **required** and must
    ///   verify against that key. This is the open-federation guarantee: no node can
    ///   forge the action, because it lacks the user's secret key.
    /// * The account has **no** key yet (legacy / not-yet-enrolled) → the action is
    ///   allowed unsigned. Enrolling a key is the irreversible opt-in to enforcement,
    ///   so a fleet migrates account-by-account without a flag day. (A deployment-wide
    ///   "require signatures from everyone" switch is a natural next step.)
    async fn verify_user_action(
        &self,
        user: UserId,
        message: &str,
        sig: Option<&str>,
    ) -> Result<(), VerifyActionError> {
        let u = self.users.get(user).await?.ok_or(StoreError::NotFound)?;
        let Some(key_hex) = u.public_key.as_deref() else {
            // No enrolled key. During rollout (`require_signatures == false`) this is
            // the node-trusted fallback; once signatures are mandatory, a key-less
            // account cannot act — otherwise a malicious node could forge its ballot.
            if self.require_signatures {
                return Err(VerifyActionError::Rejected(
                    "this deployment requires a signing key — enrol one to act".into(),
                ));
            }
            return Ok(());
        };
        let key = UserPublicKey::from_hex(key_hex).ok_or_else(|| {
            VerifyActionError::Store(StoreError::Store(
                "account has a malformed signing key".into(),
            ))
        })?;
        match sig {
            Some(s) if key.verify(message, s) => Ok(()),
            _ => Err(VerifyActionError::Rejected(
                "this action must be signed by the account's key".into(),
            )),
        }
    }

    /// Found a demos. The founder becomes voter #1 (bootstrap), placing the
    /// demos in its Seed phase. Founder influence dilutes structurally as the
    /// demos crosses phase boundaries.
    /// A franchise-barred account (a dev/content puppet) may take NO path to the
    /// franchise. `evaluate_eligibility` already refuses the normal request; this
    /// guards the founding shortcuts, which enfranchise the founder and co-signers
    /// directly, bypassing the eligibility check. Belt-and-suspenders with the
    /// domain rule so the bar holds however enfranchisement is reached.
    async fn ensure_not_barred(&self, user: UserId) -> Result<(), FoundDemosError> {
        let u = self.users.get(user).await?.ok_or(StoreError::NotFound)?;
        if u.is_franchise_barred {
            return Err(FoundDemosError::Rejected(
                "this account is barred from the franchise".into(),
            ));
        }
        Ok(())
    }

    pub async fn found_demos(
        &self,
        founder: UserId,
        slug: &str,
        name: &str,
    ) -> Result<Demos, FoundDemosError> {
        self.found_demos_tagged(founder, slug, name, Vec::new()).await
    }

    /// [`found_demos`](Self::found_demos) with founder-chosen topic `tags` (already
    /// normalized). The petition-driven path founds through here so a community's
    /// tags — captured when the petition opened — land on the demos it becomes.
    async fn found_demos_tagged(
        &self,
        founder: UserId,
        slug: &str,
        name: &str,
        tags: Vec<String>,
    ) -> Result<Demos, FoundDemosError> {
        self.ensure_not_barred(founder).await?;
        if self.demoi.by_slug(slug).await?.is_some() {
            return Err(StoreError::AlreadyExists.into());
        }
        let now = self.clock.now();
        let demos = self.demoi.create(slug, name, founder, tags, now).await?;

        let mut m = Membership::joined(founder, demos.id, now);
        m.tier = Tier::Voter;
        m.enfranchised_at = Some(now);
        self.memberships.upsert(m).await?;

        Ok(demos)
    }

    /// Open a founding petition. A demos is no longer created outright: the
    /// founder proposes a name (its slug is derived here, the single source of
    /// truth) and must gather [`domain::SIGN_OFFS_REQUIRED`] co-signers before it
    /// becomes real — see [`sign_founding`](Self::sign_founding). Rejected if the
    /// slug is already taken by a live demos or another open petition.
    pub async fn start_founding(
        &self,
        founder: UserId,
        name: &str,
    ) -> Result<FoundingPetition, StartFoundingError> {
        self.start_founding_tagged(founder, name, Vec::new()).await
    }

    /// [`start_founding`](Self::start_founding) with founder-chosen topic `tags`
    /// (already normalized by the caller, as post tags are). They are carried on
    /// the petition until the community is founded, then applied to the demos.
    pub async fn start_founding_tagged(
        &self,
        founder: UserId,
        name: &str,
        tags: Vec<String>,
    ) -> Result<FoundingPetition, StartFoundingError> {
        self.ensure_not_barred(founder).await?;
        let name = name.trim();
        let slug = slugify(name);
        if slug.is_empty() {
            return Err(StartFoundingError::Rejected(
                "a community name needs at least one letter or number".into(),
            ));
        }
        if self.demoi.by_slug(&slug).await?.is_some() {
            return Err(StoreError::AlreadyExists.into());
        }
        if self.foundings.list().await?.iter().any(|p| p.slug == slug) {
            return Err(StoreError::AlreadyExists.into());
        }
        Ok(self
            .foundings
            .create(&slug, name, founder, tags, self.clock.now())
            .await?)
    }

    pub async fn founding(&self, id: FoundingId) -> Result<Option<FoundingPetition>> {
        self.foundings.get(id).await
    }

    /// Every founding still gathering sign-offs, newest first.
    pub async fn pending_foundings(&self) -> Result<Vec<FoundingPetition>> {
        self.foundings.list().await
    }

    /// Sign off on a pending founding. Idempotent per user; the founder cannot
    /// sign their own. When the final required co-signer commits, the demos is
    /// founded for real — the founder **and every co-signer** become founding
    /// voters, so a community is born with all ten already enfranchised (which
    /// lands it past the Seed phase) — and the petition is cleared. Returns the
    /// founded [`Demos`] when quorum was reached this call, otherwise `None`.
    pub async fn sign_founding(
        &self,
        id: FoundingId,
        user: UserId,
    ) -> Result<Option<Demos>, SignFoundingError> {
        let petition = self.foundings.get(id).await?.ok_or(StoreError::NotFound)?;
        if user == petition.founder {
            return Err(SignFoundingError::Rejected(
                "the founder already backs this founding".into(),
            ));
        }
        // Co-signing enfranchises the signer when quorum lands, so a barred puppet
        // must not be able to sign its way into the franchise.
        self.ensure_not_barred(user).await?;
        let petition = self.foundings.sign(id, user).await?;
        if !petition.is_ready() {
            return Ok(None);
        }

        // Quorum reached — found the demos (which enfranchises the founder), then
        // enfranchise every co-signer as a founding voter too. The founder's tags,
        // captured when the petition opened, are applied to the new community here.
        let demos = self
            .found_demos_tagged(
                petition.founder,
                &petition.slug,
                &petition.name,
                petition.tags.clone(),
            )
            .await?;
        let now = self.clock.now();
        for signer in &petition.sign_offs {
            let mut m = Membership::joined(*signer, demos.id, now);
            m.tier = Tier::Voter;
            m.enfranchised_at = Some(now);
            self.memberships.upsert(m).await?;
        }
        self.foundings.delete(id).await?;
        Ok(Some(demos))
    }

    pub async fn join(&self, user: UserId, demos: DemosId) -> Result<Membership> {
        if let Some(existing) = self.memberships.get(user, demos).await? {
            return Ok(existing);
        }
        let m = Membership::joined(user, demos, self.clock.now());
        self.memberships.upsert(m.clone()).await?;
        Ok(m)
    }

    /// Stand-in for "a contribution was positively received by existing voters".
    /// The anti-gaming mechanism behind this number is an open design question;
    /// here we just adjust the stored score.
    pub async fn record_contribution(
        &self,
        user: UserId,
        demos: DemosId,
        delta: i64,
    ) -> Result<()> {
        let mut m = self
            .memberships
            .get(user, demos)
            .await?
            .ok_or(StoreError::NotFound)?;
        // Saturating, not wrapping: a caller-supplied `i64` delta must never wrap
        // the stored score (release builds wrap silently), which gates franchise
        // eligibility, posting policy, and vote weight.
        m.contribution = m.contribution.saturating_add(delta);
        self.memberships.upsert(m).await
    }

    pub async fn phase_of(&self, demos: DemosId) -> Result<Phase> {
        Ok(Phase::from_voter_count(
            self.memberships.voter_count(demos).await?,
        ))
    }

    // --- the franchise (Layers 1 & 2) -------------------------------------

    pub async fn check_eligibility(&self, user: UserId, demos: DemosId) -> Result<Eligibility> {
        let (u, m, d) = self.load_triplet(user, demos).await?;
        Ok(evaluate_eligibility(&u, &m, &d.criteria, self.clock.now()))
    }

    /// Apply Layer 1 (eligibility) then Layer 2 (rate cap) to admit a member to
    /// the franchise, or explain why not.
    pub async fn request_enfranchisement(
        &self,
        user: UserId,
        demos: DemosId,
    ) -> Result<EnfranchiseOutcome> {
        let now = self.clock.now();
        let (u, mut m, d) = self.load_triplet(user, demos).await?;

        if m.is_voter() {
            return Ok(EnfranchiseOutcome::Admitted);
        }

        let eligibility = evaluate_eligibility(&u, &m, &d.criteria, now);
        if !eligibility.is_eligible() {
            return Ok(EnfranchiseOutcome::NotEligible(eligibility));
        }

        // Layer 2: is there an open admission slot this window?
        let voters = self.memberships.voter_count(demos).await?;
        let window_start = Timestamp(now.0 - RATE_CAP_WINDOW_DAYS * Timestamp::SECONDS_PER_DAY);
        let admitted = self.memberships.admitted_since(demos, window_start).await?;
        if enfranchisement_slots(voters, admitted) == 0 {
            return Ok(EnfranchiseOutcome::Queued);
        }

        m.tier = Tier::Voter;
        m.enfranchised_at = Some(now);
        self.memberships.upsert(m).await?;
        Ok(EnfranchiseOutcome::Admitted)
    }

    // --- governance (Layers 3 & 4) ----------------------------------------

    pub async fn open_proposal(
        &self,
        proposer: UserId,
        demos: DemosId,
        kind: ProposalKind,
    ) -> Result<Proposal, OpenProposalError> {
        let membership = self
            .memberships
            .get(proposer, demos)
            .await?
            .ok_or(OpenProposalError::NotAVoter)?;
        if !membership.is_voter() {
            return Err(OpenProposalError::NotAVoter);
        }
        // A sanction disqualifies from the franchise, so a convicted member may
        // not open proposals either — even though they keep the `Voter` tier
        // until they re-qualify. Without this, a jury-convicted member could file
        // retaliatory proposals (Ban / Recall / AmendCriteria) against their
        // accuser. Mirrors the same gate on cast_vote / open_trial / the content
        // paths' `require_unsanctioned_member`.
        if membership.is_sanctioned(self.clock.now()) {
            return Err(OpenProposalError::Sanctioned);
        }

        // Training wheels: no constitutional change while in Seed.
        if kind.decision_class() == domain::DecisionClass::Constitutional
            && self.phase_of(demos).await? == Phase::Seed
        {
            return Err(OpenProposalError::ConstitutionalForbiddenInSeed);
        }

        // One live decision per intent: an identical proposal already open in
        // this demos would let the electorate vote on the same question twice
        // (e.g. two concurrent proposals to remove the same post), so reject it.
        let already_open = self
            .proposals
            .list(demos)
            .await?
            .into_iter()
            .any(|p| p.status == ProposalStatus::Open && p.kind == kind);
        if already_open {
            return Err(OpenProposalError::DuplicateOpenProposal);
        }

        let now = self.clock.now();
        let closes_at = now.plus_days(voting_window_days(&kind));
        Ok(self
            .proposals
            .create(demos, proposer, kind, now, closes_at)
            .await?)
    }

    pub async fn cast_vote(
        &self,
        proposal: ProposalId,
        voter: UserId,
        aye: bool,
        sig: Option<&str>,
    ) -> Result<(), CastVoteError> {
        let p = self
            .proposals
            .get(proposal)
            .await?
            .ok_or(StoreError::NotFound)?;
        if p.status != ProposalStatus::Open {
            return Err(CastVoteError::ProposalNotOpen);
        }
        // The voting window is a fixed deliberation period. A proposal remains
        // `Open` from `closes_at` until someone calls `close_proposal`; without
        // this guard a voter could let the window lapse and then cast the deciding
        // ballot after the deadline the rest of the electorate treated as final
        // (vote-sniping / timelock bypass). Enforce the window at cast time, not
        // only at close time.
        if self.clock.now() >= p.closes_at {
            return Err(CastVoteError::VotingWindowClosed);
        }
        let membership = self
            .memberships
            .get(voter, p.demos_id)
            .await?
            .ok_or(CastVoteError::NotAVoter)?;
        if !membership.is_voter() {
            return Err(CastVoteError::NotAVoter);
        }
        // A sanction disqualifies from the franchise, so a convicted voter may not
        // cast a governance ballot — even though they keep the `Voter` tier until
        // they re-qualify. Mirrors the content paths' `require_unsanctioned_member`.
        if membership.is_sanctioned(self.clock.now()) {
            return Err(CastVoteError::Sanctioned);
        }
        // The ballot must be signed by the *acting user*, verified against the
        // account's enrolled key. This is what makes a vote unforgeable by the
        // node hosting the account (or any relay): the owner re-runs this check,
        // never trusting a forwarding node's word for who voted. Enforced here, on
        // the authoritative owner, so it holds for both local and forwarded votes.
        self.verify_user_action(voter, &vote_message(proposal.0, aye), sig)
            .await?;
        let now = self.clock.now();
        let demos = self
            .demoi
            .get(p.demos_id)
            .await?
            .ok_or(StoreError::NotFound)?;
        let weight = if demos.weighting_scope.applies_to_proposals() {
            demos.vote_weighting.weight_of(&membership, now)
        } else {
            1
        };
        Ok(self.votes.cast(proposal, voter, aye, weight, now).await?)
    }

    /// Tally and close a proposal, applying the phase-appropriate threshold and
    /// (for constitutional changes) the timelock. A constitutional change that
    /// has already passed its recall window is applied to the demos's criteria.
    pub async fn close_proposal(
        &self,
        proposal: ProposalId,
    ) -> Result<ProposalStatus, CloseProposalError> {
        let mut p = self
            .proposals
            .get(proposal)
            .await?
            .ok_or(StoreError::NotFound)?;
        let now = self.clock.now();
        // Accumulate mutations and persist once at the end: a single `update` means
        // a single optimistic-concurrency `rev` bump, so the two phases below can't
        // conflict with each other on the same in-memory struct.
        let mut dirty = false;

        // Phase 1 — decide an *open* proposal, but only after its voting window has
        // elapsed. Without the window check any voter could close a proposal the
        // instant it opened, freezing the tally at a hand-picked moment and denying
        // the rest of the electorate the deliberation window (a vote-sniping /
        // timelock-bypass attack). A proposal that is no longer Open falls through
        // to phase 2 unchanged.
        if p.status == ProposalStatus::Open {
            if now < p.closes_at {
                return Err(CloseProposalError::VotingWindowOpen);
            }
            let tally = self.votes.tally(proposal).await?;
            let voters = self.memberships.voter_count(p.demos_id).await?;
            let phase = Phase::from_voter_count(voters);
            let demos = self
                .demoi
                .get(p.demos_id)
                .await?
                .ok_or(StoreError::NotFound)?;
            // The quorum denominator must be in the same units as the (possibly
            // weighted) tally: total electorate weight when proposals are weighted,
            // otherwise the plain voter head count. Phase is always head-count based.
            let electorate = if demos.weighting_scope.applies_to_proposals()
                && demos.vote_weighting != VoteWeighting::Equal
            {
                self.total_voter_weight(p.demos_id, demos.vote_weighting, now)
                    .await?
            } else {
                voters
            };
            p.close(tally, electorate, phase, now);
            dirty = true;
        }

        // Phase 2 — apply the effects of a passed proposal once its timelock has
        // matured, and *exactly once*. The `applied` flag makes re-invocation
        // idempotent: without it, repeatedly closing a passed `AddRule` would add
        // the rule again on every call, and re-closing a timelocked amendment would
        // keep pushing its `effective_at` forward, stalling it indefinitely.
        if let ProposalStatus::Passed { effective_at } = p.status {
            if !p.applied && effective_at <= now {
                // Claim the proposal BEFORE running its effects: mark it applied and
                // persist under optimistic concurrency first. Two concurrent closers
                // (trivial across shared-DB replicas) would otherwise both observe
                // `applied == false`, both run the side effects, and only then
                // contend on the rev-CAS — double-creating a rule for a passed
                // `AddRule`. Claiming first means exactly one caller wins the rev
                // bump; the loser's `update` returns `Conflict` and never reaches
                // the effects. This single `update` also persists any Phase-1 close.
                p.applied = true;
                self.proposals.update(&p).await?;
                self.apply_passed_effects(&p).await?;
                return Ok(p.status);
            }
        }
        if dirty {
            self.proposals.update(&p).await?;
        }
        Ok(p.status)
    }

    /// Apply the demos-level effects of a proposal that has passed and matured.
    /// Called at most once per proposal (guarded by `Proposal::applied`), so every
    /// arm here may assume it runs a single time.
    async fn apply_passed_effects(&self, p: &Proposal) -> Result<()> {
        match &p.kind {
            ProposalKind::AmendCriteria { proposed } => {
                self.demoi
                    .update_criteria(p.demos_id, proposed.clone())
                    .await?;
            }
            ProposalKind::AddRule {
                text,
                sanction_days,
            } => {
                // Clamp the voted term to the community ceiling now, at enactment,
                // so a stored rule never carries a term the demos hasn't sanctioned
                // (`0` = "inherit the ceiling", left as-is). Convictions clamp again
                // to the live ceiling, so lowering the ceiling later still binds.
                let capped = match self.demoi.get(p.demos_id).await? {
                    Some(d) if *sanction_days != 0 => d.cap_sanction_days(*sanction_days),
                    _ => *sanction_days,
                };
                self.rules
                    .create(p.demos_id, text, capped, self.clock.now())
                    .await?;
            }
            ProposalKind::RemoveRule { rule } => {
                self.rules.set_active(*rule, false).await?;
            }
            ProposalKind::SetMaxSanction { days } => {
                // Bound the community's own ceiling by the platform cap — no demos
                // can vote a permaban. Stored clamped; every downstream term reads
                // it back through `Demos::ban_ceiling_days`.
                let capped = (*days).min(MAX_SANCTION_DAYS);
                self.demoi.set_max_sanction(p.demos_id, capped).await?;
            }
            ProposalKind::SetNsfwPolicy { allows_nsfw } => {
                self.demoi.set_allows_nsfw(p.demos_id, *allows_nsfw).await?;
            }
            ProposalKind::SetJurySizing { sizing } => {
                self.demoi.set_jury_sizing(p.demos_id, *sizing).await?;
            }
            ProposalKind::SetVoteWeighting { scheme } => {
                self.demoi.set_vote_weighting(p.demos_id, *scheme).await?;
            }
            ProposalKind::SetWeightingScope { scope } => {
                self.demoi.set_weighting_scope(p.demos_id, *scope).await?;
            }
            ProposalKind::SetPostingPolicy { policy } => {
                self.demoi.set_posting_policy(p.demos_id, *policy).await?;
            }
            ProposalKind::GrantVoteWeight { user, weight } => {
                if let Some(mut m) = self.memberships.get(*user, p.demos_id).await? {
                    m.granted_weight = *weight;
                    self.memberships.upsert(m).await?;
                }
            }
            ProposalKind::Ban { user } => {
                // A passed ban sanctions the member — stripping the franchise via
                // the same mechanism a guilty jury verdict applies. Without this,
                // a community could vote (at the 60% BanOrRecall bar) to ban a user
                // and have nothing happen: the electorate's decision was silently
                // ignored.
                if let Some(mut m) = self.memberships.get(*user, p.demos_id).await? {
                    // A direct ban proposal isn't tied to a specific rule, so it
                    // runs to the community's own ceiling — which `sanction_for`
                    // still caps at MAX_SANCTION_DAYS (18 years), so a vote can never
                    // permaban.
                    let term = match self.demoi.get(p.demos_id).await? {
                        Some(d) => d.ban_ceiling_days(),
                        None => MAX_SANCTION_DAYS,
                    };
                    m.sanction_for(self.clock.now(), term);
                    self.memberships.upsert(m).await?;
                }
            }
            // Deliberately inert (fail-safe: no unauthorized effect), pending a
            // model that lets them be actioned:
            //   * Recall targets a leadership "office" the membership model does not
            //     yet represent (tiers are Lurker/Member/Voter; the founder identity
            //     is immutable history) — nothing to strip.
            //   * RemoveContent carries a free-text `target`, not a structured
            //     content id, so it cannot resolve a post/comment to remove. Content
            //     removal today runs through the report → jury verdict path.
            // Both need a schema change before they can apply an effect.
            ProposalKind::Recall { .. } | ProposalKind::RemoveContent { .. } => {}
        }
        Ok(())
    }

    pub async fn list_rules(&self, demos: DemosId) -> Result<Vec<Rule>> {
        self.rules.list_active(demos).await
    }

    // --- content (posts & comments) ---------------------------------------

    /// Create a post. The author must be a member in good standing. After
    /// posting, the bot detector runs and may file an automatic report.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_post(
        &self,
        author: UserId,
        demos: DemosId,
        title: &str,
        body: &str,
        media: Vec<Media>,
        tags: Vec<String>,
    ) -> Result<Post, CreatePostError> {
        self.require_can_post(author, demos).await?;
        let now = self.clock.now();
        let mut post = self
            .posts
            .create(demos, author, title, body, media, tags, now)
            .await?;
        self.run_bot_check(author, demos, now).await?;
        post.is_nsfw = self.run_nsfw_check(&post, now).await?;
        // Ping anyone named in the title or body (their opt-in is checked inside).
        self.notify_mentions(author, &format!("{title} {body}"), post.id, None)
            .await?;
        Ok(post)
    }

    /// Flag a post NSFW when its text, tags, or media look explicit. In a
    /// community that has *voted to forbid* NSFW, a flagged post also auto-files
    /// a report for a jury — "the machine flags; the demos judges". It never
    /// removes the post itself. Returns whether it was flagged.
    async fn run_nsfw_check(&self, post: &Post, now: Timestamp) -> Result<bool> {
        let text = format!(
            "{} {} {}",
            post.title,
            post.text_content(),
            post.tags.join(" ")
        );
        let mut is_nsfw = is_nsfw_text(&text) || post.tags.iter().any(|t| t == "nsfw");
        if !is_nsfw {
            // Scan each attachment; any explicit item flags the whole post.
            for m in &post.media {
                let verdict = self
                    .nsfw_scanner
                    .scan_media(&m.url, &m.caption, m.kind_label())
                    .await?;
                if verdict == MediaVerdict::Nsfw {
                    is_nsfw = true;
                    break;
                }
            }
        }
        if !is_nsfw {
            return Ok(false);
        }
        self.posts.set_is_nsfw(post.id, true).await?;

        let demos = self
            .demoi
            .get(post.demos_id)
            .await?
            .ok_or(StoreError::NotFound)?;
        if !demos.allows_nsfw {
            // Folds into any open report already on this post (and `add_flag`
            // ignores a duplicate NSFW flag if the scanner re-runs).
            self.file_or_merge_flag(
                post.demos_id,
                None,
                ReportTarget::Post(post.id),
                ReportReason::Nsfw,
                "automatic: NSFW content in a community that forbids it",
                now,
            )
            .await?;
        }
        Ok(true)
    }

    // --- sensitive-content review (platform-wide, extra-demos) ------------

    /// Opt an account in to (or out of) reviewing platform-wide sensitive content.
    /// Default off; deliberately not a demos tier.
    pub async fn set_sensitive_reviewer(
        &self,
        user: UserId,
        is_reviewer: bool,
    ) -> Result<(), SensitiveReviewError> {
        Ok(self.users.set_sensitive_reviewer(user, is_reviewer).await?)
    }

    /// Flag a post/comment as sensitive. Any signed-in user may flag; the content
    /// is **hidden pending review immediately** and a platform-wide review case is
    /// opened (or the flag merges into the open one). Returns the case.
    pub async fn flag_sensitive(
        &self,
        reporter: UserId,
        target: ReportTarget,
        note: &str,
    ) -> Result<SensitiveCase, SensitiveReviewError> {
        // Hide the target now — flagging errs toward caution.
        match target {
            ReportTarget::Post(p) => {
                if self.posts.get(p).await?.is_none() {
                    return Err(SensitiveReviewError::Rejected("no such post".into()));
                }
                self.posts.set_pending_review(p, true).await?;
            }
            ReportTarget::Comment(c) => {
                if self.comments.get(c).await?.is_none() {
                    return Err(SensitiveReviewError::Rejected("no such comment".into()));
                }
                self.comments.set_pending_review(c, true).await?;
            }
            ReportTarget::User(_) => {
                return Err(SensitiveReviewError::Rejected(
                    "only posts and comments can be flagged sensitive".into(),
                ))
            }
        }
        let now = self.clock.now();
        match self.sensitive_cases.open_for_target(target).await? {
            Some(case) => Ok(case),
            None => Ok(self
                .sensitive_cases
                .create(Some(reporter), target, note, now)
                .await?),
        }
    }

    /// The open review queue — reviewer-only.
    pub async fn list_review_queue(
        &self,
        reviewer: UserId,
    ) -> Result<Vec<SensitiveCase>, SensitiveReviewError> {
        self.require_sensitive_reviewer(reviewer).await?;
        Ok(self.sensitive_cases.list_open().await?)
    }

    /// How many cases are open — backs the reviewer nav badge.
    pub async fn open_case_count(&self) -> Result<u64> {
        self.sensitive_cases.count_open().await
    }

    /// Cast a reviewer's classification on a case. Reviewer-only; one vote per
    /// reviewer (a repeat corrects it). Once at least
    /// [`REVIEW_QUORUM`](domain::REVIEW_QUORUM) reviewers have voted, the plurality
    /// tag resolves the case and its disposition is applied to the content.
    pub async fn cast_review_vote(
        &self,
        reviewer: UserId,
        case_id: SensitiveCaseId,
        tag: SensitiveTag,
    ) -> Result<SensitiveCase, SensitiveReviewError> {
        self.require_sensitive_reviewer(reviewer).await?;
        let mut case = self
            .sensitive_cases
            .get(case_id)
            .await?
            .ok_or(StoreError::NotFound)?;
        if case.status != domain::SensitiveCaseStatus::Open {
            return Err(SensitiveReviewError::AlreadyResolved);
        }
        let now = self.clock.now();
        case.cast(reviewer, tag, now);
        // Resolve if the quorum is now met, and carry out the disposition.
        if let Some(winner) = case.try_resolve() {
            self.apply_review_outcome(case.target, outcome_for(winner)).await?;
        }
        self.sensitive_cases.update(&case).await?;
        Ok(case)
    }

    /// Carry out a resolved case's disposition on its target.
    async fn apply_review_outcome(
        &self,
        target: ReportTarget,
        outcome: ReviewOutcome,
    ) -> Result<(), SensitiveReviewError> {
        match (target, outcome) {
            // False flag → un-hide, unchanged.
            (ReportTarget::Post(p), ReviewOutcome::Restore) => {
                self.posts.set_pending_review(p, false).await?;
            }
            (ReportTarget::Comment(c), ReviewOutcome::Restore) => {
                self.comments.set_pending_review(c, false).await?;
            }
            // Lawful adult content → un-hide but NSFW-gate (posts carry the flag;
            // comments have no NSFW blur, so they are simply restored).
            (ReportTarget::Post(p), ReviewOutcome::AgeGate) => {
                self.posts.set_is_nsfw(p, true).await?;
                self.posts.set_pending_review(p, false).await?;
            }
            (ReportTarget::Comment(c), ReviewOutcome::AgeGate) => {
                self.comments.set_pending_review(c, false).await?;
            }
            // Upheld → take down platform-wide.
            (ReportTarget::Post(p), ReviewOutcome::Remove { escalate }) => {
                self.posts.set_removed(p, true).await?;
                self.posts.set_pending_review(p, false).await?;
                if escalate {
                    escalate_to_operator(target);
                }
            }
            (ReportTarget::Comment(c), ReviewOutcome::Remove { escalate }) => {
                self.comments.set_removed(c, true).await?;
                self.comments.set_pending_review(c, false).await?;
                if escalate {
                    escalate_to_operator(target);
                }
            }
            (ReportTarget::User(_), _) => {}
        }
        Ok(())
    }

    /// The reviewer gate: the account must have opted in to sensitive-content
    /// review. Deliberately a platform account attribute, not a demos membership.
    async fn require_sensitive_reviewer(
        &self,
        user: UserId,
    ) -> Result<(), SensitiveReviewError> {
        let u = self
            .users
            .get(user)
            .await?
            .ok_or(StoreError::NotFound)?;
        if !u.is_sensitive_reviewer {
            return Err(SensitiveReviewError::NotReviewer);
        }
        Ok(())
    }

    // --- NSFW age gate & verification -------------------------------------

    /// Run age verification for `user` through the provider; on success persist
    /// the result. Returns whether the user is now verified.
    pub async fn verify_age(&self, user: UserId) -> Result<bool> {
        let is_verified = self.age_verifier.verify(user).await?;
        if is_verified {
            self.users.set_is_age_verified(user, true).await?;
        }
        Ok(is_verified)
    }

    /// How `post` should be presented to `viewer` under the deployment's age
    /// policy — the one place the gate is decided ([`domain::visibility`]).
    pub async fn post_visibility(&self, post: &Post, viewer: UserId) -> Result<Visibility> {
        let is_viewer_age_verified = self
            .users
            .get(viewer)
            .await?
            .map_or(false, |u| u.is_age_verified);
        Ok(visibility(
            post.is_nsfw,
            is_viewer_age_verified,
            self.requires_age_verification,
        ))
    }

    /// Full-text-ish search over posts (title / body / tags) and, site-wide,
    /// communities (name / slug). A post matches if **any** query token is a
    /// substring of its title or body; an optional `tag` filter additionally
    /// requires that exact tag. When a `tag` is given the candidate set comes
    /// from the store's pipe-wrapped tag index ([`PostStore::by_tag`] /
    /// [`DemosStore::by_tag`]) — an indexed lookup rather than a scan of every
    /// row — and any query tokens then narrow those matches. Removed and
    /// pending-review posts are excluded.
    pub async fn search(
        &self,
        query: &str,
        scope: SearchScope,
        tag: Option<&str>,
    ) -> Result<SearchResults> {
        let tokens: Vec<String> = query.split_whitespace().map(|t| t.to_lowercase()).collect();
        // Normalize the tag filter the same way stored tags are, so it matches the
        // index exactly and can never carry a `LIKE` metacharacter into the store.
        let tag = tag.and_then(|t| normalize_tags(t).into_iter().next());

        // With a tag filter, fetch the tagged rows straight from the index; without
        // one, fall back to the full candidate list the tokens then filter.
        let candidates = match (&tag, scope) {
            (Some(t), SearchScope::All) => self.posts.by_tag(None, t).await?,
            (Some(t), SearchScope::Demos(id)) => self.posts.by_tag(Some(id), t).await?,
            (None, SearchScope::All) => self.posts.list_all().await?,
            (None, SearchScope::Demos(id)) => self.posts.list(id).await?,
        };
        let posts = candidates
            .into_iter()
            .filter(|p| !p.removed && !p.pending_review)
            .filter(|p| tokens.is_empty() || tokens.iter().any(|tok| post_matches(p, tok)))
            .collect();

        // Communities are only searched in the site-wide scope. A tag filter looks
        // them up by the same index; otherwise they match on name/slug tokens.
        let communities = if !matches!(scope, SearchScope::All) {
            Vec::new()
        } else if let Some(t) = &tag {
            self.demoi
                .by_tag(t)
                .await?
                .into_iter()
                .filter(|d| {
                    tokens.is_empty() || {
                        let name = d.name.to_lowercase();
                        let slug = d.slug.to_lowercase();
                        tokens.iter().any(|tok| name.contains(tok) || slug.contains(tok))
                    }
                })
                .collect()
        } else if !tokens.is_empty() {
            self.demoi
                .list()
                .await?
                .into_iter()
                .filter(|d| {
                    let name = d.name.to_lowercase();
                    let slug = d.slug.to_lowercase();
                    tokens.iter().any(|t| name.contains(t) || slug.contains(t))
                })
                .collect()
        } else {
            Vec::new()
        };

        Ok(SearchResults { posts, communities })
    }

    /// Reply to a post (or, with `parent`, to another comment).
    pub async fn comment(
        &self,
        author: UserId,
        post_id: PostId,
        parent: Option<CommentId>,
        body: &str,
    ) -> Result<Comment, MemberActionError> {
        let post = self.posts.get(post_id).await?.ok_or(StoreError::NotFound)?;
        self.require_unsanctioned_member(author, post.demos_id)
            .await?;
        let now = self.clock.now();
        let comment = self
            .comments
            .create(post_id, author, parent, body, now)
            .await?;
        // Every comment starts with its author's own upvote, so each begins at a
        // net score of 1 — the same baseline for everyone. `set` is idempotent per
        // (comment, voter), so this never double-counts.
        self.comment_votes
            .set(comment.id, author, Some(true))
            .await?;
        self.recompute_popularity(author, post.demos_id).await?;
        self.run_bot_check(author, post.demos_id, now).await?;
        // Ping anyone named in the reply (their opt-in is checked inside).
        self.notify_mentions(author, body, post_id, Some(comment.id))
            .await?;
        Ok(comment)
    }

    pub async fn list_posts(&self, demos: DemosId) -> Result<Vec<Post>> {
        self.posts.list(demos).await
    }

    pub async fn comments_for(&self, post: PostId) -> Result<Vec<Comment>> {
        self.comments.list_for_post(post).await
    }

    // --- post upvotes & the home feed -------------------------------------

    /// Cast (or toggle/clear) a member's up/down vote on a post. Only members in
    /// good standing of the post's community may vote. Returns the new net score.
    pub async fn vote_post(
        &self,
        post_id: PostId,
        user: UserId,
        dir: Option<bool>,
        sig: Option<&str>,
    ) -> Result<i64, VotePostError> {
        let post = self.posts.get(post_id).await?.ok_or(StoreError::NotFound)?;
        self.require_unsanctioned_member(user, post.demos_id)
            .await?;
        // Signed by the acting user (the client signs the *resolved* direction it
        // is applying, which it can compute from the vote state it rendered), so a
        // relaying node can't forge or flip a member's post vote. Verified here on
        // the authoritative owner, for both local and forwarded votes.
        self.verify_user_action(user, &post_vote_message(post_id.0, dir), sig)
            .await?;
        self.post_votes.set(post_id, user, dir).await?;
        // The post author's popularity just changed; refresh their cached score.
        self.recompute_popularity(post.author, post.demos_id)
            .await?;
        Ok(self.post_votes.score(post_id).await?)
    }

    pub async fn post_score(&self, post: PostId) -> Result<i64> {
        self.post_votes.score(post).await
    }

    pub async fn user_post_vote(&self, post: PostId, user: UserId) -> Result<Option<bool>> {
        self.post_votes.get(post, user).await
    }

    // --- comment upvotes --------------------------------------------------

    /// Cast (or toggle/clear) a member's up/down vote on a comment. Only members
    /// in good standing of the comment's community may vote. Returns the new net
    /// score.
    pub async fn vote_comment(
        &self,
        comment_id: CommentId,
        user: UserId,
        dir: Option<bool>,
    ) -> Result<i64, MemberActionError> {
        let comment = self
            .comments
            .get(comment_id)
            .await?
            .ok_or(StoreError::NotFound)?;
        let post = self
            .posts
            .get(comment.post_id)
            .await?
            .ok_or(StoreError::NotFound)?;
        self.require_unsanctioned_member(user, post.demos_id)
            .await?;
        self.comment_votes.set(comment_id, user, dir).await?;
        // The comment author's popularity just changed; refresh their cache.
        self.recompute_popularity(comment.author, post.demos_id)
            .await?;
        Ok(self.comment_votes.score(comment_id).await?)
    }

    pub async fn comment_score(&self, comment: CommentId) -> Result<i64> {
        self.comment_votes.score(comment).await
    }

    pub async fn user_comment_vote(
        &self,
        comment: CommentId,
        user: UserId,
    ) -> Result<Option<bool>> {
        self.comment_votes.get(comment, user).await
    }

    // --- popularity metrics -----------------------------------------------

    /// Compute a member's engagement metrics in one community: net upvotes on
    /// their posts and comments here (plus the counts). Popularity — the sum —
    /// is what gates the franchise and posting policy.
    pub async fn member_metrics(&self, user: UserId, demos: DemosId) -> Result<MemberMetrics> {
        let mut m = MemberMetrics::default();
        for p in self.posts.list_by_author(demos, user).await? {
            if p.removed {
                continue;
            }
            m.posts += 1;
            // Contribution must reflect the *community's* appraisal, not the
            // author's own ballot. Otherwise a user self-qualifies for the
            // franchise (`min_contribution`) and inflates their own
            // `ByContribution` vote weight just by voting on their own content
            // (comments even auto-upvote themselves). Exclude the author's own vote.
            let own = vote_value(self.post_votes.get(p.id, user).await?);
            m.net_post_upvotes += self.post_votes.score(p.id).await? - own;
        }
        // A member's comments in this community = their comments on posts here.
        let here: HashSet<PostId> = self
            .posts
            .list(demos)
            .await?
            .into_iter()
            .map(|p| p.id)
            .collect();
        for c in self.comments.list_by_author(user).await? {
            if c.removed || !here.contains(&c.post_id) {
                continue;
            }
            m.comments += 1;
            let own = vote_value(self.comment_votes.get(c.id, user).await?);
            m.net_comment_upvotes += self.comment_votes.score(c.id).await? - own;
        }
        Ok(m)
    }

    /// Refresh the cached popularity (`Membership::contribution`) for `author` in
    /// `demos` from their current metrics. Called whenever a vote changes the net
    /// score of their content, so eligibility and vote-weighting always read a
    /// current value. A no-op if the author isn't a member.
    async fn recompute_popularity(&self, author: UserId, demos: DemosId) -> Result<()> {
        let Some(mut m) = self.memberships.get(author, demos).await? else {
            return Ok(());
        };
        m.contribution = self.member_metrics(author, demos).await?.popularity();
        self.memberships.upsert(m).await
    }

    /// The site-wide "top" feed: the most popular non-removed posts across
    /// **every** community, sorted by net score (desc) then recency (desc) and
    /// capped at [`TOP_FEED_LIMIT`]. Unlike [`feed`](Self::feed) it needs no
    /// membership and applies no per-community threshold — it's a global
    /// leaderboard, available to everyone.
    pub async fn top_posts(&self) -> Result<Vec<FeedItem>> {
        let slugs: std::collections::HashMap<DemosId, String> = self
            .demoi
            .list()
            .await?
            .into_iter()
            .map(|d| (d.id, d.slug))
            .collect();
        let mut items: Vec<FeedItem> = Vec::new();
        for post in self.posts.list_all().await? {
            if post.removed || post.pending_review {
                continue;
            }
            let score = self.post_votes.score(post.id).await?;
            let community_slug = slugs.get(&post.demos_id).cloned().unwrap_or_default();
            items.push(FeedItem {
                post,
                score,
                community_slug,
            });
        }
        items.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then(b.post.created_at.0.cmp(&a.post.created_at.0))
        });
        items.truncate(TOP_FEED_LIMIT);
        Ok(items)
    }

    /// The personalized home feed: across every community the user has joined,
    /// the non-removed posts whose net score clears that community's
    /// [`feed_threshold`], sorted by score (desc) then recency (desc).
    pub async fn feed(&self, user: UserId) -> Result<Vec<FeedItem>> {
        let mut items: Vec<FeedItem> = Vec::new();
        for membership in self.memberships.list_for_user(user).await? {
            let demos = match self.demoi.get(membership.demos_id).await? {
                Some(d) => d,
                None => continue,
            };
            let threshold = feed_threshold(self.memberships.voter_count(demos.id).await?);
            for post in self.posts.list(demos.id).await? {
                if post.removed || post.pending_review {
                    continue;
                }
                let score = self.post_votes.score(post.id).await?;
                if score >= threshold {
                    items.push(FeedItem {
                        post,
                        score,
                        community_slug: demos.slug.clone(),
                    });
                }
            }
        }
        items.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then(b.post.created_at.0.cmp(&a.post.created_at.0))
        });
        Ok(items)
    }

    // --- recommendations --------------------------------------------------
    // The recommendation use cases live in `crate::recommend`; `Services` only
    // exposes the factory methods `recommend_feed()` and
    // `refresh_recommendations()` (below).

    // --- bot detection ("the machine accuses, the demos judges") -----------

    /// Assemble behavioural signals and, if they cross the threshold, file an
    /// automatic bot report (unless one is already open for this user).
    async fn run_bot_check(&self, author: UserId, demos: DemosId, now: Timestamp) -> Result<()> {
        let signals = self.bot_signals(author, demos, now).await?;
        if !is_likely_bot(&signals) {
            return Ok(());
        }
        // Folds into any open report already on this user; `add_flag` ignores a
        // duplicate Bot flag if the detector fires again.
        self.file_or_merge_flag(
            demos,
            None,
            ReportTarget::User(author),
            ReportReason::Bot,
            &format!("automatic: bot score {}", bot_score(&signals)),
            now,
        )
        .await?;
        Ok(())
    }

    pub async fn bot_signals(
        &self,
        author: UserId,
        demos: DemosId,
        now: Timestamp,
    ) -> Result<BotSignals> {
        let user = self.users.get(author).await?.ok_or(StoreError::NotFound)?;
        let posts = self.posts.list_by_author(demos, author).await?;
        let hour_ago = Timestamp(now.0 - 3600);

        let recent_posts = posts.iter().filter(|p| p.created_at >= hour_ago).count() as u32;
        let recent_comments = self
            .comments
            .count_by_author_since(author, hour_ago)
            .await? as u32;

        let distinct: HashSet<(String, String)> = posts
            .iter()
            .map(|p| (p.title.clone(), p.text_content()))
            .collect();
        let duplicate_actions = (posts.len() as u32).saturating_sub(distinct.len() as u32);
        let demos_spammed = self.posts.distinct_demos_by_author(author).await? as u32;

        Ok(BotSignals {
            account_age_days: user.account_age_days(now),
            actions_last_hour: recent_posts + recent_comments,
            duplicate_actions,
            demos_spammed,
        })
    }

    // --- reports & trial by jury (Layers above: the demos judges) ----------

    pub async fn file_report(
        &self,
        reporter: UserId,
        demos: DemosId,
        target: ReportTarget,
        reason: ReportReason,
        note: &str,
    ) -> Result<Report> {
        // Reporter must be a member of the demos.
        self.memberships
            .get(reporter, demos)
            .await?
            .ok_or(StoreError::NotFound)?;
        self.file_or_merge_flag(
            demos,
            Some(reporter),
            target,
            reason,
            note,
            self.clock.now(),
        )
        .await
    }

    /// File an accusation against `target`, folding it into the existing *open*
    /// report for that target if one exists — so a post flagged again for a
    /// different reason adds a charge to the original case rather than opening a
    /// parallel report. A report already on trial or resolved is left alone (its
    /// charges are fixed), so a fresh flag opens a new case.
    async fn file_or_merge_flag(
        &self,
        demos: DemosId,
        reporter: Option<UserId>,
        target: ReportTarget,
        reason: ReportReason,
        note: &str,
        now: Timestamp,
    ) -> Result<Report> {
        // `list_open` already restricts to Open reports, which is exactly the
        // set a new flag may merge into.
        let existing = self
            .reports
            .list_open(demos)
            .await?
            .into_iter()
            .find(|r| r.target == target);
        match existing {
            Some(mut report) => {
                report.add_flag(reporter, reason, note, now);
                self.reports.update(&report).await?;
                Ok(report)
            }
            None => {
                self.reports
                    .create(demos, reporter, target, reason, note, now)
                    .await
            }
        }
    }

    /// Empanel a jury for an open report and put it on trial. The panel is a
    /// deterministic random draw of *voters* (seeded by the report id, excluding
    /// the accused), sized by the demos's [`JurySizing`](domain::JurySizing)
    /// policy: a minority of the electorate that shrinks as the demos grows,
    /// smaller for comments than posts. Errors with `JuryTooSmall` when the demos
    /// has too few voters to seat a minority panel.
    pub async fn open_trial(
        &self,
        caller: UserId,
        report_id: ReportId,
    ) -> Result<Trial, OpenTrialError> {
        let mut report = self
            .reports
            .get(report_id)
            .await?
            .ok_or(StoreError::NotFound)?;
        if report.status != ReportStatus::Open {
            return Err(OpenTrialError::ReportNotOpen);
        }
        // Empanelling a jury is a governance/moderation action, not a public one:
        // gate it on the caller being an unsanctioned voter of the report's
        // community. Without this, any signed-in user could force *any* report in
        // *any* community to trial — freezing its charge set or griefing at will
        // (mirrors the same gate on `close_proposal`).
        let membership = self
            .memberships
            .get(caller, report.demos_id)
            .await?
            .ok_or(OpenTrialError::NotAVoter)?;
        if !membership.is_voter() || membership.is_sanctioned(self.clock.now()) {
            return Err(OpenTrialError::NotAVoter);
        }
        let accused = self.resolve_accused(&report).await?;
        let demos = self
            .demoi
            .get(report.demos_id)
            .await?
            .ok_or(StoreError::NotFound)?;
        let now = self.clock.now();

        // Jurors are drawn from the enfranchised electorate (one weight lookup
        // per voter, so keep the memberships around to weigh the panel). A
        // sanctioned member is disqualified from the franchise, so they are never
        // eligible to sit on a jury.
        let voters: Vec<Membership> = self
            .memberships
            .members(report.demos_id)
            .await?
            .into_iter()
            .filter(|m| m.is_franchised(self.clock.now()))
            .collect();

        // Comments are lower-stakes than posts; a user-level report (e.g. a bot)
        // is juried at post weight.
        let scale = match report.target {
            ReportTarget::Comment(_) => ContentScale::Comment,
            ReportTarget::Post(_) | ReportTarget::User(_) => ContentScale::Post,
        };
        let size = demos.jury_sizing.jury_size(voters.len() as u64, scale);
        if size == 0 {
            return Err(OpenTrialError::JuryTooSmall);
        }

        let candidate_ids: Vec<UserId> = voters.iter().map(|m| m.user_id).collect();
        let jurors = select_jury(&candidate_ids, accused, size, report.id.0);

        // Freeze the panel's total weight now, so the conviction bar can't shift
        // mid-trial. Unweighted juries weigh 1 each → jury_weight == jurors.len().
        let weigh_jury = demos.weighting_scope.applies_to_juries();
        // Freeze each juror's weight now, aligned by index with `jurors`, and sum
        // them for the conviction denominator. The ballot tally later weighs each
        // vote by these same frozen values (`Trial::juror_weight`), so the guilty
        // numerator and the `jury_weight` denominator share one basis and cannot
        // drift apart if a juror's live contribution changes mid-trial. Under
        // one-juror-one-vote every weight is 1.
        let juror_weights: Vec<u64> = jurors
            .iter()
            .map(|j| match voters.iter().find(|m| m.user_id == *j) {
                Some(m) if weigh_jury => demos.vote_weighting.weight_of(m, now),
                _ => 1,
            })
            .collect();
        let jury_weight: u64 = juror_weights.iter().sum();

        let trial = self
            .trials
            .create(
                report.demos_id,
                report.id,
                accused,
                jurors,
                jury_weight,
                juror_weights,
                now,
                now.plus_days(3),
            )
            .await?;
        report.status = ReportStatus::OnTrial(trial.id);
        self.reports.update(&report).await?;
        // Summon each empanelled juror who wants jury alerts to come and vote.
        for juror in &trial.jurors {
            if let Some(u) = self.users.get(*juror).await? {
                if u.allows_jury_alerts {
                    self.notifications
                        .push(
                            *juror,
                            NotificationKind::JurySummons {
                                trial: trial.id,
                                demos: trial.demos_id,
                            },
                            now,
                        )
                        .await?;
                }
            }
        }
        Ok(trial)
    }

    /// Every comment on a trial, oldest first — the public gallery discussion.
    pub async fn trial_comments(&self, trial: TrialId) -> Result<Vec<TrialComment>> {
        self.trial_comments.list_for_trial(trial).await
    }

    /// A voter comments on a trial. Any enfranchised voter of the trial's demos may
    /// speak — juror or not — so the case is argued in the open. The comment does
    /// not touch the verdict; it is context the electorate (and any watching juror)
    /// can weigh. Every party already in the case — the accused, the reporters, the
    /// jurors, and anyone who has already commented — is pinged (unless they have
    /// opted out of trial-comment alerts), so a running argument reaches the people
    /// it concerns without anyone having to poll the page.
    pub async fn comment_on_trial(
        &self,
        trial_id: TrialId,
        author: UserId,
        body: &str,
    ) -> Result<TrialComment, CommentOnTrialError> {
        let body = body.trim();
        if body.is_empty() {
            return Err(CommentOnTrialError::Empty);
        }
        let trial = self.trials.get(trial_id).await?.ok_or(StoreError::NotFound)?;
        // Commenting is a franchise right: only an enfranchised (unsanctioned) voter
        // of this demos may speak in its gallery. The accused, if not a voter, can
        // still read the public record but not argue here.
        let membership = self
            .memberships
            .get(author, trial.demos_id)
            .await?
            .ok_or(CommentOnTrialError::NotAVoter)?;
        if !membership.is_franchised(self.clock.now()) {
            return Err(CommentOnTrialError::NotAVoter);
        }
        let now = self.clock.now();
        let comment = self
            .trial_comments
            .add(trial_id, author, body.to_string(), now)
            .await?;
        self.notify_trial_comment(&trial, author, now).await?;
        Ok(comment)
    }

    /// Ping everyone party to `trial` — the accused, its reporters, its jurors, and
    /// anyone who has already commented — that a new comment landed, skipping the
    /// commenter and anyone who has opted out of trial-comment alerts. Best-effort,
    /// mirroring [`Self::notify_mentions`]: called after the comment is stored.
    async fn notify_trial_comment(
        &self,
        trial: &Trial,
        author: UserId,
        now: Timestamp,
    ) -> Result<()> {
        // Build the audience: accused + jurors + reporters + prior commenters.
        let mut audience: Vec<UserId> = Vec::new();
        audience.push(trial.accused);
        audience.extend(trial.jurors.iter().copied());
        if let Some(report) = self.reports.get(trial.report_id).await? {
            audience.extend(report.flags.iter().filter_map(|f| f.reporter));
        }
        for c in self.trial_comments.list_for_trial(trial.id).await? {
            audience.push(c.author);
        }
        // Dedup and drop the commenter — never notify yourself of your own comment.
        audience.sort_by_key(|u| u.0);
        audience.dedup();
        for recipient in audience {
            if recipient == author {
                continue;
            }
            if let Some(u) = self.users.get(recipient).await? {
                if u.allows_trial_comment_alerts {
                    self.notifications
                        .push(
                            recipient,
                            NotificationKind::TrialComment {
                                trial: trial.id,
                                demos: trial.demos_id,
                                by: author,
                            },
                            now,
                        )
                        .await?;
                }
            }
        }
        Ok(())
    }

    /// A juror votes. Returns the (possibly now-decided) verdict; a decisive
    /// verdict applies its consequences immediately.
    pub async fn cast_jury_vote(
        &self,
        trial_id: TrialId,
        juror: UserId,
        guilty: bool,
        sig: Option<&str>,
    ) -> Result<Verdict, CastJuryVoteError> {
        let trial = self.trials.get(trial_id).await?.ok_or(StoreError::NotFound)?;
        if trial.verdict != Verdict::Pending {
            return Err(CastJuryVoteError::TrialClosed);
        }
        if !trial.is_juror(juror) {
            return Err(CastJuryVoteError::NotAJuror);
        }
        // A juror sanctioned after empanelment (e.g. convicted in a parallel
        // trial) is disqualified from the franchise and may no longer vote a
        // verdict. A juror who has since left the community (no membership) keeps
        // the seat they were drawn into.
        if let Some(m) = self.memberships.get(juror, trial.demos_id).await? {
            if m.is_sanctioned(self.clock.now()) {
                return Err(CastJuryVoteError::Sanctioned);
            }
        }
        // The verdict ballot must be signed by the juror, verified on the owner —
        // so a node hosting a juror can't cast a verdict in their name.
        self.verify_user_action(juror, &jury_vote_message(trial_id.0, guilty), sig)
            .await?;
        // Weigh the ballot by this juror's weight *frozen at empanelment*, not a
        // live recomputation: the panel's total (`jury_weight`) was frozen from the
        // same per-juror values, so the guilty/nay sums and the conviction
        // denominator stay in one basis. Recomputing live here would let a juror
        // shift the 2/3 bar mid-trial by pumping their contribution.
        let weight = trial.juror_weight(juror);
        self.trials
            .cast_ballot(trial_id, juror, guilty, weight)
            .await?;
        Ok(self.settle_trial(trial_id).await?)
    }

    /// Recompute a trial's verdict and, if decisive, apply consequences:
    /// a guilty verdict sanctions the accused (which disqualifies them from the
    /// franchise) and removes the reported content.
    pub async fn settle_trial(&self, trial_id: TrialId) -> Result<Verdict, SettleTrialError> {
        let mut trial = self.trials.get(trial_id).await?.ok_or(StoreError::NotFound)?;
        if trial.verdict != Verdict::Pending {
            return Ok(trial.verdict);
        }

        let (guilty, not_guilty) = self.trials.ballot_tally(trial_id).await?;
        let verdict = reach_verdict(guilty, not_guilty, trial.jury_weight);
        if verdict == Verdict::Pending {
            return Ok(Verdict::Pending);
        }

        trial.verdict = verdict;
        self.trials.update(&trial).await?;

        let mut report = self
            .reports
            .get(trial.report_id)
            .await?
            .ok_or(StoreError::NotFound)?;
        match verdict {
            Verdict::Guilty => {
                report.status = ReportStatus::Upheld;
                if let Some(mut m) = self.memberships.get(trial.accused, trial.demos_id).await? {
                    // The ban's length is tied to the rule(s) the case cited — the
                    // term the voters fixed for that rule ahead of the trial — not a
                    // flat maximum. `sanction_for` still caps at MAX_SANCTION_DAYS.
                    let term = self.ban_term_for_report(&report).await?;
                    m.sanction_for(self.clock.now(), term);
                    self.memberships.upsert(m).await?;
                }
                match report.target {
                    ReportTarget::Post(p) => self.posts.set_removed(p, true).await?,
                    ReportTarget::Comment(c) => self.comments.set_removed(c, true).await?,
                    ReportTarget::User(_) => {}
                }
            }
            Verdict::NotGuilty => report.status = ReportStatus::Dismissed,
            Verdict::Pending => {}
        }
        self.reports.update(&report).await?;
        Ok(verdict)
    }

    /// Total vote weight of the electorate under `scheme` — the quorum
    /// denominator for a weighted proposal, summed over the same population
    /// (`tier == Voter`) that [`MembershipStore::voter_count`](crate::MembershipStore::voter_count)
    /// counts.
    async fn total_voter_weight(
        &self,
        demos: DemosId,
        scheme: VoteWeighting,
        now: Timestamp,
    ) -> Result<u64> {
        Ok(self
            .memberships
            .members(demos)
            .await?
            .iter()
            .filter(|m| m.is_voter())
            .map(|m| scheme.weight_of(m, now))
            .sum())
    }

    /// The ban term (days) a conviction on the report behind `trial` would carry,
    /// for showing jurors the stakes before they vote. `None` if the report is
    /// gone. See [`Self::ban_term_for_report`] for how it's derived.
    pub async fn proposed_ban_term(&self, report_id: ReportId) -> Result<Option<u32>> {
        match self.reports.get(report_id).await? {
            Some(report) => Ok(Some(self.ban_term_for_report(&report).await?)),
            None => Ok(None),
        }
    }

    /// The ban term (days) a conviction on `report` carries: the most severe of
    /// the rule terms the case cited, each read against the community's live
    /// ceiling. A case that cites no specific rule (a bot/NSFW flag, or a bare
    /// rule-break) falls back to the community ceiling. Never exceeds it — and
    /// `Membership::sanction_for` caps the result at the 18-year platform maximum.
    async fn ban_term_for_report(&self, report: &Report) -> Result<u32> {
        let ceiling = match self.demoi.get(report.demos_id).await? {
            Some(d) => d.ban_ceiling_days(),
            None => MAX_SANCTION_DAYS,
        };
        // The distinct rules named across the case's flags.
        let cited: Vec<RuleId> = report
            .flags
            .iter()
            .filter_map(|f| match f.reason {
                ReportReason::RuleBreak { rule: Some(id) } => Some(id),
                _ => None,
            })
            .collect();
        let mut term = 0u32;
        for id in cited {
            if let Some(rule) = self.rules.get(id).await? {
                term = term.max(rule.term_days(ceiling));
            }
        }
        // No cited rule carried a resolvable term → the community ceiling governs.
        Ok(if term == 0 { ceiling } else { term })
    }

    async fn resolve_accused(&self, report: &Report) -> Result<UserId> {
        match report.target {
            ReportTarget::User(u) => Ok(u),
            ReportTarget::Post(p) => Ok(self.posts.get(p).await?.ok_or(StoreError::NotFound)?.author),
            ReportTarget::Comment(c) => Ok(self
                .comments
                .get(c)
                .await?
                .ok_or(StoreError::NotFound)?
                .author),
        }
    }

    async fn require_unsanctioned_member(
        &self,
        user: UserId,
        demos: DemosId,
    ) -> Result<Membership, MemberActionError> {
        let m = self
            .memberships
            .get(user, demos)
            .await?
            .ok_or(StoreError::NotFound)?;
        if m.is_sanctioned(self.clock.now()) {
            return Err(MemberActionError::Sanctioned);
        }
        Ok(m)
    }

    /// Whether `user` may create a post in `demos` under its posting policy.
    /// Backs both the enforcement in [`create_post`](Self::create_post) and the
    /// composer's community picker (so it only offers postable communities).
    pub async fn can_post(&self, user: UserId, demos: DemosId) -> Result<bool, CanPostError> {
        let d = self.demoi.get(demos).await?.ok_or(StoreError::NotFound)?;
        let m = self.memberships.get(user, demos).await?;
        Ok(posting_allowed(d.posting_policy, m.as_ref(), self.clock.now()))
    }

    /// Like [`can_post`](Self::can_post) but returns a policy-specific error the
    /// UI can show, rather than a bool.
    async fn require_can_post(&self, user: UserId, demos: DemosId) -> Result<(), CanPostError> {
        let d = self.demoi.get(demos).await?.ok_or(StoreError::NotFound)?;
        let m = self.memberships.get(user, demos).await?;
        if posting_allowed(d.posting_policy, m.as_ref(), self.clock.now()) {
            return Ok(());
        }
        // A sanction is its own distinct error (blocks posting under any policy).
        if m.as_ref().is_some_and(|m| m.is_sanctioned(self.clock.now())) {
            return Err(CanPostError::Sanctioned);
        }
        let msg = match d.posting_policy {
            PostingPolicy::Members => "join this community to post here",
            PostingPolicy::Voters => "only voters may post in this community",
            PostingPolicy::MinContribution(_) => {
                "you haven't earned enough popularity to post in this community yet"
            }
            PostingPolicy::Open => "you cannot post in this community",
        };
        Err(CanPostError::Rejected(msg.to_string()))
    }

    // --- helpers -----------------------------------------------------------

    async fn load_triplet(
        &self,
        user: UserId,
        demos: DemosId,
    ) -> Result<(User, Membership, Demos)> {
        let u = self.users.get(user).await?.ok_or(StoreError::NotFound)?;
        let m = self
            .memberships
            .get(user, demos)
            .await?
            .ok_or(StoreError::NotFound)?;
        let d = self.demoi.get(demos).await?.ok_or(StoreError::NotFound)?;
        Ok((u, m, d))
    }
}

/// Thin factory methods: [`Services`] holds the shared port handles and wires up
/// each recommendation use case on demand. Callers depend on the use case, not
/// the container. (The use-case logic lives in [`crate::recommend`].)
impl Services {
    /// The recommendation read use case.
    pub fn recommend_feed(&self) -> RecommendFeed {
        RecommendFeed::new(
            self.post_votes.clone(),
            self.recommender.clone(),
            self.posts.clone(),
            self.demoi.clone(),
        )
    }

    /// The recommendation model-refresh use case.
    pub fn refresh_recommendations(&self) -> RefreshRecommendations {
        RefreshRecommendations::new(self.post_votes.clone(), self.recommender.clone())
    }
}

/// Does a single lowercase query token match this post? True if it's a
/// substring of the title or body, or equals one of its tags.
/// Pure decision: does this membership (if any) satisfy a community's posting
/// policy? A sanctioned member is always blocked; a non-member (`None`) passes
/// only under [`PostingPolicy::Open`].
fn posting_allowed(policy: PostingPolicy, membership: Option<&Membership>, now: Timestamp) -> bool {
    if membership.is_some_and(|m| m.is_sanctioned(now)) {
        return false;
    }
    match policy {
        PostingPolicy::Open => true,
        PostingPolicy::Members => membership.is_some(),
        PostingPolicy::Voters => membership.is_some_and(|m| m.is_voter()),
        PostingPolicy::MinContribution(n) => membership.is_some_and(|m| m.contribution >= n),
    }
}

fn post_matches(post: &Post, token: &str) -> bool {
    post.title.to_lowercase().contains(token)
        || post.text_content().to_lowercase().contains(token)
        || post.tags.iter().any(|t| t == token)
}
