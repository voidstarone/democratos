

use domain::{
    reach_verdict, select_jury, ContentScale, Demos,
    DemosId, Membership, NotificationKind, MAX_SANCTION_DAYS, PostingPolicy, Report, ReportId,
    ReportReason, ReportStatus, ReportTarget, RuleId, Timestamp, Trial, TrialComment, TrialId, User, UserId, Verdict,
    VoteWeighting,
};


use crate::identity::jury_vote_message::jury_vote_message;
use crate::{
    CanPostError, CastJuryVoteError, CommentOnTrialError, MemberActionError, OpenTrialError, Result,
    SettleTrialError, StoreError,
};


use super::posting_allowed::posting_allowed;
use super::services::Services;

impl Services {
    pub async fn file_report(
        &self,
        reporter: UserId,
        demos: DemosId,
        target: ReportTarget,
        reason: ReportReason,
        note: &str,
    ) -> Result<Report> {
        // Reporter must be a member of the demos.
        self.memberships
            .get(reporter, demos)
            .await?
            .ok_or(StoreError::NotFound)?;
        self.file_or_merge_flag(
            demos,
            Some(reporter),
            target,
            reason,
            note,
            self.clock.now(),
        )
        .await
    }

    /// File an accusation against `target`, folding it into the existing *open*
    /// report for that target if one exists — so a post flagged again for a
    /// different reason adds a charge to the original case rather than opening a
    /// parallel report. A report already on trial or resolved is left alone (its
    /// charges are fixed), so a fresh flag opens a new case.
    pub(super) async fn file_or_merge_flag(
        &self,
        demos: DemosId,
        reporter: Option<UserId>,
        target: ReportTarget,
        reason: ReportReason,
        note: &str,
        now: Timestamp,
    ) -> Result<Report> {
        // `list_open` already restricts to Open reports, which is exactly the
        // set a new flag may merge into.
        let existing = self
            .reports
            .list_open(demos)
            .await?
            .into_iter()
            .find(|r| r.target == target);
        match existing {
            Some(mut report) => {
                report.add_flag(reporter, reason, note, now);
                self.reports.update(&report).await?;
                Ok(report)
            }
            None => {
                self.reports
                    .create(demos, reporter, target, reason, note, now)
                    .await
            }
        }
    }

    /// Empanel a jury for an open report and put it on trial. The panel is a
    /// deterministic random draw of *voters* (seeded by the report id, excluding
    /// the accused), sized by the demos's [`JurySizing`](domain::JurySizing)
    /// policy: a minority of the electorate that shrinks as the demos grows,
    /// smaller for comments than posts. Errors with `JuryTooSmall` when the demos
    /// has too few voters to seat a minority panel.
    pub async fn open_trial(
        &self,
        caller: UserId,
        report_id: ReportId,
    ) -> Result<Trial, OpenTrialError> {
        let mut report = self
            .reports
            .get(report_id)
            .await?
            .ok_or(StoreError::NotFound)?;
        if report.status != ReportStatus::Open {
            return Err(OpenTrialError::ReportNotOpen);
        }
        // Empanelling a jury is a governance/moderation action, not a public one:
        // gate it on the caller being an unsanctioned voter of the report's
        // community. Without this, any signed-in user could force *any* report in
        // *any* community to trial — freezing its charge set or griefing at will
        // (mirrors the same gate on `close_proposal`).
        let membership = self
            .memberships
            .get(caller, report.demos_id)
            .await?
            .ok_or(OpenTrialError::NotAVoter)?;
        if !membership.is_voter() || membership.is_sanctioned(self.clock.now()) {
            return Err(OpenTrialError::NotAVoter);
        }
        let accused = self.resolve_accused(&report).await?;
        let demos = self
            .demoi
            .get(report.demos_id)
            .await?
            .ok_or(StoreError::NotFound)?;
        let now = self.clock.now();

        // Jurors are drawn from the enfranchised electorate (one weight lookup
        // per voter, so keep the memberships around to weigh the panel). A
        // sanctioned member is disqualified from the franchise, so they are never
        // eligible to sit on a jury.
        let voters: Vec<Membership> = self
            .memberships
            .members(report.demos_id)
            .await?
            .into_iter()
            .filter(|m| m.is_franchised(self.clock.now()))
            .collect();

        // Comments are lower-stakes than posts; a user-level report (e.g. a bot)
        // is juried at post weight.
        let scale = match report.target {
            ReportTarget::Comment(_) => ContentScale::Comment,
            ReportTarget::Post(_) | ReportTarget::User(_) => ContentScale::Post,
        };
        let size = demos.jury_sizing.jury_size(voters.len() as u64, scale);
        if size == 0 {
            return Err(OpenTrialError::JuryTooSmall);
        }

        let candidate_ids: Vec<UserId> = voters.iter().map(|m| m.user_id).collect();
        let jurors = select_jury(&candidate_ids, accused, size, report.id.0);

        // Freeze the panel's total weight now, so the conviction bar can't shift
        // mid-trial. Unweighted juries weigh 1 each → jury_weight == jurors.len().
        let weigh_jury = demos.weighting_scope.applies_to_juries();
        // Freeze each juror's weight now, aligned by index with `jurors`, and sum
        // them for the conviction denominator. The ballot tally later weighs each
        // vote by these same frozen values (`Trial::juror_weight`), so the guilty
        // numerator and the `jury_weight` denominator share one basis and cannot
        // drift apart if a juror's live contribution changes mid-trial. Under
        // one-juror-one-vote every weight is 1.
        let juror_weights: Vec<u64> = jurors
            .iter()
            .map(|j| match voters.iter().find(|m| m.user_id == *j) {
                Some(m) if weigh_jury => demos.vote_weighting.weight_of(m, now),
                _ => 1,
            })
            .collect();
        let jury_weight: u64 = juror_weights.iter().sum();

        let trial = self
            .trials
            .create(
                report.demos_id,
                report.id,
                accused,
                jurors,
                jury_weight,
                juror_weights,
                now,
                now.plus_days(3),
            )
            .await?;
        report.status = ReportStatus::OnTrial(trial.id);
        self.reports.update(&report).await?;
        // Summon each empanelled juror who wants jury alerts to come and vote.
        for juror in &trial.jurors {
            if let Some(u) = self.users.get(*juror).await? {
                if u.allows_jury_alerts {
                    self.notifications
                        .push(
                            *juror,
                            NotificationKind::JurySummons {
                                trial: trial.id,
                                demos: trial.demos_id,
                            },
                            now,
                        )
                        .await?;
                }
            }
        }
        Ok(trial)
    }

