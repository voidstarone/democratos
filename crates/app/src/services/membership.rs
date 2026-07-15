

use domain::{
    enfranchisement_slots, evaluate_eligibility,
    DemosId, Eligibility, Membership, Phase, Tier, Timestamp, UserId,
};


use crate::{
    Result, StoreError,
};

use super::enfranchise_outcome::EnfranchiseOutcome;

use super::services::Services;

/// Trailing window for the enfranchisement rate cap.
const RATE_CAP_WINDOW_DAYS: i64 = 30;

impl Services {
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
}
