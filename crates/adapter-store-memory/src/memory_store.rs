//! The `MemoryStore` type and its implementations of every store port.

use std::collections::HashSet;
use std::sync::Mutex;

use async_trait::async_trait;

use app::{MediaError, Result, StoreError};
use app::{
    CommentStore, CommentVoteStore, DemosStore, FoundingStore, MediaStore, MembershipStore,
    PostStore, PostVoteStore, ProposalStore, ReportStore, RuleStore, TrialStore, UserStore,
    VoteStore,
};
use domain::{
    Comment, CommentId, Demos, DemosId, FeedPaging, FoundingId, FoundingPetition, FranchiseCriteria,
    JurySizing, Media, Membership, Post, PostId, PostingPolicy, Proposal, ProposalId, ProposalKind,
    Report, ReportId, ReportReason, ReportStatus, ReportTarget, Rule, RuleId, Tally, Tier,
    Timestamp, Trial, TrialId, User, UserId, Verdict, VoteWeighting, WeightingScope,
};

use crate::comment_vote_rec::CommentVoteRec;
use crate::inner::Inner;
use crate::jury_ballot_rec::JuryBallotRec;
use crate::post_vote_rec::PostVoteRec;
use crate::vote_rec::VoteRec;

pub struct MemoryStore {
    inner: Mutex<Inner>,
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner::default()),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        // A poisoned lock means a prior panic; surfacing it as a panic here is
        // acceptable for an in-memory dev/test store.
        self.inner.lock().expect("memory store lock poisoned")
    }
}

#[async_trait]
impl UserStore for MemoryStore {
    async fn create(
        &self,
        handle: &str,
        email: Option<&str>,
        password_hash: Option<&str>,
        created_at: Timestamp,
    ) -> Result<User> {
        let mut g = self.lock();
        g.next_user += 1;
        let mut user = User::new(UserId(g.next_user), handle, created_at);
        user.email = email.map(str::to_string);
        user.password_hash = password_hash.map(str::to_string);
        g.users.push(user.clone());
        Ok(user)
    }

    async fn get(&self, id: UserId) -> Result<Option<User>> {
        Ok(self.lock().users.iter().find(|u| u.id == id).cloned())
    }

    async fn by_email(&self, email: &str) -> Result<Option<User>> {
        Ok(self
            .lock()
            .users
            .iter()
            .find(|u| u.email.as_deref() == Some(email))
            .cloned())
    }

    async fn set_is_age_verified(&self, id: UserId, is_verified: bool) -> Result<()> {
        let mut g = self.lock();
        let u = g
            .users
            .iter_mut()
            .find(|u| u.id == id)
            .ok_or(StoreError::NotFound)?;
        u.is_age_verified = is_verified;
        Ok(())
    }

    async fn set_public_key(&self, id: UserId, public_key_hex: &str) -> Result<()> {
        let mut g = self.lock();
        let u = g
            .users
            .iter_mut()
            .find(|u| u.id == id)
            .ok_or(StoreError::NotFound)?;
        u.public_key = Some(public_key_hex.to_string());
        Ok(())
    }

    async fn set_franchise_barred(&self, id: UserId, barred: bool) -> Result<()> {
        let mut g = self.lock();
        let u = g
            .users
            .iter_mut()
            .find(|u| u.id == id)
            .ok_or(StoreError::NotFound)?;
        u.is_franchise_barred = barred;
        Ok(())
    }

    async fn set_feed_paging(&self, id: UserId, paging: FeedPaging) -> Result<()> {
        let mut g = self.lock();
        let u = g
            .users
            .iter_mut()
            .find(|u| u.id == id)
            .ok_or(StoreError::NotFound)?;
        u.feed_paging = paging;
        Ok(())
    }

    async fn by_handle(&self, handle: &str) -> Result<Option<User>> {
        Ok(self
            .lock()
            .users
            .iter()
            .find(|u| u.handle == handle)
            .cloned())
    }

    async fn list(&self) -> Result<Vec<User>> {
        Ok(self.lock().users.clone())
    }
}

