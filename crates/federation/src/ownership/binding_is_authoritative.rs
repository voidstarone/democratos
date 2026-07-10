//! Whether a stored home binding is authoritative for its community.

use crate::{CommunityPublicKey, HomeBinding};

/// Whether a stored home binding is *authoritative* for its community: it must
/// verify against the community's published key. A binding that does not verify (or
/// whose community key is missing) is **not** a founder statement — it is treated as
/// **absent** everywhere ([`authorize`](crate::authorize) and
/// [`OwnershipRegistry::claim`](crate::OwnershipRegistry::claim)), so a poisoned
/// binding can neither grant ownership nor fence the community's honest owner.
/// Verified again at every read (not only when stored) as defense-in-depth against
/// a party with direct control-plane write access. See the FED-3 finding.
pub fn binding_is_authoritative(
    binding: &HomeBinding,
    community_key: Option<&CommunityPublicKey>,
) -> bool {
    matches!(community_key, Some(k) if binding.verify(k).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CommunityKeypair;

    #[test]
    fn binding_is_authoritative_only_for_a_matching_community_key() {
        // FED-3: the check both `authorize` and `claim` use to ignore a poisoned
        // binding. A binding only counts if it verifies against the community's key.
        let demos = 7u64;
        let real = CommunityKeypair::generate(demos);
        let binding = real.bind(1, vec![], 1);
        assert!(binding_is_authoritative(&binding, Some(&real.public())));
        let other = CommunityKeypair::generate(demos);
        assert!(!binding_is_authoritative(&binding, Some(&other.public()))); // poisoned
        assert!(!binding_is_authoritative(&binding, None)); // key missing
    }
}
