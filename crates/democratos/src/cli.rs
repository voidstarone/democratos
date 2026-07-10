//! The composition root's command-line interface.

use clap::Parser;

use crate::media_kind::MediaKind;
use crate::store_kind::StoreKind;
use crate::top::Top;

#[derive(Parser)]
#[command(name = "democratos", about = "Self-governing communities")]
pub(crate) struct Cli {
    /// Which storage adapter to use.
    #[arg(
        long,
        value_enum,
        default_value = "file",
        global = true,
        env = "DEMOCRATOS_STORE"
    )]
    pub(crate) store: StoreKind,

    /// Data file for the `file` store.
    #[arg(
        long,
        default_value = "democratos.json",
        global = true,
        env = "DEMOCRATOS_DATA"
    )]
    pub(crate) data: String,

    /// Connection string for the `postgres` store, e.g.
    /// `postgres://app@db/democratos`.
    #[arg(long, global = true, env = "DATABASE_URL")]
    pub(crate) database_url: Option<String>,

    /// This node's id in the federated network (`0` = single-box / bootstrap).
    /// It stamps the high 16 bits of every id this node mints, so nodes never
    /// collide. All app replicas sharing one database pass the same value.
    #[arg(long, default_value_t = 0, global = true, env = "DEMOCRATOS_NODE_ID")]
    pub(crate) node_id: u16,

    /// Max pooled connections for the `postgres` store.
    #[arg(
        long,
        default_value_t = 16,
        global = true,
        env = "DEMOCRATOS_DB_POOL_SIZE"
    )]
    pub(crate) db_pool_size: u32,

    /// Per-connection `statement_timeout` (ms) for the `postgres` store; a query
    /// running longer is aborted so one bad request can't pin a connection.
    #[arg(
        long,
        default_value_t = 30_000,
        global = true,
        env = "DEMOCRATOS_DB_STATEMENT_TIMEOUT_MS"
    )]
    pub(crate) db_statement_timeout_ms: u64,

    /// Where uploaded media is stored: `local` disk, a shared `s3`/MinIO bucket,
    /// or `none` to opt out of hosting media entirely (uploads refused, media
    /// 404s — for a small/slow box). `s3` is required for a federation so any node
    /// serves any upload. (The `memory` store always keeps media in RAM, ignoring
    /// this.)
    #[arg(
        long,
        value_enum,
        default_value = "local",
        global = true,
        env = "DEMOCRATOS_MEDIA"
    )]
    pub(crate) media: MediaKind,

    /// Directory for uploaded media (used by the `local` media backend). Swap to
    /// `--media s3` for a shared bucket.
    #[arg(
        long,
        default_value = "media",
        global = true,
        env = "DEMOCRATOS_MEDIA_DIR"
    )]
    pub(crate) media_dir: String,

    /// S3/MinIO bucket for the `s3` media backend.
    #[arg(
        long,
        default_value = "democratos-media",
        global = true,
        env = "DEMOCRATOS_S3_BUCKET"
    )]
    pub(crate) s3_bucket: String,

    /// S3 endpoint, e.g. `http://minio:9000` (MinIO) or an AWS regional endpoint.
    #[arg(long, global = true, env = "DEMOCRATOS_S3_ENDPOINT")]
    pub(crate) s3_endpoint: Option<String>,

    /// S3 region label (MinIO ignores it but still requires one).
    #[arg(
        long,
        default_value = "us-east-1",
        global = true,
        env = "DEMOCRATOS_S3_REGION"
    )]
    pub(crate) s3_region: String,

    /// S3 access key id.
    #[arg(long, global = true, env = "AWS_ACCESS_KEY_ID")]
    pub(crate) s3_access_key: Option<String>,

    /// S3 secret access key.
    #[arg(long, global = true, env = "AWS_SECRET_ACCESS_KEY")]
    pub(crate) s3_secret_key: Option<String>,

    /// Use path-style S3 addressing (`endpoint/bucket/key`). Required for MinIO.
    #[arg(
        long,
        default_value_t = true,
        global = true,
        env = "DEMOCRATOS_S3_PATH_STYLE"
    )]
    pub(crate) s3_path_style: bool,

    /// If set, media is served directly from this base URL (public bucket/CDN)
    /// instead of proxied through the app's `/media/:key` route.
    #[arg(long, global = true, env = "DEMOCRATOS_S3_PUBLIC_BASE")]
    pub(crate) s3_public_base: Option<String>,

    /// Where the recommender writes its similarity index (`file` store only).
    /// Defaults to `<data>.recindex`. It is **derived, disposable** data —
    /// rebuilt at boot — so point it at a tmpfs/RAM path to spare flash storage.
    #[arg(long, global = true, env = "DEMOCRATOS_RECOMMEND_INDEX")]
    pub(crate) recommend_index: Option<String>,

    /// Require age verification to view NSFW content. Off by default; turn it on
    /// where the law demands it (e.g. the UK). Most countries leave it off.
    #[arg(long, global = true, env = "DEMOCRATOS_AGE_VERIFICATION")]
    pub(crate) age_verification: bool,

    /// Require every governance action (votes, jury verdicts, post votes) to carry
    /// a valid per-account signature. When on, an account with no enrolled signing
    /// key cannot act until it enrols one. Turn this on before opening the
    /// federation to untrusted nodes: it closes the forgery path where a malicious
    /// node acts for its key-less members. Off by default for single-box rollout.
    #[arg(long, global = true, env = "DEMOCRATOS_REQUIRE_SIGNATURES")]
    pub(crate) require_signatures: bool,

    #[command(subcommand)]
    pub(crate) command: Top,
}
