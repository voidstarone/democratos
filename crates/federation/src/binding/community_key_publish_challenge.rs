//! The canonical bytes an origin node signs to authorise publishing a community key.

/// Canonical bytes an **origin node** signs to authorise publishing a community's
/// public key. Ties the community key to the community's origin/founding node
/// (`domain::origin_node(demos)`, the id's high bits), so a hostile peer cannot
/// pre-empt or hijack the key of a community founded by an honest node it does not
/// control — closing the first-write-wins takeover of the community key. See FED-1.
pub fn community_key_publish_challenge(demos: u64, community_public_hex: &str) -> String {
    format!("democratos:community-key-publish:v1;demos:{demos};key:{community_public_hex}")
}
