//! Top-level CLI parser.

use clap::Parser;

use crate::cmd::Cmd;

#[derive(Parser)]
#[command(about = "Load & correctness driver for a Democratos federation")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) cmd: Cmd,
}
