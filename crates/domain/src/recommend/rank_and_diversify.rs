//! Rank and diversify scored candidates.

use std::collections::HashMap;

use crate::{DemosId, PostId};

/// Order scored candidates by affinity (descending; post id breaks ties) and
/// cap how many may come from any one community, then truncate to `limit`. A
/// `per_community_cap` of 0 means no cap. Keeps a single popular demos from
/// dominating a cross-community discovery feed.
pub fn rank_and_diversify(
    scored: Vec<(PostId, DemosId, f32)>,
    limit: usize,
    per_community_cap: usize,
) -> Vec<(PostId, DemosId, f32)> {
    let mut ranked = scored;
    ranked.sort_by(|a, b| {
        b.2.partial_cmp(&a.2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0 .0.cmp(&b.0 .0))
    });

    let mut per_community: HashMap<DemosId, usize> = HashMap::new();
    let mut out = Vec::new();
    for item in ranked {
        if out.len() >= limit {
            break;
        }
        let count = per_community.entry(item.1).or_insert(0);
        if per_community_cap != 0 && *count >= per_community_cap {
            continue;
        }
        *count += 1;
        out.push(item);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diversify_caps_per_community() {
        let scored = vec![
            (PostId(1), DemosId(1), 0.9),
            (PostId(2), DemosId(1), 0.8),
            (PostId(3), DemosId(1), 0.7),
            (PostId(4), DemosId(2), 0.6),
        ];
        let out = rank_and_diversify(scored, 10, 2);
        let from_d1 = out.iter().filter(|i| i.1 == DemosId(1)).count();
        assert_eq!(from_d1, 2, "demos 1 capped at 2");
        assert!(
            out.iter().any(|i| i.1 == DemosId(2)),
            "demos 2 still included"
        );
        // Highest scores survive the cap, in order.
        assert_eq!(out[0].0, PostId(1));
    }

    #[test]
    fn diversify_respects_limit() {
        let scored = vec![
            (PostId(1), DemosId(1), 0.9),
            (PostId(2), DemosId(2), 0.8),
            (PostId(3), DemosId(3), 0.7),
        ];
        assert_eq!(rank_and_diversify(scored, 2, 0).len(), 2);
    }
}