    /// Every comment on a trial, oldest first — the public gallery discussion.
    pub async fn trial_comments(&self, trial: TrialId) -> Result<Vec<TrialComment>> {
        self.trial_comments.list_for_trial(trial).await
    }

    /// A voter comments on a trial. Any enfranchised voter of the trial's demos may
    /// speak — juror or not — so the case is argued in the open. The comment does
    /// not touch the verdict; it is context the electorate (and any watching juror)
    /// can weigh. Every party already in the case — the accused, the reporters, the
    /// jurors, and anyone who has already commented — is pinged (unless they have
    /// opted out of trial-comment alerts), so a running argument reaches the people
    /// it concerns without anyone having to poll the page.
    pub async fn comment_on_trial(
        &self,
        trial_id: TrialId,
        author: UserId,
        body: &str,
    ) -> Result<TrialComment, CommentOnTrialError> {
        let body = body.trim();
        if body.is_empty() {
            return Err(CommentOnTrialError::Empty);
        }
        let trial = self.trials.get(trial_id).await?.ok_or(StoreError::NotFound)?;
        // Commenting is a franchise right: only an enfranchised (unsanctioned) voter
        // of this demos may speak in its gallery. The accused, if not a voter, can
        // still read the public record but not argue here.
        let membership = self
            .memberships
            .get(author, trial.demos_id)
            .await?
            .ok_or(CommentOnTrialError::NotAVoter)?;
        if !membership.is_franchised(self.clock.now()) {
            return Err(CommentOnTrialError::NotAVoter);
        }
        let now = self.clock.now();
        let comment = self
            .trial_comments
            .add(trial_id, author, body.to_string(), now)
            .await?;
        self.notify_trial_comment(&trial, author, now).await?;
        Ok(comment)
    }

    /// Ping everyone party to `trial` — the accused, its reporters, its jurors, and
    /// anyone who has already commented — that a new comment landed, skipping the
    /// commenter and anyone who has opted out of trial-comment alerts. Best-effort,
    /// mirroring [`Self::notify_mentions`]: called after the comment is stored.
    async fn notify_trial_comment(
        &self,
        trial: &Trial,
        author: UserId,
        now: Timestamp,
    ) -> Result<()> {
        // Build the audience: accused + jurors + reporters + prior commenters.
        let mut audience: Vec<UserId> = Vec::new();
        audience.push(trial.accused);
        audience.extend(trial.jurors.iter().copied());
        if let Some(report) = self.reports.get(trial.report_id).await? {
            audience.extend(report.flags.iter().filter_map(|f| f.reporter));
        }
        for c in self.trial_comments.list_for_trial(trial.id).await? {
            audience.push(c.author);
        }
        // Dedup and drop the commenter — never notify yourself of your own comment.
        audience.sort_by_key(|u| u.0);
        audience.dedup();
        for recipient in audience {
            if recipient == author {
                continue;
            }
            if let Some(u) = self.users.get(recipient).await? {
                if u.allows_trial_comment_alerts {
                    self.notifications
                        .push(
                            recipient,
                            NotificationKind::TrialComment {
                                trial: trial.id,
                                demos: trial.demos_id,
                                by: author,
                            },
                            now,
                        )
                        .await?;
                }
            }
        }
        Ok(())
    }

