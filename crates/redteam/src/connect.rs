//! Connect to etcd under the attacker's node identity and publish its node key.

use std::sync::Arc;

use anyhow::{anyhow, Result};

use adapter_control_etcd::EtcdRegistry;
use domain::NodeId;
use federation::{NodeKeypair, OwnershipRegistry};

use crate::cli::Cli;

/// Connect to etcd under the attacker's node identity and publish its node key so a
/// victim's `authorize` treats it as a *known* node — making the ownership check
/// (not merely "unknown key") the thing that rejects a forged event.
pub(crate) async fn connect(cli: &Cli) -> Result<(EtcdRegistry, Arc<NodeKeypair>)> {
    let node = NodeId(cli.node);
    let keypair = match &cli.seed {
        Some(hex) => NodeKeypair::from_seed_hex(node, hex.trim())
            .map_err(|e| anyhow!("bad --seed: {e}"))?,
        None => NodeKeypair::generate(node),
    };
    let reg = EtcdRegistry::connect(&[cli.etcd.clone()], 10, node)
        .await
        .map_err(|e| anyhow!("etcd connect ({}): {}", cli.etcd, e.0))?;
    // Best-effort: publish the attacker's own key (first-write-wins; already-present
    // is fine). Any node may publish its own identity.
    if let Err(e) = reg.publish_key(node, &keypair.public().to_hex()).await {
        eprintln!("note: publish_key: {}", e.0);
    }
    Ok((reg, Arc::new(keypair)))
}
