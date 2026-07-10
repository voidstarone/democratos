//! GUARDRAIL: a forged event to an honest community is rejected.

use anyhow::Result;

use crate::cli::Cli;
use crate::connect::connect;
use crate::current_epoch::current_epoch;
use crate::demoi_event::demoi_event;
use crate::push::push;

pub(crate) async fn forge_event(cli: &Cli, demos: u64, feed: &str) -> Result<()> {
    let (reg, kp) = connect(cli).await?;
    let epoch = current_epoch(&reg, demos).await;
    eprintln!("== FORGE-EVENT d/{demos} → {feed} (as non-owner node {}, epoch {epoch}) ==", cli.node);
    let ev = demoi_event(&kp, demos, epoch, 1, "HACKED-BY-MAJORITY");
    let applied = push(feed, cli.token.clone(), cli.node as i64, std::slice::from_ref(&ev)).await?;
    eprintln!("  honest node applied {applied} of 1 forged events");
    println!("OUTCOME=APPLIED:{applied}");
    Ok(())
}
