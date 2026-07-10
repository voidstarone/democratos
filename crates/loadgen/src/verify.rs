//! `verify` — assert the authoritative tally is consistent and that a replica DB
//! converges to the same tally.

use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use adapter_store_postgres::PostgresStore;
use app::VoteStore;
use domain::{NodeId, ProposalId};

use crate::lg_cfg::lg_cfg;
use crate::manifest::Manifest;

pub(crate) async fn verify(
    manifest_path: &str,
    owner_db: &str,
    replica_db: Option<&str>,
    timeout_secs: u64,
) -> Result<()> {
    let manifest: Manifest = serde_json::from_slice(&std::fs::read(manifest_path)?)?;
    let pid = ProposalId(manifest.proposal_id);
    let owner = PostgresStore::connect_with(owner_db, NodeId(0), lg_cfg())
        .await
        .context("owner db")?;

    let tally = VoteStore::tally(&owner, pid).await?;
    let total = tally.aye + tally.nay;
    let voters = manifest.voter_ids.len() as u64;
    println!("── verify ────────────────────────────────────");
    println!(
        "  authoritative tally: aye {}  nay {}  total {}",
        tally.aye, tally.nay, total
    );
    println!("  seeded voters:       {voters}");

    let mut ok = true;
    if total > voters {
        println!("  ✗ MORE votes than voters — double counting!");
        ok = false;
    } else {
        println!("  ✓ no voter double-counted (total ≤ voters)");
    }

    if let Some(replica_db) = replica_db {
        let replica = PostgresStore::connect_with(replica_db, NodeId(0), lg_cfg())
            .await
            .context("replica db")?;
        let deadline = Instant::now() + Duration::from_secs(timeout_secs);
        let t0 = Instant::now();
        loop {
            let rt = VoteStore::tally(&replica, pid).await?;
            if rt.aye == tally.aye && rt.nay == tally.nay {
                println!(
                    "  ✓ replica converged (aye {} nay {}) in {:.2}s",
                    rt.aye,
                    rt.nay,
                    t0.elapsed().as_secs_f64()
                );
                break;
            }
            if Instant::now() >= deadline {
                println!(
                    "  ✗ replica did NOT converge within {timeout_secs}s (replica aye {} nay {} vs owner aye {} nay {})",
                    rt.aye, rt.nay, tally.aye, tally.nay
                );
                ok = false;
                break;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }

    anyhow::ensure!(ok, "verification failed");
    println!("  ✓ all invariants hold");
    Ok(())
}
