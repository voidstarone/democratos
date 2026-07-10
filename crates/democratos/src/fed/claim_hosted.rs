//! Claim the communities this node hosts, and keep their home bindings fresh.

use anyhow::{anyhow, Result};

use adapter_store_postgres::PostgresStore;
use app::DemosStore;
use domain::{origin_node, NodeId};
use federation::{
    choose_new_standby, CommunityKeypair, NodeKeypair, NodeLoad, OwnershipRegistry,
};

/// Claim every community in the local store for `node`, returning the count.
pub(crate) async fn claim_hosted(
    store: &PostgresStore,
    registry: &dyn OwnershipRegistry,
    node: NodeId,
    keypair: &NodeKeypair,
) -> Result<usize> {
    let demoi = DemosStore::list(store)
        .await
        .map_err(|e| anyhow!("list communities: {e}"))?;
    for d in &demoi {
        if let Err(e) = registry.claim(d.id.0, node).await {
            eprintln!("⚠ federation: could not claim d/{}: {e}", d.slug);
        }
    }
    let _ = registry
        .report_load(
            node,
            NodeLoad {
                hosted_communities: demoi.len() as u32,
                requests_per_sec: 0.0,
            },
        )
        .await;

    // Ensure each community we own has a standby, so votes can meet their
    // quorum of 2. Standbys are otherwise only picked during a failover, which
    // would leave a freshly-booted cluster unable to accept sync-replicated
    // votes. Pick the quietest *other* live node; converges once peers report
    // load (a later heartbeat retries if none is live yet).
    let loads = registry.live_nodes().await.unwrap_or_default();
    for d in &demoi {
        let has_standby = registry
            .standbys(d.id.0)
            .await
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        if !has_standby {
            if let Some(pick) = choose_new_standby(&[node], &loads) {
                if let Err(e) = registry.set_standby(d.id.0, pick).await {
                    eprintln!("⚠ federation: could not set standby for d/{}: {e}", d.slug);
                }
            }
        }
    }

    // Establish (or refresh) the founder-signed home binding for each community
    // THIS node homes — identified by the id's origin bits, i.e. the node it was
    // founded on (the founder's chosen host). The binding names this node as home
    // and its current standbys as the pre-authorized failover set, so no other node
    // can seize the community and a designated standby can still take over on our
    // downtime. Imported/replica communities keep their original origin, so we never
    // mint for them — they stay unbound and unconstrained, which is exactly what
    // keeps import and cross-node migration working.
    for d in &demoi {
        if origin_node(d.id.0) != node {
            continue;
        }
        if let Err(e) = ensure_home_binding(store, registry, node, d.id.0, keypair).await {
            eprintln!(
                "⚠ federation: could not establish home binding for d/{}: {e}",
                d.slug
            );
        }
    }
    Ok(demoi.len())
}

/// Establish (or refresh) a community's founder-signed home binding, naming `node`
/// as its home and its current standbys as the pre-authorized failover set. The
/// community's signing seed is minted once and persisted (home-node-only, never
/// replicated) so the identity is stable across restarts and can re-sign a future
/// re-home. First-write-wins on the public key means an established community's key
/// is never overwritten.
async fn ensure_home_binding(
    store: &PostgresStore,
    registry: &dyn OwnershipRegistry,
    node: NodeId,
    demos: u64,
    keypair: &NodeKeypair,
) -> Result<()> {
    let seed_hex = match store.community_seed(demos as i64).await? {
        Some(s) => s,
        None => {
            let kp = CommunityKeypair::generate(demos);
            let seed = kp.seed_hex();
            store.set_community_seed(demos as i64, &seed).await?;
            seed
        }
    };
    let community =
        CommunityKeypair::from_seed_hex(demos, &seed_hex).map_err(|e| anyhow!("community key: {e}"))?;
    // Origin-authenticate the community-key publish (FED-1). We only reach here for
    // communities THIS node originated (`origin_node(demos) == node`), so our node
    // key is the origin key the registry verifies the publish against.
    let community_hex = community.public().to_hex();
    let origin_proof = keypair.sign_hex(
        federation::community_key_publish_challenge(demos, &community_hex).as_bytes(),
    );
    registry
        .publish_community_key(demos, &community_hex, &origin_proof)
        .await
        .map_err(|e| anyhow!("publish community key: {e}"))?;
    let failover: Vec<u16> = registry
        .standbys(demos)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|n| n.0)
        .collect();
    registry
        .set_home_binding(&community.bind(node.0, failover, 1))
        .await
        .map_err(|e| anyhow!("set home binding: {e}"))?;
    Ok(())
}
