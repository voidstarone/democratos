//! The loadgen subcommands, run in order: seed, vote, verify, read.

use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum Cmd {
    /// Seed a community, an open proposal, and N eligible voters into the owner DB.
    Seed {
        /// Owner node's database URL.
        #[arg(long, env = "LOADGEN_OWNER_DB")]
        owner_db: String,
        /// The owner node's id (so seeded IDs are minted under it).
        #[arg(long, default_value_t = 1)]
        node_id: u16,
        /// How many eligible voters to create.
        #[arg(long, default_value_t = 500)]
        voters: u32,
        /// Community slug (a run-unique suffix is appended).
        #[arg(long, default_value = "loadtest")]
        slug: String,
        /// Where to write the manifest JSON.
        #[arg(long, default_value = "loadgen-manifest.json")]
        out: String,
    },
    /// Drive every seeded voter to cast one ballot across the given node web URLs.
    Vote {
        #[arg(long, default_value = "loadgen-manifest.json")]
        manifest: String,
        /// Comma-separated node **web** base URLs (votes are spread across them).
        #[arg(long, value_delimiter = ',')]
        nodes: Vec<String>,
        /// In-flight requests.
        #[arg(long, default_value_t = 64)]
        concurrency: usize,
    },
    /// Check the authoritative tally and that a replica converges to it.
    Verify {
        #[arg(long, default_value = "loadgen-manifest.json")]
        manifest: String,
        #[arg(long, env = "LOADGEN_OWNER_DB")]
        owner_db: String,
        /// A replica DB expected to converge to the same tally.
        #[arg(long)]
        replica_db: Option<String>,
        /// How long to wait for the replica to converge.
        #[arg(long, default_value_t = 30)]
        timeout_secs: u64,
    },
    /// Hammer a GET path across nodes for a read-throughput measurement.
    Read {
        #[arg(long, value_delimiter = ',')]
        nodes: Vec<String>,
        #[arg(long, default_value = "/")]
        path: String,
        #[arg(long, default_value_t = 2000)]
        requests: u64,
        #[arg(long, default_value_t = 64)]
        concurrency: usize,
    },
}
