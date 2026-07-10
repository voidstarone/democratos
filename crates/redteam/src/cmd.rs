//! The adversary's subcommands, each a distinct attack or guardrail probe.

use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum Cmd {
    /// GUARDRAIL: try to seize an honest, founder-bound community via the control
    /// plane (publish a rival community key + claim its lease). Both must be refused.
    SeizeBound {
        #[arg(long)]
        demos: u64,
    },
    /// GUARDRAIL: sign a `demoi` rewrite as a non-owner node and push it to an honest
    /// node's ingest. The honest `authorize()` must reject it (0 applied).
    ForgeEvent {
        #[arg(long)]
        demos: u64,
        /// Base URL of an honest node's federation endpoint (e.g. http://byz-node1:7400).
        #[arg(long)]
        feed: String,
    },
    /// XFAIL (FED-1): take over an UNBOUND community — publish an attacker community
    /// key + a self-signed home binding + claim, then forge a `demoi` for it. The
    /// honest node applies it (attacker now "owns" a community it never founded).
    TakeoverUnbound {
        #[arg(long)]
        demos: u64,
        #[arg(long)]
        feed: String,
    },
    /// PROBE (HIGH-1): push a `comments` event with NO derivable community (the exact
    /// shape the outbox emits — demos_id NULL) and report what the honest node does.
    ForgeComment {
        #[arg(long)]
        post: u64,
        #[arg(long)]
        feed: String,
    },
    /// XFAIL (FED-3): install an attacker-signed home binding for an honest community
    /// without the control plane verifying it. Poisons authorization for that
    /// community (its real owner's events then fail `binding.verify`). Destructive —
    /// run last.
    PoisonBinding {
        #[arg(long)]
        demos: u64,
    },
    /// GUARDRAIL (malicious-peer / "forked binary" config): run a rogue node that
    /// serves a feed of forged events for an honest community. Honest peers pull it
    /// and must reject every event. Long-running.
    ServeRogue {
        #[arg(long)]
        demos: u64,
        #[arg(long, default_value = "0.0.0.0:7400")]
        bind: String,
    },
}
