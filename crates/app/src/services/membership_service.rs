//! Membership use-cases: joining a community, recording contribution, the phase
//! read, eligibility evaluation, and the two-layer enfranchisement request. Also
//! owns the shared member-context read [`load_triplet`](MembershipService::load_triplet)
//! (user + membership + demos), which callers across the moderation and
//! governance paths lean on. Owns only the membership-facing ports it needs.

use std::sync::Arc;

use domain::{
    enfranchisement_slots, evaluate_eligibility, Demos, DemosId, Eligibility, Membership, Phase,
    Tier, Timestamp, User, UserId,
};

use crate::{Clock, DemosStore, MembershipStore, Result, StoreError, UserStore};

use super::enfranchise_outcome::EnfranchiseOutcome;

/// Trailing window for the enfranchisement rate cap.
const RATE_CAP_WINDOW_DAYS: i64 = 30;

/// Membership use-cases, over just the membership-facing ports.
#[derive(Clone)]
pub struct MembershipService {
    users: Arc<dyn UserStore>,
    demoi: Arc<dyn DemosStore>,
    memberships: Arc<dyn MembershipStore>,
    clock: Arc<dyn Clock>,
}

impl MembershipService {
    pub fn new(
        users: Arc<dyn UserStore>,
        demoi: Arc<dyn DemosStore>,
        memberships: Arc<dyn MembershipStore>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            users,
            demoi,
            memberships,
            clock,
        }
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

    /// Load the (user, membership, demos) triplet a member-context decision reads,
    /// erroring `NotFound` if any is missing. The shared read behind eligibility
    /// and enfranchisement — member-context, so it lives here.
    pub async fn load_triplet(
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