#[async_trait]
impl DemosStore for MemoryStore {
    async fn create(
        &self,
        slug: &str,
        name: &str,
        founder: UserId,
        created_at: Timestamp,
    ) -> Result<Demos> {
        let mut g = self.lock();
        g.next_demos += 1;
        let demos = Demos::new(DemosId(g.next_demos), slug, name, founder, created_at);
        g.demoi.push(demos.clone());
        Ok(demos)
    }

    async fn get(&self, id: DemosId) -> Result<Option<Demos>> {
        Ok(self.lock().demoi.iter().find(|d| d.id == id).cloned())
    }

    async fn by_slug(&self, slug: &str) -> Result<Option<Demos>> {
        Ok(self.lock().demoi.iter().find(|d| d.slug == slug).cloned())
    }

    async fn update_criteria(&self, id: DemosId, criteria: FranchiseCriteria) -> Result<()> {
        let mut g = self.lock();
        let d = g
            .demoi
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or(StoreError::NotFound)?;
        d.criteria = criteria;
        Ok(())
    }

    async fn set_allows_nsfw(&self, id: DemosId, allows_nsfw: bool) -> Result<()> {
        let mut g = self.lock();
        let d = g
            .demoi
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or(StoreError::NotFound)?;
        d.allows_nsfw = allows_nsfw;
        Ok(())
    }

    async fn set_jury_sizing(&self, id: DemosId, sizing: JurySizing) -> Result<()> {
        let mut g = self.lock();
        let d = g
            .demoi
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or(StoreError::NotFound)?;
        d.jury_sizing = sizing;
        Ok(())
    }

    async fn set_vote_weighting(&self, id: DemosId, scheme: VoteWeighting) -> Result<()> {
        let mut g = self.lock();
        let d = g
            .demoi
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or(StoreError::NotFound)?;
        d.vote_weighting = scheme;
        Ok(())
    }

    async fn set_weighting_scope(&self, id: DemosId, scope: WeightingScope) -> Result<()> {
        let mut g = self.lock();
        let d = g
            .demoi
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or(StoreError::NotFound)?;
        d.weighting_scope = scope;
        Ok(())
    }

    async fn set_posting_policy(&self, id: DemosId, policy: PostingPolicy) -> Result<()> {
        let mut g = self.lock();
        let d = g
            .demoi
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or(StoreError::NotFound)?;
        d.posting_policy = policy;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<Demos>> {
        Ok(self.lock().demoi.clone())
    }
}

#[async_trait]
impl FoundingStore for MemoryStore {
    async fn create(
        &self,
        slug: &str,
        name: &str,
        founder: UserId,
        created_at: Timestamp,
    ) -> Result<FoundingPetition> {
        let mut g = self.lock();
        g.next_founding += 1;
        let petition = FoundingPetition {
            id: FoundingId(g.next_founding),
            slug: slug.to_string(),
            name: name.to_string(),
            founder,
            sign_offs: Vec::new(),
            created_at,
        };
        g.foundings.push(petition.clone());
        Ok(petition)
    }

    async fn get(&self, id: FoundingId) -> Result<Option<FoundingPetition>> {
        Ok(self.lock().foundings.iter().find(|p| p.id == id).cloned())
    }

    async fn sign(&self, id: FoundingId, user: UserId) -> Result<FoundingPetition> {
        let mut g = self.lock();
        let p = g
            .foundings
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or(StoreError::NotFound)?;
        // Idempotent: a repeat sign-off (or the founder's own) never double-counts.
        if user != p.founder && !p.sign_offs.contains(&user) {
            p.sign_offs.push(user);
        }
        Ok(p.clone())
    }

    async fn delete(&self, id: FoundingId) -> Result<()> {
        self.lock().foundings.retain(|p| p.id != id);
        Ok(())
    }

