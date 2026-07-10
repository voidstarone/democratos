//! The `TextFileStore` type and its implementations of every `*Store` port.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use async_trait::async_trait;

use app::{Result, StoreError};
use app::{
    CommentStore, CommentVoteStore, DemosStore, FoundingStore, MembershipStore, PostStore,
    PostVoteStore, ProposalStore, ReportStore, RuleStore, TrialStore, UserStore, VoteStore,
};
use domain::{
    Comment, CommentId, Demos, DemosId, FeedPaging, FoundingId, FoundingPetition, FranchiseCriteria,
    JurySizing, Media, Membership, Post, PostId, PostingPolicy, Proposal, ProposalId, ProposalKind,
    Report, ReportId, ReportReason, ReportStatus, ReportTarget, Rule, RuleId, Tally, Tier,
    Timestamp, Trial, TrialId, User, UserId, Verdict, VoteWeighting, WeightingScope,
};

use crate::comment_vote_rec::CommentVoteRec;
use crate::db::Db;
use crate::jury_ballot_rec::JuryBallotRec;
use crate::post_vote_rec::PostVoteRec;
use crate::vote_rec::VoteRec;

pub struct TextFileStore {
    path: PathBuf,
    db: Mutex<Db>,
}

impl TextFileStore {
    /// Open the store at `path`, loading existing data if the file is present.
    pub fn open(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let db = if path.exists() {
            let text = std::fs::read_to_string(&path)?;
            serde_json::from_str(&text)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
        } else {
            Db::default()
        };
        Ok(Self {
            path,
            db: Mutex::new(db),
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Db> {
        // Recover rather than panic on a poisoned lock. A poisoned mutex means a
        // previous holder panicked mid-operation; propagating that by `expect`ing
        // would turn every later call into a panic too. The in-memory `Db` is a
        // plain data structure with no cross-field invariant that a partial write
        // could have left inconsistent (each mutation completes before `flush`),
        // so taking the guard via `into_inner()` is safe and keeps the store
        // usable. On-disk state is unaffected: it only changes on a full `flush`.
        self.db.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Persist the current state. Writes to a sibling temp file then renames, so
    /// a crash mid-write cannot corrupt the live file.
    fn flush(&self, db: &Db) -> Result<()> {
        let json = serde_json::to_string_pretty(db)
            .map_err(|e| StoreError::Store(format!("serialize: {e}")))?;
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, json).map_err(|e| StoreError::Store(format!("write: {e}")))?;
        std::fs::rename(&tmp, &self.path).map_err(|e| StoreError::Store(format!("rename: {e}")))?;
        Ok(())
    }
}

#[async_trait]
impl UserStore for TextFileStore {
    async fn create(
        &self,
        handle: &str,
        email: Option<&str>,
        password_hash: Option<&str>,
        created_at: Timestamp,
    ) -> Result<User> {
        let mut db = self.lock();
        db.next_user += 1;
        let mut user = User::new(UserId(db.next_user), handle, created_at);
        user.email = email.map(str::to_string);
        user.password_hash = password_hash.map(str::to_string);
        db.users.push(user.clone());
        self.flush(&db)?;
        Ok(user)
    }

    async fn set_is_age_verified(&self, id: UserId, is_verified: bool) -> Result<()> {
        let mut db = self.lock();
        let u = db
            .users
            .iter_mut()
            .find(|u| u.id == id)
            .ok_or(StoreError::NotFound)?;
        u.is_age_verified = is_verified;
        self.flush(&db)
    }

    async fn set_public_key(&self, id: UserId, public_key_hex: &str) -> Result<()> {
        let mut db = self.lock();
        let u = db
            .users
            .iter_mut()
            .find(|u| u.id == id)
            .ok_or(StoreError::NotFound)?;
        u.public_key = Some(public_key_hex.to_string());
        self.flush(&db)
    }

    async fn set_franchise_barred(&self, id: UserId, barred: bool) -> Result<()> {
        let mut db = self.lock();
        let u = db
            .users
            .iter_mut()
            .find(|u| u.id == id)
            .ok_or(StoreError::NotFound)?;
        u.is_franchise_barred = barred;
        self.flush(&db)
    }

    async fn set_feed_paging(&self, id: UserId, paging: FeedPaging) -> Result<()> {
        let mut db = self.lock();
        let u = db
            .users
            .iter_mut()
            .find(|u| u.id == id)
            .ok_or(StoreError::NotFound)?;
        u.feed_paging = paging;
        self.flush(&db)
    }

    async fn get(&self, id: UserId) -> Result<Option<User>> {
        Ok(self.lock().users.iter().find(|u| u.id == id).cloned())
    }

    async fn by_handle(&self, handle: &str) -> Result<Option<User>> {
        Ok(self
            .lock()
            .users
            .iter()
            .find(|u| u.handle == handle)
            .cloned())
    }

    async fn by_email(&self, email: &str) -> Result<Option<User>> {
        Ok(self
            .lock()
            .users
            .iter()
            .find(|u| u.email.as_deref() == Some(email))
            .cloned())
    }

    async fn list(&self) -> Result<Vec<User>> {
        Ok(self.lock().users.clone())
    }
}

#[async_trait]
impl DemosStore for TextFileStore {
    async fn create(
        &self,
        slug: &str,
        name: &str,
        founder: UserId,
        created_at: Timestamp,
    ) -> Result<Demos> {
        let mut db = self.lock();
        db.next_demos += 1;
        let demos = Demos::new(DemosId(db.next_demos), slug, name, founder, created_at);
        db.demoi.push(demos.clone());
        self.flush(&db)?;
        Ok(demos)
    }

    async fn get(&self, id: DemosId) -> Result<Option<Demos>> {
        Ok(self.lock().demoi.iter().find(|d| d.id == id).cloned())
    }

    async fn by_slug(&self, slug: &str) -> Result<Option<Demos>> {
        Ok(self.lock().demoi.iter().find(|d| d.slug == slug).cloned())
    }

    async fn update_criteria(&self, id: DemosId, criteria: FranchiseCriteria) -> Result<()> {
        let mut db = self.lock();
        let d = db
            .demoi
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or(StoreError::NotFound)?;
        d.criteria = criteria;
        self.flush(&db)
    }

    async fn set_allows_nsfw(&self, id: DemosId, allows_nsfw: bool) -> Result<()> {
        let mut db = self.lock();
        let d = db
            .demoi
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or(StoreError::NotFound)?;
        d.allows_nsfw = allows_nsfw;
        self.flush(&db)
    }

    async fn set_jury_sizing(&self, id: DemosId, sizing: JurySizing) -> Result<()> {
        let mut db = self.lock();
        let d = db
            .demoi
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or(StoreError::NotFound)?;
        d.jury_sizing = sizing;
        self.flush(&db)
    }

    async fn set_vote_weighting(&self, id: DemosId, scheme: VoteWeighting) -> Result<()> {
        let mut db = self.lock();
        let d = db
            .demoi
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or(StoreError::NotFound)?;
        d.vote_weighting = scheme;
        self.flush(&db)
    }

    async fn set_weighting_scope(&self, id: DemosId, scope: WeightingScope) -> Result<()> {
        let mut db = self.lock();
        let d = db
            .demoi
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or(StoreError::NotFound)?;
        d.weighting_scope = scope;
        self.flush(&db)
    }

    async fn set_posting_policy(&self, id: DemosId, policy: PostingPolicy) -> Result<()> {
        let mut db = self.lock();
        let d = db
            .demoi
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or(StoreError::NotFound)?;
        d.posting_policy = policy;
        self.flush(&db)
    }

    async fn list(&self) -> Result<Vec<Demos>> {
        Ok(self.lock().demoi.clone())
    }
}

#[async_trait]
impl FoundingStore for TextFileStore {
    async fn create(
        &self,
        slug: &str,
        name: &str,
        founder: UserId,
        created_at: Timestamp,
    ) -> Result<FoundingPetition> {
        let mut db = self.lock();
        db.next_founding += 1;
        let petition = FoundingPetition {
            id: FoundingId(db.next_founding),
            slug: slug.to_string(),
            name: name.to_string(),
            founder,
            sign_offs: Vec::new(),
            created_at,
        };
        db.foundings.push(petition.clone());
        self.flush(&db)?;
        Ok(petition)
    }

    async fn get(&self, id: FoundingId) -> Result<Option<FoundingPetition>> {
        Ok(self.lock().foundings.iter().find(|f| f.id == id).cloned())
    }

    async fn sign(&self, id: FoundingId, user: UserId) -> Result<FoundingPetition> {
        let mut db = self.lock();
        let petition = db
            .foundings
            .iter_mut()
            .find(|f| f.id == id)
            .ok_or(StoreError::NotFound)?;
        if user != petition.founder && !petition.sign_offs.contains(&user) {
            petition.sign_offs.push(user);
        }
        let updated = petition.clone();
        self.flush(&db)?;
        Ok(updated)
    }

    async fn delete(&self, id: FoundingId) -> Result<()> {
        let mut db = self.lock();
        db.foundings.retain(|f| f.id != id);
        self.flush(&db)
    }

    async fn list(&self) -> Result<Vec<FoundingPetition>> {
        let mut foundings = self.lock().foundings.clone();
        foundings.reverse();
        Ok(foundings)
    }
}

#[async_trait]
impl MembershipStore for TextFileStore {
    async fn upsert(&self, membership: Membership) -> Result<()> {
        let mut db = self.lock();
        if let Some(slot) = db
            .memberships
            .iter_mut()
            .find(|m| m.user_id == membership.user_id && m.demos_id == membership.demos_id)
        {
            *slot = membership;
        } else {
            db.memberships.push(membership);
        }
        self.flush(&db)
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
impl ProposalStore for TextFileStore {
    async fn create(
        &self,
        demos: DemosId,
        proposer: UserId,
        kind: ProposalKind,
        opened_at: Timestamp,
        closes_at: Timestamp,
    ) -> Result<Proposal> {
        let mut db = self.lock();
        db.next_proposal += 1;
        let p = Proposal::new(
            ProposalId(db.next_proposal),
            demos,
            proposer,
            kind,
            opened_at,
            closes_at,
        );
        db.proposals.push(p.clone());
        self.flush(&db)?;
        Ok(p)
    }

    async fn get(&self, id: ProposalId) -> Result<Option<Proposal>> {
        Ok(self.lock().proposals.iter().find(|p| p.id == id).cloned())
    }

    async fn update(&self, proposal: &Proposal) -> Result<()> {
        let mut db = self.lock();
        let slot = db
            .proposals
            .iter_mut()
            .find(|p| p.id == proposal.id)
            .ok_or(StoreError::NotFound)?;
        *slot = proposal.clone();
        self.flush(&db)
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
impl VoteStore for TextFileStore {
    async fn cast(
        &self,
        proposal: ProposalId,
        voter: UserId,
        aye: bool,
        weight: u64,
        _at: Timestamp,
    ) -> Result<()> {
        let mut db = self.lock();
        if db
            .votes
            .iter()
            .any(|v| v.proposal == proposal && v.voter == voter)
        {
            return Err(StoreError::AlreadyVoted);
        }
        db.votes.push(VoteRec {
            proposal,
            voter,
            aye,
            weight,
        });
        self.flush(&db)
    }

    async fn has_voted(&self, proposal: ProposalId, voter: UserId) -> Result<bool> {
        Ok(self
            .lock()
            .votes
            .iter()
            .any(|v| v.proposal == proposal && v.voter == voter))
    }

    async fn tally(&self, proposal: ProposalId) -> Result<Tally> {
        let db = self.lock();
        let mut tally = Tally::default();
        for v in db.votes.iter().filter(|v| v.proposal == proposal) {
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
impl PostVoteStore for TextFileStore {
    async fn set(&self, post: PostId, user: UserId, dir: Option<bool>) -> Result<()> {
        let mut db = self.lock();
        db.post_votes
            .retain(|v| !(v.post == post && v.user == user));
        if let Some(up) = dir {
            db.post_votes.push(PostVoteRec { post, user, up });
        }
        self.flush(&db)
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
impl RuleStore for TextFileStore {
    async fn create(&self, demos: DemosId, text: &str, at: Timestamp) -> Result<Rule> {
        let mut db = self.lock();
        db.next_rule += 1;
        let rule = Rule::new(RuleId(db.next_rule), demos, text, at);
        db.rules.push(rule.clone());
        self.flush(&db)?;
        Ok(rule)
    }

    async fn get(&self, id: RuleId) -> Result<Option<Rule>> {
        Ok(self.lock().rules.iter().find(|r| r.id == id).cloned())
    }

    async fn set_active(&self, id: RuleId, active: bool) -> Result<()> {
        let mut db = self.lock();
        let r = db
            .rules
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or(StoreError::NotFound)?;
        r.active = active;
        self.flush(&db)
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
impl PostStore for TextFileStore {
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
        let mut db = self.lock();
        db.next_post += 1;
        let post = Post::new(
            PostId(db.next_post),
            demos,
            author,
            title,
            body,
            media,
            tags,
            at,
        );
        db.posts.push(post.clone());
        self.flush(&db)?;
        Ok(post)
    }

    async fn get(&self, id: PostId) -> Result<Option<Post>> {
        Ok(self.lock().posts.iter().find(|p| p.id == id).cloned())
    }

    async fn set_removed(&self, id: PostId, removed: bool) -> Result<()> {
        let mut db = self.lock();
        let p = db
            .posts
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or(StoreError::NotFound)?;
        p.removed = removed;
        self.flush(&db)
    }

    async fn set_is_nsfw(&self, id: PostId, is_nsfw: bool) -> Result<()> {
        let mut db = self.lock();
        let p = db
            .posts
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or(StoreError::NotFound)?;
        p.is_nsfw = is_nsfw;
        self.flush(&db)
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
        let db = self.lock();
        let distinct: HashSet<DemosId> = db
            .posts
            .iter()
            .filter(|p| p.author == author)
            .map(|p| p.demos_id)
            .collect();
        Ok(distinct.len() as u64)
    }
}

#[async_trait]
impl CommentStore for TextFileStore {
    async fn create(
        &self,
        post: PostId,
        author: UserId,
        parent: Option<CommentId>,
        body: &str,
        at: Timestamp,
    ) -> Result<Comment> {
        let mut db = self.lock();
        db.next_comment += 1;
        let comment = Comment::new(CommentId(db.next_comment), post, author, parent, body, at);
        db.comments.push(comment.clone());
        self.flush(&db)?;
        Ok(comment)
    }

    async fn get(&self, id: CommentId) -> Result<Option<Comment>> {
        Ok(self.lock().comments.iter().find(|c| c.id == id).cloned())
    }

    async fn set_removed(&self, id: CommentId, removed: bool) -> Result<()> {
        let mut db = self.lock();
        let c = db
            .comments
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or(StoreError::NotFound)?;
        c.removed = removed;
        self.flush(&db)
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
impl CommentVoteStore for TextFileStore {
    async fn set(&self, comment: CommentId, user: UserId, dir: Option<bool>) -> Result<()> {
        let mut db = self.lock();
        db.comment_votes
            .retain(|v| !(v.comment == comment && v.user == user));
        if let Some(up) = dir {
            db.comment_votes.push(CommentVoteRec { comment, user, up });
        }
        self.flush(&db)
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
impl ReportStore for TextFileStore {
    async fn create(
        &self,
        demos: DemosId,
        reporter: Option<UserId>,
        target: ReportTarget,
        reason: ReportReason,
        note: &str,
        at: Timestamp,
    ) -> Result<Report> {
        let mut db = self.lock();
        db.next_report += 1;
        let report = Report::new(
            ReportId(db.next_report),
            demos,
            reporter,
            target,
            reason,
            note,
            at,
        );
        db.reports.push(report.clone());
        self.flush(&db)?;
        Ok(report)
    }

    async fn get(&self, id: ReportId) -> Result<Option<Report>> {
        Ok(self.lock().reports.iter().find(|r| r.id == id).cloned())
    }

    async fn update(&self, report: &Report) -> Result<()> {
        let mut db = self.lock();
        let slot = db
            .reports
            .iter_mut()
            .find(|r| r.id == report.id)
            .ok_or(StoreError::NotFound)?;
        *slot = report.clone();
        self.flush(&db)
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
impl TrialStore for TextFileStore {
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
        let mut db = self.lock();
        db.next_trial += 1;
        let trial = Trial::new(
            TrialId(db.next_trial),
            demos,
            report,
            accused,
            jurors,
            jury_weight,
            juror_weights,
            opened_at,
            closes_at,
        );
        db.trials.push(trial.clone());
        self.flush(&db)?;
        Ok(trial)
    }

    async fn get(&self, id: TrialId) -> Result<Option<Trial>> {
        Ok(self.lock().trials.iter().find(|t| t.id == id).cloned())
    }

    async fn update(&self, trial: &Trial) -> Result<()> {
        let mut db = self.lock();
        let slot = db
            .trials
            .iter_mut()
            .find(|t| t.id == trial.id)
            .ok_or(StoreError::NotFound)?;
        *slot = trial.clone();
        self.flush(&db)
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
        let mut db = self.lock();
        if db
            .jury_ballots
            .iter()
            .any(|b| b.trial == trial && b.juror == juror)
        {
            return Err(StoreError::AlreadyVoted);
        }
        db.jury_ballots.push(JuryBallotRec {
            trial,
            juror,
            guilty,
            weight,
        });
        self.flush(&db)
    }

    async fn has_voted(&self, trial: TrialId, juror: UserId) -> Result<bool> {
        Ok(self
            .lock()
            .jury_ballots
            .iter()
            .any(|b| b.trial == trial && b.juror == juror))
    }

    async fn ballot_tally(&self, trial: TrialId) -> Result<(u64, u64)> {
        let db = self.lock();
        let mut guilty = 0u64;
        let mut not_guilty = 0u64;
        for b in db.jury_ballots.iter().filter(|b| b.trial == trial) {
            if b.guilty {
                guilty += b.weight;
            } else {
                not_guilty += b.weight;
            }
        }
        Ok((guilty, not_guilty))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn persists_and_reloads_from_disk() {
        let path =
            std::env::temp_dir().join(format!("democratos-test-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);

        // `create` lives on several store traits, so direct calls on the concrete
        // type are disambiguated with fully-qualified syntax. Via `Arc<dyn ...>`
        // in the composition root this is never needed.
        let user_id = {
            let store = TextFileStore::open(&path).unwrap();
            let u = UserStore::create(&store, "alice", None, None, Timestamp(0))
                .await
                .unwrap();
            DemosStore::create(&store, "rust", "Rustaceans", u.id, Timestamp(0))
                .await
                .unwrap();
            u.id
        };

        // Re-open from the same file: data survives the process boundary.
        let reopened = TextFileStore::open(&path).unwrap();
        let u = UserStore::get(&reopened, user_id).await.unwrap();
        assert_eq!(u.unwrap().handle, "alice");
        let d = DemosStore::by_slug(&reopened, "rust").await.unwrap();
        assert_eq!(d.unwrap().founder_id, user_id);

        std::fs::remove_file(&path).ok();
    }
}
