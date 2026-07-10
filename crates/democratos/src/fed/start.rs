//! Assemble and launch the federation runtime.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};

use adapter_control_etcd::EtcdRegistry;
use adapter_federation::{
    serve_federation, spawn_puller, CommandClient, CommandState, FederatedWrites, FeedClient,
    FeedState, HttpCommandTransport, IngestClient, IngestState, Peer, Replicator, SyncVoteExecutor,
    WriteRouter,
};
use adapter_store_postgres::PostgresStore;
use app::{GovernanceWrites, Services};
use domain::NodeId;
use federation::{InMemoryRegistry, NodeKeypair, OwnershipRegistry};

use crate::fed::claim_hosted::claim_hosted;
use crate::fed::federation_args::FederationArgs;
use crate::fed::guard_federation_exposure::guard_federation_exposure;
use crate::fed::require_tls_for_remote::require_tls_for_remote;
use crate::fed::spawn_maintenance::spawn_maintenance;
use crate::fed::spawn_rehoming::spawn_rehoming;

/// Assemble and launch the federation runtime. Returns once the feed server and
/// puller are spawned; they run in the background for the life of the process.
///
/// `services` backs the command endpoint, so this node can execute writes
/// forwarded to it (authoritatively) for the communities it owns.
pub async fn start(
    store: Arc<PostgresStore>,
    services: Services,
    args: FederationArgs,
) -> Result<Arc<dyn GovernanceWrites>> {
    let node = NodeId(args.node_id);

    // Fail closed before doing any work: an exposed federation port with no token
    // is an open write endpoint for the whole network.
    guard_federation_exposure(&args.federation_addr, args.cluster_token.as_deref())?;

    // A real multi-node cluster — peers to pull from, or an external control plane
    // — must carry a cluster token regardless of the bind address. `bearer_ok`
    // treats an unset token as "open", so without this a misconfigured cluster
    // (empty DEMOCRATOS_CLUSTER_TOKEN) would run with every node-to-node endpoint
    // unauthenticated. Loopback single-node dev, which talks to nobody, still needs none.
    let has_token = args
        .cluster_token
        .as_deref()
        .map(|t| !t.is_empty())
        .unwrap_or(false);
    if !has_token && (!args.peers.is_empty() || !args.etcd_endpoints.is_empty()) {
        bail!(
            "refusing to join a federation cluster without a cluster token: peers and/or an \
             etcd control plane are configured, but the node-to-node feed/command/ingest \
             endpoints authenticate with the shared token and would otherwise be open. Set \
             --cluster-token (DEMOCRATOS_CLUSTER_TOKEN)."
        );
    }

    // Enforce TLS on every off-box federation and control-plane link. These carry
    // the shared cluster token and the whole replicated dataset; on a plaintext
    // link a network attacker captures the token and reads every community's feed.
    for (_node, url) in &args.peers {
        require_tls_for_remote("peer", url)?;
    }
    for endpoint in &args.etcd_endpoints {
        require_tls_for_remote("etcd endpoint", endpoint)?;
    }

    // --- signing identity: a persistent seed, or an ephemeral one with a loud warning ---
    let keypair = match std::env::var("DEMOCRATOS_NODE_SEED") {
        Ok(hex) => NodeKeypair::from_seed_hex(node, hex.trim())
            .map_err(|e| anyhow!("DEMOCRATOS_NODE_SEED is not a valid 32-byte hex seed: {e}"))?,
        Err(_) => {
            let kp = NodeKeypair::generate(node);
            // Do NOT print the private seed: if logs are shipped/aggregated it
            // leaks the node's federation signing key, letting anyone forge this
            // node's signed events. Print only the public key so the operator can
            // recognise the identity; the seed must be generated out-of-band
            // (`openssl rand -hex 32`) and provided via DEMOCRATOS_NODE_SEED.
            eprintln!(
                "⚠ federation: DEMOCRATOS_NODE_SEED unset — generated an EPHEMERAL identity \
                   (public key {}). It changes every restart, so peers will not recognise this \
                   node across restarts. For a stable identity, generate a seed out-of-band \
                   (`openssl rand -hex 32`) and set DEMOCRATOS_NODE_SEED to it (keep it secret).",
                kp.public().to_hex()
            );
            kp
        }
    };
    let keypair = Arc::new(keypair);

    // --- control plane: etcd for a real cluster, in-process for a single node ---
    let registry: Arc<dyn OwnershipRegistry> = if args.etcd_endpoints.is_empty() {
        eprintln!(
            "federation: in-process registry (single-node/dev). Pass --etcd-endpoints for a cluster."
        );
        Arc::new(InMemoryRegistry::new())
    } else {
        let reg = EtcdRegistry::connect(&args.etcd_endpoints, args.lease_ttl_secs, node)
            .await
            .map_err(|e| anyhow!("etcd connect: {e}"))?;
        Arc::new(reg)
    };

    registry
        .publish_key(node, &keypair.public().to_hex())
        .await
        .map_err(|e| anyhow!("publish key: {e}"))?;

    // Claim the communities this node currently hosts, so its feed stamps the
    // correct epoch and peers can authorize its events.
    let hosted = claim_hosted(&*store, registry.as_ref(), node, keypair.as_ref()).await?;

    // --- replicate from peers ---
    let replicator = Arc::new(Replicator::new(store.clone(), registry.clone()));
    if !args.peers.is_empty() {
        let peers: Vec<Peer> = args
            .peers
            .iter()
            .map(|(n, url)| Peer {
                node: *n,
                client: FeedClient::new(url.clone(), args.cluster_token.clone()),
            })
            .collect();
        spawn_puller(
            replicator.clone(),
            peers,
            Duration::from_secs(args.poll_interval_secs.max(1)),
            500,
        );
    }

    // --- the governance-write gateway the delivery adapters submit votes to ---
    // Forward writes to a community's owner; when we're the owner, execute with
    // quorum-of-2 durability (sync-replicate to a standby before acking). Every
    // peer is registered both as a command target (in case it owns a community)
    // and as a possible standby (in case we own one and it's our standby).
    let mut transport = HttpCommandTransport::new();
    let mut sync = SyncVoteExecutor::new(
        node,
        services.clone(),
        store.clone(),
        keypair.clone(),
        registry.clone(),
    );
    for (n, url) in &args.peers {
        let peer = NodeId(*n as u16);
        transport = transport.with_peer(
            peer,
            CommandClient::new(url.clone(), args.cluster_token.clone(), keypair.clone()),
        );
        sync = sync.with_standby(
            peer,
            IngestClient::new(url.clone(), args.cluster_token.clone()),
        );
    }
    let router = WriteRouter::new(
        node,
        services.clone(),
        registry.clone(),
        Arc::new(transport),
    )
    .with_sync(Arc::new(sync));
    let writes: Arc<dyn GovernanceWrites> = Arc::new(FederatedWrites::new(router));

    // --- serve this node's feed + command + ingest endpoints (node-only) ---
    let feed_state = FeedState {
        store: store.clone(),
        keypair: keypair.clone(),
        registry: registry.clone(),
        token: args.cluster_token.clone(),
    };
    let command_state = CommandState {
        services,
        token: args.cluster_token.clone(),
        registry: registry.clone(),
        // Durable replay guard: nonces persist in Postgres so a captured command
        // can't be replayed against this owner after a restart (within the window).
        replay_guard: std::sync::Arc::new(
            adapter_federation::command::replay_guard::ReplayGuard::new(
                store.clone() as std::sync::Arc<dyn adapter_federation::command::nonce_log::NonceLog>,
            ),
        ),
    };
    let ingest_state = IngestState {
        replicator,
        token: args.cluster_token.clone(),
    };
    let feed_addr = args.federation_addr.clone();
    tokio::spawn(async move {
        if let Err(e) = serve_federation(feed_state, command_state, ingest_state, &feed_addr).await
        {
            eprintln!("⚠ federation: server on {feed_addr} exited: {e}");
        }
    });

    // --- heartbeat: re-claim newly-founded communities + report load ---
    spawn_maintenance(
        store.clone(),
        registry.clone(),
        node,
        keypair.clone(),
        args.lease_ttl_secs,
    );

    // --- failover: take over communities we're the (quietest) standby for ---
    spawn_rehoming(store, registry, node, args.lease_ttl_secs);

    eprintln!(
        "federation: node {} — feed on {}, {} peer(s), {} owned communit{}",
        args.node_id,
        args.federation_addr,
        args.peers.len(),
        hosted,
        if hosted == 1 { "y" } else { "ies" }
    );
    Ok(writes)
}