    async fn list(&self) -> Result<Vec<FoundingPetition>> {
        let mut v = self.lock().foundings.clone();
        // Newest first.
        v.reverse();
        Ok(v)
    }
}

#[async_trait]
impl MembershipStore for MemoryStore {
    async fn upsert(&self, membership: Membership) -> Result<()> {
        let mut g = self.lock();
        if let Some(slot) = g
            .memberships
            .iter_mut()
            .find(|m| m.user_id == membership.user_id && m.demos_id == membership.demos_id)
        {
            *slot = membership;
        } else {
            g.memberships.push(membership);
        }
        Ok(())
    }

    async fn get(&self, user: UserId, demos: DemosId) -> Result<Option<Membership>> {
        Ok(self
            .lock()
            .memberships
            .iter()
            .find(|m| m.user_id == user && m.demos_id == demos)
            .cloned())
    }

    async fn members(&self, demos: DemosId) -> Result<Vec<Membership>> {
        Ok(self
            .lock()
            .memberships
            .iter()
            .filter(|m| m.demos_id == demos)
            .cloned()
            .collect())
    }

    async fn list_for_user(&self, user: UserId) -> Result<Vec<Membership>> {
        Ok(self
            .lock()
            .memberships
            .iter()
            .filter(|m| m.user_id == user)
            .cloned()
            .collect())
    }

    async fn voter_count(&self, demos: DemosId) -> Result<u64> {
        Ok(self
            .lock()
            .memberships
            .iter()
            .filter(|m| m.demos_id == demos && m.tier == Tier::Voter)
            .count() as u64)
    }

    async fn admitted_since(&self, demos: DemosId, since: Timestamp) -> Result<u64> {
        Ok(self
            .lock()
            .memberships
            .iter()
            .filter(|m| {
                m.demos_id == demos
                    && m.tier == Tier::Voter
                    && m.enfranchised_at.is_some_and(|t| t >= since)
            })
            .count() as u64)
    }
}

#[async_trait]
impl ProposalStore for MemoryStore {
    async fn create(
        &self,
        demos: DemosId,
        proposer: UserId,
        kind: ProposalKind,
        opened_at: Timestamp,
        closes_at: Timestamp,
    ) -> Result<Proposal> {
        let mut g = self.lock();
        g.next_proposal += 1;
        let p = Proposal::new(
            ProposalId(g.next_proposal),
            demos,
            proposer,
            kind,
            opened_at,
            closes_at,
        );
        g.proposals.push(p.clone());
        Ok(p)
    }

    async fn get(&self, id: ProposalId) -> Result<Option<Proposal>> {
        Ok(self.lock().proposals.iter().find(|p| p.id == id).cloned())
    }

    async fn update(&self, proposal: &Proposal) -> Result<()> {
        let mut g = self.lock();
        let slot = g
            .proposals
            .iter_mut()
            .find(|p| p.id == proposal.id)
            .ok_or(StoreError::NotFound)?;
        *slot = proposal.clone();
        Ok(())
    }

    async fn list(&self, demos: DemosId) -> Result<Vec<Proposal>> {
        Ok(self
            .lock()
            .proposals
            .iter()
            .filter(|p| p.demos_id == demos)
            .cloned()
            .collect())
    }
}

#[async_trait]
impl VoteStore for MemoryStore {
    async fn cast(
        &self,
        proposal: ProposalId,
        voter: UserId,
        aye: bool,
        weight: u64,
        _at: Timestamp,
    ) -> Result<()> {
        let mut g = self.lock();
        if g.votes
            .iter()
            .any(|v| v.proposal == proposal && v.voter == voter)
        {
            return Err(StoreError::AlreadyVoted);
        }
        g.votes.push(VoteRec {
            proposal,
            voter,
            aye,
            weight,
        });
        Ok(())
    }

    async fn has_voted(&self, proposal: ProposalId, voter: UserId) -> Result<bool> {
        Ok(self
            .lock()
            .votes
            .iter()
            .any(|v| v.proposal == proposal && v.voter == voter))
    }

