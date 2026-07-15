//! Facade delegators for account use-cases. The logic now lives in
//! [`AccountService`](super::account_service::AccountService); these thin methods
//! keep `services.register_account()` and friends working for call sites not yet
//! migrated off the `Services` aggregator.

use domain::{FeedPaging, User, UserId};

use crate::{
    AuthenticateError, EnrollPublicKeyError, EnsureBarredAccountError, RegisterAccountError,
    Result, SetFeedPagingError,
};

use super::account_service::AccountService;
use super::services::Services;

impl Services {
    /// Build the extracted [`AccountService`] from the ports this aggregator still
    /// holds. Cheap — `Arc` clones only — so delegators construct one per call
    /// rather than storing a field (which would break every `Services { … }`
    /// literal). Removed once all call sites inject `AccountService` directly.
    pub(super) fn account_service(&self) -> AccountService {
        AccountService::new(
            self.users.clone(),
            self.age_verifier.clone(),
            self.require_signatures,
            self.clock.clone(),
        )
    }

    /// Register a credential-less account from a handle alone. This is the
    /// dev-only path (the account switcher) — the resulting user has no email or
    /// password and so can never be reached through [`authenticate`]. Real
    /// sign-ups go through [`register_account`](Self::register_account).
    pub async fn register_user(&self, handle: &str) -> Result<User> {
        self.account_service().register_user(handle).await
    }

    /// Register a fresh credential-less account that is permanently franchise-barred
    /// — a dev/content "puppet". Used by the dev switcher's create/login-by-handle
    /// paths so anything it mints is a non-voter by construction. Errors if taken.
    pub async fn register_barred_user(&self, handle: &str) -> Result<User> {
        self.account_service().register_barred_user(handle).await
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
        self.account_service().ensure_barred_account(handle).await
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
        self.account_service()
            .register_account(handle, email, password)
            .await
    }

    /// Look up an account by handle — backs the public profile page at `/u/:handle`.
    pub async fn user_by_handle(&self, handle: &str) -> Result<Option<User>> {
        self.account_service().user_by_handle(handle).await
    }

    /// Verify an email + password login. Returns the account on success and the
    /// opaque [`AuthenticateError::InvalidCredentials`] on any failure — unknown
    /// email, a credential-less account, or a wrong password all look identical.
    pub async fn authenticate(
        &self,
        email: &str,
        password: &str,
    ) -> Result<User, AuthenticateError> {
        self.account_service().authenticate(email, password).await
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
        self.account_service()
            .authenticate_by_handle(handle, password)
            .await
    }

    /// Every registered account. Backs dev tooling that enumerates and switches
    /// between test users.
    pub async fn list_users(&self) -> Result<Vec<User>> {
        self.account_service().list_users().await
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
        self.account_service()
            .enroll_public_key(user, public_key_hex)
            .await
    }

    /// Record how a member wants long feeds delivered (paged vs. lazy-loaded). A
    /// plain account preference with no policy attached — the store just persists it.
    pub async fn set_feed_paging(
        &self,
        user: UserId,
        paging: FeedPaging,
    ) -> Result<(), SetFeedPagingError> {
        self.account_service().set_feed_paging(user, paging).await
    }

    /// Run age verification for `user` through the provider; on success persist
    /// the result. Returns whether the user is now verified.
    pub async fn verify_age(&self, user: UserId) -> Result<bool> {
        self.account_service().verify_age(user).await
    }
}
