//! The dataset to import, and the [`PostgresStore::import`] operation over it.

use sqlx::types::Json;
use sqlx::{Postgres, Transaction};

use app::Result;
use domain::{
    local_sequence, origin_node, Comment, Demos, Membership, NodeId, Post, Proposal, Report, Rule,
    Trial, User,
};

use super::import_counts::ImportCounts;
use super::jury_ballot_row::JuryBallotRow;
use super::post_vote_row::PostVoteRow;
use super::vote_row::VoteRow;
use crate::postgres_store::{report_is_open, store_err, tier_str, verdict_str};
use crate::PostgresStore;

/// A complete dataset to load, mirroring the text-file store's on-disk shape.
#[derive(Default)]
pub struct ImportData {
    pub users: Vec<User>,
    pub demoi: Vec<Demos>,
    pub memberships: Vec<Membership>,
    pub proposals: Vec<Proposal>,
    pub votes: Vec<VoteRow>,
    pub post_votes: Vec<PostVoteRow>,
    pub rules: Vec<Rule>,
    pub posts: Vec<Post>,
    pub comments: Vec<Comment>,
    pub reports: Vec<Report>,
    pub trials: Vec<Trial>,
    pub jury_ballots: Vec<JuryBallotRow>,
}

/// Highest local sequence among `ids` that were minted by `node` (0 if none), so
/// the importing node's counter can be advanced past its own imported IDs.
fn max_local_seq(ids: impl IntoIterator<Item = u64>, node: NodeId) -> u64 {
    ids.into_iter()
        .filter(|&id| origin_node(id) == node)
        .map(local_sequence)
        .max()
        .unwrap_or(0)
}

