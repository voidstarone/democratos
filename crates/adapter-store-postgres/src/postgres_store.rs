//! The `PostgresStore` type and its implementations of every `*Store` port.

use async_trait::async_trait;
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::types::Json;
use sqlx::{Executor, Postgres, Transaction};

use app::{Result, StoreError};
use app::{
    CommentStore, CommentVoteStore, DemosStore, FoundingStore, MembershipStore, PostStore,
    PostVoteStore, ProposalStore, ReportStore, RuleStore, TrialStore, UserStore, VoteStore,
};
use domain::{
    compose_id, Comment, CommentId, Demos, DemosId, FeedPaging, FoundingId, FoundingPetition,
    FranchiseCriteria, JurySizing, Media, Membership, NodeId, Post, PostId, PostingPolicy,
    Proposal, ProposalId, ProposalKind, Report, ReportId, ReportReason, ReportStatus, ReportTarget,
    Rule, RuleId, Tally, Tier, Timestamp, Trial, TrialId, User, UserId, Verdict, VoteWeighting,
    WeightingScope,
};

use crate::is_insecure_url::is_insecure_url;
use crate::PgStoreConfig;

pub(crate) fn store_err(e: impl std::fmt::Display) -> StoreError {
    StoreError::Store(e.to_string())
}

/// Hard safety cap on rows a single unbounded (no-argument) list query may
/// materialize into RAM. These port methods have no pagination in their trait
/// signature, so without a ceiling one call could pull an entire table into
/// memory — a memory-DoS vector. This is only a backstop; true keyset pagination
/// (carrying a cursor through the port) is the proper follow-up.
const MAX_ROWS: i64 = 10_000;

/// The stable string form of a tier, lifted into its own column for
/// `voter_count` / `admitted_since`.
pub(crate) fn tier_str(tier: Tier) -> &'static str {
    match tier {
        Tier::Lurker => "Lurker",
        Tier::Member => "Member",
        Tier::Voter => "Voter",
    }
}

/// The stable string form of a verdict, lifted out for `trials.list_open`.
pub(crate) fn verdict_str(v: Verdict) -> &'static str {
    match v {
        Verdict::Pending => "Pending",
        Verdict::Guilty => "Guilty",
        Verdict::NotGuilty => "NotGuilty",
    }
}

/// A shared-database store over PostgreSQL.
#[derive(Clone)]
pub struct PostgresStore {
    pub(crate) pool: PgPool,
    pub(crate) node: NodeId,
}

impl PostgresStore {
    /// Connect to `database_url` with default pool settings, run embedded
    /// migrations, and mint IDs under `node`. All app replicas that share this
    /// node's database pass the same `node` id (it identifies the *node*, not
    /// the process).
    pub async fn connect(database_url: &str, node: NodeId) -> Result<Self> {
        Self::connect_with(database_url, node, PgStoreConfig::default()).await
    }

    /// Like [`connect`](Self::connect), but with explicit pool tuning. Warns to
    /// stderr if `database_url` connects to a remote host without TLS.
    pub async fn connect_with(
        database_url: &str,
        node: NodeId,
        config: PgStoreConfig,
    ) -> Result<Self> {
        if is_insecure_url(database_url) {
            // Cleartext to a remote database exposes (and lets a network attacker
            // tamper with) every replicated row, vote, and credential in flight.
            // Fail closed rather than warn-and-continue; an operator on a trusted
            // private network can opt back in explicitly.
            if std::env::var("DEMOCRATOS_ALLOW_INSECURE_DB").is_ok() {
                eprintln!(
                    "⚠ postgres: connecting to a remote database WITHOUT TLS — allowed via \
                     DEMOCRATOS_ALLOW_INSECURE_DB. Replicated rows, votes, and credentials \
                     cross the network in the clear."
                );
            } else {
                return Err(StoreError::Store(
                    "refusing to connect to a remote Postgres over plaintext: replicated rows, \
                     votes, and credentials would cross the network unencrypted. Add \
                     `?sslmode=require` (or verify-full) to DATABASE_URL, or set \
                     DEMOCRATOS_ALLOW_INSECURE_DB=1 to override on a trusted private network."
                        .into(),
                ));
            }
        }
        let timeout_ms = config.statement_timeout_ms;
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .after_connect(move |conn, _meta| {
                Box::pin(async move {
                    // Applied per-connection so every query the pool hands out is
                    // bounded. `SET` (not `SET LOCAL`) so it outlives the setup txn.
                    let stmt = format!("SET statement_timeout = {timeout_ms}");
                    conn.execute(stmt.as_str()).await?;
                    Ok(())
                })
            })
            .connect(database_url)
            .await
            .map_err(store_err)?;
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(store_err)?;
        Ok(Self { pool, node })
    }

    /// Wrap an existing pool (used by the federation/outbox layer, which shares
    /// the same connection pool). Assumes migrations have already run.
    pub fn from_pool(pool: PgPool, node: NodeId) -> Self {
        Self { pool, node }
    }

    /// The underlying pool, so higher layers (outbox, change feed) can share it.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Allocate the next global id for `kind`, atomically within `tx`: bump this
    /// node's per-kind counter and compose it with the node id.
    async fn alloc(&self, tx: &mut Transaction<'_, Postgres>, kind: &str) -> Result<u64> {
        let seq: i64 = sqlx::query_scalar(
            "INSERT INTO id_counters (kind, next) VALUES ($1, 1)
             ON CONFLICT (kind) DO UPDATE SET next = id_counters.next + 1
             RETURNING next",
        )
        .bind(kind)
        .fetch_one(&mut **tx)
        .await
        .map_err(store_err)?;
        Ok(compose_id(self.node, seq as u64))
    }
}

// --- users ------------------------------------------------------------------

