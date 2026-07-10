//! XFAIL FED-3: poison an honest community's binding (control plane).

use anyhow::Result;

use federation::{CommunityKeypair, OwnershipRegistry};

use crate::cli::Cli;
use crate::connect::connect;

pub(crate) async fn poison_binding(cli: &Cli, demos: u64) -> Result<()> {
    let (reg, _kp) = connect(cli).await?;
    eprintln!("== POISON-BINDING d/{demos} (FED-3, DESTRUCTIVE) ==");
    // set_home_binding does not verify the binding against the community key, and it
    // keeps the highest epoch — so a high-epoch attacker binding overwrites the real
    // one. Its signature won't verify against the honest community key, so the honest
    // owner's own events then fail authorize → the community is DoSed fleet-wide.
    let atk = CommunityKeypair::generate(demos);
    let poison = atk.bind(cli.node, vec![], 9_999);
    let stored = match reg.set_home_binding(&poison).await {
        Ok(()) => {
            eprintln!("  set_home_binding: ACCEPTED an unverified attacker binding (epoch 9999)");
            true
        }
        Err(e) => {
            eprintln!("  set_home_binding: refused ({})", e.0);
            false
        }
    };
    println!("OUTCOME=POISONED:{stored}");
    Ok(())
}
