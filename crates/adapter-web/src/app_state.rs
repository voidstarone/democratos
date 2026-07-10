//! Shared handler state.

use std::sync::Arc;

use app::{GovernanceWrites, Services, SessionSigner};

/// Shared handler state: the application services (reads + non-federated
/// writes), the governance-write gateway (votes, which may forward to a
/// community's owner node), and whether developer tooling is enabled.
#[derive(Clone)]
pub struct AppState {
    pub services: Services,
    /// Where governance ballots go. Single-box this runs locally; federated it
    /// routes to the owning node. See [`app::GovernanceWrites`].
    pub writes: Arc<dyn GovernanceWrites>,
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
}
