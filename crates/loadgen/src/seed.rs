//! `seed` — create a community, an open proposal, and N eligible voters directly
//! in the owner node's Postgres, then write a manifest JSON.

use anyhow::{Context, Result};

use adapter_store_postgres::PostgresStore;
use app::{DemosStore, MembershipStore, ProposalStore, UserStore};
use domain::{Membership, NodeId, ProposalKind, Tier, Timestamp};

use crate::lg_cfg::lg_cfg;
use crate::manifest::Manifest;

pub(crate) async fn seed(
    owner_db: &str,
    node_id: u16,
    voters: u32,
    slug: &str,
    out: &str,
) -> Result<()> {
    let store = PostgresStore::connect_with(owner_db, NodeId(node_id), lg_cfg())
        .await
        .context("connect owner db")?;
    let ts = Timestamp(1);
    // A run-unique slug so repeated seeds don't collide on the unique index.
    let slug = format!("{slug}-{}", std::process::id());

    let founder = UserStore::create(
        &store,
        &format!("lg-founder-{}", std::process::id()),
        None,
        None,
        ts,
    )
    .await
    .context("create founder")?;
    let demos = DemosStore::create(&store, &slug, "Load Test", founder.id, ts)
        .await
        .context("create demos")?;
    let proposal = ProposalStore::create(
        &store,
        demos.id,
        founder.id,
        ProposalKind::AddRule {
            text: "load-test proposal".into(),
        },
        ts,
        Timestamp(10_000_000_000), // open far into the future
    )
    .await
    .context("create proposal")?;

    println!("seeding {voters} voters…");
    let mut voter_ids = Vec::with_capacity(voters as usize);
    for i in 0..voters {
        let u = UserStore::create(
            &store,
            &format!("lg-v{}-{i}", std::process::id()),
            None,
            None,
            ts,
        )
        .await
        .context("create voter")?;
        let mut m = Membership::joined(u.id, demos.id, ts);
        m.tier = Tier::Voter;
        m.enfranchised_at = Some(ts);
        MembershipStore::upsert(&store, m)
            .await
            .context("enfranchise voter")?;
        voter_ids.push(u.id.0);
        if (i + 1) % 100 == 0 {
            println!("  {} / {voters}", i + 1);
        }
    }

    let manifest = Manifest {
        demos_id: demos.id.0,
        proposal_id: proposal.id.0,
        founder_id: founder.id.0,
        voter_ids,
    };
    std::fs::write(out, serde_json::to_vec_pretty(&manifest)?)?;
    println!(
        "seeded d/{slug} (id {}), proposal {}, {} voters → {out}",
        demos.id.0, proposal.id.0, voters
    );
    Ok(())
}
