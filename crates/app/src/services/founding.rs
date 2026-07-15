//! Facade delegators for founding use-cases. The logic now lives in
//! [`FoundingService`](super::founding_service::FoundingService); these thin
//! methods keep `services.found_demos()` and friends working for call sites not
//! yet migrated off the `Services` aggregator.

use domain::{Demos, FoundingId, FoundingPetition, UserId};

use crate::{FoundDemosError, Result, SignFoundingError, StartFoundingError};

use super::founding_service::FoundingService;
use super::services::Services;

impl Services {
    /// Build the extracted [`FoundingService`] from the ports this aggregator
    /// still holds, wiring its [`AccountService`](super::account_service::AccountService)
    /// peer inline. Cheap — `Arc` clones only — so delegators construct one per
    /// call rather than storing a field (which would break every `Services { … }`
    /// literal). Removed once all call sites inject `FoundingService` directly.
    pub(super) fn founding_service(&self) -> FoundingService {
        FoundingService::new(
            self.foundings.clone(),
            self.demoi.clone(),
            self.memberships.clone(),
            self.clock.clone(),
            std::sync::Arc::new(self.account_service()),
        )
    }

    pub async fn found_demos(
        &self,
        founder: UserId,
        slug: &str,
        name: &str,
    ) -> Result<Demos, FoundDemosError> {
        self.founding_service()
            .found_demos(founder, slug, name)
            .await
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
        self.founding_service().start_founding(founder, name).await
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
        self.founding_service()
            .start_founding_tagged(founder, name, tags)
            .await
    }

    pub async fn founding(&self, id: FoundingId) -> Result<Option<FoundingPetition>> {
        self.founding_service().founding(id).await
    }

    /// Every founding still gathering sign-offs, newest first.
    pub async fn pending_foundings(&self) -> Result<Vec<FoundingPetition>> {
        self.founding_service().pending_foundings().await
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
        self.founding_service().sign_founding(id, user).await
    }
}
