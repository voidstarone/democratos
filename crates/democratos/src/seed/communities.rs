//! The seeded communities.

use domain::PostingPolicy;

use crate::seed::community::Community;

pub(crate) const COMMUNITIES: &[Community] = &[
    Community {
        slug: "photography",
        name: "Photography",
        founder: "ansel",
        final_policy: PostingPolicy::Open,
    },
    Community {
        slug: "rustlang",
        name: "Rustaceans",
        founder: "graydon",
        final_policy: PostingPolicy::Members,
    },
    Community {
        slug: "politics",
        name: "Politics",
        founder: "hypatia",
        // Only members with popularity ≥ 5 may post going forward — the low-fame
        // tail is deliberately shut out to show the gate working.
        final_policy: PostingPolicy::MinContribution(5),
    },
];
