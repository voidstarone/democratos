//! Build a tag-affinity profile from liked posts.

use std::collections::HashMap;

/// Build a tag-affinity profile from the tags of the posts a user liked: each
/// tag's weight is the fraction of liked posts carrying it. Used for the
/// cold-start fallback when a user has too few votes for collaborative
/// filtering to find neighbours.
pub fn tag_profile(liked_post_tags: &[Vec<String>]) -> HashMap<String, f32> {
    let mut counts: HashMap<String, f32> = HashMap::new();
    for tags in liked_post_tags {
        for tag in tags {
            *counts.entry(tag.clone()).or_insert(0.0) += 1.0;
        }
    }
    let n = liked_post_tags.len().max(1) as f32;
    for w in counts.values_mut() {
        *w /= n;
    }
    counts
}
