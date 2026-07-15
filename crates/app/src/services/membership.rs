//! Facade delegators for membership use-cases. The logic now lives in
//! [`MembershipService`](super::membership_service::MembershipService); these thin
//! methods keep `services.join()` and friends working for call sites not yet
//! migrated off the `Services` aggregator.

use domain::{DemosId, Eligibility, Membership, Phase, UserId};

use crate::Result;

use super::enfranchise_outcome::EnfranchiseOutcome;
use super::membership_service::MembershipService;
use super::services::Services;

impl Services {
    /// Build the extracted [`MembershipService`] from the ports this aggregator
    /// still holds. Cheap — `Arc` clones only — so delegators construct one per
    /// call rather than storing a field (which would break every `Services { … }`
    /// literal). Removed once all call sites inject `MembershipService` directly.
    pub(super) fn membership_service(&self) -> MembershipService {
        MembershipService::new(
            self.users.clone(),
            self.demoi.clone(),
            self.memberships.clone(),
            self.clock.clone(),
        )
    }

    pub async fn join(&self, user: UserId, demos: DemosId) -> Result<Membership> {
        self.membership_service().join(user, demos).await
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
        self.membership_service()
            .record_contribution(user, demos, delta)
            .await
    }

    pub async fn phase_of(&self, demos: DemosId) -> Result<Phase> {
        self.membership_service().phase_of(demos).await
    }

    pub async fn check_eligibility(&self, user: UserId, demos: DemosId) -> Result<Eligibility> {
        self.membership_service()
            .check_eligibility(user, demos)
            .await
    }

    /// Apply Layer 1 (eligibility) then Layer 2 (rate cap) to admit a member to
    /// the franchise, or explain why not.
    pub async fn request_enfranchisement(
        &self,
        user: UserId,
        demos: DemosId,
    ) -> Result<EnfranchiseOutcome> {
        self.membership_service()
            .request_enfranchisement(user, demos)
            .await
    }
}