#[async_trait]
impl UserStore for PostgresStore {
    async fn create(
        &self,
        handle: &str,
        email: Option<&str>,
        password_hash: Option<&str>,
        created_at: Timestamp,
    ) -> Result<User> {
        let mut tx = self.pool.begin().await.map_err(store_err)?;
        let id = self.alloc(&mut tx, "user").await?;
        let mut user = User::new(UserId(id), handle, created_at);
        user.email = email.map(str::to_string);
        user.password_hash = password_hash.map(str::to_string);
        // Credentials ride inside the JSONB `data` document alongside the rest
        // of the account; only the query keys (id, handle, created_at) are
        // promoted to typed columns.
        sqlx::query("INSERT INTO users (id, handle, created_at, data) VALUES ($1, $2, $3, $4)")
            .bind(id as i64)
            .bind(handle)
            .bind(created_at.0)
            .bind(Json(&user))
            .execute(&mut *tx)
            .await
            .map_err(store_err)?;
        tx.commit().await.map_err(store_err)?;
        Ok(user)
    }

    async fn get(&self, id: UserId) -> Result<Option<User>> {
        let row: Option<(Json<User>,)> = sqlx::query_as("SELECT data FROM users WHERE id = $1")
            .bind(id.0 as i64)
            .fetch_optional(&self.pool)
            .await
            .map_err(store_err)?;
        Ok(row.map(|(j,)| j.0))
    }

    async fn by_handle(&self, handle: &str) -> Result<Option<User>> {
        let row: Option<(Json<User>,)> = sqlx::query_as("SELECT data FROM users WHERE handle = $1")
            .bind(handle)
            .fetch_optional(&self.pool)
            .await
            .map_err(store_err)?;
        Ok(row.map(|(j,)| j.0))
    }

    async fn by_email(&self, email: &str) -> Result<Option<User>> {
        // Email lives in the JSONB document, not a typed column; match it there.
        let row: Option<(Json<User>,)> =
            sqlx::query_as("SELECT data FROM users WHERE data->>'email' = $1")
                .bind(email)
                .fetch_optional(&self.pool)
                .await
                .map_err(store_err)?;
        Ok(row.map(|(j,)| j.0))
    }

    async fn list(&self) -> Result<Vec<User>> {
        // Capped read (memory-DoS backstop); keyset pagination is the follow-up.
        let rows: Vec<(Json<User>,)> = sqlx::query_as(
            "SELECT data FROM users ORDER BY created_at, id LIMIT $1",
        )
        .bind(MAX_ROWS)
        .fetch_all(&self.pool)
        .await
        .map_err(store_err)?;
        Ok(rows.into_iter().map(|(j,)| j.0).collect())
    }

