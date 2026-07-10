//! Parse a `--peer <node_id>=<base_url>` value.

use anyhow::{Context, Result};

/// Parse a `--peer <node_id>=<base_url>` value.
pub fn parse_peer(s: &str) -> Result<(i64, String)> {
    let (node, url) = s
        .split_once('=')
        .context("--peer must be <node_id>=<base_url>")?;
    let node = node
        .trim()
        .parse()
        .context("peer node id must be an integer")?;
    Ok((node, url.trim().to_string()))
}