    async fn tally(&self, proposal: ProposalId) -> Result<Tally> {
        let g = self.lock();
        let mut tally = Tally::default();
        for v in g.votes.iter().filter(|v| v.proposal == proposal) {
            if v.aye {
                tally.aye += v.weight;
            } else {
                tally.nay += v.weight;
            }
        }
        Ok(tally)
    }
}

#[async_trait]
impl PostVoteStore for MemoryStore {
    async fn set(&self, post: PostId, user: UserId, dir: Option<bool>) -> Result<()> {
        let mut g = self.lock();
        g.post_votes.retain(|v| !(v.post == post && v.user == user));
        if let Some(up) = dir {
            g.post_votes.push(PostVoteRec { post, user, up });
        }
        Ok(())
    }

    async fn get(&self, post: PostId, user: UserId) -> Result<Option<bool>> {
        Ok(self
            .lock()
            .post_votes
            .iter()
            .find(|v| v.post == post && v.user == user)
            .map(|v| v.up))
    }

    async fn score(&self, post: PostId) -> Result<i64> {
        Ok(self
            .lock()
            .post_votes
            .iter()
            .filter(|v| v.post == post)
            .map(|v| if v.up { 1 } else { -1 })
            .sum())
    }

    async fn vote_count(&self) -> Result<u64> {
        Ok(self.lock().post_votes.len() as u64)
    }

    async fn all_votes(&self) -> Result<Vec<(PostId, UserId, bool)>> {
        Ok(self
            .lock()
            .post_votes
            .iter()
            .map(|v| (v.post, v.user, v.up))
            .collect())
    }

    async fn liked_by(&self, user: UserId) -> Result<Vec<PostId>> {
        Ok(self
            .lock()
            .post_votes
            .iter()
            .filter(|v| v.user == user && v.up)
            .map(|v| v.post)
            .collect())
    }

    async fn voted_by(&self, user: UserId) -> Result<Vec<PostId>> {
        Ok(self
            .lock()
            .post_votes
            .iter()
            .filter(|v| v.user == user)
            .map(|v| v.post)
            .collect())
    }
}

#[async_trait]
impl RuleStore for MemoryStore {
    async fn create(&self, demos: DemosId, text: &str, at: Timestamp) -> Result<Rule> {
        let mut g = self.lock();
        g.next_rule += 1;
        let rule = Rule::new(RuleId(g.next_rule), demos, text, at);
        g.rules.push(rule.clone());
        Ok(rule)
    }

    async fn get(&self, id: RuleId) -> Result<Option<Rule>> {
        Ok(self.lock().rules.iter().find(|r| r.id == id).cloned())
    }

    async fn set_active(&self, id: RuleId, active: bool) -> Result<()> {
        let mut g = self.lock();
        let r = g
            .rules
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or(StoreError::NotFound)?;
        r.active = active;
        Ok(())
    }

    async fn list_active(&self, demos: DemosId) -> Result<Vec<Rule>> {
        Ok(self
            .lock()
            .rules
            .iter()
            .filter(|r| r.demos_id == demos && r.active)
            .cloned()
            .collect())
    }
}

#[async_trait]
impl PostStore for MemoryStore {
    async fn create(
        &self,
        demos: DemosId,
        author: UserId,
        title: &str,
        body: &str,
        media: Vec<Media>,
        tags: Vec<String>,
        at: Timestamp,
    ) -> Result<Post> {
        let mut g = self.lock();
        g.next_post += 1;
        let post = Post::new(
            PostId(g.next_post),
            demos,
            author,
            title,
            body,
            media,
            tags,
            at,
        );
        g.posts.push(post.clone());
        Ok(post)
    }

    async fn get(&self, id: PostId) -> Result<Option<Post>> {
        Ok(self.lock().posts.iter().find(|p| p.id == id).cloned())
    }

    async fn set_removed(&self, id: PostId, removed: bool) -> Result<()> {
        let mut g = self.lock();
        let p = g
            .posts
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or(StoreError::NotFound)?;
        p.removed = removed;
        Ok(())
    }

    async fn set_is_nsfw(&self, id: PostId, is_nsfw: bool) -> Result<()> {
        let mut g = self.lock();
        let p = g
            .posts
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or(StoreError::NotFound)?;
        p.is_nsfw = is_nsfw;
        Ok(())
    }

