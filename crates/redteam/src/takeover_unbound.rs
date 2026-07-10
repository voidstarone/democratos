//! XFAIL FED-1: takeover of an UNBOUND community succeeds.

use anyhow::{anyhow, Result};

use domain::NodeId;
use federation::{ClaimOutcome, CommunityKeypair, OwnershipRegistry};

use crate::cli::Cli;
use crate::connect::connect;

pub(crate) async fn takeover_unbound(cli: &Cli, demos: u64, _feed: &str) -> Result<()> {
    let (reg, kp) = connect(cli).await?;
    eprintln!("== TAKEOVER-UNBOUND d/{demos} (FED-1) ==");

    // The attacker tries to mint a community key for a community that has none, sign a
    // home binding naming itself, publish both, and claim the lease — the durable
    // takeover that survives fencing. Origin-authentication now gates the key publish:
    // the attacker can only sign it with its OWN node key, not the community's origin
    // node key, so a real (origin-authenticated) community rejects it here.
    let atk = CommunityKeypair::generate(demos);
    let atk_hex = atk.public().to_hex();
    let proof =
        kp.sign_hex(federation::community_key_publish_challenge(demos, &atk_hex).as_bytes());
    if let Err(e) = reg.publish_community_key(demos, &atk_hex, &proof).await {
        eprintln!("  publish_community_key REFUSED (origin-auth / first-write): {}", e.0);
        // No verifying community key ⇒ no fencing-surviving binding ⇒ durable takeover
        // is blocked, even if the lease itself is later claimable (permissive, unbound).
        println!("OUTCOME=SEIZED:false");
        return Ok(());
    }
    let binding = atk.bind(cli.node, vec![], 1);
    reg.set_home_binding(&binding)
        .await
        .map_err(|e| anyhow!("set_home_binding: {}", e.0))?;
    let claimed = reg.claim(demos, NodeId(cli.node)).await;
    eprintln!("  claim: {claimed:?}");

    // The proof of takeover: the control plane now names the attacker as owner, and
    // the (attacker-forged) binding authorizes it — so authorize() would accept its
    // events. That is the FED-1 seizure, independent of any row apply.
    let owner = reg.owner_of(demos).await.ok().flatten();
    let owns = owner.map(|o| o.owner == NodeId(cli.node)).unwrap_or(false);
    let authorized = reg
        .home_binding(demos)
        .await
        .ok()
        .flatten()
        .map(|b| b.authorizes(cli.node))
        .unwrap_or(false);
    eprintln!("  owner_of(d/{demos}) = {owner:?}; binding authorizes attacker = {authorized}");
    let seized = matches!(claimed, Ok(ClaimOutcome::Claimed { .. })) && owns && authorized;
    println!("OUTCOME=SEIZED:{seized}");
    Ok(())
}