impl PostgresStore {
    /// Load `data` into this node's database, preserving IDs. See the module
    /// docs. Runs migrations must already have happened (they do at `connect`).
    pub async fn import(&self, data: &ImportData) -> Result<ImportCounts> {
        let mut tx = self.pool.begin().await.map_err(store_err)?;
        let mut c = ImportCounts::default();

        // Foreign-key order: users, then communities, then everything scoped to
        // them, then rows that reference posts/trials.
        for u in &data.users {
            c.users += ins(
                &mut tx,
                "INSERT INTO users (id, handle, created_at, data) VALUES ($1,$2,$3,$4)
                 ON CONFLICT (id) DO NOTHING",
                |q| {
                    q.bind(u.id.0 as i64)
                        .bind(&u.handle)
                        .bind(u.created_at.0)
                        .bind(Json(u))
                },
            )
            .await?;
        }
        for d in &data.demoi {
            c.demoi += ins(
                &mut tx,
                "INSERT INTO demoi (id, slug, created_at, data) VALUES ($1,$2,$3,$4)
                 ON CONFLICT (id) DO NOTHING",
                |q| {
                    q.bind(d.id.0 as i64)
                        .bind(&d.slug)
                        .bind(d.created_at.0)
                        .bind(Json(d))
                },
            )
            .await?;
        }
        for m in &data.memberships {
            c.memberships += ins(
                &mut tx,
                "INSERT INTO memberships (user_id, demos_id, tier, enfranchised_at, data)
                 VALUES ($1,$2,$3,$4,$5)
                 ON CONFLICT (user_id, demos_id) DO NOTHING",
                |q| {
                    q.bind(m.user_id.0 as i64)
                        .bind(m.demos_id.0 as i64)
                        .bind(tier_str(m.tier))
                        .bind(m.enfranchised_at.map(|t| t.0))
                        .bind(Json(m))
                },
            )
            .await?;
        }
        for p in &data.proposals {
            c.proposals += ins(
                &mut tx,
                "INSERT INTO proposals (id, demos_id, data) VALUES ($1,$2,$3)
                 ON CONFLICT (id) DO NOTHING",
                |q| {
                    q.bind(p.id.0 as i64)
                        .bind(p.demos_id.0 as i64)
                        .bind(Json(p))
                },
            )
            .await?;
        }
        for v in &data.votes {
            c.votes += ins(
                &mut tx,
                "INSERT INTO votes (proposal_id, voter_id, aye, weight) VALUES ($1,$2,$3,$4)
                 ON CONFLICT (proposal_id, voter_id) DO NOTHING",
                |q| {
                    q.bind(v.proposal as i64)
                        .bind(v.voter as i64)
                        .bind(v.aye)
                        .bind(v.weight as i64)
                },
            )
            .await?;
        }
        for r in &data.rules {
            c.rules += ins(
                &mut tx,
                "INSERT INTO rules (id, demos_id, active, data) VALUES ($1,$2,$3,$4)
                 ON CONFLICT (id) DO NOTHING",
                |q| {
                    q.bind(r.id.0 as i64)
                        .bind(r.demos_id.0 as i64)
                        .bind(r.active)
                        .bind(Json(r))
                },
            )
            .await?;
        }
        for p in &data.posts {
            c.posts += ins(
                &mut tx,
                "INSERT INTO posts (id, demos_id, author, created_at, data) VALUES ($1,$2,$3,$4,$5)
                 ON CONFLICT (id) DO NOTHING",
                |q| {
                    q.bind(p.id.0 as i64)
                        .bind(p.demos_id.0 as i64)
                        .bind(p.author.0 as i64)
                        .bind(p.created_at.0)
                        .bind(Json(p))
                },
            )
            .await?;
        }
        for v in &data.post_votes {
            c.post_votes += ins(
                &mut tx,
                "INSERT INTO post_votes (post_id, user_id, up) VALUES ($1,$2,$3)
                 ON CONFLICT (post_id, user_id) DO NOTHING",
                |q| q.bind(v.post as i64).bind(v.user as i64).bind(v.up),
            )
            .await?;
        }
        for cm in &data.comments {
            c.comments += ins(
                &mut tx,
                "INSERT INTO comments (id, post_id, author, created_at, data) VALUES ($1,$2,$3,$4,$5)
                 ON CONFLICT (id) DO NOTHING",
                |q| {
                    q.bind(cm.id.0 as i64)
                        .bind(cm.post_id.0 as i64)
                        .bind(cm.author.0 as i64)
                        .bind(cm.created_at.0)
                        .bind(Json(cm))
                },
            )
            .await?;
        }
        for r in &data.reports {
            c.reports += ins(
                &mut tx,
                "INSERT INTO reports (id, demos_id, is_open, data) VALUES ($1,$2,$3,$4)
                 ON CONFLICT (id) DO NOTHING",
                |q| {
                    q.bind(r.id.0 as i64)
                        .bind(r.demos_id.0 as i64)
                        .bind(report_is_open(&r.status))
                        .bind(Json(r))
                },
            )
            .await?;
        }
        for t in &data.trials {
            c.trials += ins(
                &mut tx,
                "INSERT INTO trials (id, demos_id, verdict, data) VALUES ($1,$2,$3,$4)
                 ON CONFLICT (id) DO NOTHING",
                |q| {
                    q.bind(t.id.0 as i64)
                        .bind(t.demos_id.0 as i64)
                        .bind(verdict_str(t.verdict))
                        .bind(Json(t))
                },
            )
            .await?;
        }
        for b in &data.jury_ballots {
            c.jury_ballots += ins(
                &mut tx,
                "INSERT INTO jury_ballots (trial_id, juror_id, guilty, weight) VALUES ($1,$2,$3,$4)
                 ON CONFLICT (trial_id, juror_id) DO NOTHING",
                |q| {
                    q.bind(b.trial as i64)
                        .bind(b.juror as i64)
                        .bind(b.guilty)
                        .bind(b.weight as i64)
                },
            )
            .await?;
        }

        // Advance each per-kind counter past the highest local sequence we
        // imported for *this* node, so a later `create` never re-mints one.
        let node = self.node;
        let counters = [
            (
                "user",
                max_local_seq(data.users.iter().map(|u| u.id.0), node),
            ),
            (
                "demos",
                max_local_seq(data.demoi.iter().map(|d| d.id.0), node),
            ),
            (
                "proposal",
                max_local_seq(data.proposals.iter().map(|p| p.id.0), node),
            ),
            (
                "rule",
                max_local_seq(data.rules.iter().map(|r| r.id.0), node),
            ),
            (
                "post",
                max_local_seq(data.posts.iter().map(|p| p.id.0), node),
            ),
            (
                "comment",
                max_local_seq(data.comments.iter().map(|c| c.id.0), node),
            ),
            (
                "report",
                max_local_seq(data.reports.iter().map(|r| r.id.0), node),
            ),
            (
                "trial",
                max_local_seq(data.trials.iter().map(|t| t.id.0), node),
            ),
        ];
        for (kind, seq) in counters {
            if seq == 0 {
                continue;
            }
            sqlx::query(
                "INSERT INTO id_counters (kind, next) VALUES ($1, $2)
                 ON CONFLICT (kind) DO UPDATE SET next = GREATEST(id_counters.next, EXCLUDED.next)",
            )
            .bind(kind)
            .bind(seq as i64)
            .execute(&mut *tx)
            .await
            .map_err(store_err)?;
        }

        tx.commit().await.map_err(store_err)?;
        Ok(c)
    }
}

/// Run one bound `INSERT ... ON CONFLICT DO NOTHING` and report whether it
/// inserted a row (1) or was a no-op (0).
async fn ins<'q, F>(tx: &mut Transaction<'_, Postgres>, sql: &'q str, bind: F) -> Result<u64>
where
    F: FnOnce(
        sqlx::query::Query<'q, Postgres, sqlx::postgres::PgArguments>,
    ) -> sqlx::query::Query<'q, Postgres, sqlx::postgres::PgArguments>,
{
    let q = bind(sqlx::query(sql));
    let n = q
        .execute(&mut **tx)
        .await
        .map_err(store_err)?
        .rows_affected();
    Ok(n)
}