    async fn list(&self, demos: DemosId) -> Result<Vec<Post>> {
        Ok(self
            .lock()
            .posts
            .iter()
            .filter(|p| p.demos_id == demos)
            .cloned()
            .collect())
    }

    async fn list_by_author(&self, demos: DemosId, author: UserId) -> Result<Vec<Post>> {
        Ok(self
            .lock()
            .posts
            .iter()
            .filter(|p| p.demos_id == demos && p.author == author)
            .cloned()
            .collect())
    }

    async fn list_all(&self) -> Result<Vec<Post>> {
        Ok(self.lock().posts.clone())
    }

    async fn distinct_demos_by_author(&self, author: UserId) -> Result<u64> {
        Ok(self
            .lock()
            .posts
            .iter()
            .filter(|p| p.author == author)
            .map(|p| p.demos_id)
            .collect::<HashSet<_>>()
            .len() as u64)
    }
}

#[async_trait]
impl MediaStore for MemoryStore {
    async fn put(&self, content_type: &str, bytes: Vec<u8>) -> Result<String, MediaError> {
        let key = app::media_key(content_type, &bytes)
            .ok_or_else(|| MediaError::Store(format!("unsupported media type: {content_type}")))?;
        self.lock()
            .media
            .insert(key.clone(), (content_type.to_string(), bytes));
        Ok(format!("/media/{key}"))
    }

    async fn get(&self, key: &str) -> Result<Option<(String, Vec<u8>)>, MediaError> {
        Ok(self.lock().media.get(key).cloned())
    }
}

#[async_trait]
impl CommentStore for MemoryStore {
    async fn create(
        &self,
        post: PostId,
        author: UserId,
        parent: Option<CommentId>,
        body: &str,
        at: Timestamp,
    ) -> Result<Comment> {
        let mut g = self.lock();
        g.next_comment += 1;
        let comment = Comment::new(CommentId(g.next_comment), post, author, parent, body, at);
        g.comments.push(comment.clone());
        Ok(comment)
    }

    async fn get(&self, id: CommentId) -> Result<Option<Comment>> {
        Ok(self.lock().comments.iter().find(|c| c.id == id).cloned())
    }

    async fn set_removed(&self, id: CommentId, removed: bool) -> Result<()> {
        let mut g = self.lock();
        let c = g
            .comments
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or(StoreError::NotFound)?;
        c.removed = removed;
        Ok(())
    }

    async fn list_for_post(&self, post: PostId) -> Result<Vec<Comment>> {
        Ok(self
            .lock()
            .comments
            .iter()
            .filter(|c| c.post_id == post)
            .cloned()
            .collect())
    }

    async fn count_by_author_since(&self, author: UserId, since: Timestamp) -> Result<u64> {
        Ok(self
            .lock()
            .comments
            .iter()
            .filter(|c| c.author == author && c.created_at >= since)
            .count() as u64)
    }

    async fn list_by_author(&self, author: UserId) -> Result<Vec<Comment>> {
        Ok(self
            .lock()
            .comments
            .iter()
            .filter(|c| c.author == author)
            .cloned()
            .collect())
    }
}

#[async_trait]
impl CommentVoteStore for MemoryStore {
    async fn set(&self, comment: CommentId, user: UserId, dir: Option<bool>) -> Result<()> {
        let mut g = self.lock();
        g.comment_votes
            .retain(|v| !(v.comment == comment && v.user == user));
        if let Some(up) = dir {
            g.comment_votes.push(CommentVoteRec { comment, user, up });
        }
        Ok(())
    }

    async fn get(&self, comment: CommentId, user: UserId) -> Result<Option<bool>> {
        Ok(self
            .lock()
            .comment_votes
            .iter()
            .find(|v| v.comment == comment && v.user == user)
            .map(|v| v.up))
    }