    /// A juror votes. Returns the (possibly now-decided) verdict; a decisive
    /// verdict applies its consequences immediately.
    pub async fn cast_jury_vote(
        &self,
        trial_id: TrialId,
        juror: UserId,
        guilty: bool,
        sig: Option<&str>,
    ) -> Result<Verdict, CastJuryVoteError> {
        let trial = self.trials.get(trial_id).await?.ok_or(StoreError::NotFound)?;
        if trial.verdict != Verdict::Pending {
            return Err(CastJuryVoteError::TrialClosed);
        }
        if !trial.is_juror(juror) {
            return Err(CastJuryVoteError::NotAJuror);
        }
        // A juror sanctioned after empanelment (e.g. convicted in a parallel
        // trial) is disqualified from the franchise and may no longer vote a
        // verdict. A juror who has since left the community (no membership) keeps
        // the seat they were drawn into.
        if let Some(m) = self.memberships.get(juror, trial.demos_id).await? {
            if m.is_sanctioned(self.clock.now()) {
                return Err(CastJuryVoteError::Sanctioned);
            }
        }
        // The verdict ballot must be signed by the juror, verified on the owner —
        // so a node hosting a juror can't cast a verdict in their name.
        self.account_service()
            .verify_user_action(juror, &jury_vote_message(trial_id.0, guilty), sig)
            .await?;
        // Weigh the ballot by this juror's weight *frozen at empanelment*, not a
        // live recomputation: the panel's total (`jury_weight`) was frozen from the
        // same per-juror values, so the guilty/nay sums and the conviction
        // denominator stay in one basis. Recomputing live here would let a juror
        // shift the 2/3 bar mid-trial by pumping their contribution.
        let weight = trial.juror_weight(juror);
        self.trials
            .cast_ballot(trial_id, juror, guilty, weight)
            .await?;
        Ok(self.settle_trial(trial_id).await?)
    }

    /// Recompute a trial's verdict and, if decisive, apply consequences:
    /// a guilty verdict sanctions the accused (which disqualifies them from the
    /// franchise) and removes the reported content.
    pub async fn settle_trial(&self, trial_id: TrialId) -> Result<Verdict, SettleTrialError> {
        let mut trial = self.trials.get(trial_id).await?.ok_or(StoreError::NotFound)?;
        if trial.verdict != Verdict::Pending {
            return Ok(trial.verdict);
        }

        let (guilty, not_guilty) = self.trials.ballot_tally(trial_id).await?;
        let verdict = reach_verdict(guilty, not_guilty, trial.jury_weight);
        if verdict == Verdict::Pending {
            return Ok(Verdict::Pending);
        }

        trial.verdict = verdict;
        self.trials.update(&trial).await?;

        let mut report = self
            .reports
            .get(trial.report_id)
            .await?
            .ok_or(StoreError::NotFound)?;
        match verdict {
            Verdict::Guilty => {
                report.status = ReportStatus::Upheld;
                if let Some(mut m) = self.memberships.get(trial.accused, trial.demos_id).await? {
                    // The ban's length is tied to the rule(s) the case cited — the
                    // term the voters fixed for that rule ahead of the trial — not a
                    // flat maximum. `sanction_for` still caps at MAX_SANCTION_DAYS.
                    let term = self.ban_term_for_report(&report).await?;
                    m.sanction_for(self.clock.now(), term);
                    self.memberships.upsert(m).await?;
                }
                match report.target {
                    ReportTarget::Post(p) => self.posts.set_removed(p, true).await?,
                    ReportTarget::Comment(c) => self.comments.set_removed(c, true).await?,
                    ReportTarget::User(_) => {}
                }
            }
            Verdict::NotGuilty => report.status = ReportStatus::Dismissed,
            Verdict::Pending => {}
        }
        self.reports.update(&report).await?;
        Ok(verdict)
    }

