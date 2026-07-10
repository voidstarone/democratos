//! `redteam` — the adversary for the Byzantine federation harness.
//!
//! It links the REAL `federation` crate, so every artifact it produces is
//! cryptographically well-formed: genuine Ed25519 signatures, genuine control-plane
//! writes. It models an attacker who has fully compromised a *majority* of the
//! cluster — it holds a node identity, the shared cluster (federation bearer) token,
//! and write access to the etcd control plane — and tries to subvert an **honest,
//! founder-bound community** owned by the honest minority.
//!
//! The thesis under test: in this open federation, ownership authority is the
//! community's **founder key**, not node-count or etcd-write. So a compromised
//! majority must NOT be able to seize, forge, or rewrite an honest founder-bound
//! community — while the known-open holes (FED-1 unbound-community takeover, FED-3
//! binding poisoning, HIGH-1 comment scope) are exercised as documented xfails.
//!
//! Every subcommand prints a final `OUTCOME=<token>` line the shell harness asserts
//! on, and exits 0 (the *shell* decides pass/fail from the token, so an "attack
//! succeeded" is not a process error).

mod cli;
mod cmd;
mod connect;
mod current_epoch;
mod demoi_event;
mod forge_comment;
mod forge_event;
mod poison_binding;
mod push;
mod seize_bound;
mod serve_rogue;
mod takeover_unbound;

use anyhow::Result;
use clap::Parser;

use crate::cli::Cli;
use crate::cmd::Cmd;
use crate::forge_comment::forge_comment;
use crate::forge_event::forge_event;
use crate::poison_binding::poison_binding;
use crate::seize_bound::seize_bound;
use crate::serve_rogue::serve_rogue;
use crate::takeover_unbound::takeover_unbound;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.cmd {
        Cmd::SeizeBound { demos } => seize_bound(&cli, *demos).await,
        Cmd::ForgeEvent { demos, feed } => forge_event(&cli, *demos, feed).await,
        Cmd::TakeoverUnbound { demos, feed } => takeover_unbound(&cli, *demos, feed).await,
        Cmd::ForgeComment { post, feed } => forge_comment(&cli, *post, feed).await,
        Cmd::PoisonBinding { demos } => poison_binding(&cli, *demos).await,
        Cmd::ServeRogue { demos, bind } => serve_rogue(&cli, *demos, bind).await,
    }
}
