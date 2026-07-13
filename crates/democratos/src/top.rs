//! The top-level subcommands.

use clap::Subcommand;

use crate::issuer_command::IssuerCommand;

#[derive(Subcommand)]
pub(crate) enum Top {
    /// Run the web server (the HTTP driving adapter).
    Serve {
        /// Address to bind. In a container bind `0.0.0.0` so the port is
        /// reachable from outside; the default stays loopback for local runs.
        #[arg(long, default_value = "127.0.0.1:5080", env = "DEMOCRATOS_ADDR")]
        addr: String,

        /// How often the background task rebuilds the recommendation model
        /// (seconds). A tick on an unchanged electorate is a cheap version
        /// check, so this is the staleness bound, not a fixed cost — raise it on
        /// a small/low-power box.
        #[arg(long, default_value_t = 60, env = "DEMOCRATOS_REFRESH_SECS")]
        refresh_secs: u64,

        /// Enable the developer account switcher (create/switch test users
        /// in-browser). Never enable this in a real deployment.
        #[arg(long, env = "DEMOCRATOS_DEV")]
        dev: bool,

        /// Require this secret as `?key=` on `GET /dev/unlock` before the dev
        /// account switcher will unlock. With `--dev`, this is what keeps the
        /// switcher yours alone on a node reachable beyond your own machine —
        /// without the secret nobody can obtain the unlock cookie. Omit for a
        /// loopback-only local run (unlock then needs `--dev` alone).
        #[arg(long, env = "DEMOCRATOS_DEV_UNLOCK_SECRET")]
        dev_unlock_secret: Option<String>,

        /// Comma-separated handles of the fixed "puppet" content accounts the dev
        /// switcher may act as. On boot (with `--dev`) each is created if missing
        /// and permanently **franchise-barred**: the switcher only ever toggles
        /// between these, and none can become a voter. e.g.
        /// `--dev-accounts seed-alice,seed-bob,seed-carol`.
        #[arg(long, value_delimiter = ',', env = "DEMOCRATOS_DEV_ACCOUNTS")]
        dev_accounts: Vec<String>,

        /// Mark session/preference cookies `Secure` so browsers only send them
        /// over HTTPS. Enable in any TLS deployment (the normal production case,
        /// e.g. behind the bundled Caddy). Leave off for plain-HTTP local runs,
        /// where `Secure` would stop the cookie being sent at all.
        #[arg(long, env = "DEMOCRATOS_SECURE_COOKIES")]
        secure_cookies: bool,

        /// Secret key used to sign session cookies. Set a long, random value in
        /// production so sessions survive restarts and are valid across every
        /// federated node (all nodes must share it). If omitted, a random key is
        /// generated at boot — secure, but sessions reset on restart and won't
        /// verify on a sibling node.
        #[arg(long, env = "DEMOCRATOS_SESSION_SECRET")]
        session_secret: Option<String>,

        /// Enable federation: serve this node's change feed on this **node-only**
        /// address (firewall it to the node network; keep it off the public one).
        /// Requires `--store postgres`.
        #[arg(long, env = "DEMOCRATOS_FEDERATION_ADDR")]
        federation_addr: Option<String>,

        /// This node's externally-reachable base URL for its federation endpoints,
        /// published to the control plane so peers can discover it (e.g. to forward
        /// account minting here when this node is a trusted issuer). A trusted issuer
        /// must set this to receive delegated sign-ups.
        #[arg(long, env = "DEMOCRATOS_ADVERTISE_URL")]
        advertise_url: Option<String>,

        /// Control-plane etcd endpoints (comma-separated). Empty uses an
        /// in-process registry — correct only for a single node / dev.
        #[arg(long, default_value = "", env = "DEMOCRATOS_ETCD_ENDPOINTS")]
        etcd_endpoints: String,

        /// Shared node-to-node bearer token protecting the feed endpoint.
        #[arg(long, env = "DEMOCRATOS_CLUSTER_TOKEN")]
        cluster_token: Option<String>,

        /// A peer to replicate from, `<node_id>=<base_url>` (repeatable).
        #[arg(long = "peer", value_name = "NODE=URL")]
        peers: Vec<String>,
    },
    /// Run a single command (the CLI driving adapter).
    #[command(subcommand)]
    Cli(adapter_cli::Command),

    /// Manage federation-trusted account issuers (root keygen, certify, publish).
    #[command(subcommand)]
    Issuer(IssuerCommand),

    /// Populate a fresh store with dev fixtures: several communities, users
    /// spanning the whole popularity range, multi-media posts, comments, and
    /// votes. Every account's password is the shared dev password. Refuses to run
    /// if the fixture communities already exist — point `--data` at a new file.
    Seed,
    /// Backfill a single-box `democratos.json` snapshot into this node's
    /// Postgres, preserving IDs. Requires `--store postgres --database-url ...`.
    /// Idempotent: re-running imports only rows not already present.
    Import {
        /// Snapshot to import. Defaults to the global `--data` path.
        #[arg(long)]
        from: Option<String>,
    },
}
