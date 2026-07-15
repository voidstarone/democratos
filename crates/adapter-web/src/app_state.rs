//! Shared handler state.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use app::{
    AccountAuthenticator, AccountMinter, AccountService, BlockingService, ContentService,
    FeedService, FoundingService, GovernanceService, GovernanceWrites, InviteService, MembershipService,
    MetricsService, ModerationService, NotificationService, ProfileService, SearchService,
    SensitiveReviewService, Services, SessionSigner,
};
use ipnet::IpNet;

/// Shared handler state: the application services (reads + non-federated
/// writes), the governance-write gateway (votes, which may forward to a
/// community's owner node), and whether developer tooling is enabled.
#[derive(Clone)]
pub struct AppState {
    pub services: Services,
    /// Per-area use-case services, injected individually. These replace the
    /// former `services` god object for use-case calls; `services` itself remains
    /// only for the raw-store reaches a later phase will migrate. Field names
    /// match [`app::ServiceSet`].
    pub accounts: Arc<AccountService>,
    pub blocking: Arc<BlockingService>,
    pub profile: Arc<ProfileService>,
    pub search: Arc<SearchService>,
    pub notifications: Arc<NotificationService>,
    pub metrics: Arc<MetricsService>,
    pub invites: Arc<InviteService>,
    pub sensitive: Arc<SensitiveReviewService>,
    pub membership: Arc<MembershipService>,
    pub founding: Arc<FoundingService>,
    pub moderation: Arc<ModerationService>,
    pub governance: Arc<GovernanceService>,
    pub content: Arc<ContentService>,
    pub feed: Arc<FeedService>,
    /// Where governance ballots go. Single-box this runs locally; federated it
    /// routes to the owning node. See [`app::GovernanceWrites`].
    pub writes: Arc<dyn GovernanceWrites>,
    /// Where account sign-ups go. Single-box (or on a trusted issuer) this mints
    /// locally; on a non-issuer federated node it forwards to a trusted issuer that
    /// mints the account in its own namespace. See [`app::AccountMinter`].
    pub minter: Arc<dyn AccountMinter>,
    /// Where sign-ins are verified. Locally when this node holds the account's
    /// credentials; otherwise forwarded to the account's home issuer. Login is by
    /// handle (emails don't replicate). See [`app::AccountAuthenticator`].
    pub authenticator: Arc<dyn AccountAuthenticator>,
    /// Signs and verifies the session cookie, so the acting-user id a browser
    /// presents is one the server actually authenticated — not a bare, forgeable
    /// integer. See [`app::SessionSigner`].
    pub session: SessionSigner,
    pub dev_mode: bool,
    /// When set, session/preference cookies are marked `Secure` so a browser only
    /// ever sends them over HTTPS. Enable behind TLS (the normal production case);
    /// leave off for plain-HTTP local development, where `Secure` would stop the
    /// cookie being sent at all.
    pub secure_cookies: bool,
    /// Optional shared secret that `GET /dev/unlock` requires as `?key=` before it
    /// hands out the dev-switcher unlock cookie. `None` keeps the legacy behaviour
    /// (unlock gated on `--dev` alone — correct for a loopback-only local run).
    /// Set it when a dev-enabled node is reachable beyond your own machine, so only
    /// a caller holding the secret can unlock the account switcher. See [`crate::dev`].
    pub dev_unlock_secret: Option<Arc<str>>,
    /// Whether new sign-ups currently require an invite. Held as an atomic so the
    /// operator can flip it live from the admin console; seeded at boot from the
    /// persisted setting (falling back to the `--invite-only` flag) and written
    /// back to storage on every toggle. Read on the hot path (`/register`) without
    /// an await.
    pub invite_only: Arc<AtomicBool>,
    /// The CIDR allowlist for the admin review queue: a request whose connection
    /// peer is outside every listed network (and not loopback) gets a `404`.
    /// Empty means "loopback only".
    pub admin_subnets: Arc<[IpNet]>,
    /// Shared secret the admin review queue requires as `?key=` on top of the
    /// subnet check. `None` disables the queue entirely (it always `404`s) — so an
    /// operator must set a secret to use it, never reachable by subnet membership
    /// alone.
    pub admin_secret: Option<Arc<str>>,
}
