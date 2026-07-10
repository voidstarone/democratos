//! GUARDRAIL: control-plane seizure of a bound community must fail.

use anyhow::Result;

use domain::NodeId;
use federation::{ClaimOutcome, CommunityKeypair, OwnershipRegistry};

use crate::cli::Cli;
use crate::connect::connect;

pub(crate) async fn seize_bound(cli: &Cli, demos: u64) -> Result<()> {
    let (reg, kp) = connect(cli).await?;
    eprintln!("== SEIZE-BOUND d/{demos} (attacker node {}) ==", cli.node);

    // 1. Try to publish a rival community key. Refused twice over: first-write-wins
    //    (the honest home node already published the real one) AND origin-auth (we
    //    can only sign the publish with our own node key, not the origin's).
    let atk = CommunityKeypair::generate(demos);
    let atk_hex = atk.public().to_hex();
    let proof = kp.sign_hex(federation::community_key_publish_challenge(demos, &atk_hex).as_bytes());
    let key_blocked = match reg.publish_community_key(demos, &atk_hex, &proof).await {
        Ok(()) => {
            eprintln!("  publish_community_key: ACCEPTED (no honest key present!)");
            false
        }
        Err(e) => {
            eprintln!("  publish_community_key: REFUSED ({})", e.0);
            true
        }
    };

    // 2. Try to claim the lease as the attacker node. The live honest owner holds it,
    //    and the founder binding does not authorize the attacker, so this must not
    //    yield ownership.
    let claim = reg.claim(demos, NodeId(cli.node)).await;
    match &claim {
        Ok(ClaimOutcome::Claimed { epoch }) => eprintln!("  claim: CLAIMED at epoch {epoch} (!!)"),
        Ok(ClaimOutcome::Held { by, epoch }) => {
            eprintln!("  claim: HELD by node {} at epoch {epoch}", by.0)
        }
        Err(e) => eprintln!("  claim: REFUSED ({})", e.0),
    }
    let owner_after = reg.owner_of(demos).await.ok().flatten();
    let seized = owner_after.map(|o| o.owner == NodeId(cli.node)).unwrap_or(false);

    if !seized && key_blocked {
        println!("OUTCOME=BLOCKED");
    } else {
        println!("OUTCOME=SEIZED");
    }
    Ok(())
}