    async fn score(&self, comment: CommentId) -> Result<i64> {
        Ok(self
            .lock()
            .comment_votes
            .iter()
            .filter(|v| v.comment == comment)
            .map(|v| if v.up { 1 } else { -1 })
            .sum())
    }
}

#[async_trait]
impl ReportStore for MemoryStore {
    async fn create(
        &self,
        demos: DemosId,
        reporter: Option<UserId>,
        target: ReportTarget,
        reason: ReportReason,
        note: &str,
        at: Timestamp,
    ) -> Result<Report> {
        let mut g = self.lock();
        g.next_report += 1;
        let report = Report::new(
            ReportId(g.next_report),
            demos,
            reporter,
            target,
            reason,
            note,
            at,
        );
        g.reports.push(report.clone());
        Ok(report)
    }

    async fn get(&self, id: ReportId) -> Result<Option<Report>> {
        Ok(self.lock().reports.iter().find(|r| r.id == id).cloned())
    }

    async fn update(&self, report: &Report) -> Result<()> {
        let mut g = self.lock();
        let slot = g
            .reports
            .iter_mut()
            .find(|r| r.id == report.id)
            .ok_or(StoreError::NotFound)?;
        *slot = report.clone();
        Ok(())
    }

    async fn list_open(&self, demos: DemosId) -> Result<Vec<Report>> {
        Ok(self
            .lock()
            .reports
            .iter()
            .filter(|r| r.demos_id == demos && r.status == ReportStatus::Open)
            .cloned()
            .collect())
    }
}

#[async_trait]
impl TrialStore for MemoryStore {
    async fn create(
        &self,
        demos: DemosId,
        report: ReportId,
        accused: UserId,
        jurors: Vec<UserId>,
        jury_weight: u64,
        juror_weights: Vec<u64>,
        opened_at: Timestamp,
        closes_at: Timestamp,
    ) -> Result<Trial> {
        let mut g = self.lock();
        g.next_trial += 1;
        let trial = Trial::new(
            TrialId(g.next_trial),
            demos,
            report,
            accused,
            jurors,
            jury_weight,
            juror_weights,
            opened_at,
            closes_at,
        );
        g.trials.push(trial.clone());
        Ok(trial)
    }

    async fn get(&self, id: TrialId) -> Result<Option<Trial>> {
        Ok(self.lock().trials.iter().find(|t| t.id == id).cloned())
    }

    async fn update(&self, trial: &Trial) -> Result<()> {
        let mut g = self.lock();
        let slot = g
            .trials
            .iter_mut()
            .find(|t| t.id == trial.id)
            .ok_or(StoreError::NotFound)?;
        *slot = trial.clone();
        Ok(())
    }

    async fn list_open(&self, demos: DemosId) -> Result<Vec<Trial>> {
        Ok(self
            .lock()
            .trials
            .iter()
            .filter(|t| t.demos_id == demos && t.verdict == Verdict::Pending)
            .cloned()
            .collect())
    }

    async fn cast_ballot(
        &self,
        trial: TrialId,
        juror: UserId,
        guilty: bool,
        weight: u64,
    ) -> Result<()> {
        let mut g = self.lock();
        if g.jury_ballots
            .iter()
            .any(|b| b.trial == trial && b.juror == juror)
        {
            return Err(StoreError::AlreadyVoted);
        }
        g.jury_ballots.push(JuryBallotRec {
            trial,
            juror,
            guilty,
            weight,
        });
        Ok(())
    }

    async fn has_voted(&self, trial: TrialId, juror: UserId) -> Result<bool> {
        Ok(self
            .lock()
            .jury_ballots
            .iter()
            .any(|b| b.trial == trial && b.juror == juror))
    }

    async fn ballot_tally(&self, trial: TrialId) -> Result<(u64, u64)> {
        let g = self.lock();
        let mut guilty = 0u64;
        let mut not_guilty = 0u64;
        for b in g.jury_ballots.iter().filter(|b| b.trial == trial) {
            if b.guilty {
                guilty += b.weight;
            } else {
                not_guilty += b.weight;
            }
        }
        Ok((guilty, not_guilty))
    }
}
