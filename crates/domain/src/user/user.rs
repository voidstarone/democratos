//! The platform-wide user account entity.

use serde::{Deserialize, Serialize};

use crate::{FeedPaging, Timestamp, UserId};

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub handle: String,
    /// Login email. `None` for accounts created without credentials (the
    /// dev-only handle switcher); real sign-ups always carry one. Unique across
    /// accounts when present. `#[serde(default)]` keeps pre-credential datasets
    /// loadable.
    #[serde(default)]
    pub email: Option<String>,
    /// Argon2 password hash for [`email`](Self::email) login. `None` for
    /// credential-less dev accounts, which therefore can never be reached by the
    /// real password path.
    ///
    /// SECURITY: never serialise a `User` wholesale into an HTTP response — this
    /// field would ride along. Responses only ever read [`handle`](Self::handle)
    /// / [`id`](Self::id); the field persists only to storage.
    #[serde(default)]
    pub password_hash: Option<String>,
    pub created_at: Timestamp,
    /// Whether the account has cleared age verification. Only consulted where a
    /// deployment turns the age-verification toggle on (e.g. the UK); elsewhere
    /// it is ignored. `#[serde(default)]` keeps older datasets loadable.
    #[serde(default)]
    pub is_age_verified: bool,
    /// The account's Ed25519 **public** signing key (hex), if the holder has
    /// enrolled one. Governance actions from this account are then required to
    /// carry a signature verifiable against this key — so no node (not even the
    /// one hosting the account) can forge the member's vote in an open federation.
    /// The matching secret key never leaves the user's device; the server only
    /// ever holds this public half. `None` = a legacy/not-yet-enrolled account,
    /// whose actions fall back to node-trusted handling during rollout.
    /// `#[serde(default)]` keeps pre-key datasets loadable.
    #[serde(default)]
    pub public_key: Option<String>,
    /// How this member wants long feeds delivered (paged vs. lazy-loaded).
    /// `#[serde(default)]` keeps pre-preference datasets loadable — they read
    /// back as [`FeedPaging::Auto`].
    #[serde(default)]
    pub feed_paging: FeedPaging,
    /// Whether this account is permanently barred from the franchise. Set on the
    /// dev/content "puppet" accounts the dev switcher toggles between: they exist
    /// only to post seed content and must NEVER become voters. The domain enforces
    /// it — [`evaluate_eligibility`](crate::evaluate_eligibility) treats a barred
    /// account as never eligible and the founding paths refuse it — so the bar
    /// holds no matter which enfranchisement path is tried. Defaults `false`;
    /// `#[serde(default)]` keeps pre-flag datasets loadable.
    #[serde(default)]
    pub is_franchise_barred: bool,
    /// Whether this account has opted in to reviewing platform-wide sensitive
    /// content (and therefore to seeing flagged content, behind a click-through).
    /// A plain platform account attribute — deliberately NOT a demos tier and not
    /// tied to the franchise. Default false; `#[serde(default)]` keeps older
    /// datasets loadable. See [`crate::sensitive`].
    #[serde(default)]
    pub is_sensitive_reviewer: bool,
    /// Accounts this user has personally blocked. Purely one-directional and
    /// unbounded — a member may block as many others as they like, and the block
    /// hides the blocked account's content from *this* viewer only (it does not
    /// touch the blocked user's own view). Distinct from a demos sanction, which is
    /// community moderation; a block is a personal mute with no governance effect.
    /// `#[serde(default)]` keeps pre-block datasets loadable (as an empty set).
    #[serde(default)]
    pub blocked: Vec<UserId>,
    /// Whether the account wants a notification when it is named (`@handle`) in a
    /// post or comment. Opt-out: defaults on. `#[serde(default = "..")]` keeps
    /// pre-notification datasets loadable as opted-in.
    #[serde(default = "alerts_default")]
    pub allows_mention_alerts: bool,
    /// Whether the account wants a notification when it is summoned to a jury.
    /// Opt-out: defaults on.
    #[serde(default = "alerts_default")]
    pub allows_jury_alerts: bool,
    /// Whether the account wants a notification when a new comment is posted on a
    /// trial it is party to or has spoken in. Opt-out: defaults on.
    #[serde(default = "alerts_default")]
    pub allows_trial_comment_alerts: bool,
}

