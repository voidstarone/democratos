//! Parse the admin review-queue CIDR allowlist from its CLI string.

use std::sync::Arc;

use anyhow::{Context, Result};
use ipnet::IpNet;

/// Parse a comma-separated CIDR list (e.g. `"192.168.1.0/24, 10.0.0.0/8"`) into
/// the allowlist the admin gate checks. `None`/empty yields an empty list, which
/// the gate treats as "loopback only". A malformed entry fails the boot.
pub(crate) fn parse_admin_subnets(spec: Option<&str>) -> Result<Arc<[IpNet]>> {
    let Some(spec) = spec else {
        return Ok(Arc::from(Vec::new()));
    };
    let mut nets = Vec::new();
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let net: IpNet = part
            .parse()
            .with_context(|| format!("invalid --admin-subnet CIDR: {part:?}"))?;
        nets.push(net);
    }
    Ok(Arc::from(nets))
}
