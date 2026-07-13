//! Select which trusted issuer to mint an account on.

use super::issuer_endpoint::IssuerEndpoint;

/// Pick the **least-loaded** trusted issuer to mint on, so account creation spreads
/// across the trusted set instead of hammering one node. Ordering is by hosted
/// community count, then recent request rate; ties break on the lower node id so the
/// choice is deterministic (useful for tests and for cache-friendly reuse). Returns
/// `None` when no trusted issuer is reachable — the caller must fail closed, since
/// no untrusted node may mint a fleet-wide account itself.
pub fn choose_issuer(candidates: &[IssuerEndpoint]) -> Option<&IssuerEndpoint> {
    candidates.iter().min_by(|a, b| {
        a.load
            .hosted_communities
            .cmp(&b.load.hosted_communities)
            .then(
                a.load
                    .requests_per_sec
                    .partial_cmp(&b.load.requests_per_sec)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(a.node.0.cmp(&b.node.0))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NodeLoad;
    use domain::NodeId;

    fn ep(node: u16, hosted: u32, rps: f64) -> IssuerEndpoint {
        IssuerEndpoint {
            node: NodeId(node),
            addr: format!("https://node{node}"),
            load: NodeLoad {
                hosted_communities: hosted,
                requests_per_sec: rps,
            },
        }
    }

    #[test]
    fn no_candidates_yields_none() {
        assert!(choose_issuer(&[]).is_none());
    }

    #[test]
    fn picks_the_least_loaded_issuer() {
        let eps = [ep(1, 10, 5.0), ep(4, 2, 50.0), ep(7, 2, 3.0)];
        // Node 4 and 7 tie on hosted communities; 7 has the lower request rate.
        assert_eq!(choose_issuer(&eps).unwrap().node, NodeId(7));
    }

    #[test]
    fn ties_break_on_the_lower_node_id_deterministically() {
        let eps = [ep(9, 1, 1.0), ep(2, 1, 1.0)];
        assert_eq!(choose_issuer(&eps).unwrap().node, NodeId(2));
    }
}
