//! Assemble the `Services` wiring from the chosen storage backend.

use std::sync::Arc;

use anyhow::Result;

use adapter_media_s3::S3Config;
use adapter_moderation_local::{AutoApproveAgeVerifier, HeuristicNsfwScanner};
use adapter_recommend_disk::DiskRecommender;
use adapter_recommend_memory::MemoryRecommender;
use adapter_store_memory::MemoryStore;
use adapter_store_postgres::{PgStoreConfig, PostgresStore};
use adapter_store_textfile::TextFileStore;
use app::{Notifier, Services};
use domain::NodeId;

use crate::build_media_guard::build_media_guard;
use crate::build_media_store::build_media_store;
use crate::media_guard_config::MediaGuardConfig;
use crate::media_kind::MediaKind;
use crate::store_kind::StoreKind;
use crate::system_clock::SystemClock;

// The composition root's wiring surface: many independent knobs by nature.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn build_services(
    kind: StoreKind,
    data: &str,
    database_url: Option<&str>,
    node_id: u16,
    pg_config: PgStoreConfig,
    media_kind: MediaKind,
    media_dir: &str,
    s3: Option<S3Config>,
    guard: MediaGuardConfig,
    recommend_index: Option<&str>,
    requires_age_verification: bool,
    require_signatures: bool,
    notifier: Arc<dyn Notifier>,
    public_base_url: String,
    invite_token_ttl_days: i64,
) -> Result<(Services, Option<Arc<PostgresStore>>)> {
    let clock = Arc::new(SystemClock);
    // The only AgeVerifier this build wires is the dev auto-approve stub, which
    // approves *everyone*. If a deployment turns the age gate on, that stub makes
    // it inert — NSFW content is blurred but never actually gated, a false sense
    // of compliance where verification is legally required. Warn loudly so the
    // misconfiguration can't pass silently; wire a real AgeVerifier before relying
    // on the gate.
    if requires_age_verification {
        eprintln!(
            "⚠ age verification is ENABLED but this build only ships the auto-approve dev \
             stub — every user is treated as verified, so the age gate does NOT restrict \
             anyone. Wire a real AgeVerifier before depending on it for compliance."
        );
    }
    // The federation runtime needs the concrete Postgres handle (outbox/apply),
    // which the trait-object `Services` erases — capture it here for the caller.
    let mut fed_store: Option<Arc<PostgresStore>> = None;
    let services = match kind {
        StoreKind::Memory => {
            let store = Arc::new(MemoryStore::new());
            Services {
                users: store.clone(),
                demoi: store.clone(),
                foundings: store.clone(),
                memberships: store.clone(),
                proposals: store.clone(),
                votes: store.clone(),
                rules: store.clone(),
                posts: store.clone(),
                comments: store.clone(),
                reports: store.clone(),
                invites: store.clone(),
                settings: store.clone(),
                sensitive_cases: store.clone(),
                trials: store.clone(),
                notifications: store.clone(),
                trial_comments: store.clone(),
                post_votes: store.clone(),
                comment_votes: store.clone(),
                // The in-RAM store always hosts media itself; guard it too so a
                // dev/`--store memory` node gets the same sanitize + scan pipeline.
                media: build_media_guard(store, &guard)?,
                recommender: Arc::new(MemoryRecommender::default()),
                nsfw_scanner: Arc::new(HeuristicNsfwScanner),
                age_verifier: Arc::new(AutoApproveAgeVerifier),
                requires_age_verification,
                require_signatures,
                notifier: notifier.clone(),
                public_base_url: public_base_url.clone(),
                invite_token_ttl_days,
                clock,
            }
        }
        StoreKind::File => {
            let store = Arc::new(TextFileStore::open(data)?);
            let media = build_media_store(media_kind, media_dir, s3).await?;
            // Wrap the backend in the sanitize + CSAM-scan + quarantine pipeline,
            // unless this node hosts no media at all.
            let media = if matches!(media_kind, MediaKind::None) {
                media
            } else {
                build_media_guard(media, &guard)?
            };
            Services {
                users: store.clone(),
                demoi: store.clone(),
                foundings: store.clone(),
                memberships: store.clone(),
                proposals: store.clone(),
                votes: store.clone(),
                rules: store.clone(),
                posts: store.clone(),
                comments: store.clone(),
                reports: store.clone(),
                invites: store.clone(),
                settings: store.clone(),
                sensitive_cases: store.clone(),
                trials: store.clone(),
                notifications: store.clone(),
                trial_comments: store.clone(),
                post_votes: store.clone(),
                comment_votes: store.clone(),
                media,
                // Disk-backed: the file arm keeps only a small offset table in
                // RAM, not the whole similarity map. Index path defaults beside
                // the data file but is overridable (point it at tmpfs on flash
                // storage). The memory arm above stays fully in-process.
                recommender: Arc::new(DiskRecommender::open(
                    recommend_index
                        .map(String::from)
                        .unwrap_or_else(|| format!("{data}.recindex")),
                )),
                nsfw_scanner: Arc::new(HeuristicNsfwScanner),
                age_verifier: Arc::new(AutoApproveAgeVerifier),
                requires_age_verification,
                require_signatures,
                notifier: notifier.clone(),
                public_base_url: public_base_url.clone(),
                invite_token_ttl_days,
                clock,
            }
        }
        StoreKind::Postgres => {
            let url = database_url.ok_or_else(|| {
                anyhow::anyhow!(
                    "--database-url (env DATABASE_URL) is required for --store postgres"
                )
            })?;
            // Every *Store port is served by one shared Postgres connection pool;
            // IDs are minted under this node's id so a federated network never
            // collides. Media comes from the selected backend (`--media s3` for a
            // shared bucket so any node serves any upload).
            let store =
                Arc::new(PostgresStore::connect_with(url, NodeId(node_id), pg_config).await?);
            let media = build_media_store(media_kind, media_dir, s3).await?;
            // Wrap the backend in the sanitize + CSAM-scan + quarantine pipeline,
            // unless this node hosts no media at all.
            let media = if matches!(media_kind, MediaKind::None) {
                media
            } else {
                build_media_guard(media, &guard)?
            };
            fed_store = Some(store.clone());
            Services {
                users: store.clone(),
                demoi: store.clone(),
                foundings: store.clone(),
                memberships: store.clone(),
                proposals: store.clone(),
                votes: store.clone(),
                rules: store.clone(),
                posts: store.clone(),
                comments: store.clone(),
                reports: store.clone(),
                invites: store.clone(),
                settings: store.clone(),
                sensitive_cases: store.clone(),
                trials: store.clone(),
                notifications: store.clone(),
                trial_comments: store.clone(),
                post_votes: store.clone(),
                comment_votes: store.clone(),
                media,
                // The recommender model is per-node, derived, and rebuilt at boot,
                // so an in-process index is fine — no cross-node sharing needed.
                recommender: Arc::new(MemoryRecommender::default()),
                nsfw_scanner: Arc::new(HeuristicNsfwScanner),
                age_verifier: Arc::new(AutoApproveAgeVerifier),
                requires_age_verification,
                require_signatures,
                notifier: notifier.clone(),
                public_base_url: public_base_url.clone(),
                invite_token_ttl_days,
                clock,
            }
        }
    };
    Ok((services, fed_store))
}