    async fn set_is_age_verified(&self, id: UserId, is_verified: bool) -> Result<()> {
        // Mutate inside the JSONB document, keeping the row's typed columns intact.
        let n = sqlx::query(
            "UPDATE users SET data = jsonb_set(data, '{is_age_verified}', to_jsonb($2::bool)) \
             WHERE id = $1",
        )
        .bind(id.0 as i64)
        .bind(is_verified)
        .execute(&self.pool)
        .await
        .map_err(store_err)?
        .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    async fn set_public_key(&self, id: UserId, public_key_hex: &str) -> Result<()> {
        // Store the key inside the JSONB document, alongside the other account
        // fields. Note the credential-redaction outbox trigger strips only
        // password_hash/email — the public key is safe (and useful) to replicate.
        let n = sqlx::query(
            "UPDATE users SET data = jsonb_set(data, '{public_key}', to_jsonb($2::text)) \
             WHERE id = $1",
        )
        .bind(id.0 as i64)
        .bind(public_key_hex)
        .execute(&self.pool)
        .await
        .map_err(store_err)?
        .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    async fn set_franchise_barred(&self, id: UserId, barred: bool) -> Result<()> {
        // Stored inside the JSONB document like the other account fields. It is
        // safe (and useful) to replicate: peers must agree an account is barred.
        let n = sqlx::query(
            "UPDATE users SET data = jsonb_set(data, '{is_franchise_barred}', to_jsonb($2::bool)) \
             WHERE id = $1",
        )
        .bind(id.0 as i64)
        .bind(barred)
        .execute(&self.pool)
        .await
        .map_err(store_err)?
        .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    async fn set_feed_paging(&self, id: UserId, paging: FeedPaging) -> Result<()> {
        // Store the enum by its serde tag ("auto"/"pages"/"lazy") so it round-trips
        // straight back into `FeedPaging` when the document is deserialized.
        let repr = serde_json::to_value(paging)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_else(|| "auto".to_string());
        let n = sqlx::query(
            "UPDATE users SET data = jsonb_set(data, '{feed_paging}', to_jsonb($2::text)) \
             WHERE id = $1",
        )
        .bind(id.0 as i64)
        .bind(repr)
        .execute(&self.pool)
        .await
        .map_err(store_err)?
        .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }
}

// --- demoi ------------------------------------------------------------------

impl PostgresStore {
    /// Fetch a demos by an arbitrary indexed column, then persist an in-place
    /// edit of the reconstructed struct. Centralizes the read-modify-write the
    /// `set_*` methods share.
    async fn update_demos(&self, id: DemosId, edit: impl FnOnce(&mut Demos)) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(store_err)?;
        let row: Option<(Json<Demos>,)> =
            sqlx::query_as("SELECT data FROM demoi WHERE id = $1 FOR UPDATE")
                .bind(id.0 as i64)
                .fetch_optional(&mut *tx)
                .await
                .map_err(store_err)?;
        let mut demos = row.ok_or(StoreError::NotFound)?.0 .0;
        edit(&mut demos);
        sqlx::query("UPDATE demoi SET data = $2 WHERE id = $1")
            .bind(id.0 as i64)
            .bind(Json(&demos))
            .execute(&mut *tx)
            .await
            .map_err(store_err)?;
        tx.commit().await.map_err(store_err)?;
        Ok(())
    }

    /// A zero-row optimistic-concurrency `UPDATE` means one of two things: the row
    /// is gone, or its `rev` moved (a concurrent writer won). Probe `id` to tell
    /// them apart so the caller gets [`StoreError::NotFound`] vs [`StoreError::Conflict`].
    /// `table` is always a caller-supplied string literal — never user input.
    async fn conflict_or_not_found(&self, table: &str, id: i64) -> StoreError {
        let sql = format!("SELECT 1 FROM {table} WHERE id = $1");
        match sqlx::query(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
        {
            Ok(Some(_)) => StoreError::Conflict,
            Ok(None) => StoreError::NotFound,
            Err(e) => store_err(e),
        }
    }
}

#[async_trait]
impl DemosStore for PostgresStore {
    async fn create(
        &self,
        slug: &str,
        name: &str,
        founder: UserId,
        created_at: Timestamp,
    ) -> Result<Demos> {
        let mut tx = self.pool.begin().await.map_err(store_err)?;
        let id = self.alloc(&mut tx, "demos").await?;
        let demos = Demos::new(DemosId(id), slug, name, founder, created_at);
        sqlx::query("INSERT INTO demoi (id, slug, created_at, data) VALUES ($1, $2, $3, $4)")
            .bind(id as i64)
            .bind(slug)
            .bind(created_at.0)
            .bind(Json(&demos))
            .execute(&mut *tx)
            .await
            .map_err(store_err)?;
        tx.commit().await.map_err(store_err)?;
        Ok(demos)
    }

    async fn get(&self, id: DemosId) -> Result<Option<Demos>> {
        let row: Option<(Json<Demos>,)> = sqlx::query_as("SELECT data FROM demoi WHERE id = $1")
            .bind(id.0 as i64)
            .fetch_optional(&self.pool)
            .await
            .map_err(store_err)?;
        Ok(row.map(|(j,)| j.0))
    }

    async fn by_slug(&self, slug: &str) -> Result<Option<Demos>> {
        let row: Option<(Json<Demos>,)> = sqlx::query_as("SELECT data FROM demoi WHERE slug = $1")
            .bind(slug)
            .fetch_optional(&self.pool)
            .await
            .map_err(store_err)?;
        Ok(row.map(|(j,)| j.0))
    }

    async fn update_criteria(&self, id: DemosId, criteria: FranchiseCriteria) -> Result<()> {
        self.update_demos(id, |d| d.criteria = criteria).await
    }

    async fn set_allows_nsfw(&self, id: DemosId, allows_nsfw: bool) -> Result<()> {
        self.update_demos(id, |d| d.allows_nsfw = allows_nsfw).await
    }

    async fn set_jury_sizing(&self, id: DemosId, sizing: JurySizing) -> Result<()> {
        self.update_demos(id, |d| d.jury_sizing = sizing).await
    }

    async fn set_vote_weighting(&self, id: DemosId, scheme: VoteWeighting) -> Result<()> {
        self.update_demos(id, |d| d.vote_weighting = scheme).await
    }

    async fn set_weighting_scope(&self, id: DemosId, scope: WeightingScope) -> Result<()> {
        self.update_demos(id, |d| d.weighting_scope = scope).await
    }

    async fn set_posting_policy(&self, id: DemosId, policy: PostingPolicy) -> Result<()> {
        self.update_demos(id, |d| d.posting_policy = policy).await
    }

    async fn list(&self) -> Result<Vec<Demos>> {
        // Capped read (memory-DoS backstop); keyset pagination is the follow-up.
        let rows: Vec<(Json<Demos>,)> = sqlx::query_as(
            "SELECT data FROM demoi ORDER BY created_at, id LIMIT $1",
        )
        .bind(MAX_ROWS)
        .fetch_all(&self.pool)
        .await
        .map_err(store_err)?;
        Ok(rows.into_iter().map(|(j,)| j.0).collect())
    }
}

// --- foundings --------------------------------------------------------------

impl PostgresStore {
    /// The petition's co-signers, in sign order (never the founder). Shared by
    /// every read path that reassembles a [`FoundingPetition`].
    async fn founding_sign_offs(&self, id: i64) -> Result<Vec<UserId>> {
        let rows: Vec<(i64,)> = sqlx::query_as(
            "SELECT user_id FROM founding_sign_offs WHERE founding_id = $1 ORDER BY position",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(store_err)?;
        Ok(rows.into_iter().map(|(u,)| UserId(u as u64)).collect())
    }
}

#[async_trait]
impl FoundingStore for PostgresStore {
    async fn create(
        &self,
        slug: &str,
        name: &str,
        founder: UserId,
        created_at: Timestamp,
    ) -> Result<FoundingPetition> {
        let mut tx = self.pool.begin().await.map_err(store_err)?;
        let id = self.alloc(&mut tx, "founding").await?;
        sqlx::query(
            "INSERT INTO foundings (id, slug, name, founder, created_at) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id as i64)
        .bind(slug)
        .bind(name)
        .bind(founder.0 as i64)
        .bind(created_at.0)
        .execute(&mut *tx)
        .await
        .map_err(store_err)?;
        tx.commit().await.map_err(store_err)?;
        Ok(FoundingPetition {
            id: FoundingId(id),
            slug: slug.to_string(),
            name: name.to_string(),
            founder,
            // A fresh petition has only the founder's intent; nobody has signed yet.
            sign_offs: Vec::new(),
            created_at,
        })
    }

    async fn get(&self, id: FoundingId) -> Result<Option<FoundingPetition>> {
        let row: Option<(i64, String, String, i64, i64)> = sqlx::query_as(
            "SELECT id, slug, name, founder, created_at FROM foundings WHERE id = $1",
        )
        .bind(id.0 as i64)
        .fetch_optional(&self.pool)
        .await
        .map_err(store_err)?;
        let Some((rid, slug, name, founder, created_at)) = row else {
            return Ok(None);
        };
        let sign_offs = self.founding_sign_offs(rid).await?;
        Ok(Some(FoundingPetition {
            id: FoundingId(rid as u64),
            slug,
            name,
            founder: UserId(founder as u64),
            sign_offs,
            created_at: Timestamp(created_at),
        }))
    }

    async fn sign(&self, id: FoundingId, user: UserId) -> Result<FoundingPetition> {
        // Idempotent co-sign: append the sign-off only if the petition exists, the
        // signer is not the founder, and they have not already signed. `position`
        // is the next ordinal so sign order is preserved; ON CONFLICT makes a
        // repeat a no-op. The founder guard (`founder <> $2`) also means a missing
        // petition inserts nothing — the re-read below then reports NotFound.
        sqlx::query(
            "INSERT INTO founding_sign_offs (founding_id, user_id, position)
             SELECT $1, $2,
                    COALESCE((SELECT MAX(position) FROM founding_sign_offs WHERE founding_id = $1), -1) + 1
             WHERE EXISTS (SELECT 1 FROM foundings WHERE id = $1 AND founder <> $2)
             ON CONFLICT (founding_id, user_id) DO NOTHING",
        )
        .bind(id.0 as i64)
        .bind(user.0 as i64)
        .execute(&self.pool)
        .await
        .map_err(store_err)?;
        FoundingStore::get(self, id)
            .await?
            .ok_or(StoreError::NotFound)
    }

    async fn delete(&self, id: FoundingId) -> Result<()> {
        // Sign-offs cascade via the FK's ON DELETE CASCADE.
        sqlx::query("DELETE FROM foundings WHERE id = $1")
            .bind(id.0 as i64)
            .execute(&self.pool)
            .await
            .map_err(store_err)?;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<FoundingPetition>> {
        let rows: Vec<(i64, String, String, i64, i64)> = sqlx::query_as(
            "SELECT id, slug, name, founder, created_at FROM foundings ORDER BY created_at DESC, id DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(store_err)?;
        let mut out = Vec::with_capacity(rows.len());
        for (rid, slug, name, founder, created_at) in rows {
            let sign_offs = self.founding_sign_offs(rid).await?;
            out.push(FoundingPetition {
                id: FoundingId(rid as u64),
                slug,
                name,
                founder: UserId(founder as u64),
                sign_offs,
                created_at: Timestamp(created_at),
            });
        }
        Ok(out)
    }
}

// --- memberships ------------------------------------------------------------

#[async_trait]
impl MembershipStore for PostgresStore {
    async fn upsert(&self, m: Membership) -> Result<()> {
        let enfranchised_at = m.enfranchised_at.map(|t| t.0);
        sqlx::query(
            "INSERT INTO memberships (user_id, demos_id, tier, enfranchised_at, data)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (user_id, demos_id)
             DO UPDATE SET tier = EXCLUDED.tier,
                           enfranchised_at = EXCLUDED.enfranchised_at,
                           data = EXCLUDED.data",
        )
        .bind(m.user_id.0 as i64)
        .bind(m.demos_id.0 as i64)
        .bind(tier_str(m.tier))
        .bind(enfranchised_at)
        .bind(Json(&m))
        .execute(&self.pool)
        .await
        .map_err(store_err)?;
        Ok(())
    }

    async fn get(&self, user: UserId, demos: DemosId) -> Result<Option<Membership>> {
        let row: Option<(Json<Membership>,)> =
            sqlx::query_as("SELECT data FROM memberships WHERE user_id = $1 AND demos_id = $2")
                .bind(user.0 as i64)
                .bind(demos.0 as i64)
                .fetch_optional(&self.pool)
                .await
                .map_err(store_err)?;
        Ok(row.map(|(j,)| j.0))
    }

    async fn members(&self, demos: DemosId) -> Result<Vec<Membership>> {
        // Capped read (memory-DoS backstop); keyset pagination is the follow-up.
        let rows: Vec<(Json<Membership>,)> = sqlx::query_as(
            "SELECT data FROM memberships WHERE demos_id = $1 LIMIT $2",
        )
        .bind(demos.0 as i64)
        .bind(MAX_ROWS)
        .fetch_all(&self.pool)
        .await
        .map_err(store_err)?;
        Ok(rows.into_iter().map(|(j,)| j.0).collect())
    }

    async fn list_for_user(&self, user: UserId) -> Result<Vec<Membership>> {
        // Capped read (memory-DoS backstop); keyset pagination is the follow-up.
        let rows: Vec<(Json<Membership>,)> = sqlx::query_as(
            "SELECT data FROM memberships WHERE user_id = $1 LIMIT $2",
        )
        .bind(user.0 as i64)
        .bind(MAX_ROWS)
        .fetch_all(&self.pool)
        .await
        .map_err(store_err)?;
        Ok(rows.into_iter().map(|(j,)| j.0).collect())
    }

    async fn voter_count(&self, demos: DemosId) -> Result<u64> {
        let n: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM memberships WHERE demos_id = $1 AND tier = 'Voter'",
        )
        .bind(demos.0 as i64)
        .fetch_one(&self.pool)
        .await
        .map_err(store_err)?;
        Ok(n as u64)
    }

    async fn admitted_since(&self, demos: DemosId, since: Timestamp) -> Result<u64> {
        let n: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM memberships
             WHERE demos_id = $1 AND tier = 'Voter'
               AND enfranchised_at IS NOT NULL AND enfranchised_at >= $2",
        )
        .bind(demos.0 as i64)
        .bind(since.0)
        .fetch_one(&self.pool)
        .await
        .map_err(store_err)?;
        Ok(n as u64)
    }
}

// --- proposals --------------------------------------------------------------

#[async_trait]
impl ProposalStore for PostgresStore {
    async fn create(
        &self,
        demos: DemosId,
        proposer: UserId,
        kind: ProposalKind,
        opened_at: Timestamp,
        closes_at: Timestamp,
    ) -> Result<Proposal> {
        let mut tx = self.pool.begin().await.map_err(store_err)?;
        let id = self.alloc(&mut tx, "proposal").await?;
        let p = Proposal::new(ProposalId(id), demos, proposer, kind, opened_at, closes_at);
        sqlx::query("INSERT INTO proposals (id, demos_id, data) VALUES ($1, $2, $3)")
            .bind(id as i64)
            .bind(demos.0 as i64)
            .bind(Json(&p))
            .execute(&mut *tx)
            .await
            .map_err(store_err)?;
        tx.commit().await.map_err(store_err)?;
        Ok(p)
    }

    async fn get(&self, id: ProposalId) -> Result<Option<Proposal>> {
        let row: Option<(Json<Proposal>,)> =
            sqlx::query_as("SELECT data FROM proposals WHERE id = $1")
                .bind(id.0 as i64)
                .fetch_optional(&self.pool)
                .await
                .map_err(store_err)?;
        Ok(row.map(|(j,)| j.0))
    }

    async fn update(&self, proposal: &Proposal) -> Result<()> {
        // Optimistic concurrency: only land if the row is still at the revision we
        // read, then bump it. Guards against two replicas' read-modify-write
        // silently losing one update. COALESCE treats a pre-`rev` row as revision 0.
        let expected = proposal.rev as i64;
        let mut next = proposal.clone();
        next.rev = proposal.rev.saturating_add(1);
        let n = sqlx::query(
            "UPDATE proposals SET data = $2 \
             WHERE id = $1 AND COALESCE((data->>'rev')::bigint, 0) = $3",
        )
        .bind(proposal.id.0 as i64)
        .bind(Json(&next))
        .bind(expected)
        .execute(&self.pool)
        .await
        .map_err(store_err)?
        .rows_affected();
        if n == 0 {
            return Err(self
                .conflict_or_not_found("proposals", proposal.id.0 as i64)
                .await);
        }
        Ok(())
    }

    async fn list(&self, demos: DemosId) -> Result<Vec<Proposal>> {
        let rows: Vec<(Json<Proposal>,)> =
            sqlx::query_as("SELECT data FROM proposals WHERE demos_id = $1 ORDER BY id")
                .bind(demos.0 as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(store_err)?;
        Ok(rows.into_iter().map(|(j,)| j.0).collect())
    }
}

// --- proposal votes ---------------------------------------------------------

#[async_trait]
impl VoteStore for PostgresStore {
    async fn cast(
        &self,
        proposal: ProposalId,
        voter: UserId,
        aye: bool,
        weight: u64,
        _at: Timestamp,
    ) -> Result<()> {
        // ON CONFLICT DO NOTHING + rows_affected distinguishes a fresh ballot
        // from a repeat without a separate SELECT (one round trip, race-free).
        let n = sqlx::query(
            "INSERT INTO votes (proposal_id, voter_id, aye, weight) VALUES ($1, $2, $3, $4)
             ON CONFLICT (proposal_id, voter_id) DO NOTHING",
        )
        .bind(proposal.0 as i64)
        .bind(voter.0 as i64)
        .bind(aye)
        .bind(weight as i64)
        .execute(&self.pool)
        .await
        .map_err(store_err)?
        .rows_affected();
        if n == 0 {
            return Err(StoreError::AlreadyVoted);
        }
        Ok(())
    }

    async fn has_voted(&self, proposal: ProposalId, voter: UserId) -> Result<bool> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM votes WHERE proposal_id = $1 AND voter_id = $2)",
        )
        .bind(proposal.0 as i64)
        .bind(voter.0 as i64)
        .fetch_one(&self.pool)
        .await
        .map_err(store_err)?;
        Ok(exists)
    }

    async fn tally(&self, proposal: ProposalId) -> Result<Tally> {
        let (aye, nay): (i64, i64) = sqlx::query_as(
            "SELECT
               COALESCE(SUM(weight) FILTER (WHERE aye), 0)::BIGINT,
               COALESCE(SUM(weight) FILTER (WHERE NOT aye), 0)::BIGINT
             FROM votes WHERE proposal_id = $1",
        )
        .bind(proposal.0 as i64)
        .fetch_one(&self.pool)
        .await
        .map_err(store_err)?;
        Ok(Tally {
            aye: aye as u64,
            nay: nay as u64,
        })
    }
}

// --- post votes -------------------------------------------------------------

#[async_trait]
impl PostVoteStore for PostgresStore {
    async fn set(&self, post: PostId, user: UserId, dir: Option<bool>) -> Result<()> {
        match dir {
            Some(up) => {
                sqlx::query(
                    "INSERT INTO post_votes (post_id, user_id, up) VALUES ($1, $2, $3)
                     ON CONFLICT (post_id, user_id) DO UPDATE SET up = EXCLUDED.up",
                )
                .bind(post.0 as i64)
                .bind(user.0 as i64)
                .bind(up)
                .execute(&self.pool)
                .await
                .map_err(store_err)?;
            }
            None => {
                sqlx::query("DELETE FROM post_votes WHERE post_id = $1 AND user_id = $2")
                    .bind(post.0 as i64)
                    .bind(user.0 as i64)
                    .execute(&self.pool)
                    .await
                    .map_err(store_err)?;
            }
        }
        Ok(())
    }

    async fn get(&self, post: PostId, user: UserId) -> Result<Option<bool>> {
        let row: Option<(bool,)> =
            sqlx::query_as("SELECT up FROM post_votes WHERE post_id = $1 AND user_id = $2")
                .bind(post.0 as i64)
                .bind(user.0 as i64)
                .fetch_optional(&self.pool)
                .await
                .map_err(store_err)?;
        Ok(row.map(|(up,)| up))
    }

    async fn score(&self, post: PostId) -> Result<i64> {
        let score: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(CASE WHEN up THEN 1 ELSE -1 END), 0)::BIGINT
             FROM post_votes WHERE post_id = $1",
        )
        .bind(post.0 as i64)
        .fetch_one(&self.pool)
        .await
        .map_err(store_err)?;
        Ok(score)
    }

    async fn vote_count(&self) -> Result<u64> {
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM post_votes")
            .fetch_one(&self.pool)
            .await
            .map_err(store_err)?;
        Ok(n as u64)
    }

    async fn all_votes(&self) -> Result<Vec<(PostId, UserId, bool)>> {
        // Capped read (memory-DoS backstop); keyset pagination is the follow-up.
        let rows: Vec<(i64, i64, bool)> = sqlx::query_as(
            "SELECT post_id, user_id, up FROM post_votes LIMIT $1",
        )
        .bind(MAX_ROWS)
        .fetch_all(&self.pool)
        .await
        .map_err(store_err)?;
        Ok(rows
            .into_iter()
            .map(|(p, u, up)| (PostId(p as u64), UserId(u as u64), up))
            .collect())
    }

    async fn liked_by(&self, user: UserId) -> Result<Vec<PostId>> {
        let rows: Vec<(i64,)> =
            sqlx::query_as("SELECT post_id FROM post_votes WHERE user_id = $1 AND up")
                .bind(user.0 as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(store_err)?;
        Ok(rows.into_iter().map(|(p,)| PostId(p as u64)).collect())
    }

    async fn voted_by(&self, user: UserId) -> Result<Vec<PostId>> {
        let rows: Vec<(i64,)> = sqlx::query_as("SELECT post_id FROM post_votes WHERE user_id = $1")
            .bind(user.0 as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(store_err)?;
        Ok(rows.into_iter().map(|(p,)| PostId(p as u64)).collect())
    }
}

// --- rules ------------------------------------------------------------------

#[async_trait]
impl RuleStore for PostgresStore {
    async fn create(&self, demos: DemosId, text: &str, at: Timestamp) -> Result<Rule> {
        let mut tx = self.pool.begin().await.map_err(store_err)?;
        let id = self.alloc(&mut tx, "rule").await?;
        let rule = Rule::new(RuleId(id), demos, text, at);
        sqlx::query("INSERT INTO rules (id, demos_id, active, data) VALUES ($1, $2, $3, $4)")
            .bind(id as i64)
            .bind(demos.0 as i64)
            .bind(rule.active)
            .bind(Json(&rule))
            .execute(&mut *tx)
            .await
            .map_err(store_err)?;
        tx.commit().await.map_err(store_err)?;
        Ok(rule)
    }

    async fn get(&self, id: RuleId) -> Result<Option<Rule>> {
        let row: Option<(Json<Rule>,)> = sqlx::query_as("SELECT data FROM rules WHERE id = $1")
            .bind(id.0 as i64)
            .fetch_optional(&self.pool)
            .await
            .map_err(store_err)?;
        Ok(row.map(|(j,)| j.0))
    }

    async fn set_active(&self, id: RuleId, active: bool) -> Result<()> {
        let n = sqlx::query(
            "UPDATE rules
             SET active = $2, data = jsonb_set(data, '{active}', to_jsonb($2::bool))
             WHERE id = $1",
        )
        .bind(id.0 as i64)
        .bind(active)
        .execute(&self.pool)
        .await
        .map_err(store_err)?
        .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    async fn list_active(&self, demos: DemosId) -> Result<Vec<Rule>> {
        let rows: Vec<(Json<Rule>,)> =
            sqlx::query_as("SELECT data FROM rules WHERE demos_id = $1 AND active ORDER BY id")
                .bind(demos.0 as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(store_err)?;
        Ok(rows.into_iter().map(|(j,)| j.0).collect())
    }
}

// --- posts ------------------------------------------------------------------

#[async_trait]
impl PostStore for PostgresStore {
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
        let mut tx = self.pool.begin().await.map_err(store_err)?;
        let id = self.alloc(&mut tx, "post").await?;
        let post = Post::new(PostId(id), demos, author, title, body, media, tags, at);
        sqlx::query(
            "INSERT INTO posts (id, demos_id, author, created_at, data) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id as i64)
        .bind(demos.0 as i64)
        .bind(author.0 as i64)
        .bind(at.0)
        .bind(Json(&post))
        .execute(&mut *tx)
        .await
        .map_err(store_err)?;
        tx.commit().await.map_err(store_err)?;
        Ok(post)
    }

    async fn get(&self, id: PostId) -> Result<Option<Post>> {
        let row: Option<(Json<Post>,)> = sqlx::query_as("SELECT data FROM posts WHERE id = $1")
            .bind(id.0 as i64)
            .fetch_optional(&self.pool)
            .await
            .map_err(store_err)?;
        Ok(row.map(|(j,)| j.0))
    }

    async fn set_removed(&self, id: PostId, removed: bool) -> Result<()> {
        let n = sqlx::query(
            "UPDATE posts SET data = jsonb_set(data, '{removed}', to_jsonb($2::bool)) WHERE id = $1",
        )
        .bind(id.0 as i64)
        .bind(removed)
        .execute(&self.pool)
        .await
        .map_err(store_err)?
        .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    async fn set_is_nsfw(&self, id: PostId, is_nsfw: bool) -> Result<()> {
        let n = sqlx::query(
            "UPDATE posts SET data = jsonb_set(data, '{is_nsfw}', to_jsonb($2::bool)) WHERE id = $1",
        )
        .bind(id.0 as i64)
        .bind(is_nsfw)
        .execute(&self.pool)
        .await
        .map_err(store_err)?
        .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    async fn list(&self, demos: DemosId) -> Result<Vec<Post>> {
        let rows: Vec<(Json<Post>,)> =
            sqlx::query_as("SELECT data FROM posts WHERE demos_id = $1 ORDER BY id")
                .bind(demos.0 as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(store_err)?;
        Ok(rows.into_iter().map(|(j,)| j.0).collect())
    }

    async fn list_by_author(&self, demos: DemosId, author: UserId) -> Result<Vec<Post>> {
        let rows: Vec<(Json<Post>,)> = sqlx::query_as(
            "SELECT data FROM posts WHERE demos_id = $1 AND author = $2 ORDER BY id",
        )
        .bind(demos.0 as i64)
        .bind(author.0 as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(store_err)?;
        Ok(rows.into_iter().map(|(j,)| j.0).collect())
    }

    async fn list_all(&self) -> Result<Vec<Post>> {
        // Capped read (memory-DoS backstop); keyset pagination is the follow-up.
        let rows: Vec<(Json<Post>,)> =
            sqlx::query_as("SELECT data FROM posts ORDER BY id LIMIT $1")
                .bind(MAX_ROWS)
                .fetch_all(&self.pool)
                .await
                .map_err(store_err)?;
        Ok(rows.into_iter().map(|(j,)| j.0).collect())
    }

    async fn distinct_demos_by_author(&self, author: UserId) -> Result<u64> {
        let n: i64 =
            sqlx::query_scalar("SELECT COUNT(DISTINCT demos_id) FROM posts WHERE author = $1")
                .bind(author.0 as i64)
                .fetch_one(&self.pool)
                .await
                .map_err(store_err)?;
        Ok(n as u64)
    }
}

// --- comments ---------------------------------------------------------------

#[async_trait]
impl CommentStore for PostgresStore {
    async fn create(
        &self,
        post: PostId,
        author: UserId,
        parent: Option<CommentId>,
        body: &str,
        at: Timestamp,
    ) -> Result<Comment> {
        let mut tx = self.pool.begin().await.map_err(store_err)?;
        let id = self.alloc(&mut tx, "comment").await?;
        let comment = Comment::new(CommentId(id), post, author, parent, body, at);
        sqlx::query(
            "INSERT INTO comments (id, post_id, author, created_at, data) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id as i64)
        .bind(post.0 as i64)
        .bind(author.0 as i64)
        .bind(at.0)
        .bind(Json(&comment))
        .execute(&mut *tx)
        .await
        .map_err(store_err)?;
        tx.commit().await.map_err(store_err)?;
        Ok(comment)
    }

    async fn get(&self, id: CommentId) -> Result<Option<Comment>> {
        let row: Option<(Json<Comment>,)> =
            sqlx::query_as("SELECT data FROM comments WHERE id = $1")
                .bind(id.0 as i64)
                .fetch_optional(&self.pool)
                .await
                .map_err(store_err)?;
        Ok(row.map(|(j,)| j.0))
    }

    async fn set_removed(&self, id: CommentId, removed: bool) -> Result<()> {
        let n = sqlx::query(
            "UPDATE comments SET data = jsonb_set(data, '{removed}', to_jsonb($2::bool)) WHERE id = $1",
        )
        .bind(id.0 as i64)
        .bind(removed)
        .execute(&self.pool)
        .await
        .map_err(store_err)?
        .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    async fn list_for_post(&self, post: PostId) -> Result<Vec<Comment>> {
        // Capped read (memory-DoS backstop); keyset pagination is the follow-up.
        let rows: Vec<(Json<Comment>,)> = sqlx::query_as(
            "SELECT data FROM comments WHERE post_id = $1 ORDER BY id LIMIT $2",
        )
        .bind(post.0 as i64)
        .bind(MAX_ROWS)
        .fetch_all(&self.pool)
        .await
        .map_err(store_err)?;
        Ok(rows.into_iter().map(|(j,)| j.0).collect())
    }

    async fn count_by_author_since(&self, author: UserId, since: Timestamp) -> Result<u64> {
        let n: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM comments WHERE author = $1 AND created_at >= $2",
        )
        .bind(author.0 as i64)
        .bind(since.0)
        .fetch_one(&self.pool)
        .await
        .map_err(store_err)?;
        Ok(n as u64)
    }

    async fn list_by_author(&self, author: UserId) -> Result<Vec<Comment>> {
        let rows: Vec<(Json<Comment>,)> =
            sqlx::query_as("SELECT data FROM comments WHERE author = $1 ORDER BY created_at, id")
                .bind(author.0 as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(store_err)?;
        Ok(rows.into_iter().map(|(j,)| j.0).collect())
    }
}

#[async_trait]
impl CommentVoteStore for PostgresStore {
    async fn set(&self, comment: CommentId, user: UserId, dir: Option<bool>) -> Result<()> {
        match dir {
            Some(up) => {
                sqlx::query(
                    "INSERT INTO comment_votes (comment_id, user_id, up) VALUES ($1, $2, $3)
                     ON CONFLICT (comment_id, user_id) DO UPDATE SET up = EXCLUDED.up",
                )
                .bind(comment.0 as i64)
                .bind(user.0 as i64)
                .bind(up)
                .execute(&self.pool)
                .await
                .map_err(store_err)?;
            }
            None => {
                sqlx::query("DELETE FROM comment_votes WHERE comment_id = $1 AND user_id = $2")
                    .bind(comment.0 as i64)
                    .bind(user.0 as i64)
                    .execute(&self.pool)
                    .await
                    .map_err(store_err)?;
            }
        }
        Ok(())
    }

    async fn get(&self, comment: CommentId, user: UserId) -> Result<Option<bool>> {
        let row: Option<(bool,)> =
            sqlx::query_as("SELECT up FROM comment_votes WHERE comment_id = $1 AND user_id = $2")
                .bind(comment.0 as i64)
                .bind(user.0 as i64)
                .fetch_optional(&self.pool)
                .await
                .map_err(store_err)?;
        Ok(row.map(|(up,)| up))
    }

    async fn score(&self, comment: CommentId) -> Result<i64> {
        let score: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(CASE WHEN up THEN 1 ELSE -1 END), 0)::BIGINT
             FROM comment_votes WHERE comment_id = $1",
        )
        .bind(comment.0 as i64)
        .fetch_one(&self.pool)
        .await
        .map_err(store_err)?;
        Ok(score)
    }
}

// --- reports ----------------------------------------------------------------

/// Whether a report is in its open (still-gathering-flags) state — lifted into
/// the `is_open` column so `list_open` is a plain indexed filter.
pub(crate) fn report_is_open(status: &ReportStatus) -> bool {
    matches!(status, ReportStatus::Open)
}

#[async_trait]
impl ReportStore for PostgresStore {
    async fn create(
        &self,
        demos: DemosId,
        reporter: Option<UserId>,
        target: ReportTarget,
        reason: ReportReason,
        note: &str,
        at: Timestamp,
    ) -> Result<Report> {
        let mut tx = self.pool.begin().await.map_err(store_err)?;
        let id = self.alloc(&mut tx, "report").await?;
        let report = Report::new(ReportId(id), demos, reporter, target, reason, note, at);
        sqlx::query("INSERT INTO reports (id, demos_id, is_open, data) VALUES ($1, $2, $3, $4)")
            .bind(id as i64)
            .bind(demos.0 as i64)
            .bind(report_is_open(&report.status))
            .bind(Json(&report))
            .execute(&mut *tx)
            .await
            .map_err(store_err)?;
        tx.commit().await.map_err(store_err)?;
        Ok(report)
    }

    async fn get(&self, id: ReportId) -> Result<Option<Report>> {
        let row: Option<(Json<Report>,)> = sqlx::query_as("SELECT data FROM reports WHERE id = $1")
            .bind(id.0 as i64)
            .fetch_optional(&self.pool)
            .await
            .map_err(store_err)?;
        Ok(row.map(|(j,)| j.0))
    }

    async fn update(&self, report: &Report) -> Result<()> {
        // Optimistic concurrency (see `ProposalStore::update`): CAS on `rev`, bump.
        let expected = report.rev as i64;
        let mut next = report.clone();
        next.rev = report.rev.saturating_add(1);
        let n = sqlx::query(
            "UPDATE reports SET is_open = $2, data = $3 \
             WHERE id = $1 AND COALESCE((data->>'rev')::bigint, 0) = $4",
        )
        .bind(report.id.0 as i64)
        .bind(report_is_open(&next.status))
        .bind(Json(&next))
        .bind(expected)
        .execute(&self.pool)
        .await
        .map_err(store_err)?
        .rows_affected();
        if n == 0 {
            return Err(self
                .conflict_or_not_found("reports", report.id.0 as i64)
                .await);
        }
        Ok(())
    }

    async fn list_open(&self, demos: DemosId) -> Result<Vec<Report>> {
        let rows: Vec<(Json<Report>,)> =
            sqlx::query_as("SELECT data FROM reports WHERE demos_id = $1 AND is_open ORDER BY id")
                .bind(demos.0 as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(store_err)?;
        Ok(rows.into_iter().map(|(j,)| j.0).collect())
    }
}

// --- trials -----------------------------------------------------------------

#[async_trait]
impl TrialStore for PostgresStore {
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
        let mut tx = self.pool.begin().await.map_err(store_err)?;
        let id = self.alloc(&mut tx, "trial").await?;
        let trial = Trial::new(
            TrialId(id),
            demos,
            report,
            accused,
            jurors,
            jury_weight,
            juror_weights,
            opened_at,
            closes_at,
        );
        sqlx::query("INSERT INTO trials (id, demos_id, verdict, data) VALUES ($1, $2, $3, $4)")
            .bind(id as i64)
            .bind(demos.0 as i64)
            .bind(verdict_str(trial.verdict))
            .bind(Json(&trial))
            .execute(&mut *tx)
            .await
            .map_err(store_err)?;
        tx.commit().await.map_err(store_err)?;
        Ok(trial)
    }

    async fn get(&self, id: TrialId) -> Result<Option<Trial>> {
        let row: Option<(Json<Trial>,)> = sqlx::query_as("SELECT data FROM trials WHERE id = $1")
            .bind(id.0 as i64)
            .fetch_optional(&self.pool)
            .await
            .map_err(store_err)?;
        Ok(row.map(|(j,)| j.0))
    }

    async fn update(&self, trial: &Trial) -> Result<()> {
        // Optimistic concurrency (see `ProposalStore::update`): CAS on `rev`, bump.
        let expected = trial.rev as i64;
        let mut next = trial.clone();
        next.rev = trial.rev.saturating_add(1);
        let n = sqlx::query(
            "UPDATE trials SET verdict = $2, data = $3 \
             WHERE id = $1 AND COALESCE((data->>'rev')::bigint, 0) = $4",
        )
        .bind(trial.id.0 as i64)
        .bind(verdict_str(next.verdict))
        .bind(Json(&next))
        .bind(expected)
        .execute(&self.pool)
        .await
        .map_err(store_err)?
        .rows_affected();
        if n == 0 {
            return Err(self
                .conflict_or_not_found("trials", trial.id.0 as i64)
                .await);
        }
        Ok(())
    }

    async fn list_open(&self, demos: DemosId) -> Result<Vec<Trial>> {
        let rows: Vec<(Json<Trial>,)> = sqlx::query_as(
            "SELECT data FROM trials WHERE demos_id = $1 AND verdict = 'Pending' ORDER BY id",
        )
        .bind(demos.0 as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(store_err)?;
        Ok(rows.into_iter().map(|(j,)| j.0).collect())
    }

    async fn cast_ballot(
        &self,
        trial: TrialId,
        juror: UserId,
        guilty: bool,
        weight: u64,
    ) -> Result<()> {
        let n = sqlx::query(
            "INSERT INTO jury_ballots (trial_id, juror_id, guilty, weight) VALUES ($1, $2, $3, $4)
             ON CONFLICT (trial_id, juror_id) DO NOTHING",
        )
        .bind(trial.0 as i64)
        .bind(juror.0 as i64)
        .bind(guilty)
        .bind(weight as i64)
        .execute(&self.pool)
        .await
        .map_err(store_err)?
        .rows_affected();
        if n == 0 {
            return Err(StoreError::AlreadyVoted);
        }
        Ok(())
    }

    async fn has_voted(&self, trial: TrialId, juror: UserId) -> Result<bool> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM jury_ballots WHERE trial_id = $1 AND juror_id = $2)",
        )
        .bind(trial.0 as i64)
        .bind(juror.0 as i64)
        .fetch_one(&self.pool)
        .await
        .map_err(store_err)?;
        Ok(exists)
    }

    async fn ballot_tally(&self, trial: TrialId) -> Result<(u64, u64)> {
        let (guilty, not_guilty): (i64, i64) = sqlx::query_as(
            "SELECT
               COALESCE(SUM(weight) FILTER (WHERE guilty), 0)::BIGINT,
               COALESCE(SUM(weight) FILTER (WHERE NOT guilty), 0)::BIGINT
             FROM jury_ballots WHERE trial_id = $1",
        )
        .bind(trial.0 as i64)
        .fetch_one(&self.pool)
        .await
        .map_err(store_err)?;
        Ok((guilty as u64, not_guilty as u64))
    }
}
