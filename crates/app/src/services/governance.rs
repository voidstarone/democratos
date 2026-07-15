

use domain::{
    DemosId, Phase, MAX_SANCTION_DAYS, Proposal, ProposalId, ProposalKind, ProposalStatus, Rule, UserId,
    VoteWeighting,
};


use crate::identity::vote_message::vote_message;
use crate::{
    CastVoteError, CloseProposalError, OpenProposalError, Result, StoreError,
};


use super::services::Services;
use super::voting_window_days::voting_window_days;

impl Services {
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
            && self.membership_service().phase_of(demos).await? == Phase::Seed
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
        self.account_service()
            .verify_user_action(voter, &vote_message(proposal.0, aye), sig)
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
                self.moderation_service()
                    .total_voter_weight(p.demos_id, demos.vote_weighting, now)
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
}
