use domain::{Membership, PostingPolicy, Timestamp};

/// Pure decision: does this membership (if any) satisfy a community's posting
/// policy? A sanctioned member is always blocked; a non-member (`None`) passes
/// only under [`PostingPolicy::Open`].
pub(super) fn posting_allowed(
    policy: PostingPolicy,
    membership: Option<&Membership>,
    now: Timestamp,
) -> bool {
    if membership.is_some_and(|m| m.is_sanctioned(now)) {
        return false;
    }
    match policy {
        PostingPolicy::Open => true,
        PostingPolicy::Members => membership.is_some(),
        PostingPolicy::Voters => membership.is_some_and(|m| m.is_voter()),
        PostingPolicy::MinContribution(n) => membership.is_some_and(|m| m.contribution >= n),
    }
}