fn alerts_default() -> bool {
    true
}

impl User {
    /// A credential-less account (handle only) — the dev switcher's shape. Real
    /// sign-ups go through [`with_credentials`](Self::with_credentials).
    pub fn new(id: UserId, handle: impl Into<String>, created_at: Timestamp) -> Self {
        Self {
            id,
            handle: handle.into(),
            email: None,
            password_hash: None,
            created_at,
            is_age_verified: false,
            public_key: None,
            feed_paging: FeedPaging::Auto,
            is_franchise_barred: false,
            is_sensitive_reviewer: false,
            blocked: Vec::new(),
            allows_mention_alerts: true,
            allows_jury_alerts: true,
            allows_trial_comment_alerts: true,
        }
    }

    /// An account that can sign in with email + password.
    pub fn with_credentials(
        id: UserId,
        handle: impl Into<String>,
        email: impl Into<String>,
        password_hash: impl Into<String>,
        created_at: Timestamp,
    ) -> Self {
        Self {
            id,
            handle: handle.into(),
            email: Some(email.into()),
            password_hash: Some(password_hash.into()),
            created_at,
            is_age_verified: false,
            public_key: None,
            feed_paging: FeedPaging::Auto,
            is_franchise_barred: false,
            is_sensitive_reviewer: false,
            blocked: Vec::new(),
            allows_mention_alerts: true,
            allows_jury_alerts: true,
            allows_trial_comment_alerts: true,
        }
    }

    /// Mark this account as permanently franchise-barred — a content-only "puppet"
    /// the dev switcher can act as but which can never become a voter. Builder-style
    /// so a barred account reads as `User::new(..).barred()`.
    pub fn barred(mut self) -> Self {
        self.is_franchise_barred = true;
        self
    }

    /// Whole days since the account was created.
    pub fn account_age_days(&self, now: Timestamp) -> i64 {
        now.days_since(self.created_at)
    }

    /// Whether this account has personally blocked `other`. A no-op for one's own
    /// id — you never block yourself (see [`block`](Self::block)).
    pub fn blocks(&self, other: UserId) -> bool {
        self.blocked.contains(&other)
    }

    /// Block `other` (idempotent, and unbounded — there is no cap on how many
    /// accounts one may block). Blocking yourself is silently ignored.
    pub fn block(&mut self, other: UserId) {
        if other != self.id && !self.blocked.contains(&other) {
            self.blocked.push(other);
        }
    }

    /// Lift a block on `other` (idempotent).
    pub fn unblock(&mut self, other: UserId) {
        self.blocked.retain(|&b| b != other);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(id: u64) -> User {
        User::new(UserId(id), format!("u{id}"), Timestamp(0))
    }

    #[test]
    fn blocking_is_one_directional_and_idempotent() {
        let mut alice = account(1);
        alice.block(UserId(2));
        alice.block(UserId(2)); // repeat is a no-op
        assert!(alice.blocks(UserId(2)));
        assert_eq!(alice.blocked, vec![UserId(2)]);
        // The block is on Alice's record only — it says nothing about who blocks her.
        assert!(!account(2).blocks(UserId(1)));
    }

    #[test]
    fn you_cannot_block_yourself() {
        let mut alice = account(1);
        alice.block(UserId(1));
        assert!(!alice.blocks(UserId(1)));
        assert!(alice.blocked.is_empty());
    }

    #[test]
    fn unblocking_lifts_the_block() {
        let mut alice = account(1);
        alice.block(UserId(2));
        alice.block(UserId(3));
        alice.unblock(UserId(2));
        assert!(!alice.blocks(UserId(2)));
        assert!(alice.blocks(UserId(3)));
        alice.unblock(UserId(2)); // idempotent
        alice.unblock(UserId(99)); // never-blocked is a no-op
    }

    #[test]
    fn blocking_is_unbounded() {
        let mut alice = account(1);
        for id in 2..1_000 {
            alice.block(UserId(id));
        }
        assert_eq!(alice.blocked.len(), 998);
    }
}
