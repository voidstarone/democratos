//! `loadgen` — functional + stress driver for a running Democratos federation.
//!
//! It has four subcommands, run in order:
//!
//! * `seed`   — create a community, an open proposal, and N eligible voters
//!              directly in the **owner** node's Postgres (through the real
//!              store, so composite IDs and the outbox are correct and the data
//!              replicates to peers). Writes a manifest JSON.
//! * `vote`   — drive every seeded voter to cast one ballot, spread across the
//!              given node **web** URLs, at a chosen concurrency. Reports latency
//!              percentiles, throughput, and an error breakdown. Because votes
//!              cast on a non-owner node forward to the owner, this exercises the
//!              whole federated write path (forward + sync-replicate).
//! * `verify` — assert the authoritative tally equals the number of accepted
//!              votes, that no voter was double-counted, and that a replica DB
//!              converges to the same tally (reporting the convergence lag).
//! * `read`   — hammer a GET endpoint across nodes for a read-throughput number.
//!
//! Auth note: the app identifies the acting user by a `uid` cookie (no
//! password), so a driver "logs in" as voter `N` simply by sending `uid=N`.

mod cli;
mod cmd;
mod lg_cfg;
mod manifest;
mod pct;
mod read;
mod seed;
mod verify;
mod vote;

use anyhow::Result;
use clap::Parser;

use crate::cli::Cli;
use crate::cmd::Cmd;
use crate::read::read;
use crate::seed::seed;
use crate::verify::verify;
use crate::vote::vote;

#[tokio::main]
async fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Seed {
            owner_db,
            node_id,
            voters,
            slug,
            out,
        } => seed(&owner_db, node_id, voters, &slug, &out).await,
        Cmd::Vote {
            manifest,
            nodes,
            concurrency,
        } => vote(&manifest, &nodes, concurrency).await,
        Cmd::Verify {
            manifest,
            owner_db,
            replica_db,
            timeout_secs,
        } => verify(&manifest, &owner_db, replica_db.as_deref(), timeout_secs).await,
        Cmd::Read {
            nodes,
            path,
            requests,
            concurrency,
        } => read(&nodes, &path, requests, concurrency).await,
    }
}
