//! Global CLI: shared attacker credentials plus the chosen subcommand.

use clap::Parser;

use crate::cmd::Cmd;

#[derive(Parser)]
#[command(about = "Adversary tool for the Byzantine federation harness")]
pub(crate) struct Cli {
    /// etcd control-plane endpoint (the attacker has write access).
    #[arg(long, env = "REDTEAM_ETCD", default_value = "http://byz-etcd:2379")]
    pub(crate) etcd: String,
    /// Shared cluster (federation bearer) token the attacker captured.
    #[arg(long, env = "REDTEAM_TOKEN")]
    pub(crate) token: Option<String>,
    /// The node identity the attacker acts under. Defaults to a high id modelling an
    /// external adversary who holds cluster credentials; pass a real compromised
    /// node's id + --seed to model a captured cluster member.
    #[arg(long, default_value_t = 250)]
    pub(crate) node: u16,
    /// Hex 32-byte seed for the attacker node identity (else a fresh one is minted).
    #[arg(long)]
    pub(crate) seed: Option<String>,
    #[command(subcommand)]
    pub(crate) cmd: Cmd,
}