    /// Total vote weight of the electorate under `scheme` — the quorum
    /// denominator for a weighted proposal, summed over the same population
    /// (`tier == Voter`) that [`MembershipStore::voter_count`](crate::MembershipStore::voter_count)
    /// counts.
    pub(super) async fn total_voter_weight(
        &self,
        demos: DemosId,
        scheme: VoteWeighting,
        now: Timestamp,
    ) -> Result<u64> {
        Ok(self
            .memberships
            .members(demos)
            .await?
            .iter()
            .filter(|m| m.is_voter())
            .map(|m| scheme.weight_of(m, now))
            .sum())
    }

    /// The ban term (days) a conviction on the report behind `trial` would carry,
    /// for showing jurors the stakes before they vote. `None` if the report is
    /// gone. See [`Self::ban_term_for_report`] for how it's derived.
    pub async fn proposed_ban_term(&self, report_id: ReportId) -> Result<Option<u32>> {
        match self.reports.get(report_id).await? {
            Some(report) => Ok(Some(self.ban_term_for_report(&report).await?)),
            None => Ok(None),
        }
    }

    /// The ban term (days) a conviction on `report` carries: the most severe of
    /// the rule terms the case cited, each read against the community's live
    /// ceiling. A case that cites no specific rule (a bot/NSFW flag, or a bare
    /// rule-break) falls back to the community ceiling. Never exceeds it — and
    /// `Membership::sanction_for` caps the result at the 18-year platform maximum.
    async fn ban_term_for_report(&self, report: &Report) -> Result<u32> {
        let ceiling = match self.demoi.get(report.demos_id).await? {
            Some(d) => d.ban_ceiling_days(),
            None => MAX_SANCTION_DAYS,
        };
        // The distinct rules named across the case's flags.
        let cited: Vec<RuleId> = report
            .flags
            .iter()
            .filter_map(|f| match f.reason {
                ReportReason::RuleBreak { rule: Some(id) } => Some(id),
                _ => None,
            })
            .collect();
        let mut term = 0u32;
        for id in cited {
            if let Some(rule) = self.rules.get(id).await? {
                term = term.max(rule.term_days(ceiling));
            }
        }
        // No cited rule carried a resolvable term → the community ceiling governs.
        Ok(if term == 0 { ceiling } else { term })
    }

    async fn resolve_accused(&self, report: &Report) -> Result<UserId> {
        match report.target {
            ReportTarget::User(u) => Ok(u),
            ReportTarget::Post(p) => Ok(self.posts.get(p).await?.ok_or(StoreError::NotFound)?.author),
            ReportTarget::Comment(c) => Ok(self
                .comments
                .get(c)
                .await?
                .ok_or(StoreError::NotFound)?
                .author),
        }
    }

    pub(super) async fn require_unsanctioned_member(
        &self,
        user: UserId,
        demos: DemosId,
    ) -> Result<Membership, MemberActionError> {
        let m = self
            .memberships
            .get(user, demos)
            .await?
            .ok_or(StoreError::NotFound)?;
        if m.is_sanctioned(self.clock.now()) {
            return Err(MemberActionError::Sanctioned);
        }
        Ok(m)
    }

    /// Whether `user` may create a post in `demos` under its posting policy.
    /// Backs both the enforcement in [`create_post`](Self::create_post) and the
    /// composer's community picker (so it only offers postable communities).
    pub async fn can_post(&self, user: UserId, demos: DemosId) -> Result<bool, CanPostError> {
        let d = self.demoi.get(demos).await?.ok_or(StoreError::NotFound)?;
        let m = self.memberships.get(user, demos).await?;
        Ok(posting_allowed(d.posting_policy, m.as_ref(), self.clock.now()))
    }

    /// Like [`can_post`](Self::can_post) but returns a policy-specific error the
    /// UI can show, rather than a bool.
    pub(super) async fn require_can_post(&self, user: UserId, demos: DemosId) -> Result<(), CanPostError> {
        let d = self.demoi.get(demos).await?.ok_or(StoreError::NotFound)?;
        let m = self.memberships.get(user, demos).await?;
        if posting_allowed(d.posting_policy, m.as_ref(), self.clock.now()) {
            return Ok(());
        }
        // A sanction is its own distinct error (blocks posting under any policy).
        if m.as_ref().is_some_and(|m| m.is_sanctioned(self.clock.now())) {
            return Err(CanPostError::Sanctioned);
        }
        let msg = match d.posting_policy {
            PostingPolicy::Members => "join this community to post here",
            PostingPolicy::Voters => "only voters may post in this community",
            PostingPolicy::MinContribution(_) => {
                "you haven't earned enough popularity to post in this community yet"
            }
            PostingPolicy::Open => "you cannot post in this community",
        };
        Err(CanPostError::Rejected(msg.to_string()))
    }

    pub(super) async fn load_triplet(
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
