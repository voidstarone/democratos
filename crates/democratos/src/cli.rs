//! The composition root's command-line interface.

use clap::Parser;

use crate::media_kind::MediaKind;
use crate::notifier_kind::NotifierKind;
use crate::sanitizer_kind::SanitizerKind;
use crate::scan_policy::ScanPolicy;
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

    /// How uploaded media is made safe before it is stored: `reencode` decodes
    /// and re-encodes images (stripping metadata, defusing decompression bombs and
    /// polyglots) and structurally validates video; `passthrough` only type-checks
    /// (lighter CPU for a tiny box, weaker guarantee). Applies to every media
    /// backend and to the in-RAM `memory` store.
    #[arg(
        long,
        value_enum,
        default_value = "reencode",
        global = true,
        env = "DEMOCRATOS_MEDIA_SANITIZER"
    )]
    pub(crate) media_sanitizer: SanitizerKind,

    /// Scan uploaded media for known illegal content (CSAM) before storing it.
    /// **Off by default**, because effective matching needs a curated known-bad
    /// hash corpus from a lawful source (NCMEC / PhotoDNA), which not every
    /// operator can obtain — without one the scan can only clear everything, a
    /// false sense of protection. Turn it on *together with* `--csam-hash-file`
    /// (or after wiring an external classifier) once you have a real source.
    /// Malicious-media sanitization runs regardless of this flag.
    #[arg(
        long,
        default_value_t = false,
        global = true,
        env = "DEMOCRATOS_CSAM_SCAN"
    )]
    pub(crate) csam_scan: bool,

    /// Path to the operator-curated known-bad hash corpus the CSAM scan matches
    /// against (lines of `sha256:<hex>` / `dhash:<hex>`; see docs/media-safety.md).
    /// Derived from a lawful source (e.g. NCMEC / PhotoDNA). Contains only opaque
    /// hashes, never imagery.
    #[arg(long, global = true, env = "DEMOCRATOS_CSAM_HASH_FILE")]
    pub(crate) csam_hash_file: Option<String>,

    /// What to do when the CSAM scanner cannot render a verdict (backend down or
    /// unreachable): `fail-closed` refuses the upload (default), `quarantine`
    /// refuses and preserves a copy for review, `allow` serves it unscanned. A
    /// positive match is always blocked and preserved, whatever this is.
    #[arg(
        long,
        value_enum,
        default_value = "fail-closed",
        global = true,
        env = "DEMOCRATOS_MEDIA_SCAN_POLICY"
    )]
    pub(crate) media_scan_policy: ScanPolicy,

    /// Directory where blocked or held media is preserved (a CSAM match, or an
    /// unscannable upload under the `quarantine` policy). Kept out of the public
    /// store; must be reachable only by trusted operators. Preserved, never
    /// deleted, so it can back a NCMEC report (18 U.S.C. §2258A).
    #[arg(
        long,
        default_value = "quarantine",
        global = true,
        env = "DEMOCRATOS_QUARANTINE_DIR"
    )]
    pub(crate) quarantine_dir: String,

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

    /// Start with invitation-only access on: open registration is closed and
    /// accounts are created only through an approved invite. This is the *initial*
    /// value only — it seeds the toggle the first time; after that the persisted
    /// setting (flipped live from the review queue) wins. Off by default.
    #[arg(long, global = true, env = "DEMOCRATOS_INVITE_ONLY")]
    pub(crate) invite_only: bool,

    /// The public origin this node is reached at (`scheme://host[:port]`, no
    /// trailing slash), used to build the absolute invite-accept links that go in
    /// approval emails. Set it to the address invitees actually use (behind a
    /// reverse proxy this is NOT the bind address).
    #[arg(
        long,
        default_value = "http://localhost:8080",
        global = true,
        env = "DEMOCRATOS_PUBLIC_BASE_URL"
    )]
    pub(crate) public_base_url: String,

    /// How many days an issued invite link stays valid before it must be
    /// re-approved.
    #[arg(
        long,
        default_value_t = 7,
        global = true,
        env = "DEMOCRATOS_INVITE_TOKEN_TTL_DAYS"
    )]
    pub(crate) invite_token_ttl_days: i64,

    /// Comma-separated CIDR allowlist for the admin invite review queue, e.g.
    /// `192.168.1.0/24,10.0.0.0/8`. A request from outside every listed network
    /// (and not loopback) gets a 404. Loopback is always allowed.
    #[arg(long, global = true, env = "DEMOCRATOS_ADMIN_SUBNET")]
    pub(crate) admin_subnet: Option<String>,

    /// Shared secret the admin review queue requires as `?key=` on top of the
    /// subnet check. With no secret set the queue is disabled entirely (always
    /// 404) — so it is never reachable by subnet membership alone.
    #[arg(long, global = true, env = "DEMOCRATOS_ADMIN_SECRET")]
    pub(crate) admin_secret: Option<String>,

    /// How invite-approval emails are delivered: `log` (print the link — dev / no
    /// SMTP) or `smtp` (send real mail; requires the `--smtp-*` settings).
    #[arg(
        long,
        value_enum,
        default_value = "log",
        global = true,
        env = "DEMOCRATOS_NOTIFIER"
    )]
    pub(crate) notifier: NotifierKind,

    /// SMTP server hostname (for `--notifier smtp`).
    #[arg(long, global = true, env = "DEMOCRATOS_SMTP_HOST")]
    pub(crate) smtp_host: Option<String>,

    /// SMTP server port. 465 for implicit TLS, 587 for STARTTLS.
    #[arg(long, default_value_t = 465, global = true, env = "DEMOCRATOS_SMTP_PORT")]
    pub(crate) smtp_port: u16,

    /// SMTP auth username.
    #[arg(long, global = true, env = "DEMOCRATOS_SMTP_USERNAME")]
    pub(crate) smtp_username: Option<String>,

    /// SMTP auth password.
    #[arg(long, global = true, env = "DEMOCRATOS_SMTP_PASSWORD")]
    pub(crate) smtp_password: Option<String>,

    /// The `From:` address for approval emails, e.g.
    /// `Democratos <no-reply@example.org>`.
    #[arg(long, global = true, env = "DEMOCRATOS_SMTP_FROM")]
    pub(crate) smtp_from: Option<String>,

    /// Use STARTTLS (port 587) instead of implicit TLS (port 465). Either way the
    /// SMTP session is encrypted.
    #[arg(long, default_value_t = false, global = true, env = "DEMOCRATOS_SMTP_STARTTLS")]
    pub(crate) smtp_starttls: bool,

    /// Subject line for the invite-approval email.
    #[arg(
        long,
        default_value = "Your Democratos invite",
        global = true,
        env = "DEMOCRATOS_SMTP_SUBJECT"
    )]
    pub(crate) smtp_subject: String,

    #[command(subcommand)]
    pub(crate) command: Top,
}
