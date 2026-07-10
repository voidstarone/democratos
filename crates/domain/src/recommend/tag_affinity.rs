//! Score a candidate post against a tag profile.

use std::collections::HashMap;

/// Score a candidate post against a tag [`crate::tag_profile`]: the mean profile weight
/// of the tags it carries. Zero for untagged posts or no overlap.
pub fn tag_affinity(profile: &HashMap<String, f32>, post_tags: &[String]) -> f32 {
    if post_tags.is_empty() {
        return 0.0;
    }
    let sum: f32 = post_tags
        .iter()
        .map(|t| profile.get(t).copied().unwrap_or(0.0))
        .sum();
    sum / post_tags.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tag_profile;

    #[test]
    fn tag_fallback_scores_overlap() {
        let profile = tag_profile(&[vec!["rust".into(), "async".into()], vec!["rust".into()]]);
        // "rust" appears in both liked posts → weight 1.0; "async" in one → 0.5.
        assert!((profile["rust"] - 1.0).abs() < 1e-6);
        assert!((profile["async"] - 0.5).abs() < 1e-6);

        let rusty = tag_affinity(&profile, &["rust".into()]);
        let unrelated = tag_affinity(&profile, &["cooking".into()]);
        assert!(rusty > unrelated);
        assert_eq!(unrelated, 0.0);
        assert_eq!(tag_affinity(&profile, &[]), 0.0);
    }
}
