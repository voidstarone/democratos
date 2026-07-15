

use domain::{
    FeedPaging, User, UserId,
};


use crate::auth::hash_password::hash_password;
use crate::auth::spend_verify_time::spend_verify_time;
use crate::auth::verify_password::verify_password;
use crate::identity::is_valid_public_key::is_valid_public_key;
use crate::identity::user_public_key::UserPublicKey;
use crate::{
    AuthenticateError, EnrollPublicKeyError,
    EnsureBarredAccountError, FoundDemosError,
    RegisterAccountError, Result, SetFeedPagingError, StoreError, VerifyActionError,
};


use super::services::Services;

impl Services {
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

    /// Look up an account by handle — backs the public profile page at `/u/:handle`.
    pub async fn user_by_handle(&self, handle: &str) -> Result<Option<User>> {
        self.users.by_handle(handle.trim()).await
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
    pub(super) async fn verify_user_action(
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
    pub(super) async fn ensure_not_barred(&self, user: UserId) -> Result<(), FoundDemosError> {
        let u = self.users.get(user).await?.ok_or(StoreError::NotFound)?;
        if u.is_franchise_barred {
            return Err(FoundDemosError::Rejected(
                "this account is barred from the franchise".into(),
            ));
        }
        Ok(())
    }
}
