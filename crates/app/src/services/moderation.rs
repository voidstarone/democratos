//! Facade delegators for moderation use-cases. The logic now lives in
//! [`ModerationService`](super::moderation_service::ModerationService); these thin
//! methods keep `services.file_report()` and friends working for call sites not
//! yet migrated off the `Services` aggregator.

use domain::{
    DemosId, Report, ReportId, ReportReason, ReportTarget, Trial, TrialComment, TrialId, UserId,
    Verdict,
};

use crate::{
    CanPostError, CastJuryVoteError, CommentOnTrialError, OpenTrialError, Result, SettleTrialError,
};

use super::moderation_service::ModerationService;
use super::services::Services;

impl Services {
    /// Build the extracted [`ModerationService`] from the ports this aggregator
    /// still holds, wiring its [`AccountService`](super::account_service::AccountService)
    /// peer inline. Cheap — `Arc` clones only — so callers construct one per call
    /// rather than storing a field (which would break every `Services { … }`
    /// literal). Removed once all call sites inject `ModerationService` directly.
    pub(super) fn moderation_service(&self) -> ModerationService {
        ModerationService::new(
            self.reports.clone(),
            self.trials.clone(),
            self.trial_comments.clone(),
            self.demoi.clone(),
            self.memberships.clone(),
            self.users.clone(),
            self.notifications.clone(),
            self.posts.clone(),
            self.comments.clone(),
            self.rules.clone(),
            self.clock.clone(),
            std::sync::Arc::new(self.account_service()),
        )
    }

    pub async fn file_report(
        &self,
        reporter: UserId,
        demos: DemosId,
        target: ReportTarget,
        reason: ReportReason,
        note: &str,
    ) -> Result<Report> {
        self.moderation_service()
            .file_report(reporter, demos, target, reason, note)
            .await
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
        self.moderation_service()
            .open_trial(caller, report_id)
            .await
    }

    /// Every comment on a trial, oldest first — the public gallery discussion.
    pub async fn trial_comments(&self, trial: TrialId) -> Result<Vec<TrialComment>> {
        self.moderation_service().trial_comments(trial).await
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
        self.moderation_service()
            .comment_on_trial(trial_id, author, body)
            .await
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
        self.moderation_service()
            .cast_jury_vote(trial_id, juror, guilty, sig)
            .await
    }

    /// Recompute a trial's verdict and, if decisive, apply consequences:
    /// a guilty verdict sanctions the accused (which disqualifies them from the
    /// franchise) and removes the reported content.
    pub async fn settle_trial(&self, trial_id: TrialId) -> Result<Verdict, SettleTrialError> {
        self.moderation_service().settle_trial(trial_id).await
    }

    /// The ban term (days) a conviction on the report behind a trial would carry,
    /// for showing jurors the stakes before they vote. `None` if the report is
    /// gone.
    pub async fn proposed_ban_term(&self, report_id: ReportId) -> Result<Option<u32>> {
        self.moderation_service().proposed_ban_term(report_id).await
    }

    /// Whether `user` may create a post in `demos` under its posting policy.
    /// Backs both the enforcement in [`create_post`](Self::create_post) and the
    /// composer's community picker (so it only offers postable communities).
    pub async fn can_post(&self, user: UserId, demos: DemosId) -> Result<bool, CanPostError> {
        self.moderation_service().can_post(user, demos).await
    }
}
